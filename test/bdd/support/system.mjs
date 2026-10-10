// Runs the real system for the scenarios: the hkdiag server (release build)
// reading a stand-in FRR over the vty protocol, and the real app in Chromium.
// Routers, the internet and the member's address are set per scenario.

import { AfterAll, After, Before, BeforeAll, setDefaultTimeout, setWorldConstructor, World } from '@cucumber/cucumber';
import { spawn } from 'node:child_process';
import { chmodSync, existsSync, mkdtempSync, writeFileSync } from 'node:fs';
import { createServer as createHttpServer } from 'node:http';
import { createServer as createTcpServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright-core';
import { ALL_ROUTERS, externalLsas, networkLsas, ospf6External, ospf6IntraPrefix, routerLsas, summaryLsas } from './network.mjs';

setDefaultTimeout(60_000);

const VTY_PASSWORD = 'bdd';
const BINARY = process.env.HKDIAG_BIN || 'target/release/hkdiag';
const BROWSERS = [process.env.BDD_BROWSER, '/usr/bin/chromium', '/usr/bin/google-chrome', '/usr/bin/chromium-browser'];

const VTY_OUTPUT = {
  'terminal length 0': '',
  'show ip ospf database router json': routerLsas(),
  'show ip ospf database network json': networkLsas(),
  'show ip ospf database external json': externalLsas(),
  'show ip ospf database summary json': summaryLsas(),
  'show ipv6 ospf6 database intra-prefix detail json': ospf6IntraPrefix(),
  'show ipv6 ospf6 database as-external detail json': ospf6External(),
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
          const out = VTY_OUTPUT[line] ?? '% Unknown command';
          socket.write(`${line}\r\n${out}${out ? '\r\n' : ''}bdd> `);
        }
      }
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

function fakeUserDb() {
  return createHttpServer((req, res) => {
    req.resume();
    const body = USERDB[new URL(req.url, 'http://x').pathname];
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
  const frrPort = await listen(frr);
  const sojkaPort = await listen(sojka);
  const userdbPort = await listen(userdb);
  const stop = () => { frr.close(); sojka.close(); userdb.close(); };
  const webPort = await freePort();
  const config = join(dir, 'config.toml');
  writeFileSync(config, [
    '[frr]', 'host = "127.0.0.1"', `port = ${frrPort}`, 'ospf6_host = "127.0.0.1"', `ospf6_port = ${frrPort}`, `password = "${VTY_PASSWORD}"`, 'timeout_secs = 5',
    '[sojka]', `url = "http://127.0.0.1:${sojkaPort}/pinger/"`, 'timeout_secs = 5',
    '[userdb]', `url = "http://127.0.0.1:${userdbPort}/userdb/api"`, 'api_key_id = 1', `api_key = "${USERDB_KEY}"`, 'timeout_secs = 5',
    '[support]', 'name = "Podpora hkfree.org"', 'email = "podpora@hkfree.org"', 'phone = "608 10 10 70"', 'phone_hours = "9-17"', 'slack_channel = "#nejdeto"',
    '[web]', 'topology_cache_secs = 1', `listen = "127.0.0.1:${webPort}"`, 'trusted_proxies = ["127.0.0.1"]', `reports_db = "${join(dir, 'reports.sqlite')}"`,
  ].join('\n'));
  chmodSync(config, 0o600);
  const proc = spawn(BINARY, ['--config', config, 'serve'], { env: { ...process.env, XDG_STATE_HOME: dir }, stdio: ['ignore', 'ignore', 'pipe'] });
  let log = '';
  await new Promise((resolve, reject) => {
    proc.stderr.on('data', (d) => {
      log += d;
      if (String(d).includes('listening')) resolve();
    });
    proc.on('exit', (code) => {
      stop();
      reject(new Error(`hkdiag exited with ${code}: ${log.trim()}`));
    });
  });
  return { url: `http://127.0.0.1:${webPort}`, close: () => { proc.kill(); stop(); } };
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
  page = null;
  context = null;

  async open(address) {
    this.context = await browser.newContext({ locale: 'en-US', extraHTTPHeaders: { 'X-Real-IP': address } });
    const hang = () => new Promise(() => {});
    await this.context.route(/^https:\/\/(1\.1\.1\.1|www\.google\.com|cloudflare\.com)\//, (route) =>
      this.internetReachable ? route.fulfill({ status: 204 }) : hang());
    await this.context.route(/^https:\/\/192\.0\.2\.1\//, () => hang());
    await this.context.route(/^https:\/\/(10\.\d+\.\d+\.\d+)\//, (route) => {
      const host = new URL(route.request().url()).hostname;
      return this.hosts.get(host) === 'answers' ? route.fulfill({ status: 204 }) : hang();
    });
    await this.context.route(`${system.url}/api/**`, (route) => {
      if (!this.serverReachable) return hang();
      if (this.contactFails && new URL(route.request().url()).pathname === '/api/contact') return route.fulfill({ status: 502, body: 'unavailable' });
      return route.continue();
    });
    await this.context.route(`${system.url}/topology.json`, (route) => (this.serverReachable ? route.continue() : hang()));
    this.page = await this.context.newPage();
    await this.page.goto(`${system.url}/?tab=check&autorun=0`);
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

  /** Changes what monitoring reports and waits until the server's map reflects it. */
  async monitoringReports(ip, state) {
    monitoring.set(ip, state);
    await new Promise((r) => setTimeout(r, 1100));
    return this.networkMap();
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
    return ((await this.page.textContent('#checks .ap-info')) || '').trim();
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
  this.hosts = new Map();
  monitoring.clear();
  monitoring.set('10.107.10.152', 'decommissioned');
});

After(async function ({ pickle }) {
  // BDD_SHOTS=<dir>: keep a screenshot of every scenario's last screen.
  if (process.env.BDD_SHOTS && this.page) {
    await this.page.screenshot({ path: join(process.env.BDD_SHOTS, `${pickle.name.replace(/[^\w-]+/g, '_')}.png`), fullPage: true });
  }
  await this.context?.close();
});
