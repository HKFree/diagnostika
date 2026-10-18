// Runs the real system for the scenarios: the hkdiag server (release build)
// reading a stand-in FRR over the vty protocol, and the real app in Chromium.
// Routers, the internet and the member's address are set per scenario.

import { AfterAll, After, Before, BeforeAll, setDefaultTimeout, setWorldConstructor, World } from '@cucumber/cucumber';
import { execFile, spawn } from 'node:child_process';
import { chmodSync, existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { createServer as createHttpServer } from 'node:http';
import { createServer as createTcpServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright-core';
import assert from 'node:assert/strict';
import { ALL_ROUTERS, EXIT, externalLsas, networkLsas, ospf, ospf6External, ospf6IntraPrefix, routerLsas, summaryLsas } from './network.mjs';

setDefaultTimeout(60_000);

const VTY_PASSWORD = 'bdd';
const BINARY = process.env.HKDIAG_BIN || 'target/release/hkdiag';
const BROWSERS = [process.env.BDD_BROWSER, '/usr/bin/chromium', '/usr/bin/google-chrome', '/usr/bin/chromium-browser'];

// Built per request: scenarios change what OSPF says.
const VTY_OUTPUT = {
  'terminal length 0': () => '',
  'show ip ospf database router json': routerLsas,
  'show ip ospf database network json': networkLsas,
  'show ip ospf database external json': externalLsas,
  'show ip ospf database summary json': summaryLsas,
  'show ipv6 ospf6 database intra-prefix detail json': ospf6IntraPrefix,
  'show ipv6 ospf6 database as-external detail json': ospf6External,
};

function listen(server) {
  return new Promise((resolve) => server.listen(0, '127.0.0.1', () => resolve(server.address().port)));
}

async function freePort() {
  const probe = createTcpServer();
  const port = await listen(probe);
  await new Promise((resolve) => probe.close(resolve));
  return port;
}

function fakeFrr() {
  return createTcpServer((socket) => {
    let loggedIn = false;
    let buffer = '';
    socket.write('\r\nUser Access Verification\r\n\r\nPassword: ');
    socket.on('data', (chunk) => {
      buffer += chunk.toString('latin1');
      let nl;
      while ((nl = buffer.indexOf('\n')) >= 0) {
        const line = buffer.slice(0, nl).replace(/\r$/, '');
        buffer = buffer.slice(nl + 1);
        if (!loggedIn) {
          if (line !== VTY_PASSWORD) {
            socket.end('\r\n% Bad passwords\r\n');
            return;
          }
          loggedIn = true;
          socket.write('\r\nbdd> ');
        } else if (line === 'exit') {
          socket.end();
        } else {
          const out = VTY_OUTPUT[line] ? VTY_OUTPUT[line]() : '% Unknown command';
          socket.write(`${line}\r\n${out}${out ? '\r\n' : ''}bdd> `);
        }
      }
    });
  });
}

/** How strongly each member's antenna receives the AP (dBm), as the AP's routers see it. */
export const radio = new Map();

/** The AP's router as Wewimo reports it: one wireless interface, a station per antenna in `radio`. */
function wewimo() {
  const stations = [...radio].map(([ip, dbm], i) => ({
    'mac-address': `4C:5E:0C:00:00:0${i}`,
    'last-ip': ip,
    'x-tx-signal-strength': String(dbm),
    'x-signal-strength': String(dbm - 2),
    'x-signal-to-noise': String(dbm + 95),
    'tx-ccq': '88',
    'rx-ccq': '91',
    'x-rx-rate': 300,
    'x-tx-rate': 270,
    uptime: '2d3h4m5s',
    'xx-user-nick': 'someone',
  }));
  return {
    devices: [
      { ip: '10.107.10.149', hostname: 'ap-podulsany', error: '', interfaces: { wlan1: { frequency: '5500', stations } } },
      { ip: '10.107.10.151', hostname: '', error: 'Error connecting to RouterOS', interfaces: {} },
    ],
  };
}

/**
 * The MikroTik border router 10.107.10.97 as the map reads it over its REST API:
 * 'blind' (answers, sees nothing beyond the backbone), 'inside' (also sees its
 * area 10.107.10.0 with the router 10.107.10.65), or 'down'.
 */
export const mikrotik = { mode: 'blind' };

const MIKROTIK_INSIDE = [
  { type: 'router', area: 'area-10', id: '10.107.10.65', originator: '10.107.10.65', sequence: '0x80000010', age: '100', instance: 'inst',
    body: 'options=E bits=E\n    type=network id=10.107.10.58 data=10.107.10.57 metric=10\n    type=stub id=10.107.10.64 data=255.255.255.240 metric=1\n' },
  { type: 'router', area: 'area-10', id: '10.107.10.97', originator: '10.107.10.97', sequence: '0x80000020', age: '100', instance: 'inst',
    body: 'options=E bits=B\n    type=network id=10.107.10.58 data=10.107.10.58 metric=10\n' },
  { type: 'network', area: 'area-10', id: '10.107.10.58', originator: '10.107.10.97', sequence: '0x80000005', age: '100', instance: 'inst',
    body: 'netmask=255.255.255.252\n    router-id=10.107.10.97\n    router-id=10.107.10.65\n' },
];

function fakeMikrotik() {
  const auth = `Basic ${Buffer.from('read:read').toString('base64')}`;
  return createHttpServer((req, res) => {
    req.resume();
    if (mikrotik.mode === 'down') return res.destroy();
    if (req.headers.authorization !== auth) return res.writeHead(401).end();
    const path = new URL(req.url, 'http://x').pathname;
    const body = {
      '/rest/routing/ospf/instance': [{ name: 'inst', version: '2', 'router-id': '10.107.10.97' }],
      '/rest/routing/ospf/area': [
        { name: 'hkfree', 'area-id': '0.0.0.0', instance: 'inst' },
        { name: 'area-10', 'area-id': '10.107.10.0', instance: 'inst' },
      ],
      '/rest/routing/ospf/lsa': mikrotik.mode === 'inside' ? MIKROTIK_INSIDE : [],
      '/rest/ip/route': [{ 'dst-address': '0.0.0.0/0', active: 'true', ospf: 'true' }],
    }[path];
    if (!body) return res.writeHead(404).end();
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(body));
  });
}

/** The hkfree.org power monitor (power.hkfree.org/vypadky.json): sources and ČEZ power cuts. */
export const power = { sources: [], cez: [] };

function fakePower() {
  return createHttpServer((req, res) => {
    req.resume();
    if (new URL(req.url, 'http://x').pathname !== '/vypadky.json') return res.writeHead(404).end();
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify({
      ok: true, appVersion: 'test', generatedAt: new Date().toISOString(), stateAt: new Date().toISOString(),
      cezCheckedAt: new Date().toISOString(), apMapUpdatedAt: new Date().toISOString(),
      summary: {}, outages: [], cez: power.cez, sources: power.sources,
    }));
  });
}

