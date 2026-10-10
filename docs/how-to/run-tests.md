# How to run the tests and refresh fixtures

```
cargo test                                  # Rust: OSPF model, config, server, reports, …
node --test web/test/engine.test.mjs       # JS: on-device engine, i18n, guide search
```

(Pass the test file explicitly; Node 22 doesn't accept a directory here.)

## The shared fixture

Both engines are tested on the same data: `tests/fixtures/snapshot.json` is
generated from the synthetic OSPF database in `tests/fixtures/*.json` by the
Rust test `export::tests::golden_snapshot_is_current`. If you change the
snapshot format or the fixture, regenerate it:

```
HKDIAG_BLESS=1 cargo test golden
```

Then run both suites. The JS tests must agree with the Rust ones (same path,
same single points of failure, same findings).

## Checking Rust and JS against the live network

With access to FRR, export a live snapshot and compare paths for a list of
addresses:

```
hkdiag export --out /tmp/topology.json
hkdiag diagnose --no-probe --json 10.107.77.5 | jq -r '[.router] + [.path[].router] | join(">")'
```

and the same address through `analyze()` from `web/engine.js` in Node.

## Dependencies

There's no `cargo audit` on the build machine. Check the locked crates
against the OSV database (it includes RustSec) by sending the name and version
pairs from `Cargo.lock` to `https://api.osv.dev/v1/querybatch`.
