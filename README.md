# hkdiag

*Česky: [README.cs.md](README.cs.md)*

Answers "why does my hkfree.org connection not work?". **The diagnosis runs on
the member's device**, in a PWA that installs on a phone and works offline,
which is exactly when it's needed. The UI is in Czech, or in English when the
browser prefers it.

| Kontrola | Výpadek | Výpadky | Pomoc |
|---|---|---|---|
| ![](docs/screenshots/11-cs-healthy-hkfree-phone.png) | ![](docs/screenshots/12-cs-outage-backbone-phone.png) | ![](docs/screenshots/14-cs-outages-tab-phone.png) | ![](docs/screenshots/15-cs-help-phone.png) |

Tabs:
- **Kontrola (Check):** verdict, what to do, and the probes.
- **Cesta (Path):** your OSPF path and findings.
- **Výpadky (Outages):** outages reported by members, public, without addresses.
- **Historie (History).**
- **Pomoc (Help):** an on-device assistant.

The header shows whether the device is on the hkfree.org network or another one.

## Specification, design, documentation

This project is developed the [NoVibe](https://novibe.org) way: specification
first, then design, then code proven by tests.

| What | Where |
|---|---|
| Specification (Gherkin, executable) | [`features/`](features/), Czech mirror in [`docs/spec/cs/`](docs/spec/cs/) |
| Architecture (LikeC4, arc42 views) | [`docs/architecture/`](docs/architecture/); view it with `npx likec4 serve docs/architecture` |
| Decisions | [`docs/adrs/`](docs/adrs/) |
| Glossary (cs/en) | [`GLOSSARY.md`](GLOSSARY.md) |
| Documentation (tutorials, how-to) | [`docs/`](docs/README.md), Czech in [`docs/cs/`](docs/cs/README.md) |

Commits belong to one phase (`spec:`, `arch:`, anything else), and git hooks
in `.githooks/` check them on push: spec lint, Czech mirror in sync, model
validation, phases, and no credential files. Claude Code sessions turn them
on by themselves. Without Claude, run `git config core.hooksPath .githooks`
once per clone, and `npm install` for the linters.

## Architecture

```
 FRR 10.107.3.1 (OSPF LSDB) ─┐
 Sojka pinger ───────────────┼─► hkdiag export / serve ─► topology.json ──┐
 UserDB (AP names, optional)─┘        (data only)                         │
                                                                          ▼
                         member's phone: PWA ── engine.js (OSPF analysis, on device)
                                             ── probes from the device itself
                                             ── verdict (cs / en), works offline
```

- **Server side:** only produces data. `hkdiag export` writes `topology.json`
  (routers, confirmed links, prefixes, internet exits, AP names, Sojka status,
  previously advertised subnets). Run it from cron and serve the file
  statically, or let `hkdiag serve` do both. The snapshot contains
  infrastructure only: no member devices, descriptions or contacts.
- **On the device** (`web/engine.js`, a port of the Rust engine that is checked
  against it on the same fixture and on the live network):
  1. locates the member's address by longest prefix (VPN/static routes more
     specific than /16 included, resolved through their forwarding address);
  2. finds the OSPF path to the internet exit (E1 before E2);
  3. detects a cut-off AP router, a partition without an exit, a subnet that
     disappeared (and which router used to advertise it), routers on the path
     that Sojka flags, links without backup, and a single remaining exit;
  4. probes from the device itself and combines everything into one verdict.

### Probes from the device

Browsers can't ping, so these checks are HTTPS fetches:

| Check | How | Reliability |
|---|---|---|
| hkfree.org server | `GET /api/whoami` (also returns your address) | reliable |
| Internet by IP | `https://1.1.1.1/…` (no DNS) | reliable |
| Internet by name | `google.com` / `cloudflare.com` | reliable; IP OK + name failing → DNS problem |
| Routers on your path | TCP timing to `https://<router>/`: a fast answer or refusal vs a timeout | heuristic, compared with a baseline recorded while everything worked |
| Control | `https://192.0.2.1/` (TEST-NET, exists nowhere) | detects "every connection fails instantly on this device" |

