---
format: aep.planning-md/3
id: story:the-chat-bridge-marks-a-message-read-on-a-reaction
kind: story
status: draft
title: The chat bridge marks a message read on the allowed chat user's reaction
summary: A reaction of the allowed chat user on a chat post sends MarkRead; a reaction of another chat user changes nothing.
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-posts-a-pending-message
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: crates/inbox-cli/src/bridge.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: inferred
  path: crates/inbox-cli/tests/bridge_read.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/support
- confidence: inferred
  path: docs/design/bridge.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
revision: 5
---
## Outcome

When the allowed chat user reacts to the chat post of a message, the bridge sends `MarkRead` for
it: a request or a decision moves from `Delivered` to `Read` (it leaves `Pending` for
`Unanswered`), and a note is handled and removed. A reaction of any other chat user changes
nothing. The person reads a message in the chat channel the way the person reads one with
`inbox read` in a terminal.

## Acceptance

A reaction of the allowed chat user on the chat post of a request, as the stand-in of
`connectors` reports it, moves the message from `Delivered` to `Read` at the next poll of the
bridge, and the same reaction of another chat user leaves it in `Delivered`, where before this
story a message the bridge posted stays in `Delivered` until the person types `inbox read`
(state transition: `Delivered` to `Read` through a reaction in the chat channel).

Checks, each of which can fail:

1. The noun is drafted first, outside `ess/`: the section "The nouns of the bridge" of
   `docs/design/bridge.md` gets the state read of a `ChatPost` and the noun `ChatReaction` (the
   chat user id that reacted, the reaction, the chat post it is on), each relation the sources do
   not settle written there as `UNMAPPED:` with the Open item that answers it. A relation enters
   `ess/` only as story:the-chat-bridge-posts-a-pending-message (check 1) says. After the story
   `grep -rn UNMAPPED ess` prints nothing and `task spec` exits 0.
2. A reaction of the allowed chat user on the chat post of a note sends one `read` request,
   which removes the note (outcome `handled`), and the record of the bridge holds the post as
   read. A forwarder in front of the service that records the target of each request (as the
   `Forwarder` of `crates/inbox-cli/tests/wake.rs` records connections) sees exactly one
   `read` request for that message, also when the stand-in lists the same reaction again at the
   next 2 polls. The case fails on a second `read`.
3. A reaction of a chat user who is not the allowed chat user sends no request to the service
   (the forwarder sees no `read`) and nothing to the chat channel (the stand-in records no post
   operation) (Open 2 A).
4. A reaction on the chat post of a message that is due (its `expires` at or before the clock of
   the service, moved with `inbox serve --clock-file`) is answered `deadline-passed`
   (`Expired`): the bridge writes one line on standard error that names the message id and
   `Expired`, the record holds the post as due, and the forwarder sees no second `read` for it.
   A reaction on a message read meanwhile with `inbox read` (`already-read`) or gone
   (`unknown-message`) writes no line on standard error, the record holds the post as read, and
   the bridge goes on: a note posted after it is posted and in `Delivered`.
5. A poll that fails (the stand-in exits non-zero or answers a refusal) marks nothing and is
   tried again after the pause of a failure (`wake::pause`), with one line on standard error per
   run of failures; a failed poll does not stop the rounds of `Pending`, so a message posted
   meanwhile is still posted and marked delivered.
6. A request whose chat post has a reaction moves from `Delivered` to `Read`, and the stand-in
   keeps listing that reaction at later polls: the forwarder sees one `read` for it, and the
   message stays in `Read` (in `Unanswered`, not in `Pending`), so a bridge that marks a post
   again at every poll fails the case.
