---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-posts-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the chat bridge that posts: 2 findings, 1 red case'
relations:
- reviews: story:the-chat-bridge-posts-a-pending-message
revision: 1
---
unit: story:the-chat-bridge-posts-a-pending-message, adversary pass 2, tree inbox-bl-s4-post at wip b1222fa (round 2 on main 2f17090), cases run in a fresh copy of b1222fa
verdict: CONFIRMED
cases: executed 329→333, red 1
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s4/post/adversary2/ (listed in part 6)
needs-coordinator: move copy/crates/inbox-cli/tests/adversary_bridge.rs into the tree (`tests/main.rs` is unchanged; the `mod` line is already there)

Round 2 holds up mostly. One new case is red: a helper process that `connectors` starts outlives a bridge stopped by SIGTERM. One mutant of the merged wake survives every case, and it was already that way on main.

**1. Diff of the tree**

`git diff --stat` of the tree is empty: I wrote nothing there. After the host restart I restored `crates/inbox-cli/src` from b1222fa and touched the files. A `diff -r` of `crates/inbox-cli` against `git archive b1222fa` (in `restored.diff`) shows only `tests/adversary_bridge.rs`.

**2. Cases added at the end of `adversary_bridge.rs`**

The stand-in got one new switch, `grandchild-first`: it starts a helper that holds no pipe of the bridge and writes its pid to `gpid`. The times are from the run on restored sources.

- **`a_signal_to_the_bridge_leaves_no_process_of_the_post_running`** (group 1): red, 0.40 s.
  - Its first run alone, in `new.log`, gave this red line: `the helper 2193025 that the connectors child started outlives the bridge ended by SIGTERM`.
  - The `connectors` child itself is killed as round 2 promises; that assertion passes.
- **`an_answer_of_exactly_the_bound_is_a_post_made`** (group 6): green, 0.52 s. It kills mutant G2.
- **`a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`** (groups 3 and 6): green, 1.74 s.
  - It checks one line over 4 tries of the start, and that a SIGTERM during the pauses of the start ends the bridge within 2 s.
  - It kills the say-once mutant in G1.
- **`a_bridge_whose_standard_output_has_no_reader_runs_on`** (group 4): green, 1.81 s.
  - With standard output a pipe without a reader, the bridge runs on, posts and marks.
  - It kills the mutant in G1 that gives the bridge a watch; that bridge exited with 4.

The 5 cases of pass 1 are all green on b1222fa.

**3. Runs** (logs in the scratch directory)

- `new.log`, the 4 new cases alone: `4 tests run: 3 passed, 1 failed`.
- `g1.log`: 3 mutants at once (the start line printed on every try; `let _ = running.wait();` dropped from the signal path; `watched()` of the bridge starts a `Watch`). Result: `24 tests run: 21 passed, 3 failed`.
  - The say-once mutant is killed by `a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`.
  - The watch mutant is killed by `a_bridge_whose_standard_output_has_no_reader_runs_on`.
  - The dropped wait is killed by no case. `bridge::a_signal_during_a_post_ends_the_connectors_child_with_the_bridge` passes, because SIGKILL takes effect without the wait; the only effect is a zombie for the moment before the exit. This mutant is equivalent, not a finding.
- `g2.log`: `read > room` → `read >= room`. Result: `22 passed, 2 failed`; killed only by my `an_answer_of_exactly_the_bound_is_a_post_made` (the note was not Delivered within 10 s). The other red case in that run is the helper case, which is red anyway.
- `g3.log`: `DRAIN` set to 0 s. Result: `23 passed, 1 failed`, and that one is only the helper case, which is red anyway.
  - The mutant survives on its merits. That is expected: the bytes are in the pipe at the exit, and a drain of 0 still gets them when the reader threads win the race. This is a note, not a finding.
