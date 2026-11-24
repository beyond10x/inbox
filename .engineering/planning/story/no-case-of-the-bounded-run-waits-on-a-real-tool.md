---
format: aep.planning-md/3
id: story:no-case-of-the-bounded-run-waits-on-a-real-tool
kind: story
status: implemented
title: No case of the bounded run waits on a real tool or a fixed sleep
summary: '3 cases of the bounded test run take 4 to 19 s under load: one runs the synthesis of ess, one builds a fixture crate with cargo, one sleeps 4 s.'
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/inbox-cli/tests/adversary.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_outbox_expiry.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: cited
  path: crates/inbox-xtask/tests/adversary_bounded_run.rs
- confidence: cited
  path: crates/inbox-xtask/tests/drift.rs
- confidence: cited
  path: crates/inbox-xtask/tests/main.rs
- confidence: cited
  path: crates/inbox-xtask/tests/synthesis.rs
- confidence: cited
  path: crates/inbox-xtask/tests/tools/test_step.rs
- confidence: cited
  path: docs/checks.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T11:58:46Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T11:58:46Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T15:34:37Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":7}}}
---
## Outcome

No case of the bounded test run waits on a real tool or on a fixed sleep: the 3 cases the
measurement of story:the-test-step-holds-its-bound-with-928-cases names take under 5 s each at
a load average of 100 on the 20 CPUs of the build host.

Measured on 2026-11-23 in 2 runs of the workspace's cases in the pool:

- `inbox-xtask::main drift::an_optional_field_added_to_the_specification_is_drift` runs the
  synthesis of `ess` on a copy of the specification: 0.19 s of CPU time, 5.8 s alone, 8 to 17 s
  in a run. Its time is the sync of the files the synthesis writes.
- `inbox-xtask::main tools::test_step::a_run_over_the_bound_leaves_no_test_process_running`
  builds a fixture crate with cargo on every run: 8 to 19 s in a run, waiting on the lock of
  cargo and on a slot.
- `inbox-cli::main adversary::an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires`
  sleeps 4 s until the clock has passed `expires`: 4.3 s.

The runner marks a case slow at 30 s and ends it at 60 s; the first 2 cases are the ones
nearest to that under load.

## Acceptance

- Each of the 3 cases takes under 5 s in 2 runs of `task test` at the load the host has, each
  run with its load average; the case of the fixed sleep waits for the fact (the clock has
  passed `expires`) and not for a time (state transition: a case that waits on a tool or a
  sleep waits on nothing the run did not make before it started).
- A product a case needs from a real tool (the synthesis of a changed specification, a built
  fixture crate) is made once per run in one of the 2 forms the folder's rule "Tests stay fast"
  accepts, or the case moves to the lane `slow`; what each case proves is unchanged, and a
  mutant of what it proves is still red.
- `task check` exits 0, with no case removed or ignored.

## Context

A finding of the implementor of story:the-test-step-holds-its-bound-with-928-cases, there
before that story. It waits until the planned waves are delivered.

## Scope

Written after the build, from the diff against `main` at ab34af6: every path below is read
from it (cited). A scope report before the build named the same places; its times for the
first case (5.8 s alone, 8 to 17 s in a run) are times of the workstation and do not hold
on the build node.

Code, tests only (no file under a `src/` directory and no file under `tools/` changes):

- `crates/inbox-xtask/tests/synthesis.rs`: the change of the specification as one function,
  the cache entry of the changed specification, its setup case.
- `crates/inbox-xtask/tests/drift.rs`: the first case reads that entry.
- `crates/inbox-xtask/tests/tools/test_step.rs`: the fixture of the stubborn run built once
  into a cache entry under `target/tmp/`, its key, its setup case; the second case runs it
  from a copy.
- `crates/inbox-cli/tests/adversary.rs`: the third case, which waits on the clock of its own
  envelope.
- `crates/inbox-xtask/tests/adversary_bounded_run.rs` and
  `crates/inbox-cli/tests/adversary_outbox_expiry.rs`, with one `mod` line each in the 2
  files `tests/main.rs`: the cases of the adversary pass.

The gate and documents:

