import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

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
  assert.ok(checks.includes('The routers behind the border router aren\'t in the network map'), checks);
});
