// On-device diagnosis engine: OSPF topology analysis on a snapshot produced
// by `hkdiag export`. Pure functions, no DOM, no network: unit-tested with
// `node --test web/test/`. Mirrors src/ospf.rs and src/diagnose.rs.
//
// Findings are returned as { sev, key, params } and translated by the UI.

export const FORMAT_VERSION = 1;

const IPV4 = /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;

export function isIPv4(s) {
  return typeof s === 'string' && IPV4.test(s);
}

export function ipToInt(ip) {
  return ip.split('.').reduce((acc, o) => ((acc << 8) | Number(o)) >>> 0, 0);
}

function maskOf(len) {
  return len === 0 ? 0 : (0xffffffff << (32 - len)) >>> 0;
}

export function parsePrefix(s) {
  if (typeof s !== 'string') return null;
  const [addr, l] = s.split('/');
  const len = Number(l);
  if (!isIPv4(addr) || !Number.isInteger(len) || len < 0 || len > 32) return null;
  const mask = maskOf(len);
  return { text: s, net: (ipToInt(addr) & mask) >>> 0, len, mask };
}

function contains(p, ipInt) {
  return ((ipInt & p.mask) >>> 0) === p.net;
}

/**
 * Validates and indexes a snapshot. Throws on anything unexpected, so a
 * corrupt or tampered cache can never reach the probes or the UI.
 */
export function loadSnapshot(raw) {
  if (!raw || raw.version !== FORMAT_VERSION || typeof raw.routers !== 'object') {
    throw new Error('unsupported snapshot format');
  }
  const routers = new Map();
  const ids = new Set(Object.keys(raw.routers));
  for (const [id, r] of Object.entries(raw.routers)) {
    if (!isIPv4(id) || !r || !Array.isArray(r.links)) throw new Error(`bad router ${id}`);
    const links = r.links.map((l) => {
      const [to, cost, net] = Array.isArray(l) ? l : [];
      if (!isIPv4(to) || !Number.isFinite(cost) || cost < 0) throw new Error(`bad link of ${id}`);
      return { to, cost, net: net === null || net === undefined ? null : String(net) };
    }).filter((l) => ids.has(l.to)); // links to unknown routers cannot be walked
    routers.set(id, {
      id,
      name: typeof r.name === 'string' ? r.name : null,
      age: Number(r.age) || 0,
      advertised: Number(r.advertised) || 0,
      links,
      ifaces: (r.ifaces || []).filter(isIPv4),
      sojka: typeof r.sojka === 'string' ? r.sojka : null,
    });
  }
  const prefixes = [];
  for (const [p, router, kind] of raw.prefixes || []) {
    const pre = parsePrefix(p);
    if (pre && isIPv4(router)) prefixes.push({ ...pre, router, kind });
  }
  const exits = (raw.exits || [])
    .filter((e) => isIPv4(e.router) && isIPv4(e.target))
    .map((e) => ({ router: e.router, target: e.target, e1: Boolean(e.e1), metric: Number(e.metric) || 0 }));
  const previous = [];
  for (const [p, router, lastSeen] of raw.previous || []) {
    const pre = parsePrefix(p);
    if (pre && isIPv4(router)) previous.push({ ...pre, router, lastSeen: Number(lastSeen) || 0 });
  }
  const ifaceOwner = new Map();
  // First router wins, like Rust's router_with_interface (routers are sorted by id).
  for (const r of routers.values()) for (const a of r.ifaces) if (!ifaceOwner.has(a)) ifaceOwner.set(a, r.id);
  return {
    generatedAt: Number(raw.generated_at) || 0,
    root: isIPv4(raw.root) && routers.has(raw.root) ? raw.root : null,
    routers,
    prefixes,
    exits,
    previous,
    ifaceOwner,
  };
}

/** All items whose prefix is the most specific one containing `ip`. */
function longestMatch(items, ip) {
  const x = ipToInt(ip);
  let best = -1;
  for (const p of items) if (p.len > best && contains(p, x)) best = p.len;
  return best < 0 ? [] : items.filter((p) => p.len === best && contains(p, x));
}