- `Taskfile.yml`: the setup command of `task test` runs the 3 setup cases before the bounded
  run.
- `AGENTS.md` (the rows of `task test` and of the crates), `docs/checks.md` (the count of
  the cases of `inbox-xtask`), `CHANGELOG.md` (one bullet under Changed).

Not touched: `ess/`, `generated/`, `.config/nextest.toml`, `tools/test-step.sh`, every
product crate.

## Decisions

- The story was a draft of the backlog (a finding of an implementor that was there before
  its wave). The operator's session accepted it with 2 others as one wave, one pull request
  each, no release from them alone (2026-11-24). No panel of plan critics read it; one
  adversary pass reviewed the build, and no pass read round 2.
- The times of the Outcome are times of the workstation (2026-11-23). On the build node the
  first case took 0.15 to 0.44 s before any change, and a load average of 100 on 20 CPUs
  cannot be made there (64 CPUs, a load of 26 to 84 in the runs). The acceptance is held
  with the load each run had.
- Where the products are made. The options were: (A) a setup step of `task test` that runs
  3 setup cases of `inbox-xtask` in one cargo-nextest call before the bounded run, each
  product a cache entry under `target/tmp/` keyed by the SHA-256 of its sources; (B) the
  lane `slow`. A is taken: what each case proves stays in the bounded run, and the setup
  cases take 0.31 s with both new entries absent and 0.02 s with them present.
- The second case still runs cargo-nextest, on a fixture package built once per run, and
  keeps its fixed waits (the bound of 2 s, then 1.5 s): it proves that a run over its bound
  leaves no test process, which needs a run that goes over its bound. It takes 3.5 s. The
  first sentence of the Outcome is not met for this case; the acceptance (under 5 s, the
  product of the real tool made once per run) is.
- The third case. The envelope over the body limit is refused and never kept, so its 4 s
  sleep and the repost after it proved nothing (a mutant of the expiry stayed green in it).
  The options were: (A) a second, small envelope posted to an address with no service, with
  `--expires` one second ahead; the case waits until the clock has passed the `expires` it
  reads back from the kept entry, and a repost that reaches no service then leaves the
  outbox empty; (B) keep the sleep, shorter. A is taken. With a real service a repost of an
  expired envelope can be answered `AlreadyExpired`, which also drops the entry, so the case
  uses no service. The case is renamed to
  `adversary::an_envelope_no_service_took_leaves_the_outbox_when_it_expires`.
- The synthesis of the changed specification has no path back to the real `ess` in the
  case: `synthesize_changed` answers from the entry and returns an error for any other
  `ess/`.
- The key of the fixture entry: the package files with the copied `.config/nextest.toml`,
  the build commands, `rust-toolchain.toml`, the version of cargo-nextest and, since round
  2, the bytes of the repository's `.cargo/config.toml` and the values of `RUSTFLAGS` and
  `CARGO_ENCODED_RUSTFLAGS` (a marker for an absent one). The wrapper is not a member: it
  does not change what is built. The host triple is not a member.
- A damaged entry is not built anew. The options were: (A) the failure line names the entry
  by its path relative to the tree and says that removing the directory makes the next run
  build it anew; (B) a check of the entry and a rebuild. A is taken: a rebuild inside the
  bounded run would be the build the story moves out of it.
- Old entries are not removed (finding 5 of the pass, recorded as no change): they lie under
  `target/tmp/` of the tree and go with its build output. One synthesis entry is 1.3 MB, one
  fixture entry 3.4 MB.
- A mistyped setup case in `Taskfile.yml` is held by a case of the adversary pass, which
  reads the setup line and checks each name against the list of the binary; `Taskfile.yml`
  checks no count itself.

## Measured

- The 3 cases in `task test` on the build node (64 CPUs), before the change, 2 runs at a load
  average of 65.9 and 76.5 at the start: 0.239 and 0.152 s, 3.909 and 5.639 s, 4.103 and
  4.019 s. After round 1, 2 runs of `task check` at a load of 83.8 to 69.7 and 69.7 to 64.4:
  0.056 and 0.060 s, 3.553 and 3.525 s, 1.027 and 1.039 s. After round 2, in `task check`:
  0.045 s, 3.524 s, 1.026 s. No case over 5 s in any run.
