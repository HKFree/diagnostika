// The test network, as FRR's OSPF database in its JSON format.
//
//   10.107.77.0/25 ── AP 10.107.77.1 ── 10.107.76.1 ──┐
//                                                      ├── exit 10.107.99.1 ── internet
//   10.107.10.128/26 ── AP 10.107.10.149 ── 10.107.241.36 ┤
//                                    AP 10.107.2.1 ──┘   (IPv6 only: 2a01:168:200:100::/56)
//   border 10.107.10.97 ──┘   hidden part of the network behind it: 10.107.10.64/28 (summary-LSA)
//   10.107.10.65 (inside the hidden part) advertises 2a01:168:a00:1500::/64 over OSPFv3
//   10.107.200.5 advertises 2a01:168:c00::/64 over OSPFv3 and is nowhere in the IPv4 map

const ROUTERS = ['10.107.77.1', '10.107.76.1', '10.107.10.149', '10.107.241.36', '10.107.2.1', '10.107.10.97', '10.107.99.1'];
const LINKS = [
  ['10.107.77.1', '10.107.76.1', '10.107.0.0'],
  ['10.107.76.1', '10.107.99.1', '10.107.0.4'],
  ['10.107.10.149', '10.107.241.36', '10.107.0.8'],
  ['10.107.241.36', '10.107.99.1', '10.107.0.12'],
  ['10.107.2.1', '10.107.99.1', '10.107.0.16'],
  ['10.107.10.97', '10.107.99.1', '10.107.0.20'],
];
const MEMBER_SUBNETS = {
  '10.107.77.1': ['10.107.77.0', '255.255.255.128'],
  '10.107.10.149': ['10.107.10.128', '255.255.255.192'],
};
export const EXIT = '10.107.99.1';

function ip(n) {
  return [n >>> 24, (n >>> 16) & 255, (n >>> 8) & 255, n & 255].join('.');
}
function toInt(a) {
  return a.split('.').reduce((acc, o) => ((acc << 8) | Number(o)) >>> 0, 0);
}

export function routerLsas() {
  const lsas = ROUTERS.map((id) => {
    const links = { l0: { linkType: 'Stub Network', networkAddress: id, networkMask: '255.255.255.255', tos0Metric: 10 } };
    if (MEMBER_SUBNETS[id]) {
      const [net, mask] = MEMBER_SUBNETS[id];
      links.l1 = { linkType: 'Stub Network', networkAddress: net, networkMask: mask, tos0Metric: 10 };
    }
    LINKS.forEach(([a, b, net], i) => {
      const base = toInt(net);
      if (id === a) links[`t${i}`] = { linkType: 'a Transit Network', designatedRouterAddress: ip(base + 1), routerInterfaceAddress: ip(base + 1), tos0Metric: 10 };
      if (id === b) links[`t${i}`] = { linkType: 'a Transit Network', designatedRouterAddress: ip(base + 1), routerInterfaceAddress: ip(base + 2), tos0Metric: 10 };
    });
    return { lsaAge: 100, linkStateId: id, advertisingRouter: id, routerLinks: links };
  });
  return JSON.stringify({ routerId: EXIT, routerLinkStates: { areas: { '0.0.0.0': lsas } } });
}

export function networkLsas() {
  const nets = LINKS.map(([a, b, net]) => ({
    lsaAge: 100,
    linkStateId: ip(toInt(net) + 1),
    advertisingRouter: a,
    networkMask: 30,
    attchedRouters: { [a]: {}, [b]: {} },
  }));
  return JSON.stringify({ routerId: EXIT, networkLinkStates: { areas: { '0.0.0.0': nets } } });
}

export function externalLsas() {
  return JSON.stringify({
    routerId: EXIT,
    asExternalLinkStates: [{ lsaAge: 100, linkStateId: '0.0.0.0', advertisingRouter: EXIT, networkMask: 0, metricType: 'E2', metric: 10, forwardAddress: '0.0.0.0' }],
  });
}

/** Inter-area summaries: the border router 10.107.10.97 announces its hidden part. */
export function summaryLsas() {
  return JSON.stringify({ routerId: EXIT, summaryLinkStates: { areas: { '0.0.0.0': [
    { lsaAge: 100, lsaType: 'summary-LSA', linkStateId: '10.107.10.64', advertisingRouter: '10.107.10.97', networkMask: 28, tos0Metric: 3 },
  ] } } });
}

export const ALL_ROUTERS = ROUTERS;

/** OSPFv3 (ospf6d detail output): the AP's own /48, plus externals incl. the global route that must not count. */
export function ospf6IntraPrefix() {
  return JSON.stringify({ areaScopedLinkStateDb: [{ areaId: '0.0.0.0', lsa: [
    { type: 'Intra-Prefix', advertisingRouter: '10.107.77.1', age: 100, prefix: [{ prefix: '2a01:168:331::/48', metric: 1 }] },
    { type: 'Intra-Prefix', advertisingRouter: '10.107.10.65', age: 100, prefix: [{ prefix: '2a01:168:a00:1500::/64', metric: 1 }] },
    { type: 'Intra-Prefix', advertisingRouter: '10.107.200.5', age: 100, prefix: [{ prefix: '2a01:168:c00::/64', metric: 1 }] },
  ] }] });
}

export function ospf6External() {
  return JSON.stringify({ asScopedLinkStateDb: [{ lsa: [
    { type: 'AS-External', advertisingRouter: EXIT, age: 100, metric: 1, prefix: '2000::/3' },
    { type: 'AS-External', advertisingRouter: EXIT, age: 100, metric: 1, prefix: '2a01:168::/29' },
    { type: 'AS-External', advertisingRouter: '10.107.2.1', age: 100, metric: 20, prefix: '2a01:168:200:100::/56' },
  ] }] });
}
