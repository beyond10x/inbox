---
format: aep.planning-md/3
id: story:the-core-applies-the-read-rule
kind: story
status: implemented
title: The core gives a client only the rows of its own messages
summary: The read rule is in the HTTP listener only, so an embedding of inbox-core hands a third principal the rows of messages between others.
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:00:50Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T13:00:50Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T15:30:11Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A view read through the core gives a client only the message rows it is a party of: the rows
whose `to` is the caller's inbox or whose `from` is the caller's principal. The rule is applied
where both entries pass: `inbox_core::handle` for an embedding and the HTTP service give the same
rows to the same caller. The views of the service's own expiry task stay unrestricted behind
their actor.

Before this story the read rule was in the HTTP listener only (`readable` in
`crates/inbox-server/src/service.rs`). `inbox_core::handle`, the documented entry for an
embedding that serves another caller per request, checked that the actor may call the view and
ran it; `Outbox`, `Settled`, `Pending`, `Unanswered`, `Thread` and `Replies` select by the
parameter the caller sends and compared no row with the caller. An embedding that handed the
core's answer on gave a third principal bodies, replies and signatures of messages between
others. `inbox serve` applied the filter.

## Acceptance

- For a client that is neither the owner of the inbox in `to` nor the sender in `from`, each of
  the 6 message views answers through `inbox_core::handle` with no row of that message, whatever
  parameter the caller sends.
- The two parties of a message get the same rows from `inbox_core::handle` and from the HTTP
  service.
- `Due` and `Messages` stay unrestricted for the actor `Sweeper`, and a client is still refused
  both.
- The rule exists once: the listener holds no second copy of it.
- Where ESS can state the rule (a view filter over the caller's attributes), it is in `ess/`.
  Where it cannot, the rule is in `inbox-core`, and `docs/design/inbox.md` lists it under what
  the specification cannot say.

## Context

Reported by a source review of the release 0.2.0 (finding 1 of 11). The operator decided on
2026-11-22 that the fix is built first and published together with the finding
(approval-record:d13-the-core-applies-the-read-rule).

## Scope

- `crates/inbox-core/src/lib.rs`: `readable` and `party`, the one copy of the read rule;
  `handle_with_events` applies it to every answer of a caller that is not the expiry task.
- `crates/inbox-server/src/service.rs`: the listener calls `inbox_core::readable`; its own
  `visible` and `answer_rows` are removed.
- `crates/inbox-conformance/src/core_target.rs`, `src/lib.rs`: the core target reads a view as
  each client of the scenario and joins the rows, as the wire target does.
- `crates/inbox-core/tests/read_rule.rs` (5 cases), `crates/inbox-server/tests/read_rule.rs`
  (3 cases), `crates/inbox-core/tests/adversary_read_rule.rs` (7 cases),
  `crates/inbox-server/tests/adversary_read_rule.rs` (1 case), `tests/main.rs` of both crates,
  `crates/inbox-core/tests/core/support.rs`.
- `docs/design/inbox.md`, `docs/design/service.md`, `docs/checks.md`, `AGENTS.md`,
  `CHANGELOG.md`.
- `ess/` and `generated/` are unchanged: ESS 0.97.3 refuses a view filter over the caller
  (`ESS-VIEW-003`), so the rule is in `inbox-core` and `docs/design/inbox.md` lists it under
  what the specification cannot say.
