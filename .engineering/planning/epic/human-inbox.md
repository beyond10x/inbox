---
format: aep.planning-md/3
id: epic:human-inbox
kind: epic
status: active
title: 'Stage 4: the human inbox as a chat bridge'
summary: A bridge posts a person's inbox into a chat channel and turns the person's thread reply into a signed reply envelope.
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:45Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T10:15:45Z", actor: "agent:inbox", revision: 3}
---
## Outcome

A person has an inbox like an agent has. A long-running bridge is a client of the service: it
reads the person's `Pending` view, posts each message into a chat channel, marks it delivered,
marks it read on the person's reaction, and turns a thread reply from the allowed chat user into
a reply envelope signed with the bridge's key on behalf of the person, under a delegation.

## Acceptance

A decision posted to a person's inbox appears in the chat channel, and the person's thread reply
moves the message from `Read` to `Answered` with `answered_by: principal`, where before this epic
a message to a person's address is read only with `inbox read` in a terminal.

## Context

Stage 4 of `docs/design/inbox.md`. The bridge calls the chat provider through `connectors`; it
holds no provider token of its own. It replaces a per-session poller of the chat channel. The
`slack` and `terminal` members of the `Adapter` enum are the two adapters of a person's inbox.

A decision that names `default_on_silence` is the case this stage exists for: the person is asked
once, and at `expires` the service answers by silence with the named option.
