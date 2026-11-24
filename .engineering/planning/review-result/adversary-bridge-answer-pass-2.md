---
format: aep.planning-md/3
id: review-result:adversary-bridge-answer-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the thread reply that becomes an answer: 9 findings, 4 cases added, 2 of them red'
relations:
- reviews: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
revision: 1
---
ADVERSARY PASS 2
unit: story:the-chat-bridge-turns-a-thread-reply-into-an-answer, adversary tree detached at 6c42fc8 (round 2 on the findings of pass 1)
verdict: NEEDS-CHANGE
cases: executed 427→431, red 2
origin: introduced 8 / pre-existing 1 / undecided 0
wrote-outside-worktree: 39 files, all in the scratch directory (listed in part 6)
needs-coordinator: none

Summary: 2 new cases are red. One shows that when an `Answer` was taken by the service but its acknowledgement was lost, standard error wrongly says it was not taken and is sent again. The other shows that after one thread read is refused `rate_limited`, the bridge still reads the next thread in the same poll, inside the wait the provider named. One case that round 2 added is racy. 7 mutants (deliberate one-line changes to the product) left the story's cases green; my 2 green cases now catch 4 of them. Nothing was committed or pushed.

1. git --no-pager diff --stat: crates/inbox-cli/tests/adversary_bridge_answer.rs | 317 +++ (1 file changed, 317 insertions). That is a test file, left uncommitted. The 3 cases of pass 1 are byte for byte unchanged (the first 171 lines match the saved copy). No product file is changed: bridge.rs is back at its md5 db7b4bef after both mutant loops. One file is git-ignored inside the tree: .engineering/drafts/mutants-adv2.sh, the mutant loop.

2. Cases added to crates/inbox-cli/tests/adversary_bridge_answer.rs. Helpers added there:
- `scripted`: a script put in front of the stand-in, which still runs behind it and records every call. It can answer a chosen read of the replies `rate_limited` (exit 1) or `ok: false`.
- `bridge_stderr` and `record_line`: read the bridge's standard error and its record of the chat posts.

Each case and its result when first run alone on build-1d (red-1.log, green-2.log):
- an_answer_the_service_took_is_not_named_as_not_taken_and_sent_again: RED, 1.2 s. The forwarder carries the `Answer` and throws away the service's answer. The case waits for the entry's end `answered`, then checks 1 `Answer` and the bridge's standard error. Red line, verbatim: "panicked at crates/inbox-cli/tests/adversary_bridge_answer.rs:293:5: the service took the one Answer of 01M86FYSQKZMP4SWCZ26BBMSYY and nothing was sent again, yet standard error says: [\"inbox bridge: the read of the reactions in the chat channel C0000000001 failed: the answer to 01M86FYSQKZMP4SWCZ26BBMSYY from the thread of its post was not taken: transport: 127.0.0.1:37347 (127.0.0.1:37347): the answer has no end of its head; it is sent again; nothing is marked read, and it is tried again after a pause (no further line until a read succeeds)\"]".
- a_rate_limited_read_of_one_thread_fires_no_read_of_another_thread_inside_its_wait: RED, 2.1 s. 2 live posts. The read of the replies of the first is refused `rate_limited` with `retry_after_seconds` 1. The case checks the next call of `connectors`: it must not be a read of the replies in the same poll (a new poll starts with the read of the channel). Red line, verbatim: "panicked at crates/inbox-cli/tests/adversary_bridge_answer.rs:348:5: the read of the thread of 1700000000.000002 was fired right after the read of the replies of 1700000000.000001 was refused rate_limited (the next read not before 1 s), in the same poll: invocation 8 is \"conversations.replies\"".
- a_failed_thread_read_is_named_once_per_run_and_again_after_a_success: GREEN, 4.1 s. Reads 1, 2 and 4 of one post answer `ok: false`; read 3 succeeds. Standard error must hold exactly 1 line after read 3 starts, and 2 lines after read 4. My first version waited only for read 4, which raced with the line being written, so mutant M3 lived. I tightened my own case and ran it again; it now catches M3 and M4.
- an_own_line_a_read_has_passed_is_never_read_again_as_the_persons_reply: GREEN, 4.1 s. A reply with no text gets the bridge's line, which the stand-in then lists as a reply of the allowed chat user. Once a read has passed that line, there must be no `Answer` and no second line, and `own_lines` must be empty in the record. It catches M2 and M8.

