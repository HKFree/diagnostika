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

/// Who a request comes from. Headers count only from a trusted proxy: with
/// `forwarded_for`, the last `X-Forwarded-For` entry (the one the proxy
/// appended); otherwise exactly one `X-Real-IP`, or nobody (never the proxy).
pub fn choose_client<T: Copy>(
    peer: T,
    peer_trusted: bool,
    forwarded_for: bool,
    forwarded_last: Option<T>,
    real_ip_count: usize,
    real_ip: Option<T>,
) -> (who: Option<T>)
    ensures
        !peer_trusted ==> who == Some(peer),
        peer_trusted && forwarded_for ==> who == forwarded_last,
        peer_trusted && !forwarded_for ==> who == (if real_ip_count == 1 { real_ip } else { None::<T> }),
{
    if !peer_trusted {
        Some(peer)
    } else if forwarded_for {
        forwarded_last
    } else if real_ip_count == 1 {
        real_ip
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
