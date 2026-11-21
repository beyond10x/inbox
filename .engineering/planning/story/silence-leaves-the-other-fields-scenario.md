---
format: aep.planning-md/3
id: story:silence-leaves-the-other-fields-scenario
kind: story
status: draft
title: An authored scenario holds the fields Expire leaves unchanged when it decides on silence
summary: A core that clears decision or default_on_silence, empties body or rewrites created or thread after deciding on silence passes the suite of 92 scenarios.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-2
revision: 1
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

## Scope

- `tools/scenarios.py`, `ess/scenarios/`, `ess/ess-inputs.yaml`
