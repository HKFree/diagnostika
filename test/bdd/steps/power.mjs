import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { power } from '../support/system.mjs';

// UserDB ids: Podůlšany is AP 10 in the test network; Lipky is an AP elsewhere.
const AP = { Podůlšany: { id: 10, area: 'Podůlšany' }, Lipky: { id: 999, area: 'Lipky' } };
const apOf = (name) => ({ id: AP[name].id, name, areaId: 1, area: AP[name].area, matchedBy: 'ip' });
const poll = () => new Promise((r) => setTimeout(r, 2200)); // the server reads the power monitor every second in tests

Given('the power monitor reports that the backup power of the AP {string} has been on battery for {int} minutes', async function (ap, minutes) {
  power.sources = [{
    id: 84, name: `ups-${ap}`, ap: apOf(ap), address: null, online: true, mainsOk: false, onBattery: true,
    batteryPercent: 73, outageSince: new Date(Date.now() - minutes * 60_000).toISOString(),
  }];
  await poll();
});

Given('ČEZ announced a power cut at the AP {string} for tomorrow from 7:30 to 16:30', async function (ap) {
  const day = new Date(Date.now() + 86_400_000);
  const at = (h, m) => new Date(Date.UTC(day.getUTCFullYear(), day.getUTCMonth(), day.getUTCDate(), h - 2, m)).toISOString();
  const d = `${day.getUTCDate()}.${day.getUTCMonth() + 1}.${day.getUTCFullYear()}`;
  power.cez = [{
    status: 'planned', isTest: false, from: at(7, 30), to: at(16, 30), dateText: `${d} 07:30 - 16:30`,
    street: 'Podůlšany', cityPart: 'Podůlšany', sources: [{ id: 84, name: `ups-${ap}`, ap: apOf(ap), address: null }], addresses: [],
  }];
  await poll();
});

Then('they see that their AP {string} runs on battery', async function (ap) {
  const items = await this.page.$$eval('#outages-power li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(ap) && t.includes('on battery') && t.includes('73 %')), JSON.stringify(items));
});

Then('they see the planned power cut at their AP {string} with its time', async function (ap) {
  const items = await this.page.$$eval('#outages-power li', (li) => li.map((x) => x.textContent));
  assert.ok(items.some((t) => t.includes(ap) && t.includes('07:30 - 16:30') && t.includes('planned')), JSON.stringify(items));
});
