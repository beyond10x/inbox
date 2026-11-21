---
format: aep.planning-md/3
id: story:read-decision-expires-on-silence-scenario
kind: story
status: draft
title: A scenario holds that a read decision is decided on silence at expiry
summary: A core that deletes a read decision at expiry passes every gate; the one scenario for decided-on-silence never reads the message.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-conformance-gate-pass-1
revision: 1
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

## Scope

- `tools/scenarios.py`, `ess/scenarios/`, `ess/ess-inputs.yaml`
