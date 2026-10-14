//! The PWA's static files, compiled into the binary.

pub struct Asset {
    pub path: &'static str,
    pub mime: &'static str,
    pub body: &'static [u8],
}

pub const ASSETS: &[Asset] = &[
    Asset { path: "/index.html", mime: "text/html; charset=utf-8", body: include_bytes!("../web/index.html") },
    Asset { path: "/app.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/app.js") },
    Asset { path: "/i18n.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/i18n.js") },
    Asset { path: "/kb.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/kb.js") },
    Asset { path: "/assistant.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/assistant.js") },
    Asset { path: "/speedtest.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/speedtest.js") },
    Asset { path: "/outages.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/outages.js") },
    Asset { path: "/verdict.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/verdict.js") },
    // WebLLM (Apache-2.0, see web/vendor/web-llm/), loaded only when a member opts into a model.
    Asset {
        path: "/vendor/web-llm/index.js",
        mime: "text/javascript; charset=utf-8",
        body: include_bytes!("../web/vendor/web-llm/index.js"),
    },
    Asset { path: "/engine.js", mime: "text/javascript; charset=utf-8", body: include_bytes!("../web/engine.js") },
    Asset { path: "/app.css", mime: "text/css; charset=utf-8", body: include_bytes!("../web/app.css") },
    Asset {
        path: "/manifest.webmanifest",
        mime: "application/manifest+json",
        body: include_bytes!("../web/manifest.webmanifest"),
    },
    Asset { path: "/icon.svg", mime: "image/svg+xml", body: include_bytes!("../web/icon.svg") },
    Asset { path: "/icon-192.png", mime: "image/png", body: include_bytes!("../web/icon-192.png") },
    Asset { path: "/icon-512.png", mime: "image/png", body: include_bytes!("../web/icon-512.png") },
];

/// The service worker; `__BUILD__` is replaced by a hash of all assets so that
/// any change to the app makes browsers install a fresh, consistent cache.
const SERVICE_WORKER: &str = include_str!("../web/sw.js");

/// The CSP: scripts and styles from this origin only. `connect-src https:`
/// is needed for the on-device probes and the optional model download;
/// `'wasm-unsafe-eval'` lets the optional on-device model compile its
/// WebAssembly (it does not allow JavaScript eval).
pub const CSP: &str = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self'; \
    manifest-src 'self'; worker-src 'self'; connect-src 'self' https:; \
    frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

fn fnv(bytes: impl Iterator<Item = u8>) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// Changes with any app asset (the vendored library is versioned separately).
pub fn build_id() -> String {
    let h = fnv(ASSETS.iter().filter(|a| !a.path.starts_with("/vendor/")).flat_map(|a| a.path.bytes().chain(a.body.iter().copied())));
    format!("{}-{h:016x}", env!("CARGO_PKG_VERSION"))
}

/// Changes only when the vendored library changes.
pub fn vendor_id() -> String {
    let h = fnv(ASSETS.iter().filter(|a| a.path.starts_with("/vendor/")).flat_map(|a| a.body.iter().copied()));
    format!("{h:016x}")
}

pub fn service_worker() -> String {
    SERVICE_WORKER.replace("__BUILD__", &build_id()).replace("__VENDOR__", &vendor_id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assets_embedded_and_sw_versioned() {
        assert!(ASSETS.iter().all(|a| !a.body.is_empty()));
        assert!(SERVICE_WORKER.contains("__BUILD__"));
        assert!(!service_worker().contains("__BUILD__"));
        assert!(SERVICE_WORKER.contains("__VENDOR__"));
        assert!(!service_worker().contains("__VENDOR__"));
    }
}
