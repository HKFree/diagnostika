import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

const AP_ROUTER = { Podůlšany: '10.107.10.149' };
const now = () => Math.floor(Date.now() / 1000);

/** Speed results as members' apps send them: each from its own member, with the AP router only. */
async function measured(world, ap, members, downs) {
  // The server judges the senders by its network map: build it first.
  await fetch(new URL('/topology.json', world.serverUrl));
  for (const [i, down] of downs.entries()) {
    const res = await fetch(new URL('/api/speeds', world.serverUrl), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json', Origin: world.serverUrl, 'X-Real-IP': members[i] },
      body: JSON.stringify([{ id: `speed-${now()}-${i}-${Math.random().toString(16).slice(2, 8)}`, ap, measured_at: now() - 3600 * (i + 1), down_mbps: down, up_mbps: down / 10, ping_ms: 8 }]),
    });
    const text = await res.text();
    assert.equal(res.status, 200, text);
    assert.equal(JSON.parse(text).accepted, 1, text);
  }
  world.speedMembers = members;
}

Given('members of the AP {string} measured download speeds of {int}, {int} and {int} Mbit\\/s this week', async function (ap, a, b, c) {
  await measured(this, AP_ROUTER[ap], ['10.107.10.141', '10.107.10.142', '10.107.10.143'], [a, b, c]);
});

Given('members behind the router {string}, which the member records don\'t name, measured download speeds of {int}, {int} and {int} Mbit\\/s this week', async function (router, a, b, c) {
  await measured(this, router, ['10.107.77.21', '10.107.77.22', '10.107.77.23'], [a, b, c]);
});

Given('one member of the AP {string} measured a download speed of {int} Mbit\\/s this week', async function (ap, a) {
  await measured(this, AP_ROUTER[ap], ['10.107.10.141'], [a]);
});

When('anyone looks at the speeds measured in the network', async function () {
  const res = await fetch(new URL('/api/v1/speeds', this.serverUrl));
  this.answerText = await res.text();
  this.speeds = JSON.parse(this.answerText).speeds;
});

Then('they see the AP {string} with a typical download speed of {int} Mbit\\/s and the lowest of {int} Mbit\\/s', function (ap, typical, lowest) {
  const g = this.speeds.find((x) => x.ap && x.ap.name === ap);
  assert.ok(g, this.answerText);
  assert.deepEqual([g.ap_router, g.count, g.down_typical, g.down_lowest], [AP_ROUTER[ap], 3, typical, lowest]);
});

Then('they don\'t see which members measured them', function () {
  for (const m of this.speedMembers) assert.ok(!this.answerText.includes(m), this.answerText);
});

Then('they see a typical download speed of {int} Mbit\\/s grouped under the router {string}', function (typical, router) {
  const g = this.speeds.find((x) => x.ap_router === router);
  assert.ok(g && g.ap === null && g.down_typical === typical, this.answerText);
});

Then('they don\'t see the AP {string} among the speeds', function (ap) {
  assert.ok(!this.speeds.some((x) => x.ap_router === AP_ROUTER[ap]), this.answerText);
});

Then('they see that members of their AP typically measure {int} Mbit\\/s download', async function (down) {
  await this.page.waitForSelector('#path-map .speed-ap', { timeout: 30_000 });
  const text = await this.page.textContent('#path-map .speed-ap');
  assert.ok(text.includes(`typically measure ${down} Mbit/s download`), text);
});

Given('the member {string} measured a download speed of {int} Mbit\\/s and then reported an outage', async function (member, down) {
  const t = now();
  const res = await fetch(new URL('/api/reports', this.serverUrl), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Origin: this.serverUrl, 'X-Real-IP': member },
    body: JSON.stringify([{
      id: `speedy-${t}-${Math.random().toString(16).slice(2, 8)}`, detected_at: t - 1800, resolved_at: t - 600, verdict: 'backbone',
      my_ip: member, ap: '10.107.10.149', first_bad: '10.107.241.36', last_ok: '10.107.10.149', path: ['10.107.10.149', '10.107.241.36'], snapshot_at: t - 1900,
      speed: { measured_at: t - 3600, down_mbps: down, up_mbps: 1, ping_ms: 30 },
    }]),
  });
  assert.equal((await res.json()).accepted, 1);
  this.reported = member;
});

Then('they see the download speed of {int} Mbit\\/s with the member\'s outage', function (down) {
  const r = this.adminRows.find((x) => x.my_ip === this.reported && x.speed);
  assert.ok(r && r.speed.down_mbps === down, JSON.stringify(this.adminRows));
});

When('the admin lists the reports', async function () {
  this.adminRows = await this.adminReports();
});