- `w1.log`: `surface.watched()?;` removed from `rounds`. Result: `43 passed, 6 failed`. Killed by 5 `reader_gone::` cases and by `wake::a_reader_that_closes_during_the_marks_of_a_round_does_not_cut_the_round`. You wrote 5 red; the log has 6.
- `w2.log`: the check `output::reaches_a_reader()` after the rows and before the first mark (wake.rs:469) replaced. Result: `48 passed, 1 failed`; killed only by `adversary_reader_gone::rows_in_a_full_pipe_whose_reader_closes_unread_are_not_marked`.
- `w3.log`: `*watch.hold() = true;` at the end of `run` (wake.rs:312) replaced by `let _ = &watch;`. Result: `49 tests run: 49 passed`. **This mutant survives.**
- `suite.log`, on restored sources: `SLOT_TIMEOUT=30m cargo nextest run -p inbox-cli --locked --no-fail-fast` gave `333 tests run: 332 passed, 1 failed`, exit 100, step 20.8 s. The red case is mine.
- `clippy.log`: `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0.
- About the counts: 329 is that run minus my 4 new cases. I did not run a separate suite without them, because the crash killed that step and the brief says not to repeat.

**4. Findings**

- **P2-F1 — a helper of `connectors` outlives the bridge on SIGTERM.** `crates/inbox-cli/src/wake.rs:662-664` (the signal path) together with `crates/inbox-cli/src/bridge.rs:381` (the spawn). CONFIRMED, introduced, note.
  - The signal path kills only the pid it holds. The child is not in a process group of its own, and the group is never killed, so a process the child started runs on.
  - `docs/design/bridge.md` says "No `connectors` child outlives a bridge ended by one of these signals", and the limits page says the signal "ends a post that is still running". Neither names a helper.
  - What reaches it: a `connectors` that starts a helper. Not shown for a released `connectors`.
  - Fix: spawn the child as the leader of its own process group and kill the group in the signal path, or name the limit in both documents.
- **P2-F2 — surviving mutant W3: the hold at the end of `run`.** `crates/inbox-cli/src/wake.rs:312`, `*watch.hold() = true;`. CONFIRMED, pre-existing, note.
  - The line is the same on main 2f17090 (wake.rs:200), so this is pre-existing.
  - With it removed, all 49 wake, `reader_gone` and `follow` cases stay green.
  - What it guards: after a wake ended in another way, a watch that sees the reader go would exit with 4 and write a second line.
  - Case that would kill it: a wake ending with a refusal (exit 1) while its standard output pipe loses its reader in the window before the process exit. It must show exactly one line and status 1. I did not write it because the window is a race, and a case that holds it deterministically needs a hook in the code. Write it in the story of the reader that is gone.

**5. Checked and found sound**

- **Signal path, timing:**
  - A signal between the spawn and the store waits on the mutex, because `invoke` takes the slot before the spawn; then it kills the stored child.
  - The main thread holds the slot only for `try_wait`, never across a blocking wait, so the signal thread does not wait forever.
  - Between the child's exit and the taking out, `try_wait` reaps and takes under one lock. During the drain the slot is empty, so the signal just ends the process.
  - A second signal while the first is handled changes nothing: the handler thread takes the first and the process ends by its default action.
- **Wake without a child:** the slot is `None`, so `end_on_signal` behaves as on main.
- **Drain:** a child's bytes are read to EOF by the reader threads before the verdict. The 10 ms look is `recv_timeout` plus `try_wait`, not a busy loop. I did not measure CPU time.
- **A child that closes both streams and never exits** loops forever. That is named: "a `connectors` that never ends holds the bridge".
- **Answer shapes:** exit 1 with any classification (`unknown`, even `applied`) is a failed post, with no mark and the same key `inbox-<id>` on the next try. No path marks a message without a post or uses another key.
- **`retry_after_seconds`:** 0, 3601, 1.5, a string, or a value in `service_code` only gives no wait of its own; the unit cases hold this.
- **Documents:** round 2's sentences in `docs/design/bridge.md` (LOOK, DRAIN, signal path, start tries, staged file, record bound) match the code, apart from P2-F1. The limits page and the CHANGELOG bullet name no story id, stage, wave or test count.

**Not reached**

- CPU time of a waiting bridge: not measured.
- An answer of 64 KiB in one write before the exit: not run as a case (read by code only).
- An answer over 256 KiB from a post that was made, run end to end: by reading the code, it is never marked and is retried for good, with a replay of the same size each time. Not reachable with real answers of a few KiB, so not raised.
- A refusal at the start that cannot pass (wrong key): not run again.
- The `AGENTS.md` bridge row: not compared sentence by sentence.

**6. Paths written outside the worktree**

All under `<scratch>/s4/post/adversary2/`:

- `copy/` — the copy of b1222fa; its `target` is removed.
- `copy/crates/inbox-cli/tests/adversary_bridge.rs` — the file to move.
- Logs: `build0.log`, `new.log`, `g1.log`, `g2.log`, `g3.log`, `w1.log`, `w2.log`, `w3.log`, `suite.log`, `clippy.log`.
- Mutant diffs: `g1.diff`, `g2.diff`, `g3.diff`, `w1.diff`, `w2.diff`, `w3.diff`, and `restored.diff`.
- Scripts and their output: `pass2.sh`, `pass2b.sh`, `waitfor.sh`, `pass2.out`, `pass2b.out`, `pass2.done`.

No job of mine is still running.

```findings
- file: crates/inbox-cli/src/wake.rs
  line: 662
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the signal path kills only the connectors child, not its process group, so a helper the child started outlives a bridge ended by SIGTERM, against the design's sentence that no connectors child outlives it.
- file: crates/inbox-cli/src/wake.rs
  line: 312
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: removing the hold of the watch at the end of run leaves every wake, reader_gone and follow case green.
```
