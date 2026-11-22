---
format: aep.planning-md/3
id: story:a-due-message-takes-no-answer
kind: story
status: implemented
title: A message past its expires takes no answer and no refusal
summary: The deadline of a message is applied only by the expiry task, so a due decision can still be answered or refused until the task runs.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:00:46Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T13:00:46Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T16:44:06Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A message whose `expires` has passed takes no answer and no refusal, whether or not the expiry
task has reached it. The service compares the stored `expires` with its own clock inside the
transition that answers or refuses, so the outcome of a decision at and after its deadline is
the one the expiry task gives it: the option of `default_on_silence`, or removal.

A post whose `expires` is not after the service's clock is refused on arrival.

Before this story the answer path checked the reply's own two timestamps and the state of the
stored message. The deadline was applied only by the expiry task, which runs every
`--sweep-interval` seconds (10 by default). Between the deadline and the next run the recipient
could still answer or refuse a due decision and so replace or suppress the silence default. A
post compared `expires` with the sender's `created` only, so a decision already due at the
service's clock was stored.

## Acceptance

- `Answer` on a message in `Sent`, `Delivered` or `Read` whose stored `expires` is at or before
  the service's clock reading is refused, changes no row and publishes no event. The same holds
  for `Refuse`.
- After such a refusal the expiry task decides the message as it would have without the late
  call: a decision with `default_on_silence` is answered by silence with that option; any other
  message is removed.
- The clock reading is the service's (`Clock` port), read inside the serialized transition. No
  value the caller sends takes its place.
- `PostMessage` with `expires` at or before the service's clock reading is refused with
  `AlreadyExpired` and stores nothing.
- A caller that is no party of the message still gets `unknown-message`: the deadline refusal
  tells a third principal nothing.
- The rule is declared in `ess/`; the core and the HTTP service give the same outcome.
- One case runs the two orders (late answer then sweep, sweep then late answer) and finds the
  same stored outcome.

## Context

Reported by a source review of the release 0.2.0 (finding 2 of 11). The operator decided on
2026-11-22 that the fix is built first and published together with the finding
(approval-record:d13-a-due-message-takes-no-answer).

## Scope

- `ess/domains/messaging.yaml`: `Answer` and `Refuse` gain the outcome `deadline-passed`
  (error `Expired`), `PostMessage` and `Answer` the outcome `expired-on-arrival` (error
  `AlreadyExpired`); each holds for a message in `Sent`, `Delivered` or `Read` (`when_subject`)
  and asks the Clock port (`external`). 4 authored scenarios in `ess/scenarios/`.
  `generated/`, the projections and `website/src/generated/animation.json` follow.
- `crates/inbox-core/src/context.rs`, `src/obligations.rs`, `src/lib.rs`: the context answers
  the 2 questions from the stored `expires`, the reply and the clock reading of the command,
  at full precision.
- `crates/inbox-server/src/clock.rs`, `src/expiry.rs`, `src/service.rs`, `src/lib.rs`: one
  reading per command inside its section; the sweep reads the clock at the same precision;
  `StartedClock` for the conformance run.
- `crates/inbox-cli/src/serve.rs`, `src/main.rs`: the hidden flag `--clock-start` of
  `inbox serve`, with a line on standard error when it is set.
- `crates/inbox-conformance/src/core_target.rs`, `src/wire_target.rs`, `unsupported.tsv`: the
  clock of each target; 4 scenarios more are unsupported over HTTP.
- Cases: `deadline.rs`, `adversary_deadline.rs` and `adversary_deadline_2.rs` in the `tests/`
  of `inbox-core` and `inbox-server`; `adversary_deadline.rs` and `cases.rs` of `inbox-cli`;
  the earlier cases of `inbox-core` that built a context take a clock.
- `docs/design/inbox.md`, `docs/design/service.md`, `docs/checks.md`, `AGENTS.md`,
  `CHANGELOG.md`, `website/docs/status.md`.

## Measured

- 89 capabilities (87 generated, 2 obligations), 117 scenarios (42 authored), 96 refusals, all
  covered; 117 passed on the interpreted target and against the core; over HTTP 32 passed and
  85 unsupported.
- `task check` on the branch with main at release 0.3.0, 2026-11-22T16:43:11Z: exit 0,
  `562 tests run: 562 passed, 5 skipped`, the test step 37 s of its bound of 120 s.
- 2 adversary passes: review-result:adversary-pass-1-on-a-due-message-takes-no-answer and
  review-result:adversary-pass-2-on-a-due-message-takes-no-answer.