### Offline

The service worker caches the app as one consistent version per build. The
last snapshot, your address, the router baseline and the history are kept on
the device. With hkfree.org down, the app opens, analyses the saved snapshot
(shown with its age) and probes your path to say where it breaks: your device,
your AP router, or a backbone router.

### What to do, support contact

Every verdict comes with concrete steps: restart Wi-Fi or the router, check
the antenna's PoE adapter, or "the problem is in the network, don't restart
anything". When the problem is on the network's side, the app shows the
support contact from the server config:

```toml
[support]
name = "Podpora hkfree.org"
url = "https://…"        # https only
email = "…"
phone = "…"
```

### Network indicator

The header badge shows **hkfree.org**, **jiná síť** (other network, e.g.
mobile data) or **offline**. It's judged from the address the server sees,
against the snapshot. On another network the app does not probe routers (they
are unreachable from outside), doesn't learn baselines and doesn't report
outages.

### Outage reports

When a check finds a problem in the network (AP router, backbone, a known
network problem) rather than on the member's side or beyond the internet
exit, the app saves the outage on the device and re-checks every minute.
Once the device is back on hkfree.org it sends the outage to the server
(`POST /api/reports`), which stores it in SQLite.

- The **Výpadky** tab (`GET /api/outages`) shows the last 7 days to everyone:
  time, duration, AP and failing router, but **no member addresses**.
- `hkdiag reports [--hours N] [--json]` is the admin view, including member
  addresses and the most often failing routers.
- The server only accepts same-origin JSON. Every field is validated
  (allowlisted verdicts, IPv4 only, sane timestamps, bounded sizes). There are
  per-client limits, deduplication by report id, retention of 180 days at
  most and 200k rows at most. The database file is mode 600.

### Assistant (Pomoc / Help)

Answers "how do I diagnose and fix this myself" on the device, offline:

1. **Built-in guide** (`web/kb.js`, cs/en): about 14 articles (restarting the
   router and antenna, PoE, Wi-Fi vs cable, DHCP, DNS, a slow connection,
   signal, a replaced router and the MAC filter, …), searched offline. Always
   available.
2. **The browser's own model** (Chrome Prompt API) if it supports the language.
3. **A small open model run with WebGPU** (WebLLM, vendored in
   `web/vendor/web-llm`, Apache-2.0): Qwen2.5 0.5B (280 MB) or 1.5B (870 MB).
   It's downloaded only on an explicit tap, then cached for offline use, and
   can be removed again.

> **Agreed but not built yet:** the app gets all data only through our
> server, so the browser vendor's model (2.) goes away, and a model (3.) will
> come from hkfree.org instead of Hugging Face, in a later slice
> (`features/pomoc/poradce.feature`, `@backlog`). Until then the code still
> offers both.

Models get only the matching guide articles and the latest check result, are
told to answer only from them, and every answer lists its sources. Output is
rendered as plain text. The CSP adds only `'wasm-unsafe-eval'` (WebAssembly,
not JavaScript eval).

### Language

Czech is the default. English is used when the browser's language list
(`navigator.languages`) prefers English over Czech/Slovak. The CS/EN switch
in the header overrides this and is remembered on the device.

## Usage

```
hkdiag serve                       # PWA + /topology.json + /api/whoami
hkdiag export --out topology.json  # snapshot only (e.g. cron, every minute)
hkdiag export --web /var/www/diag  # complete static PWA + snapshot
hkdiag diagnose <IPv4> [--no-probe] [--member] [--json]   # server-side CLI diagnosis
hkdiag reports [--hours 168] [--json]                      # outage reports (admin)
hkdiag topology [--dot]            # live backbone; `--dot | neato -Tsvg > map.svg`
```

