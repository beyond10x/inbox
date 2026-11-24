---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-posts-pass-3
kind: review-result
status: active
title: 'Adversary pass 3 on the chat bridge that posts: round 3, nothing found'
relations:
- reviews: story:the-chat-bridge-posts-a-pending-message
revision: 1
---
unit: story:the-chat-bridge-posts-a-pending-message, adversary pass 3, tree inbox-bl-s4-post at wip 5a24535 (round 3 on 6ad870d), cases run in a fresh copy of 5a24535
verdict: nothing found
cases: executed 336→336, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s4/post/adversary3/ (listed in part 6)
needs-coordinator: move <scratch>/s4/post/adversary3/copy/crates/inbox-cli/tests/adversary_bridge.rs into the tree; it is the only file that differs from 5a24535

I found nothing new in round 3. Its fix of P2-F1 holds: my helper case is green on 5a24535 and red when the signal path kills the pid alone.

**1. Diff of the tree**

`git diff --stat` of the tree is empty: I wrote nothing there. `restored.diff` compares the copy after the mutant was reverted with `git archive 5a24535`: one file differs, `crates/inbox-cli/tests/adversary_bridge.rs`.

**2. Changes to my cases (task 1)**

- **`a_signal_to_the_bridge_leaves_no_process_of_the_post_running`:**
  - It no longer sleeps 200 ms and looks once. A new helper `gone(pid)` polls `/proc` every 10 ms until the process has ended or is a zombie, and gives up only at `HANG` (10 s). That bound only ends a hang.
  - The stand-in's holds now run at most 3000 × 10 ms, so at least 30 s, longer than `HANG`. That covers the child's hold, the `grandchild` and the `grandchild-first` helper. A process that outlives the bridge therefore cannot end by itself inside the bound.
  - Green on 5a24535: 0.33 s.
  - Red under mutant M1 at 10.6 s, the bound, with this line: `the helper 2263668 that the connectors child started outlives the bridge ended by SIGTERM`.
- **Other cases with the same shape:**
  - `a_signal_to_the_bridge_leaves_no_connectors_child_running` had the same 200 ms pause and single look. It now uses `gone(child)` too. Green, 0.42 s.
  - `a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal` held a wall-clock bound of 2 s for the exit after SIGTERM, which a loaded host can break. It now waits up to `HANG`. Green, 1.81 s.
- **Cases left as they are:** each waits a fixed time and then checks that something is still true, so a loaded host can only make them falsely green, never falsely red.
  - `a_bridge_started_before_the_service_answers_waits_for_it` (1 s, then "still running").
  - The 1600 ms in the say-once case.
  - `a_bridge_whose_standard_output_has_no_reader_runs_on` (1500 ms, then "still running"; the watch period is 500 ms).
  - `a_grandchild_holding_the_pipes_does_not_hold_the_post_of_an_ended_child` polls for 5 s, and its grandchild now lives at least 30 s.

**3. Runs** (3 pool jobs; logs in the scratch directory)

- **`m1.log`**, mutant M1 with `test(/bridge::/)`:
  - The mutant, in `m1.diff`: wake.rs:663-664 becomes `if let Some(mut running) = child().take() { let _ = running.kill(); let _ = running.wait(); }` in place of `end_group(running)`.
  - Result: `25 tests run: 23 passed, 2 failed`, the same on nextest's second try.
  - Killed by `adversary_bridge::a_signal_to_the_bridge_leaves_no_process_of_the_post_running` and by `bridge::a_signal_during_a_post_ends_the_group_of_the_connectors_child_with_the_bridge` (`-INT: the helper 2266241 of the child outlives the bridge`).
- **`suite.log`**, on unchanged sources: `SLOT_TIMEOUT=30m cargo nextest run -p inbox-cli --locked --no-fail-fast` gave `336 tests run: 336 passed, 0 skipped`, exit 0, step 51.1 s. Every `adversary_bridge` case is under 2.5 s.
- **`clippy.log`**: `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0.

**4. Findings**

None.

**5. Checked and found sound (task 2)**

- **Which group gets the kill:**
  - The child is taken out of the slot under the same lock as `try_wait` reaps it. So the signal path never holds a reaped child, and a pid that is not yet reaped cannot be reused: the group id names this child's group and no other.
  - A spawn that fails stores nothing.
  - Because of `process_group(0)` the child's pid is its group id. That is never the bridge's group, and never the job group of its parent shell.
  - A child that calls `setpgid` or `setsid` itself still gets the second SIGKILL to its own pid. Processes that left the group are not reached, which the design says.
- **Order:** group kill, then child kill (harmless on a zombie), then wait. It is correct, and the wait makes sure the child is gone before the exit.
- **Process setup:** a bridge that is itself a group or session leader, or has no controlling terminal: `setpgid(0,0)` in the child works in all of these.
- **SIGHUP:** only SIGINT and SIGTERM are registered with `Signals`, so SIGHUP ends the bridge by its default action outside the signal path. The child's group, not in the foreground and not stopped, runs on. This matches the new sentences in `docs/design/bridge.md` and `limits.md`.
- **Interrupt and suspend keys:** the sentences about them match the code: the child is in another group, and its 3 streams are pipes.
- **A helper of a post that finished by itself is not signalled:** this matches the code (the slot is empty after the reap) and is held by `bridge::a_helper_of_a_post_that_ended_by_itself_is_not_ended_by_the_bridge`. That case looks once right after the bridge ends, so a mutant that killed such a helper would most likely, though not certainly, turn it red.
- **The bridge sentence of `AGENTS.md`** (group, `kill_process_group`) matches the code.

**Not reached**

- A child that calls `setsid` was not run as a case (read in the code only).
- A real terminal hang-up was not run as a case.

**6. Paths written outside the worktree**

All under `<scratch>/s4/post/adversary3/`:

- `copy/`, the copy of 5a24535; its `target` is removed.
- `copy/crates/inbox-cli/tests/adversary_bridge.rs`, the file to move.
- Logs: `m1.log`, `suite.log`, `clippy.log`.
- `m1.diff`, the mutant; `restored.diff`, the restore check.
- Script and its output: `pass3.sh`, `pass3.out`, `pass3.done`.

No job of mine is still running.

```findings
[]
```
