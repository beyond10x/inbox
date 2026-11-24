---
format: aep.planning-md/3
id: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
kind: story
status: implemented
title: An agent with a delegation acts for the inbox of its principal
summary: 'A live delegation lets its agent read, mark and answer the inbox of its on_behalf_of: the operator''s answer B of 2026-11-23 on the bridge''s key.'
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-asks-a-decision-once
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: crates/inbox-cli/src/client.rs
- confidence: inferred
  path: crates/inbox-cli/src/commands.rs
- confidence: inferred
  path: crates/inbox-cli/src/keys.rs
- confidence: inferred
  path: crates/inbox-cli/src/main.rs
- confidence: inferred
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests
- confidence: cited
  path: crates/inbox-core/src/context.rs
- confidence: cited
  path: crates/inbox-server/src/auth.rs
- confidence: cited
  path: crates/inbox-server/src/service.rs
- confidence: inferred
  path: crates/inbox-server/tests
- confidence: inferred
  path: crates/inbox-verify/src
- confidence: cited
  path: crates/inbox-verify/tests
- confidence: cited
  path: docs/design/bridge.md
- confidence: cited
  path: docs/design/inbox.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: docs/design/signing.md
- confidence: cited
  path: ess/domains/messaging.yaml
- confidence: cited
  path: generated
- confidence: inferred
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:44Z", actor: "agent:inbox", revision: 15, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-11-24T06:50:24Z", actor: "agent:inbox", revision: 16, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "active", to: "implemented", at: "2026-11-24T08:24:27Z", actor: "agent:inbox", revision: 22, decided_on: {"recorded":{"test_result":1,"review_outcome":15}}}
---
## Outcome

An agent that holds a live delegation from a person can act as the owner of that person's inbox:
read its `Pending` view, mark its messages delivered and read, and answer them with a reply
envelope signed with the agent's own key, `from` the agent, `on_behalf_of` the person and
`authority` the delegation id. The service accepts that reply and records `answered_by:
principal`. Today the service makes the inbox of a caller the same text as its principal, so a
key of the chat bridge reads no row of the person's inbox and its answer is refused.

This story exists for option B of Open 1, the reading of the epic's outcome ("a reply envelope
signed with the bridge's key on behalf of the person, under a delegation"). It is a change of
architecture, which the folder rule "A default proceeds" leaves to the operator: the operator
answered B on 2026-11-23 (approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner),
and decision-blocker:delegation-grants-inbox-ownership, which held this story, is cleared.

## Acceptance

An `Answer` sent by an agent that holds a live delegation for the person, `from` the agent,
`on_behalf_of` the person and `authority` the delegation id, in a request that names the
person's inbox as the inbox it acts for (Open 2), moves a decision of that inbox from `Read` to
`Answered` with `answered_by: principal`, where before this story the same answer is refused
`unknown-message` (state transition: `Read` to `Answered` by a caller that is not the key of the
person).

Checks, each of which can fail:

1. A `Pending` read of the agent that names the person's inbox is answered the rows of that
   inbox: after 2 posts to it, `rows` holds those 2 message ids; today it holds none.
2. A `mark-delivered` of the agent that names the person's inbox moves a message of it from
   `Sent` to `Delivered` (outcome `delivered`); today it is answered `unknown-message`.
3. A `read` of the agent that names the person's inbox moves a request of it from `Delivered`
   to `Read` (outcome `read`), and removes a note (outcome `handled`); today both are answered
   `unknown-message`.
4. Without a delegation, with a lapsed one (`valid_until` at or before the clock reading, the
   exclusive bound of `docs/design/signing.md`, "Instants"), and with a delegation of another
   pair (another agent, or another person), the same 4 requests are answered as Open 3 says
   (default: `401 {"refused":"unauthenticated"}`), and the message stays in its state.
5. A request that names no inbox to act for is answered as today: the caller's inbox is the
   text of its principal (`crates/inbox-server/src/service.rs:87`), and every case of
   `crates/inbox-server/tests` and `crates/inbox-cli/tests` that passed before passes unchanged.
