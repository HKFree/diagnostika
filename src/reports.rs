//! Outage reports sent by the PWA once a member's connection is back.
//!
//! Stored in SQLite. Everything from the client is untrusted: strict
//! validation (allowlisted verdicts, IPv4 only, sane timestamps, bounded
//! sizes), per-client rate limits and a cap on the table size.
//!
//! The public view (`public_recent`) never includes member addresses.

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Verdicts that mean "the problem was in the hkfree.org network", not on the
/// member's own network or beyond the internet exit.
pub const REPORTABLE: &[&str] = &["apDown", "backbone", "known", "generic", "split"];

pub const MAX_BATCH: usize = 20;
pub const MAX_BODY: usize = 32 * 1024;
const MAX_PATH: usize = 32;
const MAX_PER_CLIENT_PER_HOUR: i64 = 30;
const MAX_ROWS: i64 = 200_000;
const KEEP_DAYS: i64 = 180;
const MAX_AGE_SECS: i64 = 30 * 24 * 3600;
const CLOCK_SKEW_SECS: i64 = 300;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportIn {
    pub id: String,
    pub detected_at: i64,
    pub resolved_at: i64,
    pub verdict: String,
    /// The member's address: IPv4, or IPv6 when the device reached the server over IPv6.
    #[serde(default)]
    pub my_ip: Option<IpAddr>,
    /// AP router of the member (first hop of the path).
    #[serde(default)]
    pub ap: Option<Ipv4Addr>,
    /// First router on the path that stopped answering.
    #[serde(default)]
    pub first_bad: Option<Ipv4Addr>,
    /// Last router that still answered.
    #[serde(default)]
    pub last_ok: Option<Ipv4Addr>,
    #[serde(default)]
    pub path: Vec<Ipv4Addr>,
    #[serde(default)]
    pub snapshot_at: i64,
    /// The speed the member measured in the 24 hours before the outage, if any.
    #[serde(default)]
    pub speed: Option<SpeedResult>,
}

/// One speed test's result.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpeedResult {
    pub measured_at: i64,
    pub down_mbps: f64,
    pub up_mbps: f64,
    pub ping_ms: f64,
}

impl SpeedResult {
    fn validate(&self, now: i64) -> Result<()> {
        let rate = |v: f64| v.is_finite() && (0.0..=MAX_MBPS).contains(&v);
        if !rate(self.down_mbps) || !rate(self.up_mbps) || !(self.ping_ms.is_finite() && (0.0..=60_000.0).contains(&self.ping_ms)) {
            bail!("speed out of range");
        }
        if self.measured_at < now - MAX_AGE_SECS || self.measured_at > now + CLOCK_SKEW_SECS {
            bail!("measured_at out of range");
        }
        Ok(())
    }
}

/// A speed test's result as the app sends it on its own: at which AP router,
/// never by whom.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedIn {
    pub id: String,
    /// The AP router of the member's path (its first hop).
    pub ap: Ipv4Addr,
    pub measured_at: i64,
    pub down_mbps: f64,
    pub up_mbps: f64,
    pub ping_ms: f64,
}

impl SpeedIn {
    fn result(&self) -> SpeedResult {
        SpeedResult { measured_at: self.measured_at, down_mbps: self.down_mbps, up_mbps: self.up_mbps, ping_ms: self.ping_ms }
    }
}

/// Download over this is a mistake, not a measurement.
const MAX_MBPS: f64 = 100_000.0;
/// A typical speed is shown once at least this many were measured, so no
/// single member's line stands out.
pub const MIN_SPEEDS_SHOWN: usize = 3;

impl ReportIn {
    pub fn validate(&self, now: i64) -> Result<()> {
        let id_ok = (8..=64).contains(&self.id.len()) && self.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
        if !id_ok {
            bail!("invalid id");
        }
        if !REPORTABLE.contains(&self.verdict.as_str()) {
            bail!("verdict not reportable");
        }
        if self.detected_at < now - MAX_AGE_SECS || self.detected_at > now + CLOCK_SKEW_SECS {
            bail!("detected_at out of range");
        }
        if self.resolved_at < self.detected_at || self.resolved_at > now + CLOCK_SKEW_SECS {
            bail!("resolved_at out of range");
        }
        if self.path.len() > MAX_PATH {
            bail!("path too long");
        }
        if self.snapshot_at < 0 || self.snapshot_at > now + CLOCK_SKEW_SECS {
            bail!("snapshot_at out of range");
        }
        if let Some(s) = &self.speed {
            s.validate(now)?;
            if s.measured_at < self.detected_at - 24 * 3600 || s.measured_at > self.resolved_at + CLOCK_SKEW_SECS {
                bail!("speed not from around the outage");
            }
        }
        Ok(())
    }
}

