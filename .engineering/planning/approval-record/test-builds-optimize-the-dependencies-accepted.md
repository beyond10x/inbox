---
format: aep.planning-md/3
id: approval-record:test-builds-optimize-the-dependencies-accepted
kind: approval-record
status: draft
title: The story on optimized dependencies in test builds is accepted under the rule Tests stay fast
relations:
- decides: story:test-builds-optimize-the-dependencies
revision: 1
---
## Decision

story:test-builds-optimize-the-dependencies is accepted and built on a branch with its own
pull request.

## Who decided

The coordinator of this repository (session `inbox`), under the rule "Tests stay fast" of the
folder this repository is developed in: "A step over the budget is a defect. It gets a story
and is fixed before the next feature." The operator set that rule on 2026-11-21. The decision
was reported to the session `host-operator` on 2026-11-22 20:30 CET.

## What it grants

- The moves of the story from `draft` to `proposed` and `active`, run by the coordinator.
- One branch, its commits and a pull request. The fast-forward of `main` follows the
  operator's answer on the order of this change and release 0.3.1, after a green gate and the
  3 required checks.

## What it does not grant

- No change of the bounds of the test step (60 s per test, 120 s for the step).
- No release, no tag, no deployment.
