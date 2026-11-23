---
format: aep.planning-md/3
id: decision-blocker:delegation-grants-inbox-ownership
kind: decision-blocker
status: open
title: The operator has not decided whether a delegation grants acting as the owner of its principal's inbox
relations:
- blocks: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
- blocks: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
- blocks: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
revision: 1
---
## The question

Does a live delegation entry of the key file (`agent` may speak for `on_behalf_of` until
`valid_until`) also let its agent act as the owner of the inbox of its `on_behalf_of`: read its
`Pending` view, mark its messages delivered and read, and answer them? The epic asks for "a
reply envelope signed with the bridge's key on behalf of the person, under a delegation"
(epic:human-inbox, Outcome). Today the service makes the inbox of a caller the same text as its
principal (`crates/inbox-server/src/service.rs:87`; `docs/design/service.md`, "The caller";
decision 5 of `docs/design/inbox.md`), and `Answer` refuses `not-sender` when `from` is not the
caller's principal and `not-recipient` when the message's `to` is not the caller's inbox
(`ess/domains/messaging.yaml`, lines 648-680). So the reply the epic names cannot be made, and
the answer is a change of architecture, which the folder rule "A default proceeds" leaves to the
operator.

## Options

- (A) No delegation: the bridge holds a key whose entry names the person as principal (`inbox
  key new`, `inbox keyfile add-key`); its reply has `from` and `on_behalf_of` the person and no
  `authority`. No change of the service. The epic's acceptance holds (`answered_by:
  principal`), its outcome does not: a reply of the bridge cannot be told from one the person
  signed, and the bridge holds a key that speaks as the person in every command. The 2 stories
  for B are rejected, and the edge to them from
  story:the-chat-bridge-turns-a-thread-reply-into-an-answer is removed.
- (B) A delegation lets its agent act as the owner of the inbox of its `on_behalf_of`; the bridge
  holds a key of its own (story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal,
  then story:the-chat-bridge-runs-with-its-own-key-under-a-delegation). Costs: the service, the
  design decisions 4 and 5, the signed request (a sixth member, Open 2 of the first story), and
  an inbox opened by the agent is owned by the agent (Open 4 of the first story).
- (C) The bridge reads and marks with a key of the person, as in A, and signs only the reply
  with its own key under a delegation: the model's `not-sender` of `Answer` refuses that, so
  `ess/domains/messaging.yaml`, `generated/` and the conformance suite change.

Recommended: B, not taken.

## What it stops

- story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal (exists for B; rewritten
  for C).
- story:the-chat-bridge-runs-with-its-own-key-under-a-delegation (exists for B and C).
- story:the-chat-bridge-turns-a-thread-reply-into-an-answer: its reply's `from`,
  `on_behalf_of` and `authority`, and its edge `depends_on` to the story above.

The first 3 bridge stories (posting, the read mark, the decision asked once) do not wait for
the answer: they sign with the key they are given, and their cases use a key of the person.

## What clears it

The operator's answer A, B or C, relayed through `host-operator` and recorded; then the
blocker is moved to `cleared`. Nobody moves it on a default.
