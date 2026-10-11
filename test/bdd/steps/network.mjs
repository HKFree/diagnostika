import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { mikrotik } from '../support/system.mjs';

Given('the member\'s device reaches hkfree.org over IPv6 with the address {string}', async function (address) {
  await this.open(address);
  this.wholeNetworkAnswers();
});

Given('the member\'s device reaches the hkfree.org server over IPv6 from the address {string} of another provider', async function (address) {
  await this.open(address);
  this.wholeNetworkAnswers();
});

Then('they see that they are on the hkfree.org network', async function () {
  assert.equal(await this.badge(), 'hkfree.org');
});

Then('they see that they are not on the hkfree.org network', async function () {
  assert.equal(await this.badge(), 'other network');
});

Given('the router {string} advertises the IPv6 prefix {string}', async function (router, prefix) {
  const map = await this.networkMap();
  assert.ok(map.prefixes6.some(([p, r]) => p === prefix && r === router), JSON.stringify(map.prefixes6));
  assert.ok(router in map.routers);
});

Given('the router {string} advertises the IPv6 prefix {string} but isn\'t in the network map', async function (router, prefix) {
  const map = await this.networkMap();
  assert.ok(map.prefixes6.some(([p, r]) => p === prefix && r === router), JSON.stringify(map.prefixes6));
  assert.ok(!(router in map.routers));
});

Then('they see their path through the router {string}', async function (router) {
  await this.openAdvanced();
  const checks = await this.page.textContent('#checks');
  assert.ok(checks.includes(`${router} · your AP router`), checks);
});

Then('they see that their path can\'t be determined', async function () {
  await this.openAdvanced();
  const checks = await this.page.textContent('#checks');
  assert.ok(checks.includes('can\'t be determined'), checks);
});

Given('the addresses {string} are in a hidden part of the network behind the border router {string}', async function (prefix, border) {
  const map = await this.networkMap();
  assert.ok(map.prefixes.some(([p, r, kind]) => p === prefix && r === border && kind === 'a'), JSON.stringify(map.prefixes));
  assert.ok(border in map.routers);
});

Given('the router {string} advertises the IPv6 prefix {string} from a hidden part of the network behind the border router {string}', async function (router, prefix, border) {
  const map = await this.networkMap();
  assert.ok(map.prefixes6.some(([p, r]) => p === prefix && r === router), JSON.stringify(map.prefixes6));
  assert.ok(!(router in map.routers));
  assert.ok(map.prefixes.some(([p, r, kind]) => r === border && kind === 'a'), JSON.stringify(map.prefixes));
});

Then('they see their path through the border router {string}', async function (border) {
  await this.openAdvanced();
  const checks = await this.page.textContent('#checks');
  assert.ok(checks.includes(`${border} · border router of a hidden part of the network`), checks);
});

Then('they see that the routers behind the border router aren\'t in the network map', async function () {
  const checks = await this.page.textContent('#checks');
  assert.ok(checks.includes('The routers behind it aren\'t in the network map'), checks);
});

Given('the member\'s device is connected through mobile data', async function () {
  // At home earlier, so the app knows the member's hkfree.org address and path.
  await this.healthyCheck('10.107.77.5');
  await this.context.setExtraHTTPHeaders({ 'X-Real-IP': '100.64.12.34' });
  this.hosts.clear();
});

Given('the member\'s device has no contact with the hkfree.org server', async function () {
  await this.open('10.107.77.5');
  this.wholeNetworkAnswers();
  this.serverReachable = false;
});

Then('right at the top they see a prominent warning that they are connected not through hkfree.org but through another provider', async function () {
  const w = await this.page.evaluate(() => {
    const el = document.querySelector('#net-warning');
    const verdict = document.querySelector('#verdict');
    return {
      visible: Boolean(el && el.getBoundingClientRect().height),
      text: el ? el.textContent : '',
      cls: el ? el.className : '',
      aboveVerdict: Boolean(el && (el.compareDocumentPosition(verdict) & Node.DOCUMENT_POSITION_FOLLOWING)),
    };
  });
  assert.ok(w.visible && w.aboveVerdict && w.cls.includes('other'), JSON.stringify(w));
  assert.ok(w.text.startsWith('You are connected not through hkfree.org but through another provider'), w.text);
});

Then('they see the routers on their path as not verifiable from outside', async function () {
  await this.openAdvanced();
  const statuses = await this.page.$$eval('#path-map [data-node] .map-status', (t) => t.map((x) => [x.closest('[data-node]').dataset.node, x.textContent]));
  const routers = statuses.filter(([role]) => role !== 'device' && role !== 'internet');
  assert.ok(routers.length > 0 && routers.every(([, s]) => s === 'outside hkfree.org'), JSON.stringify(statuses));
});

Then('they see that it can\'t be determined which network they are on', async function () {
  assert.equal(await this.badge(), 'network ?');
  const text = await this.page.textContent('#net-warning');
  assert.equal(text, 'Could not determine which network this device uses.');
});

const mapAfterChange = async (world) => {
  await new Promise((r) => setTimeout(r, 1100)); // the server rebuilds its map every second in tests
  return world.networkMap();
};

Given('the border router {string} lets the network map see inside its part, where the router {string} serves the addresses {string}', async function (border, router, prefix) {
  mikrotik.mode = 'inside';
  const map = await mapAfterChange(this);
  assert.ok(router in map.routers && border in map.routers, Object.keys(map.routers).join(' '));
  assert.ok(map.prefixes.some(([p, r, kind]) => p === prefix && r === router && kind === 's'), JSON.stringify(map.prefixes));
});

Given('the router that lets the network map see inside the part behind {string} doesn\'t answer', async function (border) {
  mikrotik.mode = 'down';
  const map = await mapAfterChange(this);
  assert.ok(border in map.routers && !('10.107.10.65' in map.routers));
});
