---
format: aep.planning-md/3
id: approval-record:d87-the-7-stories-of-stage-4-are-accepted
kind: approval-record
status: draft
title: 'D87: the 7 stories of stage 4 are accepted, every open item at its default, the operator''s pick A'
relations:
- decides: story:the-chat-bridge-posts-a-pending-message
- decides: story:the-chat-bridge-marks-a-message-read-on-a-reaction
- decides: story:the-chat-bridge-asks-a-decision-once
- decides: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
- decides: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
- decides: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
- decides: story:the-operator-runs-the-chat-bridge-on-the-real-channel
revision: 1
---
## Decision

Option A of D87: all 7 draft stories of stage 4 are accepted. Every item under "Open" of a
story is taken at the default the story names. The coordinator had named 4 of them in the
question; they are taken at their defaults too:

- story:the-chat-bridge-posts-a-pending-message, Open 5: A (i), the write policy of the chat
  connection.
- story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal, Open 2: A, the
  delegation is a signed header and the 6th member of the signed request.
- story:the-operator-runs-the-chat-bridge-on-the-real-channel, Open 1: A, the run is made once
  now and once per release that changes the bridge.
- story:the-operator-runs-the-chat-bridge-on-the-real-channel, Open 3: A, the operator retires
  the old poller outside this repository.

The stories are built in wave order, one after the other, starting with stories 1 to 3.

## Who decided

The operator, on 2026-11-23 at about 11:10 CET, in the terminal of the host operator's session,
which relayed the pick to the coordinator: D87, "A: accept all 7". The coordinator had put the
7 drafts to the operator as one list with lettered options.

## What it grants

- The move of the 7 stories from `draft` to `proposed` now, and of each to `active` when its
  wave starts, run by the coordinator. The move of epic:human-inbox to `active`.
- Branches, their commits, pull requests and the merge to `main` after a green gate and green
  required checks. Sub-agents on Opus or lower.

## What it does not grant

- What the run on the real channel needs from the operator (the link of the chat connection,
  its write policy, the key of the bridge and its delegation, the restart of the service, the
  channel and the allowed chat user): the coordinator asks for it when that story is reached.
- No listener outside loopback and no deployment.
