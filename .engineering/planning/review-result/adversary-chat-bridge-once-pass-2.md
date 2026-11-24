---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-once-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the chat bridge that asks once: 2 findings, 3 cases added, 1 red'
relations:
- reviews: story:the-chat-bridge-asks-a-decision-once
revision: 1
---
ADVERSARY PASS 2
unit: story:the-chat-bridge-asks-a-decision-once, round 2. Adversary tree <tree>-adv, detached at 869cded (the code is d148ab4).
verdict: CONFIRMED and INFEASIBLE notes only. Nothing holds the story back. The 4 fixes of round 2 hold against their claims.
cases: executed 365→368, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 22 paths, all under the scratch directory (part 6)
needs-coordinator: none

1. git --no-pager diff --stat
 crates/inbox-cli/tests/adversary_bridge_once.rs | 175 ++++++++++++++++++++++++
 1 file changed, 175 insertions(+)
This is a test file only, left uncommitted. Mutants were reverted: `cmp` against the saved originals of bridge.rs and output.rs matched.

2. New cases, appended to crates/inbox-cli/tests/adversary_bridge_once.rs. Their first run was alone, on build-1d:
- `adversary_a_post_whose_record_cannot_be_written_ends_the_bridge_with_4_and_stays_sent` (line 597). Green, 0.08 s (at most 0.30 s over 10 runs). This is the case the brief asked for: the claim of the first bridge story that round 2 left without a case.
  - Setup: bob's outbox sits in a directory of its own. The directory loses its write permission after the bridge's first round, so the record can be read and not changed. A guard gives the mode back at the end.
  - Asserts: exactly 1 post operation, the bridge exits 4, the line names the record (`.chat.`), and the note stays `Sent` in alice's outbox.
  - Result: the product still does what docs/design/bridge.md says for this path.
- `adversary_a_held_line_that_does_not_read_gives_no_second_post_at_a_repost` (line 635). Red, 0.12 s.
  - Setup: the held entry's line is cut 2 bytes short. The service restarts and alice reposts.
  - Red output: `assertion left == right failed: a second post operation of 01M866MK9QP707MYA872ZCNA0H: inbox: reconnect at the start ... left: 2 right: 1`
- `adversary_the_bridge_clock_file_removed_while_it_runs_ends_it_with_4` (line 670). Green, 1.03 s (at most 1.15 s).
  - Setup: the bridge's `--clock-file` is removed while the bridge runs.
  - Asserts: exit 4, and the line names `inbox bridge: the clock file` and the file.
  - It kills mutant o11 (FileClock::of(NAME) replaced by FileClock::open, so the line says `inbox serve`). No case of the story kills o11.
- clippy `-p inbox-cli --all-targets --locked -- -D warnings`: exit 0. The only warning is the old one in the generated `inbox` crate.

3. Runs after the cases existed
- Crate suite `SLOT_TIMEOUT=30m cargo nextest run -p inbox-cli --locked --no-fail-fast`, exit 100:
        FAIL [   0.106s] ( 41/368) inbox-cli::main adversary_bridge_once::adversary_a_held_line_that_does_not_read_gives_no_second_post_at_a_repost
     Summary [   8.746s] 368 tests run: 367 passed, 1 failed, 0 skipped
- Stress run `--stress-count 10` of `bridge_once` and my file, without the red held-line case: `Summary [36.671s] 10/10 stress run iterations: 10 passed`. Nothing was red, and no case went over 5 s. Max / mean per case:
  - A (posted again after a restart): 4.11 / 3.66 s
  - B1 (one line at the end): 4.06 / 3.47 s
  - retention: 3.38 / 3.20 s
  - adversary read-lost: 2.32 / 2.18 s
  - adversary old-entry: 2.36 / 2.21 s
  - adversary unreadable record: 0.85 / 0.47 s
  - adversary unwritable record: 0.30 / 0.15 s
  - adversary clock file removed: 1.15 / 1.07 s
- The 10 slowest cases of the crate run:
  - 4.80 s bridge_read::a_post_whose_message_left_pending_without_a_reaction_is_settled_and_asked_about_no_more
  - 4.63 s bridge_read::a_reaction_on_a_due_message_is_named_once_and_on_a_read_or_gone_one_is_not
  - 4.51 s bridge_read::a_failed_poll_marks_nothing_is_named_once_per_run_and_stops_no_post
  - 4.04 s adversary::an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires
  - 3.60 s bridge_once::a_message_posted_again_after_a_restart_is_marked_and_never_posted_twice (round 2)
  - 3.45 s bridge_read::a_reaction_of_another_chat_user_changes_nothing
  - 3.45 s bridge_once::a_message_nobody_answered_gets_one_line_at_its_end_and_none_in_the_persons_place (round 2)
  - 3.38 s bridge_read::a_reaction_of_the_allowed_chat_user_reads_a_note_once
  - 3.34 s bridge_read::a_request_read_by_a_reaction_stays_read_while_the_reaction_is_listed
  - 3.12 s bridge_once::an_entry_leaves_the_record_once_its_message_is_gone_and_the_retention_has_passed (round 2)
  - So 3 of the 10 are round 2's cases.