7. The cases run against a real `inbox serve` on loopback and the stand-in `connectors` of
   story:the-chat-bridge-posts-a-pending-message, with a scripted list of reactions. No case
   reaches a chat provider, a network beyond loopback or a model; no case over 60 s and none
   holds a wall-clock time (the interval of the poll is a flag the case sets low, and a case
   waits on the stand-in's record of an invocation, not on a time); the test step within 120 s;
   no new Python. A build or a test runs through `slot`, and a pool job ends after 10 minutes
   unless it names its time with `slot --timeout` (30 minutes at most).

## Context

epic:human-inbox, Outcome: the bridge "marks it read on the person's reaction". The allowed chat
user and where it is configured are Open 2 and 3 of story:the-chat-bridge-posts-a-pending-message;
this story is the first to compare a chat user with it.

The bridge asks for reactions: it has no listener (Open 8 of
story:the-chat-bridge-posts-a-pending-message) and the chat provider reaches it only through
`connectors invoke`. Which read operations the connector of the chat provider has is not
verified (read from `connectors --help` and `connectors invoke --help` only: `--walk`, `--each`
and `--fields` exist for reads).

## Domain relations

- Who marks a message read, and what it does: only the owner of the inbox in `to`; from `Sent`
  or `Delivered`; a note in `Sent` or `Delivered` is deleted (`handled`), a request or a decision
  moves to `Read`; `already-read` changes nothing; `deadline-passed` (`Expired`) when the clock
  of the service has reached `expires` (`ess/domains/messaging.yaml`, command
  `inbox.messaging.MarkRead`, lines 581-630; transition `read`, line 247). Inferable from the
  ess/1 document.
- A message in `Read` is in `Unanswered`, not in `Pending` (same file, views
  `inbox.messaging.Pending`, filter line 966, and `inbox.messaging.Unanswered`, filter line
  993). Inferable from the ess/1 document.
- `ChatReaction` to `ChatPost`: no ESS document declares either. Many reactions may be on one
  chat post in the chat provider (not verified); which of them counts: `UNMAPPED:` in
  `docs/design/bridge.md`, answered by Open 1.
- `ChatReaction` to the allowed chat user: compared by chat user id (Open 3 A of
  story:the-chat-bridge-posts-a-pending-message); `UNMAPPED:` in `docs/design/bridge.md` until
  that is answered.

## Open

1. What "the reaction" is. (A) any reaction of the allowed chat user on the chat post; (B) one
   named reaction, configured with the binding; (C) a reaction or a thread reply, so a reply
   reads the message too and story:the-chat-bridge-turns-a-thread-reply-into-an-answer need not
   send `MarkRead` itself. Default: A.
2. What a reaction of another chat user does. (A) nothing: no request, no chat post, no line;
   (B) one line on standard error of the bridge, naming no chat user; (C) a thread reply that
   says only the allowed chat user reads the message: the bridge then writes into the channel on
   the act of a third user. Default: A.
3. How and how often the bridge asks. (A) a thread of the bridge asks `connectors` at an
   interval (a flag, default 10 s, at least 1 s) while the round of `Pending` is held; (B) the
   held read of `Pending` cut to the interval, and the poll between two reads. Not verified: the
   rate limits of the chat provider and the cost of one `connectors invoke`. Default: A.
4. What one poll reads. (A) one read per chat post not yet read (its reactions); (B) one read of
   the recent posts of the chat channel per poll, matched against the record of the bridge, so
   the number of invocations does not grow with the number of open posts. Not verified which
   operation of the connector gives either. Default: B.

## Scope

After story:the-chat-bridge-posts-a-pending-message (edge `depends_on`), and before
story:the-chat-bridge-asks-a-decision-once, which edits `bridge.rs` after it.

Code:

- `crates/inbox-cli/src/bridge.rs` (the poll, the comparison with the allowed chat user, the
  read mark, the state read of the record): inferred
- `crates/inbox-cli/src/commands.rs` (`read`): cited
- `crates/inbox-cli/src/main.rs` (the flag of the interval), `crates/inbox-cli/src/reference.rs`,
  `generated/cli/inbox.json`: cited
- `crates/inbox-cli/tests/bridge_read.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred
- `crates/inbox-cli/tests/support/` (reactions in the script of the stand-in): inferred

Documents:

- `docs/design/bridge.md`: a section on the read mark: inferred
- `website/docs/first-use.md`: the section of the bridge: cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`: cited
