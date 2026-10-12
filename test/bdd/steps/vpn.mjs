import { Given, Then } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

Given('the member\'s device is connected through the hkfree.org VPN with the address {string}', async function (address) {
  await this.open(address);
  this.wholeNetworkAnswers();
});

Given('the member checked their connection at home with the address {string}', async function (address) {
  await this.healthyCheck(address);
  assert.equal(await this.badge(), 'hkfree.org');
});

Given('then connected through the hkfree.org VPN with the address {string}', async function (address) {
  await this.context.setExtraHTTPHeaders({ 'X-Real-IP': address });
  this.wholeNetworkAnswers();
  await this.runCheck();
  assert.equal(await this.badge(), 'VPN hkfree.org');
});

Then('they see that they are connected through the hkfree.org VPN, so the check doesn\'t test their home connection', async function () {
  assert.equal(await this.badge(), 'VPN hkfree.org');
  const w = await this.page.evaluate(() => {
    const el = document.querySelector('#net-warning');
    return { visible: Boolean(el.getBoundingClientRect().height), cls: el.className, text: el.textContent };
  });
  assert.ok(w.visible && w.cls.includes('vpn'), JSON.stringify(w));
  assert.ok(w.text.startsWith('You are connected through the hkfree.org VPN') && w.text.includes('not your home connection'), w.text);
});

Then('they see the VPN server instead of an AP router on their path', async function () {
  const nodes = await this.page.$$eval('#path-map [data-node]', (n) => n.map((x) => ({ role: x.dataset.node, text: x.textContent })));
  assert.ok(nodes.some((n) => n.role === 'vpn' && n.text.includes('hkfree.org VPN server')), JSON.stringify(nodes));
  assert.ok(!nodes.some((n) => n.role === 'ap' || n.role === 'border'), JSON.stringify(nodes));
  assert.ok(!nodes.some((n) => n.text.includes('member records')), 'no AP name line for the VPN server');
});

Then('they see the path of the address {string}', async function (address) {
  const title = await this.page.textContent('#path-title');
  assert.ok(title.includes(address), title);
});
