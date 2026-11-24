---
format: aep.planning-md/3
id: review-result:adversary-bridge-owed-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the case that waits until the record owes no line
relations:
- reviews: story:a-bridge-case-ends-its-bridge-after-the-record-owes-no-line
revision: 1
---
ADVERSARY PASS 1

Story: story:a-bridge-case-ends-its-bridge-after-the-record-owes-no-line. Attacked commit ace9cdb (round 1). Every job ran on build-1d: the first line of each job said "slot: synced to build-1d".

Result: no finding. I could not make the new wait red, and I found no place that needs the same wait. One note is below. I added no case. The adversary tree is back at ace9cdb with no change: `git status` and `git diff --stat` are both empty.

1. The wait itself (bridge_answer.rs:382). It cannot be true too early. The order of events:
- `Poll::owe` records `owed` before the write (bridge.rs:1438).
- The write follows (`owed_line`, bridge.rs:1456).
- `keep` drops `owed` after the write (bridge.rs:1470).
- The stand-in counts `reply.<q>` while it is invoked (bridge_once.rs:110), so between 1438 and 1470.

The case first waits for `replies() >= 4` (bridge_answer.rs:375). By then the bridge has recorded every `owed` it will have. After that, `owed` is empty in both entries only once the keep of the last line has run.
- A record read while it is being replaced: the bridge writes a `.new` file and renames it over the record (outbox.rs:860), under the outbox lock (outbox.rs:635). The helper's directory scan skips `.new` (bridge_once.rs:762). A read therefore sees either the old file or the new one, never a missing or half-written one.
- Hang: `owed` stays only while a write fails, and no `fail-reply` file is set in this case. The bridge's own lines are not in the scripted `replies-of`, so no later poll owes a new line.
- The guard on the second bridge (`thread_reads >= reads + 4`, :419) is sound. A bridge that still owes a line writes it before it reads the thread (bridge.rs:1255 comes before :1261), so a line written again is counted before that wait ends.

2. Places the implementor judged "cannot fail". I added 4 temporary sleeps of 1500 ms to the bridge, each between an effect and its record:
- S1: after `commands::answer` succeeds, before the keep that records `answered` (bridge.rs:1351 to :1359).
- S2: after the reaction's `read` mark, before `mark_chat_post` (bridge.rs:1769).
- S3: after `read_first`'s read, before its keep (bridge.rs:1400).
- S4: in `owed_line`, after the write, before the keep (bridge.rs:1464 to :1470).

With all 4 in, every bridge case passed: "102 tests run: 102 passed, 334 skipped". The slowest was 11.98 s, with the sleeps. That covers bridge_once.rs:972, bridge_answer.rs:512 and bridge_own_key.rs:205. The implementor had already proved the post-to-record and `Poll::line` places, so I did not add sleeps there again.

3. Places nobody listed. I read these extra places:
- bridge_once.rs:996-1013: the second bridge waits on the service state (Delivered and Read), then asserts the `read` count, `replies() == 1` and `posts() == 4`, and ends. No bridge follows it. S2 and S3 left it green.
- bridge_answer.rs:233-243, :747 and :957-968 wait on the record.
- adversary_bridge_once.rs:428, :469 and :512 restart the service after a wait on the record or on `posted()`.
- adversary_bridge_answer.rs:126 and :160 have one bridge and assert only on the read inputs.

None of them can fail by this order of events.

4. Other stand-in counts. Every assertion on a count of posts, replies or thread reads that I read follows either a wait on the record or a wait on a later poll's read. That later read happens only after the earlier poll has kept its record, because one poll thread does this work one step at a time. No assertion can be off by one.

5. Runs:
- The changed case with `--stress-count 20`: 20 of 20 passed, 2.29 to 2.32 s each.
- All bridge cases, 2 runs on the clean tree: "102 tests run: 102 passed, 334 skipped" both times (11.2 s and 10.7 s).
- Not the 3 runs a stress count of 2 would give: the brief asked for 2 runs.

Temporary edits: the 4 sleeps (S1 to S4) in the bridge, and the mutant below in the case file. I took all of them out by hand with Edit. No case file of mine stays.

Mutants:
- The case's wait made always true (`true || …`), with S1 to S4 in: caught. `bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing` failed at "no line again": the note's line appears twice under one key (11.4 s).
- S1 (Answer before its record): no case went red.
- S2 (reaction read before its record): no case went red.
- S3 (read before an Answer, before its record): no case went red.
- S4 (owed line before its record), with the wait in place: no case went red.

Note: the helper `recorded()` returns an empty text both when `owed` is absent and when the entry or the record file is absent (bridge_once.rs:784-791). So the new wait would also pass if the entries were missing. In this case both entries exist: `posted()` waited for them, and nothing removes them before the case ends. I could not make it pass that way, so this is INFEASIBLE.

What I did not check:
- A SIGTERM while the `connectors` child is still running after its post. The story leaves this out.
- A lost update between the rounds thread and the poll thread on the same entry: `change_chat_post` does nothing when the line changed meanwhile, which leaves `owed`. This needs a repost while a line is owed. It is not in this case, and I did not attack it.
- The full `task check`, and clippy. I added no code.
- The merged tree of the failed gate.

0 pre-existing findings; no private file written.

Build commands run without the `slot` prefix: none. Every build and test was `cargo nextest` through the shim, with `SLOT_TIMEOUT` set.

```findings
[{"file":"crates/inbox-cli/tests/bridge_answer.rs","line":382,"category":"test-oracle","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"The wait 'the record owes no line' uses recorded(), which returns an empty text also when the entry or the record file is absent, so it would pass vacuously if an entry were missing; in this case both entries exist throughout and it could not be made true early."}]
```
