---
format: aep.planning-md/3
id: review-result:adversary-bridge-answer-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the thread reply that becomes an answer: 7 findings, 3 red cases'
relations:
- reviews: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
revision: 1
---
ADVERSARY PASS 1
unit: story:the-chat-bridge-turns-a-thread-reply-into-an-answer, adversary tree detached at 24dfd31 (round 1 with story 5 merged in; base 13f0ee6)
verdict: NEEDS-CHANGE
cases: executed 417→420, red 3
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 17 files, all in the scratch directory (listed in part 6)
needs-coordinator: none

1. git --no-pager diff --stat (tracked): crates/inbox-cli/tests/main.rs | 1 + (1 file changed, 1 insertion). Untracked: crates/inbox-cli/tests/adversary_bridge_answer.rs. Both are test files, uncommitted. bridge.rs came back to its saved md5 2e7e634f after the mutant runs. One file is git-ignored inside the tree: .engineering/drafts/mutants.sh, the mutant loop.

2. Cases in crates/inbox-cli/tests/adversary_bridge_answer.rs (mod line in tests/main.rs, sorted). All 3 are red now, and were red at the first run (red-1.log, run on build-1d).
- a_thread_whose_read_fails_stops_no_answer_in_another_thread (red in 1.4 s). Setup: 2 requests are posted. The thread read of the first answers ok:false thread_not_found, as for a post removed from the channel. The second thread holds a reply of the allowed chat user. Red line, verbatim: "panicked at crates/inbox-cli/tests/adversary_bridge_answer.rs:94:5: the thread of the other post was never read in 3 polls: [\"1700000000.000001\", \"1700000000.000001\", \"1700000000.000001\"]".
- a_full_page_of_other_users_does_not_hide_the_persons_reply_for_ever (red in 3.1 s). Setup: the first page is the post plus 99 replies of another chat user, with has_more true, as the provider answers limit 100. The input never moves, so the provider gives that page at every poll. Red line, verbatim: "panicked at …adversary_bridge_answer.rs:131:5: every read asks for the same first page, so the reply after it is never read: [Object([(\"channel\", Text(\"C0000000001\")), (\"ts\", Text(\"1700000000.000001\")), (\"limit\", Number(\"100\"))]), …×3]".
- a_reply_without_text_is_not_sent_as_an_empty_answer (red in 3.2 s). Setup: a reply of the allowed chat user to a request holds only white space. Red line: "panicked at …adversary_bridge_answer.rs:163:5: an Answer with no text was sent for <id>: POST /messaging/commands/answer …". The envelope carried "body":"  ".
- Removed after the mutant runs, because it is green and kills no mutant that the story's cases let live: an_answer_taken_whose_answer_was_lost_gets_no_line_that_it_was_not_taken. It used a forwarder that carries the Answer to the service and drops only the service's answer. It was green in 1.2 s: 1 Answer sent, no thread line, and the entry ends `answered`. A copy is kept in the scratch directory (finding 4).

3. Suite, after the cases existed: `cargo nextest run -p inbox-cli --locked --no-fail-fast` on build-1d gave "Summary [ 10.819s] 421 tests run: 418 passed, 3 failed, 0 skipped", exit 100. The 3 failures are the 3 cases above; 421 includes the lost-answer case that was removed afterwards. Final file alone: "3 tests run: 0 passed, 3 failed, 417 skipped". Stress of the story's cases, `--stress-count 5 -E 'test(/^bridge_answer::/)'`: "5/5 stress run iterations: 5 passed", exit 0. Item 9, `bridge_own_key` plus `adversary_bridge_own_key` (19 cases), `--stress-count 10`: "10/10 stress run iterations: 10 passed", exit 0. `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0 on the final file; the one warning is the known one in generated wire.rs.

