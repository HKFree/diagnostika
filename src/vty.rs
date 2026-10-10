//! Minimal read-only client for the FRR/Quagga vty (telnet) interface.
//!
//! Only commands from the fixed [`Command`] allowlist can be sent, so no user
//! input ever reaches the router CLI.

use crate::config::{FrrConfig, Secret};
use anyhow::{anyhow, bail, Context, Result};
use std::io::{Read, Write};
use zeroize::Zeroize;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

/// Upper bound for a single command's output (full LSDB is a few MiB).
const MAX_RESPONSE: usize = 64 * 1024 * 1024;

const IAC: u8 = 255;
const SB: u8 = 250;
const SE: u8 = 240;

#[derive(Clone, Copy, Debug)]
pub enum Command {
    RouterLsaJson,
    NetworkLsaJson,
    ExternalLsaJson,
}

impl Command {
    fn as_str(self) -> &'static str {
        match self {
            Command::RouterLsaJson => "show ip ospf database router json",
            Command::NetworkLsaJson => "show ip ospf database network json",
            Command::ExternalLsaJson => "show ip ospf database external json",
        }
    }
}

pub struct Vty {
    stream: TcpStream,
    timeout: Duration,
    prompt: Vec<u8>,
    telnet: TelnetFilter,
}

impl Vty {
    pub fn connect(cfg: &FrrConfig, password: &Secret) -> Result<Vty> {
        let timeout = Duration::from_secs(cfg.timeout_secs);
        let addr = (cfg.host.as_str(), cfg.port)
            .to_socket_addrs()
            .with_context(|| format!("cannot resolve {}", cfg.host))?
            .next()
            .ok_or_else(|| anyhow!("no address for {}", cfg.host))?;
        let stream = TcpStream::connect_timeout(&addr, timeout)
            .with_context(|| format!("cannot connect to FRR vty {addr}"))?;
        // Short socket timeout so the overall deadline in read_until is honoured.
        stream.set_read_timeout(Some(timeout.min(Duration::from_secs(1))))?;
        stream.set_write_timeout(Some(timeout))?;
        let mut vty = Vty {
            stream,
            timeout,
            prompt: Vec::new(),
            telnet: TelnetFilter::default(),
        };
        vty.login(password)?;
        vty.run_raw("terminal length 0")?;
        Ok(vty)
    }

    fn login(&mut self, password: &Secret) -> Result<()> {
        let banner = self.read_until(|b| {
            let t = trim_end(b);
            t.ends_with(b"Password:") || t.ends_with(b">") || t.ends_with(b"#")
        })?;
        if !ends_with_trimmed(&banner, b"Password:") {
            bail!("FRR vty did not ask for a password");
        }
        let mut line = Vec::with_capacity(password.expose().len() + 1);
        line.extend_from_slice(password.expose().as_bytes());
        line.push(b'\n');
        let res = self.stream.write_all(&line);
        line.zeroize();
        res?;
        let out = self.read_until(|b| {
            let t = trim_end(b);
            t.ends_with(b">") || t.ends_with(b"#") || t.ends_with(b"Password:")
        })?;
        let t = trim_end(&out);
        if t.ends_with(b"Password:") || contains(&out, b"Bad passwords") {
            bail!("FRR vty rejected the password");
        }
        // The prompt is the last line, e.g. "d-network>".
        let start = t.iter().rposition(|&c| c == b'\n').map_or(0, |i| i + 1);
        let prompt = t[start..].to_vec();
        if prompt.is_empty() || prompt.len() > 128 {
            bail!("could not detect FRR vty prompt");
        }
        self.prompt = prompt;
        Ok(())
    }

    pub fn run(&mut self, cmd: Command) -> Result<String> {
        self.run_raw(cmd.as_str())
    }

    fn run_raw(&mut self, cmd: &str) -> Result<String> {
        self.stream.write_all(cmd.as_bytes())?;
        self.stream.write_all(b"\n")?;
        let prompt = self.prompt.clone();
        let out = self.read_until(|b| {
            let t = trim_end(b);
            t.ends_with(&prompt) && (t.len() == prompt.len() || t[t.len() - prompt.len() - 1] == b'\n')
        })?;
        let text = String::from_utf8_lossy(&out).replace('\r', "");
        // Drop the trailing prompt line and the echoed command, if echoed.
        let body = text.rsplit_once('\n').map_or("", |(b, _)| b);
        let body = match body.split_once('\n') {
            Some((first, rest)) if first.trim() == cmd => rest,
            None if body.trim() == cmd => "",
            _ => body,
        };
        if body.trim_start().starts_with('%') {
            bail!("FRR error for '{cmd}': {}", body.trim());
        }
        Ok(body.to_string())
    }

    fn read_until(&mut self, done: impl Fn(&[u8]) -> bool) -> Result<Vec<u8>> {
        let deadline = Instant::now() + self.timeout;
        let mut out = Vec::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            if Instant::now() > deadline {
                bail!("timeout waiting for FRR vty");
            }
            match self.stream.read(&mut buf) {
                Ok(0) => bail!("FRR vty closed the connection"),
                Ok(n) => {
                    self.telnet.filter(&buf[..n], &mut out);
                    if out.len() > MAX_RESPONSE {
                        bail!("FRR vty response exceeds {MAX_RESPONSE} bytes");
                    }
                    if done(&out) {
                        return Ok(out);
                    }
                }
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                Err(e) => return Err(e).context("reading from FRR vty"),
            }
        }
    }
}

impl Drop for Vty {
    fn drop(&mut self) {
        let _ = self.stream.write_all(b"exit\n");
    }
}

/// Strips telnet negotiation (IAC sequences) from the byte stream; state is
/// kept across reads because a sequence can be split between packets.
#[derive(Default)]
struct TelnetFilter {
    state: TState,
}

#[derive(Default, Clone, Copy, PartialEq)]
enum TState {
    #[default]
    Data,
    Iac,
    Opt,
    Sub,
    SubIac,
}

impl TelnetFilter {
    fn filter(&mut self, input: &[u8], out: &mut Vec<u8>) {
        for &b in input {
            self.state = match (self.state, b) {
                (TState::Data, IAC) => TState::Iac,
                (TState::Data, _) => {
                    out.push(b);
                    TState::Data
                }
                (TState::Iac, IAC) => {
                    out.push(IAC);
                    TState::Data
                }
                (TState::Iac, 251..=254) => TState::Opt,
                (TState::Iac, SB) => TState::Sub,
                (TState::Iac, _) => TState::Data,
                (TState::Opt, _) => TState::Data,
                (TState::Sub, IAC) => TState::SubIac,
                (TState::Sub, _) => TState::Sub,
                (TState::SubIac, SE) => TState::Data,
                (TState::SubIac, _) => TState::Sub,
            };
        }
    }
}

fn trim_end(b: &[u8]) -> &[u8] {
    let end = b.iter().rposition(|c| !c.is_ascii_whitespace()).map_or(0, |i| i + 1);
    &b[..end]
}

fn ends_with_trimmed(b: &[u8], suffix: &[u8]) -> bool {
    trim_end(b).ends_with(suffix)
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_telnet_negotiation_split_across_reads() {
        let mut f = TelnetFilter::default();
        let mut out = Vec::new();
        f.filter(&[IAC, 251], &mut out);
        f.filter(&[1, b'H', IAC, SB, 24, 1, IAC], &mut out);
        f.filter(&[SE, b'i', IAC, IAC], &mut out);
        assert_eq!(out, vec![b'H', b'i', IAC]);
    }
}
