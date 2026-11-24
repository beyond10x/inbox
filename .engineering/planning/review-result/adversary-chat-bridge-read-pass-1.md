---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-read-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the chat bridge that marks read on a reaction: round 1'
relations:
- reviews: story:the-chat-bridge-marks-a-message-read-on-a-reaction
revision: 1
---
Adversary pass 1 on story:the-chat-bridge-marks-a-message-read-on-a-reaction, commit 3a93ddd. I attacked a copy at <scratch>/s4/read/adversary/copy and wrote nothing into the tree.

**Result:** 5 new cases. 2 stay red. 3 are green, and they kill 4 mutants the story's cases let live. One more mutant lives against every case. Reading found a blocker in the planned round-2 fix.

Cases file: <scratch>/s4/read/adversary/copy/crates/inbox-cli/tests/adversary_bridge_read.rs. Its `mod adversary_bridge_read;` line is in tests/main.rs, sorted. Clippy `-p inbox-cli --all-targets -D warnings` exits 0. The copy's target/ is deleted.

Last run: `11 tests run: 9 passed, 2 failed`. The story's 6 cases are green there.

## Findings

**F1, blocker: the oldest open post is never read.** Case `adversary_a_reaction_on_the_oldest_open_post_is_seen`, red in 10.2 s:
`the reaction on the only open post is never seen: the read input is "{\"channel\":\"C0000000001\",\"oldest\":\"1700000000.000001\",\"limit\":100}" (no "inclusive":true)`
- The provider's documented `conversations.history` returns only posts after `oldest`. The post at `oldest` comes back only with `inclusive` true.
- `read_input` (bridge.rs:995-1011) never sends `inclusive`. So a reaction on the oldest open post is never seen. With a single open post, nothing is ever marked.
- The stand-in in my case models those documented semantics. The story's stand-in ignores `oldest`, so its cases cannot see this.
- Fix: add `"inclusive":true` to `read_input`. The brief already names `inclusive` as a parameter `connectors` takes. Then update the exact-input assertions in bridge_read.rs and the input shown in docs/design/bridge.md.

**F2, blocker: the planned round-2 settle would settle every open post.** By reading. It hits brief-2's rule: "settle every `posted` entry whose message is not among the rows of that round".
- Every read of a round asks with `undelivered=1`. The service answers that with the rows in `Sent` that are not due (service.rs:314). `in_sent` (wake.rs:619) filters again.
- So a message already marked `Delivered` is never among the rows. Every `posted` entry would be settled at the next round, and no reaction would count after a second message arrives.
- Also, `held` (wake.rs:528) returns to the surface only when a row is in `Sent`. A "round" therefore happens only when a new message arrives, and nothing would be settled while the channel is idle.
- Fix: settle against a read of the full `Pending` view (no `undelivered=1`; I found no paging of that view in the service). For example, the poll thread reads it before its channel read and settles `posted` entries whose message id is absent.

**F3, warning: a turn file removed while held breaks the turn and stops the polls silently.** Case `adversary_a_turn_file_removed_while_held_keeps_one_child_at_a_time`, red in 11.4 s:
`a post ran beside the read: true; overlap: true; posts go on: true; polls go on: false; stderr` (empty)
- The turn is a `flock` on a path. After the file is removed, the post opens a new file there, takes its lock at once, and runs beside the held read.
- That post's `*held = Some(child)` in `invoke` (bridge.rs:474) overwrites the read's child in `wake::CHILD`. The read's child is dropped without a wait.
- The thread that then finds the slot empty parks for ever (bridge.rs:508-510). Here that was the poll thread: polls stop for ever, with no line. When the post thread parks instead, posts stop for ever.
- A turn held by any other process blocks posts and polls for ever, also with no line.
- Fix in bridge.rs:
  - make the turn an in-process mutex (the wake lock already excludes a second bridge), or, after `lock()`, compare the dev/inode of the open file with the path's and retry when they differ;
  - park only when a flag set by the signal path says it took the child, and check that the slot holds this thread's own child (by pid).

