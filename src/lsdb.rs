//! An OSPF database in one form, whichever router it was read from (FRR's
//! JSON, RouterOS's REST API), so several can be merged into one map.
//!
//! Each record keeps its area, sequence number and age. Merging keeps the
//! newest copy of every record: during a split each side refreshes its own
//! records, while the other side's copies age, so the newest copies describe
//! both sides as they are now.

use std::collections::HashMap;
use std::net::Ipv4Addr;

pub const BACKBONE: Ipv4Addr = Ipv4Addr::UNSPECIFIED;

/// A link in a router record.
#[derive(Clone, Debug, PartialEq)]
pub enum Link {
    Stub { net: Ipv4Addr, mask: Ipv4Addr, metric: u32 },
    /// Into a transit network whose designated router has the address `dr`.
    Transit { dr: Ipv4Addr, iface: Option<Ipv4Addr>, metric: u32 },
    PointToPoint { neighbor: Ipv4Addr, iface: Option<Ipv4Addr>, metric: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct RouterLsa {
    pub area: Ipv4Addr,
    pub id: Ipv4Addr,
    pub seq: i32,
    pub age: u32,
    pub links: Vec<Link>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NetworkLsa {
    pub area: Ipv4Addr,
    /// The designated router's interface address.
    pub id: Ipv4Addr,
    pub seq: i32,
    pub age: u32,
    pub mask_len: u8,
    pub attached: Vec<Ipv4Addr>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExternalLsa {
    pub id: Ipv4Addr,
    pub originator: Ipv4Addr,
    pub seq: i32,
    pub age: u32,
    pub mask_len: u8,
    pub e1: bool,
    pub metric: u32,
    pub forward: Option<Ipv4Addr>,
}

/// A summary (inter-area prefix) record: an area's border router announcing
/// a prefix of another area.
#[derive(Clone, Debug, PartialEq)]
pub struct SummaryLsa {
    pub area: Ipv4Addr,
    pub id: Ipv4Addr,
    pub originator: Ipv4Addr,
    pub seq: i32,
    pub age: u32,
    pub mask_len: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lsdb {
    /// The router this database was read from.
    pub root: Option<Ipv4Addr>,
    pub routers: Vec<RouterLsa>,
    pub networks: Vec<NetworkLsa>,
    pub externals: Vec<ExternalLsa>,
    pub summaries: Vec<SummaryLsa>,
}

/// OSPF sequence numbers are signed: 0x80000001 is the oldest.
pub fn parse_seq(s: &str) -> i32 {
    let hex = s.trim().trim_start_matches("0x").trim_start_matches("0X");
    u32::from_str_radix(hex, 16).map(|v| v as i32).unwrap_or(i32::MIN)
}

/// Keeps, per key, the record with the higher sequence number (the first one on a tie).
fn merge_by<T, K: std::hash::Hash + Eq>(ours: &mut Vec<T>, theirs: Vec<T>, key: impl Fn(&T) -> K, seq: impl Fn(&T) -> i32) {
    let mut at: HashMap<K, usize> = ours.iter().enumerate().map(|(i, x)| (key(x), i)).collect();
    for x in theirs {
        match at.get(&key(&x)) {
            Some(&i) => {
                if seq(&x) > seq(&ours[i]) {
                    ours[i] = x;
                }
            }
            None => {
                at.insert(key(&x), ours.len());
                ours.push(x);
            }
        }
    }
}

impl Lsdb {
    /// Adds another router's view; the newest copy of each record wins.
    pub fn merge(&mut self, other: Lsdb) {
        if self.root.is_none() {
            self.root = other.root;
        }
        merge_by(&mut self.routers, other.routers, |r| (r.area, r.id), |r| r.seq);
        merge_by(&mut self.networks, other.networks, |n| (n.area, n.id), |n| n.seq);
        merge_by(&mut self.externals, other.externals, |e| (e.id, e.originator), |e| e.seq);
        merge_by(&mut self.summaries, other.summaries, |s| (s.area, s.id, s.originator), |s| s.seq);
    }

    pub fn is_empty(&self) -> bool {
        self.routers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> Ipv4Addr {
        s.parse().unwrap()
    }

    fn router(id: &str, seq: &str, metric: u32) -> RouterLsa {
        RouterLsa { area: BACKBONE, id: ip(id), seq: parse_seq(seq), age: 10, links: vec![Link::Stub { net: ip(id), mask: ip("255.255.255.255"), metric }] }
    }

    #[test]
    fn sequence_numbers_are_signed() {
        assert!(parse_seq("0x80000001") < parse_seq("0x80000002"));
        assert!(parse_seq("80000fff") < parse_seq("0x00000001"));
        assert_eq!(parse_seq("garbage"), i32::MIN);
    }

    #[test]
    fn merge_keeps_the_newest_copy_and_adds_the_rest() {
        let mut a = Lsdb { root: Some(ip("10.0.0.1")), routers: vec![router("10.0.0.1", "0x80000005", 1), router("10.0.0.2", "0x80000009", 1)], ..Default::default() };
        let b = Lsdb {
            root: Some(ip("10.0.0.9")),
            routers: vec![router("10.0.0.1", "0x80000006", 2), router("10.0.0.2", "0x80000008", 2), router("10.0.0.3", "0x80000001", 2)],
            ..Default::default()
        };
        a.merge(b);
        assert_eq!(a.root, Some(ip("10.0.0.1")), "the first source stays the root");
        let metric = |id: &str| match &a.routers.iter().find(|r| r.id == ip(id)).unwrap().links[0] {
            Link::Stub { metric, .. } => *metric,
            _ => 0,
        };
        assert_eq!((metric("10.0.0.1"), metric("10.0.0.2"), metric("10.0.0.3")), (2, 1, 2));
    }
}
