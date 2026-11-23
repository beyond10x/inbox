---
format: aep.planning-md/3
id: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
kind: story
status: draft
title: The chat bridge runs with its own key under a delegation for the person's inbox
summary: 'Under the operator''s answer B on the bridge''s key: every request of the bridge names the person''s inbox, and the wake lock, the open record and the chat-post record are keyed by the person''s address.'
relations:
- decomposes: epic:human-inbox
- depends_on: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
scope:
- confidence: inferred
  path: crates/inbox-cli/src/bridge.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/failure.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/src/wake.rs
- confidence: inferred
  path: crates/inbox-cli/tests/bridge_own_key.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: docs/design/bridge.md
- confidence: cited
  path: docs/design/wake.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
revision: 8
---
## Outcome

Under the operator's answer B (approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner),
the chat bridge runs with a key of its own and the delegation id the person gave it, instead of
a key of the person. Every
request it sends names the person's inbox as the inbox it acts for, and the files it keeps
beside the outbox file are keyed by the person's address, not by its own principal. Posting,
the delivery mark, the read mark and the decision asked once work as before for the person's
inbox.

This is the switch the first 3 bridge stories leave out: they sign with the key they are given,
and their cases give them a key of the person. It is split from
story:the-chat-bridge-turns-a-thread-reply-into-an-answer to keep that story to one pull
request.

## Acceptance

A bridge started with its own key (principal `agent:demo/chat-bridge` in the cases) and a live
delegation for the person posts a note posted to the person's inbox as one chat post and moves
it to `Delivered`, where before this story the same bridge with its own key reads no row of the
person's inbox and posts nothing (state transition: `Sent` to `Delivered` by a bridge that holds
no key of the person).

Checks, each of which can fail:

1. Every request of the bridge names the person's inbox (the header of Open 2 A of
   story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal): both reads of each
   `Pending` round (`wait=0` and the held read, with `undelivered=1`), `mark-delivered`, `read`,
   the `thread` reads of story:the-chat-bridge-asks-a-decision-once and the `open-inbox` of the
   reconnect. A forwarder in front of the service that records the headers of each request sees
   the header with the person's address on every one; the case fails on the first request
   without it.
2. The lock of the wake, `<outbox>.wake.<16 hex digits>`, is the hash of the person's address,
   not of the bridge's principal (today `client.caller.principal`, `crates/inbox-cli/src/wake.rs`,
   `run` and `terminal_only`): a second bridge for the same person with the same outbox file and
   the same key exits 5 and posts nothing; a bridge for a second person with the same outbox file
   and the same key runs and posts that person's messages.
3. The record of `inbox open`, `<outbox>.opened`: the open of the bridge writes the line of the
   person's inbox with the adapter `slack`; after a restart of the service its reconnect opens
   the person's inbox again (the `Inboxes` view lists it `Open` with `slack`), not an inbox of
   the bridge's own address (the view lists none).
4. The record of the chat posts, `<outbox>.chat.<16 hex digits>`, is keyed by the person's
   address: a bridge that ran with a key of the person and is started again with its own key for
   the same person reads the same record, and the stand-in records no second post operation for a
   message already posted.
5. The check of the adapter (Open 4 A of story:the-chat-bridge-posts-a-pending-message) reads
   the person's line of `<outbox>.opened`: with a line `terminal` there the bridge with its own
   key ends with 1 and its one line, and posts nothing.
6. The bridge with its own key ends on a refused key or delegation, as Open 4 says. Under its
   default, the case runs 2 causes: the delegation removed from the key file or its
   `valid_until` at or before the clock of the service when the service starts; and the key
   entry of the bridge lapsed. In each, every request is answered `401`, and the bridge exits
   with status 1 after one line on standard error, the same line for both causes, that names the
   key id, the delegation id, `inbox keyfile add-key`, `inbox keyfile add-delegation`, and that
   the service reads the key file at its start. It posts nothing and sends no further request
   (the forwarder sees none after the first `401`). The case fails when the bridge goes on,
   exits 0, or its line lacks one of the 2 ids or one of the 2 commands. This story builds that
   end; no earlier story has it.
7. The cases run against a real `inbox serve` on loopback, with a key file made in the case that
   holds the bridge's key entry and the delegation, and the stand-in `connectors` of
   story:the-chat-bridge-posts-a-pending-message. No case reaches a chat provider, a network
   beyond loopback or a model; no case over 60 s, none holds a wall-clock time, the test step
   within 120 s, no new Python. A build or a test runs through `slot`, and a pool job ends after
   10 minutes unless it names its time with `slot --timeout` (30 minutes at most).

