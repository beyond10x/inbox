# Design: the inbox

Status: stage 1 (specification and conformance suite) is done. No service code exists yet.
The specification in `ess/` is the authority; where this note and `ess/` disagree, `ess/` holds.

## Problem

Agents run in separate sessions, in separate harnesses and on separate hosts. Today a message
between two of them travels over whatever the harness offers: it is not durable, it has no
address that outlives a session, and the receiver cannot check who sent it or on whose authority.

The inbox gives every agent and every person one durable, addressable inbox, and gives every
message a signed envelope.

Non-goals of version 1: history of handled messages, group inboxes, payload encryption,
delegation chains, a general identity service.

## Shape

- One service, `inbox serve`. Every client (the CLI, a harness plugin, a chat bridge) talks to the
  service only, with its own signing key. No client talks to a backend.
- Backends are inside the service, behind one storage contract, chosen by the service's
  configuration.
- The version 1 backend is the service's own: in memory, not persistent. A message is removed
  when it is handled or when `expires` passes. `expires` is mandatory, so memory is bounded.
- A restart of the service drops everything. The sender keeps each message until it is handled
  and posts it again after a reconnect; the owner opens its inbox again. Dedup by `message_id`
  and by `dedup_key` makes the repost arrive once.
- JetStream is the second backend, for persistence, in a later stage. Both backends pass the same
  conformance suite, run through the service.
- The service verifies the signature and the authority of every post and every reply before it
  stores anything. A message that fails is refused, the refusal is the sender's answer, and
  nothing acts on the message.

## The model (`ess/domains/messaging.yaml`)

Two entities.

- `Inbox`: an address, its owner and its wake adapter. Open or Closed.
- `Message`: one signed envelope in one inbox. `Sent → Delivered → Read → Answered | Refused`.

The envelope: `message_id`, `from`, `on_behalf_of`, `authority`, `to`, `kind`
(`note`, `request`, `decision`), `thread`, `reply_shape`, `decision` (lettered options, a
recommended one), `default_on_silence`, `created`, `expires`, `dedup_key`, `body`, `signature`.

Nine commands: `OpenInbox`, `CloseInbox`, `PostMessage`, `MarkDelivered`, `MarkRead`, `Answer`,
`Refuse`, `Collect`, and `Expire`, which only the service's expiry task sends.

Nine views: `Pending` and `Unanswered` for the owner of an inbox, `Outbox` and `Settled` for a
sender, `Thread` and `Replies` for a thread, `Inboxes`, and `Due` and `Messages` for the expiry
task. The views of messages publish every field of the message and its state, except that
`Pending` and `Unanswered`, which hold messages in `Sent`, `Delivered` or `Read`, leave out 4:
`reply` and `decided_option`, which only an answered message carries, and `answered_by` and
`refusal`, which only an answered or a refused message carries. `Replies`, which holds answered
messages only, leaves out `refusal`, which only a refused message carries.

When a message is removed:

| Kind | Removed by |
|---|---|
| note | `MarkRead` by the owner of the inbox, or `Expire` |
| request, decision | `Collect` by the sender after it is answered or refused, or `Expire` |

A decision that names `default_on_silence` and is not answered at `expires` is answered by
silence with that option (`answered_by: silence`) and waits for the sender until `keep_until`.

## Decisions taken while modelling

Each can be changed; the alternative is named where there is one.

1. **One record per message.** The reply is stored on the message it answers (`answered_by`,
   `decided_option`, `reply`) and waits there until the sender runs `Collect` or `expires`
   passes. There is no separate delivery record, and "expired" is a removal, not a state.
2. **Fan-out is the sender's.** A request to 11 recipients is 11 envelopes under one `thread`;
   `Thread` and `Replies` collect them. There is no group inbox in version 1. Alternative: the
   service expands a group address. ESS 0.96.0 has no construct that creates one record per
   stored member.
