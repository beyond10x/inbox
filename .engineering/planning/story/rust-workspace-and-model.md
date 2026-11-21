---
format: aep.planning-md/3
id: story:rust-workspace-and-model
kind: story
status: active
title: Rust workspace and the model crate synthesized from `ess/`
summary: A Cargo workspace whose model, codecs and obligations crate is generated from the specification and held to it by the gate.
relations:
- decomposes: epic:service-in-memory
scope:
- confidence: inferred
  path: .github/workflows/check.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/inbox-xtask
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: rust-toolchain.toml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

The repository is a Cargo workspace whose members are the globs `crates/*` and
`generated/rust/*`, so a later story adds a crate without editing a shared manifest. `generated/rust/inbox` is the crate
`ess generate synthesize --target rust --layout crate` writes from `ess/`: the model types, the
behaviour of 7 of the 9 commands, the query of all 9 views, the storage and context ports they
run on, and, as its `server` feature, the route table, the JSON codecs of every type (the key file types included) and a
transport-free `handle`. On ESS 0.96.0 that is 85 of 87 capabilities; the 2 it leaves as obligations are the
behaviours of `PostMessage` and `Expire`. The crate is checked in whole and `task check` builds
it with `--features server`. `task generate` rewrites it and `task check`
fails when it differs from a fresh synthesis, and runs `cargo fmt --check`, `cargo clippy` and
`cargo test` for the workspace. `.github/workflows/check.yml` installs the pinned Rust toolchain.

## Acceptance

After an optional field is added to `ess/domains/messaging.yaml` and `task projections` has run,
`task check` fails with a drift line that names `generated/rust`, where before this story
`task check` passes in that state.

## Context

The model is never transcribed by hand (rule of epic:service-in-memory). The `a2a` repository of
the same organisation is the layout to copy: a small `xtask` crate runs the synthesis into a
scratch directory and compares it with `generated/rust`.

Whether the synthesized crate needs an `ess-http/1` binding (`ess/http.yaml`) is settled here:
stage 1 uses the routes ESS derives (`/messaging/commands/<wire>`, `/messaging/views/<wire>`) and
has no binding file.

## Scope

- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` (new). A later story's crate changes
  `Cargo.lock` only; a merge of two such stories regenerates it with `cargo metadata`.
- `generated/rust/` (new, generated)
- `crates/inbox-xtask/` (new)
- `Taskfile.yml`: tasks `generate`, `drift`; `check` gains them and the cargo steps
- `.github/workflows/check.yml`: the Rust toolchain step
- `AGENTS.md`: Layout and Commands rows
