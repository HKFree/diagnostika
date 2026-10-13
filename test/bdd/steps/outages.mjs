import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

const AP_ROUTER = { '10.107.10.146': '10.107.10.149', '10.107.77.5': '10.107.77.1' };
const DAY = 24 * 3600;

/** A backbone outage seen by `member`: their AP router answers, the next router doesn't. */
async function findOutage(world, member, router) {
  await world.healthyCheck(member);
  world.member = member;
  world.networkFails(AP_ROUTER[member]);
  assert.notEqual(world.hosts.get(router), 'answers');
  await world.runCheck();
  assert.equal((await world.verdict()).title, 'Outage in the hkfree.org backbone');
}

Given('the member {string} found that the router {string} on their path doesn\'t answer', async function (member, router) {
  await findOutage(this, member, router);
});

Given('the member {string} found an outage of the router {string}', async function (member, router) {
  await findOutage(this, member, router);
});

Given('the member found an outage of the router {string}', async function (router) {
  // The app's own minute timer has to be steered, so the clock is installed before the app starts.
  this.fakeClock = true;
  await findOutage(this, '10.107.10.146', router);
});

Given('the check found that the device of the member {string} isn\'t connected to any network', async function (member) {
  await this.open(member);
  this.member = member;
  await this.context.setOffline(true);
  await this.runCheck();
  assert.equal((await this.verdict()).title, 'This device is offline');
});

Given('the member found an outage {int} days ago and hasn\'t opened the app since', async function (days) {
  await this.open('10.107.10.146');
  this.member = '10.107.10.146';
  this.oldOutageId = `old-outage-${days}d`;
  await this.page.evaluate(([id, at]) => {
    localStorage.setItem('outages', JSON.stringify([{
      id, detected_at: at, verdict: 'backbone', my_ip: '10.107.10.146', ap: '10.107.10.149',
      first_bad: '10.107.241.36', last_ok: '10.107.10.149', path: ['10.107.10.149', '10.107.241.36', '10.107.99.1'], snapshot_at: at,
    }]));
  }, [this.oldOutageId, Math.floor(Date.now() / 1000) - days * DAY]);
  await this.page.reload();
});

Given('then connected through mobile data', async function () {
  await this.context.setExtraHTTPHeaders({ 'X-Real-IP': '100.64.12.34' });
  this.wholeNetworkAnswers();
  this.hosts.clear();
});

Given('the app stayed open', function () {
  assert.ok(this.page && !this.page.isClosed());
});

When('their connection is restored', async function () {
  await this.context.setOffline(false);
  this.wholeNetworkAnswers();
  await this.runCheck();
});

When('they run a check', async function () {
  await this.runCheck();
});

When('the connection is restored {int} minutes later', async function (minutes) {
  this.wholeNetworkAnswers();
  // The app's minute timer fires during this jump and checks again by itself.
  await this.page.clock.fastForward(minutes * 60 * 1000);
});

Then('the member sees that the outage was reported', async function () {
  await this.page.waitForFunction(() => /Reported \d+ outage/.test(document.querySelector('#advice').textContent), null, { timeout: 15_000 });
});

Then('within a minute the member sees that the outage was reported', async function () {
  // Moving the clock only once: jumping it while a check waits for the server
  // would fire that request's timeout, which no real minute does.
  await this.page.waitForFunction(() => /Reported \d+ outage/.test(document.querySelector('#advice').textContent), null, { timeout: 45_000 });
});

Then('the admins see the outage of the router {string} from the member {string}', async function (router, member) {
  const reports = await this.adminReports();
  assert.ok(reports.some((r) => r.first_bad === router && r.my_ip === member && r.client_ip === member), JSON.stringify(reports));
});

Then('the admins see no outage from the member {string}', async function (member) {
  const reports = await this.adminReports();
  assert.ok(!reports.some((r) => r.my_ip === member || r.client_ip === member), JSON.stringify(reports));
  assert.deepEqual(await this.savedOutages(), []);
});

Then('the outage isn\'t reported yet', async function () {
  const saved = await this.savedOutages();
  assert.equal(saved.length, 1);
  assert.ok(!saved[0].resolved_at, 'still open while away from hkfree.org');
  const reports = await this.adminReports();
  assert.ok(!reports.some((r) => r.id === saved[0].id), JSON.stringify(reports));
});

Then('the admins don\'t see this outage', async function () {
  const reports = await this.adminReports();
  assert.ok(!reports.some((r) => r.id === this.oldOutageId), JSON.stringify(reports));
  assert.ok(!(await this.savedOutages()).some((o) => o.id === this.oldOutageId), 'dropped on the device');
});
