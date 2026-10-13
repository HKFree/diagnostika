//! Configuration loading and secret handling.
//!
//! Secrets (FRR vty password, UserDB API key, member password) are never taken
//! from the command line (visible in `ps` to every user on a shared host such as
//! lide.hkfree.org). They come from a config file that must not be readable by
//! group/others, or from environment variables.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::fmt;
use std::net::IpAddr;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use zeroize::Zeroize;

/// A string that is never printed and is wiped from memory on drop.
#[derive(Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    pub fn new(s: String) -> Self {
        Secret(s)
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub frr: FrrConfig,
    #[serde(default)]
    pub sojka: SojkaConfig,
    #[serde(default)]
    pub userdb: Option<UserDbConfig>,
    #[serde(default)]
    pub web: Option<WebConfig>,
    /// Shown in the app when the problem is on the network's side.
    #[serde(default)]
    pub support: Option<SupportConfig>,
    /// MikroTik routers (RouterOS 7, REST API) whose OSPF databases are read
    /// beside FRR's and merged into the map.
    #[serde(default)]
    pub mikrotik: Vec<MikrotikConfig>,
    /// The speed test server the app measures against (on the member's request).
    #[serde(default)]
    pub speedtest: Option<SpeedtestConfig>,
    #[serde(default)]
    pub network: NetworkConfig,
    /// Where the app sends members for older outages and history.
    #[serde(default)]
    pub links: LinksConfig,
}

#[derive(Clone, Debug, Default, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinksConfig {
    /// An AP's page in UserDB, with "{id}" for the AP's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userdb_ap: Option<String>,
    /// UserDB's network status page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub userdb_status: Option<String>,
    /// Network monitoring's history (Sojka).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitoring: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NetworkConfig {
    /// Address pools of the hkfree.org VPN, e.g. ["10.107.254.0/24",
    /// "2a01:168:0:dead::/64"]: a device there is on hkfree.org, but not
    /// through its home connection.
    #[serde(default)]
    pub vpn: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedtestConfig {
    /// A LibreSpeed backend, e.g. "https://speedtest.hkfree.org/backend/".
    pub url: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MikrotikConfig {
    /// e.g. "http://10.107.10.129"; https:// once the router serves www-ssl.
    pub url: String,
    /// A read-only account (group "read").
    pub user: String,
    #[serde(default)]
    pub password: Secret,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// After a failure the router isn't asked again for this long.
    #[serde(default = "default_mikrotik_retry")]
    pub retry_after_secs: u64,
}

fn default_mikrotik_retry() -> u64 {
    300
}

#[derive(Clone, Debug, Default, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// https:// only
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    /// Phone opening hours on working days, e.g. "9-17".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone_hours: Option<String>,
    /// Support channel on the hkfree.org Slack, e.g. "#nejdeto".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slack_channel: Option<String>,
}

/// `http://` to this machine only (the tests' stand-in UserDB): the authority
/// must be exactly a loopback host with an optional port, no user info.
fn is_loopback_http(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("http://") else { return false };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let port_ok = |p: &str| p.is_empty() || p.strip_prefix(':').map_or(false, |n| n.parse::<u16>().is_ok());
    ["127.0.0.1", "localhost", "[::1]"].iter().any(|h| authority.strip_prefix(h).map_or(false, port_ok))
}

/// A plain e-mail address that is safe in a `mailto:` link (no headers, no markup).
pub fn plain_email(e: &str) -> bool {
    e.len() <= 200
        && e.split_once('@').map_or(false, |(a, d)| !a.is_empty() && d.contains('.') && !d.contains('@'))
        && !e.contains(|c: char| c.is_whitespace() || c.is_control() || "<>\"'?&;,:/()[]\\".contains(c))
}