6. A reply of the agent under the delegation whose `from` is not the agent is refused
   `not-sender`, one whose `on_behalf_of` is not the person `authority-invalid`, and one without
   `authority` `no-authority`; the message stays in `Read`.
7. The design states the change: decisions 4 and 5 of `docs/design/inbox.md` (a delegation
   covers more than the messages its agent sends; a caller's `principal` and `inbox` differ under
   a delegation), "The caller" of `docs/design/service.md`, and, under Open 2 A, "Request" of
   `docs/design/signing.md` with the sixth member.
8. The run of the conformance suite over HTTP is unchanged: its wire target sends no header of
   acting for another inbox, so `crates/inbox-conformance/unsupported.tsv` still has 87 lines
   (`wc -l`), 71 of them with the reason "a caller's principal and inbox differ" (`grep -c`),
   both read on `main` at 2026-11-23, and `task conform` passes. Making those 71 supported is
   not this story.
9. The cases run against the listener of `crates/inbox-server` and the built `inbox` on
   loopback, with keys and a key file made in the case. No case over 60 s, the test step within
   120 s, no new Python; a build or a test runs through `slot`, and a pool job ends after
   10 minutes unless it names its time with `slot --timeout` (30 minutes at most).

## Context

The model has the two attributes apart: the actor `inbox.messaging.Client` carries `principal`
and `inbox`, and its summary says "In version 1 the two are the same text"
(`ess/domains/messaging.yaml`, lines 252-258). The service makes them the same text:
`docs/design/service.md`, "The caller" ("the address with the same text: decision 5"), and in
code `let inbox = Address(principal.0.clone());` (`crates/inbox-server/src/service.rs:87`).

So the reply the epic names cannot be made today. `Answer` refuses `not-sender` when `from` is
not the caller's principal, and `not-recipient` (answered as `unknown-message` to a third
principal) when the message's `to` is not the caller's inbox (`ess/domains/messaging.yaml`,
lines 648-680). With a key of its own the bridge's principal is `from`, its inbox is its own
address, and the message is in the person's inbox: refused. With a key whose entry names the
person as principal, `from` is the person and no delegation is involved.

A delegation entry is in the key file: `inbox keyfile add-delegation --delegation-id --agent
--on-behalf-of`, with `--valid-until` "30 days from now by default"
(`crates/inbox-cli/src/main.rs`, `KeyfileArgs` and `KeyfileCommand::AddDelegation`). The
service does not reload the key file while it runs (`website/docs/limits.md`, "The service";
`docs/design/service.md`, "Left out of stage 2"), so a new delegation reaches the service only
at its next start.

epic:authority-in-daily-use (stage 6) is about acting on a relayed decision because its
delegation verifies; it is out of this stage, and this story acts on nothing a delegation
relays.

## Domain relations

- Client to Inbox: the caller carries `inbox` beside `principal` (`ess/domains/messaging.yaml`,
  actor `inbox.messaging.Client`, attributes `principal` and `inbox`, lines 252-258). Inferable
  from the ess/1 document. That the two are one text is inferred from
  `crates/inbox-server/src/service.rs:87` and `docs/design/service.md`, "The caller"; no ess/1
  document declares the equality.
- DelegationEntry: `agent` may speak for `on_behalf_of` until `valid_until`, "every message the
  agent sends for that principal" (same file, struct `inbox.messaging.DelegationEntry`, lines
  130-139). A struct of the key file, not an entity: the service stores none (decision 4 of
  `docs/design/inbox.md`). Inferable from the ess/1 document.
- The checks of `Answer` that bind `from`, `on_behalf_of`, `authority` and `to` (same file,
  command `inbox.messaging.Answer`, outcomes `not-sender`, `no-authority`, `authority-invalid`,
  `foreign-message`, `not-recipient`, lines 632-680), and `answered_by: principal`, "the reply's
  `on_behalf_of` answered" (enum `inbox.messaging.AnsweredBy`, lines 149-153; outcome `answered`,
  line 732). Inferable from the ess/1 document.
