---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-once-pass-3
kind: review-result
status: active
title: 'Adversary pass 3 on the chat bridge that asks once: no finding, 1 case added, 1 case corrected'
relations:
- reviews: story:the-chat-bridge-asks-a-decision-once
revision: 1
---
ADVERSARY PASS 3
unit: story:the-chat-bridge-asks-a-decision-once, round 3. Adversary tree <tree>-adv, detached at 0568ccb (the code is 228b641).
verdict: nothing found
cases: executed 373→374, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 15 paths, all under the scratch directory (part 6)
needs-coordinator: none
Commands that compiled or tested without `slot` and outside the shim: none. Every build and test was `cargo nextest` or `cargo clippy` through the shim, and each job's first line named build-1d. `rustfmt` and `perl` only edited files.

1. git --no-pager diff --stat
 crates/inbox-cli/tests/adversary_bridge_once.rs | 47 +++++++++++++++++++++++--
 1 file changed, 45 insertions(+), 2 deletions(-)
Only my cases file is changed, uncommitted. Mutants were reverted: `cmp` against the saved bridge.rs and outbox.rs matched.

2. Cases
- My held-line case, `adversary_a_held_line_that_does_not_read_gives_no_second_post_at_a_repost`: I wrote my own wait and did not take the patch.
  - The wait now ends on whichever comes first: a second post, a `MarkDelivered`, or the end of the bridge.
  - `from` is now taken before the repost.
  - The claim `posts == 1` stays. I added what the decision adds: no `MarkDelivered` since the restart, exit 4, the request still `Sent` in alice's outbox.
  - Green, 0.11 s alone (at most 0.49 s over 10 runs).
  - Red under p1 (round 2's `filter_map` in `chat_posts_of`) at adversary_bridge_once.rs:662: `assertion left == right failed: a second post operation of 01M8685777PR2WR6D1X72EHWWC ...`
- New: `adversary_an_empty_bridge_clock_file_at_the_start_ends_it_with_4_before_any_post`.
  - Asserts: an empty `--clock-file` at the start gives exit 4, a line naming `--clock-file` and the file, 0 posts, and the note still `Sent`.
  - Green, 0.04 s (at most 0.14 s).
  - It is the only case red under c1, where the bridge falls back to the system clock when its clock file does not read: `the bridge ends: not within 10 s; it said: ` (adversary_bridge_once.rs:582).
  - `clock_file::a_clock_file_that_holds_no_instant_stops_serve_before_it_binds` holds the service's code only, not the bridge's `FileClock::of` path, so it does not catch c1.
- clippy `-p inbox-cli --all-targets --locked -- -D warnings`: exit 0.

3. Runs
- Crate suite `SLOT_TIMEOUT=30m cargo nextest run -p inbox-cli --locked --no-fail-fast`: `Summary [7.739s] 374 tests run: 374 passed, 0 skipped`. That is the brief's 373 plus my clock case.
- Stress run of `bridge_once` and my whole file, `--stress-count 10`: `Summary [33.889s] 10/10 stress run iterations: 10 passed`. Nothing was red and nothing went over 5 s.
  - Round 3's held-line case `a_held_line_that_is_no_entry_fails_the_poll_and_ends_the_bridge_at_a_repost`: at most 1.27 s, mean 1.18 s.
  - Round 3's reply case `a_reply_that_fails_is_sent_again_with_the_same_key_and_stops_nothing`: at most 3.36 s, mean 3.12 s.
  - Slowest of the rest: A at most 3.82 s, B1 3.72 s, retention 3.20 s.
- Mutants (filter `test(/bridge/) | test(/chat_posts/)`), all red:
  - p1, a cut line skipped: red in my held-line case, round 3's held-line case and both outbox unit cases.
  - o10, the resend for every `read` entry whenever any row is `Delivered`: red in `bridge::tests::a_read_is_sent_again_only_for_an_entry_read_whose_own_message_is_delivered`, left `["lost", "waiting"]`.
  - o12, the resend for an entry not `read` (the read mark check dropped): red in that unit case (left `["lost", "unread"]`), in bridge_once A, B1 and the reply case, and in 9 bridge_read and 2 adversary_bridge_read cases.
  - f1, the entry marked although its reply failed: red in the reply case, "the reply fails at a poll after: not within 10 s".
  - f2, another key at the second try: red in the reply case at bridge_once.rs:1091, key `…-end-again`.
  - c1, the clock falls back to the system clock: red only in my new case.

4. Findings: none.

5. Attacked and not broken
- F-a, the other forms of a line that does not read, from the code and the unit case:
  - JSON of another shape, or a member of the wrong type: `chat_post` gives None, so the record cannot be read.
  - An empty line: the record cannot be read. A last line without its line end reads.
  - Every change runs `chat_posts_of` before it writes, so no change drops or rewrites a line it could not read.
  - `replace` writes an empty file for an empty record, so dropping the last entry never leaves an empty line.
  - The rounds end with 4 and a line that names the record, so a bad line never stops the bridge without a line.
  - The record of the first bridge story (bab2b7c) already wrote `state`, so its lines still read.
- F-b: o10 and o12 are both red (above).
- The reply that fails, from the code (`Poll::line`, bridge.rs:1197): a reply that fails at every try costs 1 reply operation per poll per ended entry and 1 line per run of failures. A `retry_after_seconds` on a reply is not kept: the reply is tried again at the next poll, as the design says. I ran no case for these two.
- The case names of pass 1 (`…posted_again…`, `…delivered_for_good`) now name the defect they caught, not what they hold. They are left as they are.
- Not reached: a run of a reply that fails at every try, or of one answered `rate_limited`.

6. Paths written outside the worktree, all in <scratch>/s4/once/adversary3/:
- mutants.sh, mutants.txt, mutants.out
- bridge.rs.orig, outbox.rs.orig
- own.log, stress.log, suite.log, clippy.log
- mutant-p1-cut-line-skipped.log, mutant-o10-resend-any-delivered.log, mutant-o12-resend-without-read-mark.log, mutant-f1-marked-though-reply-failed.log, mutant-f2-another-key-again.log, mutant-c1-clock-falls-back.log

```findings
[]
```
