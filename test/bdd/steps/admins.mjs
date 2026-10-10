import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

const AP_ROUTER = { '10.107.10.146': '10.107.10.149', '10.107.77.5': '10.107.77.1' };

async function apOf(world, member) {
  const map = await world.networkMap();
  world.member = member;
  return (map.routers[AP_ROUTER[member]] || {}).ap;
}

Given('the AP of the member {string} is looked after by the area admin {string} with the e-mail {string}', async function (member, nick, email) {
  const ap = await apOf(this, member);
  assert.deepEqual(ap && ap.admins[0], { nick, role: 'SO' }, JSON.stringify(ap));
  assert.ok(!JSON.stringify(await this.networkMap()).includes(email), 'e-mails must not be in the shared map');
});

Given('by the deputy {string} with the e-mail {string}', async function (nick, email) {
  const ap = await apOf(this, this.member);
  assert.deepEqual(ap.admins[1], { nick, role: 'ZSO' }, JSON.stringify(ap));
  assert.ok(!JSON.stringify(await this.networkMap()).includes(email));
});

Given('the member has checked their connection while it worked', async function () {
  await this.healthyCheck(this.member);
  assert.equal((await this.verdict()).title, 'Your connection works');
});

Given('the hkfree.org server couldn\'t give the member\'s device the admin\'s e-mail', async function () {
  this.contactFails = true;
  await this.healthyCheck(this.member);
});

Given('the admin of the AP of the member {string} isn\'t known', async function (member) {
  const ap = await apOf(this, member);
  assert.ok(!ap || !ap.admins || !ap.admins.length, JSON.stringify(ap));
  await this.healthyCheck(member);
});

When('the check finds an outage in the backbone', async function () {
  assert.ok(this.page, 'the member has the app open');
  this.networkFails(AP_ROUTER[this.member]);
  await this.runCheck();
  assert.equal((await this.verdict()).title, 'Outage in the hkfree.org backbone');
});

When('the device with the address {string} asks the hkfree.org server for the contact of its AP admin', async function (address) {
  this.answer = await this.contactFor(address);
});

Then('it doesn\'t get the e-mail {string}', function (email) {
  assert.ok(!this.answer.includes(email), this.answer);
});

Then('the member sees the contact of the admin {string} with the e-mail {string}', async function (nick, email) {
  const { admins } = await this.contacts();
  assert.equal(admins[0] && admins[0].text, `Your AP admin: ${nick} · ${email}`, JSON.stringify(admins));
  assert.deepEqual(admins[0].mail, [email]);
});

Then('they see the contact of the deputy {string} with the e-mail {string}', async function (nick, email) {
  const { admins } = await this.contacts();
  assert.equal(admins[1] && admins[1].text, `Deputy admin: ${nick} · ${email}`, JSON.stringify(admins));
  assert.equal(admins.length, 2, 'only SO and ZSO are AP admins');
});

Then('the member sees the admin {string} without an e-mail', async function (nick) {
  const { admins } = await this.contacts();
  assert.equal(admins[0] && admins[0].text, `Your AP admin: ${nick}`, JSON.stringify(admins));
  assert.deepEqual(admins[0].mail, []);
});

Then(/^(?:the member sees|they see) the support contact "([^"]*)" and "([^"]*)"$/, async function (email, phone) {
  const { support } = await this.contacts();
  assert.ok(support.includes(email) && support.includes(phone), support);
});

Then('they see that the phone line is open on working days from 9 to 17', async function () {
  const { support } = await this.contacts();
  assert.ok(support.includes('working days 9–17'), support);
});

Then('they see that support can also be reached on the hkfree.org Slack in the channel {string}', async function (channel) {
  const { support } = await this.contacts();
  assert.ok(support.includes(`hkfree.org Slack ${channel}`), support);
});
