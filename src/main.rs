#![forbid(unsafe_code)]

mod collect;
mod config;
mod diagnose;
mod export;
mod history;
mod http;
mod lsdb;
mod mikrotik;
mod ospf;
mod power;
mod probe;
mod report;
mod reports;
mod server;
mod signal;
mod sojka;
mod state;
mod userdb;
mod verify;
mod verified;
mod vty;
mod web;

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::Config;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// After a failed OSPFv3 read, keep the last good prefixes and don't retry
/// for this long, so an unreachable ospf6d doesn't slow every map build.
const OSPF6_BACKOFF: Duration = Duration::from_secs(300);

/// When OSPFv3 last failed, and the last prefixes read successfully.
struct Ospf6State {
    failed_at: Option<Instant>,
    last_good: Vec<ospf::Prefix6Owner>,
}

static OSPF6: Mutex<Ospf6State> = Mutex::new(Ospf6State { failed_at: None, last_good: Vec::new() });

/// Diagnose why a host in the hkfree.org network has no connectivity.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Config file (default: ~/.config/hkdiag/config.toml). Must be chmod 600.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Diagnose connectivity of an IPv4 address (member device, CPE or router).
    Diagnose {
        ip: Ipv4Addr,
        /// Do not ping from this host (OSPF + Sojka only).
        #[arg(long)]
        no_probe: bool,
        /// Also log in as the member configured in [userdb] and check membership.
        #[arg(long)]
        member: bool,
        /// Machine-readable output.
        #[arg(long)]
        json: bool,
    },
    /// Serve the PWA, /topology.json and /api/whoami (put an HTTPS reverse proxy in front).
    Serve,
    /// Write the topology snapshot for the on-device diagnosis (e.g. from cron).
    Export {
        /// Output file (default: stdout).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also write the complete static PWA into this directory
        /// (index.html, app.js, …, topology.json) for any static web server.
        #[arg(long)]
        web: Option<PathBuf>,
    },
    /// Outage reports sent by members' apps (admin view, includes member addresses).
    Reports {
        /// How far back, in hours.
        #[arg(long, default_value_t = 168)]
        hours: u32,
        #[arg(long)]
        json: bool,
    },
    /// Check a deployed hkdiag from the outside (exit 1 if a check fails).
    /// Stores nothing: outage reporting is checked with a trial report.
    Verify {
        /// e.g. https://diag.hkfree.org
        url: String,
    },
    /// Print the live OSPF backbone.
    Topology {
        /// Graphviz output (render with: dot -Tsvg / neato -Tsvg).
        #[arg(long)]
        dot: bool,
    },
}

/// FRR's OSPF database in the common form; its summary records are optional detail.
fn read_frr(cfg: &Config) -> Result<lsdb::Lsdb> {
    if cfg.frr.password.is_empty() {
        anyhow::bail!("set frr.password in the config file or HKDIAG_FRR_PASSWORD");
    }
    let mut vty = vty::Vty::connect(&cfg.frr, &cfg.frr.password)?;
    let r = vty.run(vty::Command::RouterLsaJson)?;
    let n = vty.run(vty::Command::NetworkLsaJson)?;
    let e = vty.run(vty::Command::ExternalLsaJson)?;
    let s = vty.run(vty::Command::SummaryLsaJson);
    drop(vty);
    let mut db = ospf::lsdb_from_frr(&r, &n, &e)?;
    match s.and_then(|s| ospf::frr_summaries(&s)) {
        Ok(summaries) => db.summaries = summaries,
        Err(e) => eprintln!("warning: OSPF summary LSAs: {}", report::sanitize(&format!("{e:#}"))),
    }
    Ok(db)
}

