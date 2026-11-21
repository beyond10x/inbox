---
title: Limits
description: The open limits of the inbox on main, the security findings first.
---

# Limits

What the inbox on `main` does not do, or does wrong, today. The security findings come first.
Each open one names the story that would close it and the state of that story. The stories are
in
[`.engineering/planning/story/`](https://github.com/beyond10x/inbox/tree/main/.engineering/planning/story).

**The gate is open for one case: principals of one operator** (that operator's own sessions and
hosts; the host operator's decision, 2026-11-21). `story:message-id-per-sender` is required
before principals of different operators share one service. A release, a listener outside
loopback and a deployment each still need the operator's own order. The service refuses, before
binding, any address that is not a loopback address, and the client refuses an `INBOX_URL`
outside loopback.

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
- Story: `story:message-id-per-sender` (a draft: a message id unique per sender) removes it, and
  is required before principals of different operators share one service.

### 2. A signed request is accepted again inside its 60 s window

The service keeps no record of the requests it has seen, and neither does the verifier. A
request signature is valid from 60 s before to 60 s after the service's clock reading, both
bounds inclusive. A signed request sent again inside that window is answered again. Posts are
idempotent by `message_id` and `dedup_key`; a repeated `Answer` is answered `already-settled`.
Source: [`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md),
"The request signature". No story is filed; version 1 leaves a replay record out.

## Security findings, closed on `main`

On `main` since waves 6 and 7 of stage 2 and in no release. Release 0.1.0 holds the listener
without them.

- **A signed envelope can be posted again under another caller**
  (`story:post-from-is-the-caller`). `post` and `answer` are refused `not-sender` (403) when the
  envelope's `from` is not the caller's principal, as a rule of the specification and before
  every other refusal. Before, a principal that held another principal's signed envelope could
  post the same bytes again under its own request signature, and once the first message was
  handled and removed the service answered `posted` again. The case
  `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` in
  `crates/inbox-server/tests/adversary.rs` runs in `task check`.
- **5 commands told a third principal whether a message exists**
  (`story:foreign-message-id-answers-as-unknown`): `read`, `collect`, `refuse`, `answer` and
  `mark-delivered` answer a caller that is neither the recipient nor the sender of a message
  with `unknown-message` (404), the answer of an id no message carries. Before, they answered
  that caller 403. The case
  `adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message` in the same file
  runs in `task check`.
- **A dedup key was shared by every sender** (`story:dedup-key-scope`, wave 7). `duplicate-key`
  compares the dedup key among the stored messages of the post's sender. A sender that uses
  one key for two inboxes is still refused the second post. Before, a key of any sender refused
  the post, so a third principal learned from a post whether a key was in use and could take
  another sender's key first.
- **A post told whether a message id of another sender exists, for an address no inbox has or a
  closed inbox** (`story:post-does-not-tell-a-message-id-exists`, wave 7). `duplicate` answers the
  sender's own stored message only, and the new refusal `id-taken` comes last of the row
  refusals: `unknown-inbox`, `duplicate`, `duplicate-key`, `inbox-closed`, `id-taken`. Before,
  `duplicate` answered for any stored id and came before `unknown-inbox`. The part that stays is
  finding 1 above.

## What does not exist yet

- **No release holds the binary.** `inbox serve` and the client are on `main` since wave 4
  and are built from source ([first use](first-use.md)); release 0.1.0 has no binary.
- **`inbox repost` delivers a note once more that was read before a restart**
  (`story:repost-does-not-redeliver-a-handled-note`). A note stays in its sender's outbox
  file until it expires, 24 hours by default. Every answer of the service carries the header
  `Inbox-Epoch`, which changes with each start, and the client records it in the epochs file
  beside the outbox file (`<outbox>.epochs`) for each envelope whose post the service
  held. `repost` leaves alone an envelope recorded at the epoch the service names now, so a
  note read in the start that took it is not posted again. What stays open:
  - The first `repost` after a restart of the service posts every kept envelope, because
    the epoch changed. The service keeps no record of a handled note, so it answers `posted`
    for a note its recipient read before the restart as for one the restart dropped.
  - A service that sends no `Inbox-Epoch` (an older one) gives no epoch to compare: every
    `repost` posts every kept envelope.
  - An epochs file that is lost or unreadable, or a line of it that is malformed or doubled,
    gives the envelopes concerned no epoch, and `repost` posts them. A message the service
    still holds is answered `duplicate` and changes nothing; a note that was read is delivered
    again.
- **Loopback only, no TLS.** A listener on a second host and TLS belong to a later stage
  (`epic:two-hosts-and-wake`).
- **One backend, in memory.** A restart of the service loses every message. The design has the
  sender repost after a reconnect; dedup by `message_id` and `dedup_key` makes the repost arrive
  once. A persistent backend is a later stage (`epic:jetstream-backend`).
- **No second host and no wake-up** of an idle session (`epic:two-hosts-and-wake`).
- **Nothing is deployed** (see [releases](releases.md)).
- One request per connection: no keep-alive and no pipelining. The key file is not reloaded
  while the service runs.

## What the checks do not cover

- **The run over HTTP witnesses 25 of the 105 scenarios.** All 105 pass against the core in
  process. Against a running `inbox serve`, 80 are unsupported and listed with their
  reasons in
  [`unsupported.tsv`](https://github.com/beyond10x/inbox/blob/main/crates/inbox-conformance/unsupported.tsv)
  ([what works](status.md) names the reasons). ESS reports that run as `failed`, and the
  gate holds it to the list.
- **The run over HTTP and the 3 acceptance checks do not exercise every rule of the
  listener.** Every request they send is validly signed, with keys and delegations that
  do not lapse, and the run reads a view as every key and joins the rows. A service
  without the read rule, without the check of the request signature or its 60 s window,
  without the lapse of a key or a delegation, without the binding of a key to `from` or
  of a delegation to its pair, or one that serves `Messages` and `Due` to a client,
  passes both (8 of the 12 changed services the adversary built). The tests of
  `crates/inbox-server` and `crates/inbox-verify` fail on each of the 8, and
  `task check` runs them.
- Rules the specification cannot state on ESS 0.97.1 (who may read which rows, the blocking
  read, and others listed in
  [`docs/design/inbox.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/inbox.md))
  are implemented by the service and tested outside the suite.
- Gaps in the suite found by the adversary passes, each a draft story:
  - `story:post-refusal-order-scenarios`: no scenario holds the order in which `PostMessage`
    refuses when two refusals apply; an implementation that reads a row before it checks the
    signature passed the suite when the gap was found (92 scenarios then).
  - `story:expiry-scenarios-hold-the-remaining-cases`: 5 behaviours of `Expire` that no scenario
    holds (cases of the core do); a core that gets one wrong passes `task conform`: a
    read decision that names no silence option is decided on silence, a decision is decided 1
    second before `expires`, a delivered request is decided on silence, `thread` of
    `DecisionTakenOnSilence` is the message id, a refused decision that names a silence option
    is counted as open.
- ESS issues the code works around or waits for:
  - [beyond10x/ess#798](https://github.com/beyond10x/ess/issues/798): the `PLAN.md` that ESS
    writes states an order of refusals that the interpreted target does not follow. The
    contract text of `PostMessage` in `generated/rust/inbox/PLAN.md` states such an order; the
    core follows the order the interpreted target answers, not the text.
  - [beyond10x/ess#799](https://github.com/beyond10x/ess/issues/799): for a cleared optional
    member of a view row the suite claims `null`, while the generated Rust server leaves the
    member out. The runner treats an absent top-level member and `null` as one value; one
    adversary case that needs the exact comparison is ignored until the issue is closed.
  - [beyond10x/ess#804](https://github.com/beyond10x/ess/issues/804): with the guard `not-sender` in the specification,
    synthesis writes no stored message under it, and 49 generated scenarios leave the suite
    (measured on ESS 0.96.0). Until it is fixed the input `from` of `PostMessage` carries the
    example value `principal-262144`, the value synthesis gives its first caller and not an
    address anybody holds. It shows in the generated OpenAPI documents and the JSON Schema of
    `PostMessage` (3 files) and changes no decoder. With it, 19 generated scenarios lost a
    second run under a second caller (also measured on ESS 0.96.0).

## Signing, by design of version 1

From [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md):
no revocation list (a short `valid_until` and an edit of the key file replace it), one operator
key with no rotation, no hardware-backed keys, no payload encryption. A compromised host can
sign as its own agents until their key entries lapse.
