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
//! * AP admins' e-mails are not in the snapshot: `/api/contact` gives a device
//!   only the admins of the AP its own address belongs to.
//! * `/api/signal` gives a device only its own antenna's signal. Reading it
//!   logs in to the AP's routers, so one AP is read at most once per
//!   `signal_cache_secs`, and only one read runs at a time.
//! * `/api/v1/history` gives the measurement history of the network's own
//!   addresses only (routers, AP devices); `/api/traffic` gives a device only
//!   its own address's traffic. Each series is read at most once per
//!   `history.cache_secs`, one read at a time.

use crate::config::{ClientIpHeader, Config, WebConfig};
use crate::export;
use crate::history;
use crate::reports::{self, ReportIn};
use crate::signal;
use crate::state::State;
use std::collections::BTreeMap;
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
    /// Radio data per AP id, with when it was read (failures too, so they aren't retried at once).
    radio: Mutex<BTreeMap<u32, (Instant, std::result::Result<Arc<RadioData>, String>)>>,
    /// Held while reading one AP's radio data.
    radio_fetch: Mutex<()>,
    /// UserDB's status as last read: the AP devices with a problem now.
    status_now: Mutex<Vec<crate::userdb::Problem>>,
    /// The power monitor as last read: APs on battery, announced power cuts.
    power: Mutex<crate::power::PowerState>,
    /// When UserDB's status and the power monitor were last read, and whether the last try worked.
    sources: Mutex<Sources>,
    /// When each client last sent speeds (the last hour only, never stored): a rate limit.
    speed_clients: Mutex<std::collections::HashMap<IpAddr, Vec<i64>>>,
    /// Measured series by what, address and range, with when they were read (failures too).
    history: Mutex<HistoryCache>,
    /// Held while reading one series from monitoring.
    history_fetch: Mutex<()>,
}

/// How the server's periodic reads are going: (last successful read, did the last try work).
#[derive(Default)]
struct Sources {
    status: (Option<i64>, bool),
    power: (Option<i64>, bool),
}

type HistoryCache = BTreeMap<(history::Kind, Ipv4Addr, history::Range), (Instant, std::result::Result<Arc<history::Series>, String>)>;
/// More cached series than this are a sign of a sweep, not of use.
const HISTORY_CACHE_MAX: usize = 4000;

/// An AP's stations and the member records of its devices (to tell whose antenna is whose).
type RadioData = (Vec<signal::Station>, BTreeMap<Ipv4Addr, crate::userdb::Device>);

#[derive(Default)]
struct SnapCache {
    ok: Option<(Instant, Arc<Vec<u8>>)>,
    /// The topology and admin contacts the snapshot was built from.
    contacts: Option<Arc<(crate::ospf::Topology, export::Private)>>,
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
    // All addresses are bound before anything is served, so a taken port fails the start.
    let mut servers = Vec::new();
    for addr in &web_cfg.listen {
        servers.push(Server::http(addr).map_err(|e| anyhow!("cannot listen on {addr}: {e}"))?);
    }
    let shown: Vec<String> = web_cfg.listen.iter().map(|a| format!("http://{a}")).collect();
    eprintln!("hkdiag web listening on {} (build {})", shown.join(", "), web::build_id());
    for m in cfg.mikrotik.iter().filter(|m| m.insecure_http) {
        eprintln!("warning: MikroTik {} is read over plain http: its login crosses the network unencrypted", m.url);
    }
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
        radio: Mutex::new(BTreeMap::new()),
        radio_fetch: Mutex::new(()),
        status_now: Mutex::new(Vec::new()),
        power: Mutex::new(crate::power::PowerState::default()),
        sources: Mutex::new(Sources::default()),
        speed_clients: Mutex::new(std::collections::HashMap::new()),
        history: Mutex::new(BTreeMap::new()),
        history_fetch: Mutex::new(()),
    });
    if let Some(p) = app.cfg.power.clone() {
        let app = Arc::clone(&app);
        thread::spawn(move || poll_power(&app, &p));
    }
    if app.cfg.userdb.is_some() {
        let app = Arc::clone(&app);
        let every = Duration::from_secs(web_cfg.status_poll_secs);
        thread::spawn(move || poll_status(&app, every));
    }
    // One accept loop per address; they share the app and the connection limit.
    thread::scope(|s| {
        for server in &servers {
            let (app, web_cfg) = (&app, &web_cfg);
            s.spawn(move || accept(server, app, web_cfg));
        }
    });
    Ok(())
}

/// Reads UserDB's status every `every`: keeps it for the overview and records
/// the outage episodes network monitoring measures. Visitors never trigger it.
fn poll_status(app: &App, every: Duration) {
    let Some(u) = app.cfg.userdb.as_ref() else { return };
    let mut failing = false;
    loop {
        match crate::userdb::UserDb::new(u).problems() {
            Ok(now) => {
                failing = false;
                lock(&app.sources).status = (Some(crate::now()), true);
                if let Some(store) = &app.reports {
                    let still_down = still_silent(app, store, &now);
                    if let Err(e) = lock(store).record_problems(&now, &still_down, crate::now()) {
                        eprintln!("warning: recording measured outages: {e:#}");
                    }
                }
                *lock(&app.status_now) = now;
            }
            Err(e) => {
                lock(&app.sources).status.1 = false;
                // Said once per failure streak, not every poll.
                if !failing {
                    eprintln!("warning: UserDB status: {}", crate::report::sanitize(&format!("{e:#}")));
                }
                failing = true;
            }
        }
        thread::sleep(every);
    }
}

