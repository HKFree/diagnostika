import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { userdbReach } from '../support/system.mjs';

const MEMBER = '10.107.10.146';

/** A report as a member's app sends it: an outage of the backbone router that ended 10 minutes ago. */
function trialReport(verdict) {
  const now = Math.floor(Date.now() / 1000);
  return {
    id: `trial-${now}-${Math.random().toString(16).slice(2, 10)}`,
    detected_at: now - 1800, resolved_at: now - 600, verdict,
    my_ip: MEMBER, ap: '10.107.10.149', first_bad: '10.107.241.36', last_ok: '10.107.10.149',
    path: ['10.107.10.149', '10.107.241.36', '10.107.99.1'], snapshot_at: now - 1900,
  };
}

async function sendTrial(world, verdict) {
  world.trial = trialReport(verdict);
  const res = await fetch(new URL('/api/reports', world.serverUrl), {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Origin: world.serverUrl, 'X-Real-IP': MEMBER, 'X-Hkdiag-Trial': '1' },
    body: JSON.stringify([world.trial]),
  });
  world.answer = { status: res.status, body: await res.json() };
}

/** Waits until the server's log for this scenario matches `test` (lines are written as answers go out). */
async function logWhere(world, test) {
  for (let i = 0; i < 40; i++) {
    const log = world.serverLog();
    if (test(log)) return log;
    await new Promise((r) => setTimeout(r, 100));
  }
  return world.serverLog();
}

When('the admin sends the server a trial outage report', async function () {
  await sendTrial(this, 'backbone');
});

When('the admin sends the server a trial outage report with a verdict members\' apps don\'t report', async function () {
  await sendTrial(this, 'works');
});

Then('the server says it would store the report', function () {
  assert.equal(this.answer.status, 200);
  assert.deepEqual(this.answer.body, { trial: true, received: 1, would_store: 1, already_stored: 0, refused: [] });
});

Then('the report isn\'t stored', async function () {
  const res = await fetch(new URL(`/api/v1/reports?since=${this.trial.detected_at - 1}`, this.serverUrl));
  const { reports } = await res.json();
  assert.ok(!reports.some((r) => r.detected_at === this.trial.detected_at && r.first_bad === this.trial.first_bad), JSON.stringify(reports));
});

Then('the server says it wouldn\'t store the report, because of the verdict', function () {
  assert.equal(this.answer.status, 200);
  assert.equal(this.answer.body.would_store, 0);
  assert.deepEqual(this.answer.body.refused, ['verdict not reportable']);
});

Then('the server log shows the app\'s requests from the hkfree.org network with their results', async function () {
  // Right after the server starts, a request answered before the first map is built is "unknown".
  const whoami = /access: GET \/api\/whoami 200 \d+ms (hkfree|unknown)/;
  const map = /access: GET \/topology\.json 200 \d+ms hkfree/;
  const log = await logWhere(this, (l) => map.test(l) && whoami.test(l));
  assert.match(log, map);
  assert.match(log, whoami);
  this.logged = log;
});

Then('the server log shows that {int} report(s) was/were received from the hkfree.org network and {int} stored', async function (received, stored) {
  const line = new RegExp(`reports: ${received} received from hkfree, ${stored} stored, 0 already stored, 0 refused`);
  const log = await logWhere(this, (l) => line.test(l));
  assert.match(log, line);
  this.logged = log;
});


const reportCount = async (world) => (await (await fetch(new URL('/api/v1/reports?since=0', world.serverUrl))).json()).reports.length;

When('the admin checks the server from the outside', async function () {
  this.reportsBefore = await reportCount(this);
  this.verified = await this.verifyServer();
});

Then('every check passes', function () {
  assert.ok(this.verified.ok && !this.verified.out.includes('FAIL'), this.verified.out);
  assert.match(this.verified.out, /PASS {2}a trial report would be stored/);
});

Then('no outage report is stored', async function () {
  assert.equal(await reportCount(this), this.reportsBefore);
});

Then('the check of network monitoring\'s outages fails', function () {
  assert.ok(!this.verified.ok, this.verified.out);
  assert.match(this.verified.out, /FAIL {2}network monitoring's outages are being read/);
});

Given('the member records can\'t be reached', async function () {
  userdbReach.down = true;
  await new Promise((r) => setTimeout(r, 2200)); // the server asks UserDB's status every second in tests
});
