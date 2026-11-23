---
format: aep.planning-md/3
id: story:no-case-of-the-bounded-run-waits-on-a-real-tool
kind: story
status: draft
title: No case of the bounded run waits on a real tool or a fixed sleep
summary: '3 cases of the bounded test run take 4 to 19 s under load: one runs the synthesis of ess, one builds a fixture crate with cargo, one sleeps 4 s.'
revision: 1
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
