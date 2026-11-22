---
format: aep.planning-md/3
id: review-result:adversary-pass-1-on-a-due-message-takes-no-answer
kind: review-result
status: active
title: Adversary pass 1 on story:a-due-message-takes-no-answer
relations:
- reviews: story:a-due-message-takes-no-answer
revision: 1
---
unit: story:a-due-message-takes-no-answer, the uncommitted working tree of `fix/answer-deadline` (base 42c7eb1)
verdict: CONFIRMED
cases: executed 367→379 (the packages `inbox-core`, `inbox-server`, `inbox-cli`, `inbox-conformance`), red 2
origin: introduced 3 / pre-existing 2 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with each
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added 3 case files named
`adversary_deadline.rs`, under `crates/inbox-core/tests/`, `crates/inbox-server/tests/` and
`crates/inbox-cli/tests/`, and one `mod` line for each in the package's `tests/main.rs`. Every
mutation ran in a copy outside the tree.

**2. Cases added (12).**

- 4 cases `every_late_order_leaves_a_…` (core): green, 0.8 to 1.3 s each. The clock reads the
  `expires` of the message. For a decision with a default, a decision without one, a request and
  a note, in every state the kind can be in, all 720 orders of `Answer`, `Refuse`, `Expire`,
  `MarkRead`, `MarkDelivered` and `Collect`: a late answer or refusal is never taken and
  publishes nothing, and outcomes, events and store equal the same order without them.
- `answer_refusal_post_and_sweep_meet_the_deadline_at_the_same_reading` (core): green, 0.07 s. 6
  forms of `expires` (a fraction, an offset, lower-case letters) against 6 readings.
- `an_answered_message_past_its_retention_is_settled_not_due` (core): green, 0.02 s.
- `a_reply_already_expired_at_the_service_clock_is_not_taken_and_then_swept` (core): red, 0.01 s
  (finding F1).
- `the_reading_is_taken_after_the_wait_for_the_section` (listener): green, 2.2 s.
- `the_request_timestamp_does_not_move_the_reading` (listener): green, 1.0 s.
- `a_third_principal_gets_the_same_answer_for_a_due_message_as_for_none` (listener): green,
  1.0 s.
- `an_answer_after_a_fractional_deadline_by_the_service_clock_is_refused` (listener): red, 0.3 s
  (finding F2).
- `the_three_clock_outcomes_are_refusals_at_the_client` (client): green, 0.28 s.

**3. Suite run.** `cargo nextest run -p inbox-core -p inbox-server -p inbox-cli -p
inbox-conformance --no-fail-fast`: `379 tests run: 377 passed, 2 failed, 1 skipped`, exit status
100. Without the adversary's 12 cases: `367 tests run: 367 passed, 13 skipped`, exit status 0.

Mutants, applied in a copy, against the unit's 8 deadline cases.

- `<=` becomes `<`: unit cases fail.
- The comparison uses the `expires` of the reply: unit cases fail.
- The check applies in every state: no unit case fails (survives); the adversary's
  `an_answered_message_past_its_retention_is_settled_not_due` fails.
- No check for `Refuse`: unit cases fail.
- The clock is read outside the section: no unit case fails (survives); the adversary's
  `the_reading_is_taken_after_the_wait_for_the_section` fails.
- The clock is the envelope's `created`: unit cases fail.

**4. Findings.**

F1. `ess/domains/messaging.yaml` (`Answer`) and `crates/inbox-core/src/context.rs`. A reply whose
own `expires` is at or before the service's clock, with a `created` before it, is taken while
the message is not yet due. It is stored with that `expires`, and the next sweep at the same
reading removes it: the sender's collect gets `unknown-message`, and for a decision with a
default the sender gets neither the answer nor the default. A post in the same condition is now
refused. It existed before this unit.

F2. `crates/inbox-server/src/clock.rs`. The reading is cut to the second before the comparison,
so a message whose `expires` ends in `.500Z` is answered when the clock reads `.700Z` of the
same second, against "a message is due at its `expires` exactly" in `docs/design/service.md`.
`Answer` and the sweep agree at every reading. Introduced.

F3. The unit's 8 cases pass with the clock read outside the section and with the check applied
in every state. Two cases of the adversary hold both. Introduced.

F4. `--clock-start` of `inbox serve`. A flag made for the conformance run is on the service the
operator runs and in `website/docs/status.md`; nothing logs it. A start in the past brings back
lapsed keys and passed deadlines, and a clock that advances with the monotonic clock falls
behind over a suspend. Only the operator's own command line sets it. Introduced.

F5. `MarkRead` and `MarkDelivered` act on a due message. A due note read before the sweep is
removed with `MessageHandled`, after the sweep with `MessageExpired`: the event the sender gets
depends on when the sweep ran; the stored outcome is the same. It existed before this unit.

**5. Attacked and not broken.** All 720 orders in 11 combinations of kind and state at the
deadline; `expires` with a fraction, an offset and lower-case letters; the reading is taken
after the wait for the section; the request's timestamp does not move the reading; a third
principal gets the bytes of `unknown-message` for a due message; an answered message past its
retention answers `already-settled` and is still collected; the client exits 1 for the 3 clock
outcomes and drops the outbox line. `expired-on-arrival` before `signature-invalid` tells a
caller nothing it does not hold: the caller has passed request authentication and `not-sender`.

**What the coordinator did.**

- F1: back to the implementor as part of this unit. The story's acceptance (the same outcome
  whenever the sweep runs) needs it.
- F2: back to the implementor: full precision in the 3 commands and the sweep.
- F3: the 2 cases of the adversary stay in the unit.
- F4: back to the implementor: the flag is hidden from the help and the user documentation,
  says so on standard error at start, and advances with the system clock.
- F5: no change in this unit. Filed as the draft story:a-due-message-takes-no-mark.

```findings
- file: crates/inbox-core/src/context.rs
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a reply whose own expires is at or before the service clock is taken and removed by the next sweep at the same reading, so the sender collects nothing, while a post in the same condition is refused
- file: crates/inbox-server/src/clock.rs
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the reading is cut to the second before comparing, so an answer 0.2 s after a fractional expires is taken, against due at its expires exactly
- file: crates/inbox-server/src/service.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's 8 deadline cases pass with the clock read outside the section and with the check applied in every state; only the adversary's cases fail on those 2 mutants
- file: crates/inbox-cli/src/main.rs
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the conformance run's --clock-start is on the operator's serve without a log line, a past start brings back lapsed keys and deadlines, and its monotonic clock falls behind over a suspend
- file: ess/domains/messaging.yaml
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: MarkRead and MarkDelivered act on a due message, so the sender of a due note gets MessageHandled or MessageExpired depending on whether the sweep ran first
```
