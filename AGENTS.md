# AGENTS.md

The inbox: one durable, addressable inbox per agent or person, for messages between agents in
different sessions and on different hosts. One service, `inbox serve`, with interchangeable
backends, specified in ESS before any code. The design is `docs/design/inbox.md`; read it before
changing anything.

Status: stage 1 of 6 is done; the 5 waves of stage 2 (`docs/plan/stage-2.md`) are
delivered, and waves 6 and 7, added after them: wave 6 delivered 2 of its 3 stories, and wave 7
delivered story:dedup-key-scope and story:post-does-not-tell-a-message-id-exists on ESS 0.97.1.
Release 0.1.0 is the state of `main` after wave 3; waves 4 to 7 are on `main` and in no release.
The gate (see Rules) is open for principals of one operator. Stages 2 to 6 are the epics in
`.engineering/planning/epic/`.

## Layout

| Path | What it is | Who writes it |
|---|---|---|
| `ess/` | ESS source (`ess/37`), listed in `ess/ess-inputs.yaml`, pinned to `ess 0.97.1` | authored |
| `ess/domains/messaging.yaml` | The domain `inbox.messaging`: 2 entities, 9 commands, 9 views, 14 errors, 10 events | authored |
| `ess/scenarios/`, `ess/ess-inputs.yaml` | The 32 authored scenarios and the manifest that lists them | `task scenarios`, held by `task scenarios-drift` |
| `tools/scenarios.py` | Source of the authored scenarios and of their `covers:` claims | authored |
| `generated/{openapi,asyncapi,schema}/` | ESS projections of `ess/`; `generated/openapi` is the wire API of the service | `task projections` |
| `generated/rust/inbox/` | The crate `ess generate synthesize --target rust --layout crate` writes from `ess/`: model types, command behaviours, view queries, storage and context ports; the route table, JSON codecs, transport-free `handle` and the binary `inbox-service-server` behind the `server` feature. On ESS 0.97.1: 86 of 88 capabilities, obligations `PostMessage` and `Expire` (`PLAN.md`). Its `Cargo.lock` is checked in and kept by `task generate`; `.ess-output/` is not checked in | `task generate` |
| `Cargo.toml`, `Cargo.lock` | The Cargo workspace: members `crates/*` and `tests/acceptance`; `generated/rust` is excluded, because the synthesized `Cargo.toml` declares its own `[workspace]` | authored, `cargo` |
| `rust-toolchain.toml` | Rust 1.99.1 with `rustfmt` and `clippy`; `.github/workflows/check.yml` installs the same version from checksummed release archives | authored |
| `crates/inbox-xtask/` | Repository tasks: `generate` and `drift` synthesize `ess/` into `target/inbox-xtask/` and compare it with `generated/rust/inbox` | authored |
| `crates/inbox-core/` | The core: `Core` (the obligations `PostMessage` and `Expire` over a backend and a context), `ServiceContext` (the generated context port and `PostExternals` over a caller, a `SignatureVerifier` and the backend), `MemoryBackend` (both storage ports in memory; clones share one store; `Send + Sync`), `Exclusive` (the third trait a backend implements: one command at a time per store) and the entry `handle` / `handle_with_events`, which an embedding calls per request with a backend clone and that request's context. `system()` alone serializes nothing. A context or verifier must not call the entry over the same store | authored |
| `crates/inbox-verify/` | The version 1 signature verifier: Ed25519 signing and strict verification of envelopes, key file entries and requests over RFC 8785 canonical JSON; `KeyRing` holds the entries the pinned operator key signed and checks them at an instant the caller passes; `duplicate_member` finds a member name a parsed JSON text carries twice. Depends on the generated crate and on no other crate of the workspace | authored |
| `crates/inbox-backend-testkit/` | The backend test kit: `check_inbox_storage`, `check_message_storage` and `check_exclusive`, one call per part of the backend contract (the 2 generated storage ports and `Exclusive` of `inbox-core`), each taking a factory of fresh backends. A broken rule panics with `backend contract broken: <part>: <rule>`. Its tests run it against `MemoryBackend` and against 12 planted backends that each break one rule. A later backend crate calls the 3 functions from its own tests | authored |
| `crates/inbox-conformance/` | The conformance runner: reads the `ess-conformance/65` suite `task coverage` writes (its `coverage` block held to the suite; `ess-conformance/52` is refused) and runs it through the trait `Target` against 2 targets: `CoreTarget` (`inbox-core` in process over a fresh `MemoryBackend` per scenario) and `WireTarget` (`run --target serve --inbox <binary>`: a fresh `inbox serve` on `127.0.0.1:0` per scenario over HTTP, keys made at run time, every request and envelope signed, each answer held to the HTTP status of the generated contract, events read from the answer's `published` list). `run` writes `ess-conformance-results/1`; `gate --suite --report --unsupported [--results] [--implementation]` gates on the report/2 that `ess verify conform report` makes from it (the row of `task conform` says what it checks). A suite with a top-level or `provenance` key the runner does not know, or with a member name stated twice in one object, is refused with exit 2. A construct the runner cannot interpret, or a step the wire cannot witness, makes the scenario unsupported, and each target has its list of those with reasons: `unsupported-core.tsv` (empty) and `unsupported.tsv` for the service (80 scenarios: a caller whose principal and inbox differ (68; over HTTP both are the principal of the caller's key), a step as the Sweeper (38), a step as no actor (9), a claim on the stored `signature` (7), one delegation id for two pairs (1); a scenario can have several). The gate holds each listed reason to the reason the run gave. In a view row an absent top-level member and `null` are one value (`value::field_matches`), where ESS's runner compares exactly: the generated server leaves an unset `Optional` out (beyond10x/ess#799). When that issue is closed, remove the rule and the ignore attribute that names it in `tests/adversary.rs` | authored |
| `crates/inbox-server/` | The HTTP service library: a loopback listener in front of the generated `server` feature, one thread per connection. Each request is read with the limits of `Config`, its request signature checked (`Inbox-Key-Id`, `Inbox-Timestamp`, `Inbox-Signature`, 60 s window at the `Clock` reading), a body with a member name twice refused, then the generated `dispatch` runs inside the backend's `Exclusive` section over `inbox_core::system`. Adds the read rule, the wait on `Pending`, the expiry task, `request_timeout` (`408`) and a stop that answers `503`. An answer of a request that is not a GET whose outcome is the model's `foreign-message` is sent with the status and the body of `unknown-message` (`as_unknown` in `src/service.rs`): the model decides who is a party of a message, the listener only renames that one outcome. Every answer, whatever its status, carries the header `Inbox-Epoch` (`EPOCH_HEADER`, written in `write` of `src/listener.rs`): 32 lower-case hex digits made once per bound listener from 16 random bytes, one value for the answers of one start, no time and no secret in it. No case is ignored | authored |
| `crates/inbox-cli/` | The binary `inbox`. `inbox serve` starts `crates/inbox-server` over `MemoryBackend` on a loopback address. The other commands are the client: each request is signed with the caller's key from a secret key file (mode 0600, never on the command line or in the environment), an `INBOX_URL` outside loopback is refused, and each posted envelope is kept in an outbox file until it is collected, refused with an error of the model other than `UnknownInbox`, or expired. Beside it, `<outbox>.epochs` (one line `<message id>\t<32 hex digits>`; changed under the outbox lock, after the outbox file) records, per kept envelope the service said it holds (an accepted post, or the outcome `duplicate`; not `duplicate-key`), the `Inbox-Epoch` of that answer. Reading it never fails a command: a missing or unreadable file, a malformed line and a message id named twice mean no epoch, and `repost` posts the envelope. `inbox key new` and `inbox keyfile add-key` / `add-delegation` make keys and the signed key file. Flags and exit statuses: the rows of the binary under Commands | authored |
| `tests/acceptance/` | The package `inbox-acceptance`: 3 checks against the built `inbox` binary (`tests/checks.rs`), ignored under `cargo test --workspace` and run by `task acceptance` with `INBOX_BIN` naming the binary; fixtures in `src/lib.rs`. `tests/adversary.rs` holds 1 ignored case no task runs: it removes the epochs file before each of its 2 reposts, so that the second post reaches the duplicate check, and expects the answers `posted` then `duplicate` | authored |
| `docs/design/signing.md` | What each signature covers: context strings, canonical bytes, wire encodings, lapse and the 60 s request window, what a verifier refuses, the rules of the key file (an id is never reused), what version 1 leaves out | authored |
| `docs/design/service.md` | The HTTP service of stage 2: the steps of one request in order with each refusal's status and body, the 3 request signature headers, the caller, the read rule, the wait, the expiry task, the limits, binding and stopping, what stage 2 leaves out | authored |
| `docs/design/inbox.md` | The design, the decisions taken while modelling, and what ESS 0.97.1 cannot express | authored |
| `docs/plan/` | One wave plan page per stage: the waves in order, and per unit its branch, worktree id and stage | the wave's coordinator |
| `.engineering/` | AEP planning store; the plan lives here, not in chat or plan files | `aep` |
| `generated/docs/` | The specification pages `ess generate --kind docs` writes from `ess/`, with its `.ess-output/` record; the site reads them in place | `task projections` |
| `website/` | The documentation site (Docusaurus on `@beyond10x/docs-system`): `product.json` is the landing page, `docs/` the hand-written pages, `sidebars.ts` and `sidebars-specification.ts` the two sidebars; the specification pages are a second docs instance over `../generated/docs/docs` | authored, `npm` (`package-lock.json`) |
| `.github/workflows/pages.yml`, `.github/workflows/b10x-docs-site.yml` | Build the site on every push and pull request (job `Build documentation`); publish it from `main` through the organization's project-site workflow | authored |
| `.github/workflows/release.yml` | Job `Version`: a version tag equals the workspace version, is annotated, and has a `CHANGELOG.md` section. Builds and publishes nothing | authored |
| `CHANGELOG.md`, `docs/releasing.md` | What each release holds; how a release is cut | authored, the release's coordinator |

