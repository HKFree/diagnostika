// Which outages concern a member: those at their own AP, or at a router on
// one of the paths their traffic can take to the internet. Pure logic with
// contracts checked by LemmaScript (lsc); the app supplies the member's AP
// names and the addresses on their paths.
/** An outage belongs to an AP (by its name in the member records) and a device address. */
export function isRelevant(ap, ip, myAps, myIps) {
    //@ verify
    //@ ensures \result === (myAps.has(ap) || myIps.has(ip))
    return myAps.has(ap) || myIps.has(ip);
}
/** The app shows what is going on now and what ended within `window` seconds. */
export function isRecent(end, ongoing, now, window) {
    //@ verify
    //@ ensures \result === (ongoing || now - end <= window)
    return ongoing || now - end <= window;
}
/** Outages up to `min` seconds are ignored; an ongoing one counts up to `now`. */
export function lastedLongEnough(start, end, ongoing, now, min) {
    //@ verify
    //@ ensures \result === ((ongoing && now - start > min) || (!ongoing && end - start > min))
    if (ongoing)
        return now - start > min;
    return end - start > min;
}
/** Positions of the relevant outages, in their original order. */
export function relevantPositions(aps, ips, myAps, myIps) {
    //@ verify
    //@ requires aps.length === ips.length
    //@ ensures forall(k, 0 <= k && k < \result.length ==> 0 <= \result[k] && \result[k] < aps.length)
    //@ ensures forall(k, 0 <= k && k < \result.length ==> isRelevant(aps[\result[k]], ips[\result[k]], myAps, myIps))
    //@ ensures forall(k, 0 < k && k < \result.length ==> \result[k - 1] < \result[k])
    //@ ensures forall(j, 0 <= j && j < aps.length && isRelevant(aps[j], ips[j], myAps, myIps) ==> exists(k, 0 <= k && k < \result.length && \result[k] === j))
    const out = [];
    let i = 0;
    while (i < aps.length) {
        //@ invariant 0 <= i && i <= aps.length
        //@ invariant forall(k, 0 <= k && k < out.length ==> 0 <= out[k] && out[k] < i)
        //@ invariant forall(k, 0 <= k && k < out.length ==> isRelevant(aps[out[k]], ips[out[k]], myAps, myIps))
        //@ invariant forall(k, 0 < k && k < out.length ==> out[k - 1] < out[k])
        //@ invariant forall(j, 0 <= j && j < i && isRelevant(aps[j], ips[j], myAps, myIps) ==> exists(k, 0 <= k && k < out.length && out[k] === j))
        //@ decreases aps.length - i
        if (isRelevant(aps[i], ips[i], myAps, myIps))
            out.push(i);
        i = i + 1;
    }
    return out;
}
/** Routers on the cheapest path from `src` to each internet exit it can reach, `src` included. */
export function pathRouters(routers, src, exits) {
    const dist = new Map([[src, 0]]);
    const prev = new Map();
    const done = new Set();
    for (;;) {
        let cur = null;
        for (const [id, d] of dist)
            if (!done.has(id) && (cur === null || d < dist.get(cur)))
                cur = id;
        if (cur === null)
            break;
        done.add(cur);
        const r = routers.get(cur);
        for (const l of r ? r.links : []) {
            const d = dist.get(cur) + l.cost;
            if (!dist.has(l.to) || d < dist.get(l.to)) {
                dist.set(l.to, d);
                prev.set(l.to, cur);
            }
        }
    }
    const out = new Set(dist.has(src) ? [src] : []);
    for (const e of exits) {
        let cur = dist.has(e) ? e : undefined;
        while (cur !== undefined && !out.has(cur)) {
            out.add(cur);
            cur = prev.get(cur);
        }
    }
    return out;
}
export function concernOf(routers, exits, apRouter, apDevices) {
    const aps = new Set();
    const ips = new Set();
    if (!apRouter || !routers.has(apRouter))
        return { aps, ips };
    for (const id of pathRouters(routers, apRouter, exits)) {
        ips.add(id);
        for (const a of routers.get(id).ifaces)
            ips.add(a);
    }
    for (const d of apDevices)
        ips.add(d);
    const ap = routers.get(apRouter).ap;
    if (ap) {
        // UserDB's status names an AP "area - AP" when the area has several APs.
        aps.add(ap.name);
        aps.add(`${ap.area} - ${ap.name}`);
    }
    return { aps, ips };
}
