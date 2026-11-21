# AGENTS.md

The inbox: one durable, addressable inbox per agent or person, for messages between agents in
different sessions and on different hosts. One service, `inbox serve`, with interchangeable
backends, specified in ESS before any code. The design is `docs/design/inbox.md`; read it before
changing anything.

Status: stage 1 of 6 is done; stage 2 runs as the 5 waves of `docs/plan/stage-2.md`, of which
waves 1 to 3 are delivered. Stages 2 to 6 are the epics in `.engineering/planning/epic/`.

## Layout

| Path | What it is | Who writes it |
|---|---|---|
| `ess/` | ESS source (`ess/37`), listed in `ess/ess-inputs.yaml`, pinned to `ess 0.96.0` | authored |
| `ess/domains/messaging.yaml` | The domain `inbox.messaging`: 2 entities, 9 commands, 9 views, 13 errors, 10 events | authored |
| `ess/scenarios/`, `ess/ess-inputs.yaml` | The 23 authored scenarios and the manifest that lists them | `task scenarios` |
| `tools/scenarios.py` | Source of the authored scenarios and of their `covers:` claims | authored |
| `generated/{openapi,asyncapi,schema}/` | ESS projections of `ess/`; `generated/openapi` is the wire API of the service | `task projections` |
| `generated/rust/inbox/` | The crate `ess generate synthesize --target rust --layout crate` writes from `ess/`: model types, command behaviours, view queries, storage and context ports; the route table, JSON codecs, transport-free `handle` and the binary `inbox-service-server` behind the `server` feature. On ESS 0.96.0: 85 of 87 capabilities, obligations `PostMessage` and `Expire` (`PLAN.md`). Its `Cargo.lock` is checked in and kept by `task generate`; `.ess-output/` is not checked in | `task generate` |
| `Cargo.toml`, `Cargo.lock` | The Cargo workspace: members `crates/*`; `generated/rust` is excluded, because the synthesized `Cargo.toml` declares its own `[workspace]` | authored, `cargo` |
| `rust-toolchain.toml` | Rust 1.99.1 with `rustfmt` and `clippy`; `.github/workflows/check.yml` installs the same version from checksummed release archives | authored |
| `crates/inbox-xtask/` | Repository tasks: `generate` and `drift` synthesize `ess/` into `target/inbox-xtask/` and compare it with `generated/rust/inbox` | authored |
| `crates/inbox-core/` | The core: `Core` (the obligations `PostMessage` and `Expire` over a backend and a context), `ServiceContext` (the generated context port and `PostExternals` over a caller, a `SignatureVerifier` and the backend), `MemoryBackend` (both storage ports in memory; clones share one store; `Send + Sync`), `Exclusive` (the third trait a backend implements: one command at a time per store) and the entry `handle` / `handle_with_events`, which an embedding calls per request with a backend clone and that request's context. `system()` alone serializes nothing. A context or verifier must not call the entry over the same store | authored |
| `crates/inbox-verify/` | The version 1 signature verifier: Ed25519 signing and strict verification of envelopes, key file entries and requests over RFC 8785 canonical JSON; `KeyRing` holds the entries the pinned operator key signed and checks them at an instant the caller passes; `duplicate_member` finds a member name a parsed JSON text carries twice. Depends on the generated crate and on no other crate of the workspace | authored |
| `crates/inbox-backend-testkit/` | The backend test kit: `check_inbox_storage`, `check_message_storage` and `check_exclusive`, one call per part of the backend contract (the 2 generated storage ports and `Exclusive` of `inbox-core`), each taking a factory of fresh backends. A broken rule panics with `backend contract broken: <part>: <rule>`. Its tests run it against `MemoryBackend` and against 12 planted backends that each break one rule. A later backend crate calls the 3 functions from its own tests | authored |
| `crates/inbox-conformance/` | The conformance runner: reads the synthesized `ess-conformance/52` suite and runs it in process against `inbox-core` over a fresh `MemoryBackend` per scenario, through the trait `Target`, whose one target is `CoreTarget`. `run` writes `ess-conformance-results/1`; `gate --suite --report --unsupported [--implementation]` gates on the report/2 that `ess verify conform report` makes from it (the row of `task conform` says what it checks). A suite with a top-level or `provenance` key the runner does not know, or with a member name stated twice in one object, is refused with exit 2. A construct the runner cannot interpret makes the scenario unsupported, and `unsupported.tsv` lists those with reasons (empty today). In a view row an absent top-level member and `null` are one value (`value::field_matches`), where ESS's runner compares exactly: the generated server leaves an unset `Optional` out (beyond10x/ess#799). When that issue is closed, remove the rule and the ignore attribute that names it in `tests/adversary.rs` | authored |
| `crates/inbox-server/` | The HTTP service library: a loopback listener in front of the generated `server` feature, one thread per connection. Each request is read with the limits of `Config`, its request signature checked (`Inbox-Key-Id`, `Inbox-Timestamp`, `Inbox-Signature`, 60 s window at the `Clock` reading), a body with a member name twice refused, then the generated `dispatch` runs inside the backend's `Exclusive` section over `inbox_core::system`. Adds the read rule, the wait on `Pending`, the expiry task, `request_timeout` (`408`) and a stop that answers `503`. 3 cases in `tests/adversary.rs` are ignored, each naming the backlog story that turns it green | authored |
| `docs/design/signing.md` | What each signature covers: context strings, canonical bytes, wire encodings, lapse and the 60 s request window, what a verifier refuses, the rules of the key file (an id is never reused), what version 1 leaves out | authored |
| `docs/design/service.md` | The HTTP service of stage 2: the steps of one request in order with each refusal's status and body, the 3 request signature headers, the caller, the read rule, the wait, the expiry task, the limits, binding and stopping, what stage 2 leaves out | authored |
| `docs/design/inbox.md` | The design, the decisions taken while modelling, and what ESS 0.96.0 cannot express | authored |
| `docs/plan/` | One wave plan page per stage: the waves in order, and per unit its branch, worktree id and stage | the wave's coordinator |
| `.engineering/` | AEP planning store; the plan lives here, not in chat or plan files | `aep` |