fn fetch_topology(cfg: &Config) -> Result<ospf::Topology> {
    // FRR and the MikroTik routers at the same time; their databases are merged
    // newest record first, so each router adds what only it can see.
    let (frr, (extra, sources)) = std::thread::scope(|s| {
        let mikrotik = s.spawn(|| mikrotik::read_all(cfg));
        let frr = read_frr(cfg);
        (frr, mikrotik.join().unwrap_or_default())
    });
    let mut db = match frr {
        Ok(db) => db,
        Err(e) if !extra.is_empty() => {
            eprintln!("warning: FRR: {}; using the MikroTik routers only", report::sanitize(&format!("{e:#}")));
            lsdb::Lsdb::default()
        }
        Err(e) => return Err(e),
    };
    for other in extra {
        db.merge(other);
    }
    let mut topo = ospf::Topology::from_lsdb(&db);
    topo.sources = sources;
    // IPv6 is optional: without OSPFv3 the IPv4 map works as before.
    if let Some(host) = &cfg.frr.ospf6_host {
        let mut st = OSPF6.lock().unwrap_or_else(|e| e.into_inner());
        topo.prefixes6 = prefixes6_with_backoff(&mut st, Instant::now(), || fetch_prefixes6(cfg, host));
    }
    Ok(topo)
}

fn prefixes6_with_backoff(
    st: &mut Ospf6State,
    now: Instant,
    fetch: impl FnOnce() -> Result<Vec<ospf::Prefix6Owner>>,
) -> Vec<ospf::Prefix6Owner> {
    if st.failed_at.map_or(true, |t| now.duration_since(t) >= OSPF6_BACKOFF) {
        match fetch() {
            Ok(p) => {
                st.failed_at = None;
                st.last_good = p;
            }
            Err(e) => {
                st.failed_at = Some(now);
                eprintln!("warning: OSPFv3 (IPv6 prefixes): {}", report::sanitize(&format!("{e:#}")));
            }
        }
    }
    st.last_good.clone()
}

