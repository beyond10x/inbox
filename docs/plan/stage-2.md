# Wave plan: stage 2

The plan for epic:service-in-memory: `inbox serve` with the in-memory backend, and the CLI.
Approved as a whole by approval-record:stage-2-waves-1-to-5 (2026-11-21): waves 1 to 5 in order,
a report after each, a stop only on a red gate or an escalated finding.

Derived with `aep plan artifact waves --kind story --status active` (aep 0.80.0):
`5 wave(s), 13 collision(s), 0 unassessed`. Every collision is between stories of different waves.

| Wave | Stories | Serves | State |
|---|---|---|---|
| 1 | story:rust-workspace-and-model | the generated crate every later story builds on | delivered 2026-11-21 |
| 2 | story:core-and-memory-backend, story:signature-verifier | the rules complete in process; signatures | delivered 2026-11-21 |
| 3 | story:backend-test-kit, story:conformance-gate, story:http-service | the backend contract checked; the suite against the core; the listener | delivered 2026-11-21 |
| 4 | story:cli-client, story:declared-coverage | the client; no generated scenario left uncovered | running |
| 5 | story:service-acceptance | the suite over HTTP against a running service; closes the stage | planned |

The stage is not done while the conformance report's `conformance_status` is `inconclusive`
(50 uncovered refusals at the start of stage 2; story:declared-coverage owns them).

Before wave 2, three critics read the revision that answered the 4 findings of critic round 2
(review-result:acceptance-round-3, scope-round-3, design-round-3). Acceptance and scope approve.
Design returned 2 findings on story:core-and-memory-backend and story:conformance-gate: the
generated context port answers the external branches of `Answer` only, so the `PostMessage`
obligation asks a trait of `inbox-core`, `PostExternals`, for its own 3. Both stories say so now,
and the design critic approves that revision (review-result:design-round-4).

## Conventions of every wave

- Integration branch `wave/s2-<n>`, unit branches `impl/<story-slug>`, each forked from the
  integration branch.
- Every checkout is a managed worktree (`worktree create`). Its id is recorded below; its path is
  the manager's tree directory for that id. A unit builds into `target/` inside its own tree.
- Scratch of a unit is `w<n>/<story-slug>/` under the coordinating session's scratch directory.
- Builds run through `slot` with `CARGO_INCREMENTAL=0`. A tree's cache is discarded when its unit
  is merged.
- An implementor writes source and tests and commits nothing. The coordinator commits as
  `b10x-bot[bot]` through `b10x-gates bot`, and owns every `aep plan artifact` write.
- Sub-agents: `aep:implementor` and `aep:adversary`, on Opus.
- A wave closes on one run of `task check` on its integration branch, then a fast-forward of
  `main` and a bot push through `b10x-gates`. No release, no tag, no deployment.

## Wave 1

One unit, so no two surfaces overlap. The scope entries of the story are `inferred`; its body
cites the paths. No story of the planned wave left it.

Commits this wave makes: the opening store commit, 1 unit commit (more if a correction round
runs), the closing store commit, the fast-forward of `main`.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `db7c876`; no tree of an
earlier wave exists; 95 GB free on the root file system; no Rust build of this repository has
been measured, because none exists before this wave.

