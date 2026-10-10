//! Gathers all live inputs for one diagnosis (shared by the CLI and the web server).

use crate::config::Config;
use crate::diagnose::{self, Inputs, Report};
use crate::ospf::Topology;
use crate::state::State;
use crate::{probe, sojka, userdb};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::thread;
use std::time::Duration;

pub struct Options {
    pub probe: bool,
    pub member: bool,
    /// Pre-loaded (cached) AP list and infrastructure devices from UserDB.
    pub infra: Option<UserDbData>,
}

pub fn run(cfg: &Config, topo: &Topology, state: &State, ip: Ipv4Addr, opt: &Options) -> Report {
    let ips = diagnose::interesting_ips(topo, state, ip);
    let mut inputs = Inputs {
        target: Some(ip),
        lookback: Duration::from_secs(cfg.sojka.lookback_secs),
        ..Default::default()
    };

    // Sojka, ping and UserDB are independent network waits: run them concurrently.
    thread::scope(|s| {
        let sojka_h = s.spawn(|| sojka::ping_ips(&cfg.sojka, &ips));
        let probe_h = opt.probe.then(|| s.spawn(|| probe::ping_all(&ips)));
        let infra = opt.infra.clone();
        let userdb_h = cfg
            .userdb
            .as_ref()
            .map(|_| s.spawn(move || load_userdb(cfg, topo, ip, infra)));
        let member_h = opt.member.then(|| s.spawn(|| check_membership(cfg)));

        match sojka_h.join() {
            Ok(Ok(r)) => inputs.sojka = r,
            Ok(Err(e)) => inputs.sojka_error = Some(format!("{e:#}")),
            Err(_) => inputs.sojka_error = Some("Sojka query panicked".into()),
        }
        if let Some(h) = probe_h {
            inputs.probes = h
                .join()
                .unwrap_or_default()
                .into_iter()
                .map(|p| (p.ip, p))
                .collect();
        }
        if let Some(h) = userdb_h {
            match h.join() {
                Ok(Ok((aps, devices))) => {
                    inputs.aps = aps;
                    inputs.devices = devices;
                }
                Ok(Err(e)) => inputs.userdb_error = Some(format!("{e:#}")),
                Err(_) => inputs.userdb_error = Some("UserDB query panicked".into()),
            }
        }
        if let Some(h) = member_h {
            inputs.membership = Some(
                h.join()
                    .unwrap_or_else(|_| Err(anyhow::anyhow!("panicked")))
                    .map_err(|e| format!("{e:#}")),
            );
        }
    });
    diagnose::diagnose(topo, state, ip, &inputs)
}

pub type UserDbData = (
    BTreeMap<u32, userdb::Ap>,
    BTreeMap<Ipv4Addr, userdb::Device>,
);

/// AP list plus infrastructure IPs (small), and member devices only for the
/// AP that serves `target` instead of the whole member table.
pub fn load_userdb(
    cfg: &Config,
    topo: &Topology,
    target: Ipv4Addr,
    infra: Option<UserDbData>,
) -> Result<UserDbData> {
    let (aps, mut devices) = match infra {
        Some(i) => i,
        None => load_infrastructure(cfg)?,
    };
    let u = cfg
        .userdb
        .as_ref()
        .context("no [userdb] section in config")?;
    if !devices.contains_key(&target) {
        let ap = topo.owner_router(target).0.and_then(|r| {
            let rr = topo.routers.get(&r);
            std::iter::once(r)
                .chain(rr.into_iter().flat_map(|x| x.interfaces.iter().copied()))
                .find_map(|a| devices.get(&a)?.ap_id)
        });
        if let Some(ap) = ap {
            // Optional detail: on failure keep the names we already have.
            match userdb::UserDb::new(u).devices(true, Some(ap)) {
                Ok(d) => devices.extend(d),
                Err(e) => eprintln!("warning: UserDB member devices for AP {ap}: {e:#}"),
            }
        }
    }
    Ok((aps, devices))
}

pub fn load_infrastructure(cfg: &Config) -> Result<UserDbData> {
    let u = cfg
        .userdb
        .as_ref()
        .context("no [userdb] section in config")?;
    let db = userdb::UserDb::new(u);
    Ok((db.aps()?, db.devices(false, None)?))
}

fn check_membership(cfg: &Config) -> Result<userdb::Membership> {
    let u = cfg
        .userdb
        .as_ref()
        .context("--member needs a [userdb] section with uid")?;
    let uid = u.uid.context("--member needs userdb.uid in the config")?;
    if u.password.is_empty() {
        anyhow::bail!(
            "--member needs userdb.password in the config file or HKDIAG_USERDB_PASSWORD"
        );
    }
    userdb::UserDb::new(u).membership(uid, &u.password)
}
