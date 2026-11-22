---
format: aep.planning-md/3
id: approval-record:ess-0-97-4-ordered
kind: approval-record
status: draft
title: The move to ESS 0.97.4 is the operator's order
relations:
- decides: story:pin-ess-0-97-4
revision: 1
---
## Decision

story:pin-ess-0-97-4 is accepted and built on a branch with a pull request.

## Who decided

The operator. The host operator's session relayed on 2026-11-22 at about 19:45 CET: ESS
0.97.4 is published and is the shared binary; every repository moves its pins from 0.97.3 to
0.97.4, as for 0.97.3 (the operator, about 14:16 CET the same day: all should upgrade).

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate and
  the 3 required checks. Sub-agents on Opus or lower.
- The reorder of 2 outcomes of `PostMessage` in the same change: the coordinator took it from
  the notice of the ESS session; the unit shows what it changes in `generated/` and the suite.

## What it does not grant

- No release, no tag, no deployment.
