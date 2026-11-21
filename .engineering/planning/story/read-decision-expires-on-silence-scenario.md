---
format: aep.planning-md/3
id: story:read-decision-expires-on-silence-scenario
kind: story
status: implemented
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 9}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 10}
- {from: "active", to: "implemented", at: "2026-11-21T23:49:43Z", actor: "agent:inbox", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}}
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

## As built

Wave 8 of stage 2, 2026-11-21: commits `469d7ee` and `e6554ff` (the adversary's cases), merged
as `df50876`.

- The authored scenario `read-decision-is-decided-on-silence`: a decision its recipient read
  is decided on silence at `expires`. A core whose `Expire` leaves `Read` out of the states it
  decides on silence fails step 18 of it.
- The scenario has a step as the Sweeper, so it is listed as unsupported over HTTP
  (`crates/inbox-conformance/unsupported.tsv`).
- 8 cases in `crates/inbox-core/tests/adversary_w8_read.rs` run `Expire` over a stored row: a
  read decision is decided at `expires` and not a second before, with the option it names for
  silence; a read decision without a silence option, a read request, a delivered request and a
  sent note are removed at expiry.
- No source line of a crate and no rule changed.
- Adversary: 1 pass on `469d7ee`, 4 findings (1 warning, 3 notes), each there before the unit;
  4 changed cores passed `task check` with the unit and each fails at least 1 of the 8 cases
  (review-result:adversary-read-decision-expires-on-silence-pass-1). The same in scenarios is
  story:expiry-scenarios-hold-the-remaining-cases (a draft); the pinned total that cannot fail
  is story:pinned-scenario-total-can-fail (a draft).

Numbers after the wave: `docs/plan/stage-2.md`, "Wave 8".