**F4, note: a child started as the bridge ends can outlive it.** By reading, wake.rs:664-668. The signal path drops the `CHILD` guard after `end_group` and before `emulate_default_handler` and `exit`.
- In that window the other thread can lock the slot and spawn a `connectors`. The child is in a group of its own, so it outlives the bridge.
- The window existed before this story, with only the rounds thread starting children. A second thread that starts children widens it.
- Fix: keep the guard locked until the process ends.

**F5, note: a failed record change makes the poll resend `read` until the change succeeds.** By reading, bridge.rs:881-892 and 918-925. When `mark_chat_post` fails after the service answered:
- every retry, after a pause of up to 30 s, sends `read` again. It is answered `already-read` or `unknown-message`, so it does no harm.
- for a due message, every retry writes the `Expired` line again.
- The same second `read` follows a bridge ended between the answer and the record change.
- The design and the CHANGELOG ("sends MarkRead once per post") do not say the read is at-least-once. Fix: say so in docs/design/bridge.md and the CHANGELOG, and write the due line only once the entry is recorded `due`.

**F6, note: the record holds no channel.** By reading, outbox.rs `ChatPost`. A bridge restarted with another `--chat-channel` asks the new channel with the `oldest` and references of posts made in the old one. The brief asked about a channel comparison; there is none. Fix: record the channel in each entry and poll only the entries of the bound channel, or name the limit in the design.

**F7, note: a record the poll cannot read stays silent until a message arrives.** By reading, confirmed: such a record ends the bridge with 4 only at the next post (outbox.rs `change_chat_posts`). Until a message arrives the bridge runs, marks nothing and writes no line, possibly for ever. Fix: one line from the poll on the first unreadable read, or name it in the design.

**Surviving mutants (warning, CONFIRMED):** the story's cases let 5 mutants live. My green cases kill 4 of them; one lives against every case:

| Mutant | Killed by | Green time | Fix lands |
|---|---|---|---|
| Ma: `retry_after` ignored in `Poll::run` | `adversary_a_rate_limited_read_waits_its_named_time_and_a_failed_one_its_pause` | 2.6 s | move this case into bridge_read.rs |
| Mc / Md: `MOST_PAGES` 11 / 9 | `adversary_a_poll_reads_ten_pages_and_fails_at_an_eleventh` | 1.6 s | move this case into bridge_read.rs |
| Mi: cursor not passed to the next page | same pages case | 1.6 s | move this case into bridge_read.rs |
| Me: entry made `read` when the `read` request fails | nothing | n/a | a case whose forwarder drops one `read` connection, then checks the entry is still `posted` and is marked at a later poll |

