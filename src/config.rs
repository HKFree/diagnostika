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
        if let Some(u) = &self.url {
            if !u.starts_with("https://") || u.contains(char::is_whitespace) {
                bail!("support.url must be an https:// URL");
            }
        }
        if let Some(e) = &self.email {
            let ok = e.split_once('@').map_or(false, |(a, d)| !a.is_empty() && d.contains('.'))
                && !e.contains(|c: char| c.is_whitespace() || "<>\"'?&;,:/".contains(c));
            if !ok {
                bail!("support.email is not a plain e-mail address");
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
    #[serde(default = "default_listen")]
    pub listen: String,
    /// Peers whose X-Forwarded-For / X-Real-IP headers are believed.
    #[serde(default = "default_proxies")]
    pub trusted_proxies: Vec<IpAddr>,
    /// SQLite file for outage reports sent by the app
    /// (default: ~/.local/state/hkdiag/reports.sqlite).
    #[serde(default)]
    pub reports_db: Option<PathBuf>,
    /// How long one topology snapshot is reused.
    #[serde(default = "default_topology_cache")]
    pub topology_cache_secs: u64,
}

impl Default for WebConfig {
    fn default() -> Self {
        WebConfig {
            listen: default_listen(),
            trusted_proxies: default_proxies(),
            reports_db: None,
            topology_cache_secs: default_topology_cache(),
        }
    }
}

fn default_listen() -> String {
    "127.0.0.1:8087".into()
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
    /// ospf6d vty (OSPFv3, IPv6 prefixes); same password.
    #[serde(default = "default_ospf6_host")]
    pub ospf6_host: String,
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
            ospf6_host: default_ospf6_host(),
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
fn default_ospf6_host() -> String {
    "10.107.31.33".into()
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
            if !u.url.starts_with("https://") {
                bail!("userdb.url must use https:// (credentials are sent with each request)");
            }
            if u.api_key.is_empty() {
                bail!("userdb.api_key (or HKDIAG_USERDB_API_KEY) is required when [userdb] is configured");
            }
        }
        if let Some(w) = &self.web {
            if w.topology_cache_secs == 0 {
                bail!("web.topology_cache_secs must be at least 1");
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
        assert_eq!(w.listen, "127.0.0.1:8087");
    }

    #[test]
    fn support_contact_is_validated() {
        let ok: Config = toml::from_str("[support]\nurl = \"https://example.org/help\"\nemail = \"help@example.org\"\nphone = \"+420 123 456 789\"\n").unwrap();
        assert!(ok.validate().is_ok());
        for bad in ["url = \"javascript:alert(1)\"", "email = \"a@b.c?subject=x\"", "phone = \"123<script>\""] {
            let c: Config = toml::from_str(&format!("[support]\n{bad}\n")).unwrap();
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
