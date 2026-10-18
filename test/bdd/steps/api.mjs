import { Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';

const dayAgo = () => Math.floor(Date.now() / 1000) - 86_400;

async function getJson(world, path) {
  const res = await fetch(new URL(path, world.serverUrl));
  return { status: res.status, type: res.headers.get('content-type') || '', body: await res.json() };
}

When('a program asks for the API catalog at {string}', async function (path) {
  this.catalog = await getJson(this, path);
});

Then('it learns where the API description is', async function () {
  assert.equal(this.catalog.status, 200);
  assert.ok(this.catalog.type.startsWith('application/linkset+json'), this.catalog.type);
  const desc = this.catalog.body.linkset[0]['service-desc'][0];
  this.openapi = await getJson(this, desc.href);
  assert.equal(this.openapi.status, 200);
  assert.ok(this.openapi.body.openapi.startsWith('3.'), 'an OpenAPI 3 description');
});

Then('the description covers the network map, the outages, the measured outages and the reported outages', function () {
  const paths = Object.keys(this.openapi.body.paths);
  for (const p of ['/api/v1/network', '/api/v1/outages', '/api/v1/measured', '/api/v1/reports']) assert.ok(paths.includes(p), JSON.stringify(paths));
});

When('a program asks the API for the measured outages of the last day', async function () {
  this.answer = await getJson(this, `/api/v1/measured?since=${dayAgo()}`);
});

Then('it gets that outage of {string} at the AP {string} with its start and end', function (label, ap) {
  const m = this.answer.body.measured.find((x) => x.label === label && x.ap === ap);
  assert.ok(m && m.started && m.ended && m.ended - m.started >= 15 * 60, JSON.stringify(this.answer.body));
});

When('a program asks the API for the reported outages of the last day', async function () {
  const res = await fetch(new URL(`/api/v1/reports?since=${dayAgo()}`, this.serverUrl));
  this.answerText = await res.text();
  this.answer = { status: res.status, body: JSON.parse(this.answerText) };
});

Then('it gets the outage of the router {string}', function (router) {
  assert.ok(this.answer.body.reports.some((r) => r.first_bad === router), this.answerText);
});

Then('not the address {string}', function (address) {
  assert.ok(!this.answerText.includes(address), this.answerText);
});

/** Asks `path` from a page on another site (another origin) and says whether the browser let it read the answer. */
async function fromAnotherSite(world, path) {
  const other = createServer((req, res) => res.writeHead(200, { 'Content-Type': 'text/html' }).end('<!doctype html><title>other</title>'));
  await new Promise((r) => other.listen(0, '127.0.0.1', r));
  try {
    await world.open('10.107.77.5');
    await world.page.goto(`http://127.0.0.1:${other.address().port}/`);
    return await world.page.evaluate(async (url) => {
      try {
        const res = await fetch(url);
        await res.text();
        return true;
      } catch (_) {
        return false;
      }
    }, new URL(path, world.serverUrl).href);
  } finally {
    other.close();
  }
}

When('a page on another site asks the API for the outages', async function () {
  this.readable = await fromAnotherSite(this, '/api/v1/outages');
});

When('a page on another site asks who its visitor is', async function () {
  this.readable = await fromAnotherSite(this, '/api/whoami');
});

Then('its browser lets it read the answer', function () {
  assert.equal(this.readable, true);
});

Then('its browser doesn\'t let it read the answer', function () {
  assert.equal(this.readable, false);
});
