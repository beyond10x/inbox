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

A request names a `reply_shape` (`request-without-shape`). A decision names a `decision` with at
least one option: a decision without one, or with an empty `options` list, is refused
`decision-without-options` (`ShapeMismatch`). The letters of the options are distinct, and
`recommended_option` and `default_on_silence` each name one of them; an envelope without a
decision offers no option, so a `default_on_silence` on it is refused too (`unknown-option`,
`ShapeMismatch`). A reply to a decision names one of its options, a reply to a request names
none, and its body fits the request's `reply_shape`; else `Answer` refuses it `shape-mismatch`
(`ShapeMismatch`), an external outcome (see "What the specification cannot say").

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

A note asks for nothing: `Answer` and `Refuse` of a note are refused `note` (`NotAnswerable`)
and leave it as it was, so a note is never collected.

`Collect` answers `collected` with the settled record it removes, its typed `response`: every
member of the message's `Outbox` row, in its order (`message_id`, `from`, `on_behalf_of`,
`authority`, `to`, `kind`, `thread`, `reply_shape`, `decision`, `default_on_silence`, `created`,
`expires`, `dedup_key`, `body`, `signature`, `answered_by`, `decided_option`, `reply`, `refusal`,
`state`), as the row held them in the request that deletes it. `expires` is there because an
answer replaces it with the reply's, and the reply's signature covers it: with the record alone
the sender can verify the reply it collected. The sender takes what that answer carries, not a
row it read before: an answer or a refusal that lands before the `Collect` is in it, and a
`Collect` before either is `not-settled` and removes nothing. Only the sender gets a record; a
third principal gets `unknown-message` and the recipient `not-sender`, also on a message that is
not settled. The bodies in the record are plain text: `body` is `String` and the reply is a
`CollectedReply`, the fields of `Reply` with its body as `String` (see "What the specification
cannot say"). An answer lost on the way loses the record: the row is gone, and a second
`Collect` is `unknown-message`. Keeping a receipt until `expires` would change when a message is
removed, and is not done.

`Expire` that removes a message publishes `MessageExpired` with the message id, the sender (`to`)
and the thread, as `MessageAnswered`, `MessageRefused` and `DecisionTakenOnSilence` name them.

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
   a post is `expires <= created`, or `expires` at or before the service's clock
   (`expired-on-arrival`). A message in `Sent`, `Delivered` or `Read` whose `expires` is at or
   before the service's clock takes no answer, no refusal, no read mark and no delivery mark:
   `Answer`, `Refuse`, `MarkRead` and `MarkDelivered` refuse it `deadline-passed` (`Expired`),
   after `unknown-message`, `foreign-message` and `not-recipient`, change nothing and publish
   nothing, and the expiry task decides it as before, by silence with `default_on_silence` or by
   removal. So a due note read before the sweep is removed by the sweep with `MessageExpired`,
   as one read after it was, never with `MessageHandled`: the sender gets the same event
   whenever the task runs. A reply whose own `expires` is at or before
   the service's clock is refused `expired-on-arrival` (`AlreadyExpired`) as a post is, and the
   message stays as it was: stored, the answer would carry that past `expires` and the next
   sweep would remove it before its sender could collect it. Like `deadline-passed` it holds only
   for a message in `Sent`, `Delivered` or `Read` (a `when_subject` beside the `external:`): an
   answer to an answered or refused message is `already-settled` whatever its own `expires`. So
   a late answer and the sweep, in either order, leave the same message. These 6 outcomes read the service's Clock port inside
   the serialized section that runs the command, at the full precision of the clock, which is
   also the precision of `Due`'s and `Expire`'s `now`; no member a client sends replaces that
   reading. They are external outcomes (see "What the specification cannot say").
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
   every refusal of the message's state; the core answers it as `unknown-message`
   (`inbox_core::as_unknown`, applied by `handle` and `handle_with_events` and called by the HTTP
   service, `docs/design/service.md`), so from these 5 commands a third principal cannot tell a
   message between others from no message, through an embedding or over HTTP. A client an
   embedding hands in without its inbox or its principal is held to the same: where the model
   cannot decide one of these 5 commands on a stored message without the attribute (an unmet
   obligation), the core answers `unknown-message` as well. `PostMessage`
   tells it only into an open inbox (decision 6).
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
   0.97.1, 0.97.3, 0.97.4 and 0.98.0 interpreted targets): `unknown-inbox`, `duplicate`,
   `duplicate-key`, `inbox-closed`, `id-taken`. The model declares `unknown-inbox` before
   `duplicate` (until ESS 0.97.4 it declared it after `duplicate-key`, and the interpreter
   answered `unknown-inbox` (`exists: false`) first all the same; ESS 0.98.0 refuses that
   declaration order: `ess specify validate` answers `conflicting_declaration`, ESS-COMMAND-004,
   on a copy of `ess/` with `unknown-inbox` moved below `duplicate-key`); a sender's repost goes to the inbox its
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
`request-without-shape`, `decision-without-options`, `unknown-option`); then the 3 external
refusals (`expired-on-arrival`, `signature-invalid`, `authority-invalid`), decided before any
row is read; then the row refusals in the order measured on the ESS 0.97.1, 0.97.3, 0.97.4 and 0.98.0 interpreted targets:
`unknown-inbox`, `duplicate`, `duplicate-key`, `inbox-closed`, `id-taken` (`unknown-inbox` and
`inbox-closed` never hold together, and `id-taken` holds with every `duplicate`). The suite
holds the pairs: each external refusal, `unknown-option` and `request-without-shape` against each of the 5 row
refusals, and each pair of
row refusals one post can make hold, in the authored scenarios
`a-bad-signature-is-refused-before-any-row-is-read`,
`an-unknown-option-is-refused-before-any-row-is-read`,
`a-refused-delegation-is-answered-before-any-row-is-read`,
`a-request-without-shape-is-refused-before-any-row-is-read` and
`row-refusals-of-a-post-answer-in-one-order`; `post-from-another-principal-refused` holds
`not-sender` before `duplicate`, `duplicate-key` and `id-taken`. No scenario holds `no-authority`,
`already-expired` or `decision-without-options` against a row refusal, or `not-sender` against
`unknown-inbox` and `inbox-closed`: a core that answers the row refusal first there passes the
suite (story:post-order-scenarios-hold-the-input-guards, a draft). The case
`every_other_input_refusal_answers_before_unknown_inbox` of
`crates/inbox-core/tests/core/precedence.rs` holds the first 3 against `unknown-inbox`. Synthesis
refuses a scenario that expects a row refusal where an input guard holds (`ESS-AUTHOR-041`); that
checks the scenario and no implementation. A conformance step forces one external outcome, and
synthesis refuses a step that expects an external refusal while an input guard holds, so 2 orders
are held by the cases of the core and not by the suite: `unknown-option` before the 2 external
refusals and the order between them (`crates/inbox-core/tests/core/precedence.rs`,
`crates/inbox-core/tests/adversary_w8_order.rs`), and the other input-guarded refusals before an
external one (`an_input_refusal_answers_before_an_external_refusal` of `precedence.rs`). The contract text of `PostMessage` in `generated/rust/inbox/PLAN.md` (line 119)
states another order: "Selection precedence: on commands with `when_related:`, check
`existing_instance` then `exists: false` before input-guarded refusals; … then accepting and
external branches in declaration order", which puts `unknown-inbox` before the input-guarded
refusals and the external refusals after every row refusal. The core and the service follow the
interpreter, which the suite runs against.