/**
 * What network monitoring measured over time, per address, as its Grafana
 * hands it out: { rttMs, loss, lossLastHour } for routers, { down, up } in
 * bits per second (the peak) for a member's connection.
 */
export const measurements = new Map();

/** Network monitoring's Grafana: the data source query API, answering for the queried address. */
function fakeGrafana() {
  return createHttpServer((req, res) => {
    let body = '';
    req.on('data', (d) => { body += d; });
    req.on('end', () => {
      if (req.method !== 'POST' || req.url !== '/grafana/api/ds/query') return res.writeHead(404).end();
      const q = JSON.parse(body).queries[0].query;
      const ip = /"ip" = '([\d.]+)'/.exec(q)[1];
      const [, from, to, step] = /time >= (\d+)s AND time < (\d+)s GROUP BY time\((\d+)s\)/.exec(q).map(Number);
      const traffic = q.includes('FROM "traffic"');
      const m = measurements.get(ip);
      const times = [];
      const a = [];
      const b = [];
      const now = Date.now() / 1000;
      if (m) {
        for (let t = from; t < Math.min(to, now); t += step) {
          times.push(t * 1000);
          if (traffic) {
            // The peak once in the evening, a tenth of it otherwise.
            const peak = times.length === 100;
            a.push(peak ? m.down : m.down / 10);
            b.push(peak ? m.up : m.up / 10);
          } else {
            a.push(m.rttMs);
            b.push(now - t <= 3600 ? m.lossLastHour : 0);
          }
        }
      }
      const name = traffic ? 'traffic' : 'ping';
      const frame = (alias, values) => ({ schema: { name: `${name}.${alias}`, fields: [{ name: 'Time' }, { name: 'Value' }] }, data: { values: [times, values] } });
      res.writeHead(200, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify({ results: { A: { status: 200, frames: m ? [frame('a', a), frame('b', b)] : [] } } }));
    });
  });
}

