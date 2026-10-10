import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

/** The monitoring line about `ip` in advanced diagnostics. */
async function reportAbout(page, ip) {
  const lines = await page.$$eval('#path-map [data-node="ap"] .monitoring', (t) => t.map((x) => x.textContent.trim()));
  return { line: lines.find((l) => l.includes(`(${ip})`)), lines };
}

function apDevices(map, ap) {
  return (map.aps && map.aps[ap.id] && map.aps[ap.id].devices) || [];
}

Given('network monitoring reports packet loss at the AP router {string} of the member {string}', async function (router, member) {
  const map = await this.monitoringReports(router, 'lossy');
  assert.equal(map.routers[router].sojka, 'Lossy');
  await this.open(member);
  this.wholeNetworkAnswers();
});

Given('network monitoring reports that the device {string} with the address {string} of the AP of the member {string} doesn\'t answer', async function (label, ip, member) {
  const map = await this.monitoringReports(ip, 'dead');
  const ap = map.routers['10.107.10.149'].ap;
  assert.ok(apDevices(map, ap).some(([a, l, h]) => a === ip && l === label && h === 'Dead'), JSON.stringify(map.aps));
  await this.open(member);
  this.wholeNetworkAnswers();
});

Given('network monitoring sees all devices of the AP of the member {string} working', async function (member) {
  const map = await this.monitoringReports('10.107.10.149', 'ok');
  const devices = apDevices(map, map.routers['10.107.10.149'].ap);
  assert.ok(devices.length > 0 && devices.every(([, , h]) => h === 'Ok'), JSON.stringify(devices));
  assert.ok(!devices.some(([a]) => a === '10.107.10.152'), 'decommissioned devices are left out');
  await this.open(member);
  this.wholeNetworkAnswers();
});

Then('they see that network monitoring reports packet loss at {string}', async function (ip) {
  const { line, lines } = await reportAbout(this.page, ip);
  assert.ok(line && line.endsWith('has packet loss'), JSON.stringify(lines));
});

Then('they see that network monitoring reports that {string} {string} doesn\'t answer', async function (label, ip) {
  const { line, lines } = await reportAbout(this.page, ip);
  assert.equal(line, `monitoring: ${label} (${ip}) doesn't answer`, JSON.stringify(lines));
});

Then('they see that network monitoring sees no problem at their AP', async function () {
  const text = await this.page.textContent('#path-map [data-node="ap"] .monitoring');
  assert.ok(text.includes('Network monitoring sees no problem at your AP'), text);
});
