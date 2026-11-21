---
format: aep.planning-md/3
id: story:read-decision-expires-on-silence-scenario
kind: story
status: active
title: A scenario holds that a read decision is decided on silence at expiry
summary: A core that deletes a read decision at expiry passes every gate; the one scenario for decided-on-silence never reads the message.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-1
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
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 10}
---
## Outcome

An authored scenario holds that a decision with `default_on_silence` which its recipient has
read, and which then expires, is decided on silence. The specification's guard for
`decided-on-silence` is `state: [Sent, Delivered, Read]`; the one scenario that reaches the
outcome, `decision-taken-on-silence`, marks the message delivered and never read. A core that
deletes a read decision at expiry passes the interpreted run, `task conform` and
`cargo test -p inbox-core`.

## Acceptance

`task conform` fails against a core whose `Expire` leaves `Read` out of the states it decides on
silence, where before this story it passes with 92 of 92.

## Context

Found by the adversary of story:conformance-gate in stage 2, wave 3
(review-result:adversary-conformance-gate-pass-1, finding F7, mutant
`expire-silence-not-after-read-2`). The scenarios are the specification's and the core is
story:core-and-memory-backend's, both at the wave base, so the finding was there before the
unit. The core as it stands decides a read decision on silence; the gap is the scenario.

Scenarios are written in `tools/scenarios.py` and generated with `task scenarios`.

## Decisions

Accepted by the operator on 2026-11-21 (approval-record:wave-8-drafts-accepted) and run in
wave 8 of stage 2.

It runs in part 2 of the wave, after wave 7 is on `main`: wave 7 moves the repository to
ESS 0.97.1, and this story changes `tools/scenarios.py` and the files it writes, which are
regenerated in that move. That is the `depends_on` relation to story:dedup-key-scope.

## Scope

- `tools/scenarios.py`, `ess/scenarios/`, `ess/ess-inputs.yaml`
