---
format: aep.planning-md/3
id: story:the-step-case-passes-on-a-loaded-host
kind: story
status: implemented
title: The test-step case passes on a loaded host
summary: A case of the test step holds a bound of 2 s that includes the start of cargo, and fails the gate on a loaded host.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T15:00:04Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T15:00:04Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T15:28:58Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The case `tools::test_step::a_run_over_the_bound_leaves_no_test_process_running`
(`crates/inbox-xtask/tests/tools/test_step.rs`) passes on a loaded host as on an idle one.

The case runs the test step over a fixture crate with a bound of 2 s and expects both of the
fixture's tests to have started inside that bound. The 2 s include the start of `cargo` and
of cargo-nextest. On 2026-11-22, at a load average of 105 to 150 on 20 CPUs, the case failed
in 3 runs of the gate with "both tests ran: [None, None]"; one log shows `cargo` waiting
for the lock of the package cache inside the bound. Each failure made a whole run of the
gate red.

## Acceptance

- The case passes 10 times of 10, run alone, at a load average above 100 (state transition: a
  case that is red on a loaded host and green on an idle one is green on both).
- It still fails for the defect it was written for: with the step's kill of the test processes
  taken out, the case is red.
- It holds no bound that includes the start of `cargo`: what it measures starts when both
  tests of the fixture run.
- The case takes no longer than it took (11 s in the gate on 2026-11-22).

## Context

A defect of the case delivered with story:test-step-enforces-its-budget. The folder rule
"Tests stay fast" makes a defect of the test budget a story that is fixed before the next
feature (approval-record:d20-the-step-case-passes-on-a-loaded-host).

## Scope

- `crates/inbox-xtask/tests/tools/test_step.rs`, case
  `a_run_over_the_bound_leaves_no_test_process_running`: the fixture is built and listed before
  the step (`cargo nextest list --list-type binaries-only`, `cargo metadata`), and the step
  runs `cargo-nextest nextest run --binaries-metadata … --cargo-metadata …`. The bound of 2 s
  and every assertion are unchanged.
- `AGENTS.md`: the row of `task test`.

## Measured

- Time from the start of the step to both tests of the fixture running, 6 runs each at a load
  average of 100 to 114 on 20 CPUs: 220 to 3652 ms with `cargo nextest run --offline` (the old
  form), 57 to 115 ms with the new form.
- The unchanged case, run alone: red in 1 of 6 runs and in 2 of 12 runs of a comparison, each
  with "both tests ran: [None, None]". The changed case: 10 of 10 green at a load average of
  101 to 139 (6.0 to 19.2 s), 10 of 10 at 75 to 83 (5.8 to 10.1 s), 12 of 12 in the comparison.
- With `timeout --kill-after=10` of `tools/test-step.sh` replaced by `timeout --signal=KILL`,
  and with `grace-period = "0s"` removed from `.config/nextest.toml`, the case is red: "still
  running after the step ended".
- In the gate the case took 7.9 s (11 s before). At equal load the changed case is 0.7 to 1.4 s
  slower than the unchanged one (means of 6 pairs, twice: 8.6 against 7.2 s, 7.5 against 6.8 s).
- `task check`: exit 0 after 329 s, `524 tests run: 524 passed, 5 skipped`, the test step 64 s of
  its bound of 120 s.
