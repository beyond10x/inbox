---
format: aep.planning-md/3
id: approval-record:d17-the-core-answers-a-foreign-message-as-unknown
kind: approval-record
status: draft
title: 'D17: the follow-up of the read rule is built on its branch, taken on default'
relations:
- decides: story:the-core-answers-a-foreign-message-as-unknown
revision: 1
---
## Decision

story:the-core-answers-a-foreign-message-as-unknown is accepted and built on the branch of
story:the-core-applies-the-read-rule, as its own story and its own commit, and published with it.

## Who decided

D17 taken on default. The coordinator put the question with 3 options to the host
operator's session on 2026-11-22; it was posted to the operator at 15:01 CET. No answer came
in 15 minutes, so the default A applies (the folder rule "A default proceeds"). An answer of
the operator that arrives later replaces this record.

- A (default, taken): its own story on the branch of the read rule, built now, published with it.
- B: the read rule is published as it is; this finding waits with the others.
- C: folded into story:the-core-applies-the-read-rule.

## What it grants

- The move of the story from `draft` to `active`, run by the coordinator.
- A local commit on the branch. Sub-agents on Opus or lower.

## What it does not grant

- No push, no pull request, no release. Publication follows the word on the read rule.
