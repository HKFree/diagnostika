//! MikroTik routers (RouterOS 7) as extra views of the OSPF database, read
//! with a read-only account over the REST API.
//!
//! Each router shows the network from where it stands: inside areas the FRR
//! router can't see, and its own side of a split. Only the IPv4 (OSPFv2)
//! instances are read. A router that fails isn't asked again for
//! `retry_after_secs`, so a dead one doesn't slow every map build.

use crate::config::{Config, MikrotikConfig};
use crate::http;
use crate::lsdb::{parse_seq, ExternalLsa, Link, Lsdb, NetworkLsa, RouterLsa, SummaryLsa};
use anyhow::{Context, Result};
use base64::Engine as _;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use std::net::Ipv4Addr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The OSPF database of a large network is a few hundred kB.
const MAX_BODY: u64 = 32 * 1024 * 1024;

/// How a router answered at the last map build (shown in the snapshot).
#[derive(Clone, Debug, Serialize)]
pub struct SourceStatus {
    /// The router's OSPF router ID, once known.
    pub router: Option<Ipv4Addr>,
    pub ok: bool,
    /// Whether the router has an active default route (0.0.0.0/0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_route: Option<bool>,
    /// Number of router records it sees (all areas).
    pub routers: usize,
}

/// When each router (by URL) last failed.
static FAILED_AT: Mutex<BTreeMap<String, Instant>> = Mutex::new(BTreeMap::new());

/// Reads every configured router (in parallel) and returns the databases
/// that could be read, with a status per router.
pub fn read_all(cfg: &Config) -> (Vec<Lsdb>, Vec<SourceStatus>) {
    let results: Vec<(Option<Lsdb>, SourceStatus)> = std::thread::scope(|s| {
        let handles: Vec<_> = cfg.mikrotik.iter().map(|m| s.spawn(move || read_one(m))).collect();
        handles.into_iter().map(|h| h.join().unwrap_or((None, SourceStatus { router: None, ok: false, default_route: None, routers: 0 }))).collect()
    });
    let mut dbs = Vec::new();
    let mut statuses = Vec::new();
    for (db, st) in results {
        dbs.extend(db);
        statuses.push(st);
    }
    (dbs, statuses)
}

fn read_one(m: &MikrotikConfig) -> (Option<Lsdb>, SourceStatus) {
    let failed = SourceStatus { router: None, ok: false, default_route: None, routers: 0 };
    {
        let failures = FAILED_AT.lock().unwrap_or_else(|e| e.into_inner());
        if failures.get(&m.url).map_or(false, |t| t.elapsed() < Duration::from_secs(m.retry_after_secs)) {
            return (None, failed);
        }
    }
    match fetch(m) {
        Ok((db, default_route)) => {
            FAILED_AT.lock().unwrap_or_else(|e| e.into_inner()).remove(&m.url);
            let st = SourceStatus { router: db.root, ok: true, default_route, routers: db.routers.len() };
            (Some(db), st)
        }
        Err(e) => {
            // The URL is configuration, not a secret; the password never reaches the message.
            eprintln!("warning: MikroTik {}: {}", m.url, crate::report::sanitize(&format!("{e:#}")));
            FAILED_AT.lock().unwrap_or_else(|e| e.into_inner()).insert(m.url.clone(), Instant::now());
            (None, failed)
        }
    }
}

fn fetch(m: &MikrotikConfig) -> Result<(Lsdb, Option<bool>)> {
    let agent = http::agent(Duration::from_secs(m.timeout_secs));
    let auth = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", m.user, m.password.expose()))
    );
    let get = |path: &str| -> Result<Value> {
        let url = format!("{}/rest/{path}", m.url.trim_end_matches('/'));
        let resp = agent.get(&url).set("Authorization", &auth).call().with_context(|| format!("GET /rest/{path}"))?;
        let text = http::read_limited(resp, MAX_BODY)?;
        serde_json::from_str(&text).with_context(|| format!("/rest/{path}: invalid JSON"))
    };
    let instances = get("routing/ospf/instance")?;
    let areas = get("routing/ospf/area")?;
    let lsas = get("routing/ospf/lsa")?;
    // Optional: an older RouterOS or a narrower account may not show routes.
    let default_route = get("ip/route?dst-address=0.0.0.0/0")
        .ok()
        .map(|v| rows(&v).iter().any(|r| text(r, "active") == "true"));
    Ok((parse(&instances, &areas, &lsas), default_route))
}

fn rows(v: &Value) -> Vec<&Value> {
    v.as_array().map(|a| a.iter().collect()).unwrap_or_default()
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or_default()
}

