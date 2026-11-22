---
format: aep.planning-md/3
id: story:no-test-holds-a-wall-clock-time
kind: story
status: draft
title: No test of the gate fails because the host was slow
revision: 1
---
## Outcome

No test of the gate fails because the host was slow.

On 2026-11-22 the gate failed 2 times in a row on a host with a load of about 80 to 100, each
time in a test that holds a wall-clock time:

- `a_read_note_and_an_expired_unanswered_request_are_in_no_view`
  (`tests/acceptance/tests/checks.rs`): a request lived 2 s, and the fourth of 4 client
  processes read its view after that.
- `an_epochs_file_of_20000_lines_is_read_within_2_seconds`
  (`crates/inbox-cli/tests/adversary_w8_repost.rs`): one run took 2.2 s against 2 s.

Both were changed in the same change that found them: the first makes its claim only for views
read before `expires` and repeats with a longer life, the second holds the fastest of at most
5 runs to the bound. Each still reads the wall clock. 2 more bounds of 10 s are in
`crates/inbox-cli/tests/adversary.rs`.

## Acceptance

`cargo test --workspace` and `task acceptance` pass 5 times of 5 with the host's processors
held busy by another job (state transition: a gate that is red on a loaded host and green on
an idle one is green on both).

## Decisions

- A test of expiry gives the service its clock: `inbox serve` takes a clock file or a test
  flag only a test build has, so no test sleeps until an instant passes.
- A test of a read's cost counts work (lines parsed, comparisons) through a counter the code
  exposes to tests; it holds no time.
- Not part of this story: the runner's 60 s bound per test and the 120 s bound of the step
  (`story:test-step-enforces-its-budget`).

## Scope

- `tests/acceptance/`, `crates/inbox-cli/tests/`
- `crates/inbox-server/src/clock.rs`, `crates/inbox-cli/src/`
