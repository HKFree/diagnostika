//! Client for the Sojka pinger (`?rq=pingjson`), the same source UserDB uses
//! for its AP status page.

use crate::config::SojkaConfig;
use crate::http;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::Duration;

/// Thresholds identical to UserDB `App\Model\Status`.
pub const LOSS_DEAD: f64 = 0.8;
pub const LOSS_WARN: f64 = 0.2;
pub const RTT_WARN_SECS: f64 = 0.2;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PingResult {
    #[serde(default)]
    pub rtt: f64,
    #[serde(default)]
    pub loss: f64,
    #[serde(default)]
    pub time_lastpong: i64,
    /// Sojka sends `true`/`false`, or `-1` for addresses it never pinged.
    #[serde(default, deserialize_with = "de_alive")]
    pub alive: bool,
}

fn de_alive<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<bool, D::Error> {
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::Bool(b) => b,
        serde_json::Value::Number(n) => n.as_f64().map_or(false, |v| v > 0.0),
        _ => false,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Health {
    Ok,
    Slow,
    Lossy,
    Dead,
    /// Sojka has not heard from this IP recently; the data says nothing about now.
    Stale,
}

impl PingResult {
    /// `lookback`: IPs not seen alive for longer than this are ignored as
    /// stale (UserDB uses 7 days), as they are usually decommissioned.
    pub fn health(&self, lookback: Duration) -> Health {
        if crate::now().saturating_sub(self.time_lastpong) > lookback.as_secs() as i64 {
            return Health::Stale;
        }
        if !self.alive || self.loss >= LOSS_DEAD {
            return Health::Dead;
        }
        if self.loss >= LOSS_WARN {
            return Health::Lossy;
        }
        if self.rtt >= RTT_WARN_SECS {
            return Health::Slow;
        }
        Health::Ok
    }
}

pub fn ping_ips(cfg: &SojkaConfig, ips: &[Ipv4Addr]) -> Result<BTreeMap<Ipv4Addr, PingResult>> {
    if ips.is_empty() {
        return Ok(BTreeMap::new());
    }
    if ips.len() > 5000 {
        bail!("refusing to query more than 5000 IPs");
    }
    let agent = http::agent(Duration::from_secs(cfg.timeout_secs));
    let body: Vec<String> = ips.iter().map(|i| i.to_string()).collect();
    let resp = agent
        .post(&cfg.url)
        .query("rq", "pingjson")
        .send_json(body)
        .context("Sojka pinger request failed")?;
    let text = http::read_limited(resp, 16 * 1024 * 1024)?;
    // Unknown IPs are simply missing; an empty answer can be `[]`.
    if text.trim() == "[]" {
        return Ok(BTreeMap::new());
    }
    let raw: BTreeMap<String, serde_json::Value> =
        serde_json::from_str(&text).context("unexpected Sojka pinger response")?;
    // One odd entry must not discard the whole answer.
    Ok(raw
        .into_iter()
        .filter_map(|(k, v)| {
            Some((
                k.parse().ok()?,
                serde_json::from_value::<PingResult>(v).ok()?,
            ))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::now;

    #[test]
    fn classifies() {
        let s = Duration::from_secs(7 * 24 * 3600);
        let p = |alive, loss, rtt, ago: i64| PingResult {
            alive,
            loss,
            rtt,
            time_lastpong: now() - ago,
        };
        assert_eq!(p(true, 0.0, 0.01, 5).health(s), Health::Ok);
        assert_eq!(p(true, 0.3, 0.01, 5).health(s), Health::Lossy);
        assert_eq!(p(true, 0.0, 0.5, 5).health(s), Health::Slow);
        assert_eq!(p(false, 1.0, 0.0, 120).health(s), Health::Dead);
        assert_eq!(p(false, 0.0, 0.0, 30 * 24 * 3600).health(s), Health::Stale);
    }

    #[test]
    fn never_pinged_sentinel() {
        let r: PingResult =
            serde_json::from_str(r#"{"rtt":-1,"loss":1,"time_lastpong":-1,"alive":-1}"#).unwrap();
        assert!(!r.alive);
        assert_eq!(r.health(Duration::from_secs(7 * 24 * 3600)), Health::Stale);
    }
}
