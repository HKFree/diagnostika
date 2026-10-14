// The check's verdict from what answered: which advice the member gets, so
// which one is right matters. Proven with LemmaScript; the app adds the
// details (router names, the finding) to the message.
/**
 * `targets`: routers on the member's path that were probed; `firstLost`: the
 * position of the first one that stopped answering, or -1.
 */
export function verdictKey(online, localFailure, serverOk, dnsOk, ipOk, split, targets, critical, firstLost) {
    //@ verify
    //@ requires 0 <= targets && -1 <= firstLost && firstLost < targets
    //@ ensures !online ==> \result === 'offline'
    //@ ensures \result === 'ok' ==> online && !localFailure && serverOk && dnsOk
    //@ ensures (serverOk || ipOk || dnsOk) && online && !localFailure ==> \result !== 'apDown' && \result !== 'backbone' && \result !== 'noPath' && \result !== 'generic' && \result !== 'known'
    //@ ensures \result === 'apDown' ==> online && !serverOk && !ipOk && !dnsOk && !critical && firstLost === 0
    //@ ensures \result === 'backbone' ==> !serverOk && !ipOk && !dnsOk && !critical && firstLost > 0
    //@ ensures \result === 'split' ==> serverOk && !ipOk && !dnsOk && split
    if (!online)
        return 'offline';
    if (localFailure)
        return 'localFailure';
    if (serverOk) {
        if (dnsOk)
            return 'ok';
        if (ipOk)
            return 'dns';
        if (split)
            return 'split';
        return 'inetDown';
    }
    if (ipOk || dnsOk)
        return 'serverOnly';
    if (targets === 0)
        return 'noPath';
    if (critical)
        return 'known';
    if (firstLost === 0)
        return 'apDown';
    if (firstLost > 0)
        return 'backbone';
    return 'generic';
}