/// Devices whose "doesn't answer" outage is open but which UserDB's status no
/// longer lists (it drops a device after a week without an answer): those
/// network monitoring still doesn't hear from. If it can't be asked, all of them.
fn still_silent(app: &App, store: &Mutex<reports::Store>, listed: &[crate::userdb::Problem]) -> Vec<(String, String)> {
    let open = match lock(store).open_unanswered() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("warning: reading open outages: {e:#}");
            return Vec::new();
        }
    };
    let dropped: Vec<(String, String)> = open.into_iter().filter(|(ap, ip)| !listed.iter().any(|p| p.ap == *ap && p.ip.to_string() == *ip)).collect();
    let ips: Vec<Ipv4Addr> = dropped.iter().filter_map(|(_, ip)| ip.parse().ok()).collect();
    if ips.is_empty() {
        return Vec::new();
    }
    match crate::sojka::ping_ips(&app.cfg.sojka, &ips) {
        Ok(health) => dropped
            .into_iter()
            .filter(|(_, ip)| {
                // Answering now: heard from within the last 10 minutes and not dead.
                let answers = ip.parse().ok().and_then(|a: Ipv4Addr| health.get(&a)).map_or(false, |h| {
                    matches!(h.health(Duration::from_secs(600)), crate::sojka::Health::Ok | crate::sojka::Health::Slow | crate::sojka::Health::Lossy)
                });
                !answers
            })
            .collect(),
        Err(e) => {
            eprintln!("warning: asking network monitoring about silent devices: {}", crate::report::sanitize(&format!("{e:#}")));
            dropped
        }
    }
}

/// HKDIAG_TRACE_REQUESTS=1 logs each request when it arrives and when it's
/// answered (the test harness uses it to explain stalls; paths only).
fn tracing() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("HKDIAG_TRACE_REQUESTS").is_some_and(|v| v == "1"))
}

/// Reads the power monitor every `poll_secs` for the outage overview.
fn poll_power(app: &App, cfg: &crate::config::PowerConfig) {
    let mut failing = false;
    loop {
        match crate::power::fetch(cfg) {
            Ok(state) => {
                failing = false;
                *lock(&app.power) = state;
                lock(&app.sources).power = (Some(crate::now()), true);
            }
            Err(e) => {
                lock(&app.sources).power.1 = false;
                if !failing {
                    eprintln!("warning: power monitor: {}", crate::report::sanitize(&format!("{e:#}")));
                }
                failing = true;
            }
        }
        thread::sleep(Duration::from_secs(cfg.poll_secs));
    }
}

fn accept(server: &Server, app: &Arc<App>, web_cfg: &Arc<WebConfig>) {
    for req in server.incoming_requests() {
        let traced = tracing().then(|| (format!("{} {}", req.method(), req.url().split('?').next().unwrap_or("")), Instant::now()));
        if let Some((what, _)) = &traced {
            eprintln!("trace: {what} arrived ({} handlers busy)", app.connections.load(Ordering::SeqCst));
        }
        if app.connections.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTION_THREADS {
            app.connections.fetch_sub(1, Ordering::SeqCst);
            let _ = req.respond(text(503, "server busy"));
            continue;
        }
        let app = Arc::clone(app);
        let web_cfg = Arc::clone(web_cfg);
        thread::spawn(move || {
            // Released when the thread ends, even if handle() panics.
            let _slot = SlotGuard(&app.connections);
            let access = web_cfg.access_log.then(|| {
                let headers = headers_of(&req);
                let who = client_ip(req.remote_addr().map(|a| a.ip()), &headers, &web_cfg);
                let path: String = req.url().split('?').next().unwrap_or("").chars().take(120).map(|c| if c.is_control() { '?' } else { c }).collect();
                (format!("{} {path}", req.method()), who, Instant::now())
            });
            let status = handle(&app, &web_cfg, req);
            if let Some((what, who, t)) = access {
                // After answering: the first requests are what builds the map it's judged by.
                eprintln!("access: {what} {status} {}ms {}", t.elapsed().as_millis(), network_of(&app, who));
            }
            if let Some((what, t)) = traced {
                eprintln!("trace: {what} answered in {} ms", t.elapsed().as_millis());
            }
        });
    }
}

/// Which network a client is on, for the access log: never its address.
fn network_of(app: &App, ip: Option<IpAddr>) -> &'static str {
    let Some(ip) = ip else { return "unknown" };
    if ip.is_loopback() {
        return "local";
    }
    let in_pool = |p: &String| -> bool {
        let Some((a, l)) = p.split_once('/') else { return false };
        let Ok(len) = l.parse::<u8>() else { return false };
        match (a.parse::<IpAddr>(), ip) {
            (Ok(IpAddr::V4(net)), IpAddr::V4(x)) => crate::verified::in_prefix(u32::from(net), len, u32::from(x)),
            (Ok(IpAddr::V6(net)), IpAddr::V6(x)) if len <= 128 => {
                let mask = if len == 0 { 0 } else { u128::MAX << (128 - u32::from(len)) };
                (u128::from(net) & mask) == (u128::from(x) & mask)
            }
            _ => false,
        }
    };
    if app.cfg.network.vpn.iter().any(in_pool) {
        return "vpn";
    }
    // The last map built; a request never waits for a new one.
    let Some(c) = lock(&app.snapshot).contacts.clone() else { return "unknown" };
    let ours = match ip {
        IpAddr::V4(a) => c.0.owner_router(a).0.is_some(),
        IpAddr::V6(a) => c.0.owner_router6(a).is_some(),
    };
    if ours { "hkfree" } else { "other" }
}

