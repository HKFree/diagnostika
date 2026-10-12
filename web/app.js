// hkfree.org diagnostics PWA. Everything runs on this device:
//  * probes from this device (server, internet by IP and by name, routers on
//    the path by TCP timing, a control address),
//  * OSPF analysis of the cached network snapshot (engine.js),
//  * the verdict combining both.
// The server only provides /topology.json and /api/whoami; both are cached so
// the app also works offline.

import { analyze, isIPv4, isIPv6, loadSnapshot, networkOf } from './engine.js';
import { detectLang, LANGS, messages } from './i18n.js';
import * as ai from './assistant.js';
import { measure as measureSpeed } from './speedtest.js';

// ---------- helpers ----------

const $ = (sel) => document.querySelector(sel);

// Browser storage is a convenience only: it may be missing or throw.
const store = {
  get(key, fallback) {
    try {
      const v = localStorage.getItem(key);
      return v === null ? fallback : JSON.parse(v);
    } catch (_) {
      return fallback;
    }
  },
  set(key, value) {
    try {
      localStorage.setItem(key, JSON.stringify(value));
      return true;
    } catch (_) {
      return false; // unavailable or full
    }
  },
  remove(key) {
    try { localStorage.removeItem(key); } catch (_) { /* ignore */ }
  },
};

/** Builds an element; children are strings (as text, never HTML) or nodes. */
function el(tag, cls, ...children) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  for (const c of children) {
    if (c === null || c === undefined || c === false) continue;
    e.append(typeof c === 'string' ? document.createTextNode(c) : c);
  }
  return e;
}

const now = () => Math.floor(Date.now() / 1000);

async function timedFetch(url, options, ms) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), ms);
  const t0 = performance.now();
  try {
    const res = await fetch(url, { ...options, signal: ctrl.signal });
    return { ok: true, res, ms: performance.now() - t0 };
  } catch (_) {
    return { ok: false, aborted: ctrl.signal.aborted, ms: performance.now() - t0 };
  } finally {
    clearTimeout(timer);
  }
}

// ---------- language ----------

let lang = LANGS.includes(store.get('lang', null)) ? store.get('lang', null) : detectLang(navigator.languages || [navigator.language]);
let T = messages(lang);

function applyStaticTexts() {
  document.documentElement.lang = lang;
  document.title = T.appName;
  for (const node of document.querySelectorAll('[data-i18n]')) {
    const v = T[node.dataset.i18n];
    if (typeof v === 'string') node.textContent = v;
  }
  $('#my-ip').placeholder = T.addressPlaceholder;
  $('#view-ip').placeholder = T.addressPlaceholder;
  $('#lang').value = lang;
}

const ago = (ts) => T.ago(Math.max(0, now() - ts));

// ---------- probes ----------

const EXTERNAL = { cache: 'no-store', credentials: 'omit', referrerPolicy: 'no-referrer', mode: 'no-cors' };

/** Internet by IP address (no DNS involved). An opaque response means it worked. */
async function probeInternetIp() {
  const r = await timedFetch('https://1.1.1.1/cdn-cgi/trace', EXTERNAL, 5000);
  return r.ok ? { state: 'ok', ms: r.ms } : { state: 'bad' };
}

/** Internet by name: fails on DNS problems even when the IP check works. */
async function probeInternetName() {
  const urls = ['https://www.google.com/generate_204', 'https://cloudflare.com/cdn-cgi/trace'];
  const results = await Promise.all(urls.map((u) => timedFetch(u, EXTERNAL, 5000)));
  const good = results.find((r) => r.ok);
  return good ? { state: 'ok', ms: good.ms } : { state: 'bad' };
}

/**
 * A router, by TCP timing (browsers cannot ping). A quick answer — even a
 * refused connection or TLS error — means something answered; a timeout means
 * nothing did. Only meaningful compared with the baseline from a healthy run.
 */
async function probeHost(ip) {
  if (!isIPv4(ip)) return { cls: 'invalid' };
  const r = await timedFetch(`https://${ip}/`, EXTERNAL, 3000);
  if (r.ok || (!r.aborted && r.ms < 2500)) return { cls: 'answers', ms: r.ms };
  return { cls: 'silent', ms: r.ms };
}

/**
 * TEST-NET-1 exists nowhere: on a working network the probe times out. If it
 * "answers" instantly, errors are generated on this device (no route, no
 * link), so fast answers from routers prove nothing.
 */
const CONTROL_IP = '192.0.2.1';

/** Server reachability + this device's hkfree.org address in one request. */
async function fetchWhoami() {
  const r = await timedFetch('/api/whoami', { cache: 'no-store' }, 4000);
  // netError: nothing answered (vs. an HTTP error, e.g. a static deployment without whoami)
  if (!r.ok) return { state: 'bad', netError: true };
  if (!r.res.ok) return { state: 'bad' };
  try {
    const body = await r.res.json();
    return { state: 'ok', ms: r.ms, ip: isIPv4(body.ip) || isIPv6(body.ip) ? body.ip : null };
  } catch (_) {
    return { state: 'ok', ms: r.ms, ip: null };
  }
}

const EMAIL_RE = /^[^\s@<>"'?&;,:/()[\]\\]+@[^\s@<>"'?&;,:/()[\]\\]+\.[^\s@<>"'?&;,:/()[\]\\]+$/;

/**
 * This device's AP admins with e-mails. The server gives them only to a
 * device inside the AP; they are kept for when the network fails.
 */
async function refreshContact() {
  const r = await timedFetch('/api/contact', { cache: 'no-store' }, 4000);
  if (!r.ok || !r.res.ok) return;
  try {
    const body = await r.res.json();
    if (!isIPv4(body.router)) return;
    const admins = (Array.isArray(body.admins) ? body.admins : [])
      .filter((a) => a && typeof a.nick === 'string')
      .slice(0, 5)
      .map((a) => ({ nick: a.nick.slice(0, 100), email: typeof a.email === 'string' && EMAIL_RE.test(a.email) ? a.email : '' }));
    store.set('contact', { router: body.router, admins });
  } catch (_) { /* keep what we had */ }
}