- The test step went from 14 s to 19 s, but this measure does not find it in inbox-cli: the crate's wall time was 8.7 s here, against 9.2 s and 7.7 s in round 2's own runs and 9.3 s in round 1. I timed no other crate, so I cannot say where the 5 s went.

Mutants (filter `test(/bridge/)`; my red held-line case is left out of "caught by"):
- o1, the end not cleared on a repost (`ended_before = false`): caught by bridge_once A, "the reposted messages are followed again: not within 10 s".
- o2, a held entry counted only for the bound channel: caught by bridge_once A, "the new bridge marks the 3: not within 10 s".
- o8, `Refused` not an end: caught by B1, "the answer, the read and the refusal are seen: not within 10 s".
- o9, the silence text not escaped: caught by B1, the reply-text assertion at bridge_once.rs:915.
- f4, an extra `inbox bridge:` line in `Chat::again` (the F4 check): caught by bridge_once A at :717, which printed the extra line. The new `not_reconnect` form still fails on a line it should not allow.
- o10, the F1 resend widened to every `read` entry whenever any row of Pending is `Delivered`: survived.
- o11, the bridge's clock line named `inbox serve`: caught only by my clock case.

4. Findings (all on 869cded)

F-a. A record line that does not read hides its entry, and the repost is posted a second time.
- Measured: adversary_bridge_once.rs:659, left 2 right 1. `Outbox::chat_posts` (outbox.rs:559, `filter_map(chat_post)`) skips the line. `Chat::deliver` (bridge.rs:306) then finds no entry and posts.
- bridge.md:325-330 says "No post without a readable record". A line that does not parse is a record that cannot be read for its message, and the list there does not name that form.
- What reaches it: nothing found. The bridge writes the record whole by rename; the cut line was built in the case. So the verdict is INFEASIBLE.
- Fix: the rounds fail closed on a line that does not read, as on a file that does not read. The other choice is to name this form in bridge.md as a known second post.

F-b. No case pins the F1 resend to the message that lost its `read` (mutant o10 survived).
- Measured: bridge.rs:1041, the filter `post.state == outbox::READ && delivered.contains(&post.message_id)`. Widened to every `read` entry whenever any row is `Delivered`, the bridge sends a `read` per poll per read entry. Each is answered `already-read`, and every case stays green.
- Fix lands in a case: count the `read` requests through the forwarder for one read message beside one that lost its `read`.
- The claim itself holds. A note read before the restart gets 1 `read` after the repost, or 2 when the poll's resend races the round's; the second is answered `unknown-message`, which is taken as read. A request past its `expires` and not yet swept gets one `read` per poll, answered `deadline-passed`, until the expiry task removes it. That is bounded by the service's sweep interval, and I make no finding of it.

5. Attacked and not broken
- F1: a lost `read` is sent again by the poll (pass-1 case green, 2.2 s). A failed resend fails only that poll, which is tried again after the pause. Case A's at-least-once `read` is bounded by the race described in F-b.
- F2: in every form I built (over its bound, a directory in its place, no write permission on the directory), the bridge ends with 4 and makes no post or a single post. It goes on only after a new start, as decided.
- F3: `gone_at` is in the record, so it survives a new start of the bridge. A clock moved back only delays the drop. The old entry never gets the expired line (pass-1 old-entry case green).
- F4: the filtered assertion still catches an extra line (mutant f4).
- o1, o2, o8 and o9 are each red in the case round 2 names.
- The changed case in adversary_bridge_read.rs: it now expects `gone` with the read mark `posted`. That is the new settle, no weaker; the no-`read` check stays.
- `--chat-post-retention`: clap rejects a negative value. A value over i64 falls back to i64::MAX, and `plus_seconds` saturates.
- The sentences: `connectors --version` on this host is 0.77.0, and `connectors approvals --help` lists `grant-standing`. bridge.md, limits.md and AGENTS.md mark every connectors and provider statement "not verified, no call was run". I found no sentence that claims a run.

Not reached:
- a reply refused, failed or naming `retry_after_seconds`, and the warn-once of `failing_end` across messages;
- reply text at the bound of a post's text, an empty option letter or text;
- a poll with 50 entries at their end and 50 open;
- a repost while a post runs; a new start between the post and the record change;
- check 5 with the forwarder for each state the design names;
- a clock file missing, empty or not a number at the start (only "removed while running" was run);
- the CHANGELOG bullets line by line.

6. Paths written outside the worktree, all in <scratch>/s4/once/adversary2/:
- new.log, stress.log, suite.log, clippy.log
- mutants.sh, mutants.txt, mutants.out
- bridge.rs.orig, output.rs.orig
- mutant-o1-end-not-cleared-on-repost.log, mutant-o2-held-of-bound-channel-only.log, mutant-o8-refused-not-an-end.log, mutant-o9-silence-text-unescaped.log, mutant-f4-extra-line-on-repost.log, mutant-o10-resend-for-every-listed-read-entry.log, mutant-o11-bridge-clock-named-serve.log

```findings
[
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 306, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A held entry whose record line does not parse is skipped by chat_posts, so the repost of its message gets a second post operation, against the design's 'no post without a readable record'; the cut line was built in the case."},
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 1041, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Widening the poll's resend of a lost read to every read entry whenever any message is Delivered survives every case, so no case bounds the read requests the F1 fix sends."}
]
```
