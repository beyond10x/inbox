---
format: aep.planning-md/3
id: story:the-chat-bridge-asks-a-decision-once
kind: story
status: implemented
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:44Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-24T05:19:45Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-24T07:48:00Z", actor: "agent:inbox", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":13}}}
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

## Decisions

- The 3 items of Open are answered at their defaults (the operator accepted the 7 stories of
  the stage with every open item at its default, 2026-11-23): one chat post per message id,
  for every kind, while the record holds the id, and an entry dropped when its message is gone
  from the service and its `expires` plus a retention has passed, the flag
  `--chat-post-retention <seconds>`, default 24 h (1 A); one reply in the thread of the chat
  post at the end of a message nobody answered (2 A); the bridge learns how a message ended
  from the `Thread` view (3 A).
- The settle of the story before changes. There an entry whose message left `Pending` became
  `read`, one state for every way out. Check 3 of this story sends `MarkRead` for a reposted
  message whose entry is `read`, and a message lost by a new start of the service was never
  read by the person. So an entry is `read` only when the bridge knows it (the service
  answered the `read` of a reaction, or the row is in `Read`), and an entry now has a second
  member, its end: `answered`, `decided`, `expired` or `gone`. The case of the adversary of
  the story before that stated the old settle
  (`adversary_a_restart_on_another_channel_and_back_asks_about_the_first_channel_again`) is
  changed to the new one.
- A repost is held once per message id, not once per channel: an entry of another channel
  than the bound one counts as held, and the bridge bound to this channel posts nothing and
  still marks the message. The limits page says so.
- The reply in the thread is `chat.postMessage` with `channel`, `text` and `thread_ts` under
  `body` and the key `inbox-<message id>-end`, which differs from the key of the post and is
  the same at every try. Its shape is taken from the session of `connectors` (its contract
  and tests at 0.77.0); no call was run. Not verified: that the provider shows the message in
  the thread, the class of a failure only a reply has, and a reply under a standing grant.
  The order is the reply, then the record change: a bridge ended between the two sends the
  reply again with the same key.
- The texts of the 2 lines: "taken on silence: option <letter>, <text>" with the escapes of a
  post, and "expired, not answered". A message the person answered or refused gets neither; a
  refused message ends as `answered` (adversary pass 1, a mutant no case killed).
- A decision taken on silence that its sender collected before the bridge looked gets the
  line of an expired message: the row is gone and `expires` has passed, and the bridge cannot
  tell the two apart. The design and the limits page say so.
- The clock of the bridge: `inbox bridge --clock-file <PATH>`, a flag of a debug build only,
  hidden from `--help` and the reference, read as the service reads its own; the bridge still
  signs with the system clock.
- A `read` lost on a repost (adversary pass 1, finding 1; check 3 failed). Options: A the
  poll sends the `read` again while the entry is `read` and `Pending` lists the message in
  `Delivered` (taken); B the sentence of round 1, "not sent again", which leaves a request
  the person read in `Delivered` for good.
- A record the rounds cannot read (adversary pass 1, finding 2). Options: A no post without
  a readable record: the bridge ends with 4, as a post that cannot change the record does
  (taken); B the round reads it as empty and relies on the key of `connectors` to replay the
  post, which gives a second post operation against check 2. The case of the first bridge
  story `a_post_that_cannot_be_recorded_is_not_marked_delivered` now counts no post
  operation.
- An entry without `expires`, written by a build of the stories before, which is not
  released (adversary pass 1, finding 3). Options: A it is held as every other entry, the
  record gets the instant at which the bridge first found its message gone, and it is
  dropped after the retention from that instant, without the line of an expired message
  (taken); B the first decision, dropped at once, which posts the message a second time at
  the sender's repost; C the second post named as a limit.
- A line of the record that is no entry (adversary pass 2, finding F-a: the read skipped
  it, so its entry was not held and the message was posted a second time). Options: A the
  record counts as one that cannot be read: the rounds end the bridge with 4 and post
  nothing, the poll fails with one line per run of failures, and no change rewrites the
  record, so no line is dropped (taken); B the read skips the line and the design names the
  second post as a limit. An entry is a JSON object with the texts `message_id`, `reference`
  and `state`; a line of an earlier build with fewer members reads; an empty line is no
  entry; a last line without its line end reads. The bridge writes the record whole and
  renames it into place, so it writes no cut and no empty line itself.
- The resend of a lost `read` (adversary pass 2, finding F-b) is held by a unit case of the
  function `lost_reads`, not by a count through the forwarder: a request in `Read` is not
  listed in `Pending`, so no case over the service tells the widened resend from the right
  one.
- A reply that fails: the bridge stays up, writes one line per run of failures, sends the
  reply again at a later poll with the same key and the same input, and changes the record
  only after the reply went through. Held by a case since round 3.
- The clock file of the bridge at the start (missing, a directory, empty, no instant): the
  bridge opens it through the same `FileClock` as the service and ends with 4. Round 3
  added no case, on the case of the service
  `clock_file::a_clock_file_that_holds_no_instant_stops_serve_before_it_binds`; adversary
  pass 3 showed that this case holds the path of the service only (a bridge that falls back
  to the system clock survived it) and added a case of the bridge for an empty clock file at
  the start; its case of pass 2 holds "removed while the bridge runs".
- The wait of the adversary's case on a held line that does not read: round 3 left it red
  (it waited for a second post or a delivery mark, and the decided behaviour sends neither),
  and the implementor proposed a patch it did not apply. Options: A the adversary decides the
  wait of its own case (taken: it now ends on a second post, a delivery mark or the end of
  the bridge, and holds exit 4, no delivery mark and the message still `Sent`); B the
  coordinator applies the implementor's patch to the adversary's file.
