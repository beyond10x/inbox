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
| 4 | story:cli-client, story:declared-coverage | the client; no generated scenario left uncovered | delivered 2026-11-21 |
| 5 | story:service-acceptance | the suite over HTTP against a running service; closes the stage | delivered 2026-11-21 |
| 6 | story:post-from-is-the-caller, story:dedup-key-scope, story:foreign-message-id-answers-as-unknown | the 3 security findings of wave 3 closed before the service is used | running |

Wave 6 is not part of the plan approved as waves 1 to 5. It is the plan derived after those
were delivered, from the 3 drafts the host operator accepted on 2026-11-21
(approval-record:wave-6-security-drafts-accepted). epic:service-in-memory stays `active`
until it is delivered.

The stage is not done while the conformance report's `conformance_status` is `inconclusive`
(50 uncovered refusals at the start of stage 2; story:declared-coverage owns them). Since wave 5
the report of the run against the core says `passed` (the result of wave 5, below).

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
`main`. A third merge was added: the correction of story:declared-coverage after its second
adversary pass.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `16cd7cf`; the trees of wave
3 and of the publication are removed; 85 GB free on the root file system.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-4` | `inbox-s2-w4` | `w4/` | closed |
| story:cli-client | `impl/cli-client` | `inbox-s2-w4-cli` | `w4/cli-client/` | merged, `implemented` |
| story:declared-coverage | `impl/declared-coverage` | `inbox-s2-w4-cov` | `w4/declared-coverage/` | merged, `implemented` |

Result:

- Gate on `wave/s2-4` after the 3 merges (`a6b1660`),
  `CARGO_INCREMENTAL=0 slot task check`:
  exit 0, `task scenarios-drift` "24 scenarios, 0 path(s) differ from tools/scenarios.py",
  `task coverage` "93 selected scenario(s), 24 authored source(s), 89 refusal occurrence(s), 89
  covered by authored", "93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported" on the
  interpreted target and against the core (`task conform`, `conformance_status: inconclusive`),
  319 cargo test cases passed and 5 ignored: the 267 and 4 of wave 3, whose crates did not
  change, and 52 of `inbox-cli` (1 ignored, naming its backlog story). The run took 66 s in the
  pool. A second run with `CI=true` and `GITHUB_ACTIONS=true` set gave the same result. The
  documentation site builds with the new page (`npm --prefix website run build`, exit 0).
- story:cli-client: 3 unit commits (`0c2426d`, `e411f79`, `40a9afb`). Adversary pass 1: 21
  findings, all introduced by the unit: 8 red cases, 9 mutants the unit's suite passed, 4
  judgement findings. 20 are fixed. 1 is declined (below). Pass 2, on the first correction: 3
  findings, 1 blocker and 2 notes, all fixed; 2 mutants survive, and the adversary reads both as
  equivalent. No third pass ran: the coordinator ran the crate's cases after the second
  correction (52 passed, 1 ignored).
- story:declared-coverage: 4 unit commits (`f6d8147`, `029f462`, `d35d455`, `29575e0`).
  Adversary pass 1, on `029f462`: all 42 claims true, 3 findings, no red case, all 3 fixed in
  `d35d455`. Pass 2, on `d35d455`: 2 findings, no red case, 0 of 6 mutants pass the suite. 1
  was in the model before the unit and falls under the standing rule (below). 1 was introduced:
  `tools/scenarios.py` ran its guard after it had written the files, and no step of the gate
  held `ess/scenarios/` to the script. Both are fixed in `29575e0`. No third pass ran: the
  coordinator checked the third correction (the model diff, every content change under
  `generated/`, `task scenarios-drift` on a copy with an edited, an added and a removed file,
  `task coverage`, the status of the coverage suite).
- Differs from the plan:
  - story:cli-client: the blocker of adversary pass 2 came from the coordinator's own order in
    the first correction round ("a post answered `400` or `413` is final and leaves the outbox
    file"). A `400` or `413` of the service reflects its settings or the transport, so the
    order deleted an envelope that could still be delivered. The order is withdrawn; every
    refusal of the service itself keeps the envelope.
  - story:cli-client: finding 1 of adversary pass 1 is declined. `inbox repost` posts a note
    again after its recipient read it. The coordinator measured the reason before declining:
    the service answers a read note and a note a restart dropped alike, and holds no record of
    a handled message, so the client cannot tell them apart. The case is ignored and names the
    backlog draft; `inbox repost --help`, the README and the site say it.
  - story:cli-client: the client refuses an `INBOX_URL` outside loopback (exit 2). The story
    did not ask for it; it answers a judgement finding of pass 1 while
    story:post-from-is-the-caller is open.
  - story:declared-coverage: 42 of the 50 uncovered refusals are covered by scenarios, not 50.
    The other 8 were invariants of `reply` and `decided_option` at `Pending` and `Unanswered`,
    which no row can carry. The host operator decided on 2026-11-21 to remove the 2 fields from
    the 2 views, and on finding 3 of pass 1 to remove `answered_by` and `refusal` with them: 8
    lines of `ess/domains/messaging.yaml`. With the member of `Replies` below it is one change
    of the wire rows against 0.1.0, 3 views and 5 members, named in `CHANGELOG.md`. Each of the
    3 regenerations rewrote 34 files under `generated/`, with content changes only in the rows
    of those views. The conditions were shown before each merge: the reachability argument is
    in the story, `task coverage` exits 0 with 89 of 89, the suite it writes reports `passed`
    on the interpreted target, and the gate on the merged branch built and tested
    `crates/inbox-cli` against the regenerated crate.
  - story:declared-coverage: `task check` has one step the story did not name,
    `task scenarios-drift`. It answers finding 2 of pass 2.
  - story:declared-coverage: `task conform` still reports `inconclusive`. It synthesizes
    without declared coverage, and the runner refuses the suite `task coverage` writes (the
    key `coverage`, and row paths with a dot). The story's Context said so before dispatch.
- Decisions of the host operator in this wave (2026-11-21): the 2 model changes above, and the
  standing rule that a view member no sequence of commands can set, found before the next
  release, is removed in the same window and reported. The rule was applied once: `refusal`
  left the view `Replies`, which holds answered messages only and which only `Refuse/refused`
  could fill (finding 1 of pass 2; 1 line of the model). The adversary's script over the model
  reports no other such member in the 9 views.
- For wave 5. story:service-acceptance: the runner reads the coverage suite
  (`ess-conformance/65` from `--suite-format 5`: the key `coverage` and dotted row paths),
  `task conform` runs that suite, and `gate::passing_status` accepts `passed`; until then the
  status against the core is `inconclusive` and stage 2 is not done. The fixtures of
  `crates/inbox-cli/tests/support/mod.rs` start `inbox serve` and make keys at run time. The
  client exits 1 with the service's own text for `400`, `401`, `408`, `413` and `503`.
- Sub-agents, all on Opus, in tool uses as each run reported them: the client's implementor 224
  in 3 runs, the coverage implementor 203 in 3 runs; the adversaries 88 and 80 (client, pass 1
  and 2), 90 and 75 (coverage).

## Wave 5

One unit. Re-derived on `main` at `e8229fd` with
`aep plan artifact waves --kind story --status active`: story:service-acceptance is the one
story left (`1 wave(s), 0 collision(s), 0 unassessed`). Its scope entries are `inferred`:
`crates/inbox-conformance`, `tests/acceptance`, `Taskfile.yml` and `AGENTS.md`. No story of the
planned wave left it.

Story text changed before dispatch, wave order unchanged:

- One Context paragraph on what wave 4 settled: the binary and its flags, the fixtures that
  start the service and make keys at run time, the exit statuses, and that the repost check
  uses a message its recipient has not read.
- One Context paragraph and one Acceptance sentence on the conformance status. The runner
  reads the coverage suite (`ess-conformance/65`), `task conform` runs that suite against the
  core and against the service, and the gate accepts `passed`; the run against the core reports
  `conformance_status: passed`. The files are inside the story's scope
  (`crates/inbox-conformance/`, `Taskfile.yml`). The reason is the host operator's condition of
  the stage 2 approval: the stage is not done while the status is `inconclusive`. Reported to
  the host operator with the wave 4 report.
- The Scope gains the root `Cargo.toml` and `Cargo.lock`, for the case that `tests/acceptance`
  is a workspace member: the members are the glob `crates/*` today.

Files and who owns them in this wave: everything in the story's Scope is the unit's.
`website/`, `README.md`, `CHANGELOG.md`, the Layout table and the numbers of `AGENTS.md` are the
coordinator's at the close, from the gate's numbers.

Commits this wave makes: the opening commit (this section and the store), 1 commit for the unit
(more if a correction round runs), 1 merge, the closing commit, the fast-forward of `main`.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `e8229fd`; the trees of wave
4 are removed; 130 GB free on the root file system.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-5` | `inbox-s2-w5` | `w5/` | closed |
| story:service-acceptance | `impl/service-acceptance` | `inbox-s2-w5-acc` | `w5/service-acceptance/` | merged, `implemented` |

Result:

- Gate on `wave/s2-5` after the merge (`0c9a15a`), `CARGO_INCREMENTAL=0 slot task check`: exit 0.
  `task conform`, the core lane: "93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported",
  `conformance_status: passed`. The serve lane: "93 scenarios: 20 passed, 0 failed, 0 error, 73
  unsupported", `conformance_status: failed`, held to the 73 lines of
  `crates/inbox-conformance/unsupported.tsv`. `task acceptance`: "3 checks passed".
  `cargo test --workspace`: 343 cases passed and 9 ignored (319 and 5 after wave 4; 24 new
  cases in `crates/inbox-conformance`; the 4 new ignored ones are the 3 acceptance checks and the
  adversary's acceptance case, which `task acceptance` runs or duplicates). The run took
  79 s in the pool. A second run with `CI=true` and `GITHUB_ACTIONS=true` set gave the same
  result. The documentation site builds (`npm --prefix website run build`, exit 0).
- The condition of the stage 2 approval is met: the report of the run against the core says
  `conformance_status: passed`, and the gate refuses `inconclusive`.
- story:service-acceptance: 3 unit commits (`5e5058c`, `b919136`, `82b55fd`). Adversary pass 1,
  on `5e5058c`: 7 findings, 2 warnings and 5 notes, 4 red cases, all introduced by the unit. 5
  are fixed in `b919136`: the wire target ignored the HTTP status of an answer; acceptance check
  3 could not fail; the gate compared only the ids of the unsupported list; a row path with an
  empty member; the coverage block read more loosely than its documentation said. 2 are answered
  without a code change (below). Pass 2, on `b919136`: 3 notes, 2 red cases, all fixed in
  `82b55fd`: a `refused` entry without `covered_by` was refused where ESS reads it; the runner
  required the `events` catalog in every suite; `gate --results` did not hold the results to the
  report. No third pass ran: the coordinator ran the gate on `82b55fd`.
- Differs from the plan:
  - 20 of the 93 scenarios run over HTTP. 73 are unsupported there, each for one or more of 4
    reasons: a caller whose principal and inbox differ (64; over HTTP both are the principal of
    the caller's key, decision 5 of `docs/design/inbox.md`), a step as the Sweeper (35), a step
    as no actor (9), a row claim on the stored `signature` (7). The story expected unsupported
    scenarios and did not give a number.
  - Events are witnessed over HTTP: each answer carries its `published` list. The story's
    Outcome and the coordinator's brief assumed they were not.
  - A step that expects `unknown-option` or `shape-mismatch` is sent as the suite states it and
    the service decides, where the story said such a step is unsupported when the envelope
    carries no mismatch. Testing for the mismatch would write a rule of the specification a
    second time. The 3 such scenarios carry the mismatch and pass. The story's text says so now.
  - 8 of the 12 changed services the adversary built stay green in the core lane, the serve lane
    and the 3 checks (the read rule, the request signature and its window, the lapse of a key and
    of a delegation, the binding of a key to `from`, the delegation pair, `Messages` and `Due`
    served to a client). The tests of `crates/inbox-server` and `crates/inbox-verify` fail on
    each, so `task check` fails. Not changed in the unit; `website/docs/limits.md` and
    `CHANGELOG.md` say it.
  - The finding of pass 2 on the `events` catalog was in the runner before the unit. It is fixed
    in the unit and not filed as a draft: the unit's claim is that the runner reads the suite
    `task coverage` writes.
  - Declined by the coordinator: counting the `401` an unsigned request gets as "not granted to
    no actor" (1 more scenario over HTTP). The model states no such mapping.
  - `tests/acceptance/tests/adversary.rs` holds 1 ignored case that asserts what check 3 now
    asserts. The implementor's removal of it was refused by a permission check, and the
    coordinator did not remove it in its place. It stays; no task runs it.
  - The implementor's first report counted 312 passing cases; the gate log held 330. The numbers
    in `AGENTS.md` are the coordinator's sums over the gate log.
- Sub-agent runs: the implementor 391 tool uses in 3 runs; the adversary 101 and 54.
- Open after stage 2: the 3 security drafts of the Backlog below are the first work, before a
  listener outside loopback, a NATS backend wave, a deployment or use by a real session. Until
  story:post-from-is-the-caller lands, a caller with a valid key can post an envelope whose
  `from` is another principal (the replay named in the result of wave 3).

## Wave 6

3 units, one after another. The 3 stories were drafts of the Backlog below; the host operator
accepted them on 2026-11-21 and ordered them run as one wave, with the gate of the earlier waves,
a report after the wave and no release (approval-record:wave-6-security-drafts-accepted).

Scoped by 3 `aep:story-scoper` runs on `main` at `7b43de0`; each story's Scope section and its
typed entries are theirs, with the coordinator's corrections under each section. Derived with
`aep plan artifact waves --kind story --status active` (aep 0.80.0):

```
wave 1
  story:post-from-is-the-caller (inferred)
wave 2
  story:dedup-key-scope (inferred)
wave 3
  story:foreign-message-id-answers-as-unknown (inferred)
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown AGENTS.md
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown CHANGELOG.md (inferred)
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown README.md
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown crates/inbox-server/tests/adversary.rs
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown docs/design/inbox.md (inferred)
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown ess/domains/messaging.yaml
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown ess/scenarios (inferred)
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown generated
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown tools/scenarios.py (inferred)
collision: story:dedup-key-scope story:foreign-message-id-answers-as-unknown website/docs/limits.md
collision: story:dedup-key-scope story:post-from-is-the-caller AGENTS.md
collision: story:dedup-key-scope story:post-from-is-the-caller CHANGELOG.md (inferred)
collision: story:dedup-key-scope story:post-from-is-the-caller README.md
collision: story:dedup-key-scope story:post-from-is-the-caller crates/inbox-core/src/obligations.rs
collision: story:dedup-key-scope story:post-from-is-the-caller crates/inbox-core/tests (inferred)
collision: story:dedup-key-scope story:post-from-is-the-caller crates/inbox-server/tests/adversary.rs
collision: story:dedup-key-scope story:post-from-is-the-caller docs/design/inbox.md
collision: story:dedup-key-scope story:post-from-is-the-caller ess/domains/messaging.yaml
collision: story:dedup-key-scope story:post-from-is-the-caller ess/ess-inputs.yaml (inferred)
collision: story:dedup-key-scope story:post-from-is-the-caller ess/scenarios (inferred)
collision: story:dedup-key-scope story:post-from-is-the-caller generated
collision: story:dedup-key-scope story:post-from-is-the-caller tools/scenarios.py (inferred)
collision: story:dedup-key-scope story:post-from-is-the-caller website/docs/limits.md
collision: story:dedup-key-scope story:post-from-is-the-caller website/docs/status.md (inferred)
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller AGENTS.md
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller CHANGELOG.md
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller README.md
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller crates/inbox-conformance/unsupported.tsv (inferred)
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller crates/inbox-server/tests/adversary.rs
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller docs/design/inbox.md (inferred)
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller docs/design/service.md (inferred)
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller ess/domains/messaging.yaml
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller ess/scenarios
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller generated
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller tools/scenarios.py
collision: story:foreign-message-id-answers-as-unknown story:post-from-is-the-caller website/docs/limits.md
3 wave(s), 36 collision(s), 0 unassessed
```

Every pair collides, on the model file `ess/domains/messaging.yaml`, on `generated/` (each
regeneration rewrites the digest line of every generated file) and on the scenario script. The
stories cannot run at once. They run as one wave all the same, as ordered: one integration
branch, one closing gate, one report, and the units in sequence, each forked from the
integration branch after the unit before it is merged. The order is story:post-from-is-the-caller
(it closes the replay, and the per-sender key of the second story stops a third principal only
once `from` is the caller), story:dedup-key-scope, story:foreign-message-id-answers-as-unknown.
It is recorded as `depends_on` relations, which is why the verb prints it.

Settled by the coordinator before dispatch, in each story's Decisions section; the 2 drafts that
left a point open "for whoever accepts this" were accepted without an answer to it:

- story:post-from-is-the-caller: the outcome is `not-sender` with the error `NotSender`, first
  outcome of `PostMessage` and of `Answer`.
- story:dedup-key-scope: the scope of a dedup key is the sender (`from`).
- story:foreign-message-id-answers-as-unknown: the model names the case as the outcome
  `foreign-message`, and the listener answers that one outcome as `unknown-message`. The model
  alone cannot give the answer on ESS 0.96.0: a second outcome named `unknown-message` is
  refused (`ESS-COMMAND-006`), and a caller-guarded outcome is answered 403 under its own name.

Each was probed on a copy of the model with `ess specify validate` and
`ess generate --kind openapi`; synthesis and the suite were not run.

Files and who owns them: a unit owns `ess/domains/messaging.yaml`, `tools/scenarios.py` and what
`task scenarios` writes, `generated/` (regenerated, never edited), the crates and their tests,
`crates/inbox-conformance/unsupported.tsv`, `docs/design/inbox.md` and `docs/design/service.md`.
`website/`, `README.md`, `CHANGELOG.md`, the count lines and the backlog rule of `AGENTS.md` and
this page are the coordinator's at the close, from the gate's numbers.

Sub-agents: `aep:implementor` and `aep:adversary`, on Opus, one of each per unit, the adversary
at most 2 passes (aep plugin 0.26.0).

Commits this wave makes: the opening commit (this section and the store), 1 commit per unit and
1 per correction round, 3 merges, the closing commit, the fast-forward of `main` after the
opening commit and after the close.

Pre-flight (2026-11-21): the primary checkout is clean on `main` at `7b43de0`; the trees of wave
5 are removed; 125 GB free on the root file system. The 3 scoper runs: 43, 44 and 65 tool
uses.

| Unit | Branch | Worktree id | Scratch | Stage |
|---|---|---|---|---|
| integration | `wave/s2-6` | `inbox-s2-w6` | `w6/` | open |
| story:post-from-is-the-caller | `impl/post-from-is-the-caller` | `inbox-s2-w6-from` | `w6/post-from-is-the-caller/` | next |
| story:dedup-key-scope | `impl/dedup-key-scope` | `inbox-s2-w6-key` | `w6/dedup-key-scope/` | waits for the unit above |
| story:foreign-message-id-answers-as-unknown | `impl/foreign-message-id` | `inbox-s2-w6-foreign` | `w6/foreign-message-id/` | waits for the unit above |

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

The 3 were accepted on 2026-11-21 and are wave 6, above. They stay listed here until that
wave is delivered.

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

Draft from the client unit:

- story:repost-does-not-redeliver-a-handled-note (wave 4): `inbox repost` delivers a kept
  envelope again only when the service no longer holds the message and the recipient has not
  handled it. Today a note its recipient already read is posted again and answered `posted`.
  Its case `repost_does_not_deliver_again_a_note_its_recipient_already_read` is in
  `crates/inbox-cli/tests/adversary.rs` with an ignore attribute naming the story.

Defects filed on beyond10x/ess in wave 3: #798 and #799 (see the result of wave 3).