/// What became of a batch of reports.
#[derive(Debug, PartialEq)]
pub struct Outcome {
    pub stored: usize,
    /// Already stored earlier (the app sent them again).
    pub duplicates: usize,
    /// Why each refused report was refused.
    pub rejected: Vec<String>,
}

/// The middle value (the mean of the two middle ones for an even count).
fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Speeds at one AP router, for everyone: no member addresses, no single measurement.
#[derive(Debug, Serialize, PartialEq)]
pub struct SpeedGroup {
    pub ap_router: String,
    /// The AP, if the member records name it.
    pub ap: Option<ApName>,
    pub count: usize,
    pub down_typical: f64,
    pub down_lowest: f64,
    pub up_typical: f64,
    pub ping_typical: f64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ApName {
    pub id: u32,
    pub name: String,
}

/// What everyone may see: no member addresses.
#[derive(Debug, Serialize)]
pub struct PublicReport {
    pub detected_at: i64,
    pub resolved_at: i64,
    pub verdict: String,
    pub ap: Option<String>,
    pub first_bad: Option<String>,
    pub last_ok: Option<String>,
}

pub struct Store {
    conn: Connection,
}

/// An outage episode network monitoring measured at an AP's device (public:
/// infrastructure only).
#[derive(Debug, Serialize, PartialEq)]
pub struct Measured {
    pub ap: String,
    pub ip: String,
    pub label: String,
    pub kind: u32,
    pub started: i64,
    pub ended: Option<i64>,
}

pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("hkdiag/reports.sqlite"))
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        if let Some(dir) = path.parent() {
            std::fs::DirBuilder::new().recursive(true).mode(0o700).create(dir)?;
        }
        let conn = Connection::open(path).with_context(|| format!("opening {}", path.display()))?;
        // Member addresses are personal data: owner-only file.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS reports (
                 id          TEXT PRIMARY KEY,
                 received_at INTEGER NOT NULL,
                 client_ip   TEXT,
                 detected_at INTEGER NOT NULL,
                 resolved_at INTEGER NOT NULL,
                 verdict     TEXT NOT NULL,
                 my_ip       TEXT,
                 ap          TEXT,
                 first_bad   TEXT,
                 last_ok     TEXT,
                 path        TEXT NOT NULL,
                 snapshot_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS reports_received ON reports(received_at);
             CREATE INDEX IF NOT EXISTS reports_client ON reports(client_ip, received_at);
             CREATE TABLE IF NOT EXISTS measured (
                 ap      TEXT NOT NULL,
                 ip      TEXT NOT NULL,
                 label   TEXT NOT NULL,
                 kind    INTEGER NOT NULL,
                 started INTEGER NOT NULL,
                 ended   INTEGER
             );
             CREATE INDEX IF NOT EXISTS measured_open ON measured(ended);
             CREATE INDEX IF NOT EXISTS measured_started ON measured(started);
             CREATE TABLE IF NOT EXISTS speeds (
                 id          TEXT PRIMARY KEY,
                 received_at INTEGER NOT NULL,
                 measured_at INTEGER NOT NULL,
                 ap_router   TEXT NOT NULL,
                 ap_id       INTEGER,
                 ap_name     TEXT,
                 down        REAL NOT NULL,
                 up          REAL NOT NULL,
                 ping        REAL NOT NULL
             );
             CREATE INDEX IF NOT EXISTS speeds_measured ON speeds(measured_at);",
        )?;
        // Columns added later to a database made before them.
        let have: Vec<String> = conn.prepare("SELECT name FROM pragma_table_info('reports')")?.query_map([], |r| r.get(0))?.collect::<std::result::Result<_, _>>()?;
        for (col, ty) in [("speed_at", "INTEGER"), ("speed_down", "REAL"), ("speed_up", "REAL"), ("speed_ping", "REAL")] {
            if !have.iter().any(|h| h == col) {
                conn.execute_batch(&format!("ALTER TABLE reports ADD COLUMN {col} {ty}"))?;
            }
        }
        Ok(Store { conn })
    }

    /// Stores valid reports; returns how many were new. Invalid ones are skipped.
    pub fn insert(&mut self, client_ip: Option<IpAddr>, reports: &[ReportIn], now: i64) -> Result<usize> {
        Ok(self.submit(client_ip, reports, now, true)?.stored)
    }

    /// Checks and stores reports; with `keep` false (a trial report) everything
    /// up to the database write happens, and is then rolled back.
    pub fn submit(&mut self, client_ip: Option<IpAddr>, reports: &[ReportIn], now: i64, keep: bool) -> Result<Outcome> {
        let client = client_ip.map(|c| c.to_string());
        let recent: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reports WHERE client_ip IS ?1 AND received_at > ?2",
            params![client, now - 3600],
            |r| r.get(0),
        )?;
        let budget = (MAX_PER_CLIENT_PER_HOUR - recent).max(0) as usize;
        let tx = self.conn.transaction()?;
        let mut added = 0;
        let mut rejected = Vec::new();
        let mut valid = Vec::new();
        for r in reports {
            match r.validate(now) {
                Ok(()) if valid.len() < budget => valid.push(r),
                Ok(()) => rejected.push("over the hourly limit of this client".to_string()),
                Err(e) => rejected.push(e.to_string()),
            }
        }
        let mut duplicates = 0;
        for r in valid {
            let path: Vec<String> = r.path.iter().map(|p| p.to_string()).collect();
            added += tx.execute(
                "INSERT OR IGNORE INTO reports
                 (id, received_at, client_ip, detected_at, resolved_at, verdict, my_ip, ap, first_bad, last_ok, path, snapshot_at,
                  speed_at, speed_down, speed_up, speed_ping)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                params![
                    r.id,
                    now,
                    client,
                    r.detected_at,
                    r.resolved_at,
                    r.verdict,
                    r.my_ip.map(|a| a.to_string()),
                    r.ap.map(|a| a.to_string()),
                    r.first_bad.map(|a| a.to_string()),
                    r.last_ok.map(|a| a.to_string()),
                    serde_json::to_string(&path)?,
                    r.snapshot_at,
                    r.speed.map(|s| s.measured_at),
                    r.speed.map(|s| s.down_mbps),
                    r.speed.map(|s| s.up_mbps),
                    r.speed.map(|s| s.ping_ms),
                ],
            )?;
            if tx.changes() == 0 {
                duplicates += 1;
            }
        }
        if !keep {
            // A trial report: the writes worked, now undo them.
            tx.rollback()?;
            return Ok(Outcome { stored: added, duplicates, rejected });
        }
        // Retention: drop old rows, then cap the table size.
        tx.execute("DELETE FROM reports WHERE received_at < ?1", params![now - KEEP_DAYS * 24 * 3600])?;
        tx.execute(
            "DELETE FROM reports WHERE id IN (SELECT id FROM reports ORDER BY received_at DESC LIMIT -1 OFFSET ?1)",
            params![MAX_ROWS],
        )?;
        tx.commit()?;
        Ok(Outcome { stored: added, duplicates, rejected })
    }

    /// Recent outages for the public tab (no member addresses).
    pub fn public_recent(&self, since: i64, limit: usize) -> Result<Vec<PublicReport>> {
        let mut st = self.conn.prepare(
            "SELECT detected_at, resolved_at, verdict, ap, first_bad, last_ok FROM reports
             WHERE detected_at >= ?1 ORDER BY detected_at DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![since, limit as i64], |r| {
            Ok(PublicReport {
                detected_at: r.get(0)?,
                resolved_at: r.get(1)?,
                verdict: r.get(2)?,
                ap: r.get(3)?,
                first_bad: r.get(4)?,
                last_ok: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Devices with an open "doesn't answer" outage, as (AP, address).
    pub fn open_unanswered(&self) -> Result<Vec<(String, String)>> {
        let mut st = self.conn.prepare("SELECT ap, ip FROM measured WHERE ended IS NULL AND kind = 0")?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Records what UserDB's status reports now as outage episodes: a device
    /// with a problem opens one (or keeps it open, with the worst kind seen),
    /// a device no longer listed closes its open one, unless it is in
    /// `still_down` (it doesn't answer, but the status stopped listing it).
    /// Kept for 90 days.
    pub fn record_problems(&mut self, now_problems: &[crate::userdb::Problem], still_down: &[(String, String)], now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        let open: Vec<(i64, String, String)> = {
            let mut st = tx.prepare("SELECT rowid, ap, ip FROM measured WHERE ended IS NULL")?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        for p in now_problems {
            let ip = p.ip.to_string();
            match open.iter().find(|(_, ap, oip)| *ap == p.ap && *oip == ip) {
                Some((rowid, _, _)) => {
                    tx.execute("UPDATE measured SET kind = MIN(kind, ?1), label = ?2 WHERE rowid = ?3", params![p.kind, p.label, rowid])?;
                }
                None => {
                    // A device that doesn't answer has been out since it last answered.
                    let started = match (p.kind, p.last_answer) {
                        (0, Some(t)) if t < now => t,
                        _ => now,
                    };
                    // It's the same outage if an episode of this device ended
                    // after it began (the device dropped off the status for a
                    // while): reopen that one instead of starting another.
                    let same: Option<i64> = tx
                        .query_row(
                            "SELECT rowid FROM measured WHERE ap = ?1 AND ip = ?2 AND ended IS NOT NULL AND ended >= ?3 ORDER BY ended DESC LIMIT 1",
                            params![p.ap, ip, started],
                            |r| r.get(0),
                        )
                        .optional()?;
                    match same {
                        Some(rowid) => {
                            tx.execute(
                                "UPDATE measured SET ended = NULL, kind = MIN(kind, ?1), label = ?2, started = MIN(started, ?3) WHERE rowid = ?4",
                                params![p.kind, p.label, started, rowid],
                            )?;
                        }
                        None => {
                            tx.execute(
                                "INSERT INTO measured (ap, ip, label, kind, started) VALUES (?1, ?2, ?3, ?4, ?5)",
                                params![p.ap, ip, p.label, p.kind, started],
                            )?;
                        }
                    }
                }
            }
        }
        for (rowid, ap, ip) in &open {
            let listed = now_problems.iter().any(|p| p.ap == *ap && p.ip.to_string() == *ip);
            if !listed && !still_down.iter().any(|(a, i)| a == ap && i == ip) {
                tx.execute("UPDATE measured SET ended = ?1 WHERE rowid = ?2", params![now, rowid])?;
            }
        }
        tx.execute("DELETE FROM measured WHERE ended IS NOT NULL AND ended < ?1", params![now - 90 * 24 * 3600])?;
        tx.commit()?;
        Ok(())
    }

    /// Outage episodes network monitoring measured that touch the period since `since`, newest first.
    pub fn measured_since(&self, since: i64, limit: usize) -> Result<Vec<Measured>> {
        let mut st = self.conn.prepare(
            "SELECT ap, ip, label, kind, started, ended FROM measured
             WHERE ended IS NULL OR ended >= ?1 ORDER BY started DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![since, limit as i64], |r| {
            Ok(Measured { ap: r.get(0)?, ip: r.get(1)?, label: r.get(2)?, kind: r.get(3)?, started: r.get(4)?, ended: r.get(5)? })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Reports that ended since `since` (public: no member addresses), newest first.
    pub fn public_recent_ended(&self, since: i64, limit: usize) -> Result<Vec<PublicReport>> {
        let mut st = self.conn.prepare(
            "SELECT detected_at, resolved_at, verdict, ap, first_bad, last_ok FROM reports
             WHERE resolved_at >= ?1 ORDER BY detected_at DESC LIMIT ?2",
        )?;
        let rows = st.query_map(params![since, limit as i64], |r| {
            Ok(PublicReport {
                detected_at: r.get(0)?,
                resolved_at: r.get(1)?,
                verdict: r.get(2)?,
                ap: r.get(3)?,
                first_bad: r.get(4)?,
                last_ok: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Admin view: everything, newest first.
    pub fn all_since(&self, since: i64) -> Result<Vec<serde_json::Value>> {
        let mut st = self.conn.prepare(
            "SELECT id, received_at, client_ip, detected_at, resolved_at, verdict, my_ip, ap, first_bad, last_ok, path,
                    speed_at, speed_down, speed_up, speed_ping
             FROM reports WHERE detected_at >= ?1 ORDER BY detected_at DESC",
        )?;
        let rows = st.query_map(params![since], |r| {
            Ok(serde_json::json!({
                "id": r.get::<_, String>(0)?,
                "received_at": r.get::<_, i64>(1)?,
                "client_ip": r.get::<_, Option<String>>(2)?,
                "detected_at": r.get::<_, i64>(3)?,
                "resolved_at": r.get::<_, i64>(4)?,
                "verdict": r.get::<_, String>(5)?,
                "my_ip": r.get::<_, Option<String>>(6)?,
                "ap": r.get::<_, Option<String>>(7)?,
                "first_bad": r.get::<_, Option<String>>(8)?,
                "last_ok": r.get::<_, Option<String>>(9)?,
                "path": serde_json::from_str::<serde_json::Value>(&r.get::<_, String>(10)?).unwrap_or_default(),
                "speed": match r.get::<_, Option<i64>>(11)? {
                    Some(at) => serde_json::json!({ "measured_at": at, "down_mbps": r.get::<_, f64>(12)?, "up_mbps": r.get::<_, f64>(13)?, "ping_ms": r.get::<_, f64>(14)? }),
                    None => serde_json::Value::Null,
                },
            }))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Stores speed test results; `ap_of` names the AP of an AP router, if the
    /// member records do. Returns how many were new. Kept 180 days.
    pub fn insert_speeds(&mut self, speeds: &[SpeedIn], ap_of: impl Fn(Ipv4Addr) -> Option<(u32, String)>, now: i64) -> Result<(usize, Vec<String>)> {
        let tx = self.conn.transaction()?;
        let mut stored = 0;
        let mut refused = Vec::new();
        for s in speeds {
            let id_ok = (8..=64).contains(&s.id.len()) && s.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
            if !id_ok {
                refused.push("invalid id".to_string());
                continue;
            }
            if let Err(e) = s.result().validate(now) {
                refused.push(e.to_string());
                continue;
            }
            let ap = ap_of(s.ap);
            stored += tx.execute(
                "INSERT OR IGNORE INTO speeds (id, received_at, measured_at, ap_router, ap_id, ap_name, down, up, ping)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![s.id, now, s.measured_at, s.ap.to_string(), ap.as_ref().map(|a| a.0), ap.map(|a| a.1), s.down_mbps, s.up_mbps, s.ping_ms],
            )?;
        }
        tx.execute("DELETE FROM speeds WHERE received_at < ?1", params![now - KEEP_DAYS * 24 * 3600])?;
        tx.execute("DELETE FROM speeds WHERE id IN (SELECT id FROM speeds ORDER BY received_at DESC LIMIT -1 OFFSET ?1)", params![MAX_ROWS])?;
        tx.commit()?;
        Ok((stored, refused))
    }

    /// Speeds measured since `since`, per AP router: how many, the typical
    /// (median) and the lowest. Only groups with at least MIN_SPEEDS_SHOWN.
    pub fn speeds_by_ap(&self, since: i64) -> Result<Vec<SpeedGroup>> {
        let mut st = self.conn.prepare(
            "SELECT ap_router, ap_id, ap_name, down, up, ping FROM speeds WHERE measured_at >= ?1 ORDER BY ap_router, measured_at",
        )?;
        let rows = st.query_map(params![since], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, Option<u32>>(1)?, r.get::<_, Option<String>>(2)?, r.get::<_, f64>(3)?, r.get::<_, f64>(4)?, r.get::<_, f64>(5)?))
        })?;
        let mut groups: std::collections::BTreeMap<String, (Option<u32>, Option<String>, Vec<f64>, Vec<f64>, Vec<f64>)> = Default::default();
        for row in rows {
            let (router, id, name, d, u, p) = row?;
            let g = groups.entry(router).or_default();
            // The newest name the member records gave.
            if id.is_some() {
                (g.0, g.1) = (id, name);
            }
            g.2.push(d);
            g.3.push(u);
            g.4.push(p);
        }
        Ok(groups
            .into_iter()
            .filter(|(_, g)| g.2.len() >= MIN_SPEEDS_SHOWN)
            .map(|(router, (id, name, d, u, p))| SpeedGroup {
                ap_router: router,
                ap: id.map(|id| ApName { id, name: name.unwrap_or_default() }),
                count: d.len(),
                down_typical: median(d.clone()),
                down_lowest: d.iter().copied().fold(f64::INFINITY, f64::min),
                up_typical: median(u),
                ping_typical: median(p),
            })
            .collect())
    }

    #[cfg(test)]
    pub fn count(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM reports", [], |r| r.get(0))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(id: &str, now: i64) -> ReportIn {
        ReportIn {
            id: id.into(),
            detected_at: now - 600,
            resolved_at: now - 60,
            verdict: "backbone".into(),
            my_ip: Some("10.107.5.130".parse().unwrap()),
            ap: Some("10.107.5.1".parse().unwrap()),
            first_bad: Some("10.107.2.1".parse().unwrap()),
            last_ok: Some("10.107.5.1".parse().unwrap()),
            path: vec!["10.107.5.1".parse().unwrap(), "10.107.2.1".parse().unwrap()],
            snapshot_at: now - 700,
            speed: None,
        }
    }

    #[test]
    fn measured_outages_open_and_close() {
        let (mut st, dir) = store();
        let prob = |kind| crate::userdb::Problem { ap: "Podůlšany".into(), ip: "10.107.10.150".parse().unwrap(), label: "sektor jih".into(), kind, loss: 0.4, last_answer: None };
        st.record_problems(&[prob(2)], &[], 1000).unwrap();
        st.record_problems(&[prob(0)], &[], 1060).unwrap();
        let open = st.measured_since(0, 10).unwrap();
        assert_eq!((open.len(), open[0].kind, open[0].started, open[0].ended), (1, 0, 1000, None), "one episode, worst kind");
        st.record_problems(&[], &[], 1120).unwrap();
        let closed = st.measured_since(0, 10).unwrap();
        assert_eq!(closed[0].ended, Some(1120));
        st.record_problems(&[prob(2)], &[], 1200).unwrap();
        assert_eq!(st.measured_since(0, 10).unwrap().len(), 2, "a new episode after it ended");
        assert_eq!(st.measured_since(1150, 10).unwrap().len(), 1, "the old one ended before the period");
        // A device that doesn't answer: the outage began when it last answered.
        let down = crate::userdb::Problem { ip: "10.107.10.151".parse().unwrap(), kind: 0, last_answer: Some(800), ..prob(0) };
        st.record_problems(&[down], &[], 2000).unwrap();
        assert_eq!(st.measured_since(0, 10).unwrap().iter().find(|m| m.ip == "10.107.10.151").unwrap().started, 800);
        // It drops off the status for a while and comes back, still out since 800: one outage, not two.
        st.record_problems(&[], &[], 2060).unwrap();
        st.record_problems(&[crate::userdb::Problem { ip: "10.107.10.151".parse().unwrap(), kind: 0, last_answer: Some(800), ..prob(0) }], &[], 2120).unwrap();
        let same: Vec<_> = st.measured_since(0, 10).unwrap().into_iter().filter(|m| m.ip == "10.107.10.151").collect();
        assert_eq!(same.len(), 1, "{same:?}");
        assert_eq!((same[0].started, same[0].ended), (800, None));
        // Dropped from the status after a week but still silent: it stays open.
        st.record_problems(&[], &[("Podůlšany".into(), "10.107.10.151".into())], 3000).unwrap();
        assert_eq!(st.measured_since(0, 10).unwrap().iter().find(|m| m.ip == "10.107.10.151").unwrap().ended, None);
        assert_eq!(st.open_unanswered().unwrap(), vec![("Podůlšany".to_string(), "10.107.10.151".to_string())]);
        st.record_problems(&[], &[], 3060).unwrap();
        assert_eq!(st.measured_since(0, 10).unwrap().iter().find(|m| m.ip == "10.107.10.151").unwrap().ended, Some(3060), "it answers again");
        // Only the test's own directory, which store() made.
        assert!(dir.file_name().unwrap().to_string_lossy().starts_with("hkdiag-reports-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ipv6_member_address_is_accepted() {
        let body = r#"[{"id":"abcdefgh12","detected_at":1,"resolved_at":2,"verdict":"backbone","my_ip":"2a01:168:200:100::5","path":[]}]"#;
        let list: Vec<ReportIn> = serde_json::from_str(body).unwrap();
        assert_eq!(list[0].my_ip, Some("2a01:168:200:100::5".parse().unwrap()));
    }

    fn store() -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!("hkdiag-reports-{}-{}", std::process::id(), rand_suffix()));
        let p = dir.join("r.sqlite");
        (Store::open(&p).unwrap(), dir)
    }

    fn rand_suffix() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
    }

    #[test]
    fn speeds_are_shown_per_ap_only_from_three_on() {
        let (mut s, dir) = store();
        let now = 1_800_000_000;
        let speed = |id: &str, ap: &str, down: f64| SpeedIn { id: id.into(), ap: ap.parse().unwrap(), measured_at: now - 60, down_mbps: down, up_mbps: down / 10.0, ping_ms: 5.0 };
        let named = |r: Ipv4Addr| (r.to_string() == "10.107.10.149").then(|| (10, "Podůlšany".to_string()));
        let (n, refused) = s.insert_speeds(&[speed("speed-0001", "10.107.10.149", 80.0), speed("speed-0002", "10.107.10.149", 95.0), speed("speed-0003", "10.107.10.149", 20.0), speed("speed-0004", "10.107.77.1", 50.0), speed("bad", "10.107.77.1", 1.0), speed("speed-0005", "10.107.77.1", f64::NAN)], named, now).unwrap();
        assert_eq!((n, refused), (4, vec!["invalid id".to_string(), "speed out of range".to_string()]));
        let g = s.speeds_by_ap(0).unwrap();
        assert_eq!(g.len(), 1, "10.107.77.1 has only one: not shown");
        assert_eq!((g[0].ap_router.as_str(), g[0].ap.as_ref().unwrap().name.as_str(), g[0].count, g[0].down_typical, g[0].down_lowest), ("10.107.10.149", "Podůlšany", 3, 80.0, 20.0));
        assert_eq!(median(vec![50.0, 70.0, 60.0, 10.0]), 55.0);
        // Only the test's own directory, which store() made.
        assert!(dir.file_name().unwrap().to_string_lossy().starts_with("hkdiag-reports-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_database_from_before_speeds_gets_their_columns() {
        let dir = std::env::temp_dir().join(format!("hkdiag-reports-{}-{}", std::process::id(), rand_suffix()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("old.sqlite");
        Connection::open(&p).unwrap().execute_batch(
            "CREATE TABLE reports (id TEXT PRIMARY KEY, received_at INTEGER NOT NULL, client_ip TEXT, detected_at INTEGER NOT NULL,
             resolved_at INTEGER NOT NULL, verdict TEXT NOT NULL, my_ip TEXT, ap TEXT, first_bad TEXT, last_ok TEXT,
             path TEXT NOT NULL, snapshot_at INTEGER NOT NULL);
             INSERT INTO reports VALUES ('abcdefgh-old', 1, NULL, 1, 2, 'backbone', NULL, NULL, NULL, NULL, '[]', 0);",
        ).unwrap();
        let mut s = Store::open(&p).unwrap();
        assert_eq!(s.all_since(0).unwrap()[0]["speed"], serde_json::Value::Null, "old rows keep working");
        let now = 1_800_000_000;
        assert_eq!(s.insert(None, &[ReportIn { speed: Some(SpeedResult { measured_at: now - 60, down_mbps: 3.0, up_mbps: 1.0, ping_ms: 9.0 }), ..report("abcdefgh-new", now) }], now).unwrap(), 1);
        drop(s);
        assert!(Store::open(&p).is_ok(), "opening again doesn't add the columns twice");
        // Only the test's own directory, made above.
        assert!(dir.file_name().unwrap().to_string_lossy().starts_with("hkdiag-reports-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_outage_report_keeps_the_members_last_speed() {
        let (mut s, dir) = store();
        let now = 1_800_000_000;
        let with = ReportIn { speed: Some(SpeedResult { measured_at: now - 3600, down_mbps: 3.0, up_mbps: 1.0, ping_ms: 20.0 }), ..report("abcdefgh-9", now) };
        let stale = ReportIn { speed: Some(SpeedResult { measured_at: now - 3 * 86400, down_mbps: 3.0, up_mbps: 1.0, ping_ms: 20.0 }), ..report("abcdefgh-8", now) };
        let o = s.submit(Some("10.107.5.130".parse().unwrap()), &[with, stale], now, true).unwrap();
        assert_eq!((o.stored, o.rejected), (1, vec!["speed not from around the outage".to_string()]));
        let rows = s.all_since(0).unwrap();
        assert_eq!(rows[0]["speed"]["down_mbps"], serde_json::json!(3.0));
        // Only the test's own directory, which store() made.
        assert!(dir.file_name().unwrap().to_string_lossy().starts_with("hkdiag-reports-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_trial_report_is_checked_and_not_kept() {
        let (mut s, dir) = store();
        let now = 1_800_000_000;
        let client = Some("10.107.5.130".parse().unwrap());
        let bad = ReportIn { verdict: "works".into(), ..report("abcdefgh-2", now) };
        let o = s.submit(client, &[report("abcdefgh-1", now), bad], now, false).unwrap();
        assert_eq!(o, Outcome { stored: 1, duplicates: 0, rejected: vec!["verdict not reportable".into()] });
        assert!(s.public_recent(0, 10).unwrap().is_empty(), "a trial keeps nothing");
        assert_eq!(s.submit(client, &[report("abcdefgh-1", now)], now, true).unwrap().stored, 1, "the real one is stored after the trial");
        assert_eq!(s.submit(client, &[report("abcdefgh-1", now)], now, false).unwrap().duplicates, 1);
        // Only the test's own directory, which store() made.
        assert!(dir.file_name().unwrap().to_string_lossy().starts_with("hkdiag-reports-"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stores_dedupes_and_hides_member_ips_publicly() {
        let (mut s, dir) = store();
        let now = 1_800_000_000;
        let client = Some("10.107.5.130".parse().unwrap());
        assert_eq!(s.insert(client, &[report("abcdefgh-1", now)], now).unwrap(), 1);
        assert_eq!(s.insert(client, &[report("abcdefgh-1", now)], now).unwrap(), 0);
        let public = serde_json::to_string(&s.public_recent(0, 10).unwrap()).unwrap();
        assert!(public.contains("10.107.2.1"));
        assert!(!public.contains("10.107.5.130"), "member address leaked: {public}");
        let meta = std::fs::metadata(dir.join("r.sqlite")).unwrap();
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_invalid_reports() {
        let now = 1_800_000_000;
        let mut r = report("abcdefgh-2", now);
        r.verdict = "ok".into();
        assert!(r.validate(now).is_err());
        let mut r = report("bad id with spaces", now);
        r.id = "x'; DROP TABLE reports;--".into();
        assert!(r.validate(now).is_err());
        let mut r = report("abcdefgh-3", now);
        r.detected_at = now + 3600;
        assert!(r.validate(now).is_err());
        let mut r = report("abcdefgh-4", now);
        r.resolved_at = r.detected_at - 1;
        assert!(r.validate(now).is_err());
        let mut r = report("abcdefgh-5", now);
        r.path = vec!["10.0.0.1".parse().unwrap(); 33];
        assert!(r.validate(now).is_err());
        assert!(serde_json::from_str::<ReportIn>(r#"{"id":"abcdefgh","detected_at":1,"resolved_at":1,"verdict":"backbone","evil":1}"#).is_err());
    }

    #[test]
    fn rate_limits_per_client() {
        let (mut s, dir) = store();
        let now = 1_800_000_000;
        let client = Some("10.107.5.130".parse().unwrap());
        let batch: Vec<_> = (0..20).map(|i| report(&format!("rate-limit-{i:03}"), now)).collect();
        assert_eq!(s.insert(client, &batch, now).unwrap(), 20);
        let batch: Vec<_> = (20..40).map(|i| report(&format!("rate-limit-{i:03}"), now)).collect();
        assert_eq!(s.insert(client, &batch, now).unwrap(), 10);
        assert_eq!(s.count().unwrap(), 30);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
