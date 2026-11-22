# State of the checks

For the people and agents who build the inbox. The numbers the gate prints, what the checks
do not cover, and the story of each open limit. The end-user pages under `website/docs` carry
none of this: they list features and limits ([features](https://beyond10x.github.io/inbox/docs/status), [limits](https://beyond10x.github.io/inbox/docs/limits)).

## On `main`

Numbers from 2026-11-22, ESS 0.97.3, Rust 1.99.1. Case counts are the passed cases of
`task test` (cargo-nextest) in one run of `task check`.

| Part | What it does | Number | Proof |
|---|---|---|---|
| The specification | The domain `inbox.messaging`: inboxes, messages, their lifecycle, every refusal | 2 entities, 9 commands, 9 views, 10 events, 15 errors, 2 actors; 42 authored scenarios | [`ess/`](../ess), `task spec` |
| The generated crate | Model types, command behaviours, view queries, storage and context ports, route table and JSON codecs, written by ESS from `ess/` | 87 of 90 capabilities generated; 3 obligations left to hand-written code (`PostMessage`, `Collect`, `Expire`) | [`generated/rust/inbox/`](../generated/rust/inbox), `task drift` |
| The core | The 3 obligations over a backend; one command at a time per store; the read rule of every view answer; the answer of `unknown-message` to a client that is no party of a message, or that lacks an attribute the model needs | 186 cases | [`crates/inbox-core/`](../crates/inbox-core) |
| The in-memory backend | Both storage ports in memory; part of the core crate (`MemoryBackend`) | in the 186 cases above, and in the test kit's | [`crates/inbox-core/`](../crates/inbox-core) |
| The verifier | Ed25519 signatures over RFC 8785 canonical JSON for envelopes, key file entries and requests; the key ring | 63 cases | [`crates/inbox-verify/`](../crates/inbox-verify), [`docs/design/signing.md`](../docs/design/signing.md) |
| The backend test kit | The checks every backend passes: the 2 storage ports and the command section | 29 cases | [`crates/inbox-backend-testkit/`](../crates/inbox-backend-testkit) |
| The HTTP listener | A library: request signatures, the core's read rule on each view answer and its rename of `foreign-message` on each command answer, the wait on `Pending`, the expiry task, the limits; loopback only | 78 cases, none ignored | [`crates/inbox-server/`](../crates/inbox-server), [`docs/design/service.md`](../docs/design/service.md) |
| The binary `inbox` | `inbox serve` starts the listener over the in-memory backend on a loopback address; the other commands are the client: signed requests, the outbox file and its epochs file, exit statuses 0 to 4 | 112 cases, none ignored | [`crates/inbox-cli/`](../crates/inbox-cli), [first use](https://beyond10x.github.io/inbox/docs/first-use) |
| Conformance, interpreted | The suite ESS synthesizes from `ess/`, run on ESS's interpreted target | 117 scenarios: 117 passed, 0 failed, 0 error, 0 unsupported | `task conform-interpreted` |
| The conformance gate | The same suite run against the core in process, the report written by ESS and held to the suite | 117 scenarios: 117 passed, 0 failed, 0 error, 0 unsupported; `conformance_status: passed` | [`crates/inbox-conformance/`](../crates/inbox-conformance), `task conform` |
| The service over HTTP | The same suite against a running `inbox serve`, a fresh one per scenario; every request and envelope signed, each answer held to the HTTP status of the generated contract | 117 scenarios: 32 passed, 0 failed, 0 error, 85 unsupported, each listed with its reasons; `conformance_status: failed` (see below) | [`unsupported.tsv`](../crates/inbox-conformance/unsupported.tsv), `task conform` |
| Acceptance | 3 checks against the built binary: a signed request and its reply between 2 keys; a read note and an expired request in no view; a repost after a restart, and a second repost that posts nothing | 3 passed | [`tests/acceptance/`](../tests/acceptance), `task acceptance` |
| The conformance runner | Reads the suite, runs it against both targets, holds each report | 86 cases, 1 ignored | [`crates/inbox-conformance/`](../crates/inbox-conformance) |
| Declared coverage | Every refusal of the specification is claimed by an authored scenario; `task check` fails on one that is not | 96 refused generated scenarios, 96 covered; the suite reports `conformance_status: passed` on the interpreted target | `task coverage` |
| Repository tasks | The drift check of the generated crate, the scenario checks, the animation writer, the words of the end-user pages, the path decision of the CI job, the test step and the build settings of `.cargo/config.toml`. The cases of the drift check read one synthesis of `ess/` per run of `task test`; the 2 that change `ess/` run their own | 80 cases | [`crates/inbox-xtask/`](../crates/inbox-xtask) |

## What wave 8 added

Wave 8 of stage 2 closes gaps in the checks that the adversary passes of wave 3 found, and
narrows one limit of the client.

- The service sends the header `Inbox-Epoch` on every answer, and `inbox repost` leaves alone
  a kept envelope that the epochs file records at the epoch the service names now. The limit
  that stays is on the [limits](https://beyond10x.github.io/inbox/docs/limits) page.
- 2 authored scenarios: a decision its recipient read is decided on silence at `expires`, and
  the row a decision decided on silence leaves keeps its other fields. Both are unsupported
  over HTTP, as steps of the Sweeper.
- 6 authored scenarios hold the order in which `PostMessage` refuses when 2 refusals apply:
  the signature, the option, the delegation and the reply shape are refused before any row is
  read, and the 5 row refusals answer in one order. All 6 run over HTTP. The suite has 38
  authored scenarios and 111 in all.
- Cases of the core: one occurrence of each event `Expire` hands back, the order of the 3
  external refusals of `PostMessage`, a read decision decided on silence, and the fields
  `Expire` leaves unchanged.

## Why 32 of 117 over HTTP

Against the core in process all 117 scenarios pass, and ESS reports `passed`. Against the
running service a scenario runs only where every step can be sent and witnessed over HTTP.
85 cannot: a caller whose principal and inbox differ (71 scenarios; over HTTP both
are the principal of the caller's key), a step as the Sweeper (39; only the service's own
expiry task is the Sweeper), a step as no actor (9; a request without a signature is
answered `401`), a claim on the stored `signature` (1; the run signs each envelope with
its own key), one delegation id for two pairs of sender and principal (2; an id names one
entry of the key file), a step that expects an outcome the service takes by its own clock (4;
`expired-on-arrival`, `deadline-passed`: the run starts the service's clock before every
instant of the suite and cannot move it past a deadline). A scenario can have several reasons. ESS reports a run with an
unsupported scenario as `failed`; `task conform` fails unless the unsupported scenarios and their
reasons are those of the list, and no scenario failed or errored.

## Open limits and their stories

The limits are stated on the limits page of the site. Their stories are in
`.engineering/planning/story/`.

- Security finding 1 (a principal that holds a message id learns whether it is live):
  `story:message-id-per-sender`, a draft. Required before principals of different operators
  share one service.
- Security finding 2 (a signed request is accepted again inside its 60 s window): no story is
  filed. Source: `docs/design/service.md`, "The request signature".
- `inbox repost` delivers a note once more that was read before a restart:
  `story:repost-does-not-redeliver-a-handled-note`.
- A `collect` whose answer is lost after the service removed the message loses the record; a
  second `collect` answers `unknown-message`, and the envelope stays in the outbox file until it
  expires (`inbox repost` posts it again after a restart). No story is filed: keeping a receipt
  of a collected message until its `expires` changes when a message is removed, a decision of
  its own (the "Not in this story" of `story:collect-returns-the-settled-record`).
- The values of `Collect`'s response are not in the suite: ESS 0.97.3 (as 0.97.1) has no source
  for a response field, and a value written in an authored scenario failed on the ESS 0.97.1
  interpreted target (`docs/design/inbox.md`, "What the specification cannot say"). The suite
  checks the shape on each `collected` step, so a record with a wrong value passes it; the cases
  named below hold the values.
- Loopback only, no TLS, no second host, no wake-up: `epic:two-hosts-and-wake`.
- One backend, in memory: `epic:jetstream-backend`.

## Security findings, closed in release 0.3.1

- **A message whose `expires` had passed still took an answer or a refusal until the next
  sweep** (`story:a-due-message-takes-no-answer`). `answer` and `refuse` of a message in
  `Sent`, `Delivered` or `Read` whose `expires` is at or before the service's clock answer
  `deadline-passed` (502, error `Expired`), change nothing and publish nothing; the expiry task
  then decides the message as before. The clock is read inside the command's section; no member
  of the request replaces it. `post` refuses an envelope whose `expires` is at or before that
  clock (`expired-on-arrival`, 502, error `AlreadyExpired`), and `answer` a reply whose own
  `expires` is, while the message is open (the same outcome and error; stored, the next sweep
  would remove the answer before its sender collected it); an answer to a message already
  answered or refused stays `already-settled`, whatever its own `expires`. Every comparison is
  at the clock's full precision, the sweep's and its `keep_until` included. Before, an answer
  that arrived between `expires` and the sweep replaced the option silence was due to take. The
  cases of `crates/inbox-core/tests/deadline.rs` (5), `crates/inbox-server/tests/deadline.rs`
  (4) and the adversary's `adversary_deadline.rs` of `inbox-core` (7), `inbox-server` (4) and
  `inbox-cli` (1) and `adversary_deadline_2.rs` of `inbox-core` (6) and `inbox-server` (5) run
  the late calls in every order around the sweep, at fractional deadlines and readings, and
  find the same stored message; they run in `task check`. The authored scenario
  `a-second-expired-reply-to-an-answered-request-is-already-settled` holds the order of
  `already-settled` before the reply's own deadline over HTTP.
- **A collect deleted a reply the sender never saw** (finding 3 of the source review of
  release 0.2.0, `story:collect-returns-the-settled-record`). `Collect` answers `collected` with
  the settled record it removes (a typed `response`, read by `inbox-core` from the row in the
  request that deletes it), and `inbox collect` prints that record and drops its outbox line
  only after it holds the answer; it reads no row before. Before, `inbox collect` read `Outbox`,
  then sent `Collect`, printed the row it had read and dropped the line: a reply or a refusal
  that arrived between the two requests was deleted unseen. The cases of
  `crates/inbox-core/tests/collect_record.rs` (the record of an answered, a refused and a
  silence-decided message; every member of the record against the `Outbox` row read just
  before; `not-settled`, a third principal and the recipient get none, the latter two also on a
  message that is not settled) and `crates/inbox-cli/tests/collect_record.rs` (a stand-in
  service that answers the read with the pending row and the `Collect` with the answered record;
  a `collect` before the answer keeps the outbox line; a third principal gets `unknown-message`)
  run in `task check`, with the adversary's cases (`adversary_collect_record.rs` and
  `adversary_collect_record_2.rs` of the core, the binary and the runner: the reply verifies
  from the record alone, `--json` keeps the members 0.2.0 printed under `message`, an answer
  without the record of the message or outside the declared record of `Collect` exits 3 and
  keeps the line, every mix of caller attributes gets the outcome the guards order, the record
  is the `Outbox` row for every settlement). The text line of a decision taken on silence has
  its case. The conformance runner checks the response's shape on each `collected` step of the
  suite (`expect_direct_response`, which a member stated twice fails; 6 cases in
  `crates/inbox-conformance/tests/runner.rs`). The part that stays: an answer lost after the
  delete loses the record (above, "Open limits and their stories").
- **The client printed the control sequences of another principal** (found by the second
  adversary pass on `story:collect-returns-the-settled-record`, finding 1; in release 0.2.0,
  not among the findings of its source review;
  `story:the-client-prints-no-control-sequence-of-another-principal`). Every write of the
  client to standard output and standard error goes through `crates/inbox-cli/src/output.rs`:
  text shows each control character but the line end (C0 with the tab and a carriage return
  before a line end, DEL, C1) as its escape `\u{1b}`, and a JSON document has DEL and C1 as
  `\u` escapes (the generated writer escapes C0 only). Before, `collect`, `pending`, `wait`,
  `outbox`, `thread` and `replies` printed a body, a reply, a refusal reason or an option as
  another principal wrote it, so an erase of the screen, an OSC window title, a carriage
  return or a C1 control sequence introducer reached the terminal of whoever ran them, and a
  line on standard error did the same with the service's `refused` text. The 6 cases of
  `crates/inbox-cli/tests/control_sequences.rs` (the adversary's case for `collect` of an
  answered and a refused message; `pending` with a body and an option; `outbox` and `thread`
  of both principals; a `refused` text and a 302 body on standard error; `--json` of `outbox`
  and `collect` with no raw control character and the values as sent; a body without control
  characters printed byte for byte as before) and 3 unit cases of `output.rs` (the escape, a
  document read back to the same value, and no other file of `src/` that writes to standard
  output or standard error) run in `task check`, and so do the 7 cases of the adversary pass
  (`crates/inbox-cli/tests/adversary_control_sequences.rs`). With the escape taken out of the text
  printer the cases of `collect`, `pending`, `outbox` and `thread` fail; out of the line on
  standard error, its case; out of the JSON printer, the `--json` case; with a carriage return
  before a line end dropped again, the `outbox` case. The stored message, the signature check
  and the service are unchanged. The part that stays: a line end in such a value is printed
  as a line end, so a value can still start a line that looks like one of the client's own.
- **The core gave a client the rows of messages between others**
  (`story:the-core-applies-the-read-rule`). The read rule was in the HTTP listener only:
  `inbox_core::handle`, the entry for an embedding with another caller per request, ran
  `Outbox`, `Settled`, `Pending`, `Unanswered`, `Thread` and `Replies` by the parameter the
  caller sent and compared no row with the caller. The rule is now `inbox_core::readable`,
  applied by `handle` and `handle_with_events` and called by the listener, which holds no copy
  of it; ESS 0.97.3 refuses a view filter over the caller (`ESS-VIEW-003`). The case
  `a_third_principal_reads_no_row_of_a_message_between_two_others_through_handle` in
  `crates/inbox-core/tests/read_rule.rs` and
  `handle_and_the_service_answer_each_party_and_a_third_the_same_rows` in
  `crates/inbox-server/tests/read_rule.rs` run in `task check`.
- **The core told a third principal that a message id exists**
  (`story:the-core-answers-a-foreign-message-as-unknown`). Only the HTTP listener renamed the
  model's `foreign-message`: through `inbox_core::handle` a client that is no party of a
  message got `foreign-message` from `MarkDelivered`, `MarkRead`, `Answer`, `Refuse` and
  `Collect`, and `unknown-message` for an id no message carries. The rename is now
  `inbox_core::as_unknown`, applied by `handle` and `handle_with_events` and called by the
  listener, which holds no copy of it; the model keeps `foreign-message`, and the conformance
  run against the core judges it through `inbox_core::handle_as_specified`, the entry without
  the rename. A client an embedding hands in without its inbox or its principal got an unmet
  obligation for a message between others and `unknown-message` for no message; `handle` and
  `handle_with_events` answer that refusal of the 5 commands as `unknown-message` too. The cases
  `each_command_answers_a_client_that_is_no_party_as_a_missing_id_in_every_state`,
  `the_parties_get_from_handle_what_the_model_answers`,
  `a_client_whose_context_cannot_read_an_attribute_gets_the_answer_of_no_message` and
  `a_sender_without_an_inbox_gets_the_models_answer_where_it_decides_and_no_message_elsewhere`
  in `crates/inbox-core/tests/foreign_unknown.rs`, and
  `adv_a_client_with_one_attribute_absent_cannot_tell_a_message_between_others_from_none` in
  `crates/inbox-core/tests/adversary_foreign_unknown.rs`, run in `task check`.

## Security findings, closed in release 0.2.0

Closed in waves 6 and 7 of stage 2 and in release 0.2.0. Release 0.1.0 holds the listener
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

## What the checks do not cover

- **The run over HTTP witnesses 32 of the 117 scenarios.** All 117 pass against the core in
  process. Against a running `inbox serve`, 85 are unsupported and listed with their
  reasons in
  [`unsupported.tsv`](../crates/inbox-conformance/unsupported.tsv)
  (the reasons are above). ESS reports that run as `failed`, and the
  gate holds it to the list.
- **The run over HTTP and the 3 acceptance checks do not exercise every rule of the
  listener.** Every request they send is validly signed, with keys and delegations that
  do not lapse, and the run reads a view as every key and joins the rows (the run against the
  core reads as every caller of the scenario and joins the rows too, so neither run witnesses
  the read rule; `crates/inbox-core/tests/read_rule.rs` and
  `crates/inbox-server/tests/read_rule.rs` do). A service
  without the read rule, without the check of the request signature or its 60 s window,
  without the lapse of a key or a delegation, without the binding of a key to `from` or
  of a delegation to its pair, or one that serves `Messages` and `Due` to a client,
  passes both (8 of the 12 changed services the adversary built). The tests of
  `crates/inbox-server` and `crates/inbox-verify` fail on each of the 8, and
  `task check` runs them.
- Rules the specification cannot state on ESS 0.97.3 (who may read which rows, the blocking
  read, and others listed in
  [`docs/design/inbox.md`](../docs/design/inbox.md))
  are implemented by the service and tested outside the suite.
- Gaps in the suite found by the adversary passes, each a draft story:
  - `story:post-order-scenarios-hold-the-input-guards`: the suite holds the signature, the
    option, the delegation and the reply shape of a post before every row refusal, and the 5
    row refusals in one order. No scenario holds `no-authority`, `already-expired` or
    `decision-without-options` before a row refusal, or `not-sender` before `unknown-inbox` and
    `inbox-closed`; a core that answers the row refusal first there passes `task conform`
    (cases of the core hold 3 of these pairs). A `duplicate` rule that also compares
    `on_behalf_of` passes it too.
  - `story:silence-scenarios-claim-the-whole-message`: the scenarios on a decision decided on
    silence claim 10 members of the stored message, from one start state. A core that changes
    another member, or a claimed one only for a delivered or read decision, passes
    `task conform`; cases of the core hold both. No scenario and no case holds the other
    messages of the thread.
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
    adversary case that needs the exact comparison is ignored until the issue is closed. On
    ESS 0.97.3 the rule is still needed: with an exact comparison the run against the core
    fails `PostMessage/outcome/posted` on a row of `Due`.
  - [beyond10x/ess#804](https://github.com/beyond10x/ess/issues/804): with the guard `not-sender` in the specification,
    synthesis writes no stored message under it, and 49 generated scenarios leave the suite
    (measured on ESS 0.96.0); on ESS 0.97.3, without the example, 15 generated scenarios are
    refused and `task coverage` fails. Until it is fixed the input `from` of `PostMessage`
    carries the example value `principal-262144`, the value synthesis gives its first caller
    and not an address anybody holds. It shows in the generated OpenAPI documents and the JSON Schema of
    `PostMessage` (3 files) and changes no decoder. With it, 19 generated scenarios lost a
    second run under a second caller (also measured on ESS 0.96.0).
  - beyond10x/ess issue 809 ([link](https://github.com/beyond10x/ess/issues/809)): synthesis
    draws a candidate for an `Optional` of a newtype without the newtype's invariant. Under the
    guard `unknown-option` it then refused `posted` on ESS 0.97.1 and 54 generated scenarios
    left the suite; on ESS 0.97.3 it refuses `unknown-option` and 16 leave, and
    `task coverage` fails. Until it is fixed the inputs `decision` and `default_on_silence` of
    `PostMessage` carry an example each. On ESS 0.97.1 they made synthesis take about 4 times
    as long as before the guard; on ESS 0.97.3 synthesis takes as long with them as without.
  - Not filed: on a type a command's response reaches, the invariant `value.utf8_bytes <= 65536`
    of `Body` refuses every authored scenario that runs the returning outcome (`ESS-AUTHOR-001`,
    "does not decide over its value alone", on ESS 0.97.1 and 0.97.3), where `value.count` is
    admitted. `Collect`'s response carries the bodies as `String` (`CollectedReply` for the
    reply) until it is fixed.