/// Sends a response; returns its status.
fn send<R: Read>(req: Request, r: Response<R>) -> u16 {
    let status = r.status_code().0;
    let _ = req.respond(r);
    status
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
        ("Permissions-Policy", "camera=(), microphone=(), geolocation=()"),
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
pub fn client_ip(peer: Option<IpAddr>, headers: &[(String, String)], web: &WebConfig) -> Option<IpAddr> {
    use crate::verified::{ClientHeader, HeaderKind};
    let peer = peer?.to_canonical();
    let trusted = web.trusted_proxies.iter().any(|p| p.to_canonical() == peer);
    // Only a trusted proxy's headers are read at all. Plain Rust only recognises
    // each header and parses its value; which one decides is proven
    // (verified::client_from_headers).
    let parse = |v: &str| v.trim().parse::<IpAddr>().ok().map(|a| a.to_canonical());
    let found: Vec<ClientHeader<IpAddr>> = if !trusted {
        Vec::new()
    } else {
        headers
            .iter()
            .filter_map(|(k, v)| {
                if k.eq_ignore_ascii_case("X-Forwarded-For") {
                    // This header's last comma-separated entry.
                    Some(ClientHeader { kind: HeaderKind::ForwardedFor, addr: v.rsplit(',').next().and_then(parse) })
                } else if k.eq_ignore_ascii_case("X-Real-IP") {
                    Some(ClientHeader { kind: HeaderKind::RealIp, addr: parse(v) })
                } else {
                    None
                }
            })
            .collect()
    };
    crate::verified::client_from_headers(peer, trusted, web.client_ip_header == ClientIpHeader::XForwardedFor, &found)
}

fn headers_of(req: &Request) -> Vec<(String, String)> {
    req.headers().iter().map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string())).collect()
}

fn json_body(status: u16, v: &serde_json::Value) -> Response<std::io::Cursor<Vec<u8>>> {
    secured(
        Response::from_string(v.to_string())
            .with_status_code(status)
            .with_header(header("Content-Type", "application/json"))
            .with_header(header("Cache-Control", "no-store")),
    )
}

/// Answers one request; returns the status sent (for the access log).
fn handle(app: &App, web_cfg: &WebConfig, mut req: Request) -> u16 {
    let path = req.url().split('?').next().unwrap_or("/").to_string();
    if path == "/api/speeds" {
        let resp = if *req.method() == Method::Post {
            post_speeds(app, web_cfg, &mut req)
        } else {
            text(405, "method not allowed")
        };
        return send(req, resp);
    }
    // The other write endpoint.
    if path == "/api/reports" {
        let resp = if *req.method() == Method::Post {
            post_reports(app, web_cfg, &mut req)
        } else {
            text(405, "method not allowed")
        };
        return send(req, resp);
    }
    if !matches!(req.method(), Method::Get | Method::Head) {
        return send(req, text(405, "method not allowed"));
    }
    match path.as_str() {
        "/api/outages" | "/api/v1/outages" => {
            let resp = match outages_body(app) {
                Ok(v) => json_body(200, &v),
                Err(e) => {
                    eprintln!("outages: {e:#}");
                    text(500, "outages unavailable")
                }
            };
            send(req, public_api(&path, resp))
        }
        "/api/v1/measured" | "/api/v1/reports" => {
            let since = since_param(req.url());
            let resp = match &app.reports {
                None => json_body(200, &serde_json::json!({ "measured": [], "reports": [] })),
                Some(store) => {
                    let st = lock(store);
                    let body = if path == "/api/v1/measured" {
                        st.measured_since(since, 5000).map(|m| serde_json::json!({ "measured": m }))
                    } else {
                        st.public_recent_ended(since, 5000).map(|r| serde_json::json!({ "reports": r }))
                    };
                    drop(st);
                    match body {
                        Ok(v) => json_body(200, &v),
                        Err(e) => {
                            eprintln!("{path}: {e:#}");
                            text(500, "unavailable")
                        }
                    }
                }
            };
            send(req, public_api(&path, resp))
        }
        "/api/v1/network" => match snapshot(app, web_cfg) {
            Ok(bytes) => {
                let r = Response::new(200.into(), vec![header("Content-Type", "application/json"), header("Cache-Control", "no-cache")], &bytes[..], Some(bytes.len()), None);
                send(req, public_api(&path, secured(r)))
            }
            Err(e) => {
                eprintln!("network: {e:#}");
                send(req, public_api(&path, text(502, "network state temporarily unavailable")))
            }
        },
        "/.well-known/api-catalog" => {
            // RFC 9727: where the API's description is.
            let body = serde_json::json!({ "linkset": [{
                "anchor": "/api/v1/",
                "service-desc": [{ "href": "/api/openapi.json", "type": "application/openapi+json" }],
                "service-doc": [{ "href": "/", "type": "text/html" }],
            }] });
            let r = Response::from_string(body.to_string())
                .with_header(header("Content-Type", "application/linkset+json"))
                .with_header(header("Cache-Control", "no-cache"));
            send(req, public_api(&path, secured(r)))
        }
        "/api/openapi.json" => {
            let r = Response::from_data(web::OPENAPI.as_bytes())
                .with_header(header("Content-Type", "application/openapi+json"))
                .with_header(header("Cache-Control", "no-cache"));
            send(req, public_api(&path, secured(r)))
        }
        "/api/contact" => {
            let headers = headers_of(&req);
            let ip = client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg)
                .filter(|i| !i.is_loopback() && !i.is_unspecified());
            let resp = match snapshot(app, web_cfg) {
                Ok(_) => {
                    let c = lock(&app.snapshot).contacts.clone();
                    let router = match (ip, &c) {
                        (Some(IpAddr::V4(a)), Some(c)) => c.0.owner_router(a).0,
                        (Some(IpAddr::V6(a)), Some(c)) => c.0.owner_router6(a),
                        _ => None,
                    };
                    let admins = match (router, &c) {
                        (Some(r), Some(c)) => c.1.contacts.get(&r).cloned().unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    json_body(200, &serde_json::json!({ "router": router, "admins": admins }))
                }
                Err(e) => {
                    eprintln!("contact: {e:#}");
                    text(502, "network state temporarily unavailable")
                }
            };
            send(req, resp)
        }
        "/api/v1/speeds" => {
            let since = speeds_since(req.url());
            let resp = match &app.reports {
                None => json_body(200, &serde_json::json!({ "speeds": [], "min_count": reports::MIN_SPEEDS_SHOWN })),
                Some(store) => match lock(store).speeds_by_ap(since) {
                    Ok(g) => json_body(200, &serde_json::json!({ "speeds": g, "since": since, "min_count": reports::MIN_SPEEDS_SHOWN })),
                    Err(e) => {
                        eprintln!("speeds: {e:#}");
                        text(500, "unavailable")
                    }
                },
            };
            send(req, public_api(&path, resp))
        }
        "/api/v1/history" => {
            let resp = router_history(app, web_cfg, req.url());
            send(req, public_api(&path, resp))
        }
        "/api/traffic" => {
            let resp = own_traffic(app, web_cfg, &req);
            send(req, resp)
        }
        "/api/signal" => {
            let resp = signal_for(app, web_cfg, &req);
            send(req, resp)
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
            send(req, secured(r))
        }
        "/topology.json" => match snapshot(app, web_cfg) {
            Ok(bytes) => {
                let r = Response::new(
                    200.into(),
                    vec![header("Content-Type", "application/json"), header("Cache-Control", "no-cache")],
                    &bytes[..],
                    Some(bytes.len()),
                    None,
                );
                send(req, secured(r))
            }
            Err(e) => {
                // Details go to the server log only; they may mention internal hosts.
                eprintln!("topology snapshot: {e:#}");
                send(req, text(502, "network state temporarily unavailable"))
            }
        },
        p if p.starts_with("/models/") => {
            let resp = match web_cfg.models_dir.as_deref().and_then(|d| model_file(d, &p["/models/".len()..])) {
                Some((file, len, mime)) => secured(Response::new(
                    200.into(),
                    // Files are versioned by their path; WebLLM caches them itself.
                    vec![header("Content-Type", mime), header("Cache-Control", "public, max-age=31536000, immutable")],
                    file,
                    Some(len),
                    None,
                ))
                .boxed(),
                None => text(404, "not found").boxed(),
            };
            send(req, resp)
        }
        "/sw.js" => {
            let r = Response::new(
                200.into(),
                vec![header("Content-Type", "text/javascript; charset=utf-8"), header("Cache-Control", "no-cache")],
                &app.service_worker[..],
                Some(app.service_worker.len()),
                None,
            );
            send(req, secured(r))
        }
        p => {
            let p = if p == "/" { "/index.html" } else { p };
            match ASSETS.iter().find(|a| a.path == p) {
                Some(a) => {
                    let r = Response::new(
                        200.into(),
                        // The service worker does the caching; always revalidate.
                        vec![header("Content-Type", a.mime), header("Cache-Control", "no-cache")],
                        a.body,
                        Some(a.body.len()),
                        None,
                    );
                    send(req, secured(r))
                }
                None => {
                    send(req, text(404, "not found"))
                }
            }
        }
    }
}

