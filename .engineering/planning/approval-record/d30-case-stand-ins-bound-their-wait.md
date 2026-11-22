---
format: aep.planning-md/3
id: approval-record:d30-case-stand-ins-bound-their-wait
kind: approval-record
status: draft
title: 'D30: the story on the stand-ins of the case files is accepted, the operator''s pick B'
relations:
- decides: story:case-stand-ins-bound-their-wait
revision: 1
---
## Decision

story:case-stand-ins-bound-their-wait is accepted and built on a branch with its own pull request.

## Who decided

The operator, 2026-11-22 19:24:58 CET, relayed by the session `host-operator`: to the
question D30 ("what the inbox works on next") the pick was all three options, among them
"B: 4 gate-stability fixes": "propose, accept and implement the 4 gate-stability drafts, one
PR each".

## What it grants

- The moves of the story from `draft` to `proposed` and `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate
  and the 3 required checks. Sub-agents on Opus or lower.

## What it does not grant

- No change of the bounds of the test step (60 s per test, 120 s for the step).
- No release, no tag, no deployment, no listener outside loopback.
