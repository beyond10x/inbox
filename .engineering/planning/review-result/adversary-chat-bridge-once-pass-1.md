---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-once-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the chat bridge that asks once: 7 findings, 3 cases added, 3 red'
relations:
- reviews: story:the-chat-bridge-asks-a-decision-once
revision: 1
---
ADVERSARY PASS 1
unit: story:the-chat-bridge-asks-a-decision-once, adversary tree <tree>-adv (detached at 08b0915, round 1 of the implementor)
verdict: NEEDS-CHANGE
cases: executed 361→364, red 3
origin: introduced 7 / pre-existing 0 / undecided 0
write-outside-worktree: 19 paths, all under the scratch directory (part 6)
needs-coordinator: whether finding 3 (an old entry is posted again) stands, since it follows the coordinator's default for entries without `expires`

1. git --no-pager diff --stat (plus 1 untracked file). Only test files changed.
 crates/inbox-cli/tests/main.rs | 1 +
 ?? crates/inbox-cli/tests/adversary_bridge_once.rs (new, uncommitted)
All mutants were reverted: `cmp` against the saved originals of bridge.rs and output.rs matched.

2. The cases, in crates/inbox-cli/tests/adversary_bridge_once.rs. Each has its own forwarder (it can drop one `read` request) and its own stand-in `connectors`. All three are red now. Their first run was alone (`-E 'test(/adversary_bridge_once::/)'`, build-1d):
- `adversary_a_read_lost_on_a_repost_leaves_the_request_delivered_for_good`. Statement 2 of the brief, check 3.
  - Setup: a request is read through a reaction. The service restarts and alice reposts. The forwarder drops the bridge's `read` connection once; its MarkDelivered passes.
  - Asserts: the request is in `Read` again within the hang bound. Red, 11.3 s (the 10 s bound):
    `assertion left == right failed: the request is in Read again; the bridge said: ... inbox bridge: 01M864GVVQ6ARGAEX64N3PGD7M was read before and is not marked read again: transport: 127.0.0.1:45351 (127.0.0.1:45351): the answer has no end of its head
      left: "Delivered"
     right: "Read"`
- `adversary_a_record_the_rounds_cannot_read_at_a_repost_gives_no_second_post`. Statement 1 of the brief, checks 2 and 4.
  - Setup: a decision is posted and recorded. The record is grown sparse past its 64 MiB bound. The service restarts and alice reposts.
  - Asserts: the stand-in records 1 post operation in all. Red, 0.15 s:
    `assertion left == right failed: a second post operation of 01M864FBEW6HCK5DXY5VEWNAS4 ... left: 2 right: 1`
- `adversary_an_entry_of_the_build_before_is_posted_again_after_a_restart_of_the_service`. Checks 2 and 4 for a record left by the build of story 3.
  - Setup: a request is posted. Its record line is rewritten without `thread` and `expires`, as story 3's build wrote it. The service restarts, a poll settles the entry, alice reposts.
  - Asserts: 1 post operation in all. Red, 1.1 s:
    `assertion left == right failed: a second post operation of 01M864FBEW6NJVZXHRFPRY5FAB ... left: 2 right: 1`
- clippy `-p inbox-cli --all-targets --locked -- -D warnings`: exit 0. The file is formatted with `rustfmt --edition 2024`.

3. The suite, after the cases existed: `SLOT_TIMEOUT=30m cargo nextest run -p inbox-cli --locked --no-fail-fast` (build-1d), exit 100:
        FAIL [   0.151s] ( 42/364) inbox-cli::main adversary_bridge_once::adversary_a_record_the_rounds_cannot_read_at_a_repost_gives_no_second_post
        FAIL [  11.271s] (364/364) inbox-cli::main adversary_bridge_once::adversary_a_read_lost_on_a_repost_leaves_the_request_delivered_for_good
        FAIL [   1.163s] ( 56/364) inbox-cli::main adversary_bridge_once::adversary_an_entry_of_the_build_before_is_posted_again_after_a_restart_of_the_service
     Summary [  11.390s] 364 tests run: 361 passed, 3 failed, 0 skipped
Stress run of the story's cases: `--stress-count 5 -E 'test(/^bridge_once::/)'` gave 5 of 5 passed (A 2.38 s, B 7.27 s). Under the load of the mutant runs, case A was red twice; see finding 4.

4. Findings (all on 08b0915)

F1. A `read` lost on a repost leaves the request `Delivered` for good. Check 3 fails.
- Measured: adversary_bridge_once.rs, the assertion in the read-lost case (red above). In `Chat::again`, crates/inbox-cli/src/bridge.rs:392-398 returns the failure after MarkDelivered passed. Nothing sends the `read` again:
  - the round never sees the row again, since it is `Delivered`, not `Sent`;
  - `Poll::once` keeps only entries in `posted` for the reactions, and the entry is `read`.
- Side effect: the early return skips the clearing at bridge.rs:401. An entry the poll made `gone` before the repost stays `gone` while its message is live, so no line is written at its end.
- docs/design/bridge.md:308 states "a `read` that failed after its `MarkDelivered` succeeded is not sent again". That sentence contradicts check 3.
- What reaches it: any transport failure on that one request, for example a dropped connection or a further restart of the service in that window. The case drops one connection.
- Fix lands in bridge.rs, `Poll::once`: for an entry `read` whose message Pending lists in `Delivered`, send the `read` of `inbox read` again. Also clear the end even when the read fails.