/** Fetches and validates the snapshot; `cancel` aborts it early. */
async function fetchSnapshot(cancel) {
  const ctrl = new AbortController();
  cancel.then(() => ctrl.abort());
  const timer = setTimeout(() => ctrl.abort(), 15000);
  try {
    const res = await fetch('/topology.json', { cache: 'no-store', signal: ctrl.signal });
    if (!res.ok) return null;
    const raw = await res.json();
    return { raw, topo: loadSnapshot(raw) }; // throws on invalid data
  } catch (_) {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

// ---------- state ----------

function loadCachedSnapshot() {
  const raw = store.get('snapshot', null);
  try {
    return raw ? { raw, topo: loadSnapshot(raw) } : null;
  } catch (_) {
    return null; // stale format or corrupt: ignore
  }
}

let snap = loadCachedSnapshot();     // { raw, topo }
let snapFresh = false;               // fetched during the last check
const storedAuto = store.get('autoIp', null);
let autoIp = isIPv4(storedAuto) || isIPv6(storedAuto) ? storedAuto : null;
let manualIp = isIPv4(store.get('manualIp', null)) ? store.get('manualIp', null) : null;
let viewIp = null;                   // Path tab: another address, not persisted
// ip -> { cls: 'answers' | 'silent', misses: n }, learned only while everything works
let baseline = store.get('baseline', {});
if (typeof baseline !== 'object' || baseline === null || Array.isArray(baseline)) baseline = {};
let history = store.get('history', []);
if (!Array.isArray(history)) history = [];
// Outages detected on this device, waiting to be reported once the
// connection is back (see trackOutage / sendOutages).
let outages = store.get('outages', []);
if (!Array.isArray(outages)) outages = [];
let reportStatus = null;             // { key, n } shown under the verdict
let signalState = null;              // the antenna's signal, asked for in advanced diagnostics
let speedState = null;               // the connection's speed, measured on request
let publicOutages = store.get('publicOutages', null); // { at, enabled, outages }
let network = 'unknown';             // 'hkfree' | 'other' | 'unknown'
let recheckTimer = null;
let running = false;
let rerun = false;                   // address changed during a check: run again after it
let last = null;                     // last check results, re-rendered on language change

// analyze() runs many shortest-path searches: memoise per address and snapshot.
const analysisCache = new Map();
function analysisFor(ip) {
  if (!snap || !ip) return null;
  const key = `${ip}@${snap.topo.generatedAt}`;
  if (!analysisCache.has(key)) {
    if (analysisCache.size > 20) analysisCache.clear();
    analysisCache.set(key, analyze(snap.topo, ip));
  }
  return analysisCache.get(key);
}

const myIp = () => manualIp || autoIp;
const MISSES_BEFORE_SILENT = 3;
const baseCls = (ip) => (baseline[ip] && baseline[ip].cls) || null;

/** One timeout during a healthy run must not erase an "answers" baseline. */
function learn(ip, cls) {
  const b = baseline[ip] && typeof baseline[ip] === 'object' ? baseline[ip] : { cls, misses: 0 };
  if (cls === 'answers') {
    b.cls = 'answers';
    b.misses = 0;
  } else if (b.cls === 'answers') {
    b.misses = (b.misses || 0) + 1;
    if (b.misses >= MISSES_BEFORE_SILENT) b.cls = 'silent';
  } else {
    b.cls = 'silent';
  }
  baseline[ip] = b;
}

function probeTargets(analysis) {
  if (!analysis) return [];
  return analysis.hops.map((h, i) => ({
    ip: h.router,
    name: h.name,
    role: i === 0 ? (analysis.border ? 'border' : 'ap') : (analysis.exit && h.router === analysis.exit.target ? 'exit' : 'hop'),
  }));
}

// ---------- verdict ----------

function findingText(f) {
  const p = { ...f.params };
  if (f.key === 'notRouted') p.previous = p.previous.map((x) => ({ ...x, agoText: ago(x.lastSeen) }));
  if (f.key === 'split') p.links = p.links.map((x) => ({ ...x, agoText: ago(x.lastSeen) }));
  if (f.key === 'hopDegraded') p.healthText = T.sojka[p.health] || p.health;
  const fn = T.f[f.key];
  return fn ? fn(p) : [f.key, []];
}

/** Verdict for this device as { level, key, params }; texts come from i18n. */
function decide(r) {
  const d = (level, key, params = {}) => ({ level, key, params });
  if (r.device.state === 'bad') return d('bad', 'offline');
  if (r.localFailure) return d('bad', 'localFailure');
  if (r.serverOk) {
    if (r.inetDns.state === 'ok') return d('ok', 'ok');
    if (r.inetIp.state === 'ok') return d('warn', 'dns');
    // hkfree.org answers but the internet doesn't: the map may say why.
    const split = r.analysis && r.analysis.findings.find((f) => f.key === 'split');
    if (split) return d('bad', 'split', { count: split.params.count, links: split.params.links });
    return d('warn', 'inetDown');
  }
  if (r.internetOk) return d('warn', 'serverOnly');
  const targets = r.targets;
  if (!targets.length) return d('bad', 'noPath');
  // The cached snapshot may already explain the outage.
  const critical = r.analysis && r.analysis.findings.find((f) => f.sev === 'critical');
  if (critical) return d('bad', 'known', { snapshotAt: r.snapshotAt, finding: { key: critical.key, params: critical.params } });
  const firstLost = r.hops.findIndex((h) => h.verdict === 'lost');
  if (firstLost === 0) return d('bad', 'apDown', { ip: targets[0].ip, name: targets[0].name });
  if (firstLost > 0) {
    let lastOk = null;
    for (let i = firstLost - 1; i >= 0; i--) if (r.hops[i].verdict === 'ok') { lastOk = targets[i]; break; }
    const bad = targets[firstLost];
    return d('bad', 'backbone', { bad: { ip: bad.ip, name: bad.name }, lastOk: lastOk && { ip: lastOk.ip, name: lastOk.name } });
  }
  return d('bad', 'generic');
}

/** [title, text] of a verdict in the current language. */
function verdictText(v) {
  const m = T.v[v.key];
  if (!m) return [String(v.key), ''];
  if (v.key === 'known') {
    return m({ ago: ago(v.params.snapshotAt), title: findingText(v.params.finding)[0] });
  }
  return typeof m === 'function' ? m(v.params) : m;
}

// ---------- the check ----------

async function runCheck() {
  if (running) {
    rerun = true;
    return;
  }
  running = true;
  const btn = $('#run');
  btn.disabled = true;
  btn.textContent = T.checking;
  setVerdict('idle', T.checking, T.checkingText);
  $('#advice').hidden = true;
  $('#advanced').open = false;
  reportStatus = null;
  signalState = null;
  speedState = null;
  try {
    const r = {
      device: { state: navigator.onLine ? 'ok' : 'bad' },
      server: null, inetIp: null, inetDns: null, hops: [], targets: [],
    };
    last = r;
    renderChecks();
    const offline = r.device.state === 'bad';
    const bad = Promise.resolve({ state: 'bad' });

    // Phase 1: server (+ snapshot, address), internet, control — in parallel.
    // If nothing answers on the server, stop waiting for the snapshot.
    const whoP = offline ? bad : fetchWhoami();
    const cancelSnapshot = whoP.then((w) => (w.netError ? undefined : new Promise(() => {})));
    const [who, fresh, inetIp, inetDns, control] = await Promise.all([
      whoP,
      offline ? Promise.resolve(null) : fetchSnapshot(cancelSnapshot),
      offline ? bad : probeInternetIp(),
      offline ? bad : probeInternetName(),
      offline ? Promise.resolve({ cls: 'answers' }) : probeHost(CONTROL_IP),
    ]);
    r.inetIp = inetIp;
    r.inetDns = inetDns;
    r.internetOk = inetIp.state === 'ok' || inetDns.state === 'ok';
    // A static deployment may lack /api/whoami: a fresh snapshot also proves the server works.
    r.serverOk = who.state === 'ok' || Boolean(fresh);
    r.server = r.serverOk ? { state: 'ok', ms: who.ms } : { state: 'bad' };
    // "Nothing leaves this device" only when everything failed AND the control
    // address answered instantly (unless it normally does on this network).
    r.localFailure = !r.serverOk && !r.internetOk
      && control.cls === 'answers' && baseCls(CONTROL_IP) !== 'answers';
    snapFresh = Boolean(fresh);
    if (fresh) {
      snap = fresh;
      // Never fall back to a much older copy because the new one did not fit.
      if (!store.set('snapshot', fresh.raw)) store.remove('snapshot');
    }
    // Which network is this device on? Judged from the address the server
    // sees, against the snapshot.
    if (who.state === 'ok' && who.ip && snap) network = networkOf(snap.topo, who.ip);
    else network = 'unknown';
    r.network = network;
    // Remember the hkfree.org address only while actually on hkfree.org; elsewhere
    // (mobile data) keep the last known one for the path.
    let contactP = Promise.resolve();
    if (network === 'hkfree') {
      autoIp = who.ip;
      store.set('autoIp', autoIp);
      contactP = refreshContact();
    }
    r.snapshotAt = snap ? snap.topo.generatedAt : 0;

    // Phase 2: this device's path from the (fresh or cached) snapshot, then probe it.
    r.analysis = analysisFor(myIp());
    r.targets = probeTargets(r.analysis);
    renderChecks();
    // Outside hkfree.org the backbone routers are unreachable by design: no probes.
    const outside = network === 'other';
    const raws = offline || outside
      ? r.targets.map(() => ({ cls: 'silent' }))
      : await Promise.all(r.targets.map((t) => probeHost(t.ip)));
    // Baselines are learned only on hkfree.org, while everything works.
    const healthy = r.serverOk && r.internetOk && network === 'hkfree';
    r.hops = r.targets.map((t, i) => {
      const cur = raws[i].cls;
      const base = baseCls(t.ip);
      let verdict;
      if (outside) verdict = 'outside';
      else if (r.localFailure) verdict = 'untestable';
      else if (base === 'answers') verdict = cur === 'answers' ? 'ok' : 'lost';
      else verdict = healthy && cur === 'answers' ? 'ok' : 'untestable';
      return { ip: t.ip, cur, verdict };
    });
    // Learn what "normal" looks like only while everything works.
    if (healthy) {
      for (const h of r.hops) learn(h.ip, h.cur);
      learn(CONTROL_IP, control.cls);
      store.set('baseline', baseline);
    }
    await contactP;
    r.done = true;
    r.verdict = decide(r);
    history.unshift({ at: now(), level: r.verdict.level, key: r.verdict.key, params: r.verdict.params });
    history = history.slice(0, 30);
    store.set('history', history);
    trackOutage(r);
    if (r.serverOk) {
      await sendOutages();
      fetchPublicOutages();
    }
  } catch (e) {
    last = { ...(last || {}), done: false, error: String(e && e.message ? e.message : e) };
  } finally {
    running = false;
    btn.disabled = false;
    renderAll();
    if (rerun) {
      rerun = false;
      runCheck();
    } else {
      scheduleRecheck();
    }
  }
}

// ---------- outage reports ----------

/** Verdicts meaning "the problem is in the hkfree.org network" (not local, not the internet). */
const REPORTABLE = ['apDown', 'backbone', 'known', 'generic', 'split'];
const MAX_OUTAGES = 20;
const OUTAGE_MAX_AGE = 29 * 24 * 3600; // the server accepts reports up to 30 days old

function newId() {
  if (crypto.randomUUID) return crypto.randomUUID();
  const b = crypto.getRandomValues(new Uint8Array(16));
  return Array.from(b, (x) => x.toString(16).padStart(2, '0')).join('');
}

/** Opens an outage on a network-side failure; closes it once hkfree.org works again. */
function trackOutage(r) {
  const t = now();
  outages = outages.filter((o) => o && Number.isFinite(o.detected_at) && t - o.detected_at < OUTAGE_MAX_AGE);
  const open = outages.find((o) => !o.resolved_at);
  if (REPORTABLE.includes(r.verdict.key)) {
    if (!open) {
      const firstLost = r.hops.findIndex((h) => h.verdict === 'lost');
      let lastOk = null;
      for (let i = (firstLost < 0 ? r.hops.length : firstLost) - 1; i >= 0; i--) {
        if (r.hops[i].verdict === 'ok') { lastOk = r.targets[i].ip; break; }
      }
      outages.push({
        id: newId(),
        detected_at: t,
        verdict: r.verdict.key,
        my_ip: myIp() || null,
        ap: r.targets.length ? r.targets[0].ip : null,
        first_bad: firstLost >= 0 ? r.targets[firstLost].ip : null,
        last_ok: lastOk,
        path: r.targets.map((x) => x.ip),
        snapshot_at: r.snapshotAt || 0,
      });
      outages = outages.slice(-MAX_OUTAGES);
    }
    reportStatus = { key: 'outageSaved' };
  } else if (open && r.serverOk && r.network === 'hkfree') {
    // Back on hkfree.org with the server reachable: the outage is over.
    open.resolved_at = t;
  }
  store.set('outages', outages);
}

async function sendOutages() {
  const ready = outages.filter((o) => o.resolved_at);
  if (!ready.length) return;
  const body = JSON.stringify(ready.map((o) => ({
    id: o.id, detected_at: o.detected_at, resolved_at: o.resolved_at, verdict: o.verdict, my_ip: o.my_ip,
    ap: o.ap, first_bad: o.first_bad, last_ok: o.last_ok, path: o.path, snapshot_at: o.snapshot_at,
  })));
  const r = await timedFetch('/api/reports', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body, cache: 'no-store' }, 8000);
  if (!r.ok) return; // keep for the next attempt
  // 200: stored; 4xx: the server will never accept them (or does not store reports): drop.
  if (r.res.status === 200 || (r.res.status >= 400 && r.res.status < 500)) {
    const sent = new Set(ready.map((o) => o.id));
    outages = outages.filter((o) => !sent.has(o.id));
    store.set('outages', outages);
    if (r.res.status === 200) reportStatus = { key: 'outagesSent', n: ready.length };
  }
}

/** While an outage is open, check again every minute (only while the app is visible). */
function scheduleRecheck() {
  clearTimeout(recheckTimer);
  if (!outages.some((o) => !o.resolved_at)) return;
  recheckTimer = setTimeout(() => {
    if (document.visibilityState === 'visible') runCheck();
    else scheduleRecheck();
  }, 60000);
}

async function fetchPublicOutages() {
  const r = await timedFetch('/api/outages', { cache: 'no-store' }, 6000);
  if (!r.ok || !r.res.ok) return;
  try {
    const body = await r.res.json();
    if (!Array.isArray(body.outages)) return;
    const list = body.outages.filter((o) => o && Number.isFinite(o.detected_at) && Number.isFinite(o.resolved_at)
      && typeof o.verdict === 'string').slice(0, 200);
    publicOutages = { at: now(), enabled: body.enabled !== false, outages: list };
    store.set('publicOutages', publicOutages);
    renderOutages();
  } catch (_) { /* keep the cached list */ }
}

// ---------- rendering ----------

function pill(state, text) {
  return el('span', `pill ${state}`, text);
}

function setVerdict(level, title, text) {
  const v = $('#verdict');
  v.className = `verdict verdict-${level}`;
  v.replaceChildren(el('div', 'verdict-title', title), el('div', 'verdict-text', text));
}

function checkPill(x) {
  if (!x) return pill('wait', T.pillChecking);
  if (x.state === 'ok') return pill('ok', T.pillOk);
  return pill('bad', T.pillFailed);
}

function renderChecks() {
  const list = $('#checks');
  list.replaceChildren();
  if (!last) return;
  const rows = [
    [T.chkDevice, last.device],
    [T.chkServer, last.server],
    [T.chkInetIp, last.inetIp],
    [T.chkInetDns, last.inetDns],
  ];
  for (const [name, x] of rows) {
    const sub = x && x.ms ? T.ms({ ms: Math.round(x.ms) }) : null;
    list.append(el('li', null, el('div', 'row', el('div', 'main', el('div', 'name', name), sub && el('div', 'sub', sub)), checkPill(x))));
  }
  const a = last.analysis;
  if (last.targets.length || (a && a.pathUnknown)) list.append(el('li', 'section-title', T.pathProbes));
  if (a && a.pathUnknown) list.append(el('li', 'empty', T.pathUnknown));
  const btn = $('#signal-btn');
  btn.hidden = !last.done || !last.targets.length || network !== 'hkfree';
  btn.disabled = Boolean(signalState && signalState.loading);
  // The speed test only over hkfree.org: elsewhere it would measure (and use up) another provider's line.
  const speedBtn = $('#speed-btn');
  const speedNote = $('#speed-note');
  speedBtn.hidden = !last.done || !speedtestBase();
  speedBtn.disabled = network !== 'hkfree' || Boolean(speedState && speedState.phase);
  speedNote.hidden = speedBtn.hidden;
  speedNote.textContent = network === 'hkfree' ? T.speedNote : T.speedOnlyHkfree;
  if (last.targets.length) {
    const item = el('li', 'map-item');
    list.append(item);
    // The card has 12px of padding on each side.
    // Folded away it has no width yet; it is drawn again when opened.
    item.append(pathMap(last, item.clientWidth ? Math.max(260, item.clientWidth - 24) : 320));
  }
}

/** Support contact from the snapshot, re-validated: only https/mailto/tel links. */
function supportNode() {
  const sup = snap && snap.raw && snap.raw.support && typeof snap.raw.support === 'object' ? snap.raw.support : null;
  const parts = [];
  const link = (href, label) => {
    const a = el('a', null, label);
    a.href = href;
    a.rel = 'noopener noreferrer';
    return a;
  };
  if (sup) {
    if (typeof sup.name === 'string' && sup.name) parts.push(sup.name);
    if (typeof sup.url === 'string' && /^https:\/\/[^\s]+$/.test(sup.url)) parts.push(link(sup.url, sup.url.replace(/^https:\/\//, '')));
    if (typeof sup.email === 'string' && EMAIL_RE.test(sup.email)) parts.push(link(`mailto:${sup.email}`, sup.email));
    if (typeof sup.phone === 'string' && /^[0-9 +()-]+$/.test(sup.phone)) {
      const hours = typeof sup.phone_hours === 'string' && /^[0-9:]+-[0-9:]+$/.test(sup.phone_hours) ? sup.phone_hours.replace('-', '–') : '';
      const phone = el('span', null, link(`tel:${sup.phone.replace(/[^0-9+]/g, '')}`, sup.phone));
      if (hours) phone.append(` ${T.phoneHours({ hours })}`);
      parts.push(phone);
    }
    if (typeof sup.slack_channel === 'string' && /^#[a-z0-9._-]+$/.test(sup.slack_channel)) parts.push(T.slack({ channel: sup.slack_channel }));
  }
  const p = el('p', 'note support', `${T.support}: `);
  if (!parts.length) {
    p.textContent = T.supportFallback;
    return p;
  }
  parts.forEach((x, i) => {
    if (i) p.append(' · ');
    p.append(x);
  });
  return p;
}

/** One line per AP admin from the map, with the e-mail this device was given for its AP. */
function adminNodes() {
  const a = last && last.analysis;
  if (!a || !a.ap) return [];
  const admins = a.ap.admins || [];
  const kept = store.get('contact', null);
  const mine = kept && kept.router === a.router && Array.isArray(kept.admins) ? kept.admins : [];
  return admins.map((x) => {
    const p = el('p', 'note admin-contact', `${x.role === 'SO' ? T.adminSo : T.adminZso}: ${x.nick}`);
    const k = mine.find((m) => m && m.nick === x.nick);
    if (k && typeof k.email === 'string' && EMAIL_RE.test(k.email)) {
      const mail = el('a', null, k.email);
      mail.href = `mailto:${k.email}`;
      p.append(' · ', mail);
    }
    return p;
  });
}

const NEEDS_SUPPORT = ['apDown', 'backbone', 'known', 'generic', 'inetDown', 'split'];

function renderAdvice() {
  const box = $('#advice');
  box.replaceChildren();
  if (running || !last || !last.done) {
    box.hidden = true;
    return;
  }
  const key = last.verdict.key;
  const steps = T.steps[key] || [];
  if (steps.length) {
    box.append(el('h3', null, T.stepsTitle), el('ol', null, ...steps.map((x) => el('li', null, x))));
  }
  if (last.network === 'other') box.append(el('p', 'note', T.otherNetworkNote));
  if (NEEDS_SUPPORT.includes(key)) box.append(...adminNodes(), supportNode());
  if (reportStatus) {
    const fn = T[reportStatus.key];
    box.append(el('p', 'note', typeof fn === 'function' ? fn({ n: reportStatus.n }) : fn));
  }
  const pending = outages.filter((o) => o.resolved_at).length;
  if (pending) box.append(el('p', 'note', T.outagesPending({ n: pending })));
  box.hidden = !box.childNodes.length;
}

const SVG = 'http://www.w3.org/2000/svg';

function svg(tag, attrs, text) {
  const e = document.createElementNS(SVG, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, String(v));
  if (text !== undefined) e.textContent = text;
  return e;
}

/** Splits `text` into lines of at most `max` characters at spaces; each line but the last keeps its space. */
function wrapText(text, max) {
  const lines = [];
  let line = '';
  for (const word of String(text).split(' ')) {
    if (line && (line + word).length > max) {
      lines.push(line);
      line = '';
    }
    line += `${word} `;
  }
  lines.push(line.trimEnd());
  return lines;
}

/** good / usable / weak, by how strongly the antenna receives the AP. */
function signalRating(dbm) {
  if (dbm >= -70) return 'good';
  if (dbm >= -80) return 'usable';
  return 'weak';
}

/** Lines about the antenna's signal for the AP node, once the member asked. */
function signalLines() {
  const s = signalState;
  if (!s) return [];
  if (s.loading) return [[T.signalLoading, 'signal-status']];
  if (s.error) return [[T.signalError, 'signal']];
  if (s.reason === 'no_ap') return [[T.signalNoAp, 'signal']];
  if (s.reason === 'ipv6') return [[T.signalIpv6, 'signal']];
  if (!s.stations.length) return [[T.signalNotFound, 'signal']];
  const lines = [];
  for (const st of s.stations) {
    const rating = signalRating(st.dbm);
    const cls = rating === 'good' ? 'map-ok' : rating === 'weak' ? 'map-down' : '';
    lines.push([T.signalLine({ dbm: st.dbm, rating: T.signalRatings[rating] }), `signal ${cls}`]);
    const detail = [
      Number.isFinite(st.snr) ? `SNR ${st.snr} dB` : '',
      Number.isFinite(st.tx_ccq) ? `CCQ ${st.tx_ccq} %` : '',
      Number.isFinite(st.rx_mbps) && Number.isFinite(st.tx_mbps) ? `${Math.round(st.rx_mbps)}/${Math.round(st.tx_mbps)} Mb/s` : '',
      st.uptime ? T.signalUptime({ uptime: st.uptime }) : '',
    ].filter(Boolean).join(' · ');
    if (detail) lines.push([detail, 'signal-detail']);
    if (rating === 'weak') lines.push([T.signalAdvice, 'signal map-down']);
  }
  return lines;
}

/** The speed test server from the network map: a LibreSpeed backend over https. */
function speedtestBase() {
  const s = snap && snap.raw && snap.raw.speedtest;
  const u = s && typeof s.url === 'string' ? s.url : '';
  return /^https:\/\/[A-Za-z0-9.-]+(:\d+)?\/[A-Za-z0-9/._-]*\/$/.test(u) ? u : null;
}

/** Lines about the measured speed for the device node of the map. */
function speedLines() {
  const s = speedState;
  if (!s) return [];
  if (s.phase) return [[T.speedPhase[s.phase], 'speed-status']];
  if (s.error) return [[T.speedFailed, 'speed map-down']];
  const r = s.result;
  const mbps = (v) => (v >= 10 ? Math.round(v) : Math.round(v * 10) / 10);
  return [
    [`↓ ${mbps(r.down)} Mbit/s · ↑ ${mbps(r.up)} Mbit/s`, 'speed'],
    [T.speedLatency({ ping: Math.round(r.ping), jitter: Math.round(r.jitter) }), 'speed'],
  ];
}

/** Measures the connection's speed (only on the member's request, only on hkfree.org). */
async function runSpeedTest() {
  const base = speedtestBase();
  if (!base || network !== 'hkfree' || (speedState && speedState.phase)) return;
  speedState = { phase: 'ping' };
  renderChecks();
  try {
    const result = await measureSpeed(base, (phase) => {
      speedState = { phase };
      renderChecks();
    });
    speedState = { result };
  } catch (_) {
    speedState = { error: true };
  }
  renderChecks();
}

/** Asks the server for this device's antenna signal (only on the member's request). */
async function askSignal() {
  signalState = { loading: true };
  renderChecks();
  const r = await timedFetch('/api/signal', { cache: 'no-store' }, 60000);
  let body = null;
  try {
    if (r.ok && r.res.ok) body = await r.res.json();
  } catch (_) { /* treated as an error below */ }
  if (!body) signalState = { error: true };
  else {
    const stations = (Array.isArray(body.stations) ? body.stations : [])
      .filter((st) => st && Number.isFinite(st.dbm))
      .slice(0, 5);
    signalState = { reason: typeof body.reason === 'string' ? body.reason : null, stations };
  }
  renderChecks();
}

/** What the probe found out about a router, as shown on the map. */
function hopStatus(h) {
  if (!h) return [T.pillChecking, ''];
  if (h.verdict === 'ok') return [T.pillReachable, 'map-ok'];
  if (h.verdict === 'lost') return [T.pillLost, 'map-down'];
  if (h.verdict === 'outside') return [T.pillOutside, ''];
  return [T.pillUntestable, ''];
}

/**
 * The path as a map: device → routers → internet; traffic flows until the
 * first router that doesn't answer. Each node carries everything known about
 * it: address, role, what the probe found, and for the AP its name, place and
 * what network monitoring reports.
 */
function pathMap(r, width) {
  const a = r.analysis || {};
  const hopState = (h) => (h && h.verdict === 'ok' ? 'ok' : h && h.verdict === 'lost' ? 'down' : 'unknown');
  const roleText = { ap: T.roleAp, border: T.roleBorder, exit: T.roleExit, hop: T.roleHop };
  const nodes = [
    {
      role: 'device', ip: '', label: T.mapDevice, state: r.device.state === 'ok' && !r.localFailure ? 'ok' : 'down',
      lines: [...(myIp() ? [[myIp(), '']] : []), ...speedLines()],
      status: r.device.state === 'ok' ? null : [T.pillLost, 'map-down'],
    },
    ...r.targets.map((t, i) => {
      const first = i === 0;
      const lines = [[`${t.ip} · ${roleText[t.role]}`, '']];
      if (first && a.ap) lines.push([T.apWhere({ area: a.ap.area, address: a.ap.address }), 'ap-info']);
      else if (first && t.role === 'ap') lines.push([T.apUnknown, 'ap-info']);
      if (first && t.role === 'border') lines.push([T.hiddenPart, 'hidden-part']);
      if (first) lines.push(...signalLines());
      if (first && a.monitor && a.monitor.length) {
        const problems = a.monitor.filter((d) => d.health !== 'Ok');
        if (!problems.length) lines.push([T.monitorOk({ total: a.monitor.length }), 'monitoring map-ok']);
        for (const d of problems) {
          lines.push([T.monitorLine({ label: d.label || T.apDevice, ip: d.ip, health: T.monitorHealth[d.health] }), 'monitoring map-down']);
        }
      }
      return { role: t.role, ip: t.ip, label: first && a.ap ? a.ap.name : t.name || t.ip, state: hopState(r.hops[i]), lines, status: hopStatus(r.hops[i]) };
    }),
    {
      role: 'internet', ip: '', label: T.mapInternet, state: r.internetOk ? 'ok' : 'down',
      lines: [],
      status: r.internetOk ? [T.pillReachable, 'map-ok'] : [T.pillLost, 'map-down'],
    },
  ];
  let broken = nodes[0].state === 'down';
  const segments = nodes.slice(1).map((n) => {
    if (broken) return 'idle';
    if (n.state === 'down') {
      broken = true;
      return 'broken';
    }
    return 'flowing';
  });

  // Drawn 1:1 in the space it gets, so text keeps its size on any screen.
  const x = 46;
  // The name leaves room for the result at the right edge.
  const labelMax = Math.max(12, Math.floor((width - x - 110) / 9));
  const lineMax = Math.max(20, Math.floor((width - x - 8) / 7.2));
  let y = 20;
  const placed = nodes.map((n) => {
    const label = wrapText(n.label, labelMax);
    const lines = n.lines.map(([text, cls]) => [wrapText(text, lineMax), cls]);
    const height = label.length * 20 + lines.reduce((s, [l]) => s + l.length * 18, 0);
    const at = { n, label, lines, y };
    y += Math.max(height, 24) + 22;
    return at;
  });
  const map = svg('svg', { id: 'path-map', class: 'path-map', width, height: y, viewBox: `0 0 ${width} ${y}`, role: 'img', 'aria-label': T.pathProbes });
  segments.forEach((s, i) => {
    map.append(svg('line', { x1: 24, y1: placed[i].y, x2: 24, y2: placed[i + 1].y, class: `seg seg-${s}`, 'data-segment': s }));
  });
  for (const { n, label, lines, y: top } of placed) {
    const [status, statusCls] = n.status || [];
    const g = svg('g', { 'data-node': n.role, 'data-ip': n.ip, 'data-state': n.state });
    g.append(svg('circle', { cx: 24, cy: top, r: 9, class: `node node-${n.state}` }));
    let ty = top + 5;
    const text = (rows, cls, step) => {
      const t = svg('text', { x, y: ty, class: cls });
      rows.forEach((row, j) => t.append(svg('tspan', { x, dy: j ? step : 0 }, row)));
      ty += rows.length * step;
      g.append(t);
    };
    if (status) g.append(svg('text', { x: width - 4, y: top + 5, 'text-anchor': 'end', class: `map-status ${statusCls}` }, status));
    text(label, 'map-label', 20);
    ty -= 1;
    for (const [rows, cls] of lines) text(rows, `map-sub${cls ? ` ${cls}` : ''}`, 18);
    map.append(g);
  }
  return map;
}

let resizeTimer = null;
window.addEventListener('resize', () => {
  clearTimeout(resizeTimer);
  resizeTimer = setTimeout(() => { if ($('#advanced').open) renderChecks(); }, 150);
});
$('#advanced').addEventListener('toggle', () => { if ($('#advanced').open) renderChecks(); });

function renderVerdict() {
  if (running) return; // keeps "Checking…"
  if (last && last.error) {
    const [title, text] = T.v.failed({ msg: last.error });
    setVerdict('warn', title, text);
  } else if (last && last.done) {
    const [title, text] = verdictText(last.verdict);
    setVerdict(last.verdict.level, title, text);
  } else {
    setVerdict('idle', T.ready, T.readyText);
  }
}

function renderIdentity() {
  const ip = myIp();
  let label;
  if (!ip) label = T.myAddressUnknown;
  else if (manualIp) label = T.myAddressManual({ ip });
  else if (network !== 'hkfree') label = T.myAddressLast({ ip });
  else label = T.myAddress({ ip });
  $('#identity').textContent = label;
  // An address found automatically is shown on the map's device node.
  $('.identity').hidden = Boolean(ip) && !manualIp && network === 'hkfree' && Boolean(last && last.targets.length);
  // The manual field is shown when the address is unknown or was entered by hand.
  $('#my-ip-form').hidden = Boolean(ip) && !manualIp;
  $('#my-ip-auto').hidden = !manualIp;
  if (manualIp) $('#my-ip').value = manualIp;
}

const SEV = { critical: ['bad', 'sevCritical'], warning: ['warn', 'sevWarning'], info: ['info', 'sevInfo'] };

function renderPath() {
  const meta = $('#path-meta');
  const hopsEl = $('#hops');
  const findingsEl = $('#findings');
  const pv = $('#path-verdict');
  hopsEl.replaceChildren();
  findingsEl.replaceChildren();
  $('#exit-info').textContent = '';
  const ip = viewIp || myIp();
  $('#view-ip').value = viewIp || '';
  $('#path-title').textContent = ip ? T.pathFor({ ip }) : '';
  if (!snap || !ip) {
    meta.textContent = snap ? T.myAddressUnknown : T.noSnapshot;
    pv.hidden = true;
    return;
  }
  const savedNote = !snapFresh && last && (last.done || last.error) ? T.snapshotSaved : '';
  meta.textContent = T.snapshotFrom({ ago: ago(snap.topo.generatedAt) }) + savedNote;
  const a = analysisFor(ip);
  const top = a.findings.find((f) => f.sev !== 'info');
  pv.hidden = false;
  pv.className = `card verdict-${top ? (top.sev === 'critical' ? 'bad' : 'warn') : 'ok'}`;
  pv.replaceChildren(el('div', 'name', T.verdictTitle), el('div', null, top ? findingText(top)[0] : T.noProblem));

  if (a.exit) {
    $('#exit-info').textContent = T.exitInfo({ router: a.exit.target, e1: a.exit.e1, metric: a.exit.metric, cost: a.cost });
  }
  if (!a.hops.length) hopsEl.append(el('li', 'empty', T.noPathFound));
  for (const h of a.hops) {
    const s = h.sojka;
    const dotCls = s === 'Ok' ? 'ok' : s === 'Dead' ? 'bad' : s === 'Lossy' || s === 'Slow' ? 'warn' : '';
    const sub = T.hopSub({ ip: h.router, cost: h.cost, sojka: s ? T.sojka[s] || s : null });
    hopsEl.append(el('li', null, el('div', 'row', el('span', `dot ${dotCls}`), el('div', 'main', el('div', 'name', h.name || h.router), el('div', 'sub', sub)))));
  }
  if (!a.findings.length) findingsEl.append(el('li', 'empty', T.noFindings));
  for (const f of a.findings) {
    const [cls, key] = SEV[f.sev];
    const [title, lines] = findingText(f);
    const detail = lines.length ? el('ul', 'detail', ...lines.map((d) => el('li', null, d))) : null;
    findingsEl.append(el('li', 'row finding', el('div', 'main', el('div', 'name', title), detail), pill(cls, T[key])));
  }
}

function renderHistory() {
  const list = $('#history');
  list.replaceChildren();
  if (!history.length) {
    list.append(el('li', 'empty', T.noHistory));
    return;
  }
  const fmt = new Intl.DateTimeFormat(lang, { dateStyle: 'short', timeStyle: 'short' });
  for (const h of history) {
    if (!h || !Number.isFinite(h.at) || typeof h.key !== 'string') continue; // skip corrupt entries
    const cls = h.level === 'ok' ? 'ok' : h.level === 'warn' ? 'warn' : 'bad';
    let title;
    try {
      [title] = verdictText({ key: h.key, params: h.params || {} });
    } catch (_) {
      continue;
    }
    list.append(el('li', null, el('div', 'row', el('span', `dot ${cls}`),
      el('div', 'main', el('div', 'name', title), el('div', 'sub', fmt.format(new Date(h.at * 1000)))))));
  }
}

/** Header badge: offline, on hkfree.org, on another network, or unknown. */
function updateBadge() {
  const b = $('#net-badge');
  let text;
  let cls;
  let title;
  if (!navigator.onLine) [text, cls, title] = [T.offline, 'offline', ''];
  else if (network === 'hkfree') [text, cls, title] = [T.netHkfree, 'online', T.netHkfreeTitle];
  else if (network === 'other') [text, cls, title] = [T.netOther, 'other', T.netOtherTitle];
  else [text, cls, title] = [T.netUnknown, '', T.netUnknownTitle];
  b.textContent = text;
  b.className = `badge ${cls}`;
  b.title = title;
  // After a check, being outside hkfree.org (or not knowing) is said right at the top.
  const w = $('#net-warning');
  const show = Boolean(last && last.done) && navigator.onLine && network !== 'hkfree';
  w.hidden = !show;
  w.className = `net-warning ${network === 'other' ? 'other' : 'unknown'}`;
  w.textContent = show ? (network === 'other' ? T.otherNetworkBanner : T.netUnknownTitle) : '';
}

function renderOutages() {
  const list = $('#outages');
  const top = $('#outages-top');
  list.replaceChildren();
  top.replaceChildren();
  $('#outages-top-title').hidden = true;
  const meta = $('#outages-meta');
  if (!publicOutages) {
    meta.textContent = T.outagesUnavailable;
    return;
  }
  if (!publicOutages.enabled) {
    meta.textContent = T.outagesDisabled;
    return;
  }
  meta.textContent = now() - publicOutages.at > 120 ? T.outagesCached({ ago: ago(publicOutages.at) }) : '';
  const name = (ip) => {
    if (!isIPv4(ip)) return null;
    const r = snap && snap.topo.routers.get(ip);
    return r && r.name ? `${ip} (${r.name})` : ip;
  };
  if (!publicOutages.outages.length) {
    list.append(el('li', 'empty', T.outagesNone));
    return;
  }
  const fmt = new Intl.DateTimeFormat(lang, { dateStyle: 'short', timeStyle: 'short' });
  const counts = new Map();
  for (const o of publicOutages.outages) {
    const kind = T.outageKind[o.verdict] || o.verdict;
    const sub = T.outageSub({ ap: name(o.ap), bad: name(o.first_bad), dur: T.duration(Math.max(0, o.resolved_at - o.detected_at)) });
    const when = fmt.format(new Date(o.detected_at * 1000));
    list.append(el('li', null, el('div', 'row', el('span', 'dot bad'),
      el('div', 'main', el('div', 'name', `${when} · ${kind}`), el('div', 'sub', sub)))));
    const key = o.first_bad || o.ap;
    if (isIPv4(key)) counts.set(key, (counts.get(key) || 0) + 1);
  }
  const ranked = [...counts.entries()].sort((a, b) => b[1] - a[1]).slice(0, 5);
  if (ranked.length) {
    $('#outages-top-title').hidden = false;
    for (const [ip, n] of ranked) {
      top.append(el('li', null, el('div', 'row', el('div', 'main', el('div', 'name', name(ip))), pill('warn', T.reportsCount({ n })))));
    }
  }
}

function renderAll() {
  applyStaticTexts();
  updateBadge();
  for (const render of [renderChecks, renderVerdict, renderAdvice, renderIdentity, renderPath, renderHistory, renderOutages, renderHelp, renderChat]) {
    try {
      render();
    } catch (_) { /* a bad cached entry must not break the page */ }
  }
  $('#run').textContent = running ? T.checking : last && (last.done || last.error) ? T.runAgain : T.run;
}

// ---------- assistant (Help tab) ----------

let aiKind = ['guide', 'small', 'large'].includes(store.get('aiEngine', 'guide')) ? store.get('aiEngine', 'guide') : 'guide';
let aiEngine = null;   // loaded engine for aiKind (null = guide only)
let aiCaps = null;     // { webgpu, cached: { small, large } }
let aiBusy = false;
let aiNote = null;     // { key, params } shown in the status card
let aiGen = 0;         // bumped on every engine/language change; stale async results are dropped
const chat = [];       // { role: 'user' | 'bot', text, sources: [titles] }

async function detectAi() {
  const gen = aiGen;
  const caps = await ai.detect();
  caps.cached = { small: false, large: false };
  if (caps.webgpu) {
    caps.cached.small = await ai.modelCached(ai.MODELS.small);
    caps.cached.large = await ai.modelCached(ai.MODELS.large);
  }
  if (gen !== aiGen) return; // language changed meanwhile
  aiCaps = caps;
  renderHelp();
}

/** Drops the current engine and frees its (GPU) memory. */
async function disposeEngine() {
  const e = aiEngine;
  aiEngine = null;
  if (e && e.dispose) await e.dispose();
}

function setProgress(fraction) {
  const bar = $('#ai-progress');
  if (fraction === null) {
    bar.hidden = true;
    return;
  }
  bar.hidden = false;
  $('#ai-progress-bar').style.width = `${Math.round(Math.max(0, Math.min(1, fraction)) * 100)}%`;
  $('#ai-engine').textContent = T.loadingModel({ pct: Math.round(fraction * 100) });
}

/** Loads the selected engine (downloads a model the first time). */
async function loadEngine(kind) {
  const gen = ++aiGen;
  await disposeEngine();
  aiKind = kind;
  store.set('aiEngine', kind);
  aiNote = null;
  if (kind === 'guide') {
    renderHelp();
    return;
  }
  aiBusy = true;
  renderHelp();
  setProgress(0);
  try {
    const engine = await ai.createWebLlm(ai.MODELS[kind], setProgress);
    if (gen !== aiGen) {
      // Superseded (another choice or a language change) while loading.
      if (engine.dispose) await engine.dispose();
      return;
    }
    aiEngine = engine;
    if (aiCaps) aiCaps.cached[kind] = true;
  } catch (e) {
    if (gen !== aiGen) return;
    aiKind = 'guide';
    store.set('aiEngine', 'guide');
    aiNote = { key: 'modelFailed', params: { msg: String(e && e.message ? e.message : e).slice(0, 160) } };
  } finally {
    if (gen === aiGen) {
      aiBusy = false;
      setProgress(null);
      renderHelp();
    }
  }
}

function lastCheckSummary() {
  if (!last || !last.done || !last.verdict) return null;
  const [title, text] = verdictText(last.verdict);
  const steps = T.steps[last.verdict.key] || [];
  const net = last.network === 'hkfree' ? T.netHkfreeTitle : last.network === 'other' ? T.netOtherTitle : T.netUnknownTitle;
  return [`${title}. ${text}`, net, ...steps.map((x) => `- ${x}`)].join('\n');
}

/** Answer straight from the guide (no model). */
function guideAnswer(question, arts) {
  const lines = [];
  if (last && last.done && last.verdict) lines.push(T.lastCheck({ title: verdictText(last.verdict)[0] }), '');
  if (!arts.length) {
    lines.push(T.noMatch);
    return lines.join('\n');
  }
  lines.push(`${T.fromGuide} ${arts[0].title}`);
  arts[0].body.forEach((b, i) => lines.push(`${i + 1}. ${b}`));
  if (arts.length > 1) lines.push('', `${T.related} ${arts.slice(1).map((a) => a.title).join(', ')}`);
  return lines.join('\n');
}

async function askAssistant(question) {
  question = question.trim().slice(0, 300);
  if (!question || aiBusy) return;
  const arts = ai.sources(lang, question);
  chat.push({ role: 'user', text: question });
  const msg = { role: 'bot', text: '', sources: arts.map((a) => a.title) };
  chat.push(msg);
  renderChat();
  // A previously chosen model is loaded lazily on the first question, but
  // never re-downloaded silently if the browser evicted it from its cache.
  if (aiKind !== 'guide' && !aiEngine) {
    if ((aiKind === 'small' || aiKind === 'large') && !(await ai.modelCached(ai.MODELS[aiKind]))) {
      aiKind = 'guide';
      store.set('aiEngine', 'guide');
      if (aiCaps) aiCaps.cached = { small: false, large: false };
    } else {
      await loadEngine(aiKind);
    }
  }
  if (!aiEngine) {
    msg.text = guideAnswer(question, arts);
    renderChat();
    return;
  }
  aiBusy = true;
  renderHelp();
  try {
    const prompt = ai.buildPrompt(lang, question, arts, lastCheckSummary());
    for await (const chunk of aiEngine.ask(prompt.system, prompt.user)) {
      msg.text += chunk;
      renderChat();
    }
    if (!msg.text.trim()) msg.text = guideAnswer(question, arts);
  } catch (e) {
    msg.text = guideAnswer(question, arts);
    aiNote = { key: 'modelFailed', params: { msg: String(e && e.message ? e.message : e).slice(0, 160) } };
  } finally {
    aiBusy = false;
    renderHelp();
    renderChat();
  }
}

function renderChat() {
  const list = $('#chat');
  list.replaceChildren();
  for (const m of chat) {
    const who = el('div', 'who', m.role === 'user' ? T.you : T.assistantName);
    const body = el('div', 'text', m.text || '…');
    const src = m.role === 'bot' && m.sources && m.sources.length ? el('div', 'src', `${T.sourcesLabel}: ${m.sources.join(', ')}`) : null;
    list.append(el('li', `msg ${m.role}`, who, body, src));
  }
  const lastMsg = list.lastElementChild;
  if (lastMsg && !$('#tab-help').hidden) lastMsg.scrollIntoView({ block: 'nearest' });
}

function renderHelp() {
  const engineEl = $('#ai-engine');
  const actions = $('#ai-actions');
  const note = $('#ai-note');
  actions.replaceChildren();
  engineEl.textContent = aiKind === 'guide' ? T.engineGuide : T.engineWebllm({ name: ai.MODELS[aiKind].name });
  const button = (label, fn) => {
    const b = el('button', null, label);
    b.type = 'button';
    b.disabled = aiBusy;
    b.addEventListener('click', fn);
    actions.append(b);
  };
  const notes = [];
  if (aiCaps) {
    if (aiCaps.webgpu) {
      for (const k of ['small', 'large']) {
        if (aiKind === k) continue;
        const m = ai.MODELS[k];
        const label = aiCaps.cached[k]
          ? `${T.useModel({ name: m.name })} (${T.modelReady})`
          : T.downloadModel({ label: k === 'small' ? T.modelSmall : T.modelLarge, mb: m.mb });
        button(label, () => loadEngine(k));
      }
      if (!aiCaps.cached.small || !aiCaps.cached.large) notes.push(T.modelNote);
    } else {
      notes.push(T.noWebgpu);
    }
    if (aiKind !== 'guide') button(T.useGuide, () => loadEngine('guide'));
    if (aiKind === 'small' || aiKind === 'large') {
      const k = aiKind;
      button(T.removeModel, async () => {
        await loadEngine('guide'); // unloads the model first
        try {
          await ai.deleteModel(ai.MODELS[k]);
          aiCaps.cached[k] = false;
        } catch (e) {
          aiNote = { key: 'modelFailed', params: { msg: String(e && e.message ? e.message : e).slice(0, 160) } };
        }
        aiCaps.cached[k] = await ai.modelCached(ai.MODELS[k]);
        renderHelp();
      });
    }
  }
  if (aiNote) notes.unshift(T[aiNote.key](aiNote.params));
  note.hidden = !notes.length;
  note.textContent = notes.join(' ');
  const chips = $('#chips');
  chips.replaceChildren();
  for (const q of T.chips) {
    const b = el('button', null, q);
    b.type = 'button';
    b.disabled = aiBusy;
    b.addEventListener('click', () => askAssistant(q));
    chips.append(b);
  }
  $('#ask').placeholder = T.askPlaceholder;
}

// ---------- shell ----------

const TABS = ['check', 'path', 'history', 'outages', 'help'];

function selectTab(name) {
  if (!TABS.includes(name)) name = 'check';
  for (const btn of document.querySelectorAll('.tabs button')) {
    btn.setAttribute('aria-selected', String(btn.dataset.tab === name));
  }
  for (const p of document.querySelectorAll('.panel')) p.hidden = p.id !== `tab-${name}`;
  store.set('tab', name);
  if (name === 'outages' && (!publicOutages || now() - publicOutages.at > 60)) fetchPublicOutages();
  if (name === 'help' && !aiCaps) detectAi();
}

let installEvent = null;
window.addEventListener('beforeinstallprompt', (e) => {
  e.preventDefault();
  installEvent = e;
  $('#install').hidden = false;
});

function init() {
  for (const btn of document.querySelectorAll('.tabs button')) {
    btn.addEventListener('click', () => selectTab(btn.dataset.tab));
  }
  $('#run').addEventListener('click', runCheck);
  $('#signal-btn').addEventListener('click', askSignal);
  $('#speed-btn').addEventListener('click', runSpeedTest);
  $('#lang').addEventListener('change', (e) => {
    lang = LANGS.includes(e.target.value) ? e.target.value : 'cs';
    store.set('lang', lang);
    T = messages(lang);
    // Any model still loading belongs to the old language's request.
    aiGen++;
    aiBusy = false;
    setProgress(null);
    aiCaps = null;
    if (!$('#tab-help').hidden) detectAi();
    renderAll();
  });
  $('#my-ip-form').addEventListener('submit', (e) => {
    e.preventDefault();
    const v = $('#my-ip').value.trim();
    if (!isIPv4(v)) {
      $('#identity').textContent = T.badAddress;
      return;
    }
    manualIp = v;
    store.set('manualIp', v);
    renderAll();
    runCheck();
  });
  $('#my-ip-auto').addEventListener('click', () => {
    manualIp = null;
    store.remove('manualIp');
    renderAll();
    runCheck();
  });
  $('#view-form').addEventListener('submit', (e) => {
    e.preventDefault();
    const v = $('#view-ip').value.trim();
    if (v === '') viewIp = null;
    else if (isIPv4(v)) viewIp = v;
    else {
      $('#path-meta').textContent = T.badAddress;
      return;
    }
    renderPath();
  });
  $('#install').addEventListener('click', () => {
    if (!installEvent) return;
    installEvent.prompt();
    installEvent = null;
    $('#install').hidden = true;
  });
  $('#ask-form').addEventListener('submit', (e) => {
    e.preventDefault();
    const q = $('#ask').value;
    $('#ask').value = '';
    askAssistant(q);
  });
  $('#clear-history').addEventListener('click', () => {
    history = [];
    store.remove('history');
    renderHistory();
  });
  window.addEventListener('online', updateBadge);
  window.addEventListener('offline', updateBadge);

  const params = new URLSearchParams(location.search);
  const ipParam = params.get('ip');
  if (isIPv4(ipParam)) viewIp = ipParam;
  selectTab(params.get('tab') || store.get('tab', 'check'));
  renderAll();
  if (params.get('autorun') !== '0') runCheck();

  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('/sw.js').catch(() => { /* not a secure context */ });
  }
}

init();
