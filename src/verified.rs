//! Core logic proven with Verus (ADR 0003). Contracts and proofs are erased
//! in a normal build; `cargo verus verify` checks them.

#[allow(unused_imports)]
use vstd::prelude::*;

verus! {

/// The netmask of a prefix length: the top `len` bits set (lengths over 32 count as 32).
pub open spec fn mask_spec(len: u8) -> u32 {
    if len == 0 { 0u32 } else if len >= 32 { 0xffff_ffffu32 } else { (0xffff_ffffu32 << ((32 - len) as u32)) as u32 }
}

/// The netmask of a prefix length.
pub fn mask(len: u8) -> (m: u32)
    ensures
        m == mask_spec(len),
        len == 0 ==> m == 0,
        len >= 32 ==> m == 0xffff_ffffu32,
{
    if len == 0 {
        0
    } else if len >= 32 {
        0xffff_ffff
    } else {
        0xffff_ffffu32 << (32 - len as u32)
    }
}

/// Whether `ip` lies in the prefix `net`/`len`: they agree on the masked bits.
pub fn in_prefix(net: u32, len: u8, ip: u32) -> (b: bool)
    ensures
        b == ((ip & mask_spec(len)) == (net & mask_spec(len))),
        // A /0 holds every address; a /32 holds exactly one.
        len == 0 ==> b,
        len >= 32 ==> (b == (ip == net)),
{
    let m = mask(len);
    proof {
        if len == 0 {
            assert((ip & 0u32) == (net & 0u32)) by (bit_vector);
        }
        if len >= 32 {
            assert(((ip & 0xffff_ffffu32) == (net & 0xffff_ffffu32)) == (ip == net)) by (bit_vector);
        }
    }
    (ip & m) == (net & m)
}

/// A longer prefix inside a shorter one: an address in the longer one is in the shorter one too.
pub proof fn lemma_nested(net: u32, short: u8, long: u8, ip: u32)
    requires
        short <= long <= 32,
        (net & mask_spec(long)) == net,
        (ip & mask_spec(long)) == (net & mask_spec(long)),
    ensures
        (ip & mask_spec(short)) == (net & mask_spec(short)),
{
    let ms = mask_spec(short);
    let ml = mask_spec(long);
    // The shorter mask keeps a subset of the longer mask's bits.
    assert(ms & ml == ms) by {
        if short == 0 {
            assert(0u32 & ml == 0u32) by (bit_vector);
        } else if long >= 32 {
            assert(ms & 0xffff_ffffu32 == ms) by (bit_vector);
        } else {
            let a = (32 - short) as u32;
            let b = (32 - long) as u32;
            assert(b <= a && a <= 31 ==> ((0xffff_ffffu32 << a) & (0xffff_ffffu32 << b)) == (0xffff_ffffu32 << a)) by (bit_vector);
        }
    }
    assert((ip & ml) == (net & ml) && ms & ml == ms ==> (ip & ms) == (net & ms)) by (bit_vector);
}

/// A request header that may name the client, as plain Rust read it: which
/// kind, and the address it gives (for X-Forwarded-For: its last
/// comma-separated entry), if that parsed.
#[derive(Clone, Copy)]
pub enum HeaderKind {
    ForwardedFor,
    RealIp,
}

pub struct ClientHeader<T> {
    pub kind: HeaderKind,
    pub addr: Option<T>,
}

/// The address the last X-Forwarded-For header ends with (what the proxy appended).
pub open spec fn forwarded_last<T>(hs: Seq<ClientHeader<T>>) -> Option<T>
    decreases hs.len(),
{
    if hs.len() == 0 {
        None
    } else if hs.last().kind is ForwardedFor {
        hs.last().addr
    } else {
        forwarded_last(hs.drop_last())
    }
}

/// How many X-Real-IP headers there are.
pub open spec fn real_count<T>(hs: Seq<ClientHeader<T>>) -> nat
    decreases hs.len(),
{
    if hs.len() == 0 {
        0
    } else {
        real_count(hs.drop_last()) + if hs.last().kind is RealIp { 1nat } else { 0nat }
    }
}

/// The address of the last X-Real-IP header (the only one, when it counts).
pub open spec fn real_last<T>(hs: Seq<ClientHeader<T>>) -> Option<T>
    decreases hs.len(),
{
    if hs.len() == 0 {
        None
    } else if hs.last().kind is RealIp {
        hs.last().addr
    } else {
        real_last(hs.drop_last())
    }
}

/// Who a request comes from. Headers count only from a trusted proxy: with
/// `forwarded_for`, the address the last X-Forwarded-For header ends with (the
/// entry the proxy appended); otherwise exactly one X-Real-IP, or nobody
/// (never the proxy itself).
pub fn client_from_headers<T: Copy>(peer: T, trusted: bool, forwarded_for: bool, headers: &Vec<ClientHeader<T>>) -> (who: Option<T>)
    ensures
        !trusted ==> who == Some(peer),
        trusted && forwarded_for ==> who == forwarded_last(headers@),
        trusted && !forwarded_for ==> who == (if real_count(headers@) == 1 { real_last(headers@) } else { None::<T> }),
{
    if !trusted {
        return Some(peer);
    }
    let mut last_fwd: Option<T> = None;
    let mut reals: usize = 0;
    let mut last_real: Option<T> = None;
    let mut i: usize = 0;
    while i < headers.len()
        invariant
            i <= headers.len(),
            last_fwd == forwarded_last(headers@.subrange(0, i as int)),
            reals as nat == real_count(headers@.subrange(0, i as int)),
            reals <= i,
            last_real == real_last(headers@.subrange(0, i as int)),
        decreases headers.len() - i,
    {
        proof {
            // The prefix grows by one header; the specs look at its last one.
            assert(headers@.subrange(0, (i + 1) as int).drop_last() =~= headers@.subrange(0, i as int));
        }
        match headers[i].kind {
            HeaderKind::ForwardedFor => {
                last_fwd = headers[i].addr;
            },
            HeaderKind::RealIp => {
                reals = reals + 1;
                last_real = headers[i].addr;
            },
        }
        i = i + 1;
    }
    proof {
        assert(headers@.subrange(0, headers@.len() as int) =~= headers@);
    }
    if forwarded_for {
        last_fwd
    } else if reals == 1 {
        last_real
    } else {
        None
    }
}

/// A byte allowed in a served file name: an ASCII letter or digit, `.`, `_` or `-`.
pub open spec fn name_byte(b: u8) -> bool {
    (48 <= b <= 57) || (65 <= b <= 90) || (97 <= b <= 122) || b == 46 || b == 95 || b == 45
}

/// A path segment that may be served: 1 to 128 allowed bytes, not starting with
/// a dot (no `.`, `..` or hidden files).
pub open spec fn segment_spec(s: Seq<u8>) -> bool {
    1 <= s.len() <= 128 && s[0] != 46 && forall|i: int| 0 <= i < s.len() ==> name_byte(#[trigger] s[i])
}

pub fn segment_ok(s: &[u8]) -> (ok: bool)
    ensures
        ok == segment_spec(s@),
{
    if s.len() == 0 || s.len() > 128 || s[0] == 46 {
        return false;
    }
    let mut i: usize = 0;
    while i < s.len()
        invariant
            i <= s.len(),
            forall|j: int| 0 <= j < i ==> name_byte(#[trigger] s@[j]),
        decreases s.len() - i,
    {
        let b = s[i];
        if !((48 <= b && b <= 57) || (65 <= b && b <= 90) || (97 <= b && b <= 122) || b == 46 || b == 95 || b == 45) {
            return false;
        }
        i += 1;
    }
    true
}

} // verus!
