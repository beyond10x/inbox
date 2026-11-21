---
format: aep.planning-md/3
id: story:expire-publishes-once
kind: story
status: implemented
title: Expire is held to one occurrence of the event it publishes
summary: A core that publishes the event of Expire twice passes task conform and the core's own tests; no scenario bounds the number of occurrences.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-2
scope:
- confidence: inferred
  path: crates/inbox-core/tests/expire_once.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:09Z", actor: "agent:inbox", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T23:49:43Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}}
---
## Outcome

A check holds `Expire` to one occurrence of the event it publishes (`DecisionTakenOnSilence` or
`MessageExpired`). A core that publishes that event twice passes `task conform` and
`cargo test -p inbox-core` today. The same doubling on `PostMessage` or `OpenInbox` is caught,
because a later step of a scenario reads a field of the event and the runner refuses an event
published 2 times; no step of the suite reads a field of an event `Expire` published, and no
step bounds the number of occurrences.

## Acceptance

`task check` fails against a core whose entry publishes the last event of `Expire` twice, where
before this story `task conform` passes with 92 of 92 and `cargo test -p inbox-core` exits 0.

## Context

Found by the adversary of story:conformance-gate in stage 2, wave 3
(review-result:adversary-conformance-gate-pass-2, finding 6, mutant
`events-doubled-expire-only` in `crates/inbox-core/src/lib.rs`, `handle_with_events`). The core
and the scenarios are at the wave base, so the finding was there before the unit. The core as it
stands publishes the event once; the gap is the check.

Whether an authored scenario can hold the count is open: the story first reads what a step of
the pinned `ess-conformance` version can claim about the number of occurrences. If none can, the
check is a case of `crates/inbox-core`, and the limit of the suite is stated in
`docs/design/inbox.md`.

## Decisions

Accepted by the operator on 2026-11-21 (approval-record:wave-8-drafts-accepted) and run in
wave 8 of stage 2.

Taken by the coordinator on 2026-11-21:

- The check is a case of `crates/inbox-core`, in a test file of its own
  (`crates/inbox-core/tests/expire_once.rs`). The unit reports whether a step of the suite can
  bound the number of occurrences of an event; if it cannot, `docs/design/inbox.md` says so.
- The unit runs in part 1 of the wave, at the same time as story:external-refusal-order-cases
  and story:repost-does-not-redeliver-a-handled-note.

## Scope

- `tools/scenarios.py`, `ess/scenarios/`, `ess/ess-inputs.yaml`, or
- `crates/inbox-core/tests/`

## As built

Wave 8 of stage 2, 2026-11-21: commits `7b00bbb` and `c13275e` (the adversary's cases), merged
as `1cf9230`.

- 9 cases in `crates/inbox-core/tests/expire_once.rs` compare the whole list of events `Expire`
  hands back, so each event occurs once.
- 6 cases in `crates/inbox-core/tests/adversary_w8_expire.rs` hold the same off the points the
  first 9 build: a thread that is not the message id, a sender that is not the principal spoken
  for, a sweep after `expires`, 1 of 4 messages expired with 5 `Expire` calls on one backend,
  and a refused decision that names a silence option.
- No source line of a crate, no rule and no scenario changed.
- Adversary: 1 pass on `7b00bbb`, 4 findings (1 blocker and 1 note introduced, 2 warnings there
  before the unit); 5 changed cores that publish an event twice passed the unit's 9 cases and
  each fails at least 1 of the 6 (review-result:adversary-expire-publishes-once-pass-1). The 2
  findings from before the unit are held by cases since `c13275e` and are open in the
  scenarios: story:expiry-scenarios-hold-the-remaining-cases (a draft).

Numbers after the wave: `docs/plan/stage-2.md`, "Wave 8".