3. Suite runs, after the cases existed, all on build-1d:
- `cargo nextest run -p inbox-cli --locked --no-fail-fast`, with 3 of the cases (suite.log): "Summary [ 11.457s] 430 tests run: 428 passed, 2 failed, 0 skipped", exit 100.
- The same command after the fourth case (suite-2.log): "Summary [ 11.595s] 431 tests run: 429 passed, 2 failed, 0 skipped", exit 100. The 2 failures are the 2 red cases above.
- Stress (item 5), 10 times, with my 2 red cases left out. Filter: `(test(/^(adversary_)?bridge_answer::/) | test(/^bridge_once::/) | test(/^bridge::/) | test(/^bridge_read::/) | test(/^bridge_own_key::/))`. Result: "Summary [ 85.300s] 10/10 stress run iterations: 10 passed", 70 cases per iteration, exit 0. That includes the repost case that was racy in round 2.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings`: exit 0. The one warning is the known one in the generated wire.rs.

4. Findings. They cover the adversary tree at 6c42fc8 plus my cases file.

F1. crates/inbox-cli/src/bridge.rs:1340 (the `Err` of `Poll::reply` for an `Answer` without a usable answer), written through the frame of the failed-poll line at :1039. (Item 4.)
- What was measured: in the lost-answer run the operator reads the line quoted in part 2, word for word. It is false in 3 places:
  - "the read of the reactions … failed": it was an `Answer` that failed.
  - "was not taken" and "it is sent again": the service took the `Answer`, 1 was carried, and the entry ended `answered` with nothing sent again.
  - "nothing is marked read": by the code, a request still `Delivered` gets its `read` sent first through `read_first`. I did not assert this one.
- Against the documents: website/docs/limits.md:279-281 says such an answer "is sent once" and "writes no line". docs/design/bridge.md:553 names this line as the line of the read of the reactions.
- What reaches it: any `Answer` whose answer is lost after the service took it. Round 2's report names this path.
- Fix: in bridge.rs, the line for an `Answer` without an answer should be a line of its own. It should say that whether the service took the `Answer` is not known, and that the next poll sends it again unless the message has left `Pending`. It should not go through the frame of the channel read.
- warning, NEEDS-CHANGE, introduced.

F2. crates/inbox-cli/src/bridge.rs:1203 (`Poll::answers`: after a read of the replies fails with a named wait, the loop goes on to the next post). (Item 3.)
- What was measured: red at :348 of my file. The next call after the refused read is the read of the next thread, in the same poll.
- What reaches it: the provider's limit counts calls to the method, not one thread (not verified). With N live posts, every poll that meets the limit fires N-1 more reads into it.
- docs/design/bridge.md, around line 690, says the posts after a failed read are read in the same poll. Your brief calls a read fired inside the named wait a finding.
- Fix: in `Poll::answers`, after a read refused with a wait, read no further thread in this poll; the rest are read at the next poll, after the wait.
- warning, NEEDS-CHANGE, introduced.

F3. crates/inbox-cli/tests/bridge_answer.rs:562 (round 2's `a_reply_after_a_full_page_of_other_users_is_read_at_the_next_poll`). (Item 5.)
- What was measured: it was red once, in the run of mutant M6. M6 changes only `Chat::again`, which this case never reaches. Red line: "assertion `left == right` failed left: \"1700000002.000100\" right: \"1700000003.000001\"".
- Cause: the case waits until the sender's `Outbox` shows `Answered`, which the service sets when it takes the `Answer`. It then reads `read_to` from the record, which the bridge writes only after the service's answer comes back. It was green in the 10-times stress run and in both suite runs.
- Fix: wait on the record (`read_to` or state `answered`) before the assertion.
- warning, NEEDS-CHANGE, introduced.

F4. bridge.rs:1256. Surviving mutant M2: `own_lines` is never pruned. Every case of the story stays green. My own-line case catches it. note, CONFIRMED, introduced.

F5. bridge.rs:2108 (`read_from`). Surviving mutant M8: the next read starts after `replied` rather than `read_to`. Every case of the story stays green. Under it, the bridge's own line is sent as the person's `Answer` once a read has passed and pruned it. My own-line case catches it at :470. note, CONFIRMED, introduced.

F6. bridge.rs:1204. Surviving mutant M3: the line for a failed read is written at every failed poll. Every case of the story stays green. My failed-read case catches it at :390. note, CONFIRMED, introduced.

F7. bridge.rs:1243. Surviving mutant M4: the set of failing posts is never cleared, so no second line follows a success and a new failure. Every case of the story stays green. My failed-read case catches it ("4 reads of the thread and 2 lines: not within 10 s"). note, CONFIRMED, introduced.

F8. bridge.rs:1210. Surviving mutant M5: the wait a `rate_limited` read of the replies named is dropped. No case holds the design sentence that the next poll comes after that wait. My cases do not catch it either. note, CONFIRMED, introduced.

F9. bridge.rs:1176 (`Poll::bound`). Surviving mutant M7: a line of the record without `channel` is no longer counted as of the bound channel. No case covers it. The filter is the same at the base 13f0ee6:1063; round 2 only moved it. note, CONFIRMED, pre-existing.

None of these is a security flaw, so the private file for pre-existing security findings was not written.

Mutants. Each run used the cases of bridge_answer, adversary_bridge_answer, bridge_once, the bridge unit tests and the outbox unit tests (49 or 50 cases), all on build-1d:
- M1, an `Answer` without an answer fails no poll (`.unwrap_or(None)` at :1246): caught by bridge_answer::a_refused_answer_gets_one_line_and_a_lost_one_is_sent_again. Round 2's 408 case and lost-answer case stay green under it, so neither holds its "the poll fails" part.
- M2, `own_lines` never pruned (:1256): lived through the story's cases; caught by an_own_line_a_read_has_passed_is_never_read_again_as_the_persons_reply.
- M3, a line at every failed read (:1204): lived; caught by a_failed_thread_read_is_named_once_per_run_and_again_after_a_success (after my fix of that case).
- M4, the failing set never cleared (:1243): lived; caught by a_failed_thread_read_is_named_once_per_run_and_again_after_a_success.
- M5, the named wait dropped (:1210): survived.
- M6, no `read` at the repost of an answered entry (:498): equivalent. The repost case stays green because the poll's `read` again for an entry `read` whose message is `Delivered` does the same. The one red case in this run is F3, which this mutant does not reach.
- M7, entries without `channel` dropped (:1176): survived (F9).
- M8, `read_from` prefers `replied` (:2108): lived; caught by an_own_line_a_read_has_passed_is_never_read_again_as_the_persons_reply.

Round 2's cases (item 5):
- The 408 case and the lost-answer case hold their thread-side claims (round 2's m5 and m8) but not "the poll fails" (M1).
- The other-channel case is held by round 2's m6.
- The repost case is held by round 2's m7, not by M6, which is equivalent.
- The own-line case does not hold the pruning (M2).
- The full-page case is racy (F3).
- No earlier assertion was weakened by the helper changes in bridge_once.rs: `drop_next` now calls `next(…, Drop)` with the same behaviour, and the page limit of the stand-in does not touch scripted pages under 100. No case of round 2 holds a wall-clock time.

5. Attacked and not broken:
- `read_to` after an `Answer` that failed in the middle of a page. A line owed before it moves `read_to` to that reply only, so the failed reply is read and sent again at the next poll. Nothing is skipped (by reading the code).
- A record of a build before this round: no `read_to` reads from `replied`, then from the post. A `read_to` that is not text is ignored; an `own_lines` that is not a list reads as empty (round 2's unit test and the code). A `read_to` that is text but not of the `ts` form makes every read of that post fail for ever, with one line. Only a hand-edited record has one.
- 2 messages with the same `ts`, or a `ts` listed below `read_to`: the provider makes each `ts` unique and gives it on arrival (not verified), so I built no case.
- A post whose read fails for ever is read again at every poll: 4 reads in 4 polls, one child per poll (measured in my failed-read case). The other posts are not held up (pass 1's case is green). 50 live posts were not measured.
- End lines are added to `own_lines` but never read back: an entry with an end is not read again, and `Chat::again` clears only the ends `gone` and `answered`, which write no line.
- An own line whose reference was not recorded because the bridge stopped in between: already a documented limit (limits.md, design "Limits").
- The `AGENTS.md` row, the CHANGELOG bullet and the help text of `inbox bridge` match `generated/cli/inbox.json` and the code. I compared the changed sentences, not the whole files.
- The service has no `429` path, so its line is not reachable today.

Not reached:
- Item 6: the read-first failure paths and the lines for `400` and `404`, by case; a child answer that is very large or not JSON; `--for` with a delegation that lapsed between 2 replies.
- Item 1: a full page followed by a short one across 3 polls beyond round 2's case; a repost and `read_to` beyond what the repost case covers.

Build commands run without the `slot` prefix: none. Every cargo call (nextest, clippy) went through the cargo shim and names build-1d in its first line. There was no refusal of slot and no exit 3. Only `rustfmt --edition 2024` on my file and the perl, cp, md5sum and diff steps of the mutant loop ran on this host.

6. Paths written outside the tree, all in the scratch directory:
- adversary_bridge_answer.pass1.rs, the saved copy of the pass 1 file
- src.diff, tests.diff, docs.diff
- md5-before.txt
- red-1.log, green-2.log, suite.log, suite-2.log, stress.log, clippy.log
- mutants/: bridge.rs.saved, loop.out, mutants.txt, mutant-M1.diff to mutant-M8.diff, mutant-M1.log to mutant-M8.log
- mutants-2/: bridge.rs.saved, loop.out, mutants.txt, mutant-M2, M3 and M8 .diff and .log

7.
```findings
[
{"file":"crates/inbox-cli/src/bridge.rs","line":1340,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"An Answer the service took whose acknowledgement was lost is named on standard error as a failed read of the reactions, not taken and sent again, while one Answer was sent and the limits page says it is sent once."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1203,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"After a read of the replies is refused rate_limited, the poll fires the read of the next thread at once, inside the wait the provider named."},
{"file":"crates/inbox-cli/tests/bridge_answer.rs","line":562,"category":"concurrency","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The full-page case reads read_to from the record as soon as the service shows the request answered, before the bridge writes it, and was red once."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1256,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Never pruning own_lines leaves every case of the story green."},
{"file":"crates/inbox-cli/src/bridge.rs","line":2108,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Reading from replied before read_to leaves every case of the story green and sends a pruned own line as the person's Answer."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1204,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Writing the failed-read line at every poll leaves every case of the story green."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1243,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Never clearing the set of failing posts leaves every case of the story green."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1210,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Dropping the wait a rate_limited read of the replies named leaves every case green."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1176,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"Dropping the record lines that name no channel leaves every case green."}
]
```