## What the specification cannot say (ESS 0.98.0)

Each item is a rule of the service that stage 2 implements and tests outside the suite, or a
limit of synthesis that an authored scenario makes up for. The items on `from` and on the 2
examples of `PostMessage` are filed in the `ess` repository (beyond10x/ess#804, beyond10x/ess
issue 809).

An item that names ESS 0.97.3 was measured on that release and not again on 0.97.4. On ESS
0.97.4 the specification as it stood on 0.97.3 gives the same `generated/` files and the same
suite, byte for byte, and the release notes of 0.97.4 say that suites, refusals and notes keep
their bytes. On ESS 0.98.0 the specification of 0.97.4 validates, writes the same `generated/`
files and the same suite, byte for byte, and synthesis prints the same 98 refusals with the same
text; nothing else was measured again on 0.98.0.

`ess/` holds no `UNMAPPED:` marker, and `task spec` fails with the file and line of one. A rule
the specification cannot express is an `external:` outcome that states its reason, and is listed
here. The `external:` outcomes on ESS 0.98.0: `signature-invalid` and `authority-invalid` of
`PostMessage` and of `Answer` (a guard cannot verify a signature or resolve a delegation of the
key file; a delegation spelled as a provided entity loses 51 generated scenarios), and
`shape-mismatch` of `Answer`; `expired-on-arrival` of `PostMessage` and of `Answer` and
`deadline-passed` of `Answer`, `Refuse`, `MarkDelivered` and `MarkRead`, the 6 that read the
service's clock (decision 3).
The clock part was measured as a guard on ESS 0.97.1 and again on 0.97.3, with the same
result. A `when_subject` outcome `deadline-passed` over `now` validates and generates (87 of 89
capabilities on 0.97.3), but synthesis decides it at the run's wall clock: it turns the
`expires` of the generated post into an offset from that clock while `created` stays a literal,
so the generated `already-expired` scenario fails on the interpreted target; the authored
scenarios whose literal `expires` lies before the wall clock fail too (10 on 0.97.3, 12 on
0.97.1); 8 refusals stay uncovered on 0.97.3; and an authored scenario cannot write an instant
relative to the clock (`ESS-AUTHOR-015` on both), so none could cover the guard. With
`expires <= now` in `already-expired` of `PostMessage` as well, synthesis refuses every
authored post (`ESS-AUTHOR-041`, 224 refusals on 0.97.3). As external outcomes the suite forces
them like the other external branches, the authored scenarios
`a-late-answer-leaves-the-decision-to-silence`, `a-late-refusal-leaves-the-request-to-expire`
and `an-expired-reply-leaves-the-request-open` cover the 3 of `Answer` and `Refuse`,
`a-late-read-leaves-the-note-to-expire` and `a-late-delivery-mark-leaves-the-request-to-expire`
the 2 of `MarkRead` and `MarkDelivered`, and the cases of `deadline.rs`, `deadline_mark.rs`,
`adversary_deadline.rs` and `adversary_deadline_2.rs` of `crates/inbox-core/tests` and
`crates/inbox-server/tests` drive the clock. A suite step that
expects a refusal finds no external branch taken, so the suite cannot tell whether an external
branch is asked before the state is: `a-second-expired-reply-to-an-answered-request-is-already-settled`
holds that order over HTTP only, where the service's own clock (started before the suite's
instants) has passed the second reply's `expires`, and the `when_subject` on the 5 branches
makes the generated crate check the state before it asks the port. The service answers each of the 6
with status 502, the status the generated contract gives an external outcome. Its reply-shape
part (at most so many lines, or a JSON text) was
not tried as a guard. Its option part has one: a `when_subject` outcome `option-mismatch` with
`exists` over the stored `decision.options`, which ESS 0.97.1 validated, synthesized and
interpreted (112 scenarios, all passed). On ESS 0.97.3 it validates, and synthesis writes the
same 112 scenarios and refuses the new outcome (`ESS-SYNTH-004`). With it,
`ess generate synthesize --target rust` keeps all of `Answer` an obligation on ESS 0.97.3 as on
0.97.1 (85 of 88 capabilities generated, 3 obligations, measured before `Collect` declared its
response) and drops the method `external` of the generated `Context`, through which the service
answers the external branches of `Answer`, so the option part stays in the external branch until
synthesis generates that guard.