## Commands

| Command | Does |
|---|---|
| `task check` | The gate: `task spec`, `task scenarios-drift`, `task projections-drift`, `task drift`, `task coverage`, `task conform-interpreted`, `task conform`, `task acceptance`, `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo test --workspace --locked`, `task build-generated`. The suite is synthesized once per run (`task coverage` has `run: once`). Run it as `slot task check` on a shared host (`cargo test --workspace`: 466 passed, 0 failed, 5 ignored) |
| `task spec` | `ess specify validate --path ess --strict-requires` |
| `task projections` | Rewrite `generated/{openapi,asyncapi,schema,docs}` after a change to `ess/` |
| `task generate` | Rewrite `generated/rust/inbox` after a change to `ess/` or to the `ess` version, then bring its `Cargo.lock` up to date with `cargo metadata`. The synthesis is staged in `target/inbox-xtask/generate`; the held crate moves to `target/inbox-xtask/previous` and is moved back if the staged one cannot take its place. If a run left the held crate only in `previous` (`generated/rust/inbox` absent), the next run moves it back first, `Cargo.lock` included; a `previous` beside a present crate is removed. Works where `generated/rust/` does not exist yet |
| `task drift` | Fail with `generated/rust/inbox drifted from a fresh synthesis of ess/` and one `edited:`, `missing:` or `not generated:` line per path when the crate differs from a fresh synthesis. A symbolic link is never followed and counts as `edited:`. Of a file's mode only the executable bit is compared, the one bit git records (`edited:` when it differs); the other mode bits follow the umask of the checkout and are not compared. The crate's top-level `Cargo.lock`, `.ess-output/` and `target/` (git-ignored) are not compared. A tree without the crate reports one `missing:` line per file. `inbox-xtask` runs on the tree it is started in: two levels above the runtime `CARGO_MANIFEST_DIR` (a relative one is taken from the current directory), else the nearest directory upwards; the root must hold the directory `ess/` and the file `Cargo.toml` |
| `task build-generated` | `cargo build --features server` of `generated/rust/inbox`, into the repository's own `target/` (`--target-dir`), never into a `target/` inside `generated/` |
| `task conform-interpreted` | Run the suite `task coverage` writes on the ESS interpreted target (report `.out/conform/report.json`). On ESS 0.97.1: 105 scenarios (32 authored), 105 passed, `conformance_status: passed` |
| `task conform` | Run the suite `task coverage` writes (`.out/coverage/suite.json`, `ess-conformance/65`) against two targets, each with its results, its report by `ess verify conform report` and its gate (`inbox-conformance gate --suite --report --results --unsupported --implementation`), into `.out/conform/`: `inbox-core` in process (`core-results.json`, `core-report.json`, list `crates/inbox-conformance/unsupported-core.tsv`, empty), then the built `inbox` binary over HTTP (`inbox-conformance run --target serve --inbox target/debug/inbox`: a fresh `inbox serve` on `127.0.0.1:0` per scenario, keys made at run time, each answer held to the status the generated contract gives it, its key file and standard error in `.out/conform/serve/`; `serve-results.json`, `serve-report.json`, implementation `inbox-serve`, list `crates/inbox-conformance/unsupported.tsv`). Each gate fails unless the report's `suite.digest` is that of the suite, its outcome lists name each scenario id of the suite once and no other, its `implementation` is that target's name, its counts agree with its lists and add up to the total, 0 scenarios failed, 0 errored, 0 skipped, the unsupported ones are exactly those the target's list names (`<id>\t<reason>` per line), each with the reason the run gave it (read from the results document `--results` names; a report/2 carries no reasons), the results agree with the report (each scenario id with the status of the list the report puts it in, no id more or less, the same `completed_at`, which ESS copies from the results), and `conformance_status` (printed) is `passed`, or `failed` where only listed scenarios are unsupported; `inconclusive` fails. A suite of another format, or with a top-level or `provenance` key the runner does not know, or a member stated twice in any object, is refused (exit 2, key named). `task` exits 201 on a failure, `task -x conform` 1. On ESS 0.97.1: against `inbox-core` 105 scenarios, 105 passed, 0 unsupported, `conformance_status: passed`; against `inbox serve` 105 scenarios, 25 passed, 0 failed, 0 error, 80 unsupported (68 with a caller whose principal and inbox differ, 38 with a step as the Sweeper, 9 as no actor, 7 with a claim on the stored signature, 1 with one delegation id for two pairs; one scenario can have several), `conformance_status: failed`. About 18 s |
| `task acceptance` | Build `inbox` (`cargo build --locked -p inbox-cli`) and run the 3 checks of `tests/acceptance` (package `inbox-acceptance`, `tests/checks.rs`) with `INBOX_BIN` naming the binary: each starts its own `inbox serve` on `127.0.0.1:0` and drives it with the client commands, keys made with `inbox key new` and `inbox keyfile add-key`: a signed request and its reply between 2 keys; a read note and an unanswered request 2 sweep intervals after its `expires` in no view (`--sweep-interval 1`, the one wait on real time); a restart, `inbox open`, 2 reposts (the first answered `posted`, the second posting nothing: the epochs file records the start that answered the first), and the unread message once in `Pending`. The checks are ignored by a plain `cargo test` and fail naming the cause where `INBOX_BIN` names no file. Fails when a check fails or fewer than 3 passed; log `.out/acceptance/test.log`. 3 passed, about 5 s |
| `task coverage` | Synthesize with declared coverage into `.out/coverage/`; fails on a refused generated scenario no authored scenario covers. On ESS 0.97.1: 105 scenarios (32 authored), 93 refused, all 93 covered, exit 0; the suite it writes reports `conformance_status: passed` on the interpreted target. In `task check` |
| `task scenarios` | Rewrite `ess/scenarios/` and `ess/ess-inputs.yaml` from `tools/scenarios.py`. The script runs its checks before it writes any file (one claimant per id; each claim of a type invariant at a view field rests on a row of that view with a value at the field's path); a failed check changes no file. Needs `python3` with PyYAML |
| `task scenarios-drift` | Fail when `ess/scenarios/` or `ess/ess-inputs.yaml` differs from what `tools/scenarios.py` writes: one line per path (`edited:`, `missing:`, `not written by the script:`), exit 1; writes nothing (`python3 tools/scenarios.py ess --check`). In `task check` after `task spec` |
| `task plan` | `aep plan artifact validate` |
| `cargo build --release -p inbox-cli` | Build the binary `target/release/inbox` (`crates/inbox-cli`). Every command takes `--json` (one JSON document on standard output) and exits 0 done (a `2xx` answer), 1 refused by the service (the model's error name, or the service's `refused` text for `401`, `400`, `408`, `413`, `503`) or an outbox line `repost` cannot read, 2 wrong usage (an `INBOX_URL` that is not loopback, an envelope the generated decoder refuses), 3 transport failure or an answer that is neither success nor refusal, 4 a local file not readable or writable |
| `inbox serve --listen 127.0.0.1:0 --key-file <file> --operator-public-key <base64>` | Serve `crates/inbox-server` over `MemoryBackend` until SIGINT or SIGTERM (exit 0; the signal's line on standard error is written after the stop and never holds the exit). First line on standard output: the URL served (`--json`: `{"address","url"}`). Flags for every `Config` setting (`--max-connections`, `--request-timeout`, `--max-request-bytes`, `--max-messages`, `--max-wait`, `--sweep-interval`, `--answer-retention`; durations in seconds). Refuses a non-loopback address and an `INBOX_BACKEND` other than `memory` (exit 2), an unreadable key file (exit 4) |
| `inbox key new --out <file> [--key-id <id> --principal <p>]` | A key pair: the secret file is created with mode 0600 and never overwritten, the public key is printed. A client key's file names its key id and principal. `inbox keyfile add-key` / `add-delegation --file <keys.json> --operator-secret-key-file <file>` append an entry signed with the operator key, under a lock of `<file>.lock`, keeping the file's mode; they refuse a `--valid-until` at or before now (exit 2) and a key file holding an entry another operator key signed (exit 4) |
| `inbox open`, `close`, `post`, `pending`, `wait`, `read`, `answer`, `refuse`, `outbox`, `collect`, `thread`, `replies`, `repost` | The client: the service at `INBOX_URL` (`--url`; loopback addresses only, each resolved one tried in turn), the caller's key from `INBOX_SECRET_KEY_FILE` (`--secret-key-file`). `post` keeps each envelope in the outbox file (`INBOX_OUTBOX`, `--outbox`; default `$XDG_STATE_HOME/inbox/outbox.jsonl`) until collected, refused with a declared refusal other than `UnknownInbox`, or expired; a transport failure and every refusal of the service's own (`400`, `401`, `408`, `413`, `503`) keep it; `outbox` lists the lines of that file that do not decode; `repost` asks the service for its epoch (the header `Inbox-Epoch`) when a kept line has one in the epochs file, leaves alone the kept ones recorded at that epoch and posts the others again, `Duplicate` counting as arrived, and names an unreadable line (exit 1); it records the epoch of each answer that said the service holds the message once, after the last post, and a failed write of the epochs file ends no post and exits 4 after the report. `MessageIdTaken` (`id-taken`, 409: the id is another sender's stored message) is final: `post` and `repost` exit 1 and drop the outbox line. The client has no flag to choose a message id: it draws a ULID per post. The first `repost` after a restart of the service still delivers once more a note its recipient read before it, and so does a `repost` against a service without the header or with the epochs file lost (story:repost-does-not-redeliver-a-handled-note) |
| `slot npm --prefix website ci`, then `slot npm --prefix website run build` | Build the site into `website/build`; fails on a broken link or anchor. `npm --prefix website start` serves it locally |

## Publishing

- The repository is public at `https://github.com/beyond10x/inbox`. What must never enter it is
  in "Rules" below; the gates scan every push and refuse without printing the match.
- `main` moves only by the organization bot's fast-forward push through `b10x-gates`, after
  `task check` is green on the commit. Three checks run on every push and are read before a
  release: `common / Security and privacy`, `inbox / task check` and `Build documentation`.
- The repository has no ruleset (read on 2026-11-21: 0). Writing a ruleset or branch protection
  is the owner's act: `b10x-gates api` refuses it, and that refusal is not worked around. Until
  the owner adds a ruleset that requires the 3 checks, nothing on the forge refuses a push to
  `main`; the gate before the push is `task check` and `b10x-gates`.
- Once that ruleset exists, a commit on which the 3 checks have not run is refused on `main`:
  push it to a branch as the bot, open a pull request, wait for the checks on its head, then
  fast-forward `main` to that commit. This route has not been run in this repository. Never
  merge a pull request with GitHub's merge button: `main` holds only commits the bot wrote and
  committed.
- The site is the repository's own, at `https://beyond10x.github.io/inbox/` (route `/inbox/`).
  `.github/workflows/pages.yml` builds it on every push and pull request and, on `main`, uploads
  the build with `.well-known/b10x-site.json`; `.github/workflows/b10x-docs-site.yml` publishes
  that build through the organization's project-site workflow (pinned by commit), only for a
  push by `b10x-bot[bot]` to `main`. It is not part of the organization's shared documentation:
  no `b10x.docs.yaml`.
- Build it locally with the two `npm` commands under "Commands". The specification pages are
  not copied: the site reads `generated/docs/docs` in place, `task projections` writes it, and
  `task check` fails when it differs from `ess/`.
- The release process is `docs/releasing.md`: an annotated version tag by the bot, then a
  GitHub Release by the bot. A release holds no binary: the release workflow builds and attaches nothing, and `inbox` is built from source.
- No deployment exists.

## Rules

- Change `ess/` first. Never edit `generated/` by hand; `task check` fails on drift.
- `generated/rust/inbox` is never formatted, linted or edited: the synthesized code fails
  `cargo fmt --check` on ESS 0.97.1 (`cargo clippy -- -D warnings` failed on ESS 0.96.0 and was
  not measured again), and it stays outside the workspace, so neither step reads it. A crate
  depends on it by path (`inbox = { path = "../../generated/rust/inbox", features = ["server"] }`), implements the
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
- The repository is public (the operator's decision of 2026-11-21). A change reaches `main` only
  as a fast-forward push by the organization bot through `b10x-gates`, after the gate; see
  "Publishing".
- Nothing from a customer or employer outside beyond10x enters this repository. No person's name
  and no host path either: write "the operator".
- No release, tag or deployment without the operator's word.
- A revision of an accepted story that changes scope, wave order or a settled decision, or adds a
  wave, goes to the host operator first and waits for the answer: silence is not acceptance. Any
  other revision is reported in the wave report.
- A decision that is the operator's goes to the host operator's session, and its answer is the
  operator's (the operator, 2026-11-21, in the coordinator's terminal). The coordinator puts no
  decision to the operator's terminal and does not ask the operator to confirm one the host
  operator took; it reports the decision once. What only the repository's owner can do on the
  forge (a ruleset, branch protection) stays the owner's act.
- A member of a view that no sequence of commands can set, found before the next release, is
  removed from the view in the same unreleased window without asking first: change `ess/`,
  regenerate, name the view and the field in the changelog as a change of the wire rows, and
  report it in the wave report (the host operator's standing decision, 2026-11-21). The argument
  that no row can carry the member goes into the story text. A regeneration that changes more
  than the rows of that view stops the work and goes to the host operator.
- A reason given for declining a review finding is measured before it is written down.
- Wave 6 of stage 2 delivered story:post-from-is-the-caller and
  story:foreign-message-id-answers-as-unknown. Wave 7 delivered story:dedup-key-scope and
  story:post-does-not-tell-a-message-id-exists on ESS 0.97.1, which fixes beyond10x/ess#805. Both
  stories the gate named are implemented (`docs/plan/stage-2.md`).
- The gate (the host operator's decision, 2026-11-21): it is open for one case, principals of
  one operator (that operator's own sessions and hosts). story:message-id-per-sender (a draft: a
  message id unique per sender) is required before principals of different operators share one
  service: a principal that holds a message id learns from a post into an open inbox whether
  the id is live, and it can take a free id first. A release, a listener outside loopback and a
  deployment each still need the operator's own order. `inbox serve` refuses a `--listen`
  address outside loopback and the client an `INBOX_URL` outside loopback.
- The input `from` of `PostMessage` carries `example: principal-262144` for beyond10x/ess#804.
  When an ESS release fixes it, remove the example and its comment, regenerate, and remove the
  lines that name it in `docs/design/inbox.md`, `website/docs/limits.md` and `CHANGELOG.md`.
- The guard on `from` is in `ess/` (`not-sender`); no copy of it is written in the listener.
- A draft that an adversary pass files against an open limit of `website/docs/limits.md` is
  accepted by the coordinator without asking and named in the wave report (the host operator's
  standing decision, 2026-11-21). It joins no wave that is running.
- A commit message never puts a closing keyword (`fixes`, `closes`, `resolves` and their forms)
  before an issue reference, also across repositories and also after a negation: GitHub closes
  the issue when the commit reaches the default branch. Write "issue 805 of beyond10x/ess" in
  prose. (On 2026-11-21 a commit of this repository closed 2 issues of beyond10x/ess that way.)
- A coordinator's sub-agents run on Opus, Sonnet or Haiku, chosen by the difficulty of the task,
  and never inherit the coordinator's model (the operator, 2026-11-21).