**Park mutant (note):** Ml replaces the park at bridge.rs:508 with `return Err(..)`. It survives every case, including my green `adversary_a_signal_during_a_read_ends_it_and_names_nothing` (1.2 s: SIGTERM during a held read ends the bridge with signal 15, the read's `connectors` is gone, no line). The race the park guards against is too narrow for a case to hit reliably.

## All mutants
Each run with the story's cases, my cases and `bridge::tests`. bridge.rs was restored afterwards (cmp ok).
- Ma, `retry_after` ignored in `Poll::run`: caught only by my rate-limit case.
- Mb, pause after a failed poll dropped: caught by my rate-limit case and the story's `a_failed_poll_marks_nothing_is_named_once_per_run_and_stops_no_post`.
- Mc, `MOST_PAGES` 11: caught only by my pages case.
- Md, `MOST_PAGES` 9: caught only by my pages case.
- Me, state written when the read request failed: survived everything.
- Mf, read sent with an idempotency key: caught by the story's `a_reaction_of_the_allowed_chat_user_reads_a_note_once`.
- Mg, `unknown-message` taken as a failure: caught by the story's `a_reaction_on_a_due_message_is_named_once_and_on_a_read_or_gone_one_is_not`.
- Mh, `deadline-passed` written as `read`: caught by the same due case.
- Mi, cursor dropped: caught only by my pages case.
- Mj, turn taken for posts only: caught by the story's turn, failed-poll and due cases.
- Mk, the class `rate_limited` ignored in `read_refused`: caught by my rate-limit case and the story's unit case `a_refused_read_names_its_class_and_a_rate_limit_its_wait`.
- Ml, park replaced by `return Err`: survived everything.

## Checked and fine
The match of the allowed user (case, white space, prefix, nested values, non-strings), the JSON parser's depth bound (64), the escaping of the cursor in the next input, two documents in one answer, the stream bound, and `--chat-poll-interval` (clap refuses 0 and below, floats, an empty value and over 2^64). The documents match the code except where F1, F2, F5 and F7 touch them. The website pages name no story id, stage, wave or test count.

## Not checked
- An `ok` false answer with exit 0, a `has_more` with no cursor, or 100000 reactions as end-to-end cases (I covered them by reading the code and its unit cases only).
- A `deadline-passed` line written more than once with a forced record failure, and a turn file that is a FIFO or not writable.
- Starvation between posts and polls, and SIGTERM between a post and a waiting poll.
- Whether the earlier cases with `--chat-poll-interval 3600` lose a claim. By reading they do not.
- A restart of the service between the `read` and the record change.

```findings
[
{"file":"crates/inbox-cli/src/bridge.rs","line":995,"category":"correctness","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"The read sends `oldest` without `\"inclusive\":true`, and the provider returns only posts after `oldest`, so a reaction on the oldest open post (the only one, when one is open) is never seen; red in adversary_a_reaction_on_the_oldest_open_post_is_seen."},
{"file":"crates/inbox-cli/src/wake.rs","line":527,"category":"design","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"undecided","message":"The planned round-2 settle compares entries with the rows of a round, but those rows are only `Sent` and not due (`undelivered=1`, `in_sent`) and a round reaches the surface only with such a row, so every Delivered entry would be settled at the next round and nothing settles while idle; settle against a read of the full Pending view instead."},
{"file":"crates/inbox-cli/src/bridge.rs","line":508,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A turn file removed while held lets a post run beside a read, the post's child overwrites the read's in `wake::CHILD`, and the thread that finds the slot empty parks for ever, so polls (or posts) stop for ever with no line; red in adversary_a_turn_file_removed_while_held_keeps_one_child_at_a_time."},
{"file":"crates/inbox-cli/src/wake.rs","line":664,"category":"concurrency","severity":"note","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"The signal path releases the `CHILD` guard before it ends the process, so a `connectors` the poll or round thread spawns in that window runs on in its own group after the bridge ends; a second spawning thread widens the window."},
{"file":"crates/inbox-cli/src/bridge.rs","line":881,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"A record change that fails after the service answered makes every retry resend `read` and, for a due message, write its `Expired` line again, and the design and CHANGELOG do not say the read is at-least-once."},
{"file":"crates/inbox-cli/src/outbox.rs","line":146,"category":"design","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"A `ChatPost` holds no channel, so a bridge restarted with another `--chat-channel` asks the new channel with the `oldest` and references of posts made in the old one."},
{"file":"crates/inbox-cli/src/bridge.rs","line":868,"category":"observability","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"A record the poll cannot read is silent until a message arrives and the post ends the bridge with 4; until then no reaction is marked and no line is written."},
{"file":"crates/inbox-cli/tests/bridge_read.rs","line":801,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Mutant Ma (`retry_after_seconds` ignored in `Poll::run`) survives the story's cases; killed by adversary_a_rate_limited_read_waits_its_named_time_and_a_failed_one_its_pause (2.6 s)."},
{"file":"crates/inbox-cli/tests/bridge_read.rs","line":801,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Mutants Mc and Md (`MOST_PAGES` 11 and 9) survive the story's cases; killed by adversary_a_poll_reads_ten_pages_and_fails_at_an_eleventh (1.6 s)."},
{"file":"crates/inbox-cli/tests/bridge_read.rs","line":801,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Mutant Mi (the cursor of the next page not passed) survives the story's cases; killed by adversary_a_poll_reads_ten_pages_and_fails_at_an_eleventh."},
{"file":"crates/inbox-cli/tests/bridge_read.rs","line":801,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Mutant Me (an entry made `read` when its `read` request fails) survives every case; a case whose forwarder drops one `read` request and checks that the entry stays `posted` would kill it."},
{"file":"crates/inbox-cli/src/bridge.rs","line":508,"category":"test-gap","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Mutant Ml (the park replaced by `return Err`) survives every case, including adversary_a_signal_during_a_read_ends_it_and_names_nothing, because the race the park guards against is too narrow for a case to hit reliably."}
]
```
