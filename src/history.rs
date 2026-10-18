//! Network monitoring's measured series, read from its Grafana (InfluxDB
//! behind it): a router's response time and packet loss, and an address's
//! traffic. The server asks for one address at a time, only when someone opens
//! a chart, and keeps the answer `cache_secs`; the app never calls Grafana.
//!
//! Only a validated IPv4 address and fixed numbers go into a query, so nothing
//! from a request can change what is asked.

use crate::config::HistoryConfig;
use crate::http;
use crate::verified::slot;
use anyhow::{bail, Context, Result};
use serde_json::{json, Value};
use std::net::Ipv4Addr;
use std::time::Duration;

/// How far back a chart reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Range {
    Day,
    Week,
}

impl Range {
    /// From a `range=` query parameter; a day unless it says "week".
    pub fn from_query(url: &str) -> Range {
        let asked = url.split_once('?').and_then(|(_, q)| q.split('&').find_map(|kv| kv.strip_prefix("range=")));
        if asked == Some("week") { Range::Week } else { Range::Day }
    }

    pub fn name(self) -> &'static str {
        match self {
            Range::Day => "day",
            Range::Week => "week",
        }
    }

    /// Seconds per point: 10 minutes over a day, an hour over a week.
    fn step(self) -> i64 {
        match self {
            Range::Day => 600,
            Range::Week => 3600,
        }
    }

    fn span(self) -> i64 {
        match self {
            Range::Day => 86_400,
            Range::Week => 7 * 86_400,
        }
    }
}

/// What is measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Kind {
    /// Response time (median, ms) and packet loss (0 to 1).
    Ping,
    /// Download and upload, bits per second.
    Traffic,
}

/// Two series of one address in equal steps; a missing measurement is `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub ip: Ipv4Addr,
    pub kind: Kind,
    pub range: Range,
    /// Unix time of the first point.
    pub from: i64,
    pub step: i64,
    pub a: Vec<Option<f64>>,
    pub b: Vec<Option<f64>>,
}

impl Series {
    pub fn to_json(&self) -> Value {
        let (a, b) = match self.kind {
            Kind::Ping => ("rtt_ms", "loss"),
            Kind::Traffic => ("down_bps", "up_bps"),
        };
        json!({ "ip": self.ip, "range": self.range.name(), "from": self.from, "step": self.step, a: self.a, b: self.b })
    }
}

