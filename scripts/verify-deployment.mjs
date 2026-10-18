#!/usr/bin/env node
// Checks a deployed hkdiag from the outside, e.g.
//   node scripts/verify-deployment.mjs https://diag.hkfree.org
// Read-only, except one trial outage report, which the server checks all the
// way to its database and then throws away (X-Hkdiag-Trial). Exits non-zero
// if anything fails. Needs Node 18+ (fetch), nothing else.

const base = (process.argv[2] || 'https://diag.hkfree.org').replace(/\/$/, '');
const origin = new URL(base).origin;
let failed = 0;

function report(ok, name, detail = '') {
  if (!ok) failed++;
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? `  (${detail})` : ''}`);
}

async function get(path, init = {}) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), 20_000);
  try {
    const res = await fetch(base + path, { ...init, signal: ctrl.signal, redirect: 'manual' });
    const text = await res.text();
    let json = null;
    try { json = JSON.parse(text); } catch { /* not JSON */ }
    return { status: res.status, headers: res.headers, text, json };
  } catch (e) {
    return { status: 0, headers: new Headers(), text: String(e.message || e), json: null };
  } finally {
    clearTimeout(timer);
  }
}

async function check(name, fn) {
  try {
    const [ok, detail] = await fn();
    report(ok, name, detail);
  } catch (e) {
    report(false, name, `error: ${e.message}`);
  }
}

const post = (body, headers) => get('/api/reports', { method: 'POST', headers: { 'Content-Type': 'application/json', ...headers }, body: JSON.stringify(body) });

console.log(`Checking ${base}\n`);

// --- The app ---------------------------------------------------------------
await check('the app loads, with its security headers', async () => {
  const r = await get('/');
  const csp = r.headers.get('content-security-policy') || '';
  return [r.status === 200 && r.text.includes('/app.js') && csp.includes("default-src 'none'"), `HTTP ${r.status}`];
});

await check('every file the service worker caches is served', async () => {
  const sw = await get('/sw.js');
  const shell = JSON.parse((/const SHELL = (\[[^\]]*\])/.exec(sw.text) || [])[1]?.replace(/'/g, '"') || '[]');
  const bad = [];
  for (const p of shell) if ((await get(p)).status !== 200) bad.push(p);
  return [sw.status === 200 && shell.length > 5 && !bad.length, bad.length ? `missing: ${bad.join(', ')}` : `${shell.length} files`];
});

// --- Who the client is (the proxy must pass the address on) ----------------
await check('the server sees the client\'s own address, not the proxy\'s', async () => {
  const r = await get('/api/whoami', { headers: { Origin: origin } });
  const ip = r.json && r.json.ip;
  const proxy = ['127.0.0.1', '::1'].includes(ip);
  return [r.status === 200 && Boolean(ip) && !proxy, ip ? 'an address came through' : 'no address: is the proxy header configured?'];
});

// --- The network map -------------------------------------------------------
let network = null;
await check('the network map is fresh', async () => {
  const r = await get('/topology.json');
  network = r.json;
  const age = r.json ? Math.floor(Date.now() / 1000) - r.json.generated_at : null;
  const routers = r.json ? Object.keys(r.json.routers || {}).length : 0;
  return [r.status === 200 && routers > 0 && age !== null && age < 600, `${routers} routers, ${age ?? '?'} s old`];
});

// --- Outage reports ---------------------------------------------------------
await check('outage reports are enabled (the report database is open)', async () => {
  const r = await get('/api/outages');
  return [r.status === 200 && r.json && r.json.enabled === true, r.json ? `enabled: ${r.json.enabled}` : `HTTP ${r.status}`];
});

await check('the app\'s report request passes the proxy and the origin check', async () => {
  const r = await post([], { Origin: origin });
  return [r.status === 200 && r.json && r.json.received === 0, `HTTP ${r.status}`];
});

await check('reports from another site are refused', async () => {
  const r = await post([], { Origin: 'https://example.org' });
  return [r.status === 403, `HTTP ${r.status}`];
});

await check('a trial report would be stored (nothing is kept)', async () => {
  const now = Math.floor(Date.now() / 1000);
  const exit = network && network.exits && network.exits[0] ? network.exits[0].target : null;
  const report = {
    id: `trial-${now}-${Math.random().toString(16).slice(2, 10)}`,
    detected_at: now - 1800, resolved_at: now - 600, verdict: 'backbone',
    my_ip: null, ap: null, first_bad: exit, last_ok: null, path: exit ? [exit] : [], snapshot_at: now - 1900,
  };
  // First a report no server would store: a server that doesn't know trial
  // reports refuses it without keeping anything, and then the real one isn't sent.
  const probe = await post([{ ...report, id: `${report.id}-probe`, verdict: 'works' }], { Origin: origin, 'X-Hkdiag-Trial': '1' });
  if (!(probe.json && probe.json.trial === true)) return [false, 'this server doesn\'t know trial reports (older version); nothing was sent'];
  const r = await post([report], { Origin: origin, 'X-Hkdiag-Trial': '1' });
  const b = r.json || {};
  return [r.status === 200 && b.would_store === 1, r.status === 200 ? `would store ${b.would_store}, refused: ${JSON.stringify(b.refused)}` : `HTTP ${r.status}: ${r.text.slice(0, 80)}`];
});

// --- Network monitoring and the power monitor -------------------------------
await check('network monitoring\'s outages are being recorded', async () => {
  const r = await get(`/api/v1/measured?since=${Math.floor(Date.now() / 1000) - 86_400}`);
  const list = (r.json && r.json.measured) || [];
  return [r.status === 200 && list.length > 0, `${list.length} in the last day`];
});

await check('a router\'s measurement history comes from network monitoring', async () => {
  const exit = network && network.exits && network.exits[0] ? network.exits[0].target : null;
  if (!exit) return [false, 'no exit in the map'];
  const r = await get(`/api/v1/history?ip=${exit}`);
  const n = r.json ? r.json.rtt_ms.filter((v) => v !== null).length : 0;
  return [r.status === 200 && n > 0, r.json ? `${exit} measured at ${r.json.measured_ip}, ${n} points` : `HTTP ${r.status}`];
});

// --- The public API ---------------------------------------------------------
await check('the API catalog points to the API description', async () => {
  const c = await get('/.well-known/api-catalog');
  const href = c.json && c.json.linkset && c.json.linkset[0]['service-desc'][0].href;
  const d = href ? await get(href) : null;
  return [c.status === 200 && d && d.status === 200 && d.json.openapi.startsWith('3.'), href || 'no link'];
});

console.log(failed ? `\n${failed} check(s) failed` : '\nAll checks passed');
process.exit(failed ? 1 : 0);
