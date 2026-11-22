---
format: aep.planning-md/3
id: approval-record:d12-test-budget-story-taken-on-default
kind: approval-record
status: draft
title: 'D12 taken on default: the test budget story is accepted'
relations:
- decides: story:test-step-enforces-its-budget
revision: 1
---
## Decision

story:test-step-enforces-its-budget is accepted and built now, on the branch `chore/test-budget`
with a pull request.

## Who decided

Nobody answered. The coordinating session `inbox` put the question to the host operator's
session on 2026-11-22 as D12, with 3 lettered options and option A as the default:

- A (default): accept the story and build it on a branch with a pull request.
- B: leave it a draft.
- C: accept it together with story:no-test-holds-a-wall-clock-time and build both.

15 minutes passed without an answer. The rule "A default proceeds" of the folder this repository
is developed in (2026-11-21) says that the default is then taken, that the step stays reversible
and that the next report says "D12 taken on default". The rule "Tests stay fast" of the same
folder names this story as the first budget story of a repository whose runner does not kill a
test at 60 s, ahead of the next feature.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate and
  the 3 required checks, as every change of this repository reaches `main`.
- Sub-agents on Opus or lower.

## What it does not grant

- No release, no tag, no deployment.
- Nothing about story:no-test-holds-a-wall-clock-time. It stays a draft.
- An answer of the operator or of the host operator that arrives later replaces this record.
