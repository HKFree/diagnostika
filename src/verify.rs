//! `hkdiag verify <url>`: checks a deployed hkdiag from the outside, as a
//! member's app and the public API's users see it. Read-only, except one trial
//! outage report, which the server checks all the way to its database and then
//! throws away. Run after each deployment and hourly (a systemd timer).

use crate::http;
use serde_json::{json, Value};
use std::time::Duration;

/// How one check went.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    Pass,
    Fail,
    /// Doesn't apply here (e.g. no proxy in front of a local server).
    Skip,
}

pub struct Check {
    pub name: &'static str,
    pub verdict: Verdict,
    pub detail: String,
}

struct Answer {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
    fn header(&self, name: &str) -> &str {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map_or("", |(_, v)| v.as_str())
    }
}

struct Client {
    base: String,
    origin: String,
    agent: ureq::Agent,
}

impl Client {
    fn call(&self, method: &str, path: &str, headers: &[(&str, &str)], body: Option<&Value>) -> Answer {
        let mut req = self.agent.request(method, &format!("{}{path}", self.base));
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let resp = match body {
            Some(b) => req.set("Content-Type", "application/json").send_string(&b.to_string()),
            None => req.call(),
        };
        let resp = match resp {
            Ok(r) | Err(ureq::Error::Status(_, r)) => r,
            Err(e) => return Answer { status: 0, headers: Vec::new(), body: crate::report::sanitize(&e.to_string()) },
        };
        let status = resp.status();
        let headers = resp.headers_names().into_iter().map(|n| (n.clone(), resp.header(&n).unwrap_or_default().to_string())).collect();
        let body = http::read_limited(resp, 8 * 1024 * 1024).unwrap_or_default();
        Answer { status, headers, body }
    }
    fn get(&self, path: &str) -> Answer {
        self.call("GET", path, &[], None)
    }
    fn report(&self, list: Value, origin: &str, trial: bool) -> Answer {
        let mut h = vec![("Origin", origin)];
        if trial {
            h.push(("X-Hkdiag-Trial", "1"));
        }
        self.call("POST", "/api/reports", &h, Some(&list))
    }
}

fn check(out: &mut Vec<Check>, name: &'static str, verdict: Verdict, detail: impl Into<String>) {
    out.push(Check { name, verdict, detail: detail.into() });
}

fn pass(ok: bool) -> Verdict {
    if ok { Verdict::Pass } else { Verdict::Fail }
}