3. **The clock is an input.** `Expire` carries the clock reading (`now`, `keep_until`) and the
   view `Due` takes it as a parameter, so a scenario decides what time it is. `AlreadyExpired` on
   a post is `expires <= created`.
4. **Keys and delegations are not entities of this domain.** They sit behind the
   `SignatureVerifier` port; the specification names its refusals (`signature-invalid`,
   `authority-invalid`) as external outcomes. Version 1 ships a small verifier that reads a key
   file signed by the operator key. The file's format is typed in the specification (`KeyFile`,
   `KeyEntry`, `DelegationEntry`) and the service stores none of it. A delegation covers every
   message its agent sends for its principal until `valid_until`: version 1 has no narrower scope
   and no chain. Alternative: a delegation that names the kinds it covers. General agent identity
   belongs to Mandate.
5. **Ownership.** A caller carries a `principal` and an `inbox` address, the same text in
   version 1. Only the caller whose `inbox` is the address opens it; only the owner closes it,
   marks, reads, answers or refuses its messages; only the sender collects. A post or an answer
   whose `from` is not the caller's `principal` is refused `not-sender` (`NotSender`) before every
   other refusal of the command, so a principal that holds another's signed envelope cannot send
   it again as its own request, and learns nothing about a stored message from the answer. With
   a delegation the caller is still `from`; `on_behalf_of` is not compared. A caller that is
   neither the owner of the inbox in `to` nor the sender in `from` of a stored message gets
   `foreign-message` (error `UnknownMessage`) from `MarkDelivered`, `MarkRead`, `Answer`,
   `Refuse` and `Collect`, after `unknown-message` and before `not-recipient`, `not-sender` and
   every refusal of the message's state; the service answers it as `unknown-message`
   (`docs/design/service.md`), so from these 5 commands a third principal cannot tell a message
   between others from no message. `PostMessage` tells it only into an open inbox (decision 6).
   The sender
   still gets `not-recipient` from the 4 commands of the owner, and the owner `not-sender` from
   `Collect`: each of them knows the message exists. A message id is a ULID and not a secret.
6. **Dedup holds among live messages only**, by `message_id` and by `dedup_key`. A refused post
   stores nothing. A dedup key belongs to its sender: `duplicate-key` compares the key among the
   stored messages whose `from` is the `from` of the post, which `not-sender` holds to the
   caller. A sender that uses one key for two inboxes is refused the second post; the key of
   another sender, a third principal's in its own inbox included, refuses nothing, and a post
   tells nobody whether another sender uses a key. A message id is one name space for every
   sender, and `duplicate` answers only the sender's own stored message: a post under the id of
   another sender's live message is refused `id-taken` (`MessageIdTaken`), after every other
   refusal. The row refusals of `PostMessage` are answered in this order (measured on the ESS
   0.97.1 interpreted target): `unknown-inbox`, `duplicate`, `duplicate-key`, `inbox-closed`,
   `id-taken`. The model declares `duplicate` before `unknown-inbox`, and the interpreter still
   answers `unknown-inbox` (`exists: false`) first; a sender's repost goes to the inbox its
   message is in, which exists, so it answers `duplicate`, also after the recipient closed the
   inbox. A post to an address no inbox has, or into a closed inbox, answers alike for an id
   another sender holds and a free one (`story:post-does-not-tell-a-message-id-exists`). What
   stays: a principal that holds a message id learns from a post into an open inbox whether the
   id is live (`id-taken` against `posted`, which stores a message), and it can take a free id
   first, after which the sender's post is refused `id-taken`. That is accepted for principals
   of one operator; a message id unique per sender removes it
   (`story:message-id-per-sender`).
7. **`on_behalf_of` is always present** and equals `from` when the sender speaks for nobody else.
   A reply is the input of `Answer`, not a stored kind. `Body` is at most 65536 bytes.
8. **The wire API is the one ESS derives**: `POST /messaging/commands/<wire>` and
   `GET /messaging/views/<wire>` (`generated/openapi`). There is no `ess-http/1` binding file.
   Ids and addresses travel in bodies and query parameters, never in a path segment.