- **A response field has no value source.** ESS 0.97.3 (as 0.97.1) declares the type of
  `Collect`'s `response` and `returns: true` on `collected`, and the generated OpenAPI document
  carries it (answered `200`, the record under `response`), but an outcome takes no `response:`
  key ("unknown field `response`") and `{subject: <field>}` is a source of `sets:` and
  `payload:` only. So the specification cannot say that the record is the deleted row's.
  Synthesis keeps the behaviour of `Collect` an obligation ("kept an obligation by a typed
  response"), and `inbox-core` answers it (`crates/inbox-core/src/obligations.rs`). The suite
  checks the response's shape on each `collected` step (`expect_direct_response`, no expected
  value); a value written in an authored scenario (`response: {answered_by: principal}`) failed
  on the ESS 0.97.1 interpreted target, which does not read the row. The values are held by the
  cases of `crates/inbox-core/tests/collect_record.rs` (one compares every member of the record
  with the `Outbox` row read just before) and `crates/inbox-cli/tests/collect_record.rs`, not by
  the suite.
- **`Body` cannot be in the response.** The invariant `value.utf8_bytes <= 65536` of `Body` on
  a type a response reaches refuses every authored scenario that runs `collected`
  (`ESS-AUTHOR-001`, "invariant `{utf8_bytes: value} <= 65536` on response type
  `inbox.messaging.Body` does not decide over its value alone"; `value.count` is admitted
  there), on ESS 0.97.3 as on 0.97.1; the Rust synthesis of 0.97.3 accepts it. So the
  response's `body` is `String`, and its `reply` is `CollectedReply`, the fields of `Reply` with
  the body as `String`; the JSON of both is the same as the `Outbox` row's. The smallest
  reproduction is a newtype with that invariant as the type of an input and of a response field
  of one `returns: true` command, and one authored scenario that runs it.

