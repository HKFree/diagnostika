//! Active probes from the machine hkdiag runs on (system `ping`, no shell).

use serde::Serialize;
use std::net::Ipv4Addr;
use std::process::{Command, Stdio};
use std::thread;

#[derive(Clone, Debug, Serialize)]
pub struct Probe {
    pub ip: Ipv4Addr,
    pub sent: u32,
    pub received: u32,
    pub avg_rtt_ms: Option<f64>,
    /// `ping` could not be executed at all.
    pub error: Option<String>,
}

impl Probe {
    pub fn reachable(&self) -> bool {
        self.received > 0
    }
}

const COUNT: u32 = 3;

pub fn ping(ip: Ipv4Addr) -> Probe {
    // `ip` is a parsed Ipv4Addr, so it cannot inject options or shell syntax.
    let out = Command::new("ping")
        .args(["-n", "-q", "-c", &COUNT.to_string(), "-i", "0.2", "-W", "1", "--"])
        .arg(ip.to_string())
        .stdin(Stdio::null())
        .output();
    match out {
        Err(e) => Probe { ip, sent: 0, received: 0, avg_rtt_ms: None, error: Some(e.to_string()) },
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            match parse_counts(&text) {
                Some((sent, received)) => Probe { ip, sent, received, avg_rtt_ms: parse_avg(&text), error: None },
                // No statistics: ping itself failed (e.g. no ICMP permission),
                // which says nothing about the target.
                None => {
                    let err = String::from_utf8_lossy(&o.stderr);
                    let msg = err.lines().next().unwrap_or("").trim();
                    let msg = if msg.is_empty() { format!("ping failed ({})", o.status) } else { msg.to_string() };
                    Probe { ip, sent: 0, received: 0, avg_rtt_ms: None, error: Some(msg) }
                }
            }
        }
    }
}

/// Pings all IPs in parallel (bounded batches) and returns results in input order.
pub fn ping_all(ips: &[Ipv4Addr]) -> Vec<Probe> {
    let mut out = Vec::with_capacity(ips.len());
    for chunk in ips.chunks(32) {
        let handles: Vec<_> = chunk.iter().map(|&ip| thread::spawn(move || ping(ip))).collect();
        for (h, &ip) in handles.into_iter().zip(chunk) {
            out.push(h.join().unwrap_or_else(|_| Probe {
                ip,
                sent: 0,
                received: 0,
                avg_rtt_ms: None,
                error: Some("probe thread panicked".into()),
            }));
        }
    }
    out
}

fn parse_counts(text: &str) -> Option<(u32, u32)> {
    // "3 packets transmitted, 3 received, 0% packet loss, time 402ms"
    let line = text.lines().find(|l| l.contains("packets transmitted"))?;
    let mut nums = line.split(',').map(|p| p.trim().split(' ').next().and_then(|n| n.parse().ok()));
    Some((nums.next()??, nums.next()??))
}

fn parse_avg(text: &str) -> Option<f64> {
    // "rtt min/avg/max/mdev = 0.040/0.052/0.061/0.008 ms"
    let line = text.lines().find(|l| l.contains("min/avg/max"))?;
    line.split('=').nth(1)?.trim().split('/').nth(1)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iputils_output() {
        let t = "PING 10.0.0.1 (10.0.0.1) 56(84) bytes of data.\n\n--- 10.0.0.1 ping statistics ---\n3 packets transmitted, 2 received, 33.3333% packet loss, time 402ms\nrtt min/avg/max/mdev = 0.040/0.052/0.061/0.008 ms\n";
        assert_eq!(parse_counts(t), Some((3, 2)));
        assert_eq!(parse_avg(t), Some(0.052));
        let t2 = "3 packets transmitted, 0 received, +3 errors, 100% packet loss, time 2003ms\n";
        assert_eq!(parse_counts(t2), Some((3, 0)));
        assert_eq!(parse_avg(t2), None);
    }
}