/** What network monitoring reports per address; unlisted addresses answer fine. */
export const monitoring = new Map();

function fakeSojka() {
  return createHttpServer((req, res) => {
    let body = '';
    req.on('data', (d) => { body += d; });
    req.on('end', () => {
      const now = Math.floor(Date.now() / 1000);
      const out = {};
      for (const ip of JSON.parse(body || '[]')) {
        const m = monitoring.get(ip) || 'ok';
        out[ip] = {
          ok: { alive: true, loss: 0, rtt: 0.005, time_lastpong: now },
          lossy: { alive: true, loss: 0.4, rtt: 0.005, time_lastpong: now },
          dead: { alive: false, loss: 1, rtt: 0, time_lastpong: now - 600 },
          decommissioned: { alive: false, loss: 1, rtt: 0, time_lastpong: now - 30 * 24 * 3600 },
        }[m];
      }
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(out));
    });
  });
}

const USERDB_KEY = 'bdd-key';
const USERDB_AUTH = `Basic ${Buffer.from(`apikey1:${USERDB_KEY}`).toString('base64')}`;

/** The member records: one AP whose infrastructure includes the router 10.107.10.149. */
const USERDB = {
  '/userdb/api/areas': {
    7: {
      id: 7,
      jmeno: 'Podůlšany',
      admins: [
        { id: 3, nick: 'tech', email: 'tech@example.org', role: 'TECH' },
        { id: 2, nick: 'petr', email: 'petr@example.org', role: 'ZSO' },
        { id: 1, nick: 'jan', email: 'jan@example.org', role: 'SO' },
      ],
      aps: { 10: { id: 10, jmeno: 'Podůlšany', ulice_cp: 'Podůlšany 1', mesto: 'Podůlšany', psc: 53345 } },
    },
  },
  '/userdb/api/monitoring/get-zarizeni': {
    result: 'OK',
    zarizeni: {
      '10.107.10.149': { hostname: 'ap-podulsany', popis: '', Ap_id: 10, Uzivatel_id: null },
      '10.107.10.150': { hostname: '', popis: 'sektor jih', Ap_id: 10, Uzivatel_id: null },
      '10.107.10.151': { hostname: '', popis: 'spoj na Dolany', Ap_id: 10, Uzivatel_id: null },
      '10.107.10.152': { hostname: '', popis: 'stary sektor', Ap_id: 10, Uzivatel_id: null },
    },
  },
};

/**
 * UserDB's status (status/get-problemove-ip): per "area - AP" name, the AP's
 * devices with a problem right now. Problem codes: 0 doesn't answer, 1 high
 * loss, 2 loss, 3 slow.
 */
export const userdbStatus = {};

/** Members' own devices at the AP; UserDB lists them only when asked for users' devices. */
const MEMBER_DEVICES = {
  '10.107.10.146': { hostname: '', popis: '', Ap_id: 10, Uzivatel_id: 500 },
  '10.107.10.140': { hostname: '', popis: '', Ap_id: 10, Uzivatel_id: 501 },
};

/** The member records can be made unreachable (every request fails). */
export const userdbReach = { down: false };