- **No per-caller filter on a view.** Who may read which rows is a rule of the core: a row of
  `Pending`, `Unanswered`, `Outbox`, `Settled`, `Thread` or `Replies` is answered to a client
  only when its `to` is the caller's `inbox` or its `from` is the caller's `principal`. A view
  `filter:` that reads the caller (`{any: [to == caller.inbox, from == caller.principal]}`) is
  refused by `ess specify validate` on ESS 0.97.3 (`ESS-VIEW-003`, `unobservable_fact`: "`caller`
  ... is not a declared observable root"), so the rule is `inbox_core::readable`: `handle` and
  `handle_with_events` apply it to every answer of a view, and the HTTP service calls the same
  function, so an embedding and `inbox serve` give one caller the same rows. It binds every
  caller but the `Sweeper`, whose `Due` and `Messages` stay whole; a row without `to` and `from`
  (`Inboxes`) is answered whole. `inbox_core::system` alone applies no read rule. The suite's
  reads name no caller, so the conformance runner reads each view as every caller of the
  scenario and compares the union; the cases of `crates/inbox-core/tests/read_rule.rs` and
  `crates/inbox-server/tests/read_rule.rs` hold the rule, not the suite.
- **No blocking read.** `inbox wait` is the `Pending` view held open until it has a row whose
  `expires` has not passed, or the wait ends (`docs/design/service.md`, "The wait").
- **Two outcomes cannot share one answer** without an `ess-http/1` binding. A second outcome
  named `unknown-message` is refused (`ESS-COMMAND-006`), `alike:` needs a binding this
  repository does not have, and the generated surface answers `foreign-message` `403` under its
  own name. The core renames it once (`inbox_core::as_unknown`): `handle` and
  `handle_with_events` answer it with the members of `unknown-message`, and the HTTP service
  calls the same function and gives the answer the status `404`; the model alone decides who is
  a party. The conformance run against the core goes through `inbox_core::handle_as_specified`,
  the entry without the rename, so it judges the model's `foreign-message` and its place in the
  order of refusals. Over HTTP the run reads `unknown-message` where the scenario expects
  `foreign-message`, so the cases of `crates/inbox-core/tests/foreign_unknown.rs` and
  `crates/inbox-server/tests/` witness the renamed answer, not the suite.
- **No step bounds the number of occurrences of an event.** A claim of an event holds it at
  least once. The cases of `crates/inbox-core/tests/expire_once.rs` compare the whole list of
  events `Expire` hands back, so a core that publishes one twice fails them and not the suite.
- **`instance: caller.<attribute>` is not available**, so `OpenInbox` takes the address as input
  and refuses one that is not the caller's.
- **Synthesis arranges no caller for a guard over a stored field** that was not set from the
  caller, no second row for a row-set guard over the entity a command creates (the `dedup_key`
  rule), and only notes once an input guard depends on `kind`. 51 generated scenarios are
  refused for these reasons and are covered by authored scenarios (`covers:`).
- **Synthesis ignores a newtype's invariant in a candidate for an `Optional` of it**
  (beyond10x/ess issue 809). On ESS 0.97.1 it then found no decision whose letters it accepts
  under the guard `unknown-option`, refused `posted` (`ESS-SYNTH-003`), and 54 generated
  scenarios left the suite. On ESS 0.97.3, without the examples below, it refuses
  `unknown-option` (`ESS-SYNTH-003`) and leaves no row with a decision in a view, and 16
  generated scenarios leave the suite: `task coverage` fails. The inputs `decision` and
  `default_on_silence` of `PostMessage` carry an `example:` each until the issue is fixed. On
  ESS 0.97.1 they made synthesis take about 4 times as long (CPU time of `task coverage`'s
  synthesis on a loaded host: 10 s on `8e9c3ab`, 43 s with this specification); on ESS 0.97.3
  the synthesis takes 20 s of CPU time with them and without them.
- **Synthesis does not find a `from` equal to the caller's `principal`** under the `not-sender`
  guard of `PostMessage`, so it would store no message and 49 generated scenarios would leave the
  suite (measured on ESS 0.96.0). On ESS 0.97.3, without the example below, synthesis refuses
  15 generated scenarios more (`ESS-SYNTH-013`, the invariant of `OptionLetter` at the
  `decision` and `default_on_silence` of 5 views) and `task coverage` fails. The input `from`
  of `PostMessage` carries `example: principal-262144`, the value ESS 0.97.3 synthesis gives
  the first caller's `principal` and not an address anybody holds; it goes when ESS writes
  those scenarios without it, which is the fix of beyond10x/ess#804. With it, a generated scenario posts as one caller
  only: 19 of them lost the second run under a second caller they had before the guard (also
  measured on ESS 0.96.0).
- **42 more refusals are `ESS-SYNTH-013`**: type-invariant checks at view fields for which
  synthesis arranges no row. `decision-answered-by-the-operator` covers 36 of them: it reads an
  answered decision, its reply and its option in `Settled`, `Replies`, `Thread`, `Outbox`,
  `Messages` and `Due`. `read-decision-waits-for-its-answer` covers the other 6: it reads a
  decision that was read and not yet answered in `Unanswered`. Until stage 2 there were 50: 8
  more for `reply` and `decided_option` of `Pending` and `Unanswered`, where no sequence of
  commands leaves a row with either field set. Those 2 fields, and `answered_by` and `refusal`,
  which are as absent there and caused no refusal, left those 2 views, and `refusal` left
  `Replies`, in one change of their wire rows against 0.1.0.
- **`task coverage` on ESS 0.98.0**: synthesis writes 119 scenarios (44 authored) and refuses 98,
  and authored scenarios cover all 98, so it exits 0; `task check` runs it. The suite it writes
  runs on the interpreted target as 119 passed and reports `conformance_status: passed`.
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
on ESS 0.98.0 writes 87 of 90 capabilities: the model types, the behaviour of 6 commands, the
query of 9 views, the storage and context ports, and an HTTP route table with JSON codecs. It
leaves 3 obligations, the behaviours of `PostMessage` (the `distinct` over the options in the
guard `unknown-option`, and the row-set guards `duplicate` and `duplicate-key`), `Collect` (a
typed response) and `Expire` (a narrowed read of `default_on_silence`). The generated storage ports (`get`, `put`,
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