- Delegation to the inbox it lets its agent act for: no source declares it. Whether a delegation
  grants ownership of the inbox of its `on_behalf_of` at all was Open 1, settled B: it does.
  How many inboxes one agent may act for, and which one a request means, is Open 2.
- Inbox to owner after an open by the agent: `OpenInbox` sets `owner: {caller: principal}`
  (same file, outcome `opened`, line 348), so the owner becomes the agent, and `CloseInbox`
  refuses `not-owner` when `owner != caller.principal` (line 374): the person's own key could
  then not close the inbox. Inferable from the ess/1 document as it stands; whether that is
  wanted is Open 4.

## Open

1. Settled. The key of the bridge and the delegation (a change of architecture): the operator
   answered B on 2026-11-23 (approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner;
   decision-blocker:delegation-grants-inbox-ownership is cleared). The options were: (A) no
   delegation: the bridge holds a key whose entry names the person as principal, made with
   `inbox key new` and `inbox keyfile add-key`; its reply has `from` and `on_behalf_of` the
   person and no `authority`. No change of the service; the epic's acceptance holds
   (`answered_by: principal`), its outcome does not: a reply of the bridge cannot be told from
   one the person signed, and the bridge holds a key that speaks as the person in every command.
   This story would have been rejected. (B) this story: a delegation lets its agent act as the
   owner of the inbox of its `on_behalf_of`; the bridge holds a key of its own. Costs: the
   service, the design decisions 4 and 5, and the signed request (Open 2). (C) the bridge reads
   and marks with a key of the person (A) and signs only the reply with its own key under a
   delegation: the model's `not-sender` of `Answer` refuses that, so
   `ess/domains/messaging.yaml`, `generated/` and the suite change. Answer: B, the operator's
   answer of 2026-11-23.
2. How a request names the inbox it acts for (B only). (A) a sixth member of the signed request
   object, sent in a header `Inbox-On-Behalf-Of`; a request without it is signed and answered as
   today; (B) a query parameter: it reaches the views, but a command's input is the generated
   body, so the commands would need a change of `generated/`; (C) none: the service takes the one
   live delegation of the key's principal (two delegations make it ambiguous). Default: A.
3. What answers a request that names an inbox no live delegation lets it act for. (A) `401
   {"refused":"unauthenticated"}`, as every failure of the request signature, so a caller learns
   nothing about the key file (`docs/design/service.md`, "The request signature"); (B) a `403`
   of its own that says no delegation covers it. Default: A.
4. Who owns the inbox when the agent opens it. (A) as the model says: the agent, so only a
   caller under the same delegation, or the agent, closes it; (B) the person: `OpenInbox` sets
   `owner` from the caller's inbox, a change of `ess/domains/messaging.yaml`. Default: A.
5. Who makes the delegation entry, and for how long. (A) the operator, with `inbox keyfile
   add-delegation` and the default `valid_until` of 30 days, then a restart of the service (the
   key file is not reloaded); a renewal is a new entry under a new id (`docs/design/signing.md`,
   "An id is never reused"); (B) a longer default for a bridge; (C) a reload of the key file
   while the service runs, as a story of its own. Default: A. What the bridge does when its key or
   its delegation has lapsed (one line, the same for both causes, naming both ids) is Open 4 of
   story:the-chat-bridge-runs-with-its-own-key-under-a-delegation.

## Scope

For Open 1 B (settled) and the default of Open 2 (A). Order: after
story:the-chat-bridge-asks-a-decision-once (edge `depends_on`). The reason is the shared files:
this story edits `crates/inbox-cli/src/main.rs`, `src/reference.rs`,
`generated/cli/inbox.json`, `AGENTS.md` and `website/docs/status.md`, which
story:the-chat-bridge-posts-a-pending-message, story:the-chat-bridge-marks-a-message-read-on-a-reaction
and story:the-chat-bridge-asks-a-decision-once edit too. So the first 3 bridge stories did not
wait for the operator's answer, and this one lands after them. `generated/cli/inbox.json` is
not merged: the story that lands later regenerates it with `task cli-reference`, which the
case `the_cli_reference_document_is_current` holds (`AGENTS.md`, the rows of
`generated/cli/inbox.json` and of `task cli-reference`).
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation comes after it (edge
`depends_on`).

