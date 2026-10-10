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

/// Addresses per request to the pinger.
const BATCH: usize = 5000;
/// More than this is a mistake, not a network.
const MAX_IPS: usize = 50_000;

/// Health of `ips`, asked in batches of `BATCH`.
pub fn ping_ips(cfg: &SojkaConfig, ips: &[Ipv4Addr]) -> Result<BTreeMap<Ipv4Addr, PingResult>> {
    if ips.len() > MAX_IPS {
        bail!("refusing to query more than {MAX_IPS} IPs");
    }
    let agent = http::agent(Duration::from_secs(cfg.timeout_secs));
    let mut out = BTreeMap::new();
    for batch in ips.chunks(BATCH) {
        out.extend(ping_batch(cfg, &agent, batch)?);
    }
    Ok(out)
}

fn ping_batch(cfg: &SojkaConfig, agent: &ureq::Agent, ips: &[Ipv4Addr]) -> Result<BTreeMap<Ipv4Addr, PingResult>> {
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
        .filter_map(|(k, v)| Some((k.parse().ok()?, serde_json::from_value::<PingResult>(v).ok()?)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::now;

    /// A pinger that answers every request with each asked IP alive, and counts requests.
    fn fake_pinger() -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::io::{BufRead, BufReader, Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/pinger/", listener.local_addr().unwrap());
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let seen = std::sync::Arc::clone(&count);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut r = BufReader::new(stream);
                let mut len = 0;
                loop {
                    let mut line = String::new();
                    if r.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                        len = v.trim().parse().unwrap_or(0);
                    }
                }
                let mut body = vec![0; len];
                r.read_exact(&mut body).unwrap();
                seen.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let ips: Vec<String> = serde_json::from_slice(&body).unwrap();
                let out: serde_json::Map<String, serde_json::Value> = ips
                    .into_iter()
                    .map(|ip| (ip, serde_json::json!({"alive": true, "loss": 0.0, "rtt": 0.01, "time_lastpong": now()})))
                    .collect();
                let text = serde_json::Value::Object(out).to_string();
                let mut s = r.into_inner();
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
            }
        });
        (url, count)
    }

    #[test]
    fn asks_in_batches() {
        let (url, count) = fake_pinger();
        let cfg = SojkaConfig { url, timeout_secs: 5, lookback_secs: 7 * 24 * 3600 };
        let ips: Vec<Ipv4Addr> = (0..(BATCH as u32 + 1)).map(|i| Ipv4Addr::from(0x0a00_0000 + i)).collect();
        let got = ping_ips(&cfg, &ips).unwrap();
        assert_eq!(got.len(), ips.len());
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
        assert!(ping_ips(&cfg, &[]).unwrap().is_empty());
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2, "nothing to ask, no request");
    }

    #[test]
    fn classifies() {
        let s = Duration::from_secs(7 * 24 * 3600);
        let p = |alive, loss, rtt, ago: i64| PingResult { alive, loss, rtt, time_lastpong: now() - ago };
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