/// The outage overview: member reports (without addresses) and measured
/// outages of the last week, what monitoring sees now, and the power situation.
fn outages_body(app: &App) -> Result<serde_json::Value> {
    let now: Vec<serde_json::Value> = lock(&app.status_now)
        .iter()
        .map(|p| serde_json::json!({ "ap": p.ap, "ip": p.ip, "label": p.label, "kind": p.kind, "loss": p.loss }))
        .collect();
    let power = serde_json::to_value(&*lock(&app.power))?;
    let week = crate::now() - 7 * 24 * 3600;
    let sources = {
        let s = lock(&app.sources);
        let one = |configured: bool, (at, ok): (Option<i64>, bool)| if configured { serde_json::json!({ "read_at": at, "ok": ok }) } else { serde_json::Value::Null };
        serde_json::json!({ "status": one(app.cfg.userdb.is_some(), s.status), "power": one(app.cfg.power.is_some(), s.power) })
    };
    Ok(match &app.reports {
        None => serde_json::json!({ "enabled": false, "outages": [], "now": now, "measured": [], "power": power, "sources": sources }),
        Some(store) => {
            // One lock for both reads, released before responding.
            let st = lock(store);
            let list = st.public_recent(week, 200)?;
            let measured = st.measured_since(week, 200)?;
            serde_json::json!({ "enabled": true, "outages": list, "now": now, "measured": measured, "power": power, "sources": sources })
        }
    })
}

/// `since` from the query (Unix seconds): default a week ago, at most 90 days back.
fn since_param(url: &str) -> i64 {
    let now = crate::now();
    let asked = url
        .split_once('?')
        .and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("since=")))
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(now - 7 * 24 * 3600);
    asked.clamp(now - 90 * 24 * 3600, now)
}

/// The public API (`/api/v1/*`, its description and catalog) may be read from
/// other sites; answers about the requesting device never are.
fn public_api<R: Read>(path: &str, r: Response<R>) -> Response<R> {
    if path.starts_with("/api/v1/") || path == "/api/openapi.json" || path == "/.well-known/api-catalog" {
        r.with_header(header("Access-Control-Allow-Origin", "*"))
    } else {
        r
    }
}