fn fetch_prefixes6(cfg: &Config, host: &str) -> Result<Vec<ospf::Prefix6Owner>> {
    let mut vty = vty::Vty::connect_to(&cfg.frr, host, cfg.frr.ospf6_port, &cfg.frr.password)?;
    let intra = vty.run(vty::Command::Ospf6IntraPrefixJson)?;
    let ext = vty.run(vty::Command::Ospf6ExternalJson)?;
    ospf::prefixes6_from_json(&intra, &ext)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {}", report::sanitize(&format!("{e:#}")));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    // Needs no config: it only talks to the server from the outside.
    if let Cmd::Verify { url } = &cli.cmd {
        if !(url.starts_with("https://") || url.starts_with("http://")) {
            anyhow::bail!("verify needs an http(s) URL, e.g. https://diag.hkfree.org");
        }
        if !verify::print(&verify::run(url, now())) {
            std::process::exit(1);
        }
        return Ok(());
    }
    let cfg = Config::load(cli.config.as_deref())?;
    match cli.cmd {
        Cmd::Topology { dot } => {
            let topo = fetch_topology(&cfg)?;
            let (aps, devices) = collect::load_infrastructure(&cfg).unwrap_or_default();
            let inputs = diagnose::Inputs { aps, devices, ..Default::default() };
            let namer = diagnose::Namer::new(&topo, &inputs);
            if dot {
                print!("{}", report::dot(&topo, |r| namer.name(r)));
            } else {
                for r in topo.routers.values() {
                    println!(
                        "{:<15} {:<40} neighbours {:>2}, links advertised {:>2}",
                        r.id,
                        report::sanitize(&namer.name(r.id).unwrap_or_default()),
                        r.edges.len(),
                        r.advertised_links
                    );
                }
                for e in &topo.exits {
                    println!("exit: {} {} metric {}", e.router, if e.e1 { "E1" } else { "E2" }, e.metric);
                }
            }
        }
        Cmd::Serve => server::serve(cfg)?,
        Cmd::Verify { .. } => unreachable!("handled before the config is loaded"),
        Cmd::Reports { hours, json } => {
            let path = cfg
                .web
                .as_ref()
                .and_then(|w| w.reports_db.clone())
                .or_else(reports::default_path)
                .ok_or_else(|| anyhow::anyhow!("no reports database path"))?;
            let store = reports::Store::open(&path)?;
            let rows = store.all_since(now() - i64::from(hours) * 3600)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&rows)?);
            } else {
                let mut by_router: std::collections::BTreeMap<String, usize> = Default::default();
                for r in &rows {
                    let s = |k: &str| report::sanitize(r[k].as_str().unwrap_or("-"));
                    let dur = r["resolved_at"].as_i64().unwrap_or(0) - r["detected_at"].as_i64().unwrap_or(0);
                    println!(
                        "{}  {:>5} min  {:<12} member {:<15} AP {:<15} first bad {:<15} last ok {}",
                        r["detected_at"].as_i64().unwrap_or(0),
                        dur / 60,
                        s("verdict"),
                        s("my_ip"),
                        s("ap"),
                        s("first_bad"),
                        s("last_ok")
                    );
                    *by_router.entry(s("first_bad")).or_default() += 1;
                }
                println!("\n{} report(s); most often failing router:", rows.len());
                let mut v: Vec<_> = by_router.into_iter().collect();
                v.sort_by(|a, b| b.1.cmp(&a.1));
                for (r, n) in v.into_iter().take(10) {
                    println!("  {n:>4}  {r}");
                }
            }
        }
        Cmd::Export { out, web } => {
            let topo = fetch_topology(&cfg)?;
            let mut st = state::State::load();
            st.update(&topo);
            if let Err(e) = st.save() {
                eprintln!("warning: cannot save state: {}", report::sanitize(&format!("{e:#}")));
            }
            let json = serde_json::to_vec(&export::build(&cfg, &topo, &st))?;
            let web_written = web.is_some();
            if let Some(dir) = web {
                std::fs::create_dir_all(&dir)?;
                for a in web::ASSETS {
                    let target = dir.join(a.path.trim_start_matches('/'));
                    if let Some(parent) = target.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    export::write_atomic(&target, a.body)?;
                }
                export::write_atomic(&dir.join("sw.js"), web::service_worker().as_bytes())?;
                export::write_atomic(&dir.join("topology.json"), &json)?;
            }
            match out {
                Some(p) => export::write_atomic(&p, &json)?,
                None if web_written => {}
                None => {
                    use std::io::Write;
                    std::io::stdout().write_all(&json)?;
                }
            }
        }
        Cmd::Diagnose { ip, no_probe, member, json } => {
            let topo = fetch_topology(&cfg)?;
            let mut st = state::State::load();
            let rep = collect::run(&cfg, &topo, &st, ip, &collect::Options { probe: !no_probe, member, infra: None });
            st.update(&topo);
            if let Err(e) = st.save() {
                eprintln!("warning: cannot save state: {}", report::sanitize(&format!("{e:#}")));
            }
            if json {
                println!("{}", serde_json::to_string_pretty(&rep)?);
            } else {
                print!("{}", report::text(&rep));
            }
        }
    }
    Ok(())
}

/// Seconds since the Unix epoch.
pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ospf6_failure_keeps_last_prefixes_and_backs_off() {
        let p = ospf::Prefix6Owner { addr: "2a01:168:1::".parse().unwrap(), len: 48, router: Ipv4Addr::new(10, 107, 1, 1), kind: ospf::PrefixKind::Stub };
        let mut st = Ospf6State { failed_at: None, last_good: Vec::new() };
        let t0 = Instant::now();
        assert_eq!(prefixes6_with_backoff(&mut st, t0, || Ok(vec![p.clone()])), vec![p.clone()]);
        assert_eq!(prefixes6_with_backoff(&mut st, t0, || anyhow::bail!("down")), vec![p.clone()]);
        let mut called = false;
        prefixes6_with_backoff(&mut st, t0 + Duration::from_secs(60), || {
            called = true;
            Ok(Vec::new())
        });
        assert!(!called, "no retry within the backoff");
        assert!(prefixes6_with_backoff(&mut st, t0 + OSPF6_BACKOFF, || Ok(Vec::new())).is_empty());
    }
}
