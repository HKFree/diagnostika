//! Outage reports sent by the PWA once a member's connection is back.
//!
//! Stored in SQLite. Everything from the client is untrusted: strict
//! validation (allowlisted verdicts, IPv4 only, sane timestamps, bounded
//! sizes), per-client rate limits and a cap on the table size.
//!
//! The public view (`public_recent`) never includes member addresses.

use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection};
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
}

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
        Ok(())
    }
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
             CREATE INDEX IF NOT EXISTS measured_started ON measured(started);",
        )?;
        Ok(Store { conn })
    }

    /// Stores valid reports; returns how many were new. Invalid ones are skipped.
    pub fn insert(&mut self, client_ip: Option<IpAddr>, reports: &[ReportIn], now: i64) -> Result<usize> {
        let client = client_ip.map(|c| c.to_string());
        let recent: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM reports WHERE client_ip IS ?1 AND received_at > ?2",
            params![client, now - 3600],
            |r| r.get(0),
        )?;
        let budget = (MAX_PER_CLIENT_PER_HOUR - recent).max(0) as usize;
        let tx = self.conn.transaction()?;
        let mut added = 0;
        for r in reports.iter().filter(|r| r.validate(now).is_ok()).take(budget) {
            let path: Vec<String> = r.path.iter().map(|p| p.to_string()).collect();
            added += tx.execute(
                "INSERT OR IGNORE INTO reports
                 (id, received_at, client_ip, detected_at, resolved_at, verdict, my_ip, ap, first_bad, last_ok, path, snapshot_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                ],
            )?;
        }
        // Retention: drop old rows, then cap the table size.
        tx.execute("DELETE FROM reports WHERE received_at < ?1", params![now - KEEP_DAYS * 24 * 3600])?;
        tx.execute(
            "DELETE FROM reports WHERE id IN (SELECT id FROM reports ORDER BY received_at DESC LIMIT -1 OFFSET ?1)",
            params![MAX_ROWS],
        )?;
        tx.commit()?;
        Ok(added)
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

    /// Records what UserDB's status reports now as outage episodes: a device
    /// with a problem opens one (or keeps it open, with the worst kind seen),
    /// a device no longer listed closes its open one. Kept for 90 days.
    pub fn record_problems(&mut self, now_problems: &[crate::userdb::Problem], now: i64) -> Result<()> {
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
                    tx.execute(
                        "INSERT INTO measured (ap, ip, label, kind, started) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![p.ap, ip, p.label, p.kind, now],
                    )?;
                }
            }
        }
        for (rowid, ap, ip) in &open {
            if !now_problems.iter().any(|p| p.ap == *ap && p.ip.to_string() == *ip) {
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

    /// Admin view: everything, newest first.
    pub fn all_since(&self, since: i64) -> Result<Vec<serde_json::Value>> {
        let mut st = self.conn.prepare(
            "SELECT id, received_at, client_ip, detected_at, resolved_at, verdict, my_ip, ap, first_bad, last_ok, path
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
            }))
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
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
        }
    }

    #[test]
    fn measured_outages_open_and_close() {
        let (mut st, dir) = store();
        let prob = |kind| crate::userdb::Problem { ap: "Podůlšany".into(), ip: "10.107.10.150".parse().unwrap(), label: "sektor jih".into(), kind, loss: 0.4 };
        st.record_problems(&[prob(2)], 1000).unwrap();
        st.record_problems(&[prob(0)], 1060).unwrap();
        let open = st.measured_since(0, 10).unwrap();
        assert_eq!((open.len(), open[0].kind, open[0].started, open[0].ended), (1, 0, 1000, None), "one episode, worst kind");
        st.record_problems(&[], 1120).unwrap();
        let closed = st.measured_since(0, 10).unwrap();
        assert_eq!(closed[0].ended, Some(1120));
        st.record_problems(&[prob(2)], 1200).unwrap();
        assert_eq!(st.measured_since(0, 10).unwrap().len(), 2, "a new episode after it ended");
        assert_eq!(st.measured_since(1150, 10).unwrap().len(), 1, "the old one ended before the period");
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