export function locate(topo, ip) {
  if (topo.routers.has(ip)) return { prefix: `${ip}/32`, owners: [] };
  const owners = longestMatch(topo.prefixes, ip);
  return { prefix: owners.length ? owners[0].text : null, owners };
}

/** Minimal binary min-heap of [dist, node]. */
class Heap {
  constructor() { this.a = []; }
  get size() { return this.a.length; }
  push(x) {
    const a = this.a;
    a.push(x);
    let i = a.length - 1;
    while (i > 0) {
      const p = (i - 1) >> 1;
      if (a[p][0] <= a[i][0]) break;
      [a[p], a[i]] = [a[i], a[p]];
      i = p;
    }
  }
  pop() {
    const a = this.a;
    const top = a[0];
    const last = a.pop();
    if (a.length) {
      a[0] = last;
      let i = 0;
      for (;;) {
        const l = 2 * i + 1;
        const r = l + 1;
        let m = i;
        if (l < a.length && a[l][0] < a[m][0]) m = l;
        if (r < a.length && a[r][0] < a[m][0]) m = r;
        if (m === i) break;
        [a[m], a[i]] = [a[i], a[m]];
        i = m;
      }
    }
    return top;
  }
}

/** Dijkstra from `src`. `ban` = { a, b, net } removes one specific adjacency; `banNode` a router. */
export function spf(topo, src, ban = null, banNode = null) {
  const dist = new Map();
  const prev = new Map();
  if (!topo.routers.has(src)) return { dist, prev };
  dist.set(src, 0);
  const heap = new Heap();
  heap.push([0, src]);
  while (heap.size) {
    const [du, u] = heap.pop();
    if (du > dist.get(u)) continue;
    const r = topo.routers.get(u);
    if (!r) continue;
    for (const e of r.links) {
      if (banNode === e.to) continue;
      if (ban && ban.net === e.net && ((ban.a === u && ban.b === e.to) || (ban.a === e.to && ban.b === u))) continue;
      const nd = du + e.cost;
      if (!dist.has(e.to) || nd < dist.get(e.to)) {
        dist.set(e.to, nd);
        prev.set(e.to, { from: u, net: e.net });
        heap.push([nd, e.to]);
      }
    }
  }
  return { dist, prev };
}

export function reachableFrom(topo, src) {
  return new Set(spf(topo, src).dist.keys());
}

/** OSPF route preference: E1 beats E2; E2 compares the external metric first. */
function exitKey(e, d) {
  return e.e1 ? [0, d + e.metric, 0] : [1, e.metric, d];
}

function lessKey(a, b) {
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] < b[i];
  return false;
}

export function bestExitPath(topo, src, ban = null, banNode = null) {
  const { dist, prev } = spf(topo, src, ban, banNode);
  let best = null;
  for (const e of topo.exits) {
    if (banNode === e.target || banNode === e.router) continue;
    if (!dist.has(e.target)) continue;
    const key = exitKey(e, dist.get(e.target));
    if (!best || lessKey(key, best.key)) best = { key, exit: e };
  }
  if (!best) return null;
  const hops = [];
  let cur = best.exit.target;
  for (;;) {
    const p = prev.get(cur);
    hops.push({ router: cur, cost: dist.get(cur), net: p ? p.net : null });
    if (!p) break;
    cur = p.from;
  }
  hops.reverse();
  return { exit: best.exit, hops, cost: dist.get(best.exit.target) };
}

/** True if `ip` belongs to the network described by the snapshot. */
export function inNetwork(topo, ip) {
  if (!isIPv4(ip)) return false;
  return topo.routers.has(ip) || topo.ifaceOwner.has(ip) || locate(topo, ip).owners.length > 0;
}