/// The requesting device's own antenna signal, from its AP's radio data.
fn signal_for(app: &App, web_cfg: &WebConfig, req: &Request) -> Response<std::io::Cursor<Vec<u8>>> {
    let headers = headers_of(req);
    let ip = client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg).filter(|i| !i.is_loopback() && !i.is_unspecified());
    if let Err(e) = snapshot(app, web_cfg) {
        eprintln!("signal: {e:#}");
        return text(502, "network state temporarily unavailable");
    }
    let Some(c) = lock(&app.snapshot).contacts.clone() else {
        return text(502, "network state temporarily unavailable");
    };
    let router = match ip {
        Some(IpAddr::V4(a)) => c.0.owner_router(a).0,
        Some(IpAddr::V6(a)) => c.0.owner_router6(a),
        None => None,
    };
    let Some(ap) = router.and_then(|r| c.1.router_ap.get(&r).copied()) else {
        return json_body(200, &serde_json::json!({ "found": false, "reason": "no_ap" }));
    };
    // Antennas are known by their IPv4 address.
    let Some(IpAddr::V4(who)) = ip else {
        return json_body(200, &serde_json::json!({ "found": false, "reason": "ipv6" }));
    };
    match radio(app, web_cfg, ap) {
        Ok(data) => {
            let mine = signal::mine(&data.0, who, &data.1);
            json_body(200, &serde_json::json!({ "found": !mine.is_empty(), "stations": mine }))
        }
        Err(e) => {
            eprintln!("signal: AP {ap}: {e}");
            text(502, "radio data temporarily unavailable")
        }
    }
}

/// The network's own addresses (routers, AP devices), from the current map.
fn infrastructure(app: &App, web_cfg: &WebConfig) -> Result<Arc<(crate::ospf::Topology, export::Private)>> {
    snapshot(app, web_cfg)?;
    lock(&app.snapshot).contacts.clone().context("no network map yet")
}

/// An address from the query (`ip=`, `via=`): a plain IPv4 address, nothing else.
fn addr_param(url: &str, name: &str) -> Option<Ipv4Addr> {
    let key = format!("{name}=");
    url.split_once('?')?.1.split('&').find_map(|kv| kv.strip_prefix(key.as_str()))?.parse().ok()
}

/// Where a router can be measured, best first: its router ID, its address on
/// the link from `via` (the router before it on the path), its other link
/// addresses. Some routers don't answer pings at their router ID.
fn measurable_addresses(topo: &crate::ospf::Topology, ip: Ipv4Addr, via: Option<Ipv4Addr>) -> Vec<Ipv4Addr> {
    let mut out = vec![ip];
    let Some(r) = topo.routers.get(&ip) else { return out };
    let on_path = via.and_then(|v| {
        r.edges.iter().find(|e| e.to == v).and_then(|e| e.local_if).or_else(|| {
            topo.routers.get(&v)?.edges.iter().find(|e| e.to == ip)?.remote_if
        })
    });
    for a in on_path.into_iter().chain(r.interfaces.iter().copied()) {
        if !out.contains(&a) {
            out.push(a);
        }
    }
    // Each try is a query to monitoring; a few are enough to find one it measures.
    out.truncate(4);
    out
}

/// A router's or AP device's response time and packet loss (public).
fn router_history(app: &App, web_cfg: &WebConfig, url: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(cfg) = &app.cfg.history else {
        return text(404, "no measurement history on this server");
    };
    let Some(ip) = addr_param(url, "ip") else {
        return text(400, "ip must be an IPv4 address");
    };
    let net = match infrastructure(app, web_cfg) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("history: {e:#}");
            return text(502, "network state temporarily unavailable");
        }
    };
    // A member's device has no public history.
    if !net.1.infra.contains(&ip) {
        return text(404, "no measurement history of this address");
    }
    let range = history::Range::from_query(url);
    let mut found = None;
    for a in measurable_addresses(&net.0, ip, addr_param(url, "via")) {
        match series(app, cfg, history::Kind::Ping, a, range) {
            Ok(s) if s.measured() => {
                found = Some(s);
                break;
            }
            // Nothing measured: the router ID's empty series is the answer if no other address has one.
            Ok(s) => {
                found.get_or_insert(s);
            }
            Err(e) => {
                eprintln!("history {a}: {e}");
                return text(502, "monitoring's history temporarily unavailable");
            }
        }
    }
    let Some(s) = found else { return text(502, "monitoring's history temporarily unavailable") };
    let mut v = s.to_json();
    v["ip"] = serde_json::json!(ip);
    v["measured_ip"] = serde_json::json!(s.ip);
    json_body(200, &v)
}

/// The requesting device's own traffic: never another address's.
fn own_traffic(app: &App, web_cfg: &WebConfig, req: &Request) -> Response<std::io::Cursor<Vec<u8>>> {
    let Some(cfg) = app.cfg.history.as_ref().filter(|h| h.traffic_datasource.is_some()) else {
        return json_body(200, &serde_json::json!({ "found": false, "reason": "disabled" }));
    };
    let headers = headers_of(req);
    let ip = client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg).filter(|i| !i.is_loopback() && !i.is_unspecified());
    let Some(IpAddr::V4(who)) = ip else {
        let reason = if ip.is_some() { "ipv6" } else { "not_hkfree" };
        return json_body(200, &serde_json::json!({ "found": false, "reason": reason }));
    };
    let net = match infrastructure(app, web_cfg) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("traffic: {e:#}");
            return text(502, "network state temporarily unavailable");
        }
    };
    if net.0.owner_router(who).0.is_none() {
        return json_body(200, &serde_json::json!({ "found": false, "reason": "not_hkfree" }));
    }
    // The network's own address (e.g. a router that translates members'
    // addresses) carries other members' traffic.
    if net.1.infra.contains(&who) {
        return json_body(200, &serde_json::json!({ "found": false, "reason": "shared" }));
    }
    match series(app, cfg, history::Kind::Traffic, who, history::Range::from_query(req.url())) {
        Ok(s) => {
            let mut v = s.to_json();
            v["found"] = serde_json::Value::Bool(true);
            json_body(200, &v)
        }
        Err(e) => {
            // Not the address: the log is no place for members' traffic lookups
            // (Grafana's errors may quote the query, address included).
            eprintln!("traffic: {}", e.replace(&who.to_string(), "<member>"));
            text(502, "monitoring's history temporarily unavailable")
        }
    }
}

