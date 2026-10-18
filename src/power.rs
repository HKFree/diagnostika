//! The hkfree.org power monitor (power.hkfree.org): which APs run on battery
//! and which power cuts ČEZ announced at their addresses. Read by the server
//! every `poll_secs` for the outage overview; the app never calls it.

use crate::config::PowerConfig;
use crate::http;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::time::Duration;

/// An AP as the power monitor names it (its UserDB id, name and area).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApRef {
    pub id: u32,
    pub name: String,
    pub area: String,
}

/// A backup power source without mains power.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OnBattery {
    pub ap: Option<ApRef>,
    pub on_battery: bool,
    pub battery_percent: Option<u32>,
    /// When the mains power went (ISO 8601), if known.
    pub since: Option<String>,
}

/// A power cut ČEZ announced (planned or ongoing) at an AP's address.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PowerCut {
    pub ongoing: bool,
    pub from: Option<String>,
    pub to: Option<String>,
    /// ČEZ's own wording of the time, e.g. "20.10.2026 07:30 - 16:30".
    pub when: String,
    pub aps: Vec<ApRef>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct PowerState {
    pub on_battery: Vec<OnBattery>,
    pub cuts: Vec<PowerCut>,
}

fn text(v: &Value, max: usize) -> String {
    v.as_str().unwrap_or_default().chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string()
}

/// An ISO 8601 time as the monitor writes it; anything else is dropped.
fn time(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    (s.len() <= 40 && s.chars().all(|c| c.is_ascii_digit() || "-:.TZ+".contains(c))).then(|| s.to_string())
}

fn ap_ref(v: &Value) -> Option<ApRef> {
    Some(ApRef { id: u32::try_from(v["id"].as_u64()?).ok()?, name: text(&v["name"], 100), area: text(&v["area"], 100) })
}

/// The parts of a `vypadky.json` answer the overview needs.
pub fn parse(v: &Value) -> PowerState {
    let rows = |k: &str| v[k].as_array().cloned().unwrap_or_default();
    let on_battery = rows("sources")
        .iter()
        .filter(|s| s["onBattery"].as_bool() == Some(true) || s["mainsOk"].as_bool() == Some(false))
        .take(500)
        .map(|s| OnBattery {
            ap: ap_ref(&s["ap"]),
            on_battery: s["onBattery"].as_bool() == Some(true),
            battery_percent: s["batteryPercent"].as_u64().and_then(|p| u32::try_from(p.min(100)).ok()),
            since: time(&s["outageSince"]),
        })
        .collect();
    let cuts = rows("cez")
        .iter()
        .filter(|c| c["isTest"].as_bool() != Some(true))
        .take(500)
        .map(|c| PowerCut {
            ongoing: c["status"].as_str() == Some("ongoing"),
            from: time(&c["from"]),
            to: time(&c["to"]),
            when: text(&c["dateText"], 80),
            aps: {
                // One AP can have several sources at the affected addresses.
                let mut aps: Vec<ApRef> = Vec::new();
                for a in c["sources"].as_array().into_iter().flatten().filter_map(|x| ap_ref(&x["ap"])) {
                    if aps.len() < 50 && !aps.iter().any(|b| b.id == a.id) {
                        aps.push(a);
                    }
                }
                aps
            },
        })
        .collect();
    PowerState { on_battery, cuts }
}

pub fn fetch(cfg: &PowerConfig) -> Result<PowerState> {
    let agent = http::agent(Duration::from_secs(15));
    let resp = agent.get(&cfg.url).call().context("power monitor request failed")?;
    let body = http::read_limited(resp, 4 * 1024 * 1024)?;
    let v: Value = serde_json::from_str(&body).context("power monitor: invalid JSON")?;
    if v["ok"].as_bool() != Some(true) {
        anyhow::bail!("power monitor: answer not ok");
    }
    Ok(parse(&v))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_sources_without_mains_and_announced_cuts() {
        let v: Value = serde_json::from_str(r#"{"ok":true,"sources":[
            {"id":1,"name":"ups-a","ap":{"id":10,"name":"Podůlšany","areaId":5,"area":"Podůlšany","matchedBy":"ip"},"online":true,"mainsOk":false,"onBattery":true,"batteryPercent":73,"outageSince":"2026-10-18T08:15:00.000Z"},
            {"id":2,"name":"ups-b","ap":{"id":11,"name":"X","areaId":5,"area":"Y","matchedBy":"ip"},"online":true,"mainsOk":true,"onBattery":false,"batteryPercent":100,"outageSince":null},
            {"id":3,"name":"ups-c","ap":null,"online":false,"mainsOk":null,"onBattery":false,"batteryPercent":null,"outageSince":"<script>"}],
          "cez":[
            {"status":"planned","isTest":false,"from":"2026-10-20T05:30:00.000Z","to":"2026-10-20T14:30:00.000Z","dateText":"20.10.2026 07:30 - 16:30","sources":[{"id":1,"ap":{"id":10,"name":"Podůlšany","area":"Podůlšany"}},{"id":7,"ap":{"id":10,"name":"Podůlšany","area":"Podůlšany"}}]},
            {"status":"planned","isTest":true,"from":null,"to":null,"dateText":"test","sources":[]}]}"#).unwrap();
        let p = parse(&v);
        assert_eq!(p.on_battery.len(), 1, "only sources without mains");
        assert_eq!(p.on_battery[0].ap.as_ref().unwrap().id, 10);
        assert_eq!((p.on_battery[0].battery_percent, p.on_battery[0].since.as_deref()), (Some(73), Some("2026-10-18T08:15:00.000Z")));
        assert_eq!(p.cuts.len(), 1, "test announcements are left out");
        assert_eq!((p.cuts[0].when.as_str(), p.cuts[0].aps.len(), p.cuts[0].aps[0].id, p.cuts[0].ongoing), ("20.10.2026 07:30 - 16:30", 1, 10, false));
    }
}
