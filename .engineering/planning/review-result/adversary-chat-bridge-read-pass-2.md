---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-read-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the chat bridge that marks read: 4 findings, 3 cases added, none red'
relations:
- reviews: story:the-chat-bridge-marks-a-message-read-on-a-reaction
revision: 1
---
ADVERSARY PASS 2

Adversary pass 2 on story:the-chat-bridge-marks-a-message-read-on-a-reaction, round 3 (code 307970e, tree detached at 668b494).

**Result:** the fixes for F1 and F3 to F7 hold, and no case was flaky in 5 runs. Two mutants live against the story's cases; two new green cases of mine kill them. One note: the park in `invoke` cannot be reached.

The cases are in the adversary tree, uncommitted: crates/inbox-cli/tests/adversary_bridge_read.rs. `git diff --stat` shows only that file, 121 lines added. Every job ran on build-1d ("slot: synced to build-1d"), all after the node's hold ended at 05:41:43. No red result came from the 04:54 to 05:41 window, so nothing was rerun. Clippy `-p inbox-cli --all-targets -D warnings` exits 0. Logs and scripts are in <scratch>/s4/read/adversary2/.

## Runs
- **Baseline:** the cases of bridge.rs, adversary_bridge.rs, bridge_read.rs and adversary_bridge_read.rs, plus the bridge unit cases: `46 tests run: 46 passed`.
- **Stress:** `--stress-count 5` over the 10 cases of bridge_read.rs and my 6 cases at that time: `5/5 stress run iterations: 5 passed`. No case went red.
- **My cases after the last one was added:** `7 tests run: 7 passed`.

## Findings

**1. warning: mutant N6 lives against the story's cases.** N6 counts an entry without a channel as being of another channel (`.is_none_or(|of| of == channel)` becomes `.is_some_and(...)`, bridge.rs:861). No story case has a line without a channel, so all 46 stay green.
- New case `adversary_an_entry_without_a_channel_is_asked_about_as_of_the_bound_channel`, green in 0.06 to 0.12 s. A bridge posts a note and is stopped. I strip `"channel":"C0000000001",` from its record line and start a second bridge with a reaction listed. The note becomes `read`.
- Under N6 it is the only red case.
- Fix: the coordinator moves this case into the story's cases.

**2. warning: mutant N10 lives against every case of pass 1.** N10 makes `mark_chat_post` drop the channel (outbox.rs, `mark_chat_post`). A later story reads the post of a request in `Read` from the record, so the channel must survive the change of state.
- New case `adversary_a_marked_entry_keeps_its_channel`, green in 1.07 s. After a reaction marks the note read, its record line still names `C0000000001`.
- Under N10 it is red: `` `channel` is None ``.
- Fix: the coordinator moves this case into the story's cases.

**3. note: the park and the "gone from the slot" error cannot be reached.** The implementor said a race keeps a case from holding mutant Ml. The real reason is that the branch is dead code (by reading wake.rs:676-680 and bridge.rs:488-504; the mutants agree).
- Since round 3 the signal path takes the `CHILD` guard and holds it until the process exits. A thread in `invoke` then blocks on `wake::child()`, and never sees an empty slot.
- Under `TURN` only one `invoke` runs at a time, and nothing else empties the slot.
- So both the park (`wake::ending()` set) and the error line (flag not set) are dead code. Four mutants live against all 46 cases and are equivalent:
  - N1: the flag never set;
  - N2: the flag set after the child is taken;
  - N11 (Ml): the park becomes `return Err`;
  - N12: an empty slot always parks.
- docs/design/bridge.md lines 320-325 say "none finds the slot empty but the one whose child it took", and that a thread parks when it finds the flag set. In fact no thread finds the slot empty: the one whose child was taken blocks on the guard.
- Fix: say in the design that the branch is defensive and cannot be reached while the guard is held, or remove the flag and the park.
- F4 itself stays without a case: mutant N3 (guard released before exit) lives against all 46 cases. I agree a case for it needs a race.

**4. note: the line of a due message can be lost.** By reading, bridge.rs:890-908. The line is written after `mark_chat_post` records `due`. If the bridge is ended between the two, the entry is `due`, is never asked about again, and its line is never written. The design (line 350, "writes one line … only after the record says `due`") and the CHANGELOG do not say the line is at most once. Fix: one sentence in docs/design/bridge.md.

## Mutants
Each run against the cases of bridge.rs, adversary_bridge.rs, bridge_read.rs, adversary_bridge_read.rs and the bridge unit cases. Sources restored after each (cmp ok).

