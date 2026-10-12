//! `hkdiag export`: a self-contained topology snapshot for the on-device
//! (PWA) diagnosis engine in `web/engine.js`.
//!
//! The snapshot is meant to be readable by every member, so it contains only
//! infrastructure data: routers, links, advertised prefixes, internet exits,
//! AP names and Sojka status of routers. No member devices, no descriptions,
//! no contacts.

use crate::config::Config;
use crate::diagnose::{Inputs, Namer};
use crate::ospf::{PrefixKind, Topology};
use crate::sojka::{self, Health};
use crate::state::State;
use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::Ipv4Addr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Duration;

pub const FORMAT_VERSION: u32 = 1;

#[derive(Serialize)]
pub struct Snapshot {
    pub version: u32,
    pub generated_at: i64,
    /// Router the LSDB was read from (reachability reference).
    pub root: Option<Ipv4Addr>,
    pub routers: BTreeMap<Ipv4Addr, RouterOut>,
    /// [prefix, router, kind] with kind "s"tub, "t"ransit or "e"xternal.
    pub prefixes: Vec<(String, Ipv4Addr, &'static str)>,
    pub exits: Vec<ExitOut>,
    /// Prefixes seen in earlier snapshots but not now: [prefix, router, last_seen].
    pub previous: Vec<(String, Ipv4Addr, u64)>,
    /// Links seen in the last week that are down now: [router, router, last_seen].
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lost_links: Vec<(Ipv4Addr, Ipv4Addr, u64)>,
    /// IPv6 prefixes from OSPFv3: [prefix, advertising router, kind].
    pub prefixes6: Vec<(String, Ipv4Addr, &'static str)>,
    /// Who to contact when the problem is on the network's side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support: Option<crate::config::SupportConfig>,
    /// Address pools of the hkfree.org VPN.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub vpn: Vec<String>,
    /// Where the app measures the connection's speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speedtest: Option<crate::config::SpeedtestConfig>,
    /// What network monitoring reports for each AP's own devices, by AP id.
    pub aps: BTreeMap<u32, ApMonitor>,
    /// The extra routers the map was also read from, and how they answered.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<crate::mikrotik::SourceStatus>,
}

#[derive(Serialize)]
pub struct ApMonitor {
    /// [address, label, health]; devices monitoring hasn't heard from within
    /// the lookback (decommissioned) are left out.
    pub devices: Vec<(Ipv4Addr, String, Health)>,
}

/// A short public label for an AP device: its hostname, else the start of its description.
fn device_label(d: &crate::userdb::Device) -> String {
    let src = if d.hostname.trim().is_empty() { &d.description } else { &d.hostname };
    src.chars().filter(|c| !c.is_control()).take(60).collect::<String>().trim().to_string()
}

#[derive(Serialize)]
pub struct RouterOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub age: u32,
    /// Links the router advertises, including unconfirmed ones.
    pub advertised: usize,
    /// Confirmed adjacencies: [neighbour, cost, transit prefix or null].
    pub links: Vec<(Ipv4Addr, u32, Option<String>)>,
    /// The router's own interface addresses.
    pub ifaces: Vec<Ipv4Addr>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sojka: Option<Health>,
    /// The AP this router belongs to, from the member records (infrastructure only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ap: Option<ApOut>,
}

#[derive(Serialize)]
pub struct ApOut {
    pub id: u32,
    pub name: String,
    pub area: String,
    pub address: String,
    /// Nicknames of the AP admins (SO first). Their e-mails are never in the
    /// snapshot; see `Contacts`.
    pub admins: Vec<AdminOut>,
}

#[derive(Serialize)]
pub struct AdminOut {
    pub nick: String,
    pub role: String,
}

/// AP admins per router, with e-mails: handed out by the server only to a
/// device inside that router's AP, never put in the snapshot.
pub type Contacts = BTreeMap<Ipv4Addr, Vec<crate::userdb::Admin>>;

/// What the server keeps beside the snapshot and never hands out whole.
#[derive(Default)]
pub struct Private {
    pub contacts: Contacts,
    /// The AP (UserDB id) of each router the member records name.
    pub router_ap: BTreeMap<Ipv4Addr, u32>,
}

#[derive(Serialize)]
pub struct ExitOut {
    /// ASBR that originates the default route.
    pub router: Ipv4Addr,
    /// Router traffic is actually sent to (honours the forwarding address).
    pub target: Ipv4Addr,
    pub e1: bool,
    pub metric: u32,
}

pub fn build(cfg: &Config, topo: &Topology, state: &State) -> Snapshot {
    build_with_contacts(cfg, topo, state).0
}

pub fn build_with_contacts(cfg: &Config, topo: &Topology, state: &State) -> (Snapshot, Private) {
    let (aps, devices) = if cfg.userdb.is_some() {
        crate::collect::load_infrastructure(cfg).unwrap_or_else(|e| {
            eprintln!("warning: UserDB: {e:#}");
            Default::default()
        })
    } else {
        Default::default()
    };
    // Only infrastructure (AP-owned) addresses are used for naming.
    let devices: BTreeMap<Ipv4Addr, crate::userdb::Device> =
        devices.into_iter().filter(|(_, d)| d.ap_id.is_some()).collect();
    // Monitoring shows only the AP's own devices, never ones registered to a member.
    let ap_devices: Vec<(&Ipv4Addr, &crate::userdb::Device)> =
        devices.iter().filter(|(ip, d)| d.user_id.is_none() && !topo.routers.contains_key(ip)).collect();
    // Routers first, on their own: a failure with the AP devices must not
    // take the routers' health with it.
    let router_ids: Vec<Ipv4Addr> = topo.routers.keys().copied().collect();
    let device_ids: Vec<Ipv4Addr> = ap_devices.iter().map(|(ip, _)| **ip).collect();
    let mut sojka = BTreeMap::new();
    for (what, ids) in [("routers", &router_ids), ("AP devices", &device_ids)] {
        match sojka::ping_ips(&cfg.sojka, ids) {
            Ok(s) => sojka.extend(s),
            Err(e) => eprintln!("warning: Sojka ({what}): {e:#}"),
        }
    }
    let lookback = Duration::from_secs(cfg.sojka.lookback_secs);
    let mut ap_monitor: BTreeMap<u32, ApMonitor> = BTreeMap::new();
    let ap_routers = devices.iter().filter(|(ip, d)| d.user_id.is_none() && topo.routers.contains_key(ip));
    for (ip, d) in ap_devices.into_iter().chain(ap_routers) {
        let (Some(ap), Some(p)) = (d.ap_id, sojka.get(ip)) else { continue };
        let health = p.health(lookback);
        if health != Health::Stale {
            ap_monitor.entry(ap).or_insert_with(|| ApMonitor { devices: Vec::new() }).devices.push((*ip, device_label(d), health));
        }
    }
    let inputs = Inputs { aps, devices, ..Default::default() };
    let namer = Namer::new(topo, &inputs);

    let routers = topo
        .routers
        .values()
        .map(|r| {
            let links = r.edges.iter().map(|e| (e.to, e.cost, e.network.map(|n| n.to_string()))).collect();
            let out = RouterOut {
                name: namer.ap_of(r.id).map(|ap| format!("{} / {}", ap.area, ap.name)),
                ap: namer.ap_of(r.id).map(|ap| ApOut {
                    id: ap.id,
                    name: ap.name.clone(),
                    area: ap.area.clone(),
                    address: ap.address.clone(),
                    admins: ap.admins.iter().map(|a| AdminOut { nick: a.nick.clone(), role: a.role.clone() }).collect(),
                }),
                age: r.age,
                advertised: r.advertised_links,
                links,
                ifaces: r.interfaces.clone(),
                sojka: sojka.get(&r.id).map(|p| p.health(lookback)),
            };
            (r.id, out)
        })
        .collect();

    let prefixes = topo
        .prefixes
        .iter()
        .map(|p| {
            let kind = match p.kind {
                PrefixKind::Stub => "s",
                PrefixKind::Transit => "t",
                PrefixKind::External => "e",
                PrefixKind::Summary => "a",
            };
            (p.prefix.to_string(), p.router, kind)
        })
        .collect();

    let exits = topo
        .exits
        .iter()
        .map(|e| ExitOut { router: e.router, target: topo.exit_target(e), e1: e.e1, metric: e.metric })
        .collect();

    let current: std::collections::HashSet<(String, Ipv4Addr)> =
        topo.prefixes.iter().map(|p| (p.prefix.to_string(), p.router)).collect();
    let previous = state
        .seen
        .values()
        .filter(|s| !current.contains(&(s.prefix.to_string(), s.router)))
        .map(|s| (s.prefix.to_string(), s.router, s.last_seen))
        .collect();

    let contacts = topo
        .routers
        .keys()
        .filter_map(|&r| namer.ap_of(r).filter(|ap| !ap.admins.is_empty()).map(|ap| (r, ap.admins.clone())))
        .collect();

    // A link counts as lost when it's gone but both its routers are still in
    // the map; a router that vanished altogether shows as such.
    let up: std::collections::HashSet<(Ipv4Addr, Ipv4Addr)> = topo
        .routers
        .values()
        .flat_map(|r| r.edges.iter().map(move |e| if r.id < e.to { (r.id, e.to) } else { (e.to, r.id) }))
        .collect();
    let lost_links = state
        .links
        .values()
        .filter(|l| !up.contains(&(l.a, l.b)) && topo.routers.contains_key(&l.a) && topo.routers.contains_key(&l.b))
        .map(|l| (l.a, l.b, l.last_seen))
        .collect();

    let snapshot = Snapshot {
        version: FORMAT_VERSION,
        generated_at: crate::now(),
        root: topo.root,
        routers,
        prefixes,
        exits,
        previous,
        lost_links,
        prefixes6: topo
            .prefixes6
            .iter()
            .map(|p| (p.text(), p.router, if p.kind == PrefixKind::External { "e" } else { "s" }))
            .collect(),
        support: cfg.support.clone(),
        speedtest: cfg.speedtest.clone(),
        vpn: cfg.network.vpn.clone(),
        aps: ap_monitor,
        sources: topo.sources.clone(),
    };
    let router_ap = topo.routers.keys().filter_map(|&r| namer.ap_of(r).map(|ap| (r, ap.id))).collect();
    (snapshot, Private { contacts, router_ap })
}

/// Writes `data` to `path` atomically (temp file + rename), world-readable
/// because the snapshot is meant to be served to members.
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    let tmp = path.with_extension(format!("tmp.{}", std::process::id()));
    let _ = fs::remove_file(&tmp);
    let res = (|| -> Result<()> {
        let mut f = OpenOptions::new().write(true).create_new(true).mode(0o644).open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok(())
    })();
    if res.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    res.with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot() -> Snapshot {
        let topo = Topology::from_json(
            include_str!("../tests/fixtures/router.json"),
            include_str!("../tests/fixtures/network.json"),
            include_str!("../tests/fixtures/external.json"),
        )
        .unwrap();
        // Unreachable Sojka: no health data, deterministic output.
        let cfg: Config = toml::from_str("[sojka]\nurl = \"http://127.0.0.1:9/\"\ntimeout_secs = 1\n").unwrap();
        let mut state = State::default();
        state.seen.insert(
            "10.107.77.0/24 10.107.77.1".into(),
            crate::state::Seen { router: "10.107.77.1".parse().unwrap(), prefix: crate::ospf::Prefix::new("10.107.77.0".parse().unwrap(), 24), last_seen: 1_700_000_000 },
        );
        let mut snap = build(&cfg, &topo, &state);
        snap.generated_at = 1_700_000_100;
        snap
    }

    #[test]
    fn snapshot_contains_no_member_data_and_resolves_exits() {
        let snap = fixture_snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(json.contains("\"10.107.5.128/26\""));
        assert_eq!(snap.exits.len(), 2);
        assert_eq!(snap.routers.len(), 5);
        assert_eq!(snap.previous.len(), 1);
        assert!(!json.contains("Uzivatel") && !json.contains("popis"));
    }

    /// The JS engine tests (web/test/) use this file: Rust and JS must agree
    /// on the same data. Regenerate with HKDIAG_BLESS=1 cargo test.
    #[test]
    fn golden_snapshot_is_current() {
        let json = serde_json::to_string_pretty(&fixture_snapshot()).unwrap() + "\n";
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snapshot.json");
        if std::env::var_os("HKDIAG_BLESS").is_some() {
            fs::write(path, &json).unwrap();
        }
        let on_disk = fs::read_to_string(path).unwrap_or_default();
        assert_eq!(on_disk, json, "run HKDIAG_BLESS=1 cargo test to update {path}");
    }
}