function fakeUserDb() {
  return createHttpServer((req, res) => {
    req.resume();
    if (userdbReach.down) return res.writeHead(503).end();
    const url = new URL(req.url, 'http://x');
    let body = USERDB[url.pathname];
    if (url.pathname === '/userdb/api/wewimo/default/10') body = wewimo();
    if (url.pathname === '/userdb/api/status/get-problemove-ip') body = userdbStatus;
    if (body && url.pathname.endsWith('/get-zarizeni') && url.searchParams.get('uzivatele') === '1') {
      body = { ...body, zarizeni: { ...body.zarizeni, ...MEMBER_DEVICES } };
    }
    if (req.headers.authorization !== USERDB_AUTH) {
      res.writeHead(401).end();
    } else if (!body) {
      res.writeHead(404).end();
    } else {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(body));
    }
  });
}

async function startServer() {
  const dir = mkdtempSync(join(tmpdir(), 'hkdiag-bdd-'));
  const frr = fakeFrr();
  const sojka = fakeSojka();
  const userdb = fakeUserDb();
  const mt = fakeMikrotik();
  const pw = fakePower();
  const grafana = fakeGrafana();
  const grafanaPort = await listen(grafana);
  const frrPort = await listen(frr);
  const sojkaPort = await listen(sojka);
  const userdbPort = await listen(userdb);
  const mtPort = await listen(mt);
  const pwPort = await listen(pw);
  const stop = () => { frr.close(); sojka.close(); userdb.close(); mt.close(); pw.close(); grafana.close(); };
  const webPort = await freePort();
  const config = join(dir, 'config.toml');
  writeFileSync(config, [
    '[frr]', 'host = "127.0.0.1"', `port = ${frrPort}`, 'ospf6_host = "127.0.0.1"', `ospf6_port = ${frrPort}`, `password = "${VTY_PASSWORD}"`, 'timeout_secs = 5',
    '[sojka]', `url = "http://127.0.0.1:${sojkaPort}/pinger/"`, 'timeout_secs = 5',
    '[userdb]', `url = "http://127.0.0.1:${userdbPort}/userdb/api"`, 'api_key_id = 1', `api_key = "${USERDB_KEY}"`, 'timeout_secs = 5',
    '[support]', 'name = "Podpora hkfree.org"', 'email = "podpora@hkfree.org"', 'phone = "608 10 10 70"', 'phone_hours = "9-17"', 'slack_channel = "#nejdeto"',
    '[speedtest]', 'url = "https://speedtest.hkfree.org/backend/"',
    '[history]', `url = "http://127.0.0.1:${grafanaPort}/grafana"`, 'ping_datasource = "ping"', 'traffic_datasource = "traffic"', 'cache_secs = 1', 'timeout_secs = 5',
    '[power]', `url = "http://127.0.0.1:${pwPort}/vypadky.json"`, 'poll_secs = 1',
    '[network]', 'vpn = ["10.107.254.0/24", "2a01:168:0:dead::/64"]',
    '[links]', 'userdb_ap = "https://userdb.hkfree.org/userdb/ap/show/{id}"', 'userdb_status = "https://userdb.hkfree.org/userdb/status"',
    'monitoring = "http://sojka.hkfree.org/cgi-bin/smokeping.cgi"',
    '[[mikrotik]]', `url = "http://127.0.0.1:${mtPort}"`, 'user = "read"', 'password = "read"', 'timeout_secs = 2', 'retry_after_secs = 1', 'insecure_http = true',
    '[web]', 'topology_cache_secs = 1', 'signal_cache_secs = 1', 'status_poll_secs = 1', `listen = "127.0.0.1:${webPort}"`, 'trusted_proxies = ["127.0.0.1"]', `reports_db = "${join(dir, 'reports.sqlite')}"`,
  ].join('\n'));
  chmodSync(config, 0o600);
  const proc = spawn(BINARY, ['--config', config, 'serve'], { env: { ...process.env, XDG_STATE_HOME: dir, HKDIAG_TRACE_REQUESTS: '1' }, stdio: ['ignore', 'ignore', 'pipe'] });
  let log = '';
  let written = 0; // all the log ever written, to find a scenario's part in what's kept
  await new Promise((resolve, reject) => {
    proc.stderr.on('data', (d) => {
      // Keep only the recent part: enough to explain a failure.
      written += String(d).length;
      log = (log + d).slice(-200_000);
      if (String(d).includes('listening')) resolve();
    });
    proc.on('exit', (code) => {
      stop();
      reject(new Error(`hkdiag exited with ${code}: ${log.trim()}`));
    });
  });
  return {
    url: `http://127.0.0.1:${webPort}`, config, dir,
    /** The server's last log lines, for explaining a failure. */
    logTail: (n = 30) => log.split('\n').slice(-n).join('\n'),
    /** Where the log is now, for logSince(). */
    logMark: () => written,
    /** What the server logged since `mark` (as much of it as is kept). */
    logSince: (mark) => log.slice(-Math.min(log.length, written - mark)),
    /** The server's log lines mentioning `text` (e.g. a stalled path's trace). */
    logLines: (text) => log.split('\n').filter((l) => l.includes(text)).slice(-10).join('\n'),
    close: () => { proc.kill(); stop(); },
  };
}