/** Responsible router for `ip`: the router itself, the interface owner, or the prefix owner. */
export function ownerRouter(topo, ip) {
  let cands;
  if (topo.routers.has(ip)) cands = [ip];
  else if (topo.ifaceOwner.has(ip)) cands = [topo.ifaceOwner.get(ip)];
  else cands = locate(topo, ip).owners.map((o) => o.router);
  for (const r of cands) {
    const path = bestExitPath(topo, r);
    if (path) return { router: r, path };
  }
  const known = cands.find((r) => topo.routers.has(r));
  return { router: known || cands[0] || null, path: null };
}

export function singlePointsOfFailure(topo, path) {
  const src = path.hops[0].router;
  const links = [];
  const nodes = [];
  for (let i = 1; i < path.hops.length; i++) {
    const a = path.hops[i - 1].router;
    const b = path.hops[i].router;
    if (!bestExitPath(topo, src, { a, b, net: path.hops[i].net })) links.push([a, b]);
  }
  for (const h of path.hops.slice(1)) {
    if (!bestExitPath(topo, src, null, h.router)) nodes.push(h.router);
  }
  return { links, nodes };
}

const SEV_ORDER = { critical: 0, warning: 1, info: 2 };

/**
 * Snapshot-only analysis of `ip`: where it lives, its path out, and what the
 * topology says is wrong. Device probes are combined with this in the UI.
 */
export function analyze(topo, ip) {
  const findings = [];
  const add = (sev, key, params = {}) => findings.push({ sev, key, params });
  const name = (r) => (topo.routers.get(r) || {}).name || null;
  const { prefix } = locate(topo, ip);
  const { router, path } = ownerRouter(topo, ip);

  if (!router) {
    const prevs = longestMatch(topo.previous, ip).map((p) => ({
      prefix: p.text, router: p.router, name: name(p.router), lastSeen: p.lastSeen, present: topo.routers.has(p.router),
    }));
    add('critical', 'notRouted', { ip, previous: prevs });
  } else {
    let cutOff = false;
    const r = topo.routers.get(router);
    if (!r) {
      cutOff = true;
      add('critical', 'noLsa', { router, name: name(router) });
    } else if (topo.root && router !== topo.root && !reachableFrom(topo, topo.root).has(router)) {
      cutOff = true;
      add('critical', 'cutOff', { router, name: r.name, advertised: r.advertised, age: r.age });
    }
    if (!path && !cutOff) add('critical', 'noExit', { router, name: name(router), count: reachableFrom(topo, router).size });
  }

  const hops = [];
  if (path) {
    for (const h of path.hops) {
      const r = topo.routers.get(h.router);
      const sojka = r ? r.sojka : null;
      hops.push({ router: h.router, name: name(h.router), cost: h.cost, sojka });
      // On the path means OSPF neighbours confirm its links, so traffic flows
      // through it; it only does not answer Sojka's ping.
      if (sojka === 'Dead') add('warning', 'hopDead', { router: h.router, name: name(h.router) });
      else if (sojka === 'Lossy' || sojka === 'Slow') add('warning', 'hopDegraded', { router: h.router, name: name(h.router), health: sojka });
    }
    const spof = singlePointsOfFailure(topo, path);
    if (spof.links.length || spof.nodes.length) {
      add('info', 'spof', {
        links: spof.links.map(([a, b]) => ({ a, an: name(a), b, bn: name(b) })),
        nodes: spof.nodes.map((n) => ({ router: n, name: name(n) })),
      });
    }
    const reach = reachableFrom(topo, router);
    const reachable = topo.exits.filter((e) => reach.has(e.target)).length;
    if (topo.exits.length > 1 && reachable === 1) add('warning', 'singleExit', { reachable, total: topo.exits.length });
  }

  findings.sort((a, b) => SEV_ORDER[a.sev] - SEV_ORDER[b.sev]);
  return {
    ip,
    prefix,
    router,
    routerName: router ? name(router) : null,
    exit: path ? path.exit : null,
    cost: path ? path.cost : null,
    hops,
    findings,
  };
}