/// One series, read at most once per `cache_secs`.
fn series(app: &App, cfg: &crate::config::HistoryConfig, kind: history::Kind, ip: Ipv4Addr, range: history::Range) -> std::result::Result<Arc<history::Series>, String> {
    let key = (kind, ip, range);
    let max_age = Duration::from_secs(cfg.cache_secs);
    let cached = |app: &App| lock(&app.history).get(&key).filter(|(at, _)| at.elapsed() < max_age).map(|(_, r)| r.clone());
    if let Some(r) = cached(app) {
        return r;
    }
    let _one_at_a_time = lock(&app.history_fetch);
    if let Some(r) = cached(app) {
        return r;
    }
    let r = history::fetch(cfg, kind, ip, range, crate::now()).map(Arc::new).map_err(|e| crate::report::sanitize(&format!("{e:#}")));
    let mut c = lock(&app.history);
    if c.len() >= HISTORY_CACHE_MAX {
        c.retain(|_, (at, _)| at.elapsed() < max_age);
        if c.len() >= HISTORY_CACHE_MAX {
            c.clear();
        }
    }
    c.insert(key, (Instant::now(), r.clone()));
    r
}

/// One AP's radio data, read at most once per `signal_cache_secs`.
fn radio(app: &App, web_cfg: &WebConfig, ap: u32) -> std::result::Result<Arc<RadioData>, String> {
    let max_age = Duration::from_secs(web_cfg.signal_cache_secs);
    let cached = |app: &App| lock(&app.radio).get(&ap).filter(|(at, _)| at.elapsed() < max_age).map(|(_, r)| r.clone());
    if let Some(r) = cached(app) {
        return r;
    }
    let _one_at_a_time = lock(&app.radio_fetch);
    if let Some(r) = cached(app) {
        return r;
    }
    let read = || -> Result<RadioData> {
        let u = app.cfg.userdb.as_ref().context("no [userdb] in the config")?;
        let db = crate::userdb::UserDb::new(u);
        let stations = signal::stations(&db.wewimo(ap)?);
        Ok((stations, db.devices(true, Some(ap))?))
    };
    let r = read().map(Arc::new).map_err(|e| crate::report::sanitize(&format!("{e:#}")));
    lock(&app.radio).insert(ap, (Instant::now(), r.clone()));
    r
}

/// `since` for speeds: default a week ago, at most 180 days back.
fn speeds_since(url: &str) -> i64 {
    let now = crate::now();
    url.split_once('?')
        .and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("since=")))
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(now - 7 * 24 * 3600)
        .clamp(now - 180 * 24 * 3600, now)
}

/// More speeds per client and hour than this aren't taken.
const SPEEDS_PER_CLIENT_PER_HOUR: usize = 10;

/// Stores speed test results from the app: never with the member's address.
/// Only from the hkfree.org network itself (elsewhere the speed says nothing
/// about the AP), and only for an AP router of the network map.
fn post_speeds(app: &App, web_cfg: &WebConfig, req: &mut Request) -> Response<std::io::Cursor<Vec<u8>>> {
    let headers = headers_of(req);
    let get = |n: &str| headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(n)).map(|(_, v)| v.as_str());
    if !get("Content-Type").map_or(false, |c| c.trim_start().starts_with("application/json")) {
        return text(415, "application/json required");
    }
    if get("Origin").is_some_and(|o| !same_origin(o, get("Host"), &web_cfg.origins)) {
        eprintln!("speeds: refused, from another site");
        return text(403, "cross-origin speeds are not accepted");
    }
    let Some(store) = &app.reports else {
        return text(503, "this server keeps no speeds");
    };
    let mut body = Vec::new();
    if req.as_reader().take(reports::MAX_BODY as u64 + 1).read_to_end(&mut body).is_err() || body.len() > reports::MAX_BODY {
        return text(413, "too large");
    }
    let list = match serde_json::from_slice::<Vec<reports::SpeedIn>>(&body) {
        Ok(l) if l.len() <= reports::MAX_BATCH => l,
        Ok(_) => return text(413, "too many speeds"),
        Err(e) => {
            eprintln!("speeds: refused, not a list of speeds: {}", crate::report::sanitize(&e.to_string()));
            return text(400, "invalid speeds");
        }
    };
    let client = client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg);
    let net = network_of(app, client);
    if net != "hkfree" {
        eprintln!("speeds: refused {} from {net}: only the hkfree.org network's speeds say something about its APs", list.len());
        return json_body(200, &serde_json::json!({ "accepted": 0, "received": list.len() }));
    }
    let Some(who) = client else { return text(400, "unknown client") };
    let now = crate::now();
    let budget = {
        let mut c = lock(&app.speed_clients);
        if c.len() > 10_000 {
            c.retain(|_, t| t.iter().any(|&x| x > now - 3600));
        }
        let times = c.entry(who).or_default();
        times.retain(|&t| t > now - 3600);
        let budget = SPEEDS_PER_CLIENT_PER_HOUR.saturating_sub(times.len()).min(list.len());
        times.extend(std::iter::repeat(now).take(budget));
        budget
    };
    let Some(net_map) = lock(&app.snapshot).contacts.clone() else {
        return text(502, "network state temporarily unavailable");
    };
    let (known, unknown): (Vec<_>, Vec<_>) = list.into_iter().take(budget).partition(|s| net_map.0.routers.contains_key(&s.ap));
    let outcome = lock(store).insert_speeds(&known, |r| net_map.1.ap_names.get(&r).cloned(), now);
    match outcome {
        Ok((stored, refused)) => {
            eprintln!("speeds: {} received from hkfree, {stored} stored, {} refused{}", known.len() + unknown.len(), refused.len() + unknown.len(), if unknown.is_empty() { "" } else { " (not an AP router of the map)" });
            json_body(200, &serde_json::json!({ "accepted": stored, "received": known.len() + unknown.len() }))
        }
        Err(e) => {
            eprintln!("speeds: {e:#}");
            text(500, "cannot store speeds")
        }
    }
}