let system;
let browser;

BeforeAll(async () => {
  if (!existsSync(BINARY)) throw new Error(`${BINARY} missing: run cargo build --release first`);
  system = await startServer();
  const executablePath = BROWSERS.find((p) => p && existsSync(p));
  if (!executablePath) throw new Error('No Chromium found; set BDD_BROWSER');
  browser = await chromium.launch({ executablePath });
});

AfterAll(async () => {
  await browser?.close();
  system?.close();
});

/** Per scenario: one member device (browser context) in a network we control. */
class Member extends World {
  /** ip -> 'answers' | 'silent'; anything not listed is silent. */
  hosts = new Map();
  serverReachable = true;
  internetReachable = true;
  /** The server can't hand out the admin's contact (e.g. UserDB is down). */
  contactFails = false;
  /** Names (google.com, cloudflare.com) resolve; addresses work regardless. */
  namesResolve = true;
  /** Every connection from the device fails at once (no link, no route). */
  nothingLeaves = false;
  /** The device asks apps to reduce motion. */
  reducedMotion = false;
  /** The page's clock can be moved forward (timers fire), otherwise it runs normally. */
  fakeClock = false;
  /** The speed test server (LibreSpeed at speedtest.hkfree.org) answers. */
  speedtestUp = true;
  page = null;
  context = null;

