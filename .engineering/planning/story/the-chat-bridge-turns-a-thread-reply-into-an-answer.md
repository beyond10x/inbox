---
format: aep.planning-md/3
id: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
kind: story
status: implemented
title: The chat bridge turns a thread reply of the allowed chat user into a signed answer
summary: A thread reply to the chat post of a request or a decision is sent as Answer on behalf of the person and moves the message from Read to Answered with answered_by principal.
relations:
- decomposes: epic:human-inbox
- depends_on: story:the-chat-bridge-asks-a-decision-once
- depends_on: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: crates/inbox-cli/src/bridge.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/output.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests
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
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:45Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-24T08:37:18Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-24T10:30:21Z", actor: "agent:inbox", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":20}}}
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
- `crates/inbox-cli/src/outbox.rs` (the record of a chat post gets the reference of the
  last reply handled and the line owed for it, and the state answered): cited
- `crates/inbox-cli/tests/` (the stand-ins of `bridge_once.rs`, `bridge_read.rs`,
  `adversary_bridge_once.rs` and `adversary_bridge_read.rs` answer a read of the replies
  of a post; the forwarder of `bridge_once.rs` drops one request when a case asks for
  it): cited

Documents:

- `docs/design/bridge.md`: a section on the answer: inferred
- `website/docs/first-use.md`: the section of the bridge, how to answer in the chat channel:
  cited
- `website/docs/status.md`: the row of the bridge in "Available", the planned item "An inbox for
  a person" taken out of "Planned": cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`: cited
- `website/docs/limits.md` (what the bridge does not do with a thread reply) and
  `CHANGELOG.md` (the bullet of the story): cited

## Decisions

- The open items are answered at their defaults (the operator accepted the 7 stories of the
  stage with every open item at its default, 2026-11-23): the options of a decision are lines
  of the chat post (1 A); the match is the letter of an option, alone or before a space, a
  colon or a full stop, and a reply that matches none sends no `Answer` and gets one line
  that names the letters (2 A and A); a message still in `Delivered` is marked read first (3
  A); the first matching reply of the allowed chat user counts (4 A). Open 5 was moved to the
  story before, and Open 6 was settled by the operator's answer B.
- The story was built beside the last steps of the story before it, on that story's local
  branch. The options were: (A) start, push nothing of this story before the story before is
  on main, and resolve conflicts in this story's tree only; (B) wait for the story before on
  main. A is taken. The one commit of the story before was merged in after the round: 3 files
  conflicted (`AGENTS.md`, the paragraph of the cases in `docs/design/bridge.md`, the
  forwarder and the doc of the stand-in in `crates/inbox-cli/tests/bridge_once.rs`), resolved
  in this story's tree.
- Who signs. A bridge with `--for` and `--authority` sends the reply with `from` its own
  principal, `on_behalf_of` the `--for` address and `authority` the delegation id, under the
  header. A bridge without `--for` holds a key of the person and answers as `inbox answer`
  without flags does: as the person, with no `authority`.
- How the bridge learns of a thread reply: the poll of the chat channel reads the replies of
  each post whose message is still live, one child `connectors --output json invoke <target>
  conversations.replies --input-stdin` per post per poll, with the input `{"channel", "ts",
  "oldest", "limit": 100}`, one page. The posts of notes are read too, since a reply to a
  note gets a line. The operation and its input are not verified against `connectors` or the
  provider; the design says so.
- What the bridge did with a reply is kept in the record of the chat posts: the reference of
  the last reply handled (`replied`) and the line owed for it (`owed`). The record is written
  first, then the line is posted under the key `inbox-<message id>-reply-<reply ts>`, then
  `owed` is dropped; a line that failed is sent again at the next poll under the same key. A
  reply that was taken gets no line.
- The order for one reply: the message is already answered, the reply is too long, the
  message is a note, no option matches, else the `read` while the message is `Delivered` and
  then the `Answer`.
- A second matching reply after the first was taken gets the line of `already-settled` from
  the record of the bridge, and no second `Answer` is sent. Check 8 words it as an answer of
  the service; the line the person reads is the same.
- An answer `401`, `408` or from `500` on, and a lost connection, fail the poll, and the
  `Answer` is sent again at a later poll. Every other refusal gets one line that names it and
  is not sent again. A `401` to a bridge with `--for` is the end of the story before.
- The text of a reply is data: it is sent as the provider hands it, the escapes of the
  provider are not undone, and no line of the bridge quotes it.
- `website/docs/first-use.md` is unchanged, where the story's Scope names it: the page gets
  the bridge with the story of the real run. The item of `website/docs/status.md` stays in
  Planned with its text extended: the page claims no run on a real channel.
