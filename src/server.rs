//! `hkdiag serve`: the PWA, the topology snapshot and `whoami`.
//!
//! All diagnosis runs on the device (web/engine.js); the server only hands
//! out data, so it never spawns probes on a client's behalf.
//!
//! Meant to listen on localhost behind an HTTPS reverse proxy (a service
//! worker, and so offline mode, needs a secure context).
//!
//! Security model:
//! * Read-only GET/HEAD endpoints; static assets are compiled in, so no
//!   filesystem path is ever derived from the request.
//! * The snapshot holds infrastructure data only (see export.rs) and is
//!   rebuilt at most every `topology_cache_secs`, whatever the request rate;
//!   FRR failures back off instead of queueing requests.
//! * Only `X-Real-IP`, set (overwritten) by the proxy, is honoured, and only
//!   from `trusted_proxies`.

use crate::config::{Config, WebConfig};
use crate::export;
use crate::reports::{self, ReportIn};
use crate::state::State;
use crate::web::{self, ASSETS, CSP};
use anyhow::{anyhow, Context, Result};
use std::io::Read;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};
use tiny_http::{Header, Method, Request, Response, Server};

const MAX_CONNECTION_THREADS: usize = 64;
/// After a failed OSPF read, fail fast for this long.
const TOPOLOGY_ERROR_BACKOFF: Duration = Duration::from_secs(15);
/// Save the prefix history at least this often even without changes.
const STATE_SAVE_INTERVAL: Duration = Duration::from_secs(600);

struct App {
    cfg: Config,
    snapshot: Mutex<SnapCache>,
    /// Held only while rebuilding, so readers of `snapshot` never wait on FRR/Sojka/UserDB.
    building: Mutex<()>,
    state: Mutex<(State, Instant)>,
    connections: AtomicUsize,
    service_worker: Vec<u8>,
    /// Outage reports; `None` if the database could not be opened.
    reports: Option<Mutex<reports::Store>>,
}

#[derive(Default)]
struct SnapCache {
    ok: Option<(Instant, Arc<Vec<u8>>)>,
    failed_at: Option<Instant>,
}

/// Decrements a counter when dropped, also when the holder panics.
struct SlotGuard<'a>(&'a AtomicUsize);

impl Drop for SlotGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn serve(cfg: Config) -> Result<()> {
    let web_cfg = Arc::new(cfg.web.clone().unwrap_or_default());
    let server = Server::http(&web_cfg.listen)
        .map_err(|e| anyhow!("cannot listen on {}: {e}", web_cfg.listen))?;
    eprintln!(
        "hkdiag web listening on http://{} (build {})",
        web_cfg.listen,
        web::build_id()
    );
    let db_path = web_cfg.reports_db.clone().or_else(reports::default_path);
    let reports = match db_path.as_deref().map(reports::Store::open) {
        Some(Ok(s)) => Some(Mutex::new(s)),
        Some(Err(e)) => {
            eprintln!("warning: outage reports disabled: {e:#}");
            None
        }
        None => None,
    };
    let app = Arc::new(App {
        reports,
        cfg,
        snapshot: Mutex::new(SnapCache::default()),
        building: Mutex::new(()),
        state: Mutex::new((State::load(), Instant::now())),
        connections: AtomicUsize::new(0),
        service_worker: web::service_worker().into_bytes(),
    });
    for req in server.incoming_requests() {
        if app.connections.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTION_THREADS {
            app.connections.fetch_sub(1, Ordering::SeqCst);
            let _ = req.respond(text(503, "server busy"));
            continue;
        }
        let app = Arc::clone(&app);
        let web_cfg = Arc::clone(&web_cfg);
        thread::spawn(move || {
            // Released when the thread ends, even if handle() panics.
            let _slot = SlotGuard(&app.connections);
            handle(&app, &web_cfg, req);
        });
    }
    Ok(())
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static header is valid")
}