- The wall time of `task test`: 30 s and 22 s before (the bounded run 22 s and 21 s); 20 s
  with the setup cold, then 20 s and 19 s (the bounded run 18 to 19 s).
- Red runs of round 1, each mutant reverted. The expiry removed (`expires > now || true` in
  `unexpired`): the third case passed before the change (4.046 s) and is red after it
  (`tests/adversary.rs:308`, the entry still in the outbox file after its `expires`). The
  change not applied to the entry's input: the first case red (`tests/drift.rs:327`). The
  drift check always "no drift": the first case red (`tests/drift.rs:326`). `timeout
  --signal=KILL` in `tools/test-step.sh`: the second case red (`tests/tools/test_step.rs:417`,
  2 processes still running).
- Round 1: the cases of `inbox-xtask` 152 → 154, of `inbox-cli` 434 → 434, 2 runs in a row
  green of each crate.
- Adversary pass 1 (review-result:adversary-bounded-run-pass-1): 7 findings (2 warnings, 5
  notes), 2 cases, both green: `adversary_bounded_run::every_setup_case_of_task_test_is_a_case_of_the_binary`
  (0.006 to 0.025 s) and
  `adversary_outbox_expiry::a_post_that_starts_after_its_expires_is_kept_and_dropped_by_the_next_command`
  (0.014 to 0.047 s). It checked: one changed sentence of a domain file and one added line of
  a scenario file each make both synthesis entries anew; over 3 runs no file of the fixture
  entry changed; all entries removed and the setup cases, the first and the second case
  started together, 5 times, all green with 1 entry per key; a failed setup case stops
  `task test` before the bounded run; 3 runs of `task test` at a load of 30.9, 59.6 and 77.8,
  the 3 cases 0.064 to 0.570 s, 3.528 to 3.675 s and 1.027 to 1.105 s. 10 mutants: 4 red in
  the story's cases, 1 with no difference in behaviour, 3 red only in its own cases, and one
  (the synthesis call without its last argument) red in the crate run and with cold entries
  only. It did not check: an entry read from a moved tree beyond the 2 remap mutants, the
  host triple as a key member, a build with a changed `RUSTFLAGS`, the load of the
  workstation.
- Round 2 on those findings (6 recorded as fixed, 1 as no change): the mutant of the pass
  (the outbox read of `repost_each` 60 s behind the clock) red in the third case
  (`tests/adversary.rs:319`, the repost sent the expired envelope, status 3) and in the
  adversary's case; a setup name mistyped in `Taskfile.yml` red in the adversary's case; the
  3 unit cases of the key members red 3 of 3 before the change; the 2 cases of a damaged
  entry red before the change (the bare lines `no test binary of the fixture was built` and
  `crate/src/lib.rs is missing`). With the test binary removed from the real fixture entry,
  the setup case and the second case fail with the line that names
  `target/tmp/test-step-stubborn/<key>` and the removal; after the entry is removed the next
  run builds it (0.404 s) and is green.
- Round 2: the cases of `inbox-xtask` 155 → 160 (3 of the key, 2 of a damaged entry), of
  `inbox-cli` 435 → 435; 3 runs in a row `595 tests run: 595 passed, 0 skipped`. The third
  case 10 of 10 green, 1.024 to 1.037 s; each new case 0.004 to 0.012 s.
- Not held by a case: that the key reads the real environment and the real
  `.cargo/config.toml` (the 3 unit cases call the key function with made-up inputs); the
  locks of the cache entries have no bound of their own (the runner ends a case at 60 s; a
  cold build took 0.3 to 0.42 s). No adversary pass read round 2.
- The gate runs of the rounds on the build node (`task check`): round 1 1129 passed, 8
  skipped, the test step 19 s, exit 0, twice; round 2 1136 passed, 8 skipped, 19 s, exit 0.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T15:27:36Z to 2026-11-24T15:28:34Z, on the build
  node of the pool: exit 0, `1159 tests run: 1159 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
