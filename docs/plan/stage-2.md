# Wave plan: stage 2

The plan for epic:service-in-memory: `inbox serve` with the in-memory backend, and the CLI.
Approved as a whole by approval-record:stage-2-waves-1-to-5 (2026-11-21): waves 1 to 5 in order,
a report after each, a stop only on a red gate or an escalated finding.

Derived with `aep plan artifact waves --kind story --status active` (aep 0.80.0):
`5 wave(s), 13 collision(s), 0 unassessed`. Every collision is between stories of different waves.

| Wave | Stories | Serves | State |
|---|---|---|---|
| 1 | story:rust-workspace-and-model | the generated crate every later story builds on | running |
| 2 | story:core-and-memory-backend, story:signature-verifier | the rules complete in process; signatures | planned |
| 3 | story:backend-test-kit, story:conformance-gate, story:http-service | the backend contract checked; the suite against the core; the listener | planned |
| 4 | story:cli-client, story:declared-coverage | the client; no generated scenario left uncovered | planned |
| 5 | story:service-acceptance | the suite over HTTP against a running service; closes the stage | planned |

The stage is not done while the conformance report's `conformance_status` is `inconclusive`
(50 uncovered refusals at the start of stage 2; story:declared-coverage owns them).

Before wave 2: a critic reads the revision that answers the 4 findings of critic round 2.

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
| integration | `wave/s2-1` | `inbox-s2-w1` | `w1/` | open |
| story:rust-workspace-and-model | `impl/rust-workspace-and-model` | `inbox-s2-w1-rust` | `w1/rust-workspace-and-model/` | dispatched |

## Backlog

Drafts filed from findings wait here until every wave of this plan is delivered. None yet.