fn secured<R: Read>(mut r: Response<R>) -> Response<R> {
    for (k, v) in [
        ("Content-Security-Policy", CSP),
        ("X-Content-Type-Options", "nosniff"),
        ("Referrer-Policy", "no-referrer"),
        ("X-Frame-Options", "DENY"),
        ("Cross-Origin-Opener-Policy", "same-origin"),
        (
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        r.add_header(header(k, v));
    }
    r
}

fn text(status: u16, msg: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    secured(
        Response::from_string(msg)
            .with_status_code(status)
            .with_header(header("Content-Type", "text/plain; charset=utf-8"))
            .with_header(header("Cache-Control", "no-store")),
    )
}

/// The real client address: the socket peer, or the proxy-set `X-Real-IP`
/// when (and only when) the peer is a trusted proxy. IPv4-mapped IPv6
/// addresses are normalised to IPv4.
pub fn client_ip(
    peer: Option<IpAddr>,
    headers: &[(String, String)],
    web: &WebConfig,
) -> Option<IpAddr> {
    let peer = peer?.to_canonical();
    if !web.trusted_proxies.iter().any(|p| p.to_canonical() == peer) {
        return Some(peer);
    }
    let mut real = headers
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("X-Real-IP"));
    match (real.next(), real.next()) {
        (Some((_, v)), None) => v.trim().parse::<IpAddr>().ok().map(|a| a.to_canonical()),
        // Duplicate headers: ambiguous, refuse.
        (Some(_), Some(_)) => None,
        // A trusted proxy that did not say who the client is: unknown, rather
        // than reporting the proxy's own address as the client's.
        (None, _) => None,
    }
}

fn headers_of(req: &Request) -> Vec<(String, String)> {
    req.headers()
        .iter()
        .map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string()))
        .collect()
}

fn json_body(status: u16, v: &serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    secured(
        Response::from_string(v.to_string())
            .with_status_code(status)
            .with_header(header("Content-Type", "application/json"))
            .with_header(header("Cache-Control", "no-store")),
    )
}

fn handle(app: &App, web_cfg: &WebConfig, mut req: Request) {
    let path = req.url().split('?').next().unwrap_or("/").to_string();
    // The only write endpoint.
    if path == "/api/reports" {
        let resp = if *req.method() == Method::Post {
            post_reports(app, web_cfg, &mut req)
        } else {
            text(405, "method not allowed")
        };
        let _ = req.respond(resp);
        return;
    }
    if !matches!(req.method(), Method::Get | Method::Head) {
        let _ = req.respond(text(405, "method not allowed"));
        return;
    }
    match path.as_str() {
        "/api/outages" => {
            let resp = match &app.reports {
                None => json_body(200, &serde_json::json!({ "enabled": false, "outages": [] })),
                Some(store) => match lock(store).public_recent(crate::now() - 7 * 24 * 3600, 200) {
                    Ok(list) => json_body(
                        200,
                        &serde_json::json!({ "enabled": true, "outages": list }),
                    ),
                    Err(e) => {
                        eprintln!("outages: {e:#}");
                        text(500, "outages unavailable")
                    }
                },
            };
            let _ = req.respond(resp);
        }
        "/api/whoami" => {
            let headers = headers_of(&req);
            // Loopback (local testing) says nothing about the hkfree.org address;
            // the app then asks the user to enter it.
            let ip = client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg)
                .filter(|i| !i.is_loopback() && !i.is_unspecified());
            let body = serde_json::json!({ "ip": ip.map(|i| i.to_string()) }).to_string();
            let r = Response::from_string(body)
                .with_header(header("Content-Type", "application/json"))
                .with_header(header("Cache-Control", "no-store"));
            let _ = req.respond(secured(r));
        }
        "/topology.json" => match snapshot(app, web_cfg) {
            Ok(bytes) => {
                let r = Response::new(
                    200.into(),
                    vec![
                        header("Content-Type", "application/json"),
                        header("Cache-Control", "no-cache"),
                    ],
                    &bytes[..],
                    Some(bytes.len()),
                    None,
                );
                let _ = req.respond(secured(r));
            }
            Err(e) => {
                // Details go to the server log only; they may mention internal hosts.
                eprintln!("topology snapshot: {e:#}");
                let _ = req.respond(text(502, "network state temporarily unavailable"));
            }
        },
        "/sw.js" => {
            let r = Response::new(
                200.into(),
                vec![
                    header("Content-Type", "text/javascript; charset=utf-8"),
                    header("Cache-Control", "no-cache"),
                ],
                &app.service_worker[..],
                Some(app.service_worker.len()),
                None,
            );
            let _ = req.respond(secured(r));
        }
        p => {
            let p = if p == "/" { "/index.html" } else { p };
            match ASSETS.iter().find(|a| a.path == p) {
                Some(a) => {
                    let r = Response::new(
                        200.into(),
                        // The service worker does the caching; always revalidate.
                        vec![
                            header("Content-Type", a.mime),
                            header("Cache-Control", "no-cache"),
                        ],
                        a.body,
                        Some(a.body.len()),
                        None,
                    );
                    let _ = req.respond(secured(r));
                }
                None => {
                    let _ = req.respond(text(404, "not found"));
                }
            }
        }
    }
}

