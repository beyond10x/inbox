---
title: Limits
description: The open limits of the inbox on main, the security findings first.
---

# Limits

What the inbox on `main` does not do, or does wrong, today. The security findings come first.
Each open one names the story that would close it and the state of that story. The stories are
in
[`.engineering/planning/story/`](https://github.com/beyond10x/inbox/tree/main/.engineering/planning/story).

**Until findings 1 and 2 below are closed, the service must not listen outside loopback and
must not be used by a real session, and no NATS backend, no deployment and no release come
before them** (the host operator's decisions, 2026-11-21). In this stage the service refuses, before binding, any address
that is not a loopback address.

## Security findings, open

### 1. A refusal tells a third principal whether a message exists

Closed for 5 commands on `main`, open for `post`.

`read`, `collect`, `refuse`, `answer` and `mark-delivered` answer a caller that is neither the
recipient nor the sender of a message with `unknown-message` (404), the answer of an id no
message carries (`story:foreign-message-id-answers-as-unknown`, on `main` since wave 6 of stage
2). Before, they answered that caller 403, which told any authenticated client that the message
exists.

`post` still tells. It answers `duplicate` for a `message_id` that a stored message carries,
whoever sent it, and it answers that before `unknown-inbox`. A principal that posts as itself to
an address no inbox has gets `duplicate` under the id of a message between two others and
`unknown-inbox` under an id nobody used. Neither post stores anything. A message id is a ULID,
which carries its creation time.

- Story: `story:post-does-not-tell-a-message-id-exists` (accepted on 2026-11-21). It waits
  for the ESS release that finding 2 waits for and runs in one wave with
  `story:dedup-key-scope`.
- The gate named at the top of this page stays closed on this story.
- Proof: the case
  `adv_w6_a_third_principal_cannot_tell_a_message_between_others_from_no_message_by_post` in
  [`crates/inbox-server/tests/adversary_w6_foreign.rs`](https://github.com/beyond10x/inbox/blob/main/crates/inbox-server/tests/adversary_w6_foreign.rs).
  It is marked `#[ignore]` until the story lands, so `task check` does not run it.

### 2. A dedup key is shared by every sender

`duplicate-key` compares the dedup key alone over every stored message. A principal that posts
into its own inbox under a key learns from `duplicate-key` whether any message carries that key,
and by posting first it makes another sender's post under the same key fail.

- Story: `story:dedup-key-scope` (accepted, held). Its rule is settled: the key is compared
  among the stored messages of the post's `from`. With that rule in the specification, ESS
  0.96.0 synthesis writes a generated scenario that ESS's own interpreted target fails
  ([beyond10x/ess#805](https://github.com/beyond10x/ess/issues/805)), so `task check` fails and the unit was stopped before any
  commit. The story is taken up when an ESS release fixes that.
- The gate named at the top of this page stays closed on this story.
- Proof: the ignored case `adv_a_third_principal_cannot_take_another_senders_dedup_key` in
  [`crates/inbox-server/tests/adversary.rs`](https://github.com/beyond10x/inbox/blob/main/crates/inbox-server/tests/adversary.rs).
- What to do until then: keep the service on loopback, and do not use it from a real session.

### 3. A signed request is accepted again inside its 60 s window

The service keeps no record of the requests it has seen, and neither does the verifier. A
request signature is valid from 60 s before to 60 s after the service's clock reading, both
bounds inclusive. A signed request sent again inside that window is answered again. Posts are
idempotent by `message_id` and `dedup_key`; a repeated `Answer` is answered `already-settled`.
Source: [`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md),
"The request signature". No story is filed; version 1 leaves a replay record out.

## Security findings, closed on `main`

On `main` since wave 6 of stage 2 and in no release. Release 0.1.0 holds the listener without
both.

- **A signed envelope can be posted again under another caller**
  (`story:post-from-is-the-caller`). `post` and `answer` are refused `not-sender` (403) when the
  envelope's `from` is not the caller's principal, as a rule of the specification and before
  every other refusal. Before, a principal that held another principal's signed envelope could
  post the same bytes again under its own request signature, and once the first message was
  handled and removed the service answered `posted` again. The case
  `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` in
  `crates/inbox-server/tests/adversary.rs` runs in `task check`.
- **5 commands told a third principal whether a message exists**
  (`story:foreign-message-id-answers-as-unknown`): the closed part of finding 1 above. The case
  `adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message` in the same file
  runs in `task check`.

## What does not exist yet

- **No release holds the binary.** `inbox serve` and the client are on `main` since wave 4
  and are built from source ([first use](first-use.md)); release 0.1.0 has no binary.
- **`inbox repost` delivers a read note again**
  (`story:repost-does-not-redeliver-a-handled-note`). A note stays in its sender's outbox
  file until it expires, 24 hours by default. Once its recipient has read it the service
  holds no message with that id, so `repost` posts it again and the service answers
  `posted`. The service gives one answer for a note that was read and for one a restart
  dropped.
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

- **The run over HTTP witnesses 22 of the 97 scenarios.** All 97 pass against the core in
  process. Against a running `inbox serve`, 75 are unsupported and listed with their
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
- Rules the specification cannot state on ESS 0.96.0 (who may read which rows, the blocking
  read, and others listed in
  [`docs/design/inbox.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/inbox.md))
  are implemented by the service and tested outside the suite.
- Gaps in the suite found by the adversary passes, each a draft story:
  - `story:post-refusal-order-scenarios`: no scenario holds the order in which `PostMessage`
    refuses when two refusals apply; an implementation that reads a row before it checks the
    signature passed the suite when the gap was found (92 scenarios then).
  - `story:external-refusal-order-cases`: no case holds the order of the three external
    refusals of `PostMessage` (`unknown-option`, `signature-invalid`, `authority-invalid`); a
    core that asks them in reverse order passes.
  - `story:read-decision-expires-on-silence-scenario`: no scenario holds that a decision its
    recipient has read is decided on silence at expiry; a core that deletes it passes.
  - `story:silence-leaves-the-other-fields-scenario`: no scenario holds the fields `Expire`
    leaves unchanged when it decides on silence; a core that clears `decision` afterwards
    passes.
  - `story:expire-publishes-once`: nothing bounds `Expire` to one occurrence of the event it
    publishes; a core that publishes it twice passes.
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
    synthesis writes no stored message under it, and 49 generated scenarios leave the suite.
    Until it is fixed the input `from` of `PostMessage` carries the example value
    `principal-262144`, the value synthesis gives its first caller and not an address
    anybody holds. It shows in the generated OpenAPI documents and the JSON Schema of
    `PostMessage` (3 files) and changes no decoder. With it, 19 generated scenarios lost a
    second run under a second caller.
  - [beyond10x/ess#805](https://github.com/beyond10x/ess/issues/805): findings 1 and 2 above wait for it.

## Signing, by design of version 1

From [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md):
no revocation list (a short `valid_until` and an edit of the key file replace it), one operator
key with no rotation, no hardware-backed keys, no payload encryption. A compromised host can
sign as its own agents until their key entries lapse.