- A failed read of the replies of one post is that post's alone (adversary pass 1, finding
  1: one thread whose read failed stopped every answer of the channel at every poll). The
  posts after it are read and handled in the same poll, the poll does not count as failed,
  and one line is written per run of failures of that post. The read before an `Answer`, an
  `Answer` without a usable answer and a change of the record that fails still fail the
  poll: they are failures of the service or of the host, the same for every post.
- Where the next read of a thread starts (adversary pass 1, finding 2: after a full page of
  replies of other chat users the same page was read for ever). The options were: (A) the
  record of a post keeps `read_to`, the reference of the last message of the thread the
  bridge read, whoever wrote it, and every read starts after it; (B) the bridge follows the
  cursor of the provider through every page at each poll and writes nothing. A is taken: B
  costs one more child per 100 messages of a thread at every poll. Check 6 says a reply of
  another chat user "writes nothing"; that is read as no line and no request, and the mark
  in the bridge's own record is this decision. `read_to` is a member of its own beside
  `replied`, so the key of a line stays tied to its reply.
- A reply to a request that is empty or only white space, such as a shared file without
  text, sends nothing and gets one line of its own (adversary pass 1, finding 3). An empty
  reply to a decision gets the line that names the letters, one to a note the line of a
  note, as before.
- An `Answer` the service took whose answer was lost: the next poll finds the message out
  of `Pending` and ends the entry `answered`; 1 `Answer`, no line in the thread (adversary
  pass 1, finding 4, which measured it; 2 sentences said otherwise and are corrected).
- The lines the bridge itself writes into a thread. When the connection of `connectors`
  posts as the allowed chat user, the read of the replies hands them back as replies of that
  user, and round 2 measured an `Answer` sent with the text of the bridge's own line. The
  options were: (A) the record of a post keeps the references the provider returned for the
  bridge's own thread lines (`own_lines`), and a message with such a reference is skipped;
  (B) a limit: the connection must post as another chat user than the allowed one. A is
  taken: the post operation returns the reference. A reference is dropped once `read_to`
  passed it. Not verified against the provider.
- An `Answer` that got no answer of the service the bridge can act on (a lost connection, an
  answer without an end, `408`, `500` and on) gets a line of its own on standard error, once
  per run of such failures of the message (adversary pass 2, finding 1: the line of the
  failed read of the channel said "not taken" and "sent again" for an `Answer` the service
  had taken). The line says that whether the service took it is not known and that the next
  poll sends it again unless the message has left `Pending`. The poll still counts as failed
  and takes the pause; the line of the failed poll is not written for it and names the first
  other failure of a run.
- A read of the replies refused with a wait the provider named (`rate_limited` with
  `retry_after_seconds`) ends the reads of that poll (adversary pass 2, finding 2: the read
  of the next thread was fired at once, inside the wait). The options were: (A) no further
  thread is read in that poll; the posts after it are read at the next poll, after the wait;
  (B) the other threads are read in the same poll, as after any failed read. A is taken: the
  provider's limit is taken to count calls of the method, not of one thread (not verified).
  The cost: a post whose reads keep being refused with a wait holds the posts after it for
  as long. A read that fails without a named wait stays that post's alone.
- A line of the record that names no `channel` (a record of a build before the member
  existed) is read as a post of the bound channel, as before this story; a case holds it now
  (adversary pass 2, finding 9, pre-existing).
- The id of a post leaves the 2 in-memory sets of failing reads and failing answers when its
  entry ends or is dropped. No case: the effect is a line that is written again after a
  repost.
- Round 3 was not read by a third adversary pass: it changes 2 lines of standard error and
  one early return of the poll, each with a case, and 9 of 10 mutants are red.

## Measured

- Red first: the 4 cases of round 1 against the client before the story: `4 tests run: 0
  passed, 4 failed`; after it `4 tests run: 4 passed`. The cases of `inbox-cli`, 2 runs in a
  row: `409 tests run: 409 passed, 0 skipped` each.