/// Stores outage reports from the app. Only same-origin JSON is accepted:
/// a cross-site page cannot send `application/json` without a CORS preflight
/// (which this server never approves), and a mismatching Origin is refused.
fn post_reports(
    app: &App,
    web_cfg: &WebConfig,
    req: &mut Request,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let headers = headers_of(req);
    let get = |n: &str| {
        headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(n))
            .map(|(_, v)| v.as_str())
    };
    if !get("Content-Type").map_or(false, |c| c.trim_start().starts_with("application/json")) {
        return text(415, "application/json required");
    }
    if let Some(origin) = get("Origin") {
        let origin_host = origin.split_once("://").map_or(origin, |(_, h)| h);
        if Some(origin_host) != get("Host") {
            return text(403, "cross-origin reports are not accepted");
        }
    }
    let Some(store) = &app.reports else {
        return text(503, "reports are disabled on this server");
    };
    let mut body = Vec::new();
    if req
        .as_reader()
        .take(reports::MAX_BODY as u64 + 1)
        .read_to_end(&mut body)
        .is_err()
    {
        return text(400, "cannot read body");
    }
    if body.len() > reports::MAX_BODY {
        return text(413, "too large");
    }
    let Ok(list) = serde_json::from_slice::<Vec<ReportIn>>(&body) else {
        return text(400, "invalid reports");
    };
    if list.len() > reports::MAX_BATCH {
        return text(413, "too many reports");
    }
    let client = match client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg) {
        Some(IpAddr::V4(a)) => Some(a),
        _ => None::<Ipv4Addr>,
    };
    match lock(store).insert(client, &list, crate::now()) {
        Ok(n) => json_body(
            200,
            &serde_json::json!({ "accepted": n, "received": list.len() }),
        ),
        Err(e) => {
            eprintln!("reports: {e:#}");
            text(500, "cannot store reports")
        }
    }
}