/// The InfluxQL query for one address's buckets in [from, to).
fn query(kind: Kind, ip: Ipv4Addr, from: i64, to: i64, step: i64) -> String {
    let time = format!("time >= {from}s AND time < {to}s GROUP BY time({step}s)");
    match kind {
        Kind::Ping => format!(r#"SELECT mean("pct50") * 1000 AS "a", mean("loss") AS "b" FROM "ping" WHERE "ip" = '{ip}' AND {time} fill(none)"#),
        // As the pinger dashboard does: no record means no traffic.
        Kind::Traffic => format!(r#"SELECT mean("down") * 8 AS "a", mean("up") * 8 AS "b" FROM "traffic" WHERE "collector" = 'oko' AND "ip" = '{ip}' AND {time} fill(0)"#),
    }
}

/// The largest believable value of each series; anything else is dropped.
fn bounds(kind: Kind) -> (f64, f64) {
    match kind {
        Kind::Ping => (60_000.0, 1.0),
        Kind::Traffic => (1e11, 1e11),
    }
}

/// The frames of a Grafana data source answer, put into `n` buckets from `from`.
pub fn parse(v: &Value, kind: Kind, from: i64, step: i64, n: usize) -> Result<(Vec<Option<f64>>, Vec<Option<f64>>)> {
    let r = &v["results"]["A"];
    if let Some(e) = r["error"].as_str() {
        bail!("monitoring's history: {}", crate::report::sanitize(e));
    }
    let (max_a, max_b) = bounds(kind);
    let mut a = vec![None; n];
    let mut b = vec![None; n];
    for f in r["frames"].as_array().into_iter().flatten() {
        let name = f["schema"]["name"].as_str().unwrap_or_default();
        let (out, max) = match name.rsplit('.').next() {
            Some("a") => (&mut a, max_a),
            Some("b") => (&mut b, max_b),
            _ => continue,
        };
        let vals = &f["data"]["values"];
        let times = vals[0].as_array().map(Vec::as_slice).unwrap_or_default();
        let ys = vals[1].as_array().map(Vec::as_slice).unwrap_or_default();
        for (t, y) in times.iter().zip(ys) {
            let (Some(t), Some(y)) = (t.as_i64(), y.as_f64()) else { continue };
            if !(y.is_finite() && (0.0..=max).contains(&y)) {
                continue;
            }
            if let Some(i) = slot(t.div_euclid(1000), from, step, n) {
                out[i] = Some(y);
            }
        }
    }
    Ok((a, b))
}

/// One address's series over `range`, as of `now`.
pub fn fetch(cfg: &HistoryConfig, kind: Kind, ip: Ipv4Addr, range: Range, now: i64) -> Result<Series> {
    let datasource = match kind {
        Kind::Ping => cfg.ping_datasource.as_str(),
        Kind::Traffic => cfg.traffic_datasource.as_deref().context("no traffic data source configured")?,
    };
    let step = range.step();
    // Whole buckets, the last one still filling.
    let to = (now.div_euclid(step) + 1) * step;
    let from = to - range.span();
    let n = (range.span() / step) as usize;
    let body = json!({
        "from": (from * 1000).to_string(),
        "to": (to * 1000).to_string(),
        "queries": [{
            "refId": "A",
            "datasource": { "uid": datasource },
            "rawQuery": true,
            "resultFormat": "time_series",
            "query": query(kind, ip, from, to, step),
        }],
    });
    let agent = http::agent(Duration::from_secs(cfg.timeout_secs));
    let resp = agent
        .post(&format!("{}/api/ds/query", cfg.url))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .context("monitoring's history: request failed")?;
    let text = http::read_limited(resp, 2 * 1024 * 1024)?;
    let v: Value = serde_json::from_str(&text).context("monitoring's history: invalid JSON")?;
    let (a, b) = parse(&v, kind, from, step, n)?;
    Ok(Series { ip, kind, range, from, step, a, b })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_holds_only_the_address_and_numbers() {
        let q = query(Kind::Ping, "10.107.76.1".parse().unwrap(), 1000, 2200, 600);
        assert_eq!(q, r#"SELECT mean("pct50") * 1000 AS "a", mean("loss") AS "b" FROM "ping" WHERE "ip" = '10.107.76.1' AND time >= 1000s AND time < 2200s GROUP BY time(600s) fill(none)"#);
        assert!(query(Kind::Traffic, "10.107.10.146".parse().unwrap(), 0, 600, 600).contains(r#""collector" = 'oko' AND "ip" = '10.107.10.146'"#));
    }

    #[test]
    fn frames_land_in_their_buckets_and_nonsense_is_dropped() {
        let v: Value = serde_json::from_str(r#"{"results":{"A":{"status":200,"frames":[
            {"schema":{"name":"ping.a"},"data":{"values":[[600000,1200000,1800000,99000000],[2.5,"x",3.0,1.0]]}},
            {"schema":{"name":"ping.b"},"data":{"values":[[600000,1200000,1800000],[0.0,1.5,0.3]]}},
            {"schema":{"name":"ping.c"},"data":{"values":[[600000],[1.0]]}}]}}}"#).unwrap();
        let (a, b) = parse(&v, Kind::Ping, 600, 600, 3).unwrap();
        assert_eq!(a, vec![Some(2.5), None, Some(3.0)], "a string and a time past the end are dropped");
        assert_eq!(b, vec![Some(0.0), None, Some(0.3)], "a loss over 100 % is dropped");
    }

    #[test]
    fn an_error_from_grafana_is_an_error() {
        let v: Value = serde_json::from_str(r#"{"results":{"A":{"error":"timeout\u001b[31m"}}}"#).unwrap();
        let e = parse(&v, Kind::Ping, 0, 600, 3).unwrap_err().to_string();
        assert!(e.contains("timeout") && !e.contains('\u{1b}'), "{e}");
    }

    #[test]
    fn range_from_the_query() {
        assert_eq!(Range::from_query("/api/traffic"), Range::Day);
        assert_eq!(Range::from_query("/api/traffic?range=week"), Range::Week);
        assert_eq!(Range::from_query("/api/v1/history?ip=1.2.3.4&range=year"), Range::Day);
    }
}
