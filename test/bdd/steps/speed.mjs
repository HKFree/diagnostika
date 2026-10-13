import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

/** What the map's device node says about the speed. */
async function speedText(page) {
  return page.$$eval('#path-map [data-node="device"] .speed', (t) => t.map((x) => x.textContent.trim()).join(' | '));
}

Given('the speed test server answers', function () {
  this.speedtestUp = true;
});

Given('the speed test server doesn\'t answer', function () {
  this.speedtestUp = false;
});

When('the member measures the speed of their connection', async function () {
  this.wholeNetworkAnswers();
  await this.runCheck();
  await this.openAdvanced();
  await this.page.click('#speed-btn');
  await this.page.waitForFunction(() => {
    const b = document.querySelector('#speed-btn');
    return !b.disabled && document.querySelector('#path-map [data-node="device"] .speed');
  }, null, { timeout: 60_000 });
});

Then('they see the download and upload speed in Mbit\\/s and the response time in ms', async function () {
  const text = await speedText(this.page);
  assert.match(text, /↓ \d+(\.\d)? Mbit\/s · ↑ \d+(\.\d)? Mbit\/s/, text);
  assert.match(text, /response \d+ ms/, text);
});

Then('they see that the speed couldn\'t be measured', async function () {
  const text = await speedText(this.page);
  assert.ok(text.startsWith('The speed couldn\'t be measured'), text);
});

Then('they can\'t start a speed test', async function () {
  assert.equal(await this.page.isDisabled('#speed-btn'), true);
});

Then('they see that it measures only connections through hkfree.org', async function () {
  const note = await this.page.textContent('#speed-note');
  assert.ok(note.includes('only connections through hkfree.org'), note);
});
