---
format: aep.planning-md/3
id: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
kind: story
status: proposed
title: The chat bridge turns a thread reply of the allowed chat user into a signed answer
summary: A thread reply to the chat post of a request or a decision is sent as Answer on behalf of the person and moves the message from Read to Answered with answered_by principal.
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-asks-a-decision-once
- depends_on: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
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
  path: crates/inbox-cli/src/output.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: inferred
  path: crates/inbox-cli/tests/bridge_answer.rs
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
- confidence: cited
  path: website/docs/status.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:45Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
---
## Outcome

A thread reply of the allowed chat user to the chat post of a request or a decision becomes a
reply envelope that the bridge signs and sends as `Answer` on behalf of the person: the message
moves to `Answered` with `answered_by: principal`, a decision with the option the reply named.
A chat post of a decision shows its lettered options, so the person knows what to reply. A
reply that names no option, a reply of another chat user and a reply the service refuses send
nothing the person did not mean, and the thread says what became of the reply.

## Acceptance

A thread reply `B` of the allowed chat user to the chat post of a decision with the options A
and B moves the message from `Read` to `Answered` with `answered_by: principal` and
`decided_option: B`, and its reply carries `on_behalf_of` the person, where before this story a
message to a person is answered only with `inbox answer` in a terminal (state transition: `Read`
to `Answered` through a thread reply in the chat channel).

Checks, each of which can fail:

1. The noun is drafted first, outside `ess/`: the section "The nouns of the bridge" of
   `docs/design/bridge.md` gets the noun `ChatReply` (the chat user id, the text, the chat post
   it replies to) and the state answered of a `ChatPost`, each relation the sources do not
   settle written there as `UNMAPPED:` with the Open item that answers it (Open 4). A relation
   enters `ess/` only as story:the-chat-bridge-posts-a-pending-message (check 1) says. After the
   story `grep -rn UNMAPPED ess` prints nothing and `task spec` exits 0.
2. The chat post of a decision shows each option with its letter and text, which option the
   sender recommends, the option silence takes and `expires` (Open 1).
3. A reply that matches an option (Open 2) is sent as `Answer` with that option; a reply to a
   request is sent with its text as the body. The reply envelope has `from`, `on_behalf_of` and
   `authority` as Open 6 says, and is signed with the key the bridge is given.
4. A message still in `Delivered` when its reply arrives is first marked read, so it moves
   `Read` to `Answered` (Open 3).
5. A reply that matches no option sends nothing to the service and gets one thread line that
   names the letters of the options (Open 2); a reply to a note sends nothing and gets one line
   that a note takes no answer; a reply longer than 65536 bytes of UTF-8 sends nothing and gets
   one line that it is too long (`Body`, `ess/domains/messaging.yaml`, lines 55-61).
6. A reply of a chat user who is not the allowed chat user sends nothing and writes nothing, as a
   reaction of another chat user does
   (story:the-chat-bridge-marks-a-message-read-on-a-reaction, Open 2).
7. An `Answer` the service refuses (`already-settled`, `deadline-passed`, `expired-on-arrival`,
   `shape-mismatch`, `no-authority`, `authority-invalid`, `signature-invalid`) gets one thread
   line that names the refusal and says the reply was not taken, and is not sent again; an
   `Answer` that failed in transport is tried again after the pause of a failure. A reply after
   `expires` is so answered `deadline-passed`, and the decision is left to silence
   (story:the-chat-bridge-asks-a-decision-once). The end of the bridge when every request is
   refused for its key or its delegation is not this story's: check 6 of
   story:the-chat-bridge-runs-with-its-own-key-under-a-delegation.
8. A second matching reply after the first was taken is answered `already-settled` and gets the
   line of check 7 (Open 4).
9. The cases run against a real `inbox serve` on loopback, with a key file made in the case (and
   the bridge's key and delegation of
   story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, the operator's answer B), and
   the stand-in `connectors` with scripted thread replies; a case collects
   the answer with `inbox collect` of the sender and checks the reply. No case reaches a chat
   provider, a network beyond loopback or a model; no case over 60 s, none holds a wall-clock
   time, the test step within 120 s, no new Python. A build or a test runs through `slot`, and
   a pool job ends after 10 minutes unless it names its time with `slot --timeout` (30 minutes
   at most).

## Context

epic:human-inbox, Outcome: the bridge "turns a thread reply from the allowed chat user into a
reply envelope signed with the bridge's key on behalf of the person, under a delegation";
Acceptance: "the person's thread reply moves the message from `Read` to `Answered` with
`answered_by: principal`".

Which key signs the reply, and whether a delegation is involved, was
decision-blocker:delegation-grants-inbox-ownership: today only a key whose entry names the
person as principal can answer a message in the person's inbox (`Answer` refuses `not-sender`
and `not-recipient` otherwise). The operator answered B
(approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner):
the bridge answers with its own key under a delegation. The switch of every other request of the
bridge to its own key (the `Pending` rounds, `MarkDelivered`, `MarkRead`, the `Thread` reads,
and the 3 files beside the outbox file keyed by the caller) is
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, split from this story to keep it
to one pull request; this story adds only the reply.