/// Stores outage reports from the app. Only same-origin JSON is accepted:
/// a cross-site page cannot send `application/json` without a CORS preflight
/// (which this server never approves), and a mismatching Origin is refused.
fn post_reports(app: &App, web_cfg: &WebConfig, req: &mut Request) -> Response<std::io::Cursor<Vec<u8>>> {
    let headers = headers_of(req);
    let get = |n: &str| headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(n)).map(|(_, v)| v.as_str());
    if !get("Content-Type").map_or(false, |c| c.trim_start().starts_with("application/json")) {
        eprintln!("reports: refused, not JSON");
        return text(415, "application/json required");
    }
    if let Some(origin) = get("Origin") {
        if !same_origin(origin, get("Host"), &web_cfg.origins) {
            eprintln!("reports: refused, from another site");
            return text(403, "cross-origin reports are not accepted");
        }
    }
    let Some(store) = &app.reports else {
        return text(503, "reports are disabled on this server");
    };
    let mut body = Vec::new();
    if req.as_reader().take(reports::MAX_BODY as u64 + 1).read_to_end(&mut body).is_err() {
        return text(400, "cannot read body");
    }
    if body.len() > reports::MAX_BODY {
        return text(413, "too large");
    }
    let list = match serde_json::from_slice::<Vec<ReportIn>>(&body) {
        Ok(l) => l,
        Err(e) => {
            // serde says where and what kind of error, not the value.
            eprintln!("reports: refused, not a list of reports: {}", crate::report::sanitize(&e.to_string()));
            return text(400, "invalid reports");
        }
    };
    if list.len() > reports::MAX_BATCH {
        return text(413, "too many reports");
    }
    let client = client_ip(req.remote_addr().map(|a| a.ip()), &headers, web_cfg);
    // A trial report (the admin's check) goes all the way to the database and is rolled back.
    let trial = get("X-Hkdiag-Trial") == Some("1");
    let outcome = lock(store).submit(client, &list, crate::now(), !trial);
    match outcome {
        Ok(o) => {
            let mut why: Vec<&str> = o.rejected.iter().map(String::as_str).collect();
            why.dedup();
            // The client's network only: the log is no place for members' addresses.
            eprintln!(
                "reports{}: {} received from {}, {} stored, {} already stored, {} refused{}{}",
                if trial { " (trial, nothing kept)" } else { "" },
                list.len(),
                network_of(app, client),
                o.stored,
                o.duplicates,
                o.rejected.len(),
                if why.is_empty() { "" } else { ": " },
                why.join("; "),
            );
            if trial {
                json_body(200, &serde_json::json!({ "trial": true, "received": list.len(), "would_store": o.stored, "already_stored": o.duplicates, "refused": o.rejected }))
            } else {
                json_body(200, &serde_json::json!({ "accepted": o.stored, "received": list.len() }))
            }
        }
        Err(e) => {
            eprintln!("reports: {e:#}");
            text(500, "cannot store reports")
        }
    }
}

/// A mirrored model file: only plain names (letters, digits, `._-`) in each
/// path segment, no hidden files, and the result must stay inside `dir`.
fn model_file(dir: &std::path::Path, rel: &str) -> Option<(std::fs::File, usize, &'static str)> {
    // Each segment proven safe (verified::segment_ok): no `.`, `..`, hidden or odd names.
    if rel.len() > 512 || !rel.split('/').all(|seg| crate::verified::segment_ok(seg.as_bytes())) {
        return None;
    }
    let root = dir.canonicalize().ok()?;
    let path = root.join(rel).canonicalize().ok()?;
    if !path.starts_with(&root) {
        return None;
    }
    let file = std::fs::File::open(&path).ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("json") => "application/json",
        Some("wasm") => "application/wasm",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    };
    Some((file, usize::try_from(meta.len()).ok()?, mime))
}

