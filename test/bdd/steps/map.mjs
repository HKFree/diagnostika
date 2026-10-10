import { Given, Then, When } from '@cucumber/cucumber';
import assert from 'node:assert/strict';

/** The map as the member sees it: nodes in order and the state of each stretch between them. */
async function readMap(page) {
  return page.evaluate(() => {
    const map = document.querySelector('#path-map');
    if (!map || !map.getBoundingClientRect().height) return null;
    return {
      nodes: [...map.querySelectorAll('[data-node]')].map((n) => ({ ip: n.dataset.ip || '', role: n.dataset.node, state: n.dataset.state })),
      segments: [...map.querySelectorAll('[data-segment]')].map((s) => s.dataset.segment),
    };
  });
}

Given('their whole path answers', function () {
  this.wholeNetworkAnswers();
});

When('the member runs a check and opens advanced diagnostics', async function () {
  await this.runCheck();
  await this.openAdvanced();
});

Then('they see a map from their device through the routers {string} and {string} to the exit {string}', async function (r1, r2, exit) {
  const map = await readMap(this.page);
  assert.ok(map, 'no map visible');
  assert.deepEqual(map.nodes.map((n) => n.role), ['device', 'ap', 'hop', 'exit', 'internet']);
  assert.deepEqual(map.nodes.slice(1, 3).map((n) => n.ip), [r1, r2]);
  assert.equal(map.nodes[3].ip, exit);
});

Then('they see traffic flowing along the whole map to the internet', async function () {
  const map = await readMap(this.page);
  assert.ok(map.segments.length > 0 && map.segments.every((s) => s === 'flowing'), JSON.stringify(map.segments));
  assert.equal(map.nodes.at(-1).state, 'ok');
});

Then('they see the traffic on the map stop at the router {string}, marked as not answering', async function (router) {
  const map = await readMap(this.page);
  assert.ok(map, 'no map visible');
  const at = map.nodes.findIndex((n) => n.ip === router);
  assert.ok(at > 0, JSON.stringify(map.nodes));
  assert.equal(map.nodes[at].state, 'down');
  assert.ok(map.segments.slice(0, at - 1).every((s) => s === 'flowing'), JSON.stringify(map.segments));
  assert.equal(map.segments[at - 1], 'broken');
  assert.ok(map.segments.slice(at).every((s) => s === 'idle'), JSON.stringify(map.segments));
});
