---
format: aep.planning-md/3
id: story:expire-publishes-once
kind: story
status: draft
title: Expire is held to one occurrence of the event it publishes
summary: A core that publishes the event of Expire twice passes task conform and the core's own tests; no scenario bounds the number of occurrences.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-2
revision: 1
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

## Scope

- `tools/scenarios.py`, `ess/scenarios/`, `ess/ess-inputs.yaml`, or
- `crates/inbox-core/tests/`
