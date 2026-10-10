//! Combines OSPF topology, active probes, Sojka and UserDB into findings.

use crate::ospf::{ExitPath, Prefix, Topology};
use crate::probe::Probe;
use crate::sojka::{Health, PingResult};
use crate::state::State;
use crate::userdb::{Ap, Device, Membership};
use serde::Serialize;
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub severity: Severity,
    pub title: String,
    pub detail: Vec<String>,
}

/// Everything collected for one diagnosis run.
#[derive(Default)]
pub struct Inputs {
    pub target: Option<Ipv4Addr>,
    pub probes: BTreeMap<Ipv4Addr, Probe>,
    pub sojka: BTreeMap<Ipv4Addr, PingResult>,
    pub sojka_error: Option<String>,
    pub lookback: Duration,
    pub aps: BTreeMap<u32, Ap>,
    pub devices: BTreeMap<Ipv4Addr, Device>,
    pub userdb_error: Option<String>,
    pub membership: Option<Result<Membership, String>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct HopView {
    pub router: Ipv4Addr,
    pub name: Option<String>,
    pub cost: u32,
    pub probe: Option<Probe>,
    pub sojka: Option<Health>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub target: Ipv4Addr,
    pub prefix: Option<Prefix>,
    pub router: Option<Ipv4Addr>,
    pub router_name: Option<String>,
    pub device: Option<Device>,
    pub path: Vec<HopView>,
    pub exit: Option<Ipv4Addr>,
    pub exit_description: Option<String>,
    pub findings: Vec<Finding>,
    pub verdict: String,
}

/// Routers to probe and query in Sojka before calling [`diagnose`].
pub fn interesting_ips(topo: &Topology, state: &State, target: Ipv4Addr) -> Vec<Ipv4Addr> {
    let mut ips = vec![target];
    match topo.owner_router(target) {
        (_, Some(p)) => ips.extend(p.hops.iter().map(|h| h.router)),
        (Some(r), None) => ips.push(r),
        (None, None) => ips.extend(state.previous_owners(target).iter().map(|s| s.router)),
    }
    ips.extend(topo.exits.iter().map(|e| e.router));
    ips.sort();
    ips.dedup();
    ips
}

pub struct Namer<'a> {
    topo: &'a Topology,
    inputs: &'a Inputs,
}

impl<'a> Namer<'a> {
    pub fn new(topo: &'a Topology, inputs: &'a Inputs) -> Namer<'a> {
        Namer { topo, inputs }
    }

    /// Human name of a router: the AP that owns its id or any interface IP.
    pub fn name(&self, router: Ipv4Addr) -> Option<String> {
        if let Some(ap) = self.ap_of(router) {
            return Some(format!("{} / {}", ap.area, ap.name));
        }
        self.addresses(router)
            .filter_map(|ip| self.inputs.devices.get(&ip))
            .find(|d| !d.hostname.is_empty())
            .map(|d| d.hostname.clone())
    }

    /// Router id followed by all its interface addresses.
    fn addresses(&self, router: Ipv4Addr) -> impl Iterator<Item = Ipv4Addr> + 'a {
        let r = self.topo.routers.get(&router);
        std::iter::once(router).chain(r.into_iter().flat_map(|r| r.interfaces.iter().copied()))
    }

    pub fn ap_of(&self, router: Ipv4Addr) -> Option<&'a Ap> {
        let inputs = self.inputs;
        self.addresses(router)
            .find_map(|ip| inputs.devices.get(&ip)?.ap_id)
            .and_then(|id| inputs.aps.get(&id))
    }

    fn label(&self, router: Ipv4Addr) -> String {
        match self.name(router) {
            Some(n) => format!("{router} ({n})"),
            None => router.to_string(),
        }
    }
}

fn ago(ts: i64) -> String {
    let s = crate::now().saturating_sub(ts).max(0);
    match s {
        0..=119 => format!("{s} s ago"),
        120..=7199 => format!("{} min ago", s / 60),
        7200..=172_799 => format!("{} h ago", s / 3600),
        _ => format!("{} days ago", s / 86400),
    }
}

fn add(f: &mut Vec<Finding>, severity: Severity, title: String, detail: Vec<String>) {
    f.push(Finding {
        severity,
        title,
        detail,
    });
}

