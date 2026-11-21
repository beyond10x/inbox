---
format: aep.planning-md/3
id: story:external-refusal-order-cases
kind: story
status: active
title: A case of the core holds the order of the three external refusals of PostMessage
summary: A core that asks the external refusals in reverse order passes task conform and its own tests; no conformance run can see that order.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-1
scope:
- confidence: cited
  path: crates/inbox-core/tests/core/precedence.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 5}
---
## Outcome

A case of `crates/inbox-core` holds the order in which `PostMessage` asks its three external
refusals (`unknown-option`, `signature-invalid`, `authority-invalid`) when more than one holds.
No conformance run can see that order: a step forces one external outcome, on the interpreted
target and in the in-process target alike. A core that asks them in the reverse order passes
`task conform` and `cargo test -p inbox-core`.

## Acceptance

`cargo test -p inbox-core` fails against a core that asks `authority-invalid` before
`signature-invalid` for a post in which both hold, where before this story it passes.

## Context

Found by the adversary of story:conformance-gate in stage 2, wave 3
(review-result:adversary-conformance-gate-pass-1, finding F8). The core and its cases are
story:core-and-memory-backend's, at the wave base, so the finding was there before the unit.

story:post-refusal-order-scenarios (wave 2) holds the order between kinds of refusal with
authored scenarios; this story holds the order inside the external kind, which only a case of
the core can.

## Decisions

Accepted by the operator on 2026-11-21 (approval-record:wave-8-drafts-accepted) and run in
wave 8 of stage 2.

Taken by the coordinator on 2026-11-21:

- The cases go into `crates/inbox-core/tests/core/precedence.rs`, beside the cases that hold
  the order of the other refusals of `PostMessage`.
- The unit runs in part 1 of the wave.

## Scope

- `crates/inbox-core/tests/`
