---
format: aep.planning-md/3
id: approval-record:d12-test-budget-story-taken-on-default
kind: approval-record
status: draft
title: 'D12: the test budget story is accepted, the operator''s pick A'
relations:
- decides: story:test-step-enforces-its-budget
revision: 3
---
## Decision

story:test-step-enforces-its-budget is accepted and built, on the branch `chore/test-budget`
with a pull request.

## Who decided

The operator. The coordinating session `inbox` put the question to the host operator's session
on 2026-11-22 as D12, with 3 lettered options and option A as the default:

- A (default): accept the story and build it on a branch with a pull request.
- B: leave it a draft.
- C: accept it together with story:no-test-holds-a-wall-clock-time and build both.

The host operator's session relayed on 2026-11-22, after the work had reached `main`: the
operator picked "A: accept and build it" at 12:11 CET, and the relay was late.

What happened before the relay arrived: 15 minutes passed without an answer, and the coordinator
took option A as the rule "A default proceeds" of the folder this repository is developed in
(2026-11-21) says. The first version of this record said "Nobody answered" and named the
decision as taken on default; its id still carries those words. The default and the operator's
pick are the same option, so nothing of the work changes.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One branch, its commits, a pull request, and the fast-forward of `main` after a green gate and
  the 3 required checks, as every change of this repository reaches `main`.
- Sub-agents on Opus or lower.

## What it does not grant

- No release, no tag, no deployment.
- Nothing about story:no-test-holds-a-wall-clock-time. It stays a draft.
