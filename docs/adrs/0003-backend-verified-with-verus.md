# The backend's core logic is verified with Verus

- Status: accepted
- Date: 2026-10-14
- Shaped views: `backend`

## Context and decision

The server decides who a request comes from, which member's data it may
see, which files it may serve, and how OSPF records from several routers
combine into the map. Mistakes there leak data or show members the wrong
network. As with the frontend ([ADR 0002](0002-frontend-in-lemmascript.md)),
we prove the logic that matters: the backend is Rust checked by **Verus**.

- Verified code lives in `verus! { }` blocks with `requires`/`ensures`
  contracts and proofs; a plain `cargo build` compiles it with the proofs
  erased. `cargo verus verify` checks them.
- Pure logic goes there first: address and prefix arithmetic, the
  client-address choice behind a proxy, path checks for served files, OSPF
  sequence ordering and newest-record merging. I/O (sockets, HTTP, SQLite,
  the vty) stays plain Rust in the same crate.
- The toolchain is the one Verus needs, pinned in `rust-toolchain.toml`
  (Rust 1.98.1 with Verus 0.2026.09.27); the earlier Rust 1.75 pin is
  dropped. `vstd` is pinned to the matching release.
- CI verifies on every push, beside the normal tests.

## Considered options

- **Verus (chosen):** proofs in Rust itself, for the code we run.
- **Kani (bounded model checking):** checks small input ranges, no general
  proofs.
- **Tests only:** what we had; they cover the cases we thought of.

## Consequences

- Building needs Rust 1.98.1; verifying also needs the Verus release.
- Verus is young and released weekly: pinned exactly, upgraded deliberately,
  together with `vstd`.
- Existing modules move into `verus!` piece by piece; new core logic starts
  there.
