//! OSPF link-state database (FRR JSON) -> topology graph, IP location and
//! shortest paths to the default-route originators.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
use std::net::{Ipv4Addr, Ipv6Addr};

/// LSAs at or above MaxAge are being flushed and must be ignored (RFC 2328).
const MAX_AGE: u32 = 3600;

/// External (type-5) prefixes shorter than this are aggregates such as
/// 10.107.0.0/16 and are not used to locate an address: they would make every
/// unrouted hkfree.org address look routed.
const MIN_EXTERNAL_LEN: u8 = 17;

// ---------- raw FRR JSON ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouterDb {
    #[serde(rename = "routerId")]
    router_id: Option<Ipv4Addr>,
    router_link_states: AreaMap<RawRouterLsa>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkDb {
    network_link_states: AreaMap<RawNetworkLsa>,
}

#[derive(Deserialize)]
struct AreaMap<T> {
    #[serde(default = "BTreeMap::new")]
    areas: BTreeMap<String, Vec<T>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalDb {
    #[serde(default)]
    as_external_link_states: Vec<RawExternalLsa>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRouterLsa {
    link_state_id: Ipv4Addr,
    #[serde(default)]
    lsa_age: u32,
    #[serde(default)]
    router_links: BTreeMap<String, RawRouterLink>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawRouterLink {
    link_type: String,
    network_address: Option<Ipv4Addr>,
    network_mask: Option<Ipv4Addr>,
    designated_router_address: Option<Ipv4Addr>,
    router_interface_address: Option<Ipv4Addr>,
    neighbor_router_id: Option<Ipv4Addr>,
    #[serde(default)]
    tos0_metric: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawNetworkLsa {
    link_state_id: Ipv4Addr,
    #[serde(default)]
    lsa_age: u32,
    network_mask: u8,
    // FRR spells it "attchedRouters"; accept the correct spelling too.
    #[serde(alias = "attachedRouters", default)]
    attched_routers: BTreeMap<Ipv4Addr, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawExternalLsa {
    link_state_id: Ipv4Addr,
    advertising_router: Ipv4Addr,
    #[serde(default)]
    lsa_age: u32,
    network_mask: u8,
    #[serde(default)]
    metric_type: String,
    #[serde(default)]
    metric: u32,
    forward_address: Option<Ipv4Addr>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SummaryDb {
    summary_link_states: AreaMap<RawSummaryLsa>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSummaryLsa {
    link_state_id: Ipv4Addr,
    advertising_router: Ipv4Addr,
    #[serde(default)]
    lsa_age: u32,
    network_mask: u8,
}

// ---------- model ----------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Prefix {
    pub addr: Ipv4Addr,
    pub len: u8,
}

impl Prefix {
    pub fn new(addr: Ipv4Addr, len: u8) -> Prefix {
        let len = len.min(32);
        Prefix { addr: Ipv4Addr::from(u32::from(addr) & mask(len)), len }
    }
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        u32::from(ip) & mask(self.len) == u32::from(self.addr)
    }
}

impl std::fmt::Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.addr, self.len)
    }
}

fn mask(len: u8) -> u32 {
    if len == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(len.min(32)))
    }
}

fn mask_len(m: Ipv4Addr) -> u8 {
    u32::from(m).count_ones() as u8
}

/// All items whose prefix is the most specific one containing `ip`.
pub fn longest_match<'a, T>(
    items: impl Iterator<Item = &'a T> + Clone,
    prefix: impl Fn(&T) -> Prefix,
    ip: Ipv4Addr,
) -> Vec<&'a T> {
    let best = items.clone().map(&prefix).filter(|p| p.contains(ip)).map(|p| p.len).max();
    match best {
        None => Vec::new(),
        Some(len) => items.filter(|t| { let p = prefix(t); p.len == len && p.contains(ip) }).collect(),
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub to: Ipv4Addr,
    pub cost: u32,
    /// Our interface address on this link (if known).
    pub local_if: Option<Ipv4Addr>,
    /// Neighbour's interface address on this link (if known).
    pub remote_if: Option<Ipv4Addr>,
    /// Transit network the link runs over, `None` for point-to-point.
    pub network: Option<Prefix>,
}

