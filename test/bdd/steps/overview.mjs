import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { userdbStatus } from '../support/system.mjs';

const AP_ROUTER = { '10.107.10.146': '10.107.10.149', '10.107.77.5': '10.107.77.1', '10.107.77.6': '10.107.77.1', '10.107.77.7': '10.107.77.1' };
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
  await this.report(member, [outage(member, router, 600, minutes * 60)]);
  this.reported = member;
});

Given('one outage was reported 2 days ago and another 8 days ago', async function () {
  await this.report('10.107.77.5', [outage('10.107.77.5', '10.107.2.1', 2 * 86400, 120)]);
  await this.report('10.107.77.6', [outage('10.107.77.6', '10.107.99.1', 8 * 86400, 120)]);
});

Given('in the last week {int} members reported an outage of the router {string}', async function (n, router) {
  const members = ['10.107.77.5', '10.107.77.6', '10.107.77.7'].slice(0, n);
  for (const m of members) await this.report(m, [outage(m, router, 3 * 3600, 300)]);
});

Given('the member records\' status reports packet loss at the AP {string}', async function (ap) {
  userdbStatus[ap] = { '10.107.10.150': { problem: 2, loss: 0.4, rtt: 0.01, time_lastpong: now(), 'ip-popis': 'sektor jih', 'ip-hostname': '' } };
  await poll();
});

Given('network monitoring found that the device {string} of the AP {string} stopped answering and later answered again', async function (label, ap) {
  userdbStatus[ap] = { '10.107.10.150': { problem: 0, loss: 1, rtt: 0, time_lastpong: now() - 120, 'ip-popis': label, 'ip-hostname': '' } };
  await poll();
  delete userdbStatus[ap];
  await poll();
});

When('anyone views the outage overview', async function () {
  await this.open('10.107.77.5');
  await this.page.click('[data-tab="outages"]');
  await this.page.waitForFunction(() => document.querySelector('#outages').children.length > 0, null, { timeout: 15_000 });
});

Then('they see an outage of the router {string} that lasted {int} minute(s)', async function (router, minutes) {
  const items = await this.page.$$eval('#outages li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(router) && t.includes(`${minutes} min`)), JSON.stringify(items));
});

Then('they don\'t see the address {string}', async function (address) {
  assert.ok(!(await this.page.textContent('#tab-outages')).includes(address));
  assert.ok(!JSON.stringify(await this.publicOutages()).includes(address), 'not even in the data the server hands out');
});

Then('they see only the outage from 2 days ago', async function () {
  const text = await this.page.textContent('#outages');
  assert.ok(text.includes('10.107.2.1'), text);
  assert.ok(!text.includes('10.107.99.1'), 'the outage from 8 days ago is past the week');
});

Then('they see the router {string} among the most often reported, with {int} reports', async function (router, n) {
  const items = await this.page.$$eval('#outages-top li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(router) && t.includes(`${n} reports`)), JSON.stringify(items));
});

Then('they see the AP {string} among the places with a problem right now', async function (ap) {
  const items = await this.page.$$eval('#outages-now li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(ap) && t.includes('packet loss')), JSON.stringify(items));
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
