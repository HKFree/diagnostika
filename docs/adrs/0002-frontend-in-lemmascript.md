# The frontend is written in LemmaScript

- Status: accepted
- Date: 2026-10-14
- Shaped views: `app`

## Context and decision

The app's verdicts decide what a member does next: restart their router, or
leave it alone because the backbone is down. Its filters decide what a member
sees. Wrong logic there misleads people at the worst moment. We write the
frontend in **LemmaScript**: TypeScript whose functions carry `//@` contracts
(requires, ensures, invariants) that `lsc` proves with Dafny. Contracts are
comments, so the code runs as ordinary TypeScript.

- Sources live in `web/src/*.ts`. `tsc` compiles them to `web/*.js`, which is
  what the browser gets and what the server embeds. The compiled JS is
  committed, so `cargo build` needs no Node.
- Pure logic (address and prefix handling, path relevance, verdicts, ratings)
  is marked `//@ verify` and proven. DOM, fetch, timers, the service worker
  and `await` can't be modelled; that code stays plain TypeScript in the same
  project.
- Proof hints go into the `.dfy` file next to the source, as additions to the
  generated `.dfy.gen` only.
- CI proves every file listed in `LemmaScript-files.txt` and fails if the
  compiled JS is out of date.
- Existing JavaScript (`engine.js`, `app.js`, …) moves over module by module;
  new frontend code starts in LemmaScript.

## Considered options

- **LemmaScript (chosen):** proofs for the logic that matters, in the
  language the frontend is already close to, with no runtime cost.
- **Plain JavaScript with tests:** what we had; tests only cover the cases we
  thought of.
- **TypeScript without contracts:** catches type errors, not wrong logic.

## Consequences

- Contributors need Node, TypeScript and, to change verified code, Dafny 4.
- LemmaScript is a tech preview (0.6.x): pinned exactly, upgraded deliberately.
- A build step exists for the frontend; forgetting it is caught by CI.
