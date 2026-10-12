// Measures this device's connection against a LibreSpeed backend
// (speedtest.hkfree.org), only when the member asks: response time and
// jitter, then download and upload with a few parallel streams for a few
// seconds each, within a data cap. Nothing but test data is sent.

const PINGS = 10;
const STREAMS = 4;
const PHASE_MS = 8000;
const DOWN_CAP = 250e6;
const UP_CAP = 100e6;
const CHUNK_MB = 25;
const UP_BLOB = 1 << 20;

/** A LibreSpeed backend URL for `file`; `?cors` makes it send CORS headers. */
function url(base, file, extra = '') {
  return `${base}${file}?cors=true&r=${Math.random().toString(36).slice(2)}${extra}`;
}

async function timed(fn, ms) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), ms);
  try {
    return await fn(ctrl.signal);
  } finally {
    clearTimeout(timer);
  }
}

/** Response time (median) and jitter (mean difference of successive pings), in ms. */
async function ping(base) {
  const times = [];
  for (let i = 0; i < PINGS; i++) {
    const t0 = performance.now();
    const res = await timed((signal) => fetch(url(base, 'empty.php'), { cache: 'no-store', signal }), 3000);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    await res.arrayBuffer();
    times.push(performance.now() - t0);
  }
  const sorted = [...times].sort((a, b) => a - b);
  const jitter = times.slice(1).reduce((s, t, i) => s + Math.abs(t - times[i]), 0) / (times.length - 1);
  return { ping: sorted[Math.floor(sorted.length / 2)], jitter };
}

/**
 * Runs `transfer(signal, add)` on STREAMS parallel loops until PHASE_MS has
 * passed or `cap` bytes moved; returns Mbit/s over the time it took.
 */
async function phase(transfer, cap) {
  const ctrl = new AbortController();
  let bytes = 0;
  const add = (n) => {
    bytes += n;
    if (bytes >= cap) ctrl.abort();
  };
  const t0 = performance.now();
  const timer = setTimeout(() => ctrl.abort(), PHASE_MS);
  const loop = async () => {
    while (!ctrl.signal.aborted) {
      try {
        await transfer(ctrl.signal, add);
      } catch (e) {
        if (!ctrl.signal.aborted) throw e;
      }
    }
  };
  try {
    await Promise.all(Array.from({ length: STREAMS }, loop));
  } finally {
    clearTimeout(timer);
    ctrl.abort();
  }
  const secs = (performance.now() - t0) / 1000;
  if (!bytes || secs <= 0) throw new Error('no data moved');
  return (bytes * 8) / secs / 1e6;
}

async function download(base) {
  return phase(async (signal, add) => {
    const res = await fetch(url(base, 'garbage.php', `&ckSize=${CHUNK_MB}`), { cache: 'no-store', signal });
    if (!res.ok || !res.body) throw new Error(`HTTP ${res.status}`);
    const reader = res.body.getReader();
    for (;;) {
      const { done, value } = await reader.read();
      if (done) return;
      add(value.length);
    }
  }, DOWN_CAP);
}

/** Random data, so nothing on the way can compress it. */
function randomBlob() {
  const buf = new Uint8Array(UP_BLOB);
  for (let i = 0; i < buf.length; i += 65536) crypto.getRandomValues(buf.subarray(i, i + 65536));
  return new Blob([buf]);
}

async function upload(base) {
  const blob = randomBlob();
  return phase(async (signal, add) => {
    const res = await fetch(url(base, 'empty.php'), { method: 'POST', body: blob, cache: 'no-store', signal });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    await res.arrayBuffer();
    add(blob.size);
  }, UP_CAP);
}

/**
 * The whole measurement; `onPhase('ping' | 'download' | 'upload')` reports
 * progress. Resolves to { ping, jitter, down, up } (ms, ms, Mbit/s, Mbit/s).
 */
export async function measure(base, onPhase) {
  onPhase('ping');
  const p = await ping(base);
  onPhase('download');
  const down = await download(base);
  onPhase('upload');
  const up = await upload(base);
  return { ...p, down, up };
}