## Context

Every file the client keeps beside the outbox file is found from the caller: the lock of the
wake by the caller's address (`docs/design/wake.md`, "One wake per inbox on a host"), the record
of `inbox open` per inbox (`crates/inbox-cli/src/outbox.rs`, `opened`, read with
`client.caller.principal` in `crates/inbox-cli/src/wake.rs`), and the record of the chat posts by
the hash of the address (Open 6 A of story:the-chat-bridge-posts-a-pending-message). With a key
of the person, the caller's principal is the person's address and all 3 are right; with the
bridge's own key each would be found under the bridge's principal. The outbox file itself and
its lock, `<outbox>.lock`, hold the sender's envelopes; the bridge posts none.

## Domain relations

- Client to Inbox: the caller carries `inbox` beside `principal` (`ess/domains/messaging.yaml`,
  actor `inbox.messaging.Client`, lines 252-258). Inferable from the ess/1 document; that a
  delegation makes the two differ is story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.
- Inbox to owner after an open by the bridge: `OpenInbox` sets `owner: {caller: principal}`
  (same file, outcome `opened`, line 348), so the bridge becomes the owner; whether that is
  wanted is Open 4 of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.
  Inferable from the ess/1 document as it stands.

## Open

1. How the bridge is told the inbox it acts for. (A) flags `--for <address>` (one flag or two:
   Open 3) and `--authority <delegation id>`; the address is also the key of the 3 files; (B) the
   bridge reads the delegation from the key file: it would read a file the client does not read
   today. Default: A.
2. Whether the cases of the first 3 bridge stories run with both keys. (A) the acceptance case of
   each runs once more with the bridge's own key (3 cases more, each timed and named in the
   report); (B) only the checks above. Default: A.
3. One flag or two for the inbox acted for. The client flag that names it comes with
   story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal (its Scope: "the flag that
   names that inbox", sent as the header of its Open 2 A); no source says whether `inbox bridge`
   takes that flag or one of its own. (A) one flag: the client flag of that story, on every client
   command, `inbox bridge` included, named `--for <address>` there; this story adds only
   `--authority`; (B) two flags: the client flag, and a `--for` of `inbox bridge` alone that the
   bridge turns into it. Default: A.
4. What the bridge does when every request is refused for its key or its delegation (moved here
   from story:the-chat-bridge-turns-a-thread-reply-into-an-answer, which held it as its Open 5):
   a key or a delegation that has lapsed or was never made, answered `401` (the refusal of Open 3
   A of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal). (A) the bridge ends
   with exit 1 and one line, whatever the cause, that names the key id and the delegation id,
   `inbox keyfile add-key` and `inbox keyfile add-delegation`, and that the service reads the
   key file at its start: both causes get the same `401` whose body says nothing more
   (`docs/design/service.md`, "The request signature"), and the secret key file of the bridge
   holds no `valid_until` (`crates/inbox-cli/src/keys.rs`), so the bridge cannot tell them apart;
   (B) it keeps trying after the pause of a failure and writes one line into the chat channel.
   Default: A.

## Scope

After story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal (edge `depends_on`),
and before
story:the-chat-bridge-turns-a-thread-reply-into-an-answer, which edits `bridge.rs` after it.
This story owns the end of the bridge on a refused key or delegation (check 6, Open 4) whole.

Code:

- `crates/inbox-cli/src/bridge.rs` (the flags, the address the 3 files are keyed by, the end on
  a refused request of check 6): inferred
- `crates/inbox-cli/src/failure.rs` (the failure of check 6, its exit status 1 and its line):
  cited
- `crates/inbox-cli/src/wake.rs` (the lock and the adapter check keyed by the inbox acted for):
  cited
- `crates/inbox-cli/src/outbox.rs` (the record of `inbox open` of the inbox acted for): cited
- `crates/inbox-cli/src/commands.rs` (the reconnect opens the inbox acted for): cited
- `crates/inbox-cli/src/main.rs` (`--authority` of `inbox bridge`; `--for` only under Open 3 B),
  `crates/inbox-cli/src/reference.rs`,
  `generated/cli/inbox.json` (regenerated with `task cli-reference`): cited
- `crates/inbox-cli/tests/bridge_own_key.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred

Documents:

- `docs/design/bridge.md`: a section on the key of the bridge: inferred
- `docs/design/wake.md`: "One wake per inbox on a host", the address of the lock: cited
- `website/docs/first-use.md`: the section of the bridge, the key and the delegation: cited
