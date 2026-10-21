// What happens to a measured speed: whether it goes with an outage report,
// whether the server is done with what the app sent, and what is still to be
// sent. Pure logic with contracts checked by LemmaScript (lsc); speed-sync.ts
// does the sending and the storage.

/** A speed goes with an outage report if it was measured in the 24 hours before the outage. */
export function fitsOutage(measuredAt: number, detectedAt: number): boolean {
  //@ verify
  //@ ensures \result === (measuredAt <= detectedAt && detectedAt - measuredAt <= 86400)
  return measuredAt <= detectedAt && detectedAt - measuredAt <= 86400;
}

/**
 * Whether the server is done with what the app sent: it stored it (200), or
 * will never take it (4xx). Anything else (no answer, 5xx) is tried again.
 */
export function serverDone(status: number): boolean {
  //@ verify
  //@ ensures \result === (status === 200 || (400 <= status && status < 500))
  //@ ensures status >= 500 ==> !\result
  //@ ensures status === 0 ==> !\result
  return status === 200 || (status >= 400 && status < 500);
}

/** Positions of the queued items not among those sent, in their original order. */
export function unsentPositions(ids: string[], sent: Set<string>): number[] {
  //@ verify
  //@ ensures forall(k, 0 <= k && k < \result.length ==> 0 <= \result[k] && \result[k] < ids.length)
  //@ ensures forall(k, 0 <= k && k < \result.length ==> !sent.has(ids[\result[k]]))
  //@ ensures forall(k, 0 < k && k < \result.length ==> \result[k - 1] < \result[k])
  //@ ensures forall(j, 0 <= j && j < ids.length && !sent.has(ids[j]) ==> exists(k, 0 <= k && k < \result.length && \result[k] === j))
  const out: number[] = [];
  let i = 0;
  while (i < ids.length) {
    //@ invariant 0 <= i && i <= ids.length
    //@ invariant forall(k, 0 <= k && k < out.length ==> 0 <= out[k] && out[k] < i)
    //@ invariant forall(k, 0 <= k && k < out.length ==> !sent.has(ids[out[k]]))
    //@ invariant forall(k, 0 < k && k < out.length ==> out[k - 1] < out[k])
    //@ invariant forall(j, 0 <= j && j < i && !sent.has(ids[j]) ==> exists(k, 0 <= k && k < out.length && out[k] === j))
    //@ decreases ids.length - i
    if (!sent.has(ids[i])) out.push(i);
    i = i + 1;
  }
  return out;
}