/// RouterOS shows ages as seconds ("351") or as a duration ("5m51s").
fn parse_age(s: &str) -> u32 {
    if let Ok(n) = s.trim().parse() {
        return n;
    }
    let mut total = 0u32;
    let mut num = 0u32;
    for c in s.chars() {
        match c {
            '0'..='9' => num = num.saturating_mul(10).saturating_add(c as u32 - '0' as u32),
            'w' => (total, num) = (total.saturating_add(num * 604_800), 0),
            'd' => (total, num) = (total.saturating_add(num * 86_400), 0),
            'h' => (total, num) = (total.saturating_add(num * 3_600), 0),
            'm' => (total, num) = (total.saturating_add(num * 60), 0),
            's' => (total, num) = (total.saturating_add(num), 0),
            _ => {}
        }
    }
    total
}

/// `key=value` pairs of one line of a record body.
fn pairs(line: &str) -> HashMap<&str, &str> {
    line.split_whitespace().filter_map(|t| t.split_once('=')).collect()
}

fn mask_len(m: &str) -> Option<u8> {
    m.parse::<Ipv4Addr>().ok().map(|m| u32::from(m).count_ones() as u8)
}

/// The OSPFv2 records of a router's `/routing/ospf/lsa`, with area names
/// turned into area IDs through `/routing/ospf/area`.
pub fn parse(instances: &Value, areas: &Value, lsas: &Value) -> Lsdb {
    let v2: HashMap<&str, Option<Ipv4Addr>> = rows(instances)
        .into_iter()
        .filter(|i| text(i, "version") == "2" && text(i, "disabled") != "true")
        .map(|i| (text(i, "name"), text(i, "router-id").parse().ok()))
        .collect();
    let area_id: HashMap<(&str, &str), Ipv4Addr> = rows(areas)
        .into_iter()
        .filter_map(|a| Some(((text(a, "instance"), text(a, "name")), text(a, "area-id").parse().ok()?)))
        .collect();
    let mut db = Lsdb { root: v2.values().flatten().next().copied(), ..Default::default() };
    for l in rows(lsas) {
        let instance = text(l, "instance");
        if !v2.contains_key(instance) {
            continue;
        }
        let (Ok(id), Ok(originator)) = (text(l, "id").parse::<Ipv4Addr>(), text(l, "originator").parse::<Ipv4Addr>()) else { continue };
        let seq = parse_seq(text(l, "sequence"));
        let age = parse_age(text(l, "age"));
        let area = area_id.get(&(instance, text(l, "area"))).copied();
        let body = text(l, "body");
        let fields: HashMap<&str, &str> = body.lines().flat_map(pairs).collect();
        match (text(l, "type"), area) {
            ("router", Some(area)) => {
                let links = body
                    .lines()
                    .map(pairs)
                    .filter_map(|p| {
                        let metric = p.get("metric").and_then(|m| m.parse().ok()).unwrap_or(0);
                        let a = p.get("id").and_then(|v| v.parse::<Ipv4Addr>().ok());
                        let data = p.get("data").and_then(|v| v.parse::<Ipv4Addr>().ok());
                        match *p.get("type")? {
                            "stub" => Some(Link::Stub { net: a?, mask: data?, metric }),
                            "network" => Some(Link::Transit { dr: a?, iface: data, metric }),
                            "point-to-point" => Some(Link::PointToPoint { neighbor: a?, iface: data, metric }),
                            _ => None,
                        }
                    })
                    .collect();
                db.routers.push(RouterLsa { area, id, seq, age, links });
            }
            ("network", Some(area)) => {
                let Some(mask_len) = fields.get("netmask").and_then(|m| mask_len(m)) else { continue };
                let attached = body
                    .lines()
                    .filter_map(|line| pairs(line).get("router-id").and_then(|r| r.parse().ok()))
                    .collect();
                db.networks.push(NetworkLsa { area, id, seq, age, mask_len, attached });
            }
            ("external", _) => {
                let Some(mask_len) = fields.get("netmask").and_then(|m| mask_len(m)) else { continue };
                let metric = fields.get("metric").and_then(|m| m.parse().ok()).unwrap_or(0);
                let forward = fields.get("forwarding-address").and_then(|f| f.parse::<Ipv4Addr>().ok()).filter(|f| !f.is_unspecified());
                // "metric=10 type-2": the type is a bare word on the metric line.
                let e1 = body.lines().any(|line| line.split_whitespace().any(|t| t == "type-1"));
                db.externals.push(ExternalLsa { id, originator, seq, age, mask_len, e1, metric, forward });
            }
            ("inter-area-prefix", Some(area)) => {
                let Some(mask_len) = fields.get("netmask").and_then(|m| mask_len(m)) else { continue };
                db.summaries.push(SummaryLsa { area, id, originator, seq, age, mask_len });
            }
            _ => {}
        }
    }
    db
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> Ipv4Addr {
        s.parse().unwrap()
    }

    /// Records as 10.107.10.129 (router ID 10.107.10.97) shows them.
    fn sample() -> (Value, Value, Value) {
        let instances = serde_json::json!([
            {"name": "vez-inst", "version": "2", "router-id": "10.107.10.97"},
            {"name": "v6", "version": "3", "router-id": "main"}
        ]);
        let areas = serde_json::json!([
            {"name": "hkfree", "area-id": "0.0.0.0", "instance": "vez-inst"},
            {"name": "area-10", "area-id": "10.107.10.0", "instance": "vez-inst"},
            {"name": "v6-area", "area-id": "0.0.0.0", "instance": "v6"}
        ]);
        let lsas = serde_json::json!([
            {"type": "router", "area": "area-10", "id": "10.107.10.65", "originator": "10.107.10.65", "sequence": "0x80000697", "age": "351", "instance": "vez-inst",
             "body": "options=E bits=E\n    type=network id=10.107.10.58 data=10.107.10.57 metric=50\n    type=stub id=10.107.10.64 data=255.255.255.240 metric=1\n"},
            {"type": "router", "area": "hkfree", "id": "10.107.2.1", "originator": "10.107.2.1", "sequence": "0x80000001", "age": "5m", "instance": "vez-inst",
             "body": "options=E bits=E\n    type=point-to-point id=10.107.99.1 data=10.107.0.1 metric=10\n    type=virtual id=10.107.9.9 data=10.107.0.1 metric=1\n"},
            {"type": "network", "area": "hkfree", "id": "10.107.0.17", "originator": "10.107.15.33", "sequence": "0x800003B1", "age": "607", "instance": "vez-inst",
             "body": "netmask=255.255.255.248\n    router-id=10.107.15.1\n    router-id=10.107.15.33\n"},
            {"type": "external", "id": "0.0.0.0", "originator": "10.107.99.1", "sequence": "0x8004BE3E", "age": "1688", "instance": "vez-inst",
             "body": "options=E|O\nnetmask=0.0.0.0\nforwarding-address=0.0.0.0\nmetric=10 type-2\nroute-tag=1\n"},
            {"type": "external", "id": "10.250.1.0", "originator": "10.107.6.1", "sequence": "0x80000002", "age": "12", "instance": "vez-inst",
             "body": "netmask=255.255.255.0\nforwarding-address=10.107.0.6\nmetric=20 type-1\n"},
            {"type": "inter-area-prefix", "area": "hkfree", "id": "10.107.10.64", "originator": "10.107.10.97", "sequence": "0x80000327", "age": "790", "instance": "vez-inst",
             "body": "netmask=255.255.255.240\nmetric=3\n"},
            {"type": "router", "area": "v6-area", "id": "10.107.10.193", "originator": "10.107.10.193", "sequence": "0x80000001", "age": "1", "instance": "v6", "body": ""}
        ]);
        (instances, areas, lsas)
    }

    #[test]
    fn reads_the_ipv4_records_of_all_areas() {
        let (i, a, l) = sample();
        let db = parse(&i, &a, &l);
        assert_eq!(db.root, Some(ip("10.107.10.97")));
        assert_eq!(db.routers.len(), 2, "the OSPFv3 instance is left out");
        let inside = db.routers.iter().find(|r| r.id == ip("10.107.10.65")).unwrap();
        assert_eq!(inside.area, ip("10.107.10.0"));
        assert_eq!(inside.seq, parse_seq("0x80000697"));
        assert_eq!(inside.links, vec![
            Link::Transit { dr: ip("10.107.10.58"), iface: Some(ip("10.107.10.57")), metric: 50 },
            Link::Stub { net: ip("10.107.10.64"), mask: ip("255.255.255.240"), metric: 1 },
        ]);
        let p2p = db.routers.iter().find(|r| r.id == ip("10.107.2.1")).unwrap();
        assert_eq!(p2p.age, 300);
        assert_eq!(p2p.links.len(), 1, "virtual links are left out");
        assert_eq!(db.networks[0].attached, vec![ip("10.107.15.1"), ip("10.107.15.33")]);
        assert_eq!(db.networks[0].mask_len, 29);
        let default = db.externals.iter().find(|e| e.id == ip("0.0.0.0")).unwrap();
        assert_eq!((default.mask_len, default.e1, default.metric, default.forward), (0, false, 10, None));
        let e1 = db.externals.iter().find(|e| e.id == ip("10.250.1.0")).unwrap();
        assert_eq!((e1.e1, e1.forward), (true, Some(ip("10.107.0.6"))));
        assert_eq!(db.summaries[0].mask_len, 28);
    }

    #[test]
    fn ages_in_both_forms() {
        assert_eq!(parse_age("351"), 351);
        assert_eq!(parse_age("5m51s"), 351);
        assert_eq!(parse_age("1h"), 3600);
    }
}