## The order of the refusals of `PostMessage`

Where several refusals of a post hold, the post is answered with the first of: the input-guarded
refusals in declaration order (`not-sender`, `no-authority`, `already-expired`,
`request-without-shape`, `decision-without-options`); then the 3 external refusals
(`unknown-option`, `signature-invalid`, `authority-invalid`), decided before any row is read; then
the row refusals in the order measured on the ESS 0.97.1 interpreted target: `unknown-inbox`,
`duplicate`, `duplicate-key`, `inbox-closed`, `id-taken` (`unknown-inbox` and `inbox-closed` never
hold together, and `id-taken` holds with every `duplicate`). The suite holds the pairs: each
external refusal and `request-without-shape` against each of the 5 row refusals, and each pair of
row refusals one post can make hold, in the authored scenarios
`a-bad-signature-is-refused-before-any-row-is-read`,
`an-unknown-option-is-refused-before-any-row-is-read`,
`a-refused-delegation-is-answered-before-any-row-is-read`,
`a-request-without-shape-is-refused-before-any-row-is-read` and
`row-refusals-of-a-post-answer-in-one-order`; `post-from-another-principal-refused` holds
`not-sender` before `duplicate`, and synthesis refuses a scenario that expects a row refusal where
an input guard holds (`ESS-AUTHOR-041`). A conformance step forces one external outcome, so the
order among the 3 external refusals is held by the cases of the core
(`crates/inbox-core/tests/core/precedence.rs`, `crates/inbox-core/tests/adversary_w8_order.rs`),
not by the suite. The contract text of `PostMessage` in `generated/rust/inbox/PLAN.md` (line 117)
states another order: "Selection precedence: on commands with `when_related:`, check
`existing_instance` then `exists: false` before input-guarded refusals; … then accepting and
external branches in declaration order", which puts `unknown-inbox` before the input-guarded
refusals and the external refusals after every row refusal. The core and the service follow the
interpreter, which the suite runs against.

## What the specification cannot say (ESS 0.97.1)