/// A page may send reports if it comes from this server: its origin's host is
/// the request's Host, or the origin is one of the configured public origins.
fn same_origin(origin: &str, host: Option<&str>, origins: &[String]) -> bool {
    let origin_host = origin.split_once("://").map_or(origin, |(_, h)| h);
    Some(origin_host) == host || origins.iter().any(|o| o.eq_ignore_ascii_case(origin))
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
        let fresh = c.ok.as_ref().map_or(false, |(at, _)| at.elapsed() < max_age);
        let backoff = c.failed_at.map_or(false, |f| f.elapsed() < TOPOLOGY_ERROR_BACKOFF);
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
    let (snap, private) = export::build_with_contacts(&app.cfg, &topo, &state_snapshot);
    let bytes = Arc::new(serde_json::to_vec(&snap)?);
    let mut c = lock(&app.snapshot);
    c.ok = Some((Instant::now(), Arc::clone(&bytes)));
    c.contacts = Some(Arc::new((topo, private)));
    c.failed_at = None;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn web() -> WebConfig {
        WebConfig { trusted_proxies: vec!["127.0.0.1".parse().unwrap()], ..Default::default() }
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
        assert_eq!(client_ip(Some(proxy), &hdr, &web()), Some("10.107.5.130".parse().unwrap()));
    }

    #[test]
    fn forwarded_for_is_ignored_and_duplicates_refused() {
        let proxy: IpAddr = "127.0.0.1".parse().unwrap();
        let xff = vec![h("X-Forwarded-For", "10.107.99.5")];
        assert_eq!(client_ip(Some(proxy), &xff, &web()), None);
        assert_eq!(client_ip(Some(proxy), &[], &web()), None);
        let dup = vec![h("X-Real-IP", "10.107.99.5"), h("x-real-ip", "10.107.5.130")];
        assert_eq!(client_ip(Some(proxy), &dup, &web()), None);
        let bad = vec![h("X-Real-IP", "not-an-ip")];
        assert_eq!(client_ip(Some(proxy), &bad, &web()), None);
    }

    #[test]
    fn forwarded_for_mode_takes_the_entry_the_proxy_appended() {
        let w = WebConfig { client_ip_header: ClientIpHeader::XForwardedFor, ..web() };
        let proxy: IpAddr = "127.0.0.1".parse().unwrap();
        // The client forged both headers; Apache appended the real address.
        let forged = vec![h("X-Real-IP", "10.107.77.5"), h("X-Forwarded-For", "10.107.77.5, 10.107.10.146")];
        assert_eq!(client_ip(Some(proxy), &forged, &w), Some("10.107.10.146".parse().unwrap()));
        let two_lines = vec![h("X-Forwarded-For", "10.107.77.5"), h("x-forwarded-for", "2a01:168:200:100::5")];
        assert_eq!(client_ip(Some(proxy), &two_lines, &w), Some("2a01:168:200:100::5".parse().unwrap()));
        assert_eq!(client_ip(Some(proxy), &[h("X-Real-IP", "10.107.77.5")], &w), None, "X-Real-IP isn't believed in this mode");
        assert_eq!(client_ip(Some(proxy), &[h("X-Forwarded-For", "10.107.77.5, junk")], &w), None);
        let direct: IpAddr = "10.107.9.9".parse().unwrap();
        assert_eq!(client_ip(Some(direct), &forged, &w), Some(direct), "headers from untrusted peers are ignored");
    }

    #[test]
    fn model_files_stay_inside_their_directory() {
        let dir = std::env::temp_dir().join(format!("hkdiag-models-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("m/resolve/main")).unwrap();
        std::fs::write(dir.join("m/resolve/main/mlc-chat-config.json"), b"{}").unwrap();
        std::fs::write(dir.join("m/resolve/main/params_shard_0.bin"), b"xx").unwrap();
        std::fs::write(dir.join(".secret"), b"no").unwrap();
        let (_, len, mime) = model_file(&dir, "m/resolve/main/mlc-chat-config.json").unwrap();
        assert_eq!((len, mime), (2, "application/json"));
        assert_eq!(model_file(&dir, "m/resolve/main/params_shard_0.bin").unwrap().2, "application/octet-stream");
        for bad in ["../etc/passwd", "m/../../x", ".secret", "m//resolve", "m/resolve/main", "m/%2e%2e/x", "/etc/passwd", ""] {
            assert!(model_file(&dir, bad).is_none(), "{bad}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn since_is_bounded_and_only_the_public_api_is_cross_origin() {
        let now = crate::now();
        assert!((since_param("/api/v1/measured") - (now - 7 * 86400)).abs() <= 1, "a week by default");
        assert!((since_param("/api/v1/measured?since=0") - (now - 90 * 86400)).abs() <= 1, "at most 90 days back");
        assert!((since_param("/api/v1/measured?since=99999999999") - now).abs() <= 1);
        let cors = |p: &str| public_api(p, text(200, "x")).headers().iter().any(|h| h.field.equiv("Access-Control-Allow-Origin"));
        assert!(cors("/api/v1/outages") && cors("/api/openapi.json") && cors("/.well-known/api-catalog"));
        assert!(!cors("/api/whoami") && !cors("/api/contact") && !cors("/api/signal") && !cors("/api/reports") && !cors("/api/traffic"));
        assert!(cors("/api/v1/history"));
        assert_eq!(addr_param("/api/v1/history?range=week&ip=10.107.76.1", "ip"), Some("10.107.76.1".parse().unwrap()));
        assert_eq!(addr_param("/api/v1/history?ip=10.107.99.1&via=10.107.76.1", "via"), Some("10.107.76.1".parse().unwrap()));
        for bad in ["/api/v1/history", "/api/v1/history?ip=10.107.76.1'", "/api/v1/history?ip=010.107.76.1", "/api/v1/history?ip=::1", "/api/v1/history?vip=10.107.76.1"] {
            assert_eq!(addr_param(bad, "ip"), None, "{bad}");
        }
    }

    #[test]
    fn reports_only_from_this_servers_pages() {
        let public = vec!["https://diag.hkfree.org".to_string()];
        assert!(same_origin("http://10.107.252.72:9000", Some("10.107.252.72:9000"), &[]));
        // Behind a proxy that doesn't pass Host on.
        assert!(same_origin("https://diag.hkfree.org", Some("127.0.0.1:9000"), &public));
        assert!(!same_origin("https://diag.hkfree.org", Some("127.0.0.1:9000"), &[]));
        assert!(!same_origin("https://evil.example", Some("127.0.0.1:9000"), &public));
        assert!(!same_origin("https://diag.hkfree.org.evil.example", Some("127.0.0.1:9000"), &public));
    }

    #[test]
    fn ipv4_mapped_addresses_are_normalised() {
        let mapped: IpAddr = "::ffff:127.0.0.1".parse().unwrap();
        let hdr = vec![h("X-Real-IP", "::ffff:10.107.5.130")];
        assert_eq!(client_ip(Some(mapped), &hdr, &web()), Some("10.107.5.130".parse().unwrap()));
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
