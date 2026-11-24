---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-read-pass-3
kind: review-result
status: active
title: 'Adversary pass 3 on the chat bridge that marks read: no finding, 1 case added, green'
relations:
- reviews: story:the-chat-bridge-marks-a-message-read-on-a-reaction
revision: 1
---
ADVERSARY PASS 3

Adversary pass 3 on story:the-chat-bridge-marks-a-message-read-on-a-reaction, round 4 (code 724b6a7, tree at 6bd86e6). No findings: the round-4 claim, the signal cases and the new sentences hold.

I added one green case. It is appended, uncommitted, to crates/inbox-cli/tests/adversary_bridge_read.rs; `git diff --stat` shows only that file, 83 lines added. Every job ran on build-1d ("slot: synced to build-1d"). Logs are in <scratch>/s4/read/adversary3/.

**1. Round 4 claim (by reading).** Only `invoke` fills and empties the slot, and it never locks it again after emptying it; the claim is right.
- It fills the slot once, after `spawn` under the guard (bridge.rs:447-462).
- It empties it in two places: `held.take()` before `break status` (line 508), and in the error path with kill and wait (line 513). After either, no code locks the slot again.
- A failed `spawn` returns with the slot still empty.
- The signal path takes the guard and holds it until the process ends (wake.rs:666).
- So the remaining defensive return can't be reached, as the comment and the design say.

**2. Signal cases, `--stress-count 5`: `5/5 stress run iterations: 5 passed`.** None went red. The 6 cases:
- `bridge::a_signal_during_a_post_ends_the_group_of_the_connectors_child_with_the_bridge`
- `adversary_bridge::a_signal_to_the_bridge_leaves_no_connectors_child_running`
- `adversary_bridge::a_signal_to_the_bridge_leaves_no_process_of_the_post_running`
- `adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`
- `adversary_bridge_read::adversary_a_signal_during_a_read_ends_it_and_names_nothing`
- `adversary_bridge_read::adversary_a_signal_while_a_post_waits_for_a_read_starts_no_post`

`bridge_read.rs` has no case with "signal" in its name.

**3. Round 4 sentences against the code: all match.**
- docs/design/bridge.md, "The end by a signal": the blocked thread, the defensive check that no path reaches, and the reason F4 has no case.
- docs/design/bridge.md, the line of a due message is at most once: matches bridge.rs:890-908.
- `CHANGELOG.md`: "named in at most one line".
- website/docs/limits.md: "A bridge stopped just before that line never writes it."
- I read the round 4 diff of `AGENTS.md` only in part, since its row is one long line; no conflict seen.
- No mention of `ENDING` or `ending()` is left in the sources.

**4. `--chat-poll-interval`:** its help in main.rs:155-160 matches `generated/cli/inbox.json` lines 719-724 word for word: value name `SECONDS`, default `10`, "at least 1".

**5. New case `adversary_a_restart_on_another_channel_and_back_asks_about_the_first_channel_again`: green, 2.3 s.**
- A bridge on `C0000000001` posts 2 notes and is stopped, then the first note is read with `inbox read`.
- A bridge on `C0000000002` posts a note and reads only its own channel. It does not settle the first channel's entry whose message left `Pending`.
- Back on `C0000000001`: that entry is settled without a `read`. The other entry is asked about with exactly `{"channel":"C0000000001","oldest":"1700000000.000002","inclusive":true,"limit":100}` and read on its reaction. The second channel's entry stays `posted`, and nothing is written to standard error.
- It holds the claim and kills no named mutant.

**Signal phases of a poll (by reading only, no case):**
- A signal during the record change leaves at most a half-written `<record>.new`. The record itself is replaced only by rename, and the next change removes whatever is at the staged name.
- A signal during the poll's `read` request is covered by the at-least-once rule.
- During the poll's pause the signal path finds an empty slot, holds the guard and ends the process.
- A second signal is taken by the signal handler and never read; the process ends after `end_group`.

**Runs:**
- All 8 of my cases: `8 tests run: 8 passed`. Clippy `-p inbox-cli --all-targets --locked -- -D warnings` exits 0.
- My pages case `adversary_a_poll_reads_ten_pages_and_fails_at_an_eleventh` took 4.8 s in this run (1.3 to 1.6 s before), close to the 5 s bound for a case.

**Not reached:**
- An end-to-end case for a signal during the poll's record change, its `read` request or its pause.
- A second signal as a case.
- The `AGENTS.md` row word by word.
- A mutant for the settle across channels: the new case would catch a settle applied to another channel's entries, but I did not run that mutant.

```findings
[]
```
