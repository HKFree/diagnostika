import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { ospf } from '../support/network.mjs';

const fresh = () => new Promise((r) => setTimeout(r, 1100)); // the server rebuilds its map every second in tests

Given('the link between the routers {string} and {string} fails, leaving the AP router {string} in a part of the network without an internet exit', async function (a, b, ap) {
  // A map with the link up first, so the server has seen it (after the previous scenario's map expired).
  await fresh();
  const before = await this.networkMap();
  assert.ok(before.routers[a].links.some(([to]) => to === b), 'the link is up before it fails');
  ospf.failed.add(`${a} ${b}`);
  await fresh();
  const after = await this.networkMap();
  assert.ok(!after.routers[a].links.some(([to]) => to === b), 'the link is gone');
  assert.ok(ap in after.routers);
  this.link = [a, b];
});

Given('the member {string} still reaches the hkfree.org server', async function (member) {
  await this.open(member);
  this.member = member;
  this.networkFails('10.107.77.1', '10.107.76.1');
  this.serverReachable = true;
});

Given('the network map is read from the router {string} in that part', async function (router) {
  ospf.root = router;
  await fresh();
  assert.equal((await this.networkMap()).root, router);
});

async function viewPath(world, address) {
  await world.page.click('[data-tab="path"]');
  if (address) {
    await world.page.fill('#view-ip', address);
    await world.page.click('#view-form button[type="submit"], #view-form button');
  }
  await world.page.waitForFunction(() => document.querySelector('#findings').children.length > 0);
}

When('the member views the path of their home address', async function () {
  await viewPath(this, null);
});

When('anyone views the path of the address {string}', async function (address) {
  await this.healthyCheck('10.107.10.146');
  await viewPath(this, address);
});

Then('they see that their part of the network is cut off from the internet at the link between {string} and {string}', async function (a, b) {
  const v = await this.verdict();
  assert.equal(v.title, 'Your part of the network is cut off from the internet');
  assert.ok(v.text.includes(`link between ${a} and ${b}`), v.text);
});

Then('they see that the part of the network with the AP router {string} has no internet exit since the link between {string} and {string} failed', async function (ap, a, b) {
  const findings = await this.page.textContent('#findings');
  assert.ok(findings.includes(`The part of the network with ${ap}`) && findings.includes('has no internet exit'), findings);
  assert.ok(findings.includes(`link between ${a} and ${b}`), findings);
});

Then('they see its path to the exit {string} without a problem', async function (exit) {
  const verdict = await this.page.textContent('#path-verdict');
  assert.ok(verdict.includes('shows no problem'), verdict);
  assert.ok((await this.page.textContent('#hops')).includes(exit));
});
