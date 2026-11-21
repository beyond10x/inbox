---
format: aep.planning-md/3
id: story:repost-does-not-redeliver-a-handled-note
kind: story
status: draft
title: Repost does not deliver again a note its recipient already read
summary: inbox repost posts a note again after its recipient read it and the service answers posted; the client cannot tell a read note from one a restart dropped.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-cli-client-pass-1
revision: 1
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

## Scope

- `crates/inbox-cli/src/commands.rs`, `crates/inbox-cli/src/outbox.rs`,
  `crates/inbox-cli/tests/adversary.rs`
- `ess/domains/messaging.yaml`, `generated/`, `crates/inbox-core/` (first way), or
- `crates/inbox-server/` (second way)