The client already signs and sends a reply: `inbox answer <message id> --body --option
--on-behalf-of --authority --expires` (`crates/inbox-cli/src/main.rs`, `AnswerArgs`;
`commands::answer`, `crates/inbox-cli/src/commands.rs:737`), with `expires` created plus 24 hours
by default.

## Domain relations

- `Answer`: the input is the reply envelope; refused `not-sender`, `no-authority`,
  `already-expired`, `signature-invalid`, `authority-invalid`, `unknown-message`,
  `foreign-message`, `not-recipient`, `already-settled`, `note`, `deadline-passed`,
  `expired-on-arrival`, `shape-mismatch`; `answered` moves the message to `Answered` with
  `answered_by: principal`, `decided_option` the reply's option and the reply on the message
  (`ess/domains/messaging.yaml`, command `inbox.messaging.Answer`, lines 632-749). Inferable from
  the ess/1 document.
- A message is answered from `Sent`, `Delivered` or `Read` (same file, transition `answer`, line
  248). Inferable from the ess/1 document.
- One reply per message: `reply` is one optional field of `Message` (same file, line 228; struct
  `inbox.messaging.Reply`, lines 155-167). Inferable from the ess/1 document.
- The options of a decision: `DecisionAsk` with `options` (letter, text) and
  `recommended_option`; the option silence takes is the envelope's `default_on_silence` (same
  file, lines 82-95). Inferable from the ess/1 document.
- `ChatReply` to `ChatPost`: no ESS document declares either. A chat post may have many thread
  replies in the chat provider (not verified); which of them becomes the answer: `UNMAPPED:` in
  `docs/design/bridge.md`, answered by Open 4.

## Open

1. How the options of a decision are shown. (A) one line per option `<letter>: <text>`, the
   recommended one marked, then one line with the option silence takes and `expires`, and one
   line "reply in this thread with the letter of an option"; (B) buttons of the chat provider:
   the provider would call the bridge, a listener outside loopback (Open 8 of
   story:the-chat-bridge-posts-a-pending-message), not assumed. Default: A.
2. How a reply is matched to an option. (A) the text of the reply, trimmed, is one letter of an
   option in either case, or starts with that letter and a space, a colon or a full stop; the
   rest of the text is the body, and with no rest the body is the text of the option; (B) the
   whole text is exactly one letter; (C) the first letter of an option anywhere in the text. A
   reply that matches none: (A) no `Answer`, one thread line naming the letters; (B) the `Answer`
   is sent and the service refuses it `shape-mismatch`, which the thread line names. Default: A
   and A.
3. Read before the answer. (A) a message still in `Delivered` is marked read first, so it moves
   `Read` to `Answered`, as the epic's acceptance names; (B) it is answered from `Delivered`,
   which the model allows. Default: A.
4. Which reply counts when there are several. (A) the first reply of the allowed chat user that
   matches; every later one gets the line of `already-settled`; (B) the last one before a poll.
   Default: A.
5. Moved: what the bridge does when every request is refused for its key or its delegation is
   Open 4 of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, which builds it.
6. Settled: `from`, `on_behalf_of` and `authority` of the reply. The operator answered B of
   decision-blocker:delegation-grants-inbox-ownership on 2026-11-23
   (approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner): `from` the bridge,
   `on_behalf_of` the person, `authority` the delegation id the bridge is given (the flag of
   story:the-chat-bridge-runs-with-its-own-key-under-a-delegation). The options were: (A) `from`
   and `on_behalf_of` the person, no `authority`; (B) as settled; (C) as B, with requests signed
   by the key of the person.

## Scope

After story:the-chat-bridge-asks-a-decision-once (it edits `bridge.rs` after it) and after
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation (edge `depends_on`), which
switches the bridge to its own key and itself comes after
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.
decision-blocker:delegation-grants-inbox-ownership, which blocked this story, is cleared by the
operator's answer B (approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner),
so the edge `depends_on` to story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
stays. The end of the bridge on a refused key or delegation is check 6 and Open 4 of
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation.

Code:

- `crates/inbox-cli/src/bridge.rs` (the options in the chat post, the reading of thread
  replies, the match, the read mark before the answer, the thread lines): inferred
- `crates/inbox-cli/src/commands.rs` (`answer`, `read`): cited
- `crates/inbox-cli/src/main.rs` (the help text of `inbox bridge` on thread replies; this story
  adds no flag: the flags of the key and of the delegation id are those of
  story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, its Open 1 A and Open 3),
  `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json` (regenerated with
  `task cli-reference`): cited
- `crates/inbox-cli/src/output.rs` (the text of the options): cited
- `crates/inbox-cli/tests/bridge_answer.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred
- `crates/inbox-cli/tests/support/` (thread replies in the script of the stand-in): inferred

Documents:

- `docs/design/bridge.md`: a section on the answer: inferred
- `website/docs/first-use.md`: the section of the bridge, how to answer in the chat channel:
  cited
- `website/docs/status.md`: the row of the bridge in "Available", the planned item "An inbox for
  a person" taken out of "Planned": cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`: cited
