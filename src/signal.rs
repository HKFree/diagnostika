//! The radio signal of a member's antenna, from Wewimo (the registration
//! tables of the AP's routers, read through UserDB).
//!
//! Only the requesting member's own stations ever leave this module: a
//! station is theirs when its last IP is the member's address, or an address
//! the member records give to the same member (an antenna that routes).

use crate::userdb::Device;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

/// One antenna as an AP's router sees it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Station {
    /// The AP's router that sees the antenna.
    pub router: Ipv4Addr,
    pub interface: String,
    pub frequency: String,
    /// How strongly the antenna receives the AP (dBm). From the antenna's own
    /// report when the router has it, else how strongly the AP receives it.
    pub dbm: i32,
    /// How strongly the AP receives the antenna (dBm).
    pub ap_dbm: Option<i32>,
    pub snr: Option<i32>,
    pub tx_ccq: Option<u32>,
    pub rx_ccq: Option<u32>,
    pub rx_mbps: Option<f64>,
    pub tx_mbps: Option<f64>,
    pub uptime: String,
    #[serde(skip)]
    pub last_ip: Option<Ipv4Addr>,
}

fn int(v: &Value) -> Option<i32> {
    match v {
        Value::Number(n) => n.as_f64().map(|f| f.round() as i32),
        Value::String(s) => s.trim().parse::<f64>().ok().map(|f| f.round() as i32),
        _ => None,
    }
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn text(v: &Value, max: usize) -> String {
    v.as_str().unwrap_or_default().chars().filter(|c| !c.is_control()).take(max).collect()
}

/// All stations in a Wewimo answer (`{"devices": [{ip, interfaces: {name: {stations: [...]}}}]}`).
/// Routers that couldn't be read and stations without a signal are skipped.
pub fn stations(wewimo: &Value) -> Vec<Station> {
    let mut out = Vec::new();
    let devices: Vec<&Value> = match &wewimo["devices"] {
        Value::Array(a) => a.iter().collect(),
        Value::Object(m) => m.values().collect(),
        _ => Vec::new(),
    };
    for d in devices {
        let Some(router) = d["ip"].as_str().and_then(|s| s.parse().ok()) else { continue };
        let Value::Object(ifaces) = &d["interfaces"] else { continue };
        for (name, iface) in ifaces {
            let Value::Array(list) = &iface["stations"] else { continue };
            for s in list {
                let ap_dbm = int(&s["x-signal-strength"]);
                let Some(dbm) = int(&s["x-tx-signal-strength"]).or(ap_dbm) else { continue };
                out.push(Station {
                    router,
                    interface: name.chars().take(40).collect(),
                    frequency: text(&iface["frequency"], 20),
                    dbm,
                    ap_dbm,
                    snr: int(&s["x-signal-to-noise"]),
                    tx_ccq: int(&s["tx-ccq"]).and_then(|v| u32::try_from(v).ok()),
                    rx_ccq: int(&s["rx-ccq"]).and_then(|v| u32::try_from(v).ok()),
                    rx_mbps: num(&s["x-rx-rate"]),
                    tx_mbps: num(&s["x-tx-rate"]),
                    uptime: text(&s["uptime"], 30),
                    last_ip: s["last-ip"].as_str().and_then(|v| v.parse().ok()),
                });
            }
        }
    }
    out
}

/// The stations that belong to the member at `who`.
pub fn mine(all: &[Station], who: Ipv4Addr, members: &BTreeMap<Ipv4Addr, Device>) -> Vec<Station> {
    let user = members.get(&who).and_then(|d| d.user_id);
    all.iter()
        .filter(|s| match s.last_ip {
            Some(ip) if ip == who => true,
            Some(ip) => user.is_some() && members.get(&ip).and_then(|d| d.user_id) == user,
            None => false,
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(user: Option<u32>) -> Device {
        Device { ip: Ipv4Addr::UNSPECIFIED, hostname: String::new(), description: String::new(), ap_id: Some(10), user_id: user }
    }

    #[test]
    fn reads_stations_and_keeps_only_the_members_own() {
        let w: Value = serde_json::from_str(r#"{"devices":[
            {"ip":"10.107.10.149","error":"","interfaces":{"wlan1":{"frequency":"5500","stations":[
                {"last-ip":"10.107.10.146","x-tx-signal-strength":"-67","x-signal-strength":"-69","x-signal-to-noise":"28","tx-ccq":"88","x-rx-rate":300,"uptime":"2d"},
                {"last-ip":"10.107.10.160","x-tx-signal-strength":"","x-signal-strength":"-71"},
                {"last-ip":"10.107.10.140","x-tx-signal-strength":"-55"},
                {"last-ip":"10.107.10.141"}
            ]}}},
            {"ip":"10.107.10.151","error":"Error connecting to RouterOS","interfaces":[]}
        ]}"#).unwrap();
        let all = stations(&w);
        assert_eq!(all.len(), 3, "no signal, no station");
        assert_eq!(all[1].dbm, -71, "the AP's view when the antenna doesn't report");
        let mut members = BTreeMap::new();
        members.insert("10.107.10.146".parse().unwrap(), dev(Some(500)));
        members.insert("10.107.10.160".parse().unwrap(), dev(Some(500)));
        members.insert("10.107.10.140".parse().unwrap(), dev(Some(501)));
        let m = mine(&all, "10.107.10.146".parse().unwrap(), &members);
        assert_eq!(m.iter().map(|s| s.dbm).collect::<Vec<_>>(), vec![-67, -71], "own address and the same member's antenna");
        assert!(mine(&all, "10.107.10.199".parse().unwrap(), &members).is_empty());
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("10.107.10.160") && !json.contains("last_ip"));
    }
}
