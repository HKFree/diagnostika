import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

Given('the AP router {string} of the member {string} doesn\'t answer', async function (ap, address) {
  await this.healthyCheck(address);
  this.networkFails();
});

Given('the check of the member {string} found that the AP router {string} doesn\'t answer', async function (address, ap) {
  await this.healthyCheck(address);
  this.networkFails();
  await this.runCheck();
  assert.equal((await this.verdict()).title, 'Your AP router does not answer');
});

Given('the member {string} opened advanced diagnostics at their last check', async function (address) {
  await this.healthyCheck(address);
  await this.page.click('#advanced > summary');
  assert.equal(await this.page.isVisible('#checks'), true);
});

When('the member opens advanced diagnostics', async function () {
  await this.page.click('#advanced > summary');
});

When('the member runs a check again', async function () {
  await this.runCheck();
});

Then('they see that their connection doesn\'t work, that the problem is at the AP router {string} and what to do', async function (ap) {
  const v = await this.verdict();
  assert.equal(v.title, 'Your AP router does not answer');
  assert.ok(v.text.includes(ap), v.text);
  assert.equal(await this.page.isVisible('#advice'), true);
  assert.ok(v.advice.includes('What to do'), v.advice);
});

Then('they see the results of the individual tests and the path only once they open advanced diagnostics', async function () {
  assert.equal(await this.page.isVisible('#checks'), false);
  assert.equal(await this.page.isVisible('#advanced > summary'), true);
  await this.page.click('#advanced > summary');
  assert.equal(await this.page.isVisible('#checks'), true);
});

Then('they see the results of the individual tests', async function () {
  assert.equal(await this.page.isVisible('#checks'), true);
  assert.ok((await this.page.textContent('#checks')).includes('hkfree.org server'));
});

Then('they see their path through the AP router {string}', async function (ap) {
  assert.ok((await this.page.textContent('#checks')).includes(`${ap} · your AP router`));
});

Then('the individual tests and the path are folded away again', async function () {
  assert.equal(await this.page.isVisible('#checks'), false);
});
