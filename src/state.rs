//! Remembers which router advertised which prefix, so that when a subnet
//! disappears from OSPF we can still say which router used to own it.

use crate::ospf::{Prefix, PrefixOwner, Topology};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::Ipv4Addr;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;

const MAX_STATE: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Seen {
    pub router: Ipv4Addr,
    pub prefix: Prefix,
    /// Unix time of the last snapshot that contained this prefix.
    pub last_seen: u64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct State {
    /// keyed by "prefix router"
    pub seen: BTreeMap<String, Seen>,
}

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))?;
    Some(base.join("hkdiag/prefixes.json"))
}

impl State {
    pub fn load() -> State {
        let Some(p) = path() else {
            return State::default();
        };
        let Ok(meta) = fs::metadata(&p) else {
            return State::default();
        };
        let parsed = if meta.len() > MAX_STATE {
            None
        } else {
            fs::read_to_string(&p)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
        };
        parsed.unwrap_or_else(|| {
            // Keep the unreadable file for inspection instead of silently
            // overwriting up to 90 days of history.
            let bad = p.with_extension("json.corrupt");
            let _ = fs::rename(&p, &bad);
            eprintln!(
                "warning: state file was unreadable, moved to {}",
                bad.display()
            );
            State::default()
        })
    }

    /// Records the current prefixes; returns true if a prefix/router pair was
    /// new or one was forgotten (i.e. the file is worth saving now).
    pub fn update(&mut self, topo: &Topology) -> bool {
        let t = crate::now().max(0) as u64;
        let mut changed = false;
        for PrefixOwner { prefix, router, .. } in &topo.prefixes {
            let prev = self.seen.insert(
                format!("{prefix} {router}"),
                Seen {
                    router: *router,
                    prefix: *prefix,
                    last_seen: t,
                },
            );
            changed |= prev.is_none();
        }
        // Forget entries not seen for 90 days.
        let before = self.seen.len();
        self.seen
            .retain(|_, s| t.saturating_sub(s.last_seen) < 90 * 24 * 3600);
        changed || self.seen.len() != before
    }

    /// Most specific previously seen prefix(es) containing `ip`.
    pub fn previous_owners(&self, ip: Ipv4Addr) -> Vec<&Seen> {
        crate::ospf::longest_match(self.seen.values(), |s| s.prefix, ip)
    }

    /// Writes atomically with mode 0600 in a 0700 directory.
    pub fn save(&self) -> Result<()> {
        let Some(p) = path() else { return Ok(()) };
        let dir = p.parent().context("state path has no parent")?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        // DirBuilder's mode only applies to newly created directories.
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        let tmp = p.with_extension(format!("tmp.{}", std::process::id()));
        // Leftover from a killed run that had the same pid.
        let _ = fs::remove_file(&tmp);
        let write = || -> Result<()> {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp)?;
            f.write_all(serde_json::to_string(self)?.as_bytes())?;
            f.sync_all()?;
            fs::rename(&tmp, &p).with_context(|| format!("writing {}", p.display()))?;
            Ok(())
        };
        let res = write();
        if res.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        res
    }
}