Code:

- `crates/inbox-server/src/service.rs` (the caller's inbox from the delegation, line 87): cited
- `crates/inbox-server/src/auth.rs` (the header in the signed request): cited
- `crates/inbox-verify/src/` (the request object and the check of the delegation at the
  request): inferred
- `crates/inbox-cli/src/client.rs` (the client sends the header when it acts for another
  inbox) and `crates/inbox-cli/src/main.rs` (the flag that names that inbox, a flag of every
  client command: whether `inbox bridge` takes this one flag or a `--for` of its own is Open 3 of
  story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, default A: this one flag, named
  `--for <address>`): inferred
- `crates/inbox-cli/src/commands.rs` and `crates/inbox-cli/src/keys.rs`, when the inbox acted
  for has to travel with `Client` or `Caller`: inferred
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json` (regenerated with
  `task cli-reference`): inferred
- `crates/inbox-server/tests/` (a new case file and its `mod` line in `tests/main.rs`): inferred
- `crates/inbox-conformance/unsupported.tsv`: not edited (check 8): cited
- `ess/domains/messaging.yaml`: 3 summary sentences this story makes false and nothing else
  under `ess/` (the operator's answer A of 2026-11-24 to the question whether the story may
  correct them): line 133 (a delegation "covers every message the agent sends"), line 190
  (an inbox "belongs to the principal the address names") and line 255 (a caller's
  `principal` and `inbox` "are the same text"). No declaration changes; the compatibility
  check of the gate shows no breaking item: cited
- `generated/`: regenerated through the tasks of the repository after that edit, never by
  hand: cited

Documents:

- `docs/design/inbox.md`: decisions 4 and 5: cited
- `docs/design/service.md`: "The caller", "The request signature": cited
- `docs/design/signing.md`: "Request", "A delegation": cited
- `AGENTS.md`: the rows of `crates/inbox-server/`, `crates/inbox-verify/` and
  `crates/inbox-conformance/`: cited
- `website/docs/status.md`: the row of delegations in "Signing": cited
- `crates/inbox-cli/tests/` (a new case file `acting_for.rs` and its `mod` line in
  `tests/main.rs`): cited
- `crates/inbox-verify/tests/` (the cases of the sixth signed member and of `acts_for`):
  cited
- `crates/inbox-core/src/context.rs`: the doc comment of `CallerAttributes.inbox`, which
  this story makes false; no code: cited
- `docs/design/bridge.md`: one sentence this story makes false ("the only key that may
  read that `Pending`"): cited
- `website/docs/limits.md`: what an agent under a delegation cannot do yet and what it
  can do in the place of its person: cited
- `CHANGELOG.md`: one bullet under `## [Unreleased]` (a change a user of the service sees):
  cited

## Decisions

- The items 2 to 5 of Open are answered at their defaults (the operator accepted the 7 stories
  of the stage with every open item at its default, 2026-11-23): the inbox a request acts for
  is the sixth member `on_behalf_of` of the signed request object, sent in the header
  `Inbox-On-Behalf-Of` (2 A); a request for an inbox no live delegation of the pair covers is
  answered `401 {"refused":"unauthenticated"}`, byte for byte the answer to a bad signature
  (3 A); an inbox the agent opens for the person is the agent's (4 A); the operator makes the
  delegation entry with `inbox keyfile add-delegation`, and a new or removed entry takes
  effect at the next start of the service (5 A).
- The rule of the header sits in `inbox-server` and `inbox-verify`; `inbox-core` and the
  generated crate keep their behaviour, so the run of the suite in process is untouched. In
  `inbox-core` only the doc comment of `CallerAttributes.inbox` changed, which the story made
  false.
- Under the header a reply speaks for the person of the inbox: one whose `on_behalf_of` is any
  other principal, the agent included, is `authority-invalid` before its `authority` is looked
  up, a rule of the service. The options were: (A) that rule; (B) leave it to the verifier,
  which accepts any live delegation of the agent, so an agent with 2 delegations answers in
  the inbox of one person for the other. A is taken: check 6 of the story has no condition.
- 3 summary sentences of `ess/domains/messaging.yaml` were false with this story (a delegation
  "covers every message the agent sends", an inbox "belongs to the principal the address
  names", a caller's `principal` and `inbox` "are the same text") and are corrected, on the
  operator's answer of 2026-11-24. No declaration changed; `generated/` was written anew by
  the tasks of the repository (49 files, digests and the 3 texts). `ess verify diff
  --compatibility --fail-on breaking-or-unknown` from the specification before to the one
  after: `compatibility: 0 breaking, 0 unknown, 3 compatible`. The gate has no compatibility
  check; the command was run by hand.
- Finding 1 of the security review (an agent whose delegation lapsed still closed, without
  the header, the inbox it had opened for the person). The options were: (A) a rule of the
  service: `close-inbox` for an inbox that exists and is not the caller's own is answered the
  model's `not-owner`; (B) the person owns an inbox an agent opens for it, a change of the
  model and of Open 4, which was answered A; (C) no rule, named as a limit. A is taken. The
  generated `CloseInbox` asks its context no branch, so the rule sits in the listener
  (`closes_another_inbox`, inside the exclusive section, before the dispatch), not in
  `RequestContext`. Its consequences are held by cases and written in the design: under the
  header the agent does not close its own inbox; after the lapse nobody closes the inbox the
  agent opened for the person until the service starts again, and the person still reads it.
- Finding 2 of the security review (under the header an agent answers a decision it sent to
  the person itself, and the record says the person answered). The options were: (A) no rule;
  the design and the limits page say it, and a sender that must not answer its own question
  holds no delegation of the person it asks; (B) a rule of the service that refuses `answer`,
  `refuse` and `read` under the header for a message whose `from` is the signer; (C) a scope
  on a delegation entry, a change of the key file's format. A is taken, on the default after
  15 minutes without an answer of the operator's session (2026-11-24); B stays possible
  without a change of a format.
- The client: `inbox <command> --for <address>` on `post`, `pending`, `wait`, `read`,
  `answer`, `refuse`, `outbox`, `collect`, `thread`, `replies` and `repost`. `inbox answer
  --for` takes the `--for` address as `on_behalf_of` unless `--on-behalf-of` is given;
  `--authority` stays the caller's to give. An inbox no live delegation covers prints what an
  unknown key prints, with the same exit code.
- The value of `--for`: an empty one, one whose first or last character is white space, and
  one that holds a control character are wrong usage (exit 2) before any request and any
  file; nothing else of the value is checked on the client, and it is sent as given. The
  first form of the check refused only a space or a tab at an end. The adversary showed that
  a line break in the value put a second header line on the wire, that a control character
  was sent, and that the reader of the service trims every Unicode white space. The options
  for a control character were: (A) the client refuses every one; (B) it sends the value as
  given and the service answers `401`. A is taken: the client puts no bytes on the wire that
  differ from the bytes it signs. The function of the client that writes a request head
  makes no head with a line break in any of its parts, whatever its caller passes. The
  service is unchanged: it honoured none of these values.
- A key id with a line break. `inbox key new` writes one with exit 0, as before this story.
  Since the head of a request holds no line break, every command of the client with such a
  key file ends with exit 2 and a line that names no key file; before, the client sent the
  head with the line break in it. The options were: (A) a fourth round here: `inbox key new`
  refuses such a key id and a key file that holds one ends as a defect of the file (exit 4);
  (B) file it as a draft and change nothing here. B is taken: the key file is the caller's
  own, no boundary is crossed, and the case of the second adversary pass that holds the order
  of head and connection reaches the head through such a key id, so the check needs that case
  written anew. It is the draft
  story:a-key-id-with-a-control-character-is-refused-where-it-is-made.
- `inbox wake`, `inbox bridge`, `inbox open` and `inbox close` do not take `--for` in this
  story (exit 2, one line, nothing sent). Their state on the host (the lock of a wake, the
  record of the chat posts, the record of the opened inboxes) is keyed by the caller's
  address. The options for `open` and `close` were: (A) refuse the flag here, and key that
  state by the inbox acted for in story:the-chat-bridge-runs-with-its-own-key-under-a-delegation,
  which needs it for `bridge`; (B) key the record of the opened inboxes anew in this story. A
  is taken. The service takes `open-inbox` and `close-inbox` under the header; the cases of
  the listener hold both.
- Under `--for` the reconnect of a command opens no inbox: the record on the host is of the
  caller's own inbox, and under the header the service answers that open `not-owner`. The
  next command without `--for` opens the caller's own inbox again. A repost under `--for`
  carries the header.
- Check 2 of the story (the delivery mark for the person) is held at the listener
  (`delegation::a_delivery_mark_for_the_person_moves_its_message_to_delivered`). No command of
  the client that takes `--for` sends a delivery mark: only `wake` and `bridge` send one. The
  case against the built `inbox` reads, answers and lists for the person.
- `website/docs/first-use.md` does not tell how to make a delegation entry and is unchanged:
  the page gets the delegation with the story of the real run, as it gets the bridge.
- Read from the code and the model, not run: that the state after a lapse lasts until the
  service starts again (`config.rs`: the key file is read at the start); that the person's
  reopen of a closed inbox the agent owns keeps the agent as owner.
- This story was built beside the story before it (the bridge asks once), on that story's
  local branch, and holds its commit of `main` merged in. One file conflicted, the row of the
  test step in `AGENTS.md`; the row of `main` was taken and written anew by this story's
  record.

## Measured

- Red first, round 1 (the service): the 31 cases of `delegation` and `acting` against stubs
  with the behaviour before the story (the member ignored, no delegation admitted): `31 tests
  run: 3 passed, 28 failed` (check 1 `left: []`, the 2 ids expected; check 2 `left:
  "unknown-message" right: "delivered"`; the acceptance `left: "unknown-message" right:
  "answered"`). The 3 that passed state that nothing changes without the header; 2 of them
  are red under the mutants below. Round 1b (the close rule): `33 tests run: 29 passed, 4
  failed`, the reviewer's case with `left: ("closed", Some(Closed)) right: ("not-owner",
  Some(Open))`. Round 2 (the client): the 7 cases of `acting_for` against a flag that is
  parsed and ignored: `7 tests run: 0 passed, 6 failed, 1 timed out`; the 2 cases that hung on
  a listener that never answered now use a stand-in that answers at once and fail within a
  second.
- Mutants, each red in a named case. Round 1, 16 (the header honoured without a delegation;
  `<=` at `valid_until`; the delegation of another agent; of another person; the member left
  out of the signed bytes; the header twice read as its first value, and as its last; an
  empty header read as absent; `principal` set to the person; the check skipped for
  `Pending`, and for `mark-delivered`; the reply's `authority` not asked; the bytes of a
  request without the header changed; the reply rule skipped; the reply rule applied without
  the header; `inbox` not taken from the header). Round 1b, 3 (the close rule skipped; applied
  under the header too; applied before `unknown-inbox`). Round 2, 10 (the header sent and the
  member not signed; the member signed and the header not sent; `--for` dropped for the
  views, for the marks, for `answer`, for `post`; the default `on_behalf_of` under `--for`
  dropped; an empty value sent; `wake --for` accepted; the reconnect opening under `--for`).
  One case, `delegation::a_request_without_the_header_is_answered_as_before`, is red under
  none of them; the fixed bytes of a request without the header are held by
  `cases::the_signed_bytes_of_each_kind_are_its_context_then_a_jcs_object`.
- Security review pass 1 on round 1 (review-result:security-delegation-acts-pass-1), in a
  tree of its own on the build node: 4 cases added, 1 red, 2 findings (an agent whose
  delegation lapsed still closes the inbox it opened for the person; under the header an
  agent answers a decision it sent itself, and the design did not say so). The first is fixed
  by a rule of the service in round 1b, the second by the sentences of the design and the
  limits page (Decisions).
- Adversary pass 1 on round 2 (review-result:adversary-delegation-acts-pass-1): 5 cases
  added, 3 red, 4 findings, all in the client's check of the value of `--for` (a line break
  in the value is sent as a second header line; a control character is sent; the reader of
  the service trims every Unicode white space and the client refused only a space or a tab;
  a help sentence of `inbox answer` was out of date). No request with such a value was
  honoured: the service answered `401`. 8 mutants of the service and the client, each red in
  a case of the story. All 4 fixed in round 3: `5 tests run: 2 passed, 3 failed` before, `13
  tests run: 13 passed` after, 5 mutants of the fix each red in a named case. Not reached by
  the pass: a timing comparison of an uncovered inbox and a bad signature, a value over the
  bound of a header.
- Adversary pass 2 on round 3 (review-result:adversary-delegation-acts-pass-2), in a tree of
  its own on the build node: no flaw in the code of round 3; 4 cases added, all green; 4
  notes. 3 are gaps of the cases, each closed by a case of the pass: of 4 mutants, 3 were red
  in no case of the story or of pass 1 (the check of `--for` moved after the URL, the key
  file and the outbox are read; `char::is_control` narrowed to ASCII; the request head made
  after the connection is opened) and the fourth (the head checked only for header values)
  was red in the unit case of the client. The fourth note: a key file whose key id holds a
  line break, which `inbox key new` writes with exit 0, now ends every command of the client
  with exit 2 and a line that names no key file; it is not changed here and is filed as the
  draft story:a-key-id-with-a-control-character-is-refused-where-it-is-made (Decisions). The
  4 new cases took 0.012 s to 0.223 s. The record is the report word for word but for one
  phrase: 3 words the report joined by slashes are joined by a comma and "or", because the
  source gate reads the joined form as a path it refuses. Not reached by the pass: the TLS
  path of the order of head and connection, `bridge` and `wake` under such a key id, whether
  `post` keeps an envelope in the outbox before the head is refused.
- The specification: `ess verify diff --compatibility --fail-on breaking-or-unknown` from the
  specification before the 3 sentences to the one after: `3 change(s): 0 widening, 0
  narrowing, 3 other`, `compatibility: 0 breaking, 0 unknown, 3 compatible`. `task spec` took
  0.10 s before and after. `crates/inbox-conformance/unsupported.tsv`: 87 lines, 71 with the
  reason "a caller's principal and inbox differ", as before.
- The cases: 56 added (31 in `delegation`, 4 in `acting`, 2 in `security_delegation`,
  2 in `security_acting`, 7 in `acting_for`, 9 in `adversary_acting_for`, 1 unit case of the
  client). No case over 5 s: no case of the
  listener or the verifier took over 0.02 s in round 1, and the slowest of the client,
  `acting_for::without_a_live_delegation_of_the_pair_every_command_ends_as_an_unknown_key`,
  took 0.22 s and 1.19 s in 2 runs. The 7 cases of `acting_for`, run 10 times in a row:
  `10/10 stress run iterations: 10 passed`.
- The gate runs of the rounds on the build node (`task check`): round 1 1012 passed, 8
  skipped, the test step 16 s, exit 0; round 2, on the local branch of the story before, 1040
  passed, 1 failed (a case of the story before that its own last round made green), 15 s;
  with `main` merged in 1042 passed, 8 skipped, 15 s, exit 0; round 3 1048 passed, 8 skipped,
  15 s, exit 0.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T08:22:59Z to 2026-11-24T08:23:37Z, on the build
  node of the pool: exit 0, `1052 tests run: 1052 passed, 8 skipped`, the test step 16 s of
  its bound of 120 s.
