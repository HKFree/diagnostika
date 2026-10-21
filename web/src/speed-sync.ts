// Measured speeds on the device: the last one (it goes with an outage report),
// the queue waiting to be sent, sending it without the member's address, and
// what members of an AP typically measure. Storage and fetch, so not proven
// itself; the decisions come from the proven speeds.ts.

import { fitsOutage, serverDone, unsentPositions } from './speeds.js';

export interface Speed {
  measured_at: number;
  down_mbps: number;
  up_mbps: number;
  ping_ms: number;
}

/** A speed waiting to be sent: its AP router, never who measured it. */
export interface QueuedSpeed extends Speed {
  id: string;
  ap: string;
}

export interface Storage {
  get(key: string, fallback: unknown): unknown;
  set(key: string, value: unknown): void;
}

export type Fetch = (url: string, init: RequestInit, ms: number) => Promise<{ ok: boolean; res?: Response }>;

const MAX_QUEUED = 20;

function isSpeed(s: unknown): s is Speed {
  if (!s || typeof s !== 'object') return false;
  const x = s as Record<string, unknown>;
  return [x.measured_at, x.down_mbps, x.up_mbps, x.ping_ms].every((v) => typeof v === 'number' && Number.isFinite(v) && v >= 0);
}

function queued(storage: Storage): QueuedSpeed[] {
  const q = storage.get('pendingSpeeds', []);
  return Array.isArray(q) ? q.filter((s): s is QueuedSpeed => isSpeed(s) && typeof (s as QueuedSpeed).id === 'string' && typeof (s as QueuedSpeed).ap === 'string') : [];
}

/**
 * Remembers a measured speed. With `ap` (measured at home over hkfree.org,
 * where it says something about the AP) it is also queued for sending.
 */
export function rememberSpeed(storage: Storage, speed: Speed, ap: string | null, id: string): boolean {
  if (!isSpeed(speed)) return false;
  const s: Speed = { measured_at: speed.measured_at, down_mbps: speed.down_mbps, up_mbps: speed.up_mbps, ping_ms: speed.ping_ms };
  storage.set('lastSpeed', s);
  if (ap) storage.set('pendingSpeeds', [...queued(storage), { id, ap, ...s }].slice(-MAX_QUEUED));
  return true;
}

/** The speed to send with an outage detected at `detectedAt`, if one was measured in the 24 hours before. */
export function speedForOutage(storage: Storage, detectedAt: number): Speed | null {
  const s = storage.get('lastSpeed', null);
  if (!isSpeed(s) || !fitsOutage(s.measured_at, detectedAt)) return null;
  return { measured_at: s.measured_at, down_mbps: s.down_mbps, up_mbps: s.up_mbps, ping_ms: s.ping_ms };
}

/** Sends the queued speeds; what the server is done with leaves the queue. */
export async function sendSpeeds(storage: Storage, fetchFn: Fetch): Promise<void> {
  const q = queued(storage);
  if (!q.length) return;
  const batch = q.slice(0, MAX_QUEUED);
  const r = await fetchFn('/api/speeds', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(batch), cache: 'no-store' }, 8000);
  if (!r.ok || !r.res || !serverDone(r.res.status)) return; // kept for the next attempt
  // Read again: a speed measured while sending stays queued.
  const now = queued(storage);
  storage.set('pendingSpeeds', unsentPositions(now.map((s) => s.id), new Set(batch.map((s) => s.id))).map((i) => now[i]));
}

/** What members of the AP router `ap` typically measure this week, if enough did. */
export async function apTypicalSpeed(fetchFn: Fetch, ap: string): Promise<{ down: number; count: number } | null> {
  const r = await fetchFn('/api/v1/speeds', { cache: 'no-store' }, 6000);
  if (!r.ok || !r.res || !r.res.ok) return null;
  try {
    const body = await r.res.json();
    const list: unknown[] = Array.isArray(body.speeds) ? body.speeds : [];
    const g = list.find((x): x is { ap_router: string; down_typical: number; count: number } => Boolean(x) && (x as { ap_router?: unknown }).ap_router === ap);
    return g && Number.isFinite(g.down_typical) && Number.isInteger(g.count) ? { down: g.down_typical, count: g.count } : null;
  } catch (_) {
    return null;
  }
}
