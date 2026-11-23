---
format: aep.planning-md/3
id: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
kind: story
status: draft
title: An agent with a delegation acts for the inbox of its principal
summary: 'A live delegation lets its agent read, mark and answer the inbox of its on_behalf_of: the operator''s answer B of 2026-11-23 on the bridge''s key.'
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-asks-a-decision-once
scope:
- confidence: cited
  path: AGENTS.md
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
  path: crates/inbox-server/src/auth.rs
- confidence: cited
  path: crates/inbox-server/src/service.rs
- confidence: inferred
  path: crates/inbox-server/tests
- confidence: inferred
  path: crates/inbox-verify/src
- confidence: cited
  path: docs/design/inbox.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: docs/design/signing.md
- confidence: inferred
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/status.md
revision: 14
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

Documents:

- `docs/design/inbox.md`: decisions 4 and 5: cited
- `docs/design/service.md`: "The caller", "The request signature": cited
- `docs/design/signing.md`: "Request", "A delegation": cited
- `AGENTS.md`: the rows of `crates/inbox-server/`, `crates/inbox-verify/` and
  `crates/inbox-conformance/`: cited
- `website/docs/status.md`: the row of delegations in "Signing": cited
- `CHANGELOG.md`: not edited: cited