#[derive(Clone, Debug)]
pub struct Router {
    pub id: Ipv4Addr,
    pub age: u32,
    pub edges: Vec<Edge>,
    /// Number of transit/p2p links the router advertises (incl. one-sided ones).
    pub advertised_links: usize,
    /// Router's own interface addresses (transit + p2p).
    pub interfaces: Vec<Ipv4Addr>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Exit {
    pub router: Ipv4Addr,
    pub e1: bool,
    pub metric: u32,
    pub forward: Option<Ipv4Addr>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PrefixKind {
    Stub,
    Transit,
    /// Redistributed (static/connected/VPN) route; `router` is the router
    /// owning the forwarding address, or the ASBR.
    External,
    /// Inter-area summary: the addresses lie in a hidden part of the network
    /// (another OSPF area) and `router` is its border router (ABR).
    Summary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrefixOwner {
    pub prefix: Prefix,
    pub router: Ipv4Addr,
    pub kind: PrefixKind,
}

#[derive(Debug, Default)]
pub struct Topology {
    pub root: Option<Ipv4Addr>,
    pub routers: BTreeMap<Ipv4Addr, Router>,
    pub prefixes: Vec<PrefixOwner>,
    pub exits: Vec<Exit>,
    /// IPv6 prefixes from OSPFv3 with their advertising router (an IPv4 router ID).
    pub prefixes6: Vec<Prefix6Owner>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Prefix6Owner {
    pub addr: Ipv6Addr,
    pub len: u8,
    pub router: Ipv4Addr,
    pub kind: PrefixKind,
}

impl Prefix6Owner {
    pub fn text(&self) -> String {
        format!("{}/{}", self.addr, self.len)
    }
}

/// IPv6 externals shorter than this (e.g. 2000::/3) would cover the whole
/// internet; hkfree.org's own allocation is a /29.
const MIN_PREFIX6_LEN: u8 = 29;

fn parse_prefix6(s: &str) -> Option<(Ipv6Addr, u8)> {
    let (a, l) = s.split_once('/')?;
    let len: u8 = l.parse().ok().filter(|l| *l <= 128)?;
    let addr: Ipv6Addr = a.parse().ok()?;
    let mask = if len == 0 { 0 } else { u128::MAX << (128 - u32::from(len)) };
    Some((Ipv6Addr::from(u128::from(addr) & mask), len))
}

/// Parses ospf6d's `intra-prefix detail json` and `as-external detail json`.
pub fn prefixes6_from_json(intra: &str, external: &str) -> Result<Vec<Prefix6Owner>> {
    let intra: serde_json::Value = serde_json::from_str(intra).context("parsing OSPFv3 intra-prefix LSAs")?;
    let external: serde_json::Value = serde_json::from_str(external).context("parsing OSPFv3 external LSAs")?;
    let mut out = Vec::new();
    let mut add = |prefix: &serde_json::Value, router: &serde_json::Value, kind: PrefixKind| {
        let parsed = prefix.as_str().and_then(parse_prefix6);
        let router = router.as_str().and_then(|r| r.parse::<Ipv4Addr>().ok());
        if let (Some((addr, len)), Some(router)) = (parsed, router) {
            if len >= MIN_PREFIX6_LEN {
                out.push(Prefix6Owner { addr, len, router, kind });
            }
        }
    };
    let lsas = |v: &serde_json::Value, db: &str| -> Vec<serde_json::Value> {
        v[db].as_array().into_iter().flatten().flat_map(|area| area["lsa"].as_array().cloned().unwrap_or_default()).collect()
    };
    let fresh = |l: &serde_json::Value| l["age"].as_u64().map_or(true, |a| a < u64::from(MAX_AGE));
    for l in lsas(&intra, "areaScopedLinkStateDb").iter().filter(|l| fresh(l)) {
        for p in l["prefix"].as_array().into_iter().flatten() {
            add(&p["prefix"], &l["advertisingRouter"], PrefixKind::Stub);
        }
    }
    for l in lsas(&external, "asScopedLinkStateDb").iter().filter(|l| fresh(l)) {
        add(&l["prefix"], &l["advertisingRouter"], PrefixKind::External);
    }
    out.sort_by(|a, b| (a.addr, a.len, a.router).cmp(&(b.addr, b.len, b.router)));
    out.dedup();
    Ok(out)
}

#[derive(Clone, Debug, Serialize)]
pub struct Hop {
    pub router: Ipv4Addr,
    /// Cumulative OSPF cost from the start of the path.
    pub cost: u32,
    /// Interface addresses of the link used to reach this hop (prev side, this side).
    pub via: Option<(Option<Ipv4Addr>, Option<Ipv4Addr>)>,
    /// Transit network of that link (`None` for p2p or the first hop).
    pub network: Option<Prefix>,
}

/// One specific link between two routers (routers can share several).
type LinkId = (Ipv4Addr, Ipv4Addr, Option<Prefix>);

#[derive(Clone, Debug, Serialize)]
pub struct ExitPath {
    pub exit: Exit,
    pub hops: Vec<Hop>,
    pub cost: u32,
}

impl Topology {
    pub fn from_json(router: &str, network: &str, external: &str) -> Result<Topology> {
        let rdb: RouterDb = serde_json::from_str(router).context("parsing router LSAs")?;
        let ndb: NetworkDb = serde_json::from_str(network).context("parsing network LSAs")?;
        let edb: ExternalDb = serde_json::from_str(external).context("parsing external LSAs")?;
        // Only the backbone area is modelled (hkfree.org is a single area 0; an
        // ABR would otherwise have one router LSA per area).
        const BACKBONE: &str = "0.0.0.0";
        let mut rdb = rdb;
        let mut ndb = ndb;
        let rlsas: Vec<RawRouterLsa> = rdb
            .router_link_states
            .areas
            .remove(BACKBONE)
            .unwrap_or_default()
            .into_iter()
            .filter(|l| l.lsa_age < MAX_AGE)
            .collect();
        let nlsas: Vec<RawNetworkLsa> = ndb
            .network_link_states
            .areas
            .remove(BACKBONE)
            .unwrap_or_default()
            .into_iter()
            .filter(|l| l.lsa_age < MAX_AGE)
            .collect();

        let mut topo = Topology::default();
        let nets: HashMap<Ipv4Addr, &RawNetworkLsa> =
            nlsas.iter().map(|n| (n.link_state_id, n)).collect();

        // Transit links per router keyed by DR address: (router, dr) -> (iface, cost)
        let mut transit_if: HashMap<(Ipv4Addr, Ipv4Addr), (Option<Ipv4Addr>, u32)> = HashMap::new();
        // p2p links: (router, neighbour) -> (iface, cost)
        let mut p2p: HashMap<(Ipv4Addr, Ipv4Addr), (Option<Ipv4Addr>, u32)> = HashMap::new();

        for r in &rlsas {
            let mut router = Router {
                id: r.link_state_id,
                age: r.lsa_age,
                edges: Vec::new(),
                advertised_links: 0,
                interfaces: Vec::new(),
            };
            for l in r.router_links.values() {
                match l.link_type.as_str() {
                    "Stub Network" => {
                        if let (Some(a), Some(m)) = (l.network_address, l.network_mask) {
                            topo.prefixes.push(PrefixOwner {
                                prefix: Prefix::new(a, mask_len(m)),
                                router: r.link_state_id,
                                kind: PrefixKind::Stub,
                            });
                        }
                    }
                    "a Transit Network" => {
                        if let Some(dr) = l.designated_router_address {
                            router.advertised_links += 1;
                            router.interfaces.extend(l.router_interface_address);
                            transit_if.insert(
                                (r.link_state_id, dr),
                                (l.router_interface_address, l.tos0_metric),
                            );
                        }
                    }
                    t if t.contains("point-to-point") => {
                        if let Some(n) = l.neighbor_router_id {
                            router.advertised_links += 1;
                            router.interfaces.extend(l.router_interface_address);
                            // Parallel p2p links to the same neighbour: keep the cheapest.
                            let e = p2p.entry((r.link_state_id, n)).or_insert((l.router_interface_address, l.tos0_metric));
                            if l.tos0_metric < e.1 {
                                *e = (l.router_interface_address, l.tos0_metric);
                            }
                        }
                    }
                    _ => {}
                }
            }
            topo.routers.insert(r.link_state_id, router);
        }

        // Transit edges: both routers must be attached in the network LSA and
        // advertise the link in their router LSA (bidirectional check).
        for (&(rid, dr), &(iface, cost)) in &transit_if {
            let Some(net) = nets.get(&dr) else { continue };
            if !net.attched_routers.contains_key(&rid) {
                continue;
            }
            let prefix = Prefix::new(dr, net.network_mask);
            for &other in net.attched_routers.keys() {
                if other == rid {
                    continue;
                }
                if let Some(&(other_if, _)) = transit_if.get(&(other, dr)) {
                    if let Some(r) = topo.routers.get_mut(&rid) {
                        r.edges.push(Edge {
                            to: other,
                            cost,
                            local_if: iface,
                            remote_if: other_if,
                            network: Some(prefix),
                        });
                    }
                }
            }
        }
        for n in &nlsas {
            let prefix = Prefix::new(n.link_state_id, n.network_mask);
            for &rid in n.attched_routers.keys() {
                topo.prefixes.push(PrefixOwner { prefix, router: rid, kind: PrefixKind::Transit });
            }
        }
        for (&(rid, nid), &(iface, cost)) in &p2p {
            if let Some(&(nif, _)) = p2p.get(&(nid, rid)) {
                if let Some(r) = topo.routers.get_mut(&rid) {
                    r.edges.push(Edge { to: nid, cost, local_if: iface, remote_if: nif, network: None });
                }
            }
        }
        for r in topo.routers.values_mut() {
            r.edges.sort_by_key(|e| (e.to, e.cost));
            r.interfaces.sort();
            r.interfaces.dedup();
        }
        // The SPF root is the router we read the LSDB from.
        topo.root = rdb.router_id.filter(|r| topo.routers.contains_key(r));

        let mut externals = Vec::new();
        for e in edb.as_external_link_states {
            if e.lsa_age >= MAX_AGE {
                continue;
            }
            let forward = e.forward_address.filter(|a| !a.is_unspecified());
            if e.network_mask == 0 && e.link_state_id == Ipv4Addr::UNSPECIFIED {
                topo.exits.push(Exit {
                    router: e.advertising_router,
                    e1: e.metric_type.starts_with("E1"),
                    metric: e.metric,
                    forward,
                });
            } else if e.network_mask >= MIN_EXTERNAL_LEN {
                let router = forward.and_then(|f| topo.forward_owner(f)).unwrap_or(e.advertising_router);
                externals.push(PrefixOwner {
                    prefix: Prefix::new(e.link_state_id, e.network_mask),
                    router,
                    kind: PrefixKind::External,
                });
            }
        }
        topo.prefixes.extend(externals);
        topo.prefixes.sort_by_key(|p| (p.prefix, p.router));
        topo.prefixes.dedup_by_key(|p| (p.prefix, p.router));
        Ok(topo)
    }

    /// Adds the backbone's summary-LSAs: prefixes of other areas, owned by
    /// their border router. Summaries of areas we can't see are the only way
    /// to place those addresses at all.
    pub fn add_summaries(&mut self, summary: &str) -> Result<()> {
        let mut sdb: SummaryDb = serde_json::from_str(summary).context("parsing summary LSAs")?;
        for s in sdb.summary_link_states.areas.remove("0.0.0.0").unwrap_or_default() {
            if s.lsa_age >= MAX_AGE || s.network_mask > 32 {
                continue;
            }
            self.prefixes.push(PrefixOwner {
                prefix: Prefix::new(s.link_state_id, s.network_mask),
                router: s.advertising_router,
                kind: PrefixKind::Summary,
            });
        }
        self.prefixes.sort_by_key(|p| (p.prefix, p.router));
        self.prefixes.dedup_by_key(|p| (p.prefix, p.router));
        Ok(())
    }

    /// Routers owning the most specific prefix containing `ip`.
    pub fn locate(&self, ip: Ipv4Addr) -> (Option<Prefix>, Vec<&PrefixOwner>) {
        if self.routers.contains_key(&ip) {
            return (Some(Prefix::new(ip, 32)), Vec::new());
        }
        let owners = longest_match(self.prefixes.iter(), |p| p.prefix, ip);
        (owners.first().map(|o| o.prefix), owners)
    }

    /// The router responsible for `ip`, with its path out if it has one:
    /// the router itself, the owner of the interface address, or the owner of
    /// the most specific prefix (preferring owners that have a path out).
    pub fn owner_router(&self, ip: Ipv4Addr) -> (Option<Ipv4Addr>, Option<ExitPath>) {
        let direct = self.router_with_interface(ip);
        let cands: Vec<Ipv4Addr> = match direct {
            Some(r) => vec![r],
            None => self.locate(ip).1.iter().map(|o| o.router).collect(),
        };
        for &r in &cands {
            if let Some(p) = self.best_exit_path(r) {
                return (Some(r), Some(p));
            }
        }
        let fallback = cands.iter().copied().find(|r| self.routers.contains_key(r)).or(cands.first().copied());
        (fallback, None)
    }

    /// Router that traffic for a forwarding address is sent to: the router
    /// owning that interface, else the owner of the intra-area prefix.
    fn forward_owner(&self, ip: Ipv4Addr) -> Option<Ipv4Addr> {
        self.router_with_interface(ip).or_else(|| {
            self.prefixes
                .iter()
                .filter(|p| p.kind != PrefixKind::External && p.prefix.contains(ip))
                .max_by_key(|p| p.prefix.len)
                .map(|p| p.router)
        })
    }

    /// Router that owns the interface address `ip`, if any.
    pub fn router_with_interface(&self, ip: Ipv4Addr) -> Option<Ipv4Addr> {
        if self.routers.contains_key(&ip) {
            return Some(ip);
        }
        self.routers.values().find(|r| r.interfaces.contains(&ip)).map(|r| r.id)
    }

    /// Dijkstra from `src`; returns (dist, predecessor-with-edge).
    fn spf(
        &self,
        src: Ipv4Addr,
        banned_edge: Option<LinkId>,
        banned_node: Option<Ipv4Addr>,
    ) -> (HashMap<Ipv4Addr, u32>, HashMap<Ipv4Addr, (Ipv4Addr, Edge)>) {
        let mut dist: HashMap<Ipv4Addr, u32> = HashMap::new();
        let mut prev: HashMap<Ipv4Addr, (Ipv4Addr, Edge)> = HashMap::new();
        let mut heap = BinaryHeap::new();
        if !self.routers.contains_key(&src) {
            return (dist, prev);
        }
        dist.insert(src, 0);
        heap.push(Reverse((0u32, src)));
        while let Some(Reverse((d, u))) = heap.pop() {
            if dist.get(&u).map_or(false, |&best| d > best) {
                continue;
            }
            let Some(r) = self.routers.get(&u) else { continue };
            for e in &r.edges {
                if banned_edge.map_or(false, |(a, b, n)| {
                    n == e.network && ((a == u && b == e.to) || (a == e.to && b == u))
                })
                    || banned_node == Some(e.to)
                {
                    continue;
                }
                let nd = d.saturating_add(e.cost);
                if dist.get(&e.to).map_or(true, |&cur| nd < cur) {
                    dist.insert(e.to, nd);
                    prev.insert(e.to, (u, e.clone()));
                    heap.push(Reverse((nd, e.to)));
                }
            }
        }
        (dist, prev)
    }

    pub fn reachable_from(&self, src: Ipv4Addr) -> BTreeSet<Ipv4Addr> {
        self.spf(src, None, None).0.into_keys().collect()
    }

    /// Router the default route actually forwards to (honours the forwarding address).
    pub fn exit_target(&self, e: &Exit) -> Ipv4Addr {
        e.forward.and_then(|f| self.forward_owner(f)).unwrap_or(e.router)
    }

    /// Best path from `src` to a default-route originator, using OSPF
    /// preference rules: E1 routes beat E2; E2 compares external metric first.
    pub fn best_exit_path(&self, src: Ipv4Addr) -> Option<ExitPath> {
        self.best_exit_path_with(src, None, None)
    }

    fn best_exit_path_with(
        &self,
        src: Ipv4Addr,
        banned_edge: Option<LinkId>,
        banned_node: Option<Ipv4Addr>,
    ) -> Option<ExitPath> {
        let (dist, prev) = self.spf(src, banned_edge, banned_node);
        let mut best: Option<((u8, u32, u32), &Exit, Ipv4Addr)> = None;
        for e in &self.exits {
            let target = self.exit_target(e);
            if Some(target) == banned_node || Some(e.router) == banned_node {
                continue;
            }
            let Some(&d) = dist.get(&target) else { continue };
            let key = if e.e1 { (0, d.saturating_add(e.metric), 0) } else { (1, e.metric, d) };
            if best.as_ref().map_or(true, |(k, _, _)| key < *k) {
                best = Some((key, e, target));
            }
        }
        let (_, exit, target) = best?;
        let mut hops = Vec::new();
        let mut cur = target;
        loop {
            let cost = dist[&cur];
            match prev.get(&cur) {
                Some((p, edge)) => {
                    hops.push(Hop {
                        router: cur,
                        cost,
                        via: Some((edge.local_if, edge.remote_if)),
                        network: edge.network,
                    });
                    cur = *p;
                }
                None => {
                    hops.push(Hop { router: cur, cost, via: None, network: None });
                    break;
                }
            }
        }
        hops.reverse();
        let cost = dist[&target];
        Some(ExitPath { exit: exit.clone(), hops, cost })
    }

    /// Links and routers on `path` whose failure would leave `src` without any
    /// default route.
    pub fn single_points_of_failure(&self, path: &ExitPath) -> (Vec<(Ipv4Addr, Ipv4Addr)>, Vec<Ipv4Addr>) {
        // Note: a banned link only removes that one adjacency, so parallel
        // links between the same routers still count as a backup.
        let src = path.hops[0].router;
        let mut links = Vec::new();
        let mut nodes = Vec::new();
        for w in path.hops.windows(2) {
            let (a, b) = (w[0].router, w[1].router);
            if self.best_exit_path_with(src, Some((a, b, w[1].network)), None).is_none() {
                links.push((a, b));
            }
        }
        for h in path.hops.iter().skip(1) {
            if self.best_exit_path_with(src, None, Some(h.router)).is_none() {
                nodes.push(h.router);
            }
        }
        (links, nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Topology {
        let r = include_str!("../tests/fixtures/router.json");
        let n = include_str!("../tests/fixtures/network.json");
        let e = include_str!("../tests/fixtures/external.json");
        Topology::from_json(r, n, e).unwrap()
    }

    fn ip(s: &str) -> Ipv4Addr {
        s.parse().unwrap()
    }

    #[test]
    fn ipv6_prefixes_from_ospf6() {
        let intra = r#"{"areaScopedLinkStateDb":[{"areaId":"0.0.0.0","lsa":[
            {"advertisingRouter":"10.107.10.65","age":850,"prefix":[
              {"prefix":"2a01:168:a00:1500::/64","metric":1},
              {"prefix":"2a01:168:a00:1500:6e3b:6bff:fed0:f74a/128","metric":0}]},
            {"advertisingRouter":"10.107.9.9","age":3600,"prefix":[{"prefix":"2a01:168:999::/48"}]}]}]}"#;
        let ext = r#"{"asScopedLinkStateDb":[{"lsa":[
            {"advertisingRouter":"10.107.2.1","age":572,"prefix":"2a01:168:200:100::/56"},
            {"advertisingRouter":"10.107.2.1","age":10,"prefix":"2000::/3"},
            {"advertisingRouter":"10.107.99.1","age":10,"prefix":"2a01:168::/29"},
            {"advertisingRouter":"not-an-ip","age":10,"prefix":"2a01:168:300::/48"}]}]}"#;
        let p = prefixes6_from_json(intra, ext).unwrap();
        let texts: Vec<String> = p.iter().map(Prefix6Owner::text).collect();
        assert!(texts.contains(&"2a01:168:a00:1500::/64".to_string()));
        assert!(texts.contains(&"2a01:168::/29".to_string()));
        assert!(!texts.iter().any(|t| t == "2000::/3"), "global route must not count");
        assert!(!texts.iter().any(|t| t.starts_with("2a01:168:999:")), "MaxAge LSA must not count");
        assert!(!texts.iter().any(|t| t.starts_with("2a01:168:300:")), "bad router id must not count");
        assert_eq!(p.iter().find(|x| x.len == 64).unwrap().router, ip("10.107.10.65"));
    }

    #[test]
    fn prefix_math() {
        let p = Prefix::new(ip("10.107.5.77"), 24);
        assert_eq!(p.addr, ip("10.107.5.0"));
        assert!(p.contains(ip("10.107.5.200")));
        assert!(!p.contains(ip("10.107.6.1")));
        assert!(Prefix::new(ip("1.2.3.4"), 0).contains(ip("9.9.9.9")));
    }

    #[test]
    fn builds_bidirectional_graph() {
        let t = fixture();
        assert_eq!(t.routers.len(), 5);
        // 10.107.5.1 advertises a link to the C/E network, but E does not
        // advertise it back -> no edge.
        let c = &t.routers[&ip("10.107.5.1")];
        assert!(c.edges.iter().all(|e| e.to != ip("10.107.9.1")));
        assert_eq!(t.exits.len(), 2);
    }

    #[test]
    fn locates_customer_ip() {
        let t = fixture();
        let (p, owners) = t.locate(ip("10.107.5.130"));
        assert_eq!(p, Some(Prefix::new(ip("10.107.5.128"), 26)));
        assert_eq!(owners[0].router, ip("10.107.5.1"));
        assert_eq!(t.locate(ip("192.168.1.1")).1.len(), 0);
    }

    #[test]
    fn prefers_e1_exit_and_finds_path() {
        let t = fixture();
        let p = t.best_exit_path(ip("10.107.5.1")).unwrap();
        let routers: Vec<_> = p.hops.iter().map(|h| h.router).collect();
        // C -> B -> A(E1 exit) beats C -> B -> D (E2 exit)
        assert_eq!(routers, vec![ip("10.107.5.1"), ip("10.107.2.1"), ip("10.107.1.1")]);
        assert!(p.exit.e1);
    }

    #[test]
    fn detects_single_points_of_failure() {
        let t = fixture();
        let p = t.best_exit_path(ip("10.107.5.1")).unwrap();
        let (links, nodes) = t.single_points_of_failure(&p);
        // C has only one uplink (to B); B -> A can fail over to D.
        assert_eq!(links, vec![(ip("10.107.5.1"), ip("10.107.2.1"))]);
        assert_eq!(nodes, vec![ip("10.107.2.1")]);
    }

    #[test]
    fn summaries_locate_through_the_border_router() {
        let mut t = fixture();
        let abr = *t.routers.keys().next().unwrap();
        t.add_summaries(&format!(
            r#"{{"summaryLinkStates":{{"areas":{{"0.0.0.0":[
                {{"lsaAge":10,"linkStateId":"10.250.10.64","advertisingRouter":"{abr}","networkMask":28}},
                {{"lsaAge":3600,"linkStateId":"10.250.20.0","advertisingRouter":"{abr}","networkMask":24}}]}}}}}}"#
        ))
        .unwrap();
        let (prefix, owners) = t.locate(ip("10.250.10.70"));
        assert_eq!(prefix.unwrap().to_string(), "10.250.10.64/28");
        assert_eq!((owners[0].router, &owners[0].kind), (abr, &PrefixKind::Summary));
        assert!(t.locate(ip("10.250.20.5")).1.is_empty(), "MaxAge summaries are ignored");
    }

    #[test]
    fn externals_locate_but_aggregates_do_not() {
        let t = fixture();
        // 10.200.0.0/16 is an aggregate (< MIN_EXTERNAL_LEN): not used.
        assert!(t.locate(ip("10.200.1.1")).0.is_none());
        // 10.250.1.0/24 is redistributed by D with forward address on B's link.
        let (p, owners) = t.locate(ip("10.250.1.9"));
        assert_eq!(p, Some(Prefix::new(ip("10.250.1.0"), 24)));
        assert_eq!(owners[0].kind, PrefixKind::External);
        assert_eq!(owners[0].router, ip("10.107.2.1"));
    }

    #[test]
    fn parallel_links_are_a_backup() {
        use serde_json::json;
        let tr = |dr: &str, ifa: &str| json!({"linkType": "a Transit Network", "designatedRouterAddress": dr, "routerInterfaceAddress": ifa, "tos0Metric": 10});
        let r = json!({"routerId": "10.0.0.1", "routerLinkStates": {"areas": {"0.0.0.0": [
            {"linkStateId": "10.0.0.1", "routerLinks": {"a": tr("10.9.0.1", "10.9.0.1"), "b": tr("10.9.1.1", "10.9.1.1")}},
            {"linkStateId": "10.0.0.2", "routerLinks": {"a": tr("10.9.0.1", "10.9.0.2"), "b": tr("10.9.1.1", "10.9.1.2")}}
        ]}}});
        let net = |dr: &str| json!({"linkStateId": dr, "networkMask": 30, "attchedRouters": {"10.0.0.1": {}, "10.0.0.2": {}}});
        let n = json!({"networkLinkStates": {"areas": {"0.0.0.0": [net("10.9.0.1"), net("10.9.1.1")]}}});
        let e = json!({"asExternalLinkStates": [{"linkStateId": "0.0.0.0", "advertisingRouter": "10.0.0.1", "networkMask": 0, "metricType": "E2", "metric": 1}]});
        let t = Topology::from_json(&r.to_string(), &n.to_string(), &e.to_string()).unwrap();
        let p = t.best_exit_path(ip("10.0.0.2")).unwrap();
        let (links, _) = t.single_points_of_failure(&p);
        assert!(links.is_empty(), "{links:?}");
    }

    #[test]
    fn isolated_router_is_unreachable() {
        let t = fixture();
        assert!(!t.reachable_from(ip("10.107.1.1")).contains(&ip("10.107.9.1")));
        assert!(t.best_exit_path(ip("10.107.9.1")).is_none());
    }
}