Locally, `serve` listens on `127.0.0.1:8087`. Localhost counts as a secure
context, so the PWA (including offline mode) works without HTTPS. From
localhost the server can't tell your hkfree.org address, so the app asks for it.

### Deployment on lide (current)

It runs as a systemd user service of `jrk` on lide.hkfree.org and listens on
`10.107.252.72:9000`, the hkfree.org address only (not the public 89.248.240.8):

```
~/hkdiag/bin/hkdiag                        # binary
~/hkdiag/config.toml                       # config (mode 600)
~/hkdiag/data/                             # prefix history, reports.sqlite
~/.local/share/systemd/user/hkdiag.service # unit (linked into default.target.wants)
systemctl --user restart hkdiag            # after copying a new binary
```

The PWA's offline mode needs HTTPS; for phones, put an HTTPS vhost in front
(see below).

### Deployment with HTTPS

A phone needs HTTPS for the PWA. Either keep `hkdiag serve` on localhost
behind the existing web server:

```nginx
server {
    listen 443 ssl;
    server_name diag.hkfree.org;          # own vhost: the app lives at /
    # ssl_certificate ... (existing certificate setup)
    location / {
        proxy_pass http://127.0.0.1:8087;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

or serve it fully statically: run `hkdiag export --web /var/www/diag` from
cron and add the one dynamic endpoint in nginx:

```nginx
location = /api/whoami {
    default_type application/json;
    return 200 '{"ip":"$remote_addr"}';
}
```

The app uses absolute paths (`/app.js`, `/topology.json`), so it needs its
own vhost root. Consider restricting the vhost to hkfree.org address space: the
snapshot describes the network's infrastructure.

## Install

Builds with Rust 1.75+. `Cargo.lock` pins dependency versions that work with
1.75; don't run `cargo update` with an old toolchain.

```
cargo build --release
install -m 755 target/release/hkdiag ~/bin/
install -m 600 -D config.example.toml ~/.config/hkdiag/config.toml   # then edit
```

Tests: `cargo test` (Rust) and `node --test web/test/engine.test.mjs`
(engine and translations). If the snapshot format changes, regenerate the
shared fixture with `HKDIAG_BLESS=1 cargo test`.

## Security notes

- **Secrets** (FRR password, API key, member password) come only from the
  config file or `HKDIAG_*` environment variables, never from argv (visible in
  `ps` on a shared host). The config file must be mode 600 and owned by you.
  Secrets are redacted in debug output and zeroed on drop.
- **FRR:** read-only. Only three hard-coded `show ... json` commands can be
  sent, so no user input reaches the router CLI. Output is capped and every
  read has a deadline. The vty protocol is telnet, so the password crosses the
  network in clear text; use a dedicated read-only vty password.
- **UserDB:** https only (enforced), redirects disabled so basic-auth
  credentials can't be forwarded, response bodies capped. `app/get-token` is
  never retried (UserDB locks an IP out after 5 failures).
- **Web server:** GET/HEAD only, except `POST /api/reports` (same-origin
  JSON, validated, limited per client). Assets are compiled in (no filesystem
  access), and the server does no work on a client's behalf: the snapshot is rebuilt at most
  every `topology_cache_secs`, whatever the request rate, with backoff when FRR
  fails. `X-Real-IP` is honoured only from `trusted_proxies`, and
  `X-Forwarded-For` is ignored. Internal errors go to the server log only.
  Strict CSP (`default-src 'none'`, scripts/styles from self), `nosniff`,
  `no-referrer`, `frame-ancestors 'none'`.
- **PWA:** the snapshot is validated before use or caching (malformed or
  hostile data is rejected). No `innerHTML`: all data is inserted as text.
  Only validated IPv4 addresses go into probe URLs, and probes send no
  credentials or referrer.
- **CLI output:** control characters from remote data are stripped (no
  terminal escape injection).
- `#![forbid(unsafe_code)]`. Locked dependencies checked against OSV/RustSec
  advisories.