F2. When the rounds cannot read the record at a repost, the message gets a second post operation. Statement 1 holds as the implementor wrote it.
- Measured: bridge.rs:293 (`.unwrap_or_default()` in `Chat::deliver`). The stand-in records 2 post operations, and the bridge then ends with 4.
- Check 2 says "no second post operation". The design (bridge.md:310) instead relies on `connectors` replaying the post from the key. The outbox module doc (outbox.rs, record of the chat posts) says a record that cannot be read fails the read "instead of being read as holding nothing".
- What reaches it: nothing found. The case builds the state: a record over 64 MiB, which the retention makes unlikely, or a record that cannot be read. So the verdict is INFEASIBLE.
- Fix: fail the round with the failure that names the record, as the poll does, instead of reading the record as empty.

F3. An entry written before this story is posted again after a restart of the service.
- Measured: bridge.rs:1101 (`None => self.change(post, None)` in `Poll::ended`). After a restart, the first poll finds no row and drops the entry. The sender's repost then posts it a second time.
- What reaches it: a bridge upgraded from the story-3 build while it has live posts, then any restart of the service. The memory backend loses every message at a restart, and the 1 s to 10 s poll nearly always runs before the sender's next command.
- This follows the coordinator's default ("dropped once its message is gone"). Its effect on checks 2 and 4 is written nowhere.
- Fix: keep such an entry as `gone`, and drop it only after the retention counted from the bridge's clock. The other choice is to state the second post in bridge.md and limits.md.

F4. The story's case A fails at random under load.
- Measured: crates/inbox-cli/tests/bridge_once.rs:606 `assert_eq!(bridge.end(), "")`. It was red in 2 of 8 mutant runs, with left = "inbox: reconnect at the start <epoch> of the service: opened agent:test/bob again with the adapter slack (opened)\n".
- The 2 runs were mutants m9 (retention comparison) and o8 (the `Refused` arm). Neither touches case A's path. Logs: mutant-m9-drop-at-expires.log, mutant-o8-refused-not-an-end.log.
- The case was green 5 of 5 times alone. The bridge's reconnect line after a restart of the service depends on timing.
- Fix lands in bridge_once.rs: allow or filter the reconnect line.

F5 to F7. Mutants the story's cases let live (part "Mutants" below): o1 (bridge.rs:401), o2 (bridge.rs:296), o9 (output.rs:280). No case writes an option text that needs an escape into the silence line. No case checks that a repost clears `gone`. No case covers an entry of another channel at a repost. I wrote no case that kills them.

Mutants (bridge filter `test(/bridge/)`, 53 cases; my 3 red cases are left out of "caught by"):
- m2, MarkRead for an entry not read (`if true`): caught by bridge_once A ("the 3 are marked as before the restart").
- m8, the expired line for a message gone before `expires`: caught by bridge_once B, bridge_read reads_a_note_once and adversary_bridge_read restart_on_another_channel.
- m11, the reply key equal to the post key: caught by bridge_once B.
- m9, dropped at `expires` without the retention: caught by bridge_once B. Case A was also red here, from the F4 flake.
- o1, the `gone`/`answered` end not cleared on a repost: survived.
- o2, a held entry counted only when it is of the bound channel: survived.
- o8, `Refused` not an end: only case A went red, and only from the F4 flake. Treated as survived.
- o9, the option text of the silence line not escaped: survived. A `<!here>` in an option text would reach the channel as a mention.

5. Attacked and not broken, or not reached
- Retention flag overflow: read only. `i64::try_from` falls back to MAX, and `plus_seconds` saturates.
- `--clock-file` of `inbox bridge`: read only. Not in `generated/cli/inbox.json`, `#[cfg(debug_assertions)]`, hidden.
- The drop of an entry happens under the outbox lock (`change_chat_post`): read only.
- Not reached:
  - check 5 with a forwarder per entry state;
  - a reply that is refused, fails or names `retry_after_seconds`;
  - the reply text with control characters;
  - a reply for an entry of another channel;
  - the number of requests of a poll with many entries;
  - a clock file of the bridge that is missing, empty or not a number, and a release build;
  - the changed case in adversary_bridge_read.rs against the new settle;
  - the sentences of AGENTS.md, CHANGELOG.md, limits.md and status.md;
  - a repost while a post is still running, and a new start of the bridge between the post and the record change.

6. Paths written outside the worktree, all in <scratch>/s4/once/adversary1/:
- red.log, red-read.log, stress.log, clippy.log, suite.log
- mutants.sh, mutants.txt, mutants.out
- bridge.rs.orig, output.rs.orig
- mutant-m2-markread-always.log, mutant-m8-expired-line-before-expires.log, mutant-m11-reply-key-is-post-key.log, mutant-m9-drop-at-expires.log, mutant-o1-end-not-cleared-on-repost.log, mutant-o2-held-of-bound-channel-only.log, mutant-o8-refused-not-an-end.log, mutant-o9-silence-text-unescaped.log

7.
```findings
[
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 392, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A read that fails on a repost after MarkDelivered is never sent again, so a request the person read stays Delivered for good instead of Read (check 3), and its entry may stay gone while the message is live."},
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 293, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "When the rounds cannot read the record, it is read as empty and a reposted message the record holds gets a second post operation, against check 2 and the outbox doc's fail-closed rule; the state was built in the case."},
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 1101, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "An entry from the build before this story (no expires) is dropped by the first poll after a restart of the service, so the sender's repost is posted a second time."},
  {"file": "crates/inbox-cli/tests/bridge_once.rs", "line": 606, "category": "concurrency", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Case A expects an empty standard error from the bridge, but under load the bridge's reconnect line after the restart appears and the case goes red (2 of 8 runs, on mutants that do not touch its path)."},
  {"file": "crates/inbox-cli/src/output.rs", "line": 280, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Writing the option text of the silence line without its escapes survives every case, since no case uses an option text that needs one."},
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 401, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Not clearing the gone or answered end on a repost survives every case."},
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 296, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Counting a held entry only when it is of the bound channel survives every case, so a repost of a message posted into another channel is untested."}
]
```
