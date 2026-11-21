---
title: Limits
description: The open limits of the inbox on main, the security findings first.
---

# Limits

What the inbox on `main` does not do, or does wrong, today. The security findings come first.
Each names the draft story that would close it; a draft is filed and not yet accepted or
scheduled. The stories are in
[`.engineering/planning/story/`](https://github.com/beyond10x/inbox/tree/main/.engineering/planning/story).

**Until the first finding below is fixed, the service must not listen outside loopback and must
not be used by a real session.** In this stage the service refuses, before binding, any address
that is not a loopback address.

## Security findings, open

### 1. A signed envelope can be posted again under another caller

The service does not check that the envelope's `from` is the caller that signed the request.
`PostMessage` reads no attribute of the caller. A principal that holds another principal's
signed envelope can send the same bytes again under its own request signature. Every recipient
holds the envelopes it received: the `Pending` view hands them out. While the first message is
stored, the repost is refused as a duplicate. Once the first message has been handled and
removed, the service answers `posted` a second time: the message reappears under the original
sender's name, in the same inbox and in the sender's outbox, and a decision with
`default_on_silence` gives the sender a second decided answer. Time does not end this:
`already-expired` compares the envelope's own `expires` with its own `created`, not with the
clock. Who holds an envelope can therefore make it arrive again, as often as it is handled.

- Story: `story:post-from-is-the-caller` (draft). It refuses a post or an answer whose `from` is
  not the caller's principal, as a rule of the specification.
- Proof: the case `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` in
  [`crates/inbox-server/tests/adversary.rs`](https://github.com/beyond10x/inbox/blob/main/crates/inbox-server/tests/adversary.rs)
  shows the repost answered `posted`. It is marked `#[ignore]` until the story lands, so
  `task check` does not run it.
- What to do until then: keep the service on loopback, and do not use it from a real session.

### 2. A refusal tells a third principal whether a message exists

`read`, `collect`, `refuse`, `answer` and `mark-delivered` answer `unknown-message` (404) for an
id no message carries and `not-recipient` (403) for a message addressed to another inbox. Any
authenticated client that knows or guesses a message id learns whether that message exists. A
message id is a ULID, which carries its creation time.

- Story: `story:foreign-message-id-answers-as-unknown` (draft).
- Proof: the ignored case `adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message`
  in the same file.

### 3. A dedup key is shared by every sender

`duplicate-key` compares the dedup key alone over every stored message. A principal that posts
into its own inbox under a key learns from `duplicate-key` whether any message carries that key,
and by posting first it makes another sender's post under the same key fail.

- Story: `story:dedup-key-scope` (draft).
- Proof: the ignored case `adv_a_third_principal_cannot_take_another_senders_dedup_key` in the
  same file.

### 4. A signed request is accepted again inside its 60 s window

The service keeps no record of the requests it has seen, and neither does the verifier. A
request signature is valid from 60 s before to 60 s after the service's clock reading, both
bounds inclusive. A signed request sent again inside that window is answered again. Posts are
idempotent by `message_id` and `dedup_key`; a repeated `Answer` is answered `already-settled`.
Source: [`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md),
"The request signature". No story is filed; version 1 leaves a replay record out.

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

- **The run over HTTP witnesses 20 of the 93 scenarios.** All 93 pass against the core in
  process. Against a running `inbox serve`, 73 are unsupported and listed with their
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
    signature passes all 93.
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
- Two ESS issues the code works around:
  - [beyond10x/ess#798](https://github.com/beyond10x/ess/issues/798): the `PLAN.md` that ESS
    writes states an order of refusals that the interpreted target does not follow. The
    contract text of `PostMessage` in `generated/rust/inbox/PLAN.md` states such an order; the
    core follows the order the interpreted target answers, not the text.
  - [beyond10x/ess#799](https://github.com/beyond10x/ess/issues/799): for a cleared optional
    member of a view row the suite claims `null`, while the generated Rust server leaves the
    member out. The runner treats an absent top-level member and `null` as one value; one
    adversary case that needs the exact comparison is ignored until the issue is closed.

## Signing, by design of version 1

From [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md):
no revocation list (a short `valid_until` and an edit of the key file replace it), one operator
key with no rotation, no hardware-backed keys, no payload encryption. A compromised host can
sign as its own agents until their key entries lapse.
