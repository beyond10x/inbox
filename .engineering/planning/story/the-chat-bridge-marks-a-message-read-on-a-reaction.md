---
format: aep.planning-md/3
id: story:the-chat-bridge-marks-a-message-read-on-a-reaction
kind: story
status: implemented
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
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:44Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-24T00:38:50Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-24T05:06:43Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":20}}}
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

## Decisions

- The 4 items of Open are answered at their defaults (the operator accepted the 7 stories of
  the stage with every open item at its default, 2026-11-23): any reaction of the allowed chat
  user on the chat post reads the message (1 A); a reaction of another chat user does nothing
  (2 A); a thread of the bridge asks `connectors` at an interval, the flag
  `--chat-poll-interval <seconds>`, default 10, at least 1 (3 A); one read of the recent posts
  of the channel per poll, matched against the record by the reference of the post (4 B).
- The read is `connectors --output json invoke <target> conversations.history --input-stdin`
  with the input `channel`, `oldest`, `inclusive` true and `limit`, up to 10 pages by `cursor`.
  Its shape is taken from the session of `connectors` (its contract and tests at 0.76.1); no
  call was run. Not verified against the provider: that the `reactions` of a message are
  handed through, and the provider's rule for `oldest` with `inclusive`. The run on a real
  channel is story:the-operator-runs-the-chat-bridge-on-the-real-channel.
- The time of the cases: the interval has a floor of 1 s by the story, so a case that needs 2
  polls takes seconds. Options: A accept it and keep the floor (taken; the 10 cases of the
  story sum to about 30 s and run in parallel, the test step took 16 s on the build node); B
  an interval under 1 s for the cases, which changes the flag the story names.
- An entry of the record whose message left `Pending` without a reaction (`inbox read`, an
  answer, its deadline) would stay `posted` for ever, `oldest` would never move, and after 10
  pages every poll would fail. Options: A fixed in this story (taken): each poll reads the
  whole `Pending` once and settles every `posted` entry whose message is not listed, without
  a request and without a line; B a named limit and a later story.
- A record the poll cannot read. Options: A no open post and no line (the first decision); B
  a failed poll like any other, with one line per run of failures that names the record
  (taken after adversary pass 1, finding 7). A post that has to change the record still ends
  the bridge with 4.
- One `connectors` child at a time. Options: A a mutex inside the process, taken by a post
  and by a read (taken after adversary pass 1, finding 3; the lock of the wake already
  excludes a second bridge on the inbox); B a turn file beside the outbox file, compared by
  device and inode, which a removed file defeats.
- The record held no channel (adversary pass 1, finding 6). Options: A each entry records the
  channel it was posted in, the poll asks about and settles only the entries of the bound
  channel, and an entry without a channel (written by the build of the story before, which is
  not released) counts as of the bound channel (taken); B a named limit.
- A child started while the bridge ends (adversary pass 1, finding 4, there before this
  story). Options: A fixed here: the signal path keeps the guard of the slot until the
  process ends (taken; no case holds it, the window cannot be hit without a race on the
  scheduler, and the design says so); B a draft story.
- The `read` is sent at least once per reacted post, not exactly once: a record change that
  fails, or a bridge ended between the answer of the service and the record change, sends it
  again, and the service answers `already-read` or `unknown-message`. The line of a due
  message is written only once its entry is recorded `due`.
- A bridge started again with another `--chat-channel` does not see reactions on the posts of
  the channel before; the limits page says so.
- `website/docs/first-use.md` (in the Scope of the story) is not changed: the bridge cannot
  post through a released `connectors` yet (approval-record:d89-the-chat-bridge-posts-under-a-standing-grant-of-connectors),
  so the page of the first use gets its section with the story of the real run.
- Mutant Ml of adversary pass 1 (the park of a thread replaced by a return) got no case in
  round 3; pass 2 showed why none can be red: since round 3 the park is not reached.
- The park of a thread and its flag (round 3) cannot be reached: the signal path holds the
  guard of the slot until the process ends, so no thread finds the slot empty (adversary pass
  2, finding 3; 4 mutants on them change nothing). Options: A remove the flag and the park,
  keep one defensive return for an empty slot, and say so in the design (taken); B keep them
  as code no path runs.