## Commands

| Command | Does |
|---|---|
| `task check` | The gate: `task spec`, `task projections-drift`, `task drift`, `task conform-interpreted`, `task conform`, `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `task build-generated`. Run it as `slot task check` on a shared host |
| `task spec` | `ess specify validate --path ess --strict-requires` |
| `task projections` | Rewrite `generated/{openapi,asyncapi,schema}` after a change to `ess/` |
| `task generate` | Rewrite `generated/rust/inbox` after a change to `ess/` or to the `ess` version, then bring its `Cargo.lock` up to date with `cargo metadata`. The synthesis is staged in `target/inbox-xtask/generate`; the held crate moves to `target/inbox-xtask/previous` and is moved back if the staged one cannot take its place. If a run left the held crate only in `previous` (`generated/rust/inbox` absent), the next run moves it back first, `Cargo.lock` included; a `previous` beside a present crate is removed. Works where `generated/rust/` does not exist yet |
| `task drift` | Fail with `generated/rust/inbox drifted from a fresh synthesis of ess/` and one `edited:`, `missing:` or `not generated:` line per path when the crate differs from a fresh synthesis. A symbolic link is never followed and counts as `edited:`. Of a file's mode only the executable bit is compared, the one bit git records (`edited:` when it differs); the other mode bits follow the umask of the checkout and are not compared. The crate's top-level `Cargo.lock`, `.ess-output/` and `target/` (git-ignored) are not compared. A tree without the crate reports one `missing:` line per file. `inbox-xtask` runs on the tree it is started in: two levels above the runtime `CARGO_MANIFEST_DIR` (a relative one is taken from the current directory), else the nearest directory upwards; the root must hold the directory `ess/` and the file `Cargo.toml` |
| `task build-generated` | `cargo build --features server` of `generated/rust/inbox`, into the repository's own `target/` (`--target-dir`), never into a `target/` inside `generated/` |
| `task conform-interpreted` | Synthesize the suite into `.out/conform/` and run it on the ESS interpreted target. On ESS 0.96.0: 92 scenarios (23 authored), 92 passed |
| `task conform` | Synthesize the suite into `.out/conform/`, run it against `inbox-core` in process (`inbox-conformance run`, results in `core-results.json`), turn the results into `core-report.json` with `ess verify conform report`, and gate on that report (`inbox-conformance gate --suite --report --unsupported --implementation`, the name the task's `IMPLEMENTATION` var also gives `ess verify conform report`): fails unless the report's `suite.digest` is that of the synthesized suite, its outcome lists name each scenario id of the suite once and no other, its `implementation` is that name, its counts agree with its lists and add up to the total, 0 scenarios failed, 0 errored, 0 skipped, the unsupported ones are exactly those `crates/inbox-conformance/unsupported.tsv` lists (`<id>\t<reason>` per line), and `conformance_status` (printed) is `inconclusive`, or `failed` where only listed scenarios are unsupported. A suite with a top-level or `provenance` key the runner does not know, or a member stated twice in any object, is refused (exit 2, key named). `task` exits 201 on a failure, `task -x conform` 1. On ESS 0.96.0: 92 scenarios, 92 passed, 0 unsupported |
| `task coverage` | Synthesize with declared coverage. Exits 1 on ESS 0.96.0: 50 of 97 refused generated scenarios are covered by nothing (all `ESS-SYNTH-013`). Not in `task check` |
| `task scenarios` | Rewrite `ess/scenarios/` and `ess/ess-inputs.yaml` from `tools/scenarios.py`. Needs `python3` with PyYAML |
| `task plan` | `aep plan artifact validate` |

## Rules

- Change `ess/` first. Never edit `generated/` by hand; `task check` fails on drift.
- `generated/rust/inbox` is never formatted, linted or edited: the synthesized code fails
  `cargo fmt --check` and `cargo clippy -- -D warnings` on ESS 0.96.0, and it stays outside the
  workspace, so neither step reads it. A crate depends on it by path
  (`inbox = { path = "../../generated/rust/inbox", features = ["server"] }`), implements the
  obligations named in its `PLAN.md` outside it, and adds itself under `crates/` without editing
  `Cargo.toml`.
- A scenario changes in `tools/scenarios.py`, then `task scenarios`; commit both. The script
  removes no file: delete the file of a scenario taken out of it.
- An `UNMAPPED:` marker in `ess/` is a rule ESS cannot express yet. It stays until ESS can, and
  `docs/design/inbox.md` lists it. Never resolve one by guessing.
- A generated scenario that synthesis refuses is claimed by an authored scenario's `covers:`
  list (the `COVERS` map in `tools/scenarios.py`), one claimant per id.
- Every client talks to the service only. No client talks to a backend, and backend credentials
  exist on the service host only.
- No statement of the specification depends on a JetStream fact. A JetStream fact is read in the
  NATS documentation and cited before anything relies on it.
- The plan changes only through `aep plan artifact`; never edit a file under
  `.engineering/planning/` by hand. Every write runs with `AEP_ACTOR=agent:<session name>`:
  without it `aep` records the login name as the actor of an evidence record.
- Tokens, keys and secrets are never written to a file, a commit or a message.
- Every commit and push is `b10x-bot[bot]` through `b10x-gates bot`. The repository is enrolled in
  the common Gates policy (`.github/workflows/shared-gates.yml`).
- The repository is private. A change reaches `main` as a fast-forward bot push: `b10x-gates`
  refuses a later push once `main` holds a merge commit GitHub created ("remote repository is not
  public"), until the policy baseline is moved past it.
- Nothing from a customer or employer outside beyond10x enters this repository. No person's name
  and no host path either: write "the operator".
- No release, tag or deployment without the operator's word.
- A revision of an accepted story that changes scope, wave order or a settled decision, or adds a
  wave, goes to the host operator first and waits for the answer: silence is not acceptance. Any
  other revision is reported in the wave report.
- A reason given for declining a review finding is measured before it is written down.
- Three backlog drafts are the first work after wave 5 of stage 2: story:post-from-is-the-caller,
  story:foreign-message-id-answers-as-unknown and story:dedup-key-scope. They land before any of:
  a listener bound outside loopback, a NATS backend wave, a deployment, use by a real session
  (the host operator's decision, 2026-11-21; `docs/plan/stage-2.md`, Backlog). Until the first
  lands, a recipient can post a sender's signed envelope again under its own request signature.
  Its guard goes into `ess/`; no copy of it is written in the listener.