/// Runs every check against `base` (e.g. "https://diag.hkfree.org").
pub fn run(base: &str, now: i64) -> Vec<Check> {
    let base = base.trim_end_matches('/').to_string();
    let origin = base.splitn(4, '/').take(3).collect::<Vec<_>>().join("/");
    let c = Client { base: base.clone(), origin: origin.clone(), agent: http::agent(Duration::from_secs(20)) };
    let mut out = Vec::new();

    // --- The app
    let index = c.get("/");
    check(&mut out, "the app loads, with its security headers", pass(index.status == 200 && index.body.contains("/app.js") && index.header("content-security-policy").contains("default-src 'none'")), format!("HTTP {}", index.status));
    let sw = c.get("/sw.js");
    let shell: Vec<String> = sw
        .body
        .split_once("const SHELL = [")
        .and_then(|(_, rest)| rest.split_once(']'))
        .map(|(list, _)| list.split(',').map(|p| p.trim().trim_matches('\'').to_string()).filter(|p| !p.is_empty()).collect())
        .unwrap_or_default();
    let missing: Vec<&String> = shell.iter().filter(|p| c.get(p).status != 200).collect();
    check(&mut out, "every file the app keeps offline is served", pass(sw.status == 200 && shell.len() > 5 && missing.is_empty()), if missing.is_empty() { format!("{} files", shell.len()) } else { format!("missing: {missing:?}") });

    // --- The client's address must come through the proxy
    let local = ["//127.0.0.1", "//localhost", "//[::1]"].iter().any(|h| base.contains(h));
    let who = c.get("/api/whoami").json();
    match who["ip"].as_str() {
        _ if local => check(&mut out, "the server sees the client's own address", Verdict::Skip, "a local server has no proxy in front"),
        Some(ip) => check(&mut out, "the server sees the client's own address", pass(ip != "127.0.0.1" && ip != "::1"), "an address came through"),
        None => check(&mut out, "the server sees the client's own address", Verdict::Fail, "no address: is the proxy's client address header configured?"),
    }

    // --- The network map
    let map = c.get("/topology.json");
    let topo = map.json();
    let routers = topo["routers"].as_object().map_or(0, |r| r.len());
    let age = topo["generated_at"].as_i64().map(|t| now - t);
    check(&mut out, "the network map is fresh", pass(map.status == 200 && routers > 0 && age.is_some_and(|a| a < 600)), format!("{routers} routers, {} s old", age.map_or("?".into(), |a| a.to_string())));
    let exit = topo["exits"][0]["target"].as_str().map(str::to_string);

    // --- Outage reporting
    let outages = c.get("/api/v1/outages");
    let o = outages.json();
    check(&mut out, "outage reports are enabled (the report database is open)", pass(outages.status == 200 && o["enabled"] == json!(true)), format!("enabled: {}", o["enabled"]));
    let empty = c.report(json!([]), &origin, false);
    check(&mut out, "the app's report request passes the proxy and the origin check", pass(empty.status == 200 && empty.json()["received"] == json!(0)), format!("HTTP {}", empty.status));
    let foreign = c.report(json!([]), "https://example.org", false);
    check(&mut out, "reports from another site are refused", pass(foreign.status == 403), format!("HTTP {}", foreign.status));
    let id = format!("trial-{now}-{:08x}", (now as u64).wrapping_mul(2654435761) & 0xffff_ffff);
    let report = json!({
        "id": id, "detected_at": now - 1800, "resolved_at": now - 600, "verdict": "backbone",
        "my_ip": null, "ap": null, "first_bad": exit, "last_ok": null,
        "path": exit.iter().collect::<Vec<_>>(), "snapshot_at": now - 1900,
    });
    // First a report no server would store: a server that doesn't know trial
    // reports refuses it without keeping anything, and the real one isn't sent.
    let mut probe = report.clone();
    probe["id"] = json!(format!("{id}-probe"));
    probe["verdict"] = json!("works");
    let p = c.report(json!([probe]), &origin, true);
    if p.json()["trial"] != json!(true) {
        check(&mut out, "a trial report would be stored (nothing is kept)", Verdict::Fail, "the server doesn't know trial reports (older version); nothing was sent");
    } else {
        let t = c.report(json!([report]), &origin, true);
        let b = t.json();
        check(&mut out, "a trial report would be stored (nothing is kept)", pass(t.status == 200 && b["would_store"] == json!(1)), format!("would store {}, refused: {}", b["would_store"], b["refused"]));
    }

    // --- The sources the server reads
    for (key, name) in [("status", "network monitoring's outages are being read"), ("power", "the power monitor is being read")] {
        if o["sources"].is_null() {
            check(&mut out, name, Verdict::Fail, "the server doesn't say how its sources are doing (older version)");
            continue;
        }
        let s = &o["sources"][key];
        if s.is_null() {
            check(&mut out, name, Verdict::Skip, "not configured");
            continue;
        }
        let fresh = s["read_at"].as_i64().is_some_and(|t| now - t < 900);
        check(&mut out, name, pass(s["ok"] == json!(true) && fresh), format!("last read ok: {}, {} s ago", s["ok"], s["read_at"].as_i64().map_or("never".into(), |t| (now - t).to_string())));
    }
    match exit {
        Some(exit) => {
            let h = c.get(&format!("/api/v1/history?ip={exit}"));
            let j = h.json();
            let n = j["rtt_ms"].as_array().map_or(0, |a| a.iter().filter(|v| !v.is_null()).count());
            if h.status == 404 {
                check(&mut out, "a router's measurement history comes from network monitoring", Verdict::Skip, "not configured");
            } else {
                check(&mut out, "a router's measurement history comes from network monitoring", pass(h.status == 200 && n > 0), format!("{exit} measured at {}, {n} points", j["measured_ip"].as_str().unwrap_or("?")));
            }
        }
        None => check(&mut out, "a router's measurement history comes from network monitoring", Verdict::Fail, "no internet exit in the map"),
    }

    // --- The public API
    let cat = c.get("/.well-known/api-catalog").json();
    let href = cat["linkset"][0]["service-desc"][0]["href"].as_str().unwrap_or("");
    let desc = if href.starts_with('/') { c.get(href).json() } else { Value::Null };
    check(&mut out, "the API catalog points to the API description", pass(desc["openapi"].as_str().is_some_and(|v| v.starts_with("3."))), if href.is_empty() { "no link".into() } else { href.to_string() });
    out
}

/// Prints the checks; true if none failed.
pub fn print(checks: &[Check]) -> bool {
    for c in checks {
        let v = match c.verdict {
            Verdict::Pass => "PASS",
            Verdict::Fail => "FAIL",
            Verdict::Skip => "SKIP",
        };
        println!("{v}  {}  ({})", c.name, c.detail);
    }
    let failed = checks.iter().filter(|c| c.verdict == Verdict::Fail).count();
    if failed == 0 {
        println!("all checks passed");
    } else {
        println!("{failed} check(s) failed");
    }
    failed == 0
}
