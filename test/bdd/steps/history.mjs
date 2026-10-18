import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { measurements } from '../support/system.mjs';

const fresh = () => new Promise((r) => setTimeout(r, 1100)); // the server keeps a series for a second in tests

Given('network monitoring measured the router {string} answering in {int} ms over the last day, with {int} % packet loss in the last hour', async function (ip, ms, loss) {
  measurements.set(ip, { rttMs: ms, lossLastHour: loss / 100 });
  await fresh();
});

Given('network monitoring measured the router {string} answering in {int} ms over the last week', async function (ip, ms) {
  measurements.set(ip, { rttMs: ms, lossLastHour: 0 });
  await fresh();
});

Given('network monitoring measured the connection of the member {string} downloading up to {int} Mbit\\/s and uploading up to {int} Mbit\\/s over the last day', async function (member, down, up) {
  measurements.set(member, { down: down * 1e6, up: up * 1e6 });
  this.member = member;
  await fresh();
});

/** Opens the path tab of `address` and the history of `router`, over `range`. */
async function openHistory(world, address, router, range) {
  await world.healthyCheck(address);
  await world.page.click('[data-tab="path"]');
  await world.page.click(`#hops button[data-history="${router}"]`);
  if (range === 'week') await world.page.click(`#hops .chart-box[data-history="${router}"] button[data-range="week"]`);
  await world.page.waitForSelector(`#hops .chart-box[data-history="${router}"] .chart-summary`, { timeout: 30_000 });
  await world.page.waitForFunction((r) => {
    const box = document.querySelector(`#hops .chart-box[data-history="${r}"]`);
    return box && box.querySelectorAll('svg.chart path.chart-line').length === 2;
  }, router);
}

When('the member views the path of their address {string} and opens the history of the router {string}', async function (address, router) {
  await openHistory(this, address, router, 'day');
});

When('the member views the path of their address {string} and opens the history of the router {string} over the last week', async function (address, router) {
  await openHistory(this, address, router, 'week');
});

Then('they see a chart of the response time and packet loss of {string} over the last day', async function (router) {
  const box = await this.page.textContent(`#hops .chart-box[data-history="${router}"]`);
  assert.ok(box.includes(`Measurement history of ${router} over the last day`), box);
  assert.ok(box.includes('Response time (ms)') && box.includes('Packet loss (%)'), box);
  assert.ok(box.includes('Response time up to 2 ms'), box);
});

Then('the chart shows packet loss up to {int} %', async function (loss) {
  const box = await this.page.textContent('#hops .chart-box');
  assert.ok(box.includes(`packet loss up to ${loss} %`), box);
});

Then('they see a chart of the response time and packet loss of {string} over the last week', async function (router) {
  const sel = `#hops .chart-box[data-history="${router}"]`;
  await this.page.waitForFunction((s) => (document.querySelector(s) || {}).textContent?.includes('over the last week'), sel);
  const box = await this.page.textContent(sel);
  assert.ok(box.includes('Response time up to 2 ms'), box);
  assert.equal(await this.page.getAttribute(`${sel} button[data-range="week"]`, 'aria-pressed'), 'true');
});

When('a program asks the API for the measurement history of the router {string}', async function (router) {
  const res = await fetch(new URL(`/api/v1/history?ip=${router}`, this.serverUrl));
  this.answer = { status: res.status, cors: res.headers.get('access-control-allow-origin'), body: res.status === 200 ? await res.json() : await res.text() };
});

When('a program asks the API for the measurement history of the address {string}', async function (address) {
  measurements.set(address, { rttMs: 3, lossLastHour: 0 }); // monitoring may well know it
  const res = await fetch(new URL(`/api/v1/history?ip=${address}`, this.serverUrl));
  this.answer = { status: res.status, body: await res.text() };
});

Then('it gets its response time and packet loss over the last day', function () {
  const { status, cors, body } = this.answer;
  assert.equal(status, 200, JSON.stringify(body));
  assert.equal(cors, '*');
  assert.equal(body.range, 'day');
  assert.equal(body.step, 600);
  assert.equal(body.rtt_ms.length, 144);
  assert.ok(body.rtt_ms.filter((v) => v === 2).length >= 140, JSON.stringify(body.rtt_ms));
  assert.ok(Math.max(...body.loss.filter((v) => v !== null)) === 0.3, JSON.stringify(body.loss));
});

Then('it gets no measurements of that address', function () {
  assert.equal(this.answer.status, 404, this.answer.body);
  assert.ok(!this.answer.body.includes('rtt_ms'), this.answer.body);
});

When('the member asks for their connection\'s traffic', async function () {
  await this.healthyCheck(this.member);
  await this.openAdvanced();
  await this.page.click('#traffic-btn');
  await this.page.waitForSelector('#traffic .chart-summary, #traffic .traffic-reason', { timeout: 30_000 });
});

Then('they see a chart of their download and upload over the last day, up to {int} Mbit\\/s down and {int} Mbit\\/s up', async function (down, up) {
  const box = await this.page.textContent('#traffic');
  assert.ok(box.includes(`Traffic of your connection ${this.member} over the last day`), box);
  assert.ok(box.includes(`Download up to ${down} Mbit/s, upload up to ${up} Mbit/s`), box);
  assert.equal(await this.page.locator('#traffic svg.chart path.chart-line').count(), 2);
});

When('the device with the address {string} asks the hkfree.org server for its connection\'s traffic', async function (address) {
  const res = await fetch(new URL('/api/traffic', this.serverUrl), { headers: { 'X-Real-IP': address } });
  this.answer = await res.text();
  this.asker = address;
});

Then('it doesn\'t get the traffic of {string}', function (member) {
  const body = JSON.parse(this.answer);
  assert.equal(body.ip, this.asker, this.answer);
  assert.ok(!this.answer.includes(member), this.answer);
  // Monitoring knows nothing of the asker's own address: nothing of the member's shows up either.
  assert.ok(body.down_bps.every((v) => v === null || v === 0), this.answer);
});

Then('they can\'t ask for their connection\'s traffic', async function () {
  assert.equal(await this.page.isHidden('#traffic-btn'), true);
});
