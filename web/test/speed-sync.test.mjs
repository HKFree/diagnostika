// The device side of measured speeds: queueing, sending, and what goes with an outage report.
import test from 'node:test';
import assert from 'node:assert/strict';
import { rememberSpeed, sendSpeeds, speedForOutage } from '../speed-sync.js';

function memory() {
  const m = new Map();
  return { get: (k, f) => (m.has(k) ? m.get(k) : f), set: (k, v) => m.set(k, v), m };
}
const speed = (at, down = 80) => ({ measured_at: at, down_mbps: down, up_mbps: 8, ping_ms: 5 });
const answer = (status) => async () => ({ ok: true, res: { status } });

test('a speed measured at home is queued, one measured elsewhere only remembered', () => {
  const s = memory();
  assert.equal(rememberSpeed(s, speed(1000), '10.107.10.149', 'speed-a'), true);
  assert.equal(rememberSpeed(s, speed(1100), null, 'speed-b'), true);
  assert.deepEqual(s.get('pendingSpeeds', []).map((x) => x.id), ['speed-a']);
  assert.equal(s.get('lastSpeed', null).measured_at, 1100);
  assert.equal(rememberSpeed(s, { ...speed(1200), down_mbps: NaN }, '10.107.10.149', 'speed-c'), false, 'nonsense isn\'t kept');
});

test('what the server is done with leaves the queue; a failure keeps it', async () => {
  const s = memory();
  rememberSpeed(s, speed(1000), '10.107.10.149', 'speed-a');
  await sendSpeeds(s, answer(503));
  assert.equal(s.get('pendingSpeeds', []).length, 1, 'kept after a server error');
  await sendSpeeds(s, async () => ({ ok: false }));
  assert.equal(s.get('pendingSpeeds', []).length, 1, 'kept without an answer');
  // A speed measured while sending stays queued.
  await sendSpeeds(s, async () => {
    rememberSpeed(s, speed(1100), '10.107.10.149', 'speed-b');
    return { ok: true, res: { status: 200 } };
  });
  assert.deepEqual(s.get('pendingSpeeds', []).map((x) => x.id), ['speed-b']);
  await sendSpeeds(s, answer(400));
  assert.deepEqual(s.get('pendingSpeeds', []), [], 'refused for good: dropped');
});

test('only a speed from the 24 hours before an outage goes with its report', () => {
  const s = memory();
  rememberSpeed(s, speed(100_000), null, 'speed-a');
  assert.equal(speedForOutage(s, 100_000 + 3600).down_mbps, 80);
  assert.equal(speedForOutage(s, 100_000 + 2 * 86_400), null, 'too old');
  assert.equal(speedForOutage(s, 99_000), null, 'measured after the outage began');
});
