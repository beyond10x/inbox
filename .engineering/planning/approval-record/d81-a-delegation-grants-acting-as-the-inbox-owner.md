---
format: aep.planning-md/3
id: approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner
kind: approval-record
status: draft
title: 'D81: a delegation lets its agent act as the owner of its principal''s inbox, the operator''s pick B'
relations:
- decides: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
- decides: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
- decides: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
revision: 1
---
## Decision

Option B of decision-blocker:delegation-grants-inbox-ownership: a delegation lets its agent act
as the owner of the inbox of its principal (`on_behalf_of`), and the chat bridge holds a key of
its own. All 7 draft stories of stage 4 stay; the 3 stories the blocker stopped are no longer
stopped by it.

## Who decided

The operator, on 2026-11-23 between 10:50 and 10:59 CET, in the terminal of the host operator's
session, which relayed the pick to the coordinator: D81, "B: delegation". The coordinator had
put the question with 3 lettered options and no default, because it is a change of
architecture.

## What it grants

- The move of the blocker to `cleared`.
- The 3 stories keep their place in the draft set and their edges.

## What it does not grant

- No acceptance of a stage 4 story and no stage 4 code: the 7 drafts are put to the operator as
  one list, and each is accepted by the operator's answer to it.
- No answer to any other Open item of the 7 stories.
