import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

const AP_ROUTER = { '10.107.10.146': '10.107.10.149' };

Given('the address {string} belongs to the AP {string} in the area {string} at {string}', async function (address, name, area, at) {
  const map = await this.networkMap();
  const router = map.routers[AP_ROUTER[address]];
  const ap = router && router.ap;
  assert.deepEqual(ap && { name: ap.name, area: ap.area, address: ap.address }, { name, area, address: at });
  await this.open(address);
  this.wholeNetworkAnswers();
});

Given('the AP router {string} of the member {string} isn\'t named in the member records', async function (router, address) {
  const map = await this.networkMap();
  assert.ok(router in map.routers);
  assert.equal(map.routers[router].ap, undefined);
  await this.open(address);
  this.wholeNetworkAnswers();
});

Then('they see that they connect through the AP {string} in the area {string} at {string}', async function (name, area, at) {
  assert.deepEqual(await this.apInfo(), { name, where: `area ${area} · ${at}` });
});

Then('they see their AP router {string} without an AP name', async function (router) {
  assert.deepEqual(await this.apInfo(), { name: router, where: 'The member records don\'t name this AP.' });
});
