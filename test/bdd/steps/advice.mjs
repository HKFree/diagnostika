import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

const MEMBER = '10.107.77.5';

Given('the internet answers at the address {string}', async function (address) {
  assert.equal(address, '1.1.1.1', 'the app checks the internet by address at 1.1.1.1');
  await this.open(MEMBER);
  this.wholeNetworkAnswers();
});

Given('the name {string} doesn\'t resolve', function (name) {
  assert.equal(name, 'google.com');
  this.namesResolve = false;
});

Given('the hkfree.org server answers', async function () {
  await this.open(MEMBER);
  this.wholeNetworkAnswers();
});

Given('the internet doesn\'t answer, not even at the address {string}', function (address) {
  assert.equal(address, '1.1.1.1');
  this.internetReachable = false;
});

Given('the device reports a Wi-Fi connection', async function () {
  // A previous working check left the network map on the device.
  await this.healthyCheck(MEMBER);
  assert.equal(await this.page.evaluate(() => navigator.onLine), true);
});

Given('every connection from it fails immediately', function () {
  this.nothingLeaves = true;
});

Given('the member\'s device asks apps to reduce motion', async function () {
  this.reducedMotion = true;
  await this.open(MEMBER);
  this.wholeNetworkAnswers();
});

Then('they see that it is a DNS problem', async function () {
  assert.equal((await this.verdict()).title, 'DNS problem');
});

Then('they are advised to set different DNS servers', async function () {
  const { advice } = await this.verdict();
  assert.ok(advice.includes('Set different DNS servers'), advice);
});

Then('they see that their path in hkfree.org is fine and the problem is beyond the internet exit', async function () {
  const v = await this.verdict();
  assert.equal(v.title, 'hkfree.org works, the internet does not');
  assert.ok(v.text.includes('Your path inside hkfree.org is fine') && v.text.includes('internet exit or beyond'), v.text);
});

Then('they see that nothing leaves their device', async function () {
  assert.equal((await this.verdict()).title, 'Nothing leaves this device');
});

Then('they are advised to renew the connection and restart the router', async function () {
  const { advice } = await this.verdict();
  assert.ok(advice.includes('reconnect') && advice.includes('Restart your router'), advice);
});

Then('they see the map of their path without moving traffic', async function () {
  const still = await this.page.evaluate(() => {
    const segs = [...document.querySelectorAll('#path-map [data-segment]')];
    return { n: segs.length, moving: segs.filter((s) => getComputedStyle(s).animationName !== 'none').length };
  });
  assert.ok(still.n > 0, 'no map');
  assert.equal(still.moving, 0, JSON.stringify(still));
});
