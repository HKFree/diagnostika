//! Shared HTTP helpers: bounded timeouts, no redirects, bounded body size.

use anyhow::{bail, Context, Result};
use std::io::Read;
use std::time::Duration;

pub fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(timeout)
        .timeout(timeout)
        // Never follow redirects: they could carry basic-auth credentials
        // or downgrade https to http.
        .redirects(0)
        .user_agent(concat!("hkdiag/", env!("CARGO_PKG_VERSION")))
        .build()
}

pub fn read_limited(resp: ureq::Response, limit: u64) -> Result<String> {
    // With redirects disabled ureq hands 3xx back as a normal response.
    if (300..400).contains(&resp.status()) {
        bail!(
            "refusing to follow HTTP redirect ({}) to {}",
            resp.status(),
            resp.header("location").unwrap_or("?").chars().filter(|c| !c.is_control()).collect::<String>()
        );
    }
    let mut buf = Vec::new();
    resp.into_reader()
        .take(limit + 1)
        .read_to_end(&mut buf)
        .context("reading HTTP response")?;
    if buf.len() as u64 > limit {
        bail!("HTTP response larger than {limit} bytes");
    }
    String::from_utf8(buf).context("HTTP response is not UTF-8")
}