- `website/docs/first-use.md` (in the Scope of the story) is not changed. Options: A the page
  gets the bridge with the story of the real run
  (story:the-operator-runs-the-chat-bridge-on-the-real-channel), since no run against
  `connectors` was made (taken); B written now from the contract.
- `connectors` 0.77.0, which has the standing grant, was released while this story was built
  (2026-11-24). The sentences of the design, the limits page, the status page and the
  changelog that said the grant is not released are corrected: the bridge needs `connectors`
  0.77.0 or later and one standing grant of the operator for the channel. Every statement
  about the answers of `connectors` stays marked as not verified.

## Measured

- Red first, round 1: the 2 cases of `bridge_once` against the build before the story, on
  the implementor's run: `2 tests run: 0 passed, 2 failed, 359 skipped` ("the 3 are marked as
  before the restart: not within 10 s"; the second was red because the build before refuses
  the 2 new flags, and is red on each behaviour under the mutants below). Round 2: its new
  and changed cases against the source of round 1, `6 tests run: 2 passed, 4 failed, 359
  skipped` (the 3 cases of the adversary and the case of the retention; the 2 that passed
  are red under the mutants of round 2). Round 3: the case of a line of the record that is
  no entry against the source of round 2, "the poll fails on the record: not within 10 s".
- Mutants of the 3 rounds, each red in a named case of `bridge_once`, `bridge`,
  `adversary_bridge_once` or the unit cases of `outbox` and `bridge`: 13 of round 1 (m1 to
  m12 and m6b: a repost posted again, `MarkRead` for an entry not read, no `MarkRead` for a
  read entry, a new bridge that posts a held message, the bridge answering at `expires`,
  the line of silence twice, twice after a new start, a line for an answered message, the
  line of an expired message for one gone before `expires`, an entry dropped before
  `expires` plus the retention, never dropped, the key of the reply equal to the key of the
  post, the reply without `thread_ts`); 10 of round 2 (no resend of a lost `read` by the
  poll, no `read` at a repost at all, the `read` of the repost removed, a record that cannot
  be read taken as empty, an old entry dropped at once, never dropped, the end not cleared
  at a repost, an entry held only when it is of the bound channel, `Refused` not an end, the
  text of the line of silence without its escapes); 6 of round 3 (the resend widened to
  every read entry, a cut line skipped, an empty line skipped, the check before a change of
  the record removed, an entry marked as written although its reply failed, another key at
  the second try of a reply).
- Adversary pass 1 on round 1 (review-result:adversary-chat-bridge-once-pass-1), in a tree
  of its own on the build node: 3 cases added, all 3 red, 7 findings (a `read` lost at a
  repost is never sent again; a record the rounds cannot read is taken as empty and the
  message is posted twice; an entry of the build before is dropped at once and posted
  again; case A red under load on the line of the reconnect; 3 mutants no case of the story
  killed). All 7 fixed in round 2.
- Adversary pass 2 on round 2 (review-result:adversary-chat-bridge-once-pass-2): 3 cases
  added, 1 red, 2 findings (a line of the record that does not parse hides its entry and the
  message is posted twice; the mutant that widens the resend of a lost `read` survives).
  Both fixed in round 3. The 3 cases of the story and the 6 of the adversary at that time,
  run 10 times in a row: `10/10 stress run iterations: 10 passed`. Not reached by the pass
  and not held by a case: a poll with 50 entries, a repost while a post of the same message
  runs, a new start of the bridge between a post and its record change, the text of a reply
  at the bound of a post's text.
- Adversary pass 3 on round 3 (review-result:adversary-chat-bridge-once-pass-3): no finding.
  It gave its case on a held line that does not read a wait of its own (a second post, a
  delivery mark or the end of the bridge) and the claims of the decision (exit 4, no
  delivery mark, the message still `Sent`), red under the mutant that skips a cut line; and
  it added 1 case, green: an empty clock file of the bridge at the start ends it with 4
  before any post, the only case red under a bridge that falls back to the system clock. 6
  mutants, each red in a named case. `bridge_once` and its file, run 10 times in a row:
  `10/10 stress run iterations: 10 passed`. Not reached: a run of a reply that fails at
  every try, and of one answered `rate_limited` (read from the code: 1 reply operation per
  poll and entry, 1 line per run of failures).
- The cases: 15 added (5 in `bridge_once`, 7 in `adversary_bridge_once`, 3 unit cases of the
  binary). No case over 5 s: the slowest of the story in the gate run below is
  `bridge_once::a_message_posted_again_after_a_restart_is_marked_and_never_posted_twice` at
  3.3 s. The 2 cases of round 3, run 10 times in a row with its 3 unit cases: `10/10 stress
  run iterations: 10 passed`, at most 1.3 s and 3.4 s.
- The gate runs of the rounds on the build node (`task check`): round 1 983 passed, 8
  skipped, the test step 14 s, exit 0; round 2 987 passed, 8 skipped, 19 s, exit 0; round 3
  stopped at its first failure (the adversary's case that waited for a post the decided
  behaviour does not make). The 19 s of round 2 was 5 s over round 1: adversary pass 2
  measured the cases of `inbox-cli` alone at 8.7 s, not slower than before, and the run on
  the final tree below, with 9 more cases than round 2, took 16 s. No part of the step is
  named as grown.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T07:46:35Z to 2026-11-24T07:47:07Z, on the build
  node of the pool: exit 0, `996 tests run: 996 passed, 8 skipped`, the test step 16 s of
  its bound of 120 s.