- Each check and its case, in `bridge_answer` unless named: the acceptance and checks 2, 3
  and 4 `a_thread_reply_moves_a_decision_from_read_to_answered_with_the_persons_option` (a
  bridge with its own key under `--for`: the text of the decision's post, `Read` to
  `Answered` with `answered_by: principal` and the option of the reply, the envelope `from`
  the bridge with `on_behalf_of` and `authority`, the `read` first for a message still in
  `Delivered`, the reply to a request sent as its body, the sender's `inbox collect`);
  `a_bridge_with_the_persons_key_answers_as_the_person`; checks 5, 6 and 8
  `a_reply_that_is_not_taken_gets_one_line_and_sends_nothing` (no option, a note, over 65536
  bytes, another chat user, a second matching reply; after a new start of the bridge no
  second line and no second `Answer`); check 7
  `a_refused_answer_gets_one_line_and_a_lost_one_is_sent_again`; check 1: the noun
  `ChatReply` and the state `answered` are in `docs/design/bridge.md`, and `grep -rn UNMAPPED
  ess` prints nothing. Refusals other than `deadline-passed` and `shape-mismatch` have no
  case of their own: they take the same path.
- Mutants of round 1, 15, each red in a named case: another chat user's reply taken; a reply
  with no matching option sent; the `read` before the `Answer` left out; a second `Answer`
  for a second matching reply; the record's `replied` not read back; no `on_behalf_of`; no
  `authority`; `from` the person under `--for`; a refused `Answer` sent again; a second
  thread line after a new start; a failed transport not tried again; a reply over the bound
  sent; the letter of an option matched anywhere in the text; a reply to a note sent; the
  post of a decision without its options.
- Adversary pass 1 (review-result:adversary-bridge-answer-pass-1): 7 findings, 3 red cases:
  one post whose thread read fails failed the whole poll at every poll
  (`a_thread_whose_read_fails_stops_no_answer_in_another_thread`); after a full page of
  replies of other chat users the same page was read for ever
  (`a_full_page_of_other_users_does_not_hide_the_persons_reply_for_ever`); a reply without
  text to a request was sent as an empty `Answer`
  (`a_reply_without_text_is_not_sent_as_an_empty_answer`). 2 sentences on a lost answer were
  wrong against its measurement (1 `Answer`, no line). 3 mutants of its own that every case
  let live: `408` dropped from the answers tried again; the channel filter of the answers
  always true; a repost of an answered entry not made `read` again. 5 mutants of round 1 run
  again, all red. The cases of `bridge_answer` 5 times in a row and the modules of the
  bridge with its own key 10 times: no red run.
- Round 2 on those findings (all 7 recorded as fixed): the 3 red cases before the round: `3
  tests run: 0 passed, 3 failed`; after it the cases of `inbox-cli`, 2 runs in a row: `427
  tests run: 427 passed, 0 skipped` each. 6 cases and 1 unit case added:
  `a_reply_after_a_full_page_of_other_users_is_read_at_the_next_poll` (the adversary's case
  of the full page stays green but no longer holds the fix once every read names where it
  starts; this one does), `an_answer_taken_whose_answer_was_lost_ends_the_entry_with_no_line`
  (the adversary's fourth case, taken over: the end `answered`, 1 `Answer`, 0 lines, the
  collected body), `an_answer_answered_408_gets_no_line_and_is_sent_again`,
  `an_entry_of_another_channel_is_not_read`,
  `a_repost_of_an_answered_message_takes_a_later_reply_again`,
  `the_bridges_own_line_read_back_as_the_persons_reply_is_not_sent`, and the unit case of the
  record with `read_to` and `own_lines`. The case of the bridge's own line was written
  first and was red: an `Answer` was sent with the text of the line the bridge had posted.
  The filter of the adversary's channel mutant could not be reached (the poll had dropped
  the entries of other channels before); both steps now use one filter, and its mutant is
  red.
- Mutants of round 2, 8, each red in a named case: a failed read fails the poll again;
  `read_to` never moves past a page; a reply without text is sent; the bridge's own lines
  taken as replies; `408` dropped; the channel filter always true; the repost of an answered
  entry not made `read`; the row `Answered` live for any entry.
- A case of round 2 was red once, in the first run of `task check` of the round: `1088 tests
  run: 1087 passed, 1 failed`, the case of the repost, at its wait "not within 10 s". The
  wait read the person's `Pending`, which no longer lists the reposted message once the
  bridge has marked it read, within milliseconds. The wait now reads the sender's outbox.
  After the change: the cases of `bridge_answer` and `adversary_bridge_answer` 5 times in a
  row, `5/5 stress run iterations: 5 passed`, and the mutant of that case red again.
