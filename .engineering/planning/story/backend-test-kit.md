---
format: aep.planning-md/3
id: story:backend-test-kit
kind: story
status: active
title: A backend test kit that every implementation of the storage ports must pass
summary: Shared checks of the storage contract, run against the in-memory backend now and the JetStream backend in stage 5.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:core-and-memory-backend
scope:
- confidence: inferred
  path: crates/inbox-backend-testkit
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`crates/inbox-backend-testkit` holds checks written against the generated storage ports alone
(`InboxStorage`, `MessageStorage`), the contract story:core-and-memory-backend states: a snapshot
stored with `put` is answered equal by `get`; a second `put` under one identity replaces the
first; `delete` removes the snapshot from `get` and from `list`; `list` answers every stored
snapshot once, in the same order on two calls with no write between them; two clones of one
backend handle read one store. A backend crate runs the kit with one call per port.

## Acceptance

`cargo test -p inbox-backend-testkit` passes against `MemoryBackend` and fails against a planted
backend whose `delete` leaves the snapshot in `list`, where before this story nothing checks a
storage port apart from the rules that run on it.

## Context

The conformance suite checks commands, states and refusals through the service. It cannot tell a
backend that breaks the storage contract in a case no scenario reaches. The kit is the second
test layer both backends pass; epic:jetstream-backend adds its own checks to it (a write that
survives a restart, a failure between a write and its acknowledgement).

Dedup and expiry are rules of the core, so the kit holds no check of them.

## Scope

- `crates/inbox-backend-testkit/` (new)
