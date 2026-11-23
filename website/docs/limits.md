---
title: Limits
description: What the inbox does not do, or does wrong, in release 0.3.1, the security limits first.
---

# Limits

What the inbox does not do, or does wrong, in release 0.3.1. The security limits come first.

**Use it for the principals of one operator only**: that operator's own sessions and hosts.
The service refuses, before binding, any address that is not a loopback address, and the
client refuses an `INBOX_URL` outside loopback.

## Security findings, open

### 1. A principal that holds a message id learns whether it is live

A post into an open inbox answers `id-taken` (409, error `MessageIdTaken`) for a `message_id`
that a stored message of another sender carries, and `posted`, which stores a message, for a free
one. So a principal that holds a message id learns from a post whether the id is live, and it
can take a free id first, after which the sender's post is refused `id-taken`. A message id is a
ULID the sender's client makes, and it carries its creation time; the client has no flag to
choose one.

The same post to an address no inbox has, or into a closed inbox, answers the same for a held
and a free id: `unknown-inbox` or `inbox-closed`, or `duplicate-key` when a live message of the
caller carries the post's dedup key. `duplicate` answers the sender's own stored message only,
also after the recipient closed the inbox.

- Accepted for principals of one operator.
- A message id unique per sender removes it. That change is planned
  ([features](status.md#planned)) and required before principals of different operators share
  one service.

### 2. A signed request is accepted again inside its 60 s window

The service keeps no record of the requests it has seen. A request signature is valid from
60 s before to 60 s after the service's clock reading, both bounds inclusive. A signed request
sent again inside that window is answered again. Posts are idempotent by `message_id` and
`dedup_key`; a repeated `Answer` is answered `already-settled`. Version 1 leaves a replay
record out.

## The service

- **Loopback only.**
- **No second host and no wake-up** of an idle session.
- **One backend, in memory.** A restart of the service loses every message. The sender posts
  again with `inbox repost`; dedup by `message_id` and `dedup_key` makes the repost arrive
  once.
- **Nothing is deployed**, and no release attaches a binary ([releases](releases.md)).
- One request per connection: no keep-alive and no pipelining.
- The key file is not reloaded while the service runs.

## `inbox repost` can deliver a read note once more

A note stays in its sender's outbox file until it expires, 24 hours by default. `repost`
leaves alone an envelope the current start of the service already took
([first use](first-use.md#the-outbox-file)). A note is delivered a second time in 3 cases:

- The first `repost` after a restart of the service posts every kept envelope. The service
  keeps no record of a handled note, so a note its recipient read before the restart is posted
  and delivered again.
- A service that sends no `Inbox-Epoch` header (an older one) gives the client nothing to
  compare: every `repost` posts every kept envelope.
- An epochs file (`<outbox>.epochs`) that is lost or unreadable, or a line of it that is
  malformed or doubled, makes `repost` post the envelopes concerned.

A message the service still holds is answered `duplicate` and changes nothing.

## A collect whose answer is lost loses the answer

`inbox collect` takes the answer or the refusal of a message off the service: the service
removes the message and, in the same request, answers with what it held: the message as
`inbox outbox` shows it, with its state, who answered, the decided option, the reply and the
refusal reason. `collect` prints that and only then drops the envelope from the outbox file. When the answer does not reach the client, for
example because the connection breaks after the service removed the message, the reply or the
refusal is gone:

- a second `inbox collect` of that message answers `unknown-message`;
- the envelope stays in the outbox file until it expires. `inbox repost` leaves it alone until
  the service is restarted and then posts it again, so its recipient gets it once more.

The service keeps no receipt of a collected message.

## The generated API documents

The input `from` of `PostMessage` carries the example value `principal-262144` in the OpenAPI
documents and in the JSON Schema of `PostMessage`. It is a placeholder, not an address anybody
holds ([beyond10x/ess#804](https://github.com/beyond10x/ess/issues/804)).

## Signing, by design of version 1

From [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md):
no revocation list (a short `valid_until` and an edit of the key file replace it), one operator
key with no rotation, no hardware-backed keys, no payload encryption. A compromised host can
sign as its own agents until their key entries lapse.
