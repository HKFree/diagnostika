//! OSPF link-state database (FRR JSON) -> topology graph, IP location and
//! shortest paths to the default-route originators.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap};
use std::net::{Ipv4Addr, Ipv6Addr};

use crate::lsdb::{self, ExternalLsa, Link, Lsdb, NetworkLsa, RouterLsa, SummaryLsa};

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
    lsa_seq_number: Option<String>,
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
    #[serde(default)]
    lsa_seq_number: Option<String>,
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
    #[serde(default)]
    lsa_seq_number: Option<String>,
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
    #[serde(default)]
    lsa_seq_number: Option<String>,
    network_mask: u8,
}

fn seq_of(s: &Option<String>) -> i32 {
    s.as_deref().map_or(i32::MIN, lsdb::parse_seq)
}

/// FRR's router, network and external records in the common form (all areas).
pub fn lsdb_from_frr(router: &str, network: &str, external: &str) -> Result<Lsdb> {
    let rdb: RouterDb = serde_json::from_str(router).context("parsing router LSAs")?;
    let ndb: NetworkDb = serde_json::from_str(network).context("parsing network LSAs")?;
    let edb: ExternalDb = serde_json::from_str(external).context("parsing external LSAs")?;
    let mut out = Lsdb { root: rdb.router_id, ..Default::default() };
    for (area, lsas) in rdb.router_link_states.areas {
        let Ok(area) = area.parse::<Ipv4Addr>() else { continue };
        for r in lsas {
            let links = r
                .router_links
                .values()
                .filter_map(|l| match l.link_type.as_str() {
                    "Stub Network" => Some(Link::Stub { net: l.network_address?, mask: l.network_mask?, metric: l.tos0_metric }),
                    "a Transit Network" => {
                        Some(Link::Transit { dr: l.designated_router_address?, iface: l.router_interface_address, metric: l.tos0_metric })
                    }
                    t if t.contains("point-to-point") => Some(Link::PointToPoint {
                        neighbor: l.neighbor_router_id?,
                        iface: l.router_interface_address,
                        metric: l.tos0_metric,
                    }),
                    _ => None,
                })
                .collect();
            out.routers.push(RouterLsa { area, id: r.link_state_id, seq: seq_of(&r.lsa_seq_number), age: r.lsa_age, links });
        }
    }
    for (area, lsas) in ndb.network_link_states.areas {
        let Ok(area) = area.parse::<Ipv4Addr>() else { continue };
        for n in lsas {
            out.networks.push(NetworkLsa {
                area,
                id: n.link_state_id,
                seq: seq_of(&n.lsa_seq_number),
                age: n.lsa_age,
                mask_len: n.network_mask,
                attached: n.attched_routers.keys().copied().collect(),
            });
        }
    }
    for e in edb.as_external_link_states {
        out.externals.push(ExternalLsa {
            id: e.link_state_id,
            originator: e.advertising_router,
            seq: seq_of(&e.lsa_seq_number),
            age: e.lsa_age,
            mask_len: e.network_mask,
            e1: e.metric_type.starts_with("E1"),
            metric: e.metric,
            forward: e.forward_address.filter(|a| !a.is_unspecified()),
        });
    }
    Ok(out)
}

/// FRR's summary records in the common form.
pub fn frr_summaries(summary: &str) -> Result<Vec<SummaryLsa>> {
    let sdb: SummaryDb = serde_json::from_str(summary).context("parsing summary LSAs")?;
    let mut out = Vec::new();
    for (area, lsas) in sdb.summary_link_states.areas {
        let Ok(area) = area.parse::<Ipv4Addr>() else { continue };
        for s in lsas {
            out.push(SummaryLsa {
                area,
                id: s.link_state_id,
                originator: s.advertising_router,
                seq: seq_of(&s.lsa_seq_number),
                age: s.lsa_age,
                mask_len: s.network_mask,
            });
        }
    }
    Ok(out)
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
        Prefix { addr: Ipv4Addr::from(u32::from(addr) & crate::verified::mask(len)), len }
    }
    /// Proven in `verified::in_prefix`.
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        crate::verified::in_prefix(u32::from(self.addr), self.len, u32::from(ip))
    }
}

