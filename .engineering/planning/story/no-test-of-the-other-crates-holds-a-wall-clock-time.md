---
format: aep.planning-md/3
id: story:no-test-of-the-other-crates-holds-a-wall-clock-time
kind: story
status: draft
title: No test of the listener, the backend test kit or the tools fails because the host was slow
summary: Cases of inbox-server, inbox-backend-testkit and inbox-xtask hold a wall-clock time a slow host breaks; 3 were shown red with a 3 s delay.
revision: 2
---
## Problem

Cases of `inbox-server`, `inbox-backend-testkit` and `inbox-xtask` hold a wall-clock time that
a slow host breaks. story:no-test-holds-a-wall-clock-time changed the cases of
`crates/inbox-cli/` and `tests/acceptance/`; these are the ones it listed and left.

Shown red with a delay of 3 s planted in the test's own thread (adversary pass 1 on that
story, review-result:adversary-no-test-holds-a-wall-clock-time-pass-1, findings 8 and 9, and
its check of the implementor's list):

- `crates/inbox-server/tests/cases.rs:521`, `a_request_not_arrived_within_the_request_timeout_is_408`:
  a request timeout of 300 ms held to under 900 ms; line 532 holds a stop to under 2 s.
- `crates/inbox-server/tests/adversary_deadline_wait.rs:256`,
  `one_wait_of_a_second_on_a_note_due_at_the_reading_reads_its_view_once`: a wait of 1 s held
  to under 3 s.
- `crates/inbox-server/tests/read_rule.rs:190`, `a_wait_on_another_inbox_does_not_wake_for_its_rows`:
  `max_wait` of 2 s of real time, and the owner's wait must answer within 1.5 s.

Listed by the implementor of that story, not shown red:

- `crates/inbox-server/tests/adversary.rs:140-185` and `adversary2.rs:85-275`: the cases of the
  request timeout (an answer within 4 s, pieces 400 ms apart inside 3 s, a stop within 3 s).
  The request timeout and the gap between pieces run on the monotonic clock of `signals.rs`,
  not on `Clock`, so `ManualClock` does not reach them.
- `crates/inbox-backend-testkit/src/lib.rs:134` (`check_exclusive`, `BOUND` and `HOLD` of 2 s):
  shipped code of the kit; a correct backend can fail the check on a slow host.
- `crates/inbox-backend-testkit/tests/adversary.rs:238` (a holder thread gets 50 ms to enter
  and holds 800 ms) and `adversary2.rs:374`, `434` (2 s, 500 ms).
- `crates/inbox-xtask/tests/tools/test_step.rs:101`: a step bound of 2 s around a shell that
  must start and copy one file.

## Acceptance

Each case above passes with a delay of 3 s planted in its own thread at the step a slow host
delays, or is shown to hold no time (state transition: red with the plant before, green
after). `task test` stays within its bound of 120 s.

## Open before it is accepted

- The request timeout of the listener needs a monotonic clock a test can move. That changes
  `crates/inbox-server/src/signals.rs`, shipped code.
- `check_exclusive` is a public check of the kit: its bound is part of what a backend author
  meets.

## Scope

- `crates/inbox-server/tests/`, `crates/inbox-server/src/signals.rs`
- `crates/inbox-backend-testkit/src/lib.rs`, `crates/inbox-backend-testkit/tests/`
- `crates/inbox-xtask/tests/tools/test_step.rs`
