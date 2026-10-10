//! Terminal and DOT rendering.
//!
//! Strings from UserDB / routers are untrusted: control characters are
//! stripped before printing so they cannot inject terminal escape sequences.

use crate::diagnose::{Report, Severity};
use crate::ospf::Topology;
use std::fmt::Write;

pub fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

fn dot_escape(s: &str) -> String {
    sanitize(s).replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn text(r: &Report) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "Target:   {}", r.target);
    if let Some(d) = &r.device {
        let _ = writeln!(
            o,
            "UserDB:   {} {}",
            sanitize(&d.hostname),
            sanitize(&d.description)
        );
    }
    match (r.prefix, r.router) {
        (Some(p), Some(rt)) => {
            let _ = writeln!(
                o,
                "Subnet:   {p} on router {rt}{}",
                r.router_name
                    .as_deref()
                    .map(|n| format!(" ({})", sanitize(n)))
                    .unwrap_or_default()
            );
        }
        _ => {
            let _ = writeln!(o, "Subnet:   not found in OSPF");
        }
    }
    if let Some(e) = &r.exit_description {
        let _ = writeln!(o, "Exit:     {}", sanitize(e));
    }
    if !r.path.is_empty() {
        let _ = writeln!(o, "\nPath to internet exit (OSPF):");
        for h in &r.path {
            let ping = match &h.probe {
                Some(p) if p.error.is_some() => "ping n/a".to_string(),
                Some(p) if p.reachable() => {
                    format!(
                        "ping ok{}",
                        p.avg_rtt_ms
                            .map(|v| format!(" {v:.1} ms"))
                            .unwrap_or_default()
                    )
                }
                Some(_) => "NO PING".into(),
                None => "-".into(),
            };
            let sojka = h.sojka.map(|s| format!("sojka {s:?}")).unwrap_or_default();
            let _ = writeln!(
                o,
                "  cost {:>5}  {:<15} {:<40} {:<16} {}",
                h.cost,
                h.router,
                sanitize(h.name.as_deref().unwrap_or("")),
                ping,
                sojka
            );
        }
    }
    let _ = writeln!(o);
    for f in &r.findings {
        let tag = match f.severity {
            Severity::Critical => "[CRIT]",
            Severity::Warning => "[WARN]",
            Severity::Info => "[info]",
        };
        let _ = writeln!(o, "{tag} {}", sanitize(&f.title));
        for d in &f.detail {
            let _ = writeln!(o, "       {}", sanitize(d));
        }
    }
    let _ = writeln!(o, "\nVerdict: {}", sanitize(&r.verdict));
    o
}

/// Graphviz rendering of the live backbone (a generated "cloud map").
pub fn dot(topo: &Topology, name: impl Fn(std::net::Ipv4Addr) -> Option<String>) -> String {
    let mut o = String::from(
        "graph hkfree {\n  overlap=false; splines=true;\n  node [shape=box, style=rounded];\n",
    );
    for r in topo.routers.values() {
        let label = match name(r.id) {
            Some(n) => format!("{}\\n{}", dot_escape(&n), r.id),
            None => r.id.to_string(),
        };
        let exit = topo.exits.iter().any(|e| e.router == r.id);
        let _ = writeln!(
            o,
            "  \"{}\" [label=\"{}\"{}];",
            r.id,
            label,
            if exit {
                ", color=red, penwidth=2"
            } else if r.edges.is_empty() {
                ", color=gray, fontcolor=gray"
            } else {
                ""
            }
        );
    }
    for r in topo.routers.values() {
        for e in &r.edges {
            // Undirected graph: print each adjacency once.
            if r.id < e.to {
                let _ = writeln!(o, "  \"{}\" -- \"{}\" [label=\"{}\"];", r.id, e.to, e.cost);
            }
        }
    }
    o.push_str("}\n");
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_escape_sequences() {
        assert_eq!(sanitize("AP\x1b]0;pwned\x07 1"), "AP?]0;pwned? 1");
        assert_eq!(dot_escape("a\"b\\"), "a\\\"b\\\\");
    }
}
