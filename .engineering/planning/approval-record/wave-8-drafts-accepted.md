---
format: aep.planning-md/3
id: approval-record:wave-8-drafts-accepted
kind: approval-record
status: draft
title: 'Wave 8 accepted: the 6 draft stories of the backlog'
relations:
- decides: story:expire-publishes-once
- decides: story:external-refusal-order-cases
- decides: story:repost-does-not-redeliver-a-handled-note
- decides: story:read-decision-expires-on-silence-scenario
- decides: story:silence-leaves-the-other-fields-scenario
- decides: story:post-refusal-order-scenarios
revision: 1
---
## Decision

The 6 draft stories of the backlog are accepted and implemented now, as wave 8:
story:expire-publishes-once, story:external-refusal-order-cases,
story:repost-does-not-redeliver-a-handled-note,
story:read-decision-expires-on-silence-scenario,
story:silence-leaves-the-other-fields-scenario and story:post-refusal-order-scenarios.

## Who decided

The operator, in a message typed on 2026-11-21 into the coordinating session `inbox`. As
received (the message ends where the quotation ends; its last sentence is cut off):

> move the inbox further. Use multiple subagents. Opus, Sonnet, Haiku to implement more stuff.
> Accept all the drafts you have in your stories. Implement them all now. Cut a new

## What it grants

- The moves of the 6 stories from `draft` to `active`, run by the coordinator.
- For wave 8: the opening commit, one commit per unit and per correction round, the merges into
  the integration branch, the closing store commit, the fast-forward of `main` and the bot push
  of `main` through `b10x-gates`.
- Sub-agents on Opus, Sonnet and Haiku.

## What it does not grant

- No release, no tag, no deployment. The sentence "Cut a new" has no object. The coordinator
  reads it as a release and asks the operator once, at the close of the wave, before any tag.
  Until then the host operator's decision of 2026-11-21 holds: no release before
  story:dedup-key-scope and story:post-does-not-tell-a-message-id-exists are implemented.
- Nothing about those 2 stories. They are not drafts. A second coordinating session runs them
  as wave 7 on ESS 0.97.1 at the same time.

## Taken by the coordinator, not by this decision

- The wave is number 8, because wave 7 was opened by the second coordinating session before
  this record was written.
- 2 parts. Part 1 runs now from `main`: story:external-refusal-order-cases,
  story:expire-publishes-once and story:repost-does-not-redeliver-a-handled-note, at the same
  time, each in its own worktree. None of them changes the model, a scenario or a generated
  file.
- Part 2 runs when wave 7 is on `main`: the 3 scenario stories, one after another, because all
  3 change `tools/scenarios.py` and the files it writes, and wave 7 moves the repository to
  ESS 0.97.1 and changes the rules of `PostMessage` that story:post-refusal-order-scenarios
  writes scenarios for. Each of the 3 carries a `depends_on` relation to story:dedup-key-scope,
  and story:post-refusal-order-scenarios one to
  story:post-does-not-tell-a-message-id-exists as well.
- The points the drafts left open are settled in each story's Decisions section.
