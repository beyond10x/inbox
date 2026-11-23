---
format: aep.planning-md/3
id: story:the-chat-bridge-asks-a-decision-once
kind: story
status: proposed
title: The chat bridge asks a decision once, across restarts of the service and of the bridge
summary: One chat post per message id across restarts; at expires the service answers a decision by silence and the bridge answers nothing in its place; under Open 2 A only, one thread line says how it ended.
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-marks-a-message-read-on-a-reaction
scope:
- confidence: inferred
  path: crates/inbox-cli/src/bridge.rs
- confidence: cited
  path: crates/inbox-cli/src/clock_file.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: inferred
  path: crates/inbox-cli/tests/bridge_once.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: docs/design/bridge.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:44Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":4}}}
---
## Outcome

A decision that names `default_on_silence` is asked once in the chat channel: one chat post,
also after a restart of the service (the sender posts it again under the same message id) and
after a restart of the bridge. When nobody answers, the service answers it by silence at
`expires` with the named option, as it does today, and the bridge sends nothing in its place.
The same holds for a note and a request: a message is posted once.

This is the case the stage exists for (epic:human-inbox, Context: "the person is asked once, and
at `expires` the service answers by silence with the named option"). Under Open 2 A the bridge
also writes one line into the thread of the chat post when silence answered or the message
expired; that line is an addition the epic does not ask for.

## Acceptance

A decision with `default_on_silence: B` that comes back in `Sent` after a restart of the service
and the sender's repost is moved to `Delivered` by the bridge with no second invocation of the
post operation, where before this story the bridge holds no rule for a message whose id its
record already holds (state transition: `Sent` to `Delivered` of a reposted message, with the
stand-in recording one post operation for that message id in all).

Checks, each of which can fail:

1. The states are drafted first, outside `ess/`: the section "The nouns of the bridge" of
   `docs/design/bridge.md` gets the states of a `ChatPost` after posted and read (decided on
   silence, expired, answered, gone) and the relation of `ChatPost` to Message, as `UNMAPPED:`
   with Open 1 until the operator answers it. A relation enters `ess/` only as
   story:the-chat-bridge-posts-a-pending-message (check 1) says. After the story
   `grep -rn UNMAPPED ess` prints nothing and `task spec` exits 0.
2. After a restart of the service and the sender's next command, which posts the message again
   under its id (`docs/design/service.md`, "The epoch"), the message is in `Sent` again: the
   bridge sends `MarkDelivered` for it, and the stand-in records no second post operation for
   that message id.
3. The same with the record holding the chat post as read: the bridge sends `MarkRead` after
   `MarkDelivered`, so a request is in `Read` again (in `Unanswered`), and a note read before
   the restart is removed (the sender's `inbox outbox` no longer lists it), with no post.
4. After a restart of the bridge, a message whose id its record holds, in `Sent` or in
   `Delivered`, gets no post operation from the new bridge (the stand-in records none).
5. A decision with `default_on_silence` and no reply: after the clock of the service passed its
   `expires` and the expiry task ran, the sender's `inbox collect` prints `answered_by: silence`
   and `decided_option` the named option, and the forwarder in front of the service saw no
   `answer`, `read` or `refuse` request of the bridge for that message. This holds with the
   default of every Open item; it fails when the bridge answers in the person's place.
6. Under Open 2 A only: the thread of that chat post gets one line that names the option and
   its text and says it was taken on silence, once (the stand-in records one reply operation for
   that thread, also after a restart of the bridge); a decision without `default_on_silence`, a
   request and a note that expire unanswered get one line "expired, not answered", once; a
   message the person answered in a terminal (`inbox answer`) gets neither line.
7. The record does not grow without a bound (Open 1 A): an entry is dropped when its message is
   gone from the service (its `Thread` read answers no row) and the clock of the bridge has
   passed its `expires` plus the retention. The bridge reads its instant from a clock the case
   can move: `inbox bridge --clock-file <PATH>`, named and read as `inbox serve --clock-file` is
   (`crates/inbox-cli/src/main.rs`, `ServeArgs::clock_file`: a flag of a debug build only,
   `#[cfg(debug_assertions)]`, hidden from `--help` and the reference, the file read by
   `FileClock` of `crates/inbox-cli/src/clock_file.rs` each time the bridge asks for the time,
   failing closed as the service's does); without it, the system clock. The bridge still signs
   its requests with the system clock, as every client command does. The case gives the bridge
   a clock file of its own: the service's file stays within 60 s of the system clock, since the
   service takes a request signed within 60 s of its reading (`docs/design/service.md`, "The
   expiry task"). With the retention flag at 1 h, the case lets the decision be taken on silence
   (the service's file moved 1 s past `expires`) and collected by the sender, then writes the
   bridge's file to `expires` plus 59 min and reads the record after the stand-in has recorded 2
   more polls: the entry is still there; then to `expires` plus 1 h and 1 s, and after 2 more
   polls the record no longer holds the message id. No sleep and no real instant: the case waits
   only on the stand-in's count of polls.
8. The cases run against a real `inbox serve` of a debug build with `--clock-file`
   (`docs/design/service.md`, "The expiry task"), so a case moves the clock of the service past
   `expires` by writing the file and holds no wall-clock time, and `--sweep-interval 1`; the
   bridge with a `--clock-file` of its own where its clock matters (check 7); a
   restart of the service is a new `inbox serve` behind one forwarder, as the cases of stage 3
   do. The stand-in `connectors` of story:the-chat-bridge-posts-a-pending-message; no case
   reaches a chat provider, a network beyond loopback or a model; no case over 60 s, the test
   step within 120 s, no new Python. A build or a test runs through `slot`, and a pool job ends
   after 10 minutes unless it names its time with `slot --timeout` (30 minutes at most).

## Context

The service already answers a decision by silence: `Expire` with `decided-on-silence`
(`ess/domains/messaging.yaml`, lines 861-893); story:read-decision-expires-on-silence-scenario
and story:silence-leaves-the-other-fields-scenario hold it. What the person does not have is a
chat channel that asks once and then says what silence decided.

A message comes back after a restart: the memory backend loses every message at a restart, and
the sender's next command posts its kept envelopes again under the same ids
(`docs/design/inbox.md`, "Shape"; `docs/design/service.md`, "The epoch";
`website/docs/limits.md`, "`inbox repost` can deliver a read note once more"). The reconnect of
the bridge opens the inbox again with its adapter, as every client command does.

The owner of an inbox may read the `Thread` view of a thread whose rows are in its inbox (the
read rule: `docs/design/service.md`, "The read rule"); its rows carry `answered_by` and
`decided_option` (`ess/domains/messaging.yaml`, view `inbox.messaging.Thread`, lines 1078-1108).
An answer taken on silence waits there until `keep_until`, the answer retention of the service
(24 h by default, `website/docs/status.md`, "The service's defaults"), or until the sender
collects it.

## Domain relations

- A repost keeps the message id: "A sender that posts a message again sends the same id"
  (`ess/domains/messaging.yaml`, newtype `inbox.messaging.MessageId`, line 32). Inferable from
  the ess/1 document.
- `Expire`: a decision in `Sent`, `Delivered` or `Read` with `default_on_silence` moves to
  `Answered` with `answered_by: silence`, `decided_option` its `default_on_silence` and `expires`
  set to `keep_until`; every other due message is deleted with `MessageExpired` (same file,
  command `inbox.messaging.Expire`, outcomes `decided-on-silence` and `expired`, lines 861-902).
  Inferable from the ess/1 document.
- The `Thread` view holds every live message of a thread (same file, view
  `inbox.messaging.Thread`, filter `thread == param.thread`, line 1082), cut to the caller's rows
  by the read rule (`docs/design/service.md`, "The read rule"). Inferable from the ess/1
  document and the design.
- `ChatPost` to Message: at most one chat post per decision, from the epic ("asked once"). For a
  note and a request, and for how long a `ChatPost` outlives its message, no source says:
  `UNMAPPED:` in `docs/design/bridge.md`, answered by Open 1.

## Open

1. What "once" binds, and how long the record keeps an entry. (A) one chat post per message id,
   for every kind, while the record holds the id; an entry is dropped when its message is gone
   from the service and its `expires`, plus a retention the bridge is given by a flag (default
   24 h, the default answer retention of the service), has passed; (B) the same, kept forever
   (the record grows); (C) once per decision only, a note or a request may be posted again after
   a restart. Default: A.
2. What the bridge writes at the end of a message nobody answered: an addition, which the epic
   does not ask for (it promises only that the person is asked once and that the service answers
   by silence). (A) one reply in the thread of its chat post: for a decision taken on silence the
   option, its text and "taken on silence"; for a message that expired without an answer
   "expired, not answered"; (B) an edit of the chat post instead of a reply (not verified that
   the connector has such an operation); (C) nothing, the letter of the epic: check 6 then falls
   away. Default: A.
3. How the bridge learns how a message ended. (A) at each poll it reads the `Thread` view of each
   chat post whose message left `Pending` and `Unanswered` without an answer of the bridge, and
   takes `answered_by` and `decided_option` from the row; a row that is gone (collected by the
   sender, or removed) gives the line of an expired message only when its `expires` has passed,
   and else nothing; (B) it writes the line at `expires` from the envelope alone, without the
   word of the service: wrong when the person answered in a terminal. Default: A.

## Scope

After story:the-chat-bridge-marks-a-message-read-on-a-reaction (edge `depends_on`: it restores
the read mark of that story after a restart, and edits `bridge.rs` after it). After it come
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal (it shares `main.rs`,
`reference.rs` and `generated/cli/inbox.json` with this story),
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation and
story:the-chat-bridge-turns-a-thread-reply-into-an-answer, which edit `bridge.rs` after it.

Code:

- `crates/inbox-cli/src/bridge.rs` (the marks without a post, the end of a message, the
  retention of the record): inferred
- `crates/inbox-cli/src/commands.rs` (`thread`): cited
- `crates/inbox-cli/src/outbox.rs` (dropping entries of the record under the outbox lock):
  cited
- `crates/inbox-cli/src/main.rs`, `crates/inbox-cli/src/reference.rs`,
  `generated/cli/inbox.json` (the flag of the retention, and `--clock-file` of `inbox bridge`,
  hidden and of a debug build only; this story introduces the clock of the bridge, no earlier
  bridge story reads one): cited
- `crates/inbox-cli/src/clock_file.rs` (`FileClock`, read by the bridge as by the service):
  cited
- `crates/inbox-cli/tests/bridge_once.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred

Documents:

- `docs/design/bridge.md`: a section on once and on silence: inferred
- `website/docs/first-use.md`, `website/docs/limits.md`: the section of the bridge, and what a
  restart does to it: cited
