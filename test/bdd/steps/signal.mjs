import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { radio } from '../support/system.mjs';

const fresh = () => new Promise((r) => setTimeout(r, 1100)); // the server keeps an AP's answer for a second in tests

Given('the antenna of the member {string} receives the AP with a signal of {int} dBm', async function (member, dbm) {
  radio.set(member, dbm);
  this.member = member;
  await fresh();
});

Given('the AP\'s routers don\'t see the antenna of the member {string}', async function (member) {
  radio.set('10.107.10.146', -60); // someone else's antenna is there
  this.member = member;
  await fresh();
});

When('the member asks for the radio signal of their antenna', async function () {
  await this.healthyCheck(this.member);
  await this.askForSignal();
});

When('the device with the address {string} asks the hkfree.org server for its antenna\'s signal', async function (address) {
  this.answer = await this.signalFor(address);
});

Then('they see the signal {int} dBm, rated as good', async function (dbm) {
  assert.equal(await this.signalText(), `Your antenna receives the AP at ${dbm} dBm: good`);
});

Then('they see the signal {int} dBm, rated as weak', async function (dbm) {
  const text = await this.signalText();
  assert.ok(text.startsWith(`Your antenna receives the AP at ${dbm} dBm: weak`), text);
});

Then('they are advised to check that the antenna points at the AP and nothing blocks the view', async function () {
  const text = await this.signalText();
  assert.ok(text.includes('Check that the antenna points at the AP and nothing blocks its view'), text);
});

Then('they see that their antenna wasn\'t found at the AP', async function () {
  assert.equal(await this.signalText(), 'Your antenna wasn\'t found at the AP');
});

Then('it doesn\'t get the signal of the antenna of {string}', function (member) {
  assert.ok(!this.answer.includes('-67') && !this.answer.includes(member), this.answer);
});
