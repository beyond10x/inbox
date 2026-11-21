---
format: aep.planning-md/3
id: epic:two-hosts-and-wake
kind: epic
status: draft
title: 'Stage 3: two hosts through one service, and waking an idle session'
summary: A session on a second host reaches the service over TLS; an idle session takes its next turn after a post.
revision: 2
---
## Outcome

A session on one host and a session on a second host exchange a request and a reply through one
`inbox serve`. The service is reached over TLS with a certificate the clients pin. An idle agent
session is woken by a post to its inbox: the harness adapter holds `inbox wait` open as a
background task and the session resumes when it returns.

## Acceptance

An idle session on the second host takes its next turn within 60 s of a post to its inbox from a
session on the first host, where before this epic a post reaches only sessions on the host that
runs the service and an idle session reads its inbox only when something else wakes it.

## Context

Stage 3 of `docs/design/inbox.md`. It adds the transport (TLS, a reachable listen address or a
tunnel), the wake adapters (`claude_code`, `metaharness`, `terminal` of the `Adapter` enum) and
the client reposting its outbox by itself after a reconnect. The outbox file and the manual
`inbox repost` are stage 2 (story:cli-client). The restart rule is checked across hosts
here: after a restart of the service the sender's repost arrives once.

Risk: waking an idle session relies on the harness resuming a session when a background task
exits. That behaviour belongs to the harness and can change.