impl SupportConfig {
    fn validate(&self) -> Result<()> {
        let short = |s: &Option<String>, what: &str| -> Result<()> {
            if let Some(v) = s {
                if v.len() > 200 || v.chars().any(|c| c.is_control()) {
                    bail!("support.{what} is too long or contains control characters");
                }
            }
            Ok(())
        };
        short(&self.name, "name")?;
        short(&self.url, "url")?;
        short(&self.email, "email")?;
        short(&self.phone, "phone")?;
        short(&self.phone_hours, "phone_hours")?;
        short(&self.slack_channel, "slack_channel")?;
        if let Some(u) = &self.url {
            if !u.starts_with("https://") || u.contains(char::is_whitespace) {
                bail!("support.url must be an https:// URL");
            }
        }
        if let Some(e) = &self.email {
            if !plain_email(e) {
                bail!("support.email is not a plain e-mail address");
            }
        }
        if let Some(h) = &self.phone_hours {
            let ok = h.split_once('-').map_or(false, |(a, b)| {
                let time = |t: &str| {
                    let (hh, mm) = t.trim().split_once(':').unwrap_or((t.trim(), "00"));
                    matches!((hh.parse::<u8>(), mm.parse::<u8>()), (Ok(h), Ok(m)) if h <= 24 && m < 60 && mm.len() == 2)
                };
                time(a) && time(b)
            });
            if !ok {
                bail!("support.phone_hours must look like \"9-17\" or \"8:30-16:30\"");
            }
        }
        if let Some(c) = &self.slack_channel {
            let name = c.strip_prefix('#').unwrap_or_default();
            if name.is_empty() || name.len() > 80 || !name.chars().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || "-_.".contains(x)) {
                bail!("support.slack_channel must look like \"#nejdeto\"");
            }
        }
        if let Some(p) = &self.phone {
            if !p.chars().all(|c| c.is_ascii_digit() || " +-()".contains(c)) {
                bail!("support.phone may contain only digits, spaces and + - ( )");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebConfig {
    /// Keep this on localhost and put an HTTPS reverse proxy in front.
    /// One address or several, e.g. ["127.0.0.1:9000", "10.107.252.72:9000"]:
    /// localhost for a reverse proxy on the same machine, the hkfree.org
    /// address for members directly.
    #[serde(default = "default_listen", deserialize_with = "one_or_many")]
    pub listen: Vec<String>,
    /// Peers whose client-address header (`client_ip_header`) is believed.
    #[serde(default = "default_proxies")]
    pub trusted_proxies: Vec<IpAddr>,
    /// Which header a trusted proxy puts the client's address in.
    #[serde(default)]
    pub client_ip_header: ClientIpHeader,
    /// Public origins the app is served under, e.g. ["https://diag.hkfree.org"]:
    /// pages there may send outage reports even when a proxy doesn't pass the
    /// original Host header on.
    #[serde(default)]
    pub origins: Vec<String>,
    /// Directory with the assistant's mirrored model files, served under
    /// /models/ (see docs/how-to/mirror-assistant-models.md).
    #[serde(default)]
    pub models_dir: Option<PathBuf>,
    /// SQLite file for outage reports sent by the app
    /// (default: ~/.local/state/hkdiag/reports.sqlite).
    #[serde(default)]
    pub reports_db: Option<PathBuf>,
    /// How long one topology snapshot is reused.
    #[serde(default = "default_topology_cache")]
    pub topology_cache_secs: u64,
    /// How often UserDB's status (devices with a problem now) is read, to show
    /// it and to record the outages network monitoring measures.
    #[serde(default = "default_status_poll")]
    pub status_poll_secs: u64,
    /// How long one AP's radio data is reused: reading it logs in to the AP's routers.
    #[serde(default = "default_signal_cache")]
    pub signal_cache_secs: u64,
}

impl Default for WebConfig {
    fn default() -> Self {
        WebConfig {
            listen: default_listen(),
            trusted_proxies: default_proxies(),
            client_ip_header: ClientIpHeader::default(),
            origins: Vec::new(),
            models_dir: None,
            reports_db: None,
            topology_cache_secs: default_topology_cache(),
            signal_cache_secs: default_signal_cache(),
            status_poll_secs: default_status_poll(),
        }
    }
}

/// How a trusted reverse proxy tells who the client is.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
pub enum ClientIpHeader {
    /// `X-Real-IP`, which the proxy must set (overwrite) itself.
    #[default]
    #[serde(rename = "x-real-ip")]
    XRealIp,
    /// The last entry of `X-Forwarded-For`: the one the proxy appended.
    /// Apache's mod_proxy does this without any configuration.
    #[serde(rename = "x-forwarded-for")]
    XForwardedFor,
}

fn default_status_poll() -> u64 {
    60
}
fn default_signal_cache() -> u64 {
    60
}
fn default_listen() -> Vec<String> {
    vec!["127.0.0.1:8087".into()]
}

/// `"a"` or `["a", "b"]`.
fn one_or_many<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }
    Ok(match OneOrMany::deserialize(d)? {
        OneOrMany::One(s) => vec![s],
        OneOrMany::Many(v) => v,
    })
}
fn default_proxies() -> Vec<IpAddr> {
    vec![IpAddr::from([127, 0, 0, 1]), IpAddr::from(std::net::Ipv6Addr::LOCALHOST)]
}
fn default_topology_cache() -> u64 {
    30
}


#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrrConfig {
    #[serde(default = "default_frr_host")]
    pub host: String,
    /// ospfd vty port
    #[serde(default = "default_frr_port")]
    pub port: u16,
    #[serde(default)]
    pub password: Secret,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// ospf6d vty (OSPFv3, IPv6 prefixes); same password. Without it there
    /// is no IPv6 data, so the password never goes to an unconfigured host.
    #[serde(default)]
    pub ospf6_host: Option<String>,
    #[serde(default = "default_ospf6_port")]
    pub ospf6_port: u16,
}

