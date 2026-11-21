# Wave plan: stage 2

The plan for epic:service-in-memory: `inbox serve` with the in-memory backend, and the CLI.
Approved as a whole by approval-record:stage-2-waves-1-to-5 (2026-11-21): waves 1 to 5 in order,
a report after each, a stop only on a red gate or an escalated finding.

Derived with `aep plan artifact waves --kind story --status active` (aep 0.80.0):
`5 wave(s), 13 collision(s), 0 unassessed`. Every collision is between stories of different waves.

| Wave | Stories | Serves | State |
|---|---|---|---|
| 1 | story:rust-workspace-and-model | the generated crate every later story builds on | delivered 2026-11-21 |
| 2 | story:core-and-memory-backend, story:signature-verifier | the rules complete in process; signatures | planned |
| 3 | story:backend-test-kit, story:conformance-gate, story:http-service | the backend contract checked; the suite against the core; the listener | planned |
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

## Backlog

Drafts filed from findings wait here until every wave of this plan is delivered. None yet.
