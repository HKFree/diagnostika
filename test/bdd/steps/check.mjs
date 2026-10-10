import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { ALL_ROUTERS } from '../support/network.mjs';

function wholeNetworkAnswers(member) {
  for (const r of ALL_ROUTERS) member.hosts.set(r, 'answers');
  member.serverReachable = true;
  member.internetReachable = true;
}

Given('the member\'s device is connected through hkfree.org with the address {string}', async function (address) {
  await this.open(address);
});

Given('their AP router {string} and the internet answer', function (ap) {
  this.hosts.set(ap, 'answers');
  this.internetReachable = true;
});

Given('the AP router {string} of the member {string} usually answers', async function (ap, member) {
  await this.open(member);
  wholeNetworkAnswers(this);
  await this.runCheck();
  assert.equal((await this.verdict()).title, 'Your connection works');
  assert.ok((await this.page.textContent('#checks')).includes(`${ap} · your AP router`));
});

Given('now it doesn\'t', function () {
  this.hosts.clear();
  this.serverReachable = false;
  this.internetReachable = false;
});

Given('the path of the member {string} goes through the AP router {string} and the router {string}', async function (member, ap, router) {
  await this.open(member);
  wholeNetworkAnswers(this);
  await this.runCheck();
  const checks = await this.page.textContent('#checks');
  assert.ok(checks.includes(`${ap} · your AP router`), checks);
  assert.ok(checks.includes(`${router} · backbone`), checks);
});

Given('the AP router {string} answers, but the router {string} doesn\'t', function (ap, router) {
  this.hosts.clear();
  this.hosts.set(ap, 'answers');
  this.serverReachable = false;
  this.internetReachable = false;
  assert.notEqual(this.hosts.get(router), 'answers');
});

When('the member runs a check', async function () {
  await this.runCheck();
});

Then('they see that their connection works', async function () {
  assert.equal((await this.verdict()).title, 'Your connection works');
});

Then('they see that the problem is between them and the AP router {string}', async function (ap) {
  const v = await this.verdict();
  assert.equal(v.title, 'Your AP router does not answer');
  assert.ok(v.text.includes(ap), v.text);
});

Then('they are advised to restart the router and check the antenna\'s power', async function () {
  const { advice } = await this.verdict();
  assert.ok(advice.includes('Restart your router'), advice);
  assert.ok(advice.includes('antenna power'), advice);
});

Then('they see that the problem is in the hkfree.org backbone beyond the router {string}', async function (ap) {
  const v = await this.verdict();
  assert.equal(v.title, 'Outage in the hkfree.org backbone');
  assert.ok(v.text.includes(`Reachable up to ${ap}`), v.text);
});

Then('they are advised not to restart anything', async function () {
  const { advice } = await this.verdict();
  assert.ok(advice.includes('Do not restart anything'), advice);
});
