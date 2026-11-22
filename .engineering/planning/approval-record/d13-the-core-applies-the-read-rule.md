---
format: aep.planning-md/3
id: approval-record:d13-the-core-applies-the-read-rule
kind: approval-record
status: draft
title: 'D13: story:the-core-applies-the-read-rule is accepted, the operator''s pick B'
relations:
- decides: story:the-core-applies-the-read-rule
revision: 1
---
## Decision

story:the-core-applies-the-read-rule is accepted and built on a local branch. The branch, the story and
the finding it answers are published together, with the fix, and not before.

## Who decided

The operator. A source review of the release 0.2.0 reported 11 findings. The coordinating
session `inbox` put the question what happens with them to the host operator's session on
2026-11-22 as D13, with 3 lettered options and no default:

- A: file all 11 findings as draft stories at once and fix in the order 2, 3, 1.
- B: fix the findings 2, 3 and 1 first and publish each with its fix; the other 8 wait.
- C: file nothing.

The host operator's session relayed the operator's pick of 13:57 CET: option B.
This story is finding 1.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- One local branch and its commits. Sub-agents on Opus or lower.

## What it does not grant

- No push and no pull request before the host operator's session has seen the finished fix.
- No release, no tag, no deployment.
- Nothing about the other findings of the review.
