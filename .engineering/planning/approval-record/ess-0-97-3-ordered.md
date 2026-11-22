---
format: aep.planning-md/3
id: approval-record:ess-0-97-3-ordered
kind: approval-record
status: draft
title: The upgrade to ESS 0.97.3 is the operator's order
relations:
- decides: story:pin-ess-0-97-3
revision: 1
---
## Decision

story:pin-ess-0-97-3 is accepted and built on a branch with a pull request.

## Who decided

The operator. The host operator's session relayed the operator's order of 2026-11-22
14:17 CET to every session: all upgrade to ESS 0.97.3 and report when their pins are on it.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate and
  the 3 required checks. Sub-agents on Opus or lower.

## What it does not grant

- No release, no tag, no deployment.
