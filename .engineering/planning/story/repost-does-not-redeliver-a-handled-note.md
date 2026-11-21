---
format: aep.planning-md/3
id: story:repost-does-not-redeliver-a-handled-note
kind: story
status: implemented
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
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 13}
- {from: "proposed", to: "active", at: "2026-11-21T20:14:10Z", actor: "agent:inbox", revision: 14}
- {from: "active", to: "implemented", at: "2026-11-21T23:49:43Z", actor: "agent:inbox", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":11,"verification":1}}}
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

## As built

Wave 8 of stage 2, 2026-11-21: commits `e8266d5` (the unit), `996748a` (an
acceptance case), `a9bd783` and `8b71c0f` (2 corrections), merged as `0ee3b2f`, `8195f85` and
`2e6e987`. Built as the second way of the Decisions above, with one difference named below.

- The service sends the header `Inbox-Epoch` on every answer: 32 lower-case hex digits, one
  value per start, no time and no secret in it.
- The client keeps the file `<outbox>.epochs` beside the outbox file, one line
  `<message id>\t<epoch>` per kept envelope whose post the service said it holds (an accepted
  post, or the outcome `duplicate`; `duplicate-key` records none). `repost` asks the service
  for its epoch when a kept envelope has one, and leaves alone the envelopes recorded at that
  epoch. It posts every other kept envelope, and every one when the service sends no epoch.
- `repost` records each epoch right after its answer. When the epochs file or the outbox file
  cannot be written, every envelope is still posted, the report is printed and the exit is 4;
  that exit decides over a refusal and over an unreadable outbox line.
- Differs from the Decisions: the epoch is not written into the outbox file. A line of that
  file is the envelope's JSON and nothing else, and the encoding the unit first tried (white
  space after the JSON) was rejected by the coordinator; the outbox file keeps its form.
- What stays open is on the limits page: the first `repost` after a restart delivers once more
  a note its recipient read before the restart.
- `tests/acceptance/tests/adversary.rs` changed in `996748a` under a decision of the host
  operator of 2026-11-21: its case removes the epochs file before each of its 2 reposts.
- Adversary: 2 passes. Pass 1 on `e8266d5`: 7 findings, 6 introduced (`a9bd783` corrects them
  and adds the adversary's 7 cases), 1 there before the unit and in the scope of
  story:test-step-enforces-its-budget (a draft)
  (review-result:adversary-repost-does-not-redeliver-pass-1). Pass 2 on `a9bd783`: 4 findings,
  all introduced by the first correction; `8b71c0f` records per answer again and adds the
  adversary's 3 cases (review-result:adversary-repost-does-not-redeliver-pass-2). No third
  pass ran: 2 is the most a unit gets.
- One case of pass 1 was rewritten in `8b71c0f` for story:dedup-key-scope of wave 7: the dedup
  key is held by a second note of the same sender.

Numbers after the wave: `docs/plan/stage-2.md`, "Wave 8".
