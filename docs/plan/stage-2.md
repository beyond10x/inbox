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
| 3 | story:backend-test-kit, story:conformance-gate, story:http-service | the backend contract checked; the suite against the core; the listener | running |
| 4 | story:cli-client, story:declared-coverage | the client; no generated scenario left uncovered | planned |
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
| integration | `wave/s2-3` | `inbox-s2-w3` | `w3/` | open |
| story:backend-test-kit | `impl/backend-test-kit` | `inbox-s2-w3-kit` | `w3/backend-test-kit/` | dispatched |
| story:conformance-gate | `impl/conformance-gate` | `inbox-s2-w3-conf` | `w3/conformance-gate/` | dispatched |
| story:http-service | `impl/http-service` | `inbox-s2-w3-http` | `w3/http-service/` | dispatched |

## Backlog

Drafts filed from findings wait here until every wave of this plan is delivered. Accepting one
is the operator's act.

- story:post-refusal-order-scenarios (wave 2): authored scenarios hold the order in which
  `PostMessage` refuses when two refusals apply. The suite of 92 scenarios passes an
  implementation that reads a row before it has checked the signature.
- story:refuse-duplicate-members (wave 2): the listener refuses a request whose JSON carries a
  member name twice. The generated reader keeps the first of two members, so one signature
  covers two texts. `crates/inbox-verify` holds the check (`duplicate_member`); the call in the
  listener is not part of story:http-service as accepted.