- The line of a due message is written at most once: a bridge ended between the record change
  and the line never writes it (adversary pass 2, finding 4; the design says so).

## Measured

- Red first: the 6 cases of round 1 against a build with the flag and no poll, on the
  implementor's run: `6 tests run: 0 passed, 6 failed, 336 skipped`, each "not within 10 s" at
  its first wait (a read holds, the note is read, the request is read, the 3 posts are
  settled). Round 2: its 2 new cases against the source of round 1, `8 tests run: 6 passed, 2
  failed, 340 skipped`. The adversary's case
  `adversary_a_reaction_on_the_oldest_open_post_is_seen` was red against rounds 1 and 2 and is
  green since round 3.
- Mutants of the 3 rounds, each red in a named case of `bridge_read`, `bridge` or
  `adversary_bridge_read`: 8 of round 1 (m1 to m8: any chat user counts, a `read` at every
  poll for an entry already read, a `read` for an entry that is `due`, a failed poll that
  stops the poll, a failed poll that marks, a line at every failed poll, a poll and a post at
  once, a poll with no open post); 4 of round 2 (r1 to r4: the settle dropped, every `posted`
  entry settled, the settle after a failed read of `Pending`, `Pending` read before the
  record); 5 of round 3 (`inclusive` dropped, the comparison of the channel dropped, the mutex
  not taken by the poll, an entry made `read` when its `read` request failed, the line of a
  due message written before the record change).
- Adversary pass 1 on round 1 (review-result:adversary-chat-bridge-read-pass-1): 5 cases
  added, 2 red, 12 findings (1 blocker in the code: the read without `inclusive`; 1 blocker
  against the plan of round 2; 1 warning on the turn file; 4 notes; 5 on mutants no case of the
  story killed). All 12 have an outcome: 11 fixed, 1 without a case (the park of a thread
  replaced by a return; pass 2 found the park is not reached since round 3). Its case on the turn file went with the
  file; 4 of its cases stay, and 4 of its mutants (`retry_after_seconds` ignored, the bound of
  10 pages at 11 and at 9, the cursor not passed) are red only in them.
- Adversary pass 2 on round 3 (review-result:adversary-chat-bridge-read-pass-2), in a tree of
  its own on the build node: 3 cases added, none red, 4 findings (2 mutants no case of the
  story killed: an entry without a channel counted as of another channel, and the change of
  state dropping the channel of an entry, each red in a case the pass added; 2 notes: the
  park and its flag cannot be reached, and the line of a due message is at most once). 11
  mutants: 6 red in a named case, 4 that change nothing (the park and the flag), 1 alive (the
  guard released before the exit, which only a race could show). The 10 cases of the story
  and the 6 of the adversary at that time, run 5 times in a row: `5/5 stress run iterations:
  5 passed`. Not reached by the pass: a signal during the record change, the `read` request
  or the pause of a poll, and a second signal.
- Adversary pass 3 on round 4 (review-result:adversary-chat-bridge-read-pass-3): no finding, 1
  case added, green (a bridge started on a second channel and back on the first asks about
  the entries of the first again). The 6 cases that send a signal, run 5 times in a row:
  `5/5 stress run iterations: 5 passed`. Read only, no case: a signal during the record
  change, the `read` request or the pause of a poll, and a second signal.
- The cases: 23 added (10 in `bridge_read`, 8 in `adversary_bridge_read`, the rest unit cases
  of the binary). In the gate run below the 10 of `bridge_read` sum to 29.7 s and the 8 of the
  adversary to 9.2 s; they run beside each other. No case over 5 s: the slowest is
  `bridge_read::a_post_whose_message_left_pending_without_a_reaction_is_settled_and_asked_about_no_more`
  at 4.4 s (the interval of the poll has a floor of 1 s, and the case waits for several polls).
- The gate runs of the rounds on the build node (`task check`, exit 0): round 1 968 passed, 8
  skipped, the test step 16 s; round 2 971 passed, 13 s; round 3 977 passed, 16 s; round 4 980
  passed, 16 s.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T05:05:02Z to 2026-11-24T05:06:07Z, on the build
  node of the pool: exit 0, `981 tests run: 981 passed, 8 skipped`, the test step 14 s of
  its bound of 120 s.