impl std::fmt::Display for Prefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.addr, self.len)
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
    /// How each extra (MikroTik) router answered when this map was built.
    pub sources: Vec<crate::mikrotik::SourceStatus>,
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
        Ok(Topology::from_lsdb(&lsdb_from_frr(router, network, external)?))
    }

    /// The map from an OSPF database, across all its areas: a border router's
    /// records in each area make one router with the links of all of them.
    pub fn from_lsdb(db: &Lsdb) -> Topology {
        let rlsas: Vec<&RouterLsa> = db.routers.iter().filter(|l| l.age < MAX_AGE).collect();
        let nlsas: Vec<&NetworkLsa> = db.networks.iter().filter(|l| l.age < MAX_AGE).collect();

        let mut topo = Topology::default();
        let nets: HashMap<(Ipv4Addr, Ipv4Addr), &NetworkLsa> = nlsas.iter().map(|n| ((n.area, n.id), *n)).collect();

        // Transit links per router keyed by area and DR address: (area, router, dr) -> (iface, cost)
        let mut transit_if: HashMap<(Ipv4Addr, Ipv4Addr, Ipv4Addr), (Option<Ipv4Addr>, u32)> = HashMap::new();
        // p2p links: (router, neighbour) -> (iface, cost)
        let mut p2p: HashMap<(Ipv4Addr, Ipv4Addr), (Option<Ipv4Addr>, u32)> = HashMap::new();

        for r in &rlsas {
            let router = topo.routers.entry(r.id).or_insert_with(|| Router {
                id: r.id,
                age: r.age,
                edges: Vec::new(),
                advertised_links: 0,
                interfaces: Vec::new(),
            });
            router.age = router.age.min(r.age);
            for l in &r.links {
                match *l {
                    Link::Stub { net, mask, .. } => {
                        topo.prefixes.push(PrefixOwner { prefix: Prefix::new(net, mask_len(mask)), router: r.id, kind: PrefixKind::Stub });
                    }
                    Link::Transit { dr, iface, metric } => {
                        router.advertised_links += 1;
                        router.interfaces.extend(iface);
                        transit_if.insert((r.area, r.id, dr), (iface, metric));
                    }
                    Link::PointToPoint { neighbor, iface, metric } => {
                        router.advertised_links += 1;
                        router.interfaces.extend(iface);
                        // Parallel p2p links to the same neighbour: keep the cheapest.
                        let e = p2p.entry((r.id, neighbor)).or_insert((iface, metric));
                        if metric < e.1 {
                            *e = (iface, metric);
                        }
                    }
                }
            }
        }

        // Transit edges: both routers must be attached in the network LSA and
        // advertise the link in their router LSA (bidirectional check).
        for (&(area, rid, dr), &(iface, cost)) in &transit_if {
            let Some(net) = nets.get(&(area, dr)) else { continue };
            if !net.attached.contains(&rid) {
                continue;
            }
            let prefix = Prefix::new(dr, net.mask_len);
            for &other in &net.attached {
                if other == rid {
                    continue;
                }
                if let Some(&(other_if, _)) = transit_if.get(&(area, other, dr)) {
                    if let Some(r) = topo.routers.get_mut(&rid) {
                        r.edges.push(Edge { to: other, cost, local_if: iface, remote_if: other_if, network: Some(prefix) });
                    }
                }
            }
        }
        for n in &nlsas {
            let prefix = Prefix::new(n.id, n.mask_len);
            for &rid in &n.attached {
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
            r.edges.dedup_by_key(|e| (e.to, e.network));
            r.interfaces.sort();
            r.interfaces.dedup();
        }
        // The SPF root is the router the (first) database was read from.
        topo.root = db.root.filter(|r| topo.routers.contains_key(r));

        let mut externals = Vec::new();
        for e in &db.externals {
            if e.age >= MAX_AGE {
                continue;
            }
            if e.mask_len == 0 && e.id == Ipv4Addr::UNSPECIFIED {
                topo.exits.push(Exit { router: e.originator, e1: e.e1, metric: e.metric, forward: e.forward });
            } else if e.mask_len >= MIN_EXTERNAL_LEN {
                let router = e.forward.and_then(|f| topo.forward_owner(f)).unwrap_or(e.originator);
                externals.push(PrefixOwner { prefix: Prefix::new(e.id, e.mask_len), router, kind: PrefixKind::External });
            }
        }
        topo.prefixes.extend(externals);
        topo.add_summary_records(&db.summaries);
        topo
    }

    /// Adds the backbone's summary records: prefixes of other areas, owned by
    /// their border router. Aggregates as broad as a /16 (or a default) would
    /// claim every address nothing else routes, like external aggregates.
    fn add_summary_records(&mut self, summaries: &[SummaryLsa]) {
        for s in summaries {
            if s.area != lsdb::BACKBONE || s.age >= MAX_AGE || s.mask_len > 32 || s.mask_len < MIN_EXTERNAL_LEN {
                continue;
            }
            self.prefixes.push(PrefixOwner { prefix: Prefix::new(s.id, s.mask_len), router: s.originator, kind: PrefixKind::Summary });
        }
        self.prefixes.sort_by_key(|p| (p.prefix, p.router));
        self.prefixes.dedup_by_key(|p| (p.prefix, p.router));
    }

    /// Adds the backbone's summary-LSAs: prefixes of other areas, owned by
    /// their border router. Summaries of areas we can't see are the only way
    /// to place those addresses at all.
    pub fn add_summaries(&mut self, summary: &str) -> Result<()> {
        self.add_summary_records(&frr_summaries(summary)?);
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
            // A router inside an area before its border router's summary of it.
            None => {
                let mut owners = self.locate(ip).1;
                owners.sort_by_key(|o| o.kind == PrefixKind::Summary);
                owners.iter().map(|o| o.router).collect()
            }
        };
        for &r in &cands {
            if let Some(p) = self.best_exit_path(r) {
                return (Some(r), Some(p));
            }
        }
        let fallback = cands.iter().copied().find(|r| self.routers.contains_key(r)).or(cands.first().copied());
        (fallback, None)
    }

    /// The router in the map responsible for an IPv6 address: the advertiser of
    /// the most specific OSPFv3 prefix longer than the /29 allocation, or, if
    /// that router is in a hidden part of the network, its border router.
    pub fn owner_router6(&self, ip: Ipv6Addr) -> Option<Ipv4Addr> {
        let x = u128::from(ip);
        let covers = |p: &&Prefix6Owner| {
            let mask = if p.len == 0 { 0 } else { u128::MAX << (128 - u32::from(p.len.min(128))) };
            p.len > MIN_PREFIX6_LEN && x & mask == u128::from(p.addr) & mask
        };
        let best = self.prefixes6.iter().filter(covers).map(|p| p.len).max()?;
        let owners: Vec<Ipv4Addr> = self.prefixes6.iter().filter(covers).filter(|p| p.len == best).map(|p| p.router).collect();
        owners.iter().copied().find(|r| self.routers.contains_key(r)).or_else(|| owners.iter().find_map(|&r| self.border_of(r)))
    }

    /// The border router of the most specific summary covering `ip`, even
    /// when a more specific backbone prefix covers it too.
    pub fn border_of(&self, ip: Ipv4Addr) -> Option<Ipv4Addr> {
        self.prefixes
            .iter()
            .filter(|p| p.kind == PrefixKind::Summary && p.prefix.contains(ip) && self.routers.contains_key(&p.router))
            .max_by_key(|p| p.prefix.len)
            .map(|p| p.router)
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
                {{"lsaAge":3600,"linkStateId":"10.250.20.0","advertisingRouter":"{abr}","networkMask":24}},
                {{"lsaAge":10,"linkStateId":"10.250.0.0","advertisingRouter":"{abr}","networkMask":16}}]}}}}}}"#
        ))
        .unwrap();
        let (prefix, owners) = t.locate(ip("10.250.10.70"));
        assert_eq!(prefix.unwrap().to_string(), "10.250.10.64/28");
        assert_eq!((owners[0].router, &owners[0].kind), (abr, &PrefixKind::Summary));
        assert!(t.locate(ip("10.250.20.5")).1.is_empty(), "MaxAge summaries and broad aggregates are ignored");
    }

    #[test]
    fn ipv6_owner_is_in_the_map_or_its_border_router() {
        let mut t = fixture();
        let (inside, abr) = { let mut k = t.routers.keys(); (*k.next().unwrap(), *k.next().unwrap()) };
        t.add_summaries(&format!(
            r#"{{"summaryLinkStates":{{"areas":{{"0.0.0.0":[{{"linkStateId":"10.250.10.64","advertisingRouter":"{abr}","networkMask":28}}]}}}}}}"#
        ))
        .unwrap();
        let p6 = |p: &str, router: Ipv4Addr| {
            let (a, l) = p.split_once('/').unwrap();
            Prefix6Owner { addr: a.parse().unwrap(), len: l.parse().unwrap(), router, kind: PrefixKind::Stub }
        };
        t.prefixes6 = vec![p6("2a01:168::/29", inside), p6("2a01:168:1::/48", inside), p6("2a01:168:2::/64", ip("10.250.10.65")), p6("2a01:168:3::/64", ip("10.251.0.1"))];
        let o = |s: &str| t.owner_router6(s.parse().unwrap());
        assert_eq!(o("2a01:168:1::5"), Some(inside));
        assert_eq!(o("2a01:168:2::5"), Some(abr));
        assert_eq!(o("2a01:168:3::5"), None, "neither in the map nor behind a border router");
        assert_eq!(o("2a01:168:7::5"), None, "the /29 allocation names no router");
        // A more specific backbone prefix over the hidden router's ID doesn't hide the border.
        t.prefixes.push(PrefixOwner { prefix: Prefix::new(ip("10.250.10.64"), 30), router: inside, kind: PrefixKind::Stub });
        assert_eq!(t.owner_router6("2a01:168:2::5".parse().unwrap()), Some(abr));
    }

    #[test]
    fn a_border_routers_view_adds_the_inside_of_its_area() {
        use crate::lsdb::{Link, Lsdb, NetworkLsa, RouterLsa, SummaryLsa};
        let mut t = fixture();
        let abr = *t.routers.keys().next().unwrap();
        let area = ip("10.250.10.0");
        let mut db = Lsdb::default();
        db.routers.push(RouterLsa { area, id: ip("10.250.10.65"), seq: 1, age: 10, links: vec![
            Link::Transit { dr: ip("10.250.10.58"), iface: Some(ip("10.250.10.57")), metric: 10 },
            Link::Stub { net: ip("10.250.10.64"), mask: ip("255.255.255.240"), metric: 1 },
        ] });
        db.routers.push(RouterLsa { area, id: abr, seq: 1, age: 10, links: vec![Link::Transit { dr: ip("10.250.10.58"), iface: Some(ip("10.250.10.58")), metric: 10 }] });
        db.networks.push(NetworkLsa { area, id: ip("10.250.10.58"), seq: 1, age: 10, mask_len: 30, attached: vec![abr, ip("10.250.10.65")] });
        db.summaries.push(SummaryLsa { area: crate::lsdb::BACKBONE, id: ip("10.250.10.64"), originator: abr, seq: 1, age: 10, mask_len: 28 });
        let mut frr = lsdb_from_frr(
            include_str!("../tests/fixtures/router.json"),
            include_str!("../tests/fixtures/network.json"),
            include_str!("../tests/fixtures/external.json"),
        )
        .unwrap();
        frr.merge(db);
        t = Topology::from_lsdb(&frr);
        assert!(t.routers[&abr].edges.iter().any(|e| e.to == ip("10.250.10.65")), "the border router keeps its backbone links and gains the area's");
        let (owner, path) = t.owner_router(ip("10.250.10.70"));
        assert_eq!(owner, Some(ip("10.250.10.65")), "the router inside before the border router's summary");
        let hops: Vec<Ipv4Addr> = path.expect("a path out").hops.iter().map(|h| h.router).collect();
        assert_eq!(&hops[..2], &[ip("10.250.10.65"), abr]);
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
