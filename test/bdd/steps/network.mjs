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
