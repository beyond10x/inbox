---
format: aep.planning-md/3
id: approval-record:d21-drift-cases-synthesize-once
kind: approval-record
status: draft
title: 'D21: the drift cases synthesize once, under the rule of the test budget'
relations:
- decides: story:drift-cases-synthesize-once
revision: 1
---
## Decision

story:drift-cases-synthesize-once is accepted and built on a branch with a pull request.

## Who decided

The coordinator (D21), under the folder rule "Tests stay fast": a step over its budget is a
defect that gets a story and is fixed before the next feature. On 2026-11-22, at a load
average of 105 on 20 CPUs, the test step of the gate ended at its bound of 120 s with 531 of
545 cases started; 7 of its 10 slowest cases were cases of `crates/inbox-xtask`, 5 of them drift
cases (10.7 to 13.3 s each). Reported to the operator in the wave report. The
alternative not taken: run a red gate again until the step ends inside its bound.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate and
  the 3 required checks. Sub-agents on Opus or lower.

## What it does not grant

- No change of the bounds of the test step (60 s per test, 120 s for the step).
- No release, no tag, no deployment.