4. Findings.
- F1. crates/inbox-cli/src/bridge.rs:1188 (`self.replies(entry.clone(), row)?;` in `Poll::answers`).
  - What was measured: one post whose thread read fails stops every post after it in that poll. This repeats at every poll, so a reply in another thread is never sent. The case is red at :94.
  - What reaches it: a post removed from the channel (by an admin or the channel's retention) while its message is still live. Any page member that the strict `thread_page` refuses does the same, for example a `ts` that is not `<digits>.<digits>` or a `user` that is not text.
  - Fix: handle a failed read per post (one line per run of failures, then go on to the next post) instead of failing the poll. warning, NEEDS-CHANGE, introduced.
- F2. crates/inbox-cli/src/bridge.rs:2127 (`thread_page`: only a reply of the allowed chat user moves `replied`, which is the `oldest` of the next read).
  - What was measured: once 99 messages of others follow the last handled reply, the input stays the same and so does the page. The person's next reply is never read while the message lives. Red at :131.
  - What reaches it: a busy thread in a shared channel.
  - Fix: keep a read mark past the last `ts` of each page, or follow `has_more` and the cursor.
  - website/docs/limits.md:131 ("until fewer follow") and docs/design/bridge.md:891 say the reply shows up once fewer replies follow. It does not: the 99 never become fewer. warning, NEEDS-CHANGE, introduced.
- F3. crates/inbox-cli/src/bridge.rs:1259 (`(None, reply.text.clone())`, with `text: said.unwrap_or_default()` at :2132).
  - What was measured: a reply to a request whose text is empty or only white space is sent as the person's Answer and settles the message. A shared file or image has an empty `text`, and so does a reply without `text`. Red at :163.
  - Fix: give such a reply its own line and send nothing. warning, NEEDS-CHANGE, introduced.
- F4. website/docs/limits.md:132 and docs/design/bridge.md:717.
  - What was measured: both say that an Answer whose answer was lost is sent again and then gets the false `already-settled` line. With the lost-answer case (the service took the Answer, its answer was dropped, 1 Answer carried) there is no second Answer and no line. The next poll finds the message out of `Pending`, and `Poll::ended` gives the entry the end `answered` before any thread read.
  - The person gets no line at all in that path. The documented false line was not reached; I found no path that reaches it.
  - Fix: correct both sentences. note, CONFIRMED, introduced.
- F5. Surviving mutant at crates/inbox-cli/src/bridge.rs:2196: `*status == 408 ||` dropped. A 408 then gets a "refused" line and the reply is never sent again, against the design. All cases of the story stay green; no case covers a 408. note, CONFIRMED, introduced.
- F6. Surviving mutant at crates/inbox-cli/src/bridge.rs:1184: the channel filter of `Poll::answers` made always true, so the entries of another channel are read. No case covers it. note, CONFIRMED, introduced.
- F7. Surviving mutant at crates/inbox-cli/src/bridge.rs:465: `let answered = …` set to false (a repost of an answered entry is not made `read` again). No case covers it. note, CONFIRMED, introduced.

Mutants (filter `test(/bridge/)`, 91 cases each, all runs on build-1d). My 3 red cases are red in every run and are not counted as kills.
- i01, a reply of another chat user taken: caught by bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing and the unit test a_page_of_replies_lists_the_allowed_chat_users_after_the_last_handled.
- i04, a second Answer after the first was taken: caught by bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing.
- i10, a transport failure gets a line and is not tried again: caught by bridge_answer::a_refused_answer_gets_one_line_and_a_lost_one_is_sent_again.
- i11, a reply over the bound is sent: caught by bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing.
- o1, the `Answered` row is live for any entry (state test dropped at :1591): caught by bridge_once::a_message_nobody_answered_gets_one_line_at_its_end_and_none_in_the_persons_place, and by the removed lost-answer case.
- o2, 408 not retried: survived (F5).
- o3, entries of another channel read: survived (F6).
- o4, no read again at the repost of an answered entry: survived (F7).

5. Attacked and not broken:
- The match: lower case, `Bob`, a letter beyond the options, a no-width or direction character before the letter, a letter that looks Latin. All read as no option, so the person gets the letters line and nothing is sent. OptionLetter is one character A–Z in the model.
- The 65536-byte bound is `>` on bytes of UTF-8.
- The thread line text quotes none of the reply.
- The Answer's error lines carry no reply text.
- 413 cannot come from the default service: its body limit is 1 MiB, and 65536 bytes escaped is at most about 393 kB.
- The record is written before the line, and the line goes under one key per reply.
- The merge: the 10× own-key stress is green. The AGENTS.md row, the CHANGELOG bullet and the status item each appear once.
- No pre-existing finding, so no private file was written.

Not reached:
- Item 2: the bridge's own lines read back as replies of the allowed chat user, for a connector that posts with the person's token. Not built.
- Item 4: read-first failure paths.
- Item 5: 400, 404 and 429 lines; a 401 at every poll without `--for`.
- Item 6: a child answer that is very large or not JSON.
- Item 7: a hanging child (`invoke` has no time bound; that is the same for every child before this story); records of builds before and after this story.
- Item 8: `--for` with a lapsed delegation.
- Item 10: the decision post text beyond the story's unit test.
- Item 11: generated/cli/inbox.json and `--help` were not compared word by word.

Build commands run without the slot prefix: none. Every cargo call went through the shim on build-1d. Only `rustfmt --edition 2024` on my file and the sed/cp of the mutant loop ran locally.

6. Paths written outside the tree, all in the scratch directory:
- src.diff
- bridge.rs.orig
- red-1.log
- mutants.txt
- the 8 mutant-*.log files
- suite.log
- stress.log
- clippy.log
- clippy-2.log
- final.log
- own-key-stress.log
- adversary_bridge_answer.with-lost-answer.rs

7.
```findings
[
{"file":"crates/inbox-cli/src/bridge.rs","line":1188,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"A thread read that fails for one post fails the whole poll, so no reply in the thread of any later post is ever sent while that post stays live."},
{"file":"crates/inbox-cli/src/bridge.rs","line":2127,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The read mark moves only past replies of the allowed chat user, so once 99 messages of others follow it the same first page is read for ever and the limits page wrongly says the reply shows up once fewer follow."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1259,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"A reply to a request that is empty or only white space, such as a shared file, is sent as the person's Answer and settles the message."},
{"file":"website/docs/limits.md","line":132,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The limits page and the design say a lost acknowledgement leads to a second Answer and a false already-settled line, but the entry ends at the next poll with one Answer and no line."},
{"file":"crates/inbox-cli/src/bridge.rs","line":2196,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Dropping the 408 retry from refused_line leaves every case green."},
{"file":"crates/inbox-cli/src/bridge.rs","line":1184,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Making the channel filter of Poll::answers always true leaves every case green."},
{"file":"crates/inbox-cli/src/bridge.rs","line":465,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Turning off the read-again of an answered entry at a repost leaves every case green."}
]
```