What the story's move to `implemented` needs: the lifecycle declares no evidence requirement
(`aep plan artifact lifecycle story`). The wave records the gate run as `test_result` and the
claim check as `verification`.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-1` | `inbox-s2-w1` | `w1/` | closed |
| story:rust-workspace-and-model | `impl/rust-workspace-and-model` | `inbox-s2-w1-rust` | `w1/rust-workspace-and-model/` | merged, `implemented` |

Result:

- Gate on `wave/s2-1` after the merge, `CARGO_INCREMENTAL=0 slot task check`: exit 0,
  "92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported", 35 cases of `inbox-xtask` passed
  (3 unit, 5 and 3 in the two adversary files, 24 in `tests/drift.rs`).
- Claim check by the coordinator, one optional field added and `task projections` run: the base
  `8abe907` exits 0, the unit exits 201 with "generated/rust/inbox drifted from a fresh synthesis
  of ess/".
- 3 unit commits: the unit, and one correction after each adversary pass. Pass 1: 4 findings
  (1 warning, 3 notes). Pass 2: 3 notes, 1 of them a regression of the first correction. All 7
  fixed; none was there before the unit, so the backlog gains nothing.
- Differs from the plan: the workspace members are `crates/*` only. The synthesized `Cargo.toml`
  declares its own `[workspace]`, so `generated/rust` is excluded and a crate depends on it by
  path. The synthesized code fails `cargo fmt --check` and clippy with warnings denied; the gate
  builds it and neither step reads it.
- Not shown by this wave: the Rust install step of `.github/workflows/check.yml` first runs when
  the wave reaches `main`.
- Build cost: a cold `cargo build --workspace` takes 4 s and 58 MB of `target/`, 109 MB with the
  generated crate's `server` feature (implementor's measurement); the gate ran 45 s in the pool.
  The integration tree's `target/` was 169 MB at close.
- Sub-agents, all on Opus: the implementor in 4 runs (171 tool uses, 33 min); the adversary in 2
  runs (43 tool uses, 10 min).

## Wave 2

Two units at once. Re-derived on `main` at `0ad1a7f` with
`aep plan artifact waves --kind story --status active`: both stories are in the first wave that
is left, and no collision is reported between them. Their scope entries are `inferred`:
`crates/inbox-core` for story:core-and-memory-backend, `crates/inbox-verify` and
`docs/design/signing.md` for story:signature-verifier. No story of the planned wave left it.

Two files both units would touch, and who owns them in this wave:

- `Cargo.lock`: each unit's build adds its crate and dependencies. Each unit keeps its own lock
  current; at the second merge the coordinator takes one side and regenerates it with
  `cargo metadata`.
- `AGENTS.md`: neither unit edits it. Each reports its Layout row and the coordinator adds both on
  the integration branch after the merges.

Neither unit edits the root `Cargo.toml`: a crate declares its dependencies in its own manifest.

The host operator accepted that this wave runs on the story text as revised after the round 3
read (2026-11-21); the revision changed neither scope nor wave order.

Commits this wave makes: the opening store commit, 1 commit per unit (more if a correction round
runs), 2 merges, 1 commit for `AGENTS.md` and the lock if it needs one, the closing store commit,
the fast-forward of `main`.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `0ad1a7f`; the 2 trees of wave
1 are removed; 176 GB free on the root file system; a cold build of the workspace with the
generated crate's `server` feature is 109 MB of `target/` (wave 1), so 2 unit trees and the
integration tree stay below 1 GB before the new dependencies.

What a story's move to `implemented` needs: no evidence requirement in the lifecycle. The wave
records the gate run as `test_result` and each claim check as `verification`.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-2` | `inbox-s2-w2` | `w2/` | closed |
| story:core-and-memory-backend | `impl/core-and-memory-backend` | `inbox-s2-w2-core` | `w2/core-and-memory-backend/` | merged, `implemented` |
| story:signature-verifier | `impl/signature-verifier` | `inbox-s2-w2-verify` | `w2/signature-verifier/` | merged, `implemented` |

Result:

- Gate on `wave/s2-2` after both merges (`7baec2f`), `CARGO_INCREMENTAL=0 slot task check`:
  exit 0, "92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported", 146 cargo test cases
  passed: 48 of `inbox-core` (37, 7 and 4 in its 3 targets), 63 of `inbox-verify` (13 unit,
  1 acceptance, 5 and 3 in the two adversary files, 34 in `tests/cases.rs`, 7 in `tests/jcs.rs`),
  35 of `inbox-xtask`. The run took 28 s in the pool; `target/` of the integration tree was
  335 MB afterwards.
- Claim checks by the coordinator, each on the base, on the unit and on mutated copies: recorded
  as `verification` evidence on both stories.
- story:core-and-memory-backend: 3 unit commits (`ff76d83`, `32b954f`, `9eba82c`). Adversary
  pass 1: 3 findings (two posts at once over clones of one store were both accepted; 2 mutants
  the cases did not catch). Pass 2: 2 notes on the first correction. All 5 introduced by the
  unit, all fixed.
- story:signature-verifier: 3 unit commits (`4125267`, `746046b`, `4de8174`). Adversary pass 1:
  6 findings, 5 introduced and fixed, 1 there before the unit (a backlog draft). Pass 2: 3
  findings on the first correction, 1 of them a regression (the duplicate check held about
  160 MiB for a key file of 32 801 bytes), all fixed.
- Differs from the plan:
  - The coordinator's brief told the core's implementor to follow the order of refusals in the
    contract text of `PostMessage` (`generated/rust/inbox/PLAN.md`). The ESS interpreted target
    answers another order, and the 92 scenarios pass both. The coordinator measured it with 12
    scenarios and 3 controls in a scratch copy of `ess/` and sent the unit back before its
    first commit. The story's Context states the measured order.
  - `inbox-core` has a third trait a backend implements, `Exclusive`, and an entry `handle` that
    runs one command at a time per store (finding 1 of pass 1). The backend contract for
    story:backend-test-kit and a later backend is the 2 storage ports and `Exclusive`.
  - `AGENTS.md` and the lock: the 3 Layout rows are in the closing commit; `Cargo.lock` was
    resolved inside the second merge (the wave branch's side, then `cargo metadata`).
- Decisions the coordinator took in the wave, each written into the story or the design note:
  an id of the key file is never reused, and an id two operator-signed entries carry is ignored
  in both; a key file with a member name carried twice is refused whole; a command's context
  that calls the entry again over the same store gets a panic naming the rule; `getrandom` 0.4,
  which the lock already held, in place of a third version.
- Not shown by this wave: a `schema` reply shape is only checked for a body that parses as JSON;
  no code path decodes an envelope from a request yet (story:http-service).
- Sub-agents, all on Opus. Counted from the transcripts, where each tool call appears twice: the
  core's implementor 171 tool uses in 4 runs, the verifier's implementor 140 in 3 runs; the
  adversaries 57 and 42 (core, pass 1 and 2) and 54 and 39 (verifier).

## Wave 3

Three units at once. Re-derived on `main` at `88b336a` with
`aep plan artifact waves --kind story --status active`: the 3 stories are the first wave that is
left, and no collision is reported among them (`3 wave(s), 7 collision(s), 0 unassessed`; every
collision is between stories of different waves). Their scope entries are `inferred`:
`crates/inbox-backend-testkit` for story:backend-test-kit; `crates/inbox-conformance`,
`Taskfile.yml` and `AGENTS.md` for story:conformance-gate; `crates/inbox-server` and
`docs/design/service.md` for story:http-service. No story of the planned wave left it.

Files more than one unit would touch, and who owns them in this wave:

- `Cargo.lock`: each unit's build adds its crate and dependencies. Each unit keeps its own lock
  current; at the second and third merge the coordinator takes the wave branch's side and
  regenerates it with `cargo metadata`.
- `AGENTS.md`: story:conformance-gate edits the Commands table (its scope). No unit edits the
  Layout table: each reports its row and the coordinator adds the 3 rows after the merges.
- `Taskfile.yml`: story:conformance-gate only.

No unit edits the root `Cargo.toml`, and none depends on another unit's crate: all 3 depend on
`crates/inbox-core`, story:http-service also on `crates/inbox-verify`.

Story text changed before dispatch, scope and wave order unchanged:

- Each of the 3 stories has one Context paragraph on what wave 2 settled: the entry
  `inbox_core::handle`, and `Exclusive` as the third part of the backend contract, which
  story:backend-test-kit checks too.
- story:http-service calls `inbox_verify::duplicate_member` on every request body and refuses a
  body with a member name twice: the host operator's decision of 2026-11-21 on finding 4 of
  review-result:adversary-signature-verifier-pass-1. story:refuse-duplicate-members stays in the
  backlog only if something of it is left after this wave.

Commits this wave makes: the opening store commit, 1 commit per unit (more if a correction round
runs), 3 merges, the closing commit with the Layout rows and the store, the fast-forward of
`main`.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `88b336a`; the 3 trees of
wave 2 are removed; 161 GB free on the root file system; the integration tree of wave 2 held
335 MB of `target/` after the gate, so 4 trees stay below 2 GB before the new dependencies.

What a story's move to `implemented` needs: no evidence requirement in the lifecycle. The wave
records the gate run as `test_result` and each claim check as `verification`.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-3` | `inbox-s2-w3` | `w3/` | closed |
| story:backend-test-kit | `impl/backend-test-kit` | `inbox-s2-w3-kit` | `w3/backend-test-kit/` | merged, `implemented` |
| story:conformance-gate | `impl/conformance-gate` | `inbox-s2-w3-conf` | `w3/conformance-gate/` | merged, `implemented` |
| story:http-service | `impl/http-service` | `inbox-s2-w3-http` | `w3/http-service/` | merged, `implemented` |

Result:

- Gate on `wave/s2-3` after the 3 merges (`3509bc0`), `CARGO_INCREMENTAL=0 slot task check`:
  exit 0, "92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported" on the interpreted target
  and against the core (`task conform`, `conformance_status: inconclusive`), 267 cargo test
  cases passed and 4 ignored: 29 of `inbox-backend-testkit`, 54 of `inbox-conformance` (1
  ignored, naming beyond10x/ess#799), 48 of `inbox-core`, 38 of `inbox-server` (3 ignored, each
  naming its backlog story), 63 of `inbox-verify`, 35 of `inbox-xtask`. The run took 107 s in
  the pool; `target/` of the integration tree was 502 MB afterwards. A second run with `CI=true`
  and `GITHUB_ACTIONS=true` set gave the same result.
- Claim checks by the coordinator, each on the base, on the unit and on mutated copies: recorded
  as `verification` evidence on the 3 stories.
- story:backend-test-kit: 3 unit commits (`8ed00d9`, `efe92d7`, `58ee30c`). Adversary pass 1:
  10 findings. Pass 2: 5 findings on the first correction. All 15 introduced by the unit, all
  fixed.
- story:http-service: 3 unit commits (`df9d07b`, `973b5a7`, `68aff6e`). Adversary pass 1:
  8 findings, 5 introduced and fixed (a slow unauthenticated connection held a connection slot
  and `stop` without end; an unbounded `max_wait` dropped the wait; 2 missing cases), 3 that were
  in the model before the unit (backlog drafts). Pass 2: 2 notes on missing cases, closed by the
  adversary's own cases.
- story:conformance-gate: 3 unit commits (`df1ecc5`, `3ff527d`, `2fecd3d`). Adversary pass 1:
  8 findings, 5 introduced and fixed, 1 declined (below), 2 that were in the core and the
  suite before the unit (backlog drafts). Pass 2: 6 findings on the first correction, 4
  introduced and fixed (`satisfies` passed on a view with no rows; the gate bound a report to
  the bytes of the suite only; a member stated twice in a step was ignored; lines of the
  first correction no case held), 2 that were in the suite before the unit (backlog drafts).
  The adversary's mutant script on the corrected unit: 54 mutants, 48 killed, 6 surviving,
  the 6 its review names as equivalent. No third pass ran.
- Differs from the plan:
  - story:http-service: the listener runs the generated `dispatch` inside the backend's
    `Exclusive` section over `inbox_core::system`. The story named the entry
    `inbox_core::handle`, which answers outcomes and no HTTP status. The story's Context states
    it.
  - story:http-service: `Config` has one limit the story did not name, `request_timeout`
    (default 10 s, `408`), and `stop` answers connections still sending with `503`. Both answer
    findings of adversary pass 1. story:cli-client gains one flag.
  - story:conformance-gate: a failing `task conform` exits 201 (go-task); the gate command
    exits 1. The story's Acceptance said "exits 1" and states both now.
  - story:conformance-gate: finding 3 of adversary pass 1 is declined. In a view row the runner
    reads an absent top-level member and `null` as one value; ESS's runner compares exactly.
    The coordinator measured the reason before declining: the generated Rust server leaves an
    unset `Optional` member out under every `presence:` value, the suite claims `null` under
    every value, and with the exact comparison 7 of the 92 scenarios fail on the unchanged core.
    Until beyond10x/ess#799 is closed the gate is weaker than ESS's runner at this point.
  - story:conformance-gate: 2 cases of adversary pass 1 were deleted. Their fixture declared 1
    of the 4 types its root reaches, which ESS answers with an error and not a failure; a case
    of the unit holds the same rows with the full declarations.
  - story:conformance-gate: finding 2 of adversary pass 2 asked the gate to hold a report to the
    implementation it names. The gate takes `--implementation` as an option and the Taskfile
    passes it; a report without that member stays a valid report where the option is not given.
  - story:conformance-gate: one case holds a reading ESS's runner does not share. A plain
    snapshot followed by `expect_complete_subject_unchanged` is an error here; ESS compares the
    rows. The synthesizer always pairs the two steps, so no scenario of the suite reaches it.
  - story:conformance-gate: the 2 mutants of the core (no duplicate-key check; `Expire` not
    writing `decided_option`) ran at the first correction and failed `task conform`. They were
    not run again after the second: the machine had under 105 GB free for a second tree copy.
  - story:backend-test-kit: 1 case of adversary pass 2 was deleted with its helper and imports.
    `check_exclusive` holds a command for 500 ms and the crate documentation states that a
    section which admits a second command only after a longer hold is not seen; it cannot check
    that a factory answers separate stores.
- Decisions of the host operator in this wave (2026-11-21): the duplicate member check is part
  of story:http-service; the guard "a post's `from` is the caller" goes into `ess/` and no copy
  of it is written in the listener, with the gate stated under Backlog.
- Defects filed on beyond10x/ess: #798 (the refusal order text of `PLAN.md` against the
  interpreted target) and #799 (a cleared `Optional` view member: claim, interpreted target,
  row schema and generated server disagree).
- For the next waves. story:cli-client: one flag more for `request_timeout`; a client may be
  answered `408`, or `503` with `{"refused": "stopping"}`, and can send again.
  story:declared-coverage: `task coverage` writes `ess-conformance/53`, which the runner
  refuses (it reads `/52` only), and `gate::passing_status` requires `inconclusive` today.
  story:service-acceptance: the wire target implements the trait `Target`; `gate` needs
  `--suite`; every request needs the 3 signature headers; views are filtered by caller.
- Sub-agents, all on Opus, in tool uses as each run reported them: the kit's implementor 121 in
  3 runs, the listener's 179 in 2 runs, the conformance runner's 375 in 4 runs; the adversaries
  47 and 55 (kit, pass 1 and 2), 51 and 60 (listener), 91 and 93 (conformance).

## Publication and release 0.1.0

Outside the 5 waves, on the operator's instruction of 2026-11-21 (it replaces "private, no
release, no tag" of the stage 2 approval; "no deployment" stays): story:public-docs-and-release,
`implemented`.

- The history that became public was scanned 3 times (29, 34 and 35 commits; the last on
  `main` at `16cd7cf`: 480 text blobs, 0 findings). No fix was needed and no history rewritten.
- The repository is public since 2026-11-21T11:54:54Z. The site is
  <https://beyond10x.github.io/inbox/>, published from `main` by
  `.github/workflows/b10x-docs-site.yml`; Pages builds from the workflow, and the environment
  `github-pages` deploys from `main` only.
- Release 0.1.0: the annotated tag `0.1.0` on `16cd7cf`, written by the organization bot, and
  the GitHub Release <https://github.com/beyond10x/inbox/releases/tag/0.1.0>, published
  2026-11-21T11:57:53Z. `release.yml` passed on `main` by hand and on the tag.
- Not done: rulesets. `b10x-gates api` refuses a write to a ruleset as the owner's act. The
  repository has 0 rulesets; the 5 payloads (the 3 required checks on `main`, and the bot's
  authority and identity on branches and tags) are with the operator.

## Wave 4

Two units at once. Re-derived on `main` at `16cd7cf` with
`aep plan artifact waves --kind story --status active`: story:cli-client and
story:declared-coverage are the first wave that is left, and no collision is reported between
them (`3 wave(s), 7 collision(s), 0 unassessed`; the second of the 3 waves was
story:public-docs-and-release, which is `implemented` now). Their scope entries are `inferred`:
`crates/inbox-cli`, `README.md` and `AGENTS.md` for story:cli-client; `ess/scenarios`,
`ess/ess-inputs.yaml`, `tools/scenarios.py`, `Taskfile.yml` and `docs/design/inbox.md` for
story:declared-coverage. No story of the planned wave left it.

Files more than one unit would touch, and who owns them in this wave:

- `Cargo.lock`: story:cli-client only (its crate and dependencies).
- `AGENTS.md` and `README.md`: story:cli-client (the Commands rows of the binary; install and
  first use). story:declared-coverage edits neither: it reports the rows and numbers that
  change, and the coordinator writes them after the merges.
- `Taskfile.yml`: story:declared-coverage only.
- `generated/`: story:declared-coverage, through `task generate` and `task projections` only,
  where a scenario changes what they write.
- `website/`, `CHANGELOG.md` and the Layout table of `AGENTS.md`: the coordinator, at the close,
  from the gate's numbers.

Story text changed before dispatch, scope and wave order unchanged:

- story:cli-client has one Context paragraph on what wave 3 settled: the library the binary
  starts, the 2 flags more than its list (`--request-timeout`, `--max-wait`), the 3 signature
  headers, the answers of the service itself that the client names, and loopback only.
- story:declared-coverage has one Context paragraph: each scenario it adds also runs against
  the core in `task conform`; a scenario that fails there is reported and not fixed in this
  unit; the report of `task conform` stays `inconclusive` after this story, because that task
  synthesizes the suite without declared coverage, and the switch of `task conform` to the suite
  `task coverage` writes is read at the opening of wave 5 (story:service-acceptance owns
  `crates/inbox-conformance/` and the `conform` task).

Revision during the wave, decided by the host operator on 2026-11-21 before it was made:
story:declared-coverage covers 42 of the 50 refusals with scenarios; the other 8 are the
invariants of `reply` and `decided_option` at the views `Pending` and `Unanswered`, which no row
can carry. The story's scope gains `ess/domains/messaging.yaml` (the 2 fields leave the 2 views,
4 lines) and `generated/`. Conditions: the changelog names the change of the wire rows against
0.1.0; before the merge `task coverage` exits 0 and the suite it writes reports a status other
than `inconclusive`; story:cli-client is checked against the regenerated crate at the merge; a
regeneration that changes more than the rows of the 2 views stops the unit. No release with it.

Second revision, decided by the host operator on 2026-11-21 on the adversary's first pass on
story:declared-coverage: `answered_by` and `refusal` are always absent in the views `Pending` and
`Unanswered` for the same reason, and leave them in the same change (4 more lines of
`ess/domains/messaging.yaml`), under the same conditions. The changelog names 2 views and 4
fields. Standing from that answer: a member of a view that no sequence of commands can set,
found before the next release, is removed in the same window and reported in the wave report.

Commits this wave makes: the opening commit (this section, the store, the ruleset state in
`AGENTS.md` and `docs/releasing.md`), 1 commit per unit (more if a correction round runs), 2
merges, the closing commit with the Layout row, the numbers and the store, the fast-forward of
`main`.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `16cd7cf`; the trees of wave
3 and of the publication are removed; 85 GB free on the root file system.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-4` | `inbox-s2-w4` | `w4/` | open |
| story:cli-client | `impl/cli-client` | `inbox-s2-w4-cli` | `w4/cli-client/` | dispatched |
| story:declared-coverage | `impl/declared-coverage` | `inbox-s2-w4-cov` | `w4/declared-coverage/` | dispatched |

## Backlog

Drafts filed from findings wait here until every wave of this plan is delivered. Accepting one
is the operator's act.

- story:post-refusal-order-scenarios (wave 2): authored scenarios hold the order in which
  `PostMessage` refuses when two refusals apply. The suite of 92 scenarios passes an
  implementation that reads a row before it has checked the signature.
- story:refuse-duplicate-members (wave 2): closed as covered and archived without having
  been accepted. story:http-service calls `inbox_verify::duplicate_member` on every request
  body and answers `400` before any command runs (the host operator's decision,
  2026-11-21). Nothing of the draft is left.

The host operator's gate (2026-11-21): the 3 drafts of story:http-service's first adversary pass
are the first work after wave 5 and land before any of: a listener bound outside loopback, a
NATS backend wave, a deployment, use by a real session. Each carries `blocks` relations to
epic:two-hosts-and-wake and epic:jetstream-backend. Until story:post-from-is-the-caller lands, a
recipient can post a sender's signed envelope again under its own request signature. Its guard
goes into `ess/` (`from != caller.principal`); no copy of it is written in the listener.

- story:post-from-is-the-caller (wave 3): a post or an answer is refused when its `from` is not
  the caller. Its case `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal`
  is in `crates/inbox-server/tests/adversary.rs` with an ignore attribute naming the story.
- story:foreign-message-id-answers-as-unknown (wave 3): a third principal gets one answer for a
  message between others and for no message. Today `not-recipient` (403) against
  `unknown-message` (404) tells any authenticated client whether a message id exists.
- story:dedup-key-scope (wave 3): a dedup key of one sender does not refuse another sender's
  post. Today `duplicate-key` compares the key alone over every message.

Drafts from the conformance unit, each a gap of the suite and no defect of the core as built:

- story:read-decision-expires-on-silence-scenario (wave 3): a core that deletes a read decision
  at expiry passes every gate; the one scenario for decided-on-silence never reads the message.
- story:external-refusal-order-cases (wave 3): a core that asks the 3 external refusals of
  `PostMessage` in reverse order passes `task conform` and its own tests.
- story:silence-leaves-the-other-fields-scenario (wave 3): a core that clears `decision` or
  `default_on_silence`, empties `body` or rewrites `created` or `thread` after deciding on
  silence passes the 92 scenarios.
- story:expire-publishes-once (wave 3): a core that publishes the event of `Expire` twice passes
  `task conform` and the core's own tests.

Defects filed on beyond10x/ess from this wave: #798 and #799 (see the result of wave 3).