pub fn diagnose(topo: &Topology, state: &State, target: Ipv4Addr, inp: &Inputs) -> Report {
    let namer = Namer::new(topo, inp);
    let mut f: Vec<Finding> = Vec::new();

    let device = inp.devices.get(&target).cloned();
    if let Some(e) = &inp.userdb_error {
        add(
            &mut f,
            Severity::Info,
            "UserDB data unavailable".into(),
            vec![e.clone()],
        );
    }
    if let Some(e) = &inp.sojka_error {
        add(
            &mut f,
            Severity::Info,
            "Sojka pinger data unavailable".into(),
            vec![e.clone()],
        );
    }

    // 1. Membership (only when a member login is configured).
    match &inp.membership {
        Some(Ok(m)) if m.kind.is_empty() => add(
            &mut f,
            Severity::Critical,
            format!("UserDB returned no membership type for UID {}", m.uid),
            vec!["Check the account in UserDB (membership may be suspended).".into()],
        ),
        Some(Ok(m)) => add(
            &mut f,
            Severity::Info,
            format!("Membership of UID {} is active: {}", m.uid, m.kind),
            vec![],
        ),
        Some(Err(e)) => add(
            &mut f,
            Severity::Warning,
            "Membership check failed".into(),
            vec![e.clone()],
        ),
        None => {}
    }

    // 2. Target as seen from here and by Sojka.
    let target_probe = inp.probes.get(&target);
    let target_up = target_probe.map(Probe::reachable);
    let target_sojka = inp.sojka.get(&target);

    // 3. Where does the target live in OSPF?
    let (prefix, _) = topo.locate(target);
    let (router, path): (Option<Ipv4Addr>, Option<ExitPath>) = topo.owner_router(target);
    let reach_from_router = router.map(|r| topo.reachable_from(r)).unwrap_or_default();
    match router {
        None => {
            let prev = state.previous_owners(target);
            let mut detail = vec![
                "No router in the OSPF database advertises a subnet containing this address."
                    .into(),
            ];
            if let Some(d) = &device {
                if let Some(ap) = d.ap_id.and_then(|id| inp.aps.get(&id)) {
                    detail.push(format!(
                        "UserDB: address belongs to AP {} / {}",
                        ap.area, ap.name
                    ));
                }
            }
            for s in &prev {
                let present = topo.routers.contains_key(&s.router);
                detail.push(format!(
                    "{} was advertised by router {} (last seen {}); router is {}",
                    s.prefix,
                    namer.label(s.router),
                    ago(s.last_seen as i64),
                    if present {
                        "still in OSPF but no longer advertises it"
                    } else {
                        "missing from OSPF (down?)"
                    }
                ));
                if let Some(p) = inp.probes.get(&s.router) {
                    detail.push(format!(
                        "router {} {} ping from here",
                        s.router,
                        if p.reachable() {
                            "answers"
                        } else {
                            "does NOT answer"
                        }
                    ));
                }
            }
            if prev.is_empty() {
                detail.push("No earlier snapshot knows this subnet either (first run, or not an hkfree.org address).".into());
            }
            if target_up == Some(true) {
                detail.push(
                    "It answers ping, so it is reachable some other way (static route, VPN, NAT)."
                        .into(),
                );
                add(
                    &mut f,
                    Severity::Warning,
                    format!("{target} answers ping but is not in OSPF"),
                    detail,
                );
            } else {
                add(
                    &mut f,
                    Severity::Critical,
                    format!("{target} is not routed in the hkfree.org network"),
                    detail,
                );
            }
        }
        Some(r) => {
            let mut cut_off = false;
            match topo.routers.get(&r) {
                None => {
                    cut_off = true;
                    add(
                        &mut f,
                        Severity::Critical,
                        format!("Router {} has no router LSA", namer.label(r)),
                        vec!["Its subnet is still listed by a neighbour, but the router itself is gone from OSPF: it is down.".into()],
                    );
                }
                Some(rr) => {
                    if let Some(root) = topo.root {
                        if r != root && !topo.reachable_from(root).contains(&r) {
                            cut_off = true;
                            add(
                                &mut f,
                                Severity::Critical,
                                format!("AP router {} is cut off from the network", namer.label(r)),
                                vec![
                                    format!(
                                        "It still advertises {} link(s) but none of them is confirmed by a neighbour.",
                                        rr.advertised_links
                                    ),
                                    format!("Its LSA is {} s old: the router or all its uplinks are down.", rr.age),
                                ],
                            );
                        }
                    }
                }
            }
            if path.is_none() && !cut_off {
                add(
                    &mut f,
                    Severity::Critical,
                    format!("No internet exit is reachable from {}", namer.label(r)),
                    vec![format!(
                        "The network partition containing this router has {} router(s) and no default-route originator.",
                        reach_from_router.len()
                    )],
                );
            }
        }
    }

    // 4. Walk the path.
    let mut hops = Vec::new();
    if let Some(p) = &path {
        // A hop that does not answer while something behind it does (or that
        // Sojka has not seen alive for the whole lookback) just filters ICMP.
        let answers = |ip: &Ipv4Addr| inp.probes.get(ip).map_or(false, Probe::reachable);
        let last_answering = p.hops.iter().rposition(|h| answers(&h.router));
        let target_answers = target_up == Some(true);
        let mut silent: Vec<Ipv4Addr> = Vec::new();
        let mut filtering: Vec<Ipv4Addr> = Vec::new();
        for (i, h) in p.hops.iter().enumerate() {
            let probe = inp.probes.get(&h.router).cloned();
            let health = inp.sojka.get(&h.router).map(|s| s.health(inp.lookback));
            if probe
                .as_ref()
                .map_or(false, |p| !p.reachable() && p.error.is_none())
            {
                if target_answers
                    || last_answering.map_or(false, |l| l > i)
                    || health == Some(Health::Stale)
                {
                    filtering.push(h.router);
                } else {
                    silent.push(h.router);
                }
            }
            let pinged_ok = probe.as_ref().map_or(false, Probe::reachable);
            let health_issue = match health {
                // Our own live probe beats Sojka's (possibly older) view.
                Some(Health::Dead) if pinged_ok => None,
                h @ Some(Health::Lossy | Health::Slow | Health::Dead) => h,
                _ => None,
            };
            if let Some(hh) = health_issue {
                add(
                    &mut f,
                    if hh == Health::Dead {
                        Severity::Critical
                    } else {
                        Severity::Warning
                    },
                    format!(
                        "Path hop {} is {:?} according to Sojka",
                        namer.label(h.router),
                        hh
                    ),
                    inp.sojka.get(&h.router).map_or(Vec::new(), |s| {
                        vec![format!(
                            "loss {:.0} %, rtt {:.0} ms, last reply {}",
                            s.loss * 100.0,
                            s.rtt * 1000.0,
                            ago(s.time_lastpong)
                        )]
                    }),
                );
            }
            hops.push(HopView {
                router: h.router,
                name: namer.name(h.router),
                cost: h.cost,
                probe,
                sojka: health,
            });
        }
        if let Some(d) = silent.first() {
            add(
                &mut f,
                Severity::Warning,
                format!("Path hop {} does not answer ping from here", namer.label(*d)),
                vec!["Nothing behind it answers either, but OSPF still lists its adjacencies as up: the router may be overloaded, its forwarding may be stuck, or it filters ICMP.".into()],
            );
        }
        if !filtering.is_empty() {
            let list: Vec<String> = filtering.iter().map(|r| namer.label(*r)).collect();
            add(
                &mut f,
                Severity::Info,
                "Hops that do not answer ping (ICMP filtered, not a fault)".into(),
                list,
            );
        }
        let (links, nodes) = topo.single_points_of_failure(p);
        if !links.is_empty() || !nodes.is_empty() {
            let mut d: Vec<String> = links
                .iter()
                .map(|(a, b)| format!("link {} -> {}", namer.label(*a), namer.label(*b)))
                .collect();
            d.extend(nodes.iter().map(|n| format!("router {}", namer.label(*n))));
            add(
                &mut f,
                Severity::Info,
                "Single points of failure on the path (no backup route)".into(),
                d,
            );
        }
    }

    // 5. Exits.
    let reachable_exits: Vec<_> = topo
        .exits
        .iter()
        .filter(|e| reach_from_router.contains(&topo.exit_target(e)))
        .collect();
    if router.is_some() && path.is_some() && reachable_exits.len() == 1 && topo.exits.len() > 1 {
        add(
            &mut f,
            Severity::Warning,
            "Only one internet exit is reachable".into(),
            vec![format!(
                "{} of {} default-route originators reachable",
                reachable_exits.len(),
                topo.exits.len()
            )],
        );
    }

    // 6. Last mile: AP router fine, target not.
    let router_up = router
        .and_then(|r| inp.probes.get(&r))
        .map(Probe::reachable);
    if let (Some(r), Some(false)) = (router, target_up) {
        if r != target && path.is_some() && router_up == Some(false) {
            add(
                &mut f,
                Severity::Critical,
                format!("AP router {} and {target} both do not answer", namer.label(r)),
                vec!["OSPF still lists the router's adjacencies, but it does not answer ping: it is hung/overloaded, or it just died and OSPF has not noticed yet (dead interval ~40 s).".into()],
            );
        } else if r != target && path.is_some() {
            let mut detail = vec![format!(
                "The backbone path to AP router {} is up, but {target} does not answer ping.",
                namer.label(r)
            )];
            detail.push("Likely causes: CPE/radio link to the AP, device powered off, or the device drops ICMP.".into());
            if let Some(s) = target_sojka {
                match s.health(inp.lookback) {
                    Health::Stale => detail.push(
                        "Sojka has not seen this IP alive within the lookback window.".into(),
                    ),
                    h => detail.push(format!("Sojka: {h:?}, last reply {}", ago(s.time_lastpong))),
                }
            }
            if let Some(ap) = namer.ap_of(r) {
                if !ap.admins.is_empty() {
                    detail.push(format!("AP admins: {}", ap.admins.join(", ")));
                }
            }
            add(
                &mut f,
                Severity::Critical,
                format!("{target} is unreachable behind a working AP"),
                detail,
            );
        }
    }
    if target_up == Some(true) {
        add(&mut f, Severity::Info,
            format!("{target} answers ping from this host"),
            vec!["If the member still has no internet: check the device's default gateway and DNS, whether internet access is enabled for the IP in UserDB, and the path from the exit outward.".into()],
        );
    }

    f.sort_by(|a, b| b.severity.cmp(&a.severity));
    let verdict = match f.first() {
        Some(x) if x.severity > Severity::Info => x.title.clone(),
        _ if target_up == Some(true) => {
            "No network problem found between the hkfree.org core and the target".into()
        }
        _ => "No problem found in the OSPF topology".into(),
    };
    Report {
        target,
        prefix,
        router,
        router_name: router.and_then(|r| namer.name(r)),
        device,
        exit: path.as_ref().and_then(|p| p.hops.last()).map(|h| h.router),
        exit_description: path.as_ref().map(|p| {
            let e = &p.exit;
            let kind = if e.e1 { "E1" } else { "E2" };
            match e.forward {
                Some(fwd) => format!(
                    "{kind} default route from {} (metric {}), forwarded via {fwd}; path cost {}",
                    namer.label(e.router),
                    e.metric,
                    p.cost
                ),
                None => format!(
                    "{kind} default route from {} (metric {}); path cost {}",
                    namer.label(e.router),
                    e.metric,
                    p.cost
                ),
            }
        }),
        path: hops,
        findings: f,
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topo() -> Topology {
        Topology::from_json(
            include_str!("../tests/fixtures/router.json"),
            include_str!("../tests/fixtures/network.json"),
            include_str!("../tests/fixtures/external.json"),
        )
        .unwrap()
    }

    fn probe(ip: &str, ok: bool) -> (Ipv4Addr, Probe) {
        let ip: Ipv4Addr = ip.parse().unwrap();
        (
            ip,
            Probe {
                ip,
                sent: 3,
                received: if ok { 3 } else { 0 },
                avg_rtt_ms: None,
                error: None,
            },
        )
    }

    #[test]
    fn unrouted_ip_is_critical() {
        let r = diagnose(
            &topo(),
            &State::default(),
            "10.107.77.5".parse().unwrap(),
            &Inputs::default(),
        );
        assert!(r.verdict.contains("not routed"));
    }

    #[test]
    fn isolated_ap_router() {
        let r = diagnose(
            &topo(),
            &State::default(),
            "10.107.9.20".parse().unwrap(),
            &Inputs::default(),
        );
        assert!(r.verdict.contains("cut off"), "{}", r.verdict);
    }

    #[test]
    fn last_mile_failure() {
        let inp = Inputs {
            probes: [
                probe("10.107.5.130", false),
                probe("10.107.5.1", true),
                probe("10.107.2.1", true),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let r = diagnose(
            &topo(),
            &State::default(),
            "10.107.5.130".parse().unwrap(),
            &inp,
        );
        assert!(r.verdict.contains("behind a working AP"), "{}", r.verdict);
        assert_eq!(r.path.len(), 3);
    }

    #[test]
    fn silent_ap_router_is_not_blamed_on_last_mile() {
        let inp = Inputs {
            probes: [
                probe("10.107.5.130", false),
                probe("10.107.5.1", false),
                probe("10.107.2.1", true),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let r = diagnose(
            &topo(),
            &State::default(),
            "10.107.5.130".parse().unwrap(),
            &inp,
        );
        assert!(r.verdict.contains("both do not answer"), "{}", r.verdict);
    }

    #[test]
    fn interface_address_maps_to_its_router() {
        let r = diagnose(
            &topo(),
            &State::default(),
            "10.107.0.2".parse().unwrap(),
            &Inputs::default(),
        );
        assert_eq!(r.router, Some("10.107.2.1".parse().unwrap()));
    }

    #[test]
    fn healthy_target() {
        let inp = Inputs {
            probes: [probe("10.107.5.130", true)].into_iter().collect(),
            ..Default::default()
        };
        let r = diagnose(
            &topo(),
            &State::default(),
            "10.107.5.130".parse().unwrap(),
            &inp,
        );
        assert!(r.verdict.starts_with("No network problem"), "{}", r.verdict);
    }
}
