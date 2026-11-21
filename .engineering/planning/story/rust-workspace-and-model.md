---
format: aep.planning-md/3
id: story:rust-workspace-and-model
kind: story
status: implemented
title: Rust workspace and the model crate synthesized from `ess/`
summary: A Cargo workspace whose model, codecs and obligations crate is generated from the specification and held to it by the gate.
relations:
- decomposes: epic:service-in-memory
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: .gitignore
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/inbox-xtask
- confidence: cited
  path: generated/rust
- confidence: cited
  path: rust-toolchain.toml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-21T08:37:15Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":8,"verification":1}}}
---
## Outcome

The repository is a Cargo workspace whose members are the glob `crates/*`, so a later story adds
a crate without editing a shared manifest. `generated/rust/inbox` is the crate
`ess generate synthesize --target rust --layout crate` writes from `ess/`: the model types, the
behaviour of 7 of the 9 commands, the query of all 9 views, the storage and context ports they
run on, and, as its `server` feature, the route table, the JSON codecs of every type (the key file types included) and a
transport-free `handle`. On ESS 0.96.0 that is 85 of 87 capabilities; the 2 it leaves as obligations are the
behaviours of `PostMessage` and `Expire`. The synthesized `Cargo.toml` declares its own
`[workspace]`, so the crate stays outside the workspace (`exclude = ["generated/rust"]`) and a
crate under `crates/` depends on it by path. The crate is checked in whole and `task check` builds
it with `--features server`. `task generate` rewrites it and `task check`
fails when it differs from a fresh synthesis, and runs `cargo fmt --check`, `cargo clippy` and
`cargo test` for the workspace. The synthesized code is built, not formatted or linted: it fails
both steps on ESS 0.96.0 and is never edited. `.github/workflows/check.yml` installs the pinned
Rust toolchain.

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

The binary `inbox-service-server` of the generated crate builds, and at start prints 7 unmet
start-up obligations and exits 1: the 2 caller attributes (`inbox`, `principal`), the behaviours of
`PostMessage` and `Expire`, and the 3 external branch answers of `Answer`.

## Scope

Confirmed by the implementor against the merged change.

- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` (new). Members `crates/*`; `generated/rust`
  excluded. A later story's crate changes `Cargo.lock` only; a merge of two such stories
  regenerates it with `cargo metadata`.
- `generated/rust/inbox/` (new, generated): the 23 synthesized files and the crate's own
  `Cargo.lock`, which `task generate` keeps. `.ess-output/` is not checked in.
- `crates/inbox-xtask/` (new): library, binary and tests
- `Taskfile.yml`: tasks `generate`, `drift`, `build-generated`; `check` gains them and the cargo
  steps
- `.github/workflows/check.yml`: the Rust toolchain step
- `.gitignore`: `/generated/rust/*/target`
- `AGENTS.md`: Layout and Commands rows, one Rules line
