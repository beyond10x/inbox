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
task.

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
   marks, reads, answers or refuses its messages; only the sender collects.
6. **Dedup holds among live messages only**, by `message_id` and by `dedup_key`. A refused post
   stores nothing.
7. **`on_behalf_of` is always present** and equals `from` when the sender speaks for nobody else.
   A reply is the input of `Answer`, not a stored kind. `Body` is at most 65536 bytes.
8. **The wire API is the one ESS derives**: `POST /messaging/commands/<wire>` and
   `GET /messaging/views/<wire>` (`generated/openapi`). There is no `ess-http/1` binding file.
   Ids and addresses travel in bodies and query parameters, never in a path segment.

## What the specification cannot say (ESS 0.96.0)

Each item is a rule of the service that stage 2 implements and tests outside the suite, or a
limit of synthesis that an authored scenario makes up for. None is filed in the `ess` repository
yet.

- **No per-caller filter on a view.** Who may read which rows is a rule of the service: a row is
  answered to the owner of the inbox in `to` or to the sender in `from`.
- **No blocking read.** `inbox wait` is the `Pending` view held open until it has a row.
- **A value is not testable against the elements of an input list.** "The letters name options
  of the decision" is the external refusal `unknown-option`.
- **`{subject: <field>}` is refused beside `deletes:`**, so `MessageExpired` carries the id alone
  and not the sender and the thread. The refusal text names `creates:`.
- **`instance: caller.<attribute>` is not available**, so `OpenInbox` takes the address as input
  and refuses one that is not the caller's.
- **Synthesis arranges no caller for a guard over a stored field** that was not set from the
  caller, no second row for a row-set guard over the entity a command creates (the `dedup_key`
  rule), and only notes once an input guard depends on `kind`. 47 generated scenarios are
  refused for these reasons and are covered by authored scenarios (`covers:`).
- **50 more refusals are `ESS-SYNTH-013`**: type-invariant checks at view fields for which
  synthesis arranges no row. 42 are covered: `decision-answered-by-the-operator` reads an
  answered decision, its reply and its option in `Settled`, `Replies`, `Thread`, `Outbox`,
  `Messages` and `Due`, and `read-decision-waits-for-its-answer` reads a read decision in
  `Unanswered`. The other 8 are `reply` and `decided_option` of `Pending` and `Unanswered`. No
  sequence of commands leaves a row there: those views hold a message in `Sent`, `Delivered` or
  `Read`, and only an answered message carries a reply or a decided option. They stay uncovered
  until the two fields leave those two views, a change to the model.
- **`task coverage` on ESS 0.96.0**: synthesis writes 93 scenarios (24 authored) and refuses 97;
  89 are covered by authored scenarios and 8 are not, so it exits 1 and stays out of
  `task check`. The suite it writes runs on the interpreted target as 93 passed and reports
  `conformance_status: inconclusive`.

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
