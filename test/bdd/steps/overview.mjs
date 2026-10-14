import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { userdbStatus } from '../support/system.mjs';

const AP_ROUTER = {
  '10.107.10.146': '10.107.10.149', '10.107.10.147': '10.107.10.149',
  '10.107.77.5': '10.107.77.1', '10.107.77.6': '10.107.77.1', '10.107.77.7': '10.107.77.1',
};
const now = () => Math.floor(Date.now() / 1000);
const poll = () => new Promise((r) => setTimeout(r, 2200)); // the server asks UserDB's status every second in tests
let seq = 0;

/** One outage report as the app sends it, of `router` failing, `ago` seconds ago, for `secs` seconds. */
function outage(member, router, ago, secs) {
  const at = now() - ago;
  seq += 1;
  return {
    id: `bdd-${Date.now().toString(36)}-${seq}`, detected_at: at, resolved_at: at + secs, verdict: 'backbone',
    my_ip: member, ap: AP_ROUTER[member], first_bad: router, last_ok: AP_ROUTER[member], path: [AP_ROUTER[member], router], snapshot_at: at,
  };
}

Given('the member {string} reported an outage of the router {string} that lasted {int} minute(s)', async function (member, router, minutes) {
  // Ended 10 minutes ago, so its end isn't in the future.
  await this.report(member, [outage(member, router, minutes * 60 + 600, minutes * 60)]);
  this.reported = member;
});

Given('outages on the path of the member {string} were reported 2 hours ago and 2 days ago', async function (member) {
  // The exit and the AP router are both on that member's path (77.1 > 76.1 > 99.1).
  await this.report('10.107.77.6', [outage('10.107.77.6', '10.107.99.1', 2 * 3600, 1200)]);
  await this.report('10.107.77.7', [outage('10.107.77.7', '10.107.77.1', 2 * 86400, 1200)]);
  this.viewer = member;
});

Given('in the last day {int} members reported an outage of the router {string}', async function (n, router) {
  const members = ['10.107.77.5', '10.107.77.6', '10.107.77.7'].slice(0, n);
  for (const m of members) await this.report(m, [outage(m, router, 3 * 3600, 1200)]);
});

// The AP's devices as UserDB knows them: Podůlšany's sector, and a device of an AP elsewhere.
const STATUS_DEVICE = { Podůlšany: ['10.107.10.150', 'sektor jih'], Lipky: ['10.107.19.36', 'LAN V Lipkách 903'] };

Given('the member records\' status reports that a device of the AP {string} hasn\'t answered for {int} minutes', async function (ap, minutes) {
  const [ip, label] = STATUS_DEVICE[ap];
  userdbStatus[ap] = { [ip]: { problem: 0, loss: 1, rtt: 0, time_lastpong: now() - minutes * 60, 'ip-popis': label, 'ip-hostname': '' } };
  await poll();
});

Given('network monitoring found that the device {string} of the AP {string} stopped answering {int} minutes ago and now answers again', async function (label, ap, minutes) {
  userdbStatus[ap] = { '10.107.10.150': { problem: 0, loss: 1, rtt: 0, time_lastpong: now() - minutes * 60, 'ip-popis': label, 'ip-hostname': '' } };
  await poll();
  delete userdbStatus[ap];
  await poll();
});

When('the member {string} views the outage overview', async function (member) {
  // A check first: the app then knows the member's address and paths.
  await this.healthyCheck(member);
  await this.page.click('[data-tab="outages"]');
  await this.page.waitForFunction(() => document.querySelector('#outages').children.length > 0, null, { timeout: 15_000 });
});

Then('they don\'t see an outage of the router {string}', async function (router) {
  // Reported outages say which router didn't answer; the AP router alone is just context.
  const reported = await this.page.$$eval('#outages li', (li) => li.map((x) => x.textContent));
  assert.ok(!reported.some((t) => t.includes(`${router} did not answer`)), JSON.stringify(reported));
  const top = await this.page.textContent('#outages-top');
  assert.ok(!top.includes(router), top);
});

Then('they don\'t see the AP {string}', async function (ap) {
  const text = await this.page.textContent('#tab-outages');
  assert.ok(!text.includes(ap), text);
});

Then('they see an outage of the router {string} that lasted {int} minute(s)', async function (router, minutes) {
  const items = await this.page.$$eval('#outages li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(router) && t.includes(`${minutes} min`)), JSON.stringify(items));
});

Then('they don\'t see the address {string}', async function (address) {
  assert.ok(!(await this.page.textContent('#tab-outages')).includes(address));
  assert.ok(!JSON.stringify(await this.publicOutages()).includes(address), 'not even in the data the server hands out');
});

Then('they see only the outage from 2 hours ago', async function () {
  const items = await this.page.$$eval('#outages li', (li) => li.map((x) => x.textContent));
  // Dates as the app shows them (en-US, short): nothing from 2 days ago.
  const day = (ago) => new Intl.DateTimeFormat('en-US', { dateStyle: 'short' }).format(new Date(Date.now() - ago * 86400 * 1000));
  assert.ok(items.some((t) => t.includes('10.107.99.1')), JSON.stringify(items));
  assert.ok(!items.some((t) => t.startsWith(day(2))), JSON.stringify(items));
});

Then('they see a link to the page of their AP {string} in the member records', async function (ap) {
  const links = await this.page.$$eval('#outages-links a', (as) => as.map((a) => ({ href: a.getAttribute('href'), text: a.textContent })));
  assert.ok(links.some((l) => l.href === 'https://userdb.hkfree.org/userdb/ap/show/10' && l.text.includes(ap)), JSON.stringify(links));
});

Then('a link to the history in network monitoring', async function () {
  const links = await this.page.$$eval('#outages-links a', (as) => as.map((a) => a.getAttribute('href')));
  assert.ok(links.includes('http://sojka.hkfree.org/cgi-bin/smokeping.cgi'), JSON.stringify(links));
});

Then('they see the router {string} among the most often reported, with {int} reports', async function (router, n) {
  const items = await this.page.$$eval('#outages-top li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(router) && t.includes(`${n} reports`)), JSON.stringify(items));
});

Then('they see the AP {string} among the places with a problem right now', async function (ap) {
  const items = await this.page.$$eval('#outages-now li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(ap) && t.includes('doesn\'t answer')), JSON.stringify(items));
});

Then('they see that outage of {string} at the AP {string} with when it started and ended, measured by network monitoring', async function (label, ap) {
  const items = await this.page.$$eval('#outages-measured li', (li) => li.map((x) => x.textContent));
  const item = items.find((t) => t.includes(ap) && t.includes(label));
  assert.ok(item && item.includes('didn\'t answer'), JSON.stringify(items));
  assert.match(item, /\d{1,2}:\d{2}(\s?[AP]M)? – \d{1,2}:\d{2}/, 'with its start and end');
  const data = await this.publicOutages();
  const ep = data.measured.find((m) => m.ap === ap && m.label === label);
  assert.ok(ep && ep.ended && ep.ended >= ep.started, JSON.stringify(data.measured));
});
