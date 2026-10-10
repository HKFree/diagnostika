//! UserDB API client (https://userdb.hkfree.org/userdb/api).
//!
//! Every endpoint, including the member `app/*` ones, requires HTTP basic auth
//! with an API key (`apikey<id>` / key). A member login (UID + password)
//! additionally yields a token for `app/get-membership`.

use crate::config::{Secret, UserDbConfig};
use crate::http;
use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::net::Ipv4Addr;
use std::time::Duration;

const MAX_BODY: u64 = 32 * 1024 * 1024;

pub struct UserDb<'a> {
    cfg: &'a UserDbConfig,
    agent: ureq::Agent,
    auth: Secret,
}

#[derive(Clone, Debug, Serialize)]
pub struct Ap {
    pub id: u32,
    pub name: String,
    pub area: String,
    /// Street address, e.g. "Podůlšany 1, 533 45 Podůlšany".
    pub address: String,
    pub admins: Vec<String>,
}

/// "Podůlšany 1, 533 45 Podůlšany" from the UserDB fields (postcode may be a number).
fn ap_address(ap: &Value) -> String {
    let psc = match &ap["psc"] {
        Value::Number(n) => n.to_string(),
        v => as_string(v),
    };
    let psc = if psc.len() == 5 && psc.chars().all(|c| c.is_ascii_digit()) {
        format!("{} {}", &psc[..3], &psc[3..])
    } else {
        psc
    };
    let town = [psc.as_str(), as_string(&ap["mesto"]).trim()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    [as_string(&ap["ulice_cp"]).trim().to_string(), town]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone, Debug, Serialize)]
pub struct Device {
    pub ip: Ipv4Addr,
    pub hostname: String,
    pub description: String,
    pub ap_id: Option<u32>,
    pub user_id: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Membership {
    pub uid: u32,
    pub kind: String,
}

/// PHP serialises an empty associative array as `[]` and a non-empty one as
/// an object; iterate over both uniformly.
fn values(v: &Value) -> Box<dyn Iterator<Item = &Value> + '_> {
    match v {
        Value::Object(m) => Box::new(m.values()),
        Value::Array(a) => Box::new(a.iter()),
        _ => Box::new(std::iter::empty()),
    }
}

fn as_u32(v: &Value) -> Option<u32> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn as_string(v: &Value) -> String {
    v.as_str().unwrap_or_default().to_string()
}

impl<'a> UserDb<'a> {
    pub fn new(cfg: &'a UserDbConfig) -> UserDb<'a> {
        let raw = format!("apikey{}:{}", cfg.api_key_id, cfg.api_key.expose());
        let auth = Secret::new(format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(raw.as_bytes())
        ));
        let mut raw = raw;
        zeroize::Zeroize::zeroize(&mut raw);
        UserDb {
            cfg,
            agent: http::agent(Duration::from_secs(cfg.timeout_secs)),
            auth,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.cfg.url.trim_end_matches('/'), path)
    }

    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<Value> {
        let mut req = self
            .agent
            .get(&self.url(path))
            .set("Authorization", self.auth.expose());
        for (k, v) in query {
            req = req.query(k, v);
        }
        Self::parse(req.call(), path)
    }

    fn parse(res: Result<ureq::Response, ureq::Error>, path: &str) -> Result<Value> {
        match res {
            Ok(r) => {
                let text = http::read_limited(r, MAX_BODY)?;
                serde_json::from_str(&text).with_context(|| format!("UserDB {path}: invalid JSON"))
            }
            Err(ureq::Error::Status(401, _)) => bail!("UserDB {path}: API key rejected (401)"),
            Err(ureq::Error::Status(403, _)) => {
                bail!("UserDB {path}: API key not allowed for this endpoint (403)")
            }
            Err(ureq::Error::Status(code, _)) => bail!("UserDB {path}: HTTP {code}"),
            // Don't print a transport error's Display: it includes the full URL,
            // and for app/get-membership that URL carries the member token.
            Err(ureq::Error::Transport(t)) => Err(anyhow!(
                "UserDB {path}: {} {}",
                t.kind(),
                t.message().unwrap_or_default()
            )),
        }
    }

