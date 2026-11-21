---
format: aep.planning-md/3
id: story:repost-does-not-redeliver-a-handled-note
kind: story
status: active
title: Repost does not deliver again a note its recipient already read
summary: inbox repost posts a note again after its recipient read it and the service answers posted; the client cannot tell a read note from one a restart dropped.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-cli-client-pass-1
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/inbox-cli/src
- confidence: cited
  path: crates/inbox-cli/tests
- confidence: cited
  path: crates/inbox-server/Cargo.toml
- confidence: cited
  path: crates/inbox-server/src
- confidence: cited
  path: crates/inbox-server/tests/cases.rs
- confidence: cited
  path: crates/inbox-server/tests/support
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: tests/acceptance/src
- confidence: cited
  path: tests/acceptance/tests/checks.rs
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 13}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 14}
---
## Outcome

`inbox repost` delivers a kept envelope again only when the service no longer holds the message
and the recipient has not handled it. Today a note the recipient already read is delivered a
second time: a note is never collectable, so its envelope stays in the sender's outbox file until
it expires (24 h by default); once the recipient has read it, the service holds no live message
with that id, so the next `repost` posts it again and the service answers `posted`.

## Acceptance

The case `repost_does_not_deliver_again_a_note_its_recipient_already_read` in
`crates/inbox-cli/tests/adversary.rs` runs without `#[ignore]` and passes: after the recipient
read a note, `inbox repost` by its sender leaves the recipient's inbox without that note. The
case `repost` after a restart of the service still passes: a message the restart dropped is
delivered again.

## Context

Found by the adversary of story:cli-client in stage 2, wave 4
(review-result:adversary-cli-client-pass-1, finding F1; `crates/inbox-cli/src/commands.rs`,
`repost`, and `crates/inbox-cli/src/outbox.rs`). The finding was declined in that unit: the
client cannot tell a note that was read from a note a restart dropped, because the service gives
the same answer for both and keeps no record of a handled note. story:cli-client's own rule
keeps a note in the outbox file until its expiry. Until this story lands, `inbox repost --help`
and `README.md` say that `repost` is for after a restart of the service and that a note already
read is delivered again.

Two ways are open and the story first decides between them:

- the service keeps the ids of handled messages for the lifetime of each message and answers a
  post of one of them as a duplicate, which the model would have to declare (a change of
  `ess/domains/messaging.yaml` and of the backend contract); or
- the service tells its callers which start it is in (an identifier that changes on restart),
  and the client reposts only envelopes posted to an earlier start.

The first also closes the case for the memory backend only while it runs: a restart drops the
record with the messages. The durable backend of a later stage holds both.

Related: story:dedup-key-scope (the scope of the key a duplicate is recognised by).

## Decisions

Accepted by the operator on 2026-11-21 (approval-record:wave-8-drafts-accepted) and run in
wave 8 of stage 2.

Taken by the coordinator on 2026-11-21, of the 2 ways the Context names: the second.

- `crates/inbox-server` puts a response header `Inbox-Epoch` on every answer it writes. Its
  value is made once when a listener is bound, from 16 random bytes (`getrandom`, which the lock
  file already holds), and differs between 2 starts.
- `crates/inbox-cli` records, per kept envelope in the outbox file, the epoch of the answer that
  said the post arrived (an accepted post, or the refusal `duplicate`). No epoch is recorded for
  any other refusal or for a request that got no answer.
- `inbox repost` asks the service for its epoch once, when at least one kept line carries one,
  and posts again only the lines whose epoch differs from it and the lines without one. A line
  it posts again takes the new epoch.
- The model, `generated/` and `crates/inbox-core` do not change.
- The sentence of the Acceptance "the case `repost` after a restart of the service still
  passes" is read as: after a restart the first `repost` delivers the dropped message again
  (`posted`, and `pending` holds it once). The assertions that exist today also expect a second
  `repost` to be answered `duplicate`; with the epoch recorded, the second `repost` has nothing
  to post. Those assertions are rewritten to that, and a case with a line in the old format
  keeps the `duplicate` path covered.
- Left open and stated in the limits: a note its recipient read before a restart is delivered
  once more by the first `repost` after it. The service keeps no record of a handled note.
- The header is on answers to unauthenticated requests too: a caller without a key can tell
  that the service was restarted. The value carries no time and no secret.
- The unit runs in part 1 of the wave.

## Scope

Derived on 2026-11-21 by an `aep:story-scoper` run for the second way.

- `crates/inbox-server/src/listener.rs`: `fn write` is the one place that writes the head of an
  answer; `Server::bind` makes the epoch; `connection` hands it to `write`.
- `crates/inbox-server/src/service.rs` (`Shared` holds the epoch), `src/auth.rs` and `src/lib.rs`
  (the name of the header, exported), `Cargo.toml` (`getrandom`), `Cargo.lock` (an edge, no new
  package).
- `crates/inbox-server/tests/cases.rs` and `tests/support/mod.rs`: the header on answers of each
  status, and 2 starts with 2 values.
- `crates/inbox-cli/src/client.rs` (`Reply` reads the header), `src/commands.rs` (`post`,
  `repost`), `src/outbox.rs` (the line format, the epoch per line), `src/failure.rs`,
  `src/main.rs` (the help text of `repost`).
- `crates/inbox-cli/tests/adversary.rs` (the case loses its ignore attribute), `tests/cases.rs`
  and `tests/support/mod.rs` (the cases that repost after a restart; `World::kept`).
- `tests/acceptance/tests/checks.rs` (check 3) and `tests/acceptance/src/lib.rs`.
  `tests/acceptance/tests/adversary.rs` expects `posted` then `duplicate` from 2 reposts after a
  restart and is a file the unit does not edit: the unit reports what that case does.
- `docs/design/service.md`: the header.
- At the close, by the coordinator: `README.md`, `website/docs/first-use.md`,
  `website/docs/limits.md`, `website/docs/status.md`, `AGENTS.md`, `CHANGELOG.md`.
- Not touched: `ess/`, `generated/`, `crates/inbox-core`, `crates/inbox-conformance` (its wire
  target reads the status line of an answer and no other header).