| Mutant | Result |
|---|---|
| N1 flag never set | survived (equivalent, finding 3) |
| N2 flag set after the take | survived (equivalent, finding 3) |
| N3 guard released before exit | survived (F4: no case without a race) |
| N4 `inclusive` false | caught by `adversary_a_reaction_on_the_oldest_open_post_is_seen`, the pages case and 7+ cases of bridge_read.rs |
| N5 channel not read back | caught by `bridge_read::a_bridge_bound_to_another_channel_asks_about_and_marks_only_its_own_posts` |
| N6 entry without a channel as another channel | caught only by my `adversary_an_entry_without_a_channel_is_asked_about_as_of_the_bound_channel` (finding 1) |
| N7 unreadable record skipped without a line | caught by `bridge::a_post_that_cannot_be_recorded_is_not_marked_delivered` |
| N8 line of a due message never written | caught by `bridge_read::a_reaction_on_a_due_message_is_named_once_and_on_a_read_or_gone_one_is_not` and `bridge_read::a_read_without_an_answer_or_without_its_record_change_marks_nothing_and_is_sent_again` |
| N10 `mark_chat_post` drops the channel | caught only by my `adversary_a_marked_entry_keeps_its_channel` (finding 2) |
| N11 (Ml) park returns an error | survived (equivalent, finding 3) |
| N12 empty slot always parks | survived (equivalent, finding 3) |

## Pass 1 findings rechecked
- **F1:** fixed. N4 is red in many cases. The input carries `inclusive` byte for byte on page 1 and on cursor pages (the pages case).
- **F3:** fixed with `TURN`. The story's turn case reads `/proc/<pid>/task/<tid>/syscall`. Where that file cannot be read it hangs and goes red, not green. A post beside a read is still caught by its `posts() == 1` check.
- **F4:** the guard is held until exit, with no case (N3 lives).
- **F5:** fixed; N8 is caught.
- **F6:** fixed. N5 is caught by the story; N6 and N10 only by my new cases.
- **F7:** fixed; N7 is caught.
- The implementor's changes to my pass 1 cases keep their claims.
- The stand-in's new `page()` filter makes no case vacuous: N4 reddens the cases that need the post at `oldest` listed.
- The F5 case without `reads_of(due) >= 3` still holds "sent again" (`>= 2` while the entry is `posted`) and the final `due` with its line.

## New cases (all green)
- `adversary_an_entry_without_a_channel_is_asked_about_as_of_the_bound_channel`: 0.06 to 0.12 s.
- `adversary_a_signal_while_a_post_waits_for_a_read_starts_no_post`: 1.0 to 1.1 s. SIGTERM while a read holds and the post's thread waits in `futex`: the bridge ends by signal 15, the read's `connectors` is gone, no post starts, the message stays `Sent`, no line. It kills no mutant; it holds item 3 of the brief for that phase.
- `adversary_a_marked_entry_keeps_its_channel`: 1.07 s.
- My 7 cases sum to about 8.4 s.

## Checked by reading only, no finding
- A poisoned `TURN` is ignored (`into_inner`); I found no panic path while it is held.
- Lock order: `TURN`, then `CHILD`. The outbox lock is never taken while either is held, so there is no deadlock.
- Starvation: the mutex is not fair, but a wait is bounded by 10 pages or the rows of one round.
- A `channel` member that is not a string reads as no channel, so it counts as the bound channel, consistent with the rule for lines without one.
- An unreadable record alternating with a refused read gives one line per run, as the design says.
- No mention of a turn file is left; `/proc/locks` appears only in the wake tests.
- The website pages name no story id, stage, wave or test count.

## Not reached
- A signal during the record change, the `read` request or the pause of a poll.
- A second signal.
- The reference document and `generated/cli/inbox.json` against the new line of a due message.
- The help of `inbox bridge` read word by word.
- Two entries of one message id in two channels: by reading, `record_chat_post` keeps one line per id.
- A restart back to the first channel.

```findings
[
{"file":"crates/inbox-cli/src/bridge.rs","line":861,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Mutant N6 (an entry without a channel counted as of another channel) survives the story's cases; killed by adversary_an_entry_without_a_channel_is_asked_about_as_of_the_bound_channel (0.12 s)."},
{"file":"crates/inbox-cli/src/outbox.rs","line":530,"category":"test-gap","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Mutant N10 (`mark_chat_post` drops the channel of the entry) survives the story's cases; killed by adversary_a_marked_entry_keeps_its_channel (1.07 s)."},
{"file":"crates/inbox-cli/src/bridge.rs","line":493,"category":"design","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Because the signal path holds the `CHILD` guard until exit, no thread ever finds the slot empty, so the park and the `ending` flag are dead code (mutants N1, N2, Ml, N12 are equivalent), and docs/design/bridge.md lines 320-325 describe a path that cannot run."},
{"file":"crates/inbox-cli/src/bridge.rs","line":901,"category":"documentation","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The line of a due message is written after the record says `due`, so a bridge ended between the two never writes it, and the design does not say the line is at most once."}
]
```