- Adversary pass 2 (review-result:adversary-bridge-answer-pass-2), on round 2: 9 findings, 4
  cases added, 2 of them red: `431 tests run: 429 passed, 2 failed`. An `Answer` the service
  took whose answer was lost was named on standard error as a failed read of the reactions,
  "not taken" and "sent again", while 1 `Answer` was carried and the entry ended `answered`
  (`an_answer_the_service_took_is_not_named_as_not_taken_and_sent_again`). After a read of
  the replies refused `rate_limited` with a wait of 1 s the poll read the next thread at
  once (`a_rate_limited_read_of_one_thread_fires_no_read_of_another_thread_inside_its_wait`).
  The case of the full page of round 2 was red once: it waited on the sender's outbox and
  then read `read_to` from the record, which the bridge writes later. 8 mutants: 1 red in a
  case of the story, 1 equivalent (no `read` at the repost of an answered entry: the poll's
  `read` does the same), 4 that every case of the story let live and the 2 green cases of
  the pass kill (`own_lines` never pruned; the read starts after `replied`, not `read_to`;
  a line at every failed read; the set of failing posts never cleared), 2 that lived (the
  named wait of a refused read of the replies dropped; a line of the record without
  `channel` dropped). Its stress run, 10 times, 70 cases each, without its 2 red cases:
  `10/10 stress run iterations: 10 passed`. It did not reach: the failure paths of the
  `read` sent first, the lines of `400` and `404`, an answer of a child that is very large
  or no JSON, a delegation that lapsed between 2 replies, 50 live posts.
- Round 3 on those findings (all 9 recorded as fixed): the 2 red cases before the round: `2
  tests run: 0 passed, 2 failed`; after it the cases of `inbox-cli`, 2 runs in a row: `434
  tests run: 434 passed, 0 skipped` each. The line of an `Answer` without an answer, as
  standard error has it: `inbox bridge: the answer to <message id> from the thread of its
  post got no answer of the service that the bridge can act on (<failure>), so whether the
  service took it is not known; the next poll sends it again unless the message has left
  `Pending` by then (no further line until the service answers an answer to it)`. 3 cases
  added: `a_read_of_the_replies_refused_with_a_wait_holds_the_next_call_for_that_wait` (a
  wait of 2 s named; the next call of the stand-in at least 2 s after the refusal, a lower
  bound only), `an_entry_that_names_no_channel_is_read_as_a_post_of_the_bound_channel`, and
  `an_answer_without_an_answer_fails_its_poll_and_the_posts_after_it_wait_for_the_next`: the
  fix of the line let the mutant "an `Answer` without an answer fails no poll" live (`19
  passed`), since the case that held it counted the line of the failed poll; with the new
  case it is red. The case of the full page waits for the state `answered` of the record
  before it reads `read_to`; no other case of rounds 1 and 2 reads the record right after a
  wait on the service. 10 mutants, 9 red in a named case: the 2 fixes, the named wait
  dropped, a line without `channel` dropped, the poll not failed, and the 4 of the 2 green
  cases of the pass. 1 lives: the ids not taken out of the 2 in-memory sets at the end of an
  entry (no case). The stress run, 10 times, 75 cases each: `10/10 stress run iterations: 10
  passed`. No adversary pass read round 3.
- The cases: 24: 13 in `bridge_answer` (4 of round 1, 6 of round 2, 3 of round 3), 7 in
  `adversary_bridge_answer` (3 of pass 1, 4 of pass 2) and 4 unit cases (3 of round 1, 1 of
  round 2). No case over 5 s: the slowest,
  `a_refused_answer_gets_one_line_and_a_lost_one_is_sent_again`, took 4.46 s in round 1; the
  cases of round 2 took 1.2 to 3.4 s, the 3 of adversary pass 1 3.1 to 3.7 s, 2 of the pass
  2 cases 4.1 s each, and of round 3 3.26 s, 3.06 s (it waits the 2 s it names) and 0.07 s.
- The test step of `task check` on the build node: 16 s in round 1 (1070 passed, 8
  skipped); with the one commit of the story before merged in 1078 passed, 8 skipped, exit
  0; after round 2 17 s and 18 s (1088 passed, 8 skipped: 10 more cases, 3 of adversary pass
  1 and 7 of round 2, of 1.2 to 3.7 s each); after round 3 19 s (1095 passed, 8 skipped);
  with `main` at 1608756 merged in 19 s (1127 passed, 8 skipped), exit 0. In that run the
  compatibility step of the gate compared `ess/` with 598a1ed, the commit the local branch
  forked from, and printed `compatibility: 0 breaking, 0 unknown, 3 compatible`: the 3
  summary sentences a story before this one changed; this story changes nothing under
  `ess/`. The run of `main` before the story took 16 s. The story adds
  24 cases of up to 4.46 s each; whether a case of before got slower was not measured.
- Not held by a case, and not built: a child `connectors` that hangs (no child of the bridge
  has a time bound; that was so before this story); a reply edited or removed after it was
  handled. The operation `conversations.replies`, its input and the reference the post
  operation returns are not verified against `connectors` or the provider: the run on the
  real channel is the next story.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T10:26:58Z to 2026-11-24T10:29:57Z, on the build
  node of the pool: exit 0, `1127 tests run: 1127 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