/// The serialized snapshot, rebuilt at most every `topology_cache_secs`.
///
/// One request rebuilds while the others get the previous (stale) snapshot
/// immediately; it carries its own timestamp. Only the very first request,
/// with nothing cached yet, waits for a build.
fn snapshot(app: &App, web_cfg: &WebConfig) -> Result<Arc<Vec<u8>>> {
    let max_age = Duration::from_secs(web_cfg.topology_cache_secs);
    let cached = |app: &App| -> (Option<Arc<Vec<u8>>>, bool, bool) {
        let c = lock(&app.snapshot);
        let fresh =
            c.ok.as_ref()
                .map_or(false, |(at, _)| at.elapsed() < max_age);
        let backoff = c
            .failed_at
            .map_or(false, |f| f.elapsed() < TOPOLOGY_ERROR_BACKOFF);
        (c.ok.as_ref().map(|(_, b)| Arc::clone(b)), fresh, backoff)
    };

    let (stale, fresh, backoff) = cached(app);
    if let (Some(b), true) = (&stale, fresh) {
        return Ok(Arc::clone(b));
    }
    if backoff {
        return stale.ok_or_else(|| anyhow!("OSPF database unavailable (backing off)"));
    }
    let _build = match (&stale, app.building.try_lock()) {
        (_, Ok(g)) => g,
        // Someone else is rebuilding: serve what we have.
        (Some(b), Err(_)) => return Ok(Arc::clone(b)),
        // Nothing cached yet: wait for the running build, then use its result.
        (None, Err(_)) => {
            let g = lock(&app.building);
            if let (Some(b), true, _) = cached(app) {
                return Ok(b);
            }
            g
        }
    };
    // Re-check: a build may have finished between our look and taking the lock.
    if let (Some(b), true, _) = cached(app) {
        return Ok(b);
    }

    let topo = match crate::fetch_topology(&app.cfg).context("reading OSPF database") {
        Ok(t) => t,
        Err(e) => {
            lock(&app.snapshot).failed_at = Some(Instant::now());
            return stale.ok_or(e);
        }
    };
    let state_snapshot = {
        let mut st = lock(&app.state);
        let changed = st.0.update(&topo);
        if changed || st.1.elapsed() > STATE_SAVE_INTERVAL {
            st.1 = Instant::now();
            if let Err(e) = st.0.save() {
                eprintln!("warning: cannot save state: {e:#}");
            }
        }
        st.0.clone()
    };
    // Sojka and UserDB are queried here, outside every lock readers take.
    let bytes = Arc::new(serde_json::to_vec(&export::build(
        &app.cfg,
        &topo,
        &state_snapshot,
    ))?);
    let mut c = lock(&app.snapshot);
    c.ok = Some((Instant::now(), Arc::clone(&bytes)));
    c.failed_at = None;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn web() -> WebConfig {
        WebConfig {
            trusted_proxies: vec!["127.0.0.1".parse().unwrap()],
            ..Default::default()
        }
    }

    fn h(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[test]
    fn real_ip_only_from_trusted_proxy() {
        let hdr = vec![h("X-Real-IP", "10.107.5.130")];
        let ext: IpAddr = "10.107.9.9".parse().unwrap();
        assert_eq!(client_ip(Some(ext), &hdr, &web()), Some(ext));
        let proxy: IpAddr = "127.0.0.1".parse().unwrap();
        assert_eq!(
            client_ip(Some(proxy), &hdr, &web()),
            Some("10.107.5.130".parse().unwrap())
        );
    }

    #[test]
    fn forwarded_for_is_ignored_and_duplicates_refused() {
        let proxy: IpAddr = "127.0.0.1".parse().unwrap();
        let xff = vec![h("X-Forwarded-For", "10.107.99.5")];
        assert_eq!(client_ip(Some(proxy), &xff, &web()), None);
        assert_eq!(client_ip(Some(proxy), &[], &web()), None);
        let dup = vec![
            h("X-Real-IP", "10.107.99.5"),
            h("x-real-ip", "10.107.5.130"),
        ];
        assert_eq!(client_ip(Some(proxy), &dup, &web()), None);
        let bad = vec![h("X-Real-IP", "not-an-ip")];
        assert_eq!(client_ip(Some(proxy), &bad, &web()), None);
    }

    #[test]
    fn ipv4_mapped_addresses_are_normalised() {
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        let hdr = vec![h("X-Real-IP", "::ffff:10.107.5.130")];
        assert_eq!(
            client_ip(Some(mapped), &hdr, &web()),
            Some("10.107.5.130".parse().unwrap())
        );
    }

    #[test]
    fn slot_guard_releases_on_panic() {
        let c = AtomicUsize::new(1);
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = SlotGuard(&c);
            panic!("boom");
        }));
        assert!(r.is_err());
        assert_eq!(c.load(Ordering::SeqCst), 0);
    }
}