impl Default for FrrConfig {
    fn default() -> Self {
        FrrConfig {
            host: default_frr_host(),
            port: default_frr_port(),
            password: Secret::default(),
            timeout_secs: default_timeout(),
            ospf6_host: None,
            ospf6_port: default_ospf6_port(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SojkaConfig {
    #[serde(default = "default_sojka_url")]
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// IPs the pinger has not seen alive for longer than this are treated as
    /// stale/unknown rather than "down" (UserDB uses 7 days).
    #[serde(default = "default_lookback_secs")]
    pub lookback_secs: u64,
}

impl Default for SojkaConfig {
    fn default() -> Self {
        SojkaConfig {
            url: default_sojka_url(),
            timeout_secs: default_timeout(),
            lookback_secs: default_lookback_secs(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserDbConfig {
    #[serde(default = "default_userdb_url")]
    pub url: String,
    /// Numeric id of the API key (HTTP basic user is `apikey<id>`).
    pub api_key_id: u32,
    #[serde(default)]
    pub api_key: Secret,
    /// Optional member login (UID + password) for the `app/*` endpoints.
    #[serde(default)]
    pub uid: Option<u32>,
    #[serde(default)]
    pub password: Secret,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_frr_host() -> String {
    "10.107.3.1".into()
}
fn default_frr_port() -> u16 {
    2604
}
fn default_ospf6_port() -> u16 {
    2606
}
fn default_timeout() -> u64 {
    15
}
fn default_lookback_secs() -> u64 {
    7 * 24 * 3600
}
fn default_sojka_url() -> String {
    "http://sojka.hkfree.org/pinger/".into()
}
fn default_userdb_url() -> String {
    "https://userdb.hkfree.org/userdb/api".into()
}

pub fn default_path() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(x).join("hkdiag/config.toml"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/hkdiag/config.toml"))
}

/// Refuse config files that other local users could read or modify.
fn check_permissions(path: &Path) -> Result<()> {
    let meta = fs::metadata(path).with_context(|| format!("cannot stat {}", path.display()))?;
    if !meta.is_file() {
        bail!("{} is not a regular file", path.display());
    }
    let mode = meta.mode() & 0o777;
    if mode & 0o077 != 0 {
        bail!(
            "{} has mode {:o}; it may contain secrets, run: chmod 600 {}",
            path.display(),
            mode,
            path.display()
        );
    }
    if meta.uid() != effective_uid()? {
        bail!("{} is not owned by the current user", path.display());
    }
    Ok(())
}

/// Effective uid from /proc/self/status ("Uid: real effective saved fs").
/// Fails closed: without it the ownership check cannot be done.
fn effective_uid() -> Result<u32> {
    let status = fs::read_to_string("/proc/self/status").context("cannot read /proc/self/status")?;
    status
        .lines()
        .find_map(|l| l.strip_prefix("Uid:"))
        .and_then(|v| v.split_whitespace().nth(1)?.parse().ok())
        .context("cannot determine effective uid")
}

impl Config {
    pub fn load(explicit: Option<&Path>) -> Result<Config> {
        let path = explicit.map(Path::to_path_buf).or_else(default_path);
        let mut cfg = match path {
            Some(p) if p.exists() => {
                check_permissions(&p)?;
                let mut text = fs::read_to_string(&p)
                    .with_context(|| format!("cannot read {}", p.display()))?;
                // toml's Display quotes the offending line, which may hold a
                // secret; report only the message and position.
                let parsed = toml::from_str::<Config>(&text).map_err(|e| {
                    let line = e.span().map(|s| text[..s.start.min(text.len())].lines().count().max(1));
                    anyhow::anyhow!(
                        "invalid config {} (line {}): {}",
                        p.display(),
                        line.map_or_else(|| "?".into(), |l| l.to_string()),
                        e.message()
                    )
                });
                // The raw text holds every secret; wipe it.
                text.zeroize();
                parsed?
            }
            Some(p) if explicit.is_some() => bail!("config file {} does not exist", p.display()),
            _ => Config {
                frr: FrrConfig::default(),
                sojka: SojkaConfig::default(),
                userdb: None,
                web: None,
                support: None,
                mikrotik: Vec::new(),
                speedtest: None,
                network: NetworkConfig::default(),
                links: LinksConfig::default(),
            },
        };
        cfg.apply_env();
        cfg.validate()?;
        Ok(cfg)
    }

    fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("HKDIAG_FRR_PASSWORD") {
            self.frr.password = Secret::new(v);
        }
        if let Some(u) = self.userdb.as_mut() {
            if let Ok(v) = std::env::var("HKDIAG_USERDB_API_KEY") {
                u.api_key = Secret::new(v);
            }
            if let Ok(v) = std::env::var("HKDIAG_USERDB_PASSWORD") {
                u.password = Secret::new(v);
            }
        }
    }

    fn validate(&self) -> Result<()> {
        if let Some(s) = &self.support {
            s.validate()?;
        }
        if self.frr.host.is_empty() {
            bail!("frr.host must not be empty");
        }
        if !self.sojka.url.starts_with("http://") && !self.sojka.url.starts_with("https://") {
            bail!("sojka.url must be an http(s) URL");
        }
        if let Some(u) = &self.userdb {
            // Credentials are sent with every request, so plaintext HTTP is not allowed.
            // Plain http only where credentials can't leave the machine (tests).
            if !u.url.starts_with("https://") && !is_loopback_http(&u.url) {
                bail!("userdb.url must use https:// (credentials are sent with each request)");
            }
            if u.api_key.is_empty() {
                bail!("userdb.api_key (or HKDIAG_USERDB_API_KEY) is required when [userdb] is configured");
            }
        }
        if let Some(s) = &self.speedtest {
            let rest = s.url.strip_prefix("https://").unwrap_or_default();
            let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
            let ok = !host.is_empty()
                && host.chars().all(|c| c.is_ascii_alphanumeric() || ".-:".contains(c))
                && path.chars().all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
                && s.url.ends_with('/');
            if !ok {
                bail!("speedtest.url must be an https:// URL of a LibreSpeed backend ending with '/', e.g. \"https://speedtest.hkfree.org/backend/\"");
            }
        }
        for (what, url) in [("userdb_ap", &self.links.userdb_ap), ("userdb_status", &self.links.userdb_status), ("monitoring", &self.links.monitoring)] {
            let Some(u) = url else { continue };
            let rest = u.strip_prefix("https://").or_else(|| u.strip_prefix("http://")).unwrap_or_default();
            let host = rest.split('/').next().unwrap_or_default();
            let ok = !host.is_empty()
                && !host.contains('@')
                && u.len() <= 300
                && u.chars().all(|c| c.is_ascii_graphic() && !"\"'<>\\`".contains(c))
                && (what == "userdb_ap") == u.contains("{id}");
            if !ok {
                bail!("links.{what} must be an http(s) URL{}", if what == "userdb_ap" { " with {id} for the AP" } else { "" });
            }
        }
        for p in &self.network.vpn {
            let ok = p.split_once('/').map_or(false, |(a, l)| match (a.parse::<IpAddr>(), l.parse::<u8>()) {
                (Ok(IpAddr::V4(_)), Ok(l)) => l <= 32,
                (Ok(IpAddr::V6(_)), Ok(l)) => l <= 128,
                _ => false,
            });
            if !ok {
                bail!("network.vpn: \"{p}\" is not a prefix such as \"10.107.254.0/24\"");
            }
        }
        if self.mikrotik.len() > 50 {
            bail!("at most 50 [[mikrotik]] routers");
        }
        for m in &self.mikrotik {
            let host = m.url.strip_prefix("https://").or_else(|| m.url.strip_prefix("http://")).unwrap_or_default();
            let authority = host.split('/').next().unwrap_or_default();
            if authority.is_empty() || authority.contains('@') || m.url.contains(char::is_whitespace) {
                bail!("mikrotik.url must be an http(s):// URL without user info, e.g. \"http://10.107.10.129\"");
            }
            if m.user.is_empty() || m.password.is_empty() {
                bail!("mikrotik: user and password of a read-only account are required ({})", m.url);
            }
        }
        if let Some(w) = &self.web {
            if w.topology_cache_secs == 0 {
                bail!("web.topology_cache_secs must be at least 1");
            }
            for o in &w.origins {
                let host = o.strip_prefix("https://").or_else(|| o.strip_prefix("http://")).unwrap_or_default();
                if host.is_empty() || host.contains(['/', '@', '?', '#']) || o.contains(char::is_whitespace) {
                    bail!("web.origins: \"{o}\" must be a bare origin such as \"https://diag.hkfree.org\"");
                }
            }
            if w.listen.is_empty() || w.listen.len() > 8 {
                bail!("web.listen needs one to eight addresses");
            }
            for (i, a) in w.listen.iter().enumerate() {
                if a.parse::<std::net::SocketAddr>().is_err() {
                    bail!("web.listen: \"{a}\" is not an address:port such as \"127.0.0.1:9000\"");
                }
                if w.listen[..i].contains(a) {
                    bail!("web.listen: \"{a}\" is listed twice");
                }
            }
            if w.status_poll_secs == 0 {
                bail!("web.status_poll_secs must be at least 1");
            }
            if w.signal_cache_secs == 0 {
                bail!("web.signal_cache_secs must be at least 1");
            }
        }
        let userdb_timeout = self.userdb.as_ref().map(|u| u.timeout_secs);
        for t in [Some(self.frr.timeout_secs), Some(self.sojka.timeout_secs), userdb_timeout].into_iter().flatten() {
            if t == 0 || t > 300 {
                bail!("timeouts must be between 1 and 300 seconds");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_debug_is_redacted() {
        let s = Secret::new("hunter2".into());
        assert_eq!(format!("{:?}", s), "Secret(***)");
    }

    #[test]
    fn rejects_http_userdb() {
        let cfg: Config = toml::from_str(
            "[userdb]\nurl = \"http://userdb.hkfree.org/userdb/api\"\napi_key_id = 1\napi_key = \"x\"\n",
        )
        .unwrap();
        assert!(cfg.validate().is_err());
        let local: Config = toml::from_str(
            "[userdb]\nurl = \"http://127.0.0.1:8099/userdb/api\"\napi_key_id = 1\napi_key = \"x\"\n",
        )
        .unwrap();
        assert!(local.validate().is_ok());
        for sneaky in [
            "http://127.0.0.1.evil.example/userdb/api",
            "http://127.0.0.1:8099@evil.example/userdb/api",
            "http://localhost:x@evil.example/",
            "http://[::1]:80@evil.example/",
            "http://evil.example/?127.0.0.1:",
        ] {
            let c: Config =
                toml::from_str(&format!("[userdb]\nurl = \"{sneaky}\"\napi_key_id = 1\napi_key = \"x\"\n")).unwrap();
            assert!(c.validate().is_err(), "{sneaky}");
        }
        for ok in ["http://localhost:8099/api", "http://[::1]:8099/api", "http://127.0.0.1/api"] {
            assert!(is_loopback_http(ok), "{ok}");
        }
    }

    #[test]
    fn parse_errors_do_not_echo_secrets() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("hkdiag-test2-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.toml");
        fs::write(&p, "[frr]\npassword = \"s3cr3t-value\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        let err = format!("{:#}", Config::load(Some(&p)).unwrap_err());
        fs::remove_dir_all(&dir).unwrap();
        assert!(!err.contains("s3cr3t"), "{err}");
        assert!(err.contains("line 2"), "{err}");
    }

    #[test]
    fn parses_web_section() {
        let cfg: Config = toml::from_str("[web]\ntopology_cache_secs = 60\n").unwrap();
        let w = cfg.web.unwrap();
        assert_eq!(w.topology_cache_secs, 60);
        assert_eq!(w.listen, vec!["127.0.0.1:8087"]);
        let one: Config = toml::from_str("[web]\nlisten = \"10.107.252.72:9000\"\n").unwrap();
        assert_eq!(one.web.unwrap().listen, vec!["10.107.252.72:9000"]);
        let two: Config = toml::from_str("[web]\nlisten = [\"127.0.0.1:9000\", \"10.107.252.72:9000\"]\n").unwrap();
        assert!(two.validate().is_ok());
        assert_eq!(two.web.unwrap().listen.len(), 2);
        for bad in ["listen = []", "listen = [\"localhost\"]", "listen = [\"127.0.0.1:9000\", \"127.0.0.1:9000\"]"] {
            let c: Config = toml::from_str(&format!("[web]\n{bad}\n")).unwrap();
            assert!(c.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn support_contact_is_validated() {
        let ok: Config = toml::from_str("[support]\nurl = \"https://example.org/help\"\nemail = \"help@example.org\"\nphone = \"+420 123 456 789\"\nphone_hours = \"8:30-16\"\nslack_channel = \"#nejdeto\"\n").unwrap();
        assert!(ok.validate().is_ok());
        for bad in [
            "url = \"javascript:alert(1)\"",
            "email = \"a@b.c?subject=x\"",
            "phone = \"123<script>\"",
            "phone_hours = \"nonstop\"",
            "phone_hours = \"9-25\"",
            "slack_channel = \"nejdeto\"",
            "slack_channel = \"#<b>\"",
        ] {
            let c: Config = toml::from_str(&format!("[support]\n{bad}\n")).unwrap();
            assert!(c.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn speedtest_url_is_validated() {
        let ok: Config = toml::from_str("[speedtest]\nurl = \"https://speedtest.hkfree.org/backend/\"\n").unwrap();
        assert!(ok.validate().is_ok());
        for bad in ["http://speedtest.hkfree.org/backend/", "https://speedtest.hkfree.org/backend", "https://x/a?b/", "javascript:alert(1)//", "https://a@b/"] {
            let c: Config = toml::from_str(&format!("[speedtest]\nurl = \"{bad}\"\n")).unwrap();
            assert!(c.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn vpn_pools_are_validated() {
        let ok: Config = toml::from_str("[network]\nvpn = [\"10.107.254.0/24\", \"2a01:168:0:dead::/64\"]\n").unwrap();
        assert!(ok.validate().is_ok());
        for bad in ["10.107.254.0", "10.107.254.0/33", "vpn.hkfree.org/24", "2a01:168::/129"] {
            let c: Config = toml::from_str(&format!("[network]\nvpn = [\"{bad}\"]\n")).unwrap();
            assert!(c.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn links_are_validated() {
        let ok: Config = toml::from_str("[links]\nuserdb_ap = \"https://userdb.hkfree.org/userdb/ap/show/{id}\"\nmonitoring = \"http://sojka.hkfree.org/cgi-bin/smokeping.cgi\"\n").unwrap();
        assert!(ok.validate().is_ok());
        for bad in [
            "userdb_ap = \"https://userdb.hkfree.org/userdb/ap/show/\"",
            "monitoring = \"javascript:alert(1)\"",
            "userdb_status = \"https://a@evil.example/\"",
            "monitoring = \"https://x/\\\"onclick\"",
        ] {
            let c: Config = toml::from_str(&format!("[links]\n{bad}\n")).unwrap();
            assert!(c.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(toml::from_str::<Config>("[frr]\nhots = \"x\"\n").is_err());
    }

    #[test]
    fn rejects_world_readable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("hkdiag-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let p = dir.join("config.toml");
        fs::write(&p, "").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(check_permissions(&p).is_err());
        fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(check_permissions(&p).is_ok());
        fs::remove_dir_all(&dir).unwrap();
    }
}