Each item is a rule of the service that stage 2 implements and tests outside the suite, or a
limit of synthesis that an authored scenario makes up for. Only the item on `from` is filed in
the `ess` repository (beyond10x/ess#804).

- **No per-caller filter on a view.** Who may read which rows is a rule of the service: a row is
  answered to the owner of the inbox in `to` or to the sender in `from`.
- **No blocking read.** `inbox wait` is the `Pending` view held open until it has a row.
- **Two outcomes cannot share one answer** without an `ess-http/1` binding. A second outcome
  named `unknown-message` is refused (`ESS-COMMAND-006`), `alike:` needs a binding this
  repository does not have, and the generated surface answers `foreign-message` `403` under its
  own name. The service gives it the status and the body of `unknown-message`; the model alone
  decides who is a party. Over HTTP the conformance run reads `unknown-message` where the
  scenario expects `foreign-message`, so the cases of `crates/inbox-server/tests/` witness that
  answer, not the suite.
- **A value is not testable against the elements of an input list.** "The letters name options
  of the decision" is the external refusal `unknown-option`.
- **`{subject: <field>}` is refused beside `deletes:`**, so `MessageExpired` carries the id alone
  and not the sender and the thread. The refusal text names `creates:`.
- **`instance: caller.<attribute>` is not available**, so `OpenInbox` takes the address as input
  and refuses one that is not the caller's.
- **Synthesis arranges no caller for a guard over a stored field** that was not set from the
  caller, no second row for a row-set guard over the entity a command creates (the `dedup_key`
  rule), and only notes once an input guard depends on `kind`. 53 generated scenarios are
  refused for these reasons and are covered by authored scenarios (`covers:`).
- **Synthesis does not find a `from` equal to the caller's `principal`** under the `not-sender`
  guard of `PostMessage`, so it would store no message and 49 generated scenarios would leave the
  suite. The input `from` of `PostMessage` carries `example: principal-262144`, the value ESS
  0.97.1 synthesis gives the first caller's `principal` and not an address anybody holds; it goes
  when ESS writes those scenarios without it, which is the fix of beyond10x/ess#804. With it, a
  generated scenario posts as one caller only: 19 of them lost the second run under a second
  caller they had before the guard.
- **42 more refusals are `ESS-SYNTH-013`**: type-invariant checks at view fields for which
  synthesis arranges no row. `decision-answered-by-the-operator` covers 36 of them: it reads an
  answered decision, its reply and its option in `Settled`, `Replies`, `Thread`, `Outbox`,
  `Messages` and `Due`. `read-decision-waits-for-its-answer` covers the other 6: it reads a
  decision that was read and not yet answered in `Unanswered`. Until stage 2 there were 50: 8
  more for `reply` and `decided_option` of `Pending` and `Unanswered`, where no sequence of
  commands leaves a row with either field set. Those 2 fields, and `answered_by` and `refusal`,
  which are as absent there and caused no refusal, left those 2 views, and `refusal` left
  `Replies`, in one change of their wire rows against 0.1.0.
- **`task coverage` on ESS 0.97.1**: synthesis writes 98 scenarios (28 authored) and refuses 95,
  and authored scenarios cover all 95, so it exits 0; `task check` runs it. The suite it writes
  runs on the interpreted target as 98 passed and reports `conformance_status: passed`.
  Since wave 5 of stage 2 `task conform` runs that suite, and its report against the core
  says `passed`.

## Stage 1 result

With ESS 0.96.0: `ess specify validate` exits 0; synthesis writes 92 scenarios (23 authored) and
refuses 97; the run on the interpreted target reports 92 passed, 0 failed, 0 error, 0
unsupported. The report's `conformance_status` is `inconclusive` because of the 50 uncovered
refusals.

## Stages

1. The specification and its suite. Done.
2. `inbox serve` with the in-memory backend, and the CLI. The suite is green against the running
   service; two local sessions exchange a signed request and reply; a handled message and an
   expired message are gone from the service.
3. Two hosts and wake-up. A session on each host exchanges request and reply through one service;
   an idle session takes its next turn within 60 s of a post; after a restart of the service the
   sender's repost arrives once.
4. The human inbox: a chat bridge as a client of the service.
5. The JetStream backend. The same suite is green; a restart loses nothing.
6. Authority in daily use: a relayed decision is acted on because its delegation verifies.

The plan is in `.engineering/planning/`: one epic per stage 2 to 6 and draft stories for stage 2.

Stage 2 starts from what ESS synthesizes. `ess generate synthesize --target rust --layout crate`
on ESS 0.96.0 writes 85 of 87 capabilities: the model types, the behaviour of 7 commands, the
query of 9 views, the storage and context ports, and an HTTP route table with JSON codecs. It
leaves 2 obligations, the behaviours of `PostMessage` (the row-set guard `duplicate-key`) and
`Expire` (a narrowed read of `default_on_silence`). The generated storage ports (`get`, `put`,
`delete`, `list` per entity) are the backend contract. The generated HTTP surface has no
concurrency and no authentication by decision; the service adds both.

## JetStream: not verified

No statement of the specification depends on JetStream. The following are assumptions from
memory, to be read in the NATS documentation before stage 5 relies on one: publish dedup by the
`Nats-Msg-Id` header inside a stream's duplicate window, and that window's default; one durable
pull consumer per inbox with explicit acknowledgement; in-order redelivery with
`max_ack_pending = 1`; server-side per-message TTL; a key-value bucket for the key file.

## Risks

- Waking an idle session relies on a harness resuming a session when a background task exits.
- A compromised host can sign as its own agents until their key entries lapse.
- The service is one process: while it is down nothing moves, and on the in-memory backend a
  restart costs every sender a repost.
