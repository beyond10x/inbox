---
format: aep.planning-md/3
id: story:silence-leaves-the-other-fields-scenario
kind: story
status: implemented
title: An authored scenario holds the fields Expire leaves unchanged when it decides on silence
summary: A core that clears decision or default_on_silence, empties body or rewrites created or thread after deciding on silence passes the suite of 92 scenarios.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-2
- depends_on: story:dedup-key-scope
scope:
- confidence: inferred
  path: crates/inbox-conformance/tests/adversary3.rs
- confidence: inferred
  path: crates/inbox-conformance/unsupported.tsv
- confidence: inferred
  path: ess/ess-inputs.yaml
- confidence: inferred
  path: ess/scenarios
- confidence: inferred
  path: generated
- confidence: inferred
  path: tools/scenarios.py
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 10}
- {from: "active", to: "implemented", at: "2026-11-21T23:49:43Z", actor: "agent:inbox", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}}
---
## Outcome

An authored scenario holds the fields of a decision that `Expire` leaves as they were when it
decides on silence. After the outcome `decided-on-silence` a read of the message row claims
`body`, `created`, `decision`, `default_on_silence` and a `thread` that differs from the
`message_id`. The one scenario that reaches the outcome, `decision-taken-on-silence`, claims
`answered_by`, `decided_option`, `expires`, `message_id` and `state` only (step 22), and its
message is the first of its thread, so `thread` equals `message_id`.

## Acceptance

`task conform` fails against a core whose `Expire`, after it has written `decided_option`, also
clears `decision`, where before this story it passes with 92 of 92.

## Context

Found by the adversary of story:conformance-gate in stage 2, wave 3
(review-result:adversary-conformance-gate-pass-2, finding 5). Five mutants of
`crates/inbox-core/src/obligations.rs`, each one line after `decided_option` is written, pass
`task conform` and `cargo test -p inbox-core`: `decision` cleared, `default_on_silence` cleared,
`body` emptied, `created` set to `keep_until`, `thread` set to the message id. The interpreted run
does not run the core. The scenarios are the specification's and the core is
story:core-and-memory-backend's, both at the wave base, so the finding was there before the unit.
The core as it stands leaves the 5 fields unchanged; the gap is the scenario.

Scenarios are written in `tools/scenarios.py` and generated with `task scenarios`.

## Decisions

Accepted by the operator on 2026-11-21 (approval-record:wave-8-drafts-accepted) and run in
wave 8 of stage 2.

It runs in part 2 of the wave, after wave 7 is on `main`: wave 7 moves the repository to
ESS 0.97.1, and this story changes `tools/scenarios.py` and the files it writes, which are
regenerated in that move. That is the `depends_on` relation to story:dedup-key-scope.

## Scope

- `tools/scenarios.py`, `ess/scenarios/`, `ess/ess-inputs.yaml`

## As built

Wave 8 of stage 2, 2026-11-21: commits `b0a4ac5` and `0ece50d` (the adversary's cases), merged
as `923f535` and `48f727b`.

- The authored scenario `silence-leaves-the-decision-as-it-was`: a decision decided on silence
  keeps the 10 members of the stored message the scenario claims.
- The scenario has a step as the Sweeper, so it is listed as unsupported over HTTP
  (`crates/inbox-conformance/unsupported.tsv`).
- 3 cases in `crates/inbox-core/tests/adversary_w8_fields.rs`: a decision with every optional
  member set is decided on silence from Sent, from Delivered and from Read, and the whole
  stored message equals the one posted apart from `expires`, `answered_by` and
  `decided_option`. The silence option differs from the recommended one.
- No source line of a crate and no rule changed.
- Adversary: 1 pass on `b0a4ac5`, 4 findings (2 warnings, 2 notes; 1 introduced, 3 there before
  the unit); 19 changed cores of `Expire` passed `task conform` with the unit, and 18 of them
  fail at least 1 of the 3 cases (review-result:adversary-silence-leaves-the-other-fields-pass-1).
  The same claim in scenarios, and the other rows of the thread, are
  story:silence-scenarios-claim-the-whole-message (a draft).

Numbers after the wave: `docs/plan/stage-2.md`, "Wave 8".
