---
format: aep.planning-md/3
id: epic:jetstream-backend
kind: epic
status: draft
title: 'Stage 5: the JetStream backend of the service'
summary: A second backend behind the same contract, chosen by service configuration, so that a restart loses nothing.
revision: 1
---
## Outcome

`inbox serve` runs with a second backend that stores messages in NATS JetStream, chosen by the
service's environment. Clients do not change: they talk to the service only, and NATS credentials
exist on the service host only. Both backends pass the same conformance suite, run through the
service, and the same backend test kit.

## Acceptance

A message posted before a restart of `inbox serve` on the JetStream backend is in the recipient's
`Pending` view after the restart without a repost, where on the in-memory backend the same
restart leaves the view empty.

## Context

Stage 5 of `docs/design/inbox.md`. With a persistent backend the service can keep history: an
audit log, and `Thread` and `Outbox` rows of handled messages. Whether it does is a specification
change decided in this stage.

No statement of the specification depends on a JetStream fact. Every JetStream fact in the design
note is marked "not verified": the duplicate window and its default, per-inbox ordering with
`max_ack_pending`, per-message TTL, the key-value bucket. Each is read in the NATS documentation
before a story of this epic relies on it, and the story cites the page.
