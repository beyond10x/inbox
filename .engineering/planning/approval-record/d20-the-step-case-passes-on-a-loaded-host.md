---
format: aep.planning-md/3
id: approval-record:d20-the-step-case-passes-on-a-loaded-host
kind: approval-record
status: draft
title: 'D20: the test-step case is fixed under the rule of the test budget'
relations:
- decides: story:the-step-case-passes-on-a-loaded-host
revision: 1
---
## Decision

story:the-step-case-passes-on-a-loaded-host is accepted and built on a branch with a pull
request.

## Who decided

The coordinator (D20), under the folder rule "Tests stay fast": a defect of the test budget
gets a story and is fixed before the next feature. Reported to the operator in the wave
report. The alternative not taken: run a red gate again until the case passes.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate and
  the 3 required checks. Sub-agents on Opus or lower.

## What it does not grant

- No change of the bounds of the test step (60 s per test, 120 s for the step).
- No release, no tag, no deployment.
