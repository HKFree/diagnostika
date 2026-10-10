// The test network, as FRR's OSPF database in its JSON format.
//
//   10.107.77.0/25 ── AP 10.107.77.1 ── 10.107.76.1 ──┐
//                                                      ├── exit 10.107.99.1 ── internet
//   10.107.10.128/26 ── AP 10.107.10.149 ── 10.107.241.36 ┘

const ROUTERS = ['10.107.77.1', '10.107.76.1', '10.107.10.149', '10.107.241.36', '10.107.99.1'];
const LINKS = [
  ['10.107.77.1', '10.107.76.1', '10.107.0.0'],
  ['10.107.76.1', '10.107.99.1', '10.107.0.4'],
  ['10.107.10.149', '10.107.241.36', '10.107.0.8'],
  ['10.107.241.36', '10.107.99.1', '10.107.0.12'],
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

export const ALL_ROUTERS = ROUTERS;