    /// All APs with their area name and active area admins.
    pub fn aps(&self) -> Result<BTreeMap<u32, Ap>> {
        let v = self.get("areas", &[])?;
        let mut out = BTreeMap::new();
        for area in values(&v) {
            let area_name = as_string(&area["jmeno"]);
            let admins: Vec<String> = values(&area["admins"])
                .map(|a| {
                    let nick = as_string(&a["nick"]);
                    let role = as_string(&a["role"]);
                    let email = as_string(&a["email"]);
                    format!("{nick} <{email}> ({role})")
                })
                .collect();
            for ap in values(&area["aps"]) {
                if let Some(id) = as_u32(&ap["id"]) {
                    out.insert(
                        id,
                        Ap {
                            id,
                            name: as_string(&ap["jmeno"]),
                            area: area_name.clone(),
                            address: ap_address(ap),
                            admins: admins.clone(),
                        },
                    );
                }
            }
        }
        Ok(out)
    }

    /// All registered IP addresses (AP infrastructure and, with `users`, members' devices).
    /// `ap` limits the result to one AP (and its members' devices with `users`).
    pub fn devices(&self, users: bool, ap: Option<u32>) -> Result<BTreeMap<Ipv4Addr, Device>> {
        let ap_s = ap.map(|a| a.to_string());
        let mut q = vec![("uzivatele", if users { "1" } else { "0" })];
        if let Some(a) = &ap_s {
            q.push(("ap", a.as_str()));
        }
        let v = self.get("monitoring/get-zarizeni", &q)?;
        if v["result"].as_str() != Some("OK") {
            bail!("UserDB monitoring/get-zarizeni: {}", v["result"]);
        }
        let mut out = BTreeMap::new();
        if let Value::Object(m) = &v["zarizeni"] {
            for (ip, d) in m {
                let Ok(ip) = ip.parse::<Ipv4Addr>() else {
                    continue;
                };
                out.insert(
                    ip,
                    Device {
                        ip,
                        hostname: as_string(&d["hostname"]),
                        description: as_string(&d["popis"]),
                        ap_id: as_u32(&d["Ap_id"]),
                        user_id: as_u32(&d["Uzivatel_id"]),
                    },
                );
            }
        }
        Ok(out)
    }

    /// Member login: exchanges UID + password for a token and checks membership.
    /// Note: UserDB locks the source IP out after 5 failed attempts in 15 minutes,
    /// so this is never retried.
    pub fn membership(&self, uid: u32, password: &Secret) -> Result<Membership> {
        let uid_s = uid.to_string();
        let res = self
            .agent
            .post(&self.url("app/get-token"))
            .set("Authorization", self.auth.expose())
            .send_form(&[("uid", uid_s.as_str()), ("heslo", password.expose())]);
        let v = Self::parse(res, "app/get-token")?;
        if v["result"].as_str() != Some("OK") {
            bail!(
                "UserDB member login failed: {}",
                v["result"].as_str().unwrap_or("unknown error")
            );
        }
        let token = Secret::new(as_string(&v["token"]));
        let m = self.get(
            "app/get-membership",
            &[("uid", &uid_s), ("token", token.expose())],
        )?;
        if m["result"].as_str() != Some("OK") {
            bail!(
                "UserDB membership check failed: {}",
                m["result"].as_str().unwrap_or("unknown error")
            );
        }
        Ok(Membership {
            uid,
            kind: as_string(&m["clenstvi"]),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ap_address_formats_postcode() {
        let ap: Value =
            serde_json::from_str(r#"{"ulice_cp":"Podůlšany 1","mesto":"Podůlšany","psc":53345}"#)
                .unwrap();
        assert_eq!(ap_address(&ap), "Podůlšany 1, 533 45 Podůlšany");
        let partial: Value =
            serde_json::from_str(r#"{"ulice_cp":"","mesto":"Hradec Králové","psc":null}"#).unwrap();
        assert_eq!(ap_address(&partial), "Hradec Králové");
    }

    #[test]
    fn php_arrays_and_objects() {
        let v: Value = serde_json::from_str(r#"{"a":[],"b":{"1":{"id":"7"}}}"#).unwrap();
        assert_eq!(values(&v["a"]).count(), 0);
        let ids: Vec<_> = values(&v["b"]).filter_map(|x| as_u32(&x["id"])).collect();
        assert_eq!(ids, vec![7]);
    }
}
