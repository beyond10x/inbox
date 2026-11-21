---
format: aep.planning-md/3
id: story:backend-test-kit
kind: story
status: implemented
title: A backend test kit that every implementation of the storage ports must pass
summary: Shared checks of the storage contract, run against the in-memory backend now and the JetStream backend in stage 5.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:core-and-memory-backend
scope:
- confidence: cited
  path: crates/inbox-backend-testkit
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-21T11:42:34Z", actor: "agent:inbox", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":17,"verification":1}}}
---
## Outcome

`crates/inbox-backend-testkit` holds checks written against the backend contract alone: the
generated storage ports (`InboxStorage`, `MessageStorage`) and the command section `Exclusive` of
`inbox-core`. For each storage port: a snapshot stored with `put` is answered equal by `get`; a
second `put` under one identity replaces the first; `delete` removes the snapshot from `get` and
from `list`; `list` answers every stored snapshot once, in the same order on two calls with no
write between them; two clones of one backend handle read one store. For the section: clones of
one backend share it, a second command waits while one runs, and the section is free again after
a command that panics. A backend crate runs the kit with one call per part
(`check_inbox_storage`, `check_message_storage`, `check_exclusive`).

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

As built in wave 3 of stage 2:

- Each of the 3 functions takes a factory that answers a backend over a new, empty store on
  every call, and makes its clones itself. A broken rule panics with one line,
  `backend contract broken: <part>: <rule> (<what was seen>)`.
- The 2 storage checks refuse a factory whose backend already holds a snapshot of the port under
  check. `check_exclusive` cannot check that a factory answers separate stores: it gives every
  backend the factory answers one command that must run within 2 s, and no more.
- A check that waits fails after 2 s. `check_exclusive` holds a command inside the section for
  500 ms while others try to get in; a section that admits a second command only after a longer
  hold is not seen. The crate documentation states this limit.
- `cargo test -p inbox-backend-testkit` runs 29 cases: `MemoryBackend` passes the 3 checks, 12
  planted backends that each break one rule fail on that rule, 1 control passes, and the 2
  adversary files hold 13 cases.
- Adversary pass 1 returned 10 findings and pass 2 returned 5; all 15 were introduced by the
  unit and are fixed (review-result:adversary-backend-test-kit-pass-1, -pass-2).

## Scope

- `crates/inbox-backend-testkit/` (new)