  async open(address) {
    // BDD_PHONE=1: run the scenarios at a phone's screen size.
    const phone = process.env.BDD_PHONE ? { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } : {};
    this.context = await browser.newContext({
      locale: 'en-US', extraHTTPHeaders: { 'X-Real-IP': address }, reducedMotion: this.reducedMotion ? 'reduce' : 'no-preference', ...phone,
    });
    const hang = () => new Promise(() => {});
    const fail = (route) => route.abort('internetdisconnected');
    await this.context.route(/^https:\/\/(1\.1\.1\.1|www\.google\.com|cloudflare\.com)\//, (route) => {
      if (this.nothingLeaves) return fail(route);
      const byName = new URL(route.request().url()).hostname !== '1.1.1.1';
      if (byName && !this.namesResolve) return route.abort('namenotresolved');
      return this.internetReachable ? route.fulfill({ status: 204 }) : hang();
    });
    // LibreSpeed's backend, answering with its CORS headers as the real one does with ?cors.
    const CORS = { 'Access-Control-Allow-Origin': '*', 'Access-Control-Allow-Methods': 'GET, POST' };
    const chunk = Buffer.alloc(1 << 20, 7);
    await this.context.route(/^https:\/\/speedtest\.hkfree\.org\/backend\//, (route) => {
      if (this.nothingLeaves || !this.speedtestUp) return fail(route);
      const url = new URL(route.request().url());
      if (url.pathname.endsWith('/garbage.php')) return route.fulfill({ status: 200, headers: { ...CORS, 'Content-Type': 'application/octet-stream' }, body: chunk });
      if (url.pathname.endsWith('/empty.php')) return route.fulfill({ status: 200, headers: CORS, body: '' });
      return route.fulfill({ status: 404, headers: CORS, body: '' });
    });
    await this.context.route(/^https:\/\/192\.0\.2\.1\//, (route) => (this.nothingLeaves ? fail(route) : hang()));
    await this.context.route(/^https:\/\/(10\.\d+\.\d+\.\d+)\//, (route) => {
      if (this.nothingLeaves) return fail(route);
      const host = new URL(route.request().url()).hostname;
      return this.hosts.get(host) === 'answers' ? route.fulfill({ status: 204 }) : hang();
    });
    await this.context.route(`${system.url}/api/**`, (route) => {
      if (this.nothingLeaves) return fail(route);
      if (!this.serverReachable) return hang();
      if (this.contactFails && new URL(route.request().url()).pathname === '/api/contact') return route.fulfill({ status: 502, body: 'unavailable' });
      return route.continue();
    });
    await this.context.route(`${system.url}/topology.json`, (route) => {
      if (this.nothingLeaves) return fail(route);
      return this.serverReachable ? route.continue() : hang();
    });
    this.page = await this.context.newPage();
    if (this.fakeClock) await this.page.clock.install();
    // Which requests are still open if the page doesn't load (CI has stalled here).
    const pending = new Map();
    const opened = (r) => pending.set(r, Date.now());
    const closed = (r) => pending.delete(r);
    this.context.on('request', opened);
    this.context.on('requestfinished', closed);
    this.context.on('requestfailed', closed);
    try {
      await this.page.goto(`${system.url}/?tab=check&autorun=0`);
    } catch (e) {
      const open = [...pending].map(([r, t]) => `${r.method()} ${r.url()} (${Math.round((Date.now() - t) / 1000)} s)`);
      // Did the stalled requests reach the server at all, and were they answered?
      const traces = [...pending.keys()].map((r) => {
        const path = new URL(r.url()).pathname;
        return `${path}:\n${system.logLines(` ${path} `) || '  (never reached the server)'}`;
      });
      throw new Error(`${e.message}\nrequests still open:\n  ${open.join('\n  ') || '(none)'}`
        + `\nserver trace of those:\n${traces.join('\n')}\nserver log:\n${system.logTail()}`);
    } finally {
      // Only the page load is watched.
      this.context.off('request', opened);
      this.context.off('requestfinished', closed);
      this.context.off('requestfailed', closed);
      pending.clear();
    }
  }

  wholeNetworkAnswers() {
    for (const r of ALL_ROUTERS) this.hosts.set(r, 'answers');
    this.serverReachable = true;
    this.internetReachable = true;
  }

  /** Everything beyond the device stops answering, except `answering` routers. */
  networkFails(...answering) {
    this.hosts.clear();
    for (const r of answering) this.hosts.set(r, 'answers');
    this.serverReachable = false;
    this.internetReachable = false;
  }

  async healthyCheck(address) {
    await this.open(address);
    this.wholeNetworkAnswers();
    await this.runCheck();
  }

  async runCheck() {
    await this.page.click('#run');
    await this.page.waitForFunction(() => !document.querySelector('#run').disabled
      && !document.querySelector('#verdict').className.includes('idle'), null, { timeout: 45_000 });
  }

  /** What the server tells a device with `address` when it asks for its AP admin's contact. */
  async contactFor(address) {
    return (await fetch(`${system.url}/api/contact`, { headers: { 'X-Real-IP': address } })).text();
  }

  /** The "whom to turn to" part of the advice: admin lines and the support line. */
  async contacts() {
    return this.page.evaluate(() => ({
      admins: [...document.querySelectorAll('#advice .admin-contact')].map((p) => ({
        text: p.textContent.trim(),
        mail: [...p.querySelectorAll('a[href^="mailto:"]')].map((a) => a.getAttribute('href').slice(7)),
      })),
      support: (document.querySelector('#advice .support') || {}).textContent || '',
    }));
  }

  /** What the server tells a device with `address` when it asks for its antenna's signal. */
  async signalFor(address) {
    return (await fetch(`${system.url}/api/signal`, { headers: { 'X-Real-IP': address } })).text();
  }

  /** Asks for the antenna's signal in advanced diagnostics and waits for the answer. */
  async askForSignal() {
    await this.openAdvanced();
    await this.page.click('#signal-btn');
    await this.page.waitForSelector('#path-map [data-node] .signal', { timeout: 30_000 });
  }

  /** What the map's AP node says about the antenna's signal. */
  async signalText() {
    return this.page.$$eval('#path-map [data-node] .signal', (t) => t.map((x) => x.textContent.trim()).join(' | '));
  }

  /** Changes what monitoring reports and waits until the server's map reflects it. */
  async monitoringReports(ip, state) {
    monitoring.set(ip, state);
    await new Promise((r) => setTimeout(r, 1100));
    return this.networkMap();
  }

  /** Sends outage reports as the app on `member`'s device would. */
  async report(member, reports) {
    const res = await fetch(`${system.url}/api/reports`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Origin: system.url, 'X-Real-IP': member },
      body: JSON.stringify(reports),
    });
    assert.equal(res.status, 200, await res.text());
  }

  /** The public outage overview, as the server hands it out. */
  async publicOutages() {
    return (await fetch(`${system.url}/api/outages`)).json();
  }

  /** Outage reports as an admin sees them today: `hkdiag reports --json` on the server. */
  async adminReports() {
    const out = await new Promise((resolve, reject) => {
      execFile(BINARY, ['--config', system.config, 'reports', '--json', '--hours', '2000'],
        { env: { ...process.env, XDG_STATE_HOME: system.dir } }, (err, stdout, stderr) => (err ? reject(new Error(stderr || err.message)) : resolve(stdout)));
    });
    return JSON.parse(out || '[]');
  }

  /** Outages the app keeps on the device. */
  async savedOutages() {
    return this.page.evaluate(() => JSON.parse(localStorage.getItem('outages') || '[]'));
  }

  /** The test server's address, for steps that talk to it directly. */
  /** Runs `hkdiag verify` against the test server, as the admin would after a deployment. */
  verifyServer() {
    return new Promise((resolve) => {
      execFile(BINARY, ['verify', system.url], { timeout: 120_000 }, (err, stdout) => {
        resolve({ ok: !err, out: stdout });
      });
    });
  }

  get serverUrl() {
    return system.url;
  }

  /** What the server logged during this scenario. */
  serverLog() {
    return system.logSince(this.logStart);
  }

  /** The network map as the real server hands it out. */
  async networkMap() {
    return (await fetch(`${system.url}/topology.json`)).json();
  }

  async openAdvanced() {
    if (!(await this.page.isVisible('#checks'))) await this.page.click('#advanced > summary');
  }

  /** What advanced diagnostics say about the member's AP, or '' if nothing. */
  async apInfo() {
    return this.page.evaluate(() => {
      const ap = document.querySelector('#path-map [data-node="ap"]');
      if (!ap) return null;
      return {
        name: ap.querySelector('.map-label').textContent,
        where: (ap.querySelector('.ap-info') || {}).textContent || '',
      };
    });
  }

  async badge() {
    return (await this.page.textContent('#net-badge')).trim();
  }

  async verdict() {
    return {
      title: await this.page.textContent('#verdict .verdict-title'),
      text: await this.page.textContent('#verdict .verdict-text'),
      advice: (await this.page.textContent('#advice')) || '',
    };
  }
}

setWorldConstructor(Member);

Before(function () {
  this.logStart = system.logMark();
  this.hosts = new Map();
  monitoring.clear();
  power.sources = [];
  power.cez = [];
  for (const k of Object.keys(userdbStatus)) delete userdbStatus[k];
  mikrotik.mode = 'blind';
  radio.clear();
  measurements.clear();
  userdbReach.down = false;
  monitoring.set('10.107.10.152', 'decommissioned');
});

After(async function ({ pickle }) {
  // A scenario that changed what OSPF says leaves the next one a healthy map.
  if (ospf.failed.size || ospf.root !== EXIT) {
    ospf.failed.clear();
    ospf.root = EXIT;
    await new Promise((r) => setTimeout(r, 1100));
  }
  // BDD_SHOTS=<dir>: keep a screenshot of every scenario's last screen.
  if (process.env.BDD_SHOTS && this.page) {
    await this.page.screenshot({ path: join(process.env.BDD_SHOTS, `${pickle.name.replace(/[^\w-]+/g, '_')}.png`), fullPage: true });
  }
  await this.context?.close();
});
