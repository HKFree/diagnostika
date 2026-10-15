// The check's verdict from what answered: which advice the member gets, so
// which one is right matters. verdictKey and the hop helpers are proven with
// LemmaScript; decide() adds the details (router names, the finding).
export function verdictKey(f) {
    //@ verify
    //@ requires 0 <= f.targets && -1 <= f.firstLost && f.firstLost < f.targets
    //@ ensures !f.online ==> \result === 'offline'
    //@ ensures \result === 'ok' ==> f.online && !f.localFailure && f.serverOk && f.dnsOk
    //@ ensures (f.serverOk || f.ipOk || f.dnsOk) && f.online && !f.localFailure ==> \result !== 'apDown' && \result !== 'backbone' && \result !== 'noPath' && \result !== 'generic' && \result !== 'known'
    //@ ensures \result === 'apDown' ==> f.online && !f.serverOk && !f.ipOk && !f.dnsOk && !f.critical && f.firstLost === 0
    //@ ensures \result === 'backbone' ==> !f.serverOk && !f.ipOk && !f.dnsOk && !f.critical && f.firstLost > 0
    //@ ensures \result === 'split' ==> f.serverOk && !f.ipOk && !f.dnsOk && f.split
    if (!f.online)
        return 'offline';
    if (f.localFailure)
        return 'localFailure';
    if (f.serverOk) {
        if (f.dnsOk)
            return 'ok';
        if (f.ipOk)
            return 'dns';
        if (f.split)
            return 'split';
        return 'inetDown';
    }
    if (f.ipOk || f.dnsOk)
        return 'serverOnly';
    if (f.targets === 0)
        return 'noPath';
    if (f.critical)
        return 'known';
    if (f.firstLost === 0)
        return 'apDown';
    if (f.firstLost > 0)
        return 'backbone';
    return 'generic';
}
/** The first of the probed routers (`silent[i]`: did router i stop answering?) that went silent, or -1. */
export function firstSilent(silent) {
    //@ verify
    //@ ensures -1 <= \result && \result < silent.length
    //@ ensures \result >= 0 ==> silent[\result]
    //@ ensures forall(k, 0 <= k && k < silent.length && (\result === -1 || k < \result) ==> !silent[k])
    let i = 0;
    while (i < silent.length) {
        //@ invariant 0 <= i && i <= silent.length
        //@ invariant forall(k, 0 <= k && k < i ==> !silent[k])
        //@ decreases silent.length - i
        if (silent[i])
            return i;
        i = i + 1;
    }
    return -1;
}
/** The last router before position `before` that answered, or -1. */
export function lastAnsweredBefore(answered, before) {
    //@ verify
    //@ requires 0 <= before && before <= answered.length
    //@ ensures -1 <= \result && \result < before
    //@ ensures \result >= 0 ==> answered[\result]
    //@ ensures forall(k, \result < k && k < before ==> !answered[k])
    let i = before - 1;
    while (i >= 0) {
        //@ invariant -1 <= i && i < before
        //@ invariant forall(k, i < k && k < before ==> !answered[k])
        //@ decreases i + 1
        if (answered[i])
            return i;
        i = i - 1;
    }
    return -1;
}
/** Of the probed routers (the hops that belong to a target), which went silent first, or -1. */
export function firstLostOf(r) {
    return firstSilent(r.targets.map((_, i) => Boolean(r.hops[i]) && r.hops[i].verdict === 'lost'));
}
/** The last router that answered before the first silent one (or of all, if none went silent). */
export function lastOkOf(r) {
    const lost = firstLostOf(r);
    const i = lastAnsweredBefore(r.targets.map((_, k) => Boolean(r.hops[k]) && r.hops[k].verdict === 'ok'), lost < 0 ? r.targets.length : lost);
    return i < 0 ? null : r.targets[i];
}
export function decide(r) {
    const d = (level, key, params = {}) => ({ level, key, params });
    const findings = (r.analysis && r.analysis.findings) || [];
    const split = findings.find((f) => f.key === 'split');
    // The saved map may already explain the outage.
    const critical = findings.find((f) => f.sev === 'critical');
    const firstLost = firstLostOf(r);
    const key = verdictKey({
        online: r.device.state !== 'bad',
        localFailure: Boolean(r.localFailure),
        serverOk: Boolean(r.serverOk),
        dnsOk: r.inetDns.state === 'ok',
        ipOk: r.inetIp.state === 'ok',
        split: Boolean(split),
        targets: r.targets.length,
        critical: Boolean(critical),
        firstLost,
    });
    const router = (x) => x && { ip: x.ip, name: x.name };
    switch (key) {
        case 'ok': return d('ok', key);
        case 'dns':
        case 'inetDown':
        case 'serverOnly': return d('warn', key);
        case 'split': return d('bad', key, { count: split?.params.count, links: split?.params.links });
        case 'known': return d('bad', key, { snapshotAt: r.snapshotAt, finding: { key: critical?.key, params: critical?.params } });
        case 'apDown': return d('bad', key, { ip: r.targets[0].ip, name: r.targets[0].name });
        case 'backbone': return d('bad', key, { bad: router(r.targets[firstLost]), lastOk: router(lastOkOf(r)) });
        default: return d('bad', key);
    }
}
