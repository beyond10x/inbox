---
format: aep.planning-md/3
id: review-result:adversary-wake-reader-gone-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the wake whose reader is gone: 7 findings, 1 red case'
relations:
- reviews: story:a-wake-whose-reader-is-gone-marks-nothing
revision: 1
---
unit: story:a-wake-whose-reader-is-gone-marks-nothing, tree inbox-bl-reader at 5e2c100 (branch backlog/wake-reader-gone; base 9c88a0d); attacked in a copy of HEAD made with git archive
verdict: NEEDS-CHANGE
cases: executed 40→42 (inbox-cli filter 7→8, inbox-xtask claude_code_adapter 33→34), red 1
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the copy, 4 test files in it), plus logs and probes in the scratch directory (listed in part 6)
needs-coordinator: move the 2 case files and their 2 `mod` lines into the tree. Decide whether the 2 s window of the hook stays open and gets a name, or is closed

Kernel: 6.18.55-1-MANJARO. /usr/bin/sh is bash and dash is not on this host, so every hook case ran under bash only. I went over the 35-minute bound by about 10 minutes.

1. `git -C <tree> --no-pager diff --stat` prints nothing: the tree has no change from me. The copy differs from the tree in 4 test paths only:
- `crates/inbox-cli/tests/adversary_reader_gone.rs` (new)
- `crates/inbox-cli/tests/main.rs` (+ `mod adversary_reader_gone;`, in sorted order)
- `crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_3.rs` (new)
- `crates/inbox-xtask/tests/tools/mod.rs` (+ `mod adversary_claude_code_adapter_3;`)
Product files in the copy (`wake.rs`, `wake.sh`) are byte-equal to the tree's after the mutant runs. Both new files are formatted with rustfmt, and clippy with `-D warnings` is clean for both crates.

2. The cases I added, each first run alone:

**`adversary_reader_gone::rows_in_a_full_pipe_whose_reader_closes_unread_are_not_marked`** (inbox-cli): green now, 0.47 s.
- Setup: one post, then a pipe pre-filled with 15 pages, so the round's rows take the 16th slot and the pipe is full. The reader is alive and reads nothing.
- On a full pipe, the check after the write blocks in splice. The case waits until the main thread is in splice (`/proc/<pid>/task/<pid>/syscall` starts with 275), then closes the reader.
- It asserts: exit 4, one line, the post still in `Sent`.
- It kills the implementor's surviving mutant m5b. Red line under m5b:
  `panicked at crates/inbox-cli/tests/adversary_reader_gone.rs:74:13: the post was marked delivered while its rows sat unread in a full pipe, before the reader closed it: no check of the reader ran after the write`
- Seen on the way: on a full pipe with a live reader, the check blocks until there is room or the reader is gone. It holds the mutex while it waits, as the write before it already did. Nothing hangs beyond the reader's own pace.

**`adversary_claude_code_adapter_3::a_post_right_after_the_harness_is_gone_is_not_marked`** (xtask): RED now, 0.48 s.
- Setup: the harness (`sh -c '"$0" "$1" & ...; wait'`, as the turned case uses) is killed with SIGKILL. The post is written right after that, before the hook's watch looks at its parent (every 2 s).
- The hook is still alive and reads its FIFO, so the stand-in wake writes the rows and marks them. The hook's write to the harness reaches no one.
- Red output, captured when the case was written:
  `panicked at crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_3.rs:129:9: a post that arrived after Claude Code was gone was marked delivered and handed to no one: the hook still read its FIFO until the watch of its parent looked` (under /usr/bin/sh)

3. Suite runs, after both cases existed, on the unmutated copy:
- `cargo nextest run -p inbox-cli --locked --no-fail-fast -E 'test(/reader_gone/) | test(/a_closed_standard_output_ends_the_wake_without_a_mark/)'`: 8 run, 8 passed, exit 0.
- `cargo nextest run -p inbox-xtask --locked --no-fail-fast -E 'test(/claude_code_adapter/)'`: 34 run, 32 passed, 2 failed, exit 100.
  - Red: `adversary_claude_code_adapter_3::a_post_right_after_the_harness_is_gone_is_not_marked`.
  - Also red: `claude_code_adapter::the_plugin_declares_an_async_rewake_stop_hook_that_runs_the_script` (`not executable in git: ""`). It is red in every run of the copy because the copy has no git. It is not a finding and is not in the red count.
- The `before` numbers (7 and 33) come from these same runs with my 2 cases counted out. No separate run deselected them.

4. Findings (all against 5e2c100):

**F1. A post in the first 2 s after the harness is gone is still marked** (`adapters/claude-code/scripts/wake.sh:102`, warning, NEEDS-CHANGE, introduced)
- What was measured: the red case above.
- What reaches it: Claude Code crashes, or is killed with SIGKILL, and a post arrives before the watch's next look. That is up to 2 s plus one `ps` on an idle host, longer under load.
- Acceptance 3 asks that the wake be ended "before a message is marked", or that the story say what stays open. The documents only state the notice time ("within 2 s" in `docs/design/wake.md:296-300`, "within seconds" in first-use and limits). None of them says that a post inside that time is marked and lost.
- Fix lands in `docs/design/wake.md` (the `claude_code` part of "A reader that is gone") and `website/docs/limits.md`: name the window. A shorter period only narrows it.

**F2. Mutant m5b lives through every case the story wrote** (`crates/inbox-cli/src/wake.rs:321`, warning, CONFIRMED, introduced)
- The check after the write and before the first mark is removed, and all 7 story cases plus `wake::a_closed_standard_output_ends_the_wake_without_a_mark` stay green.
- My full-pipe case is the one that kills it. Fix lands: move `adversary_reader_gone.rs` into the tree.

**F3. The design overclaims what the check after the write does** (`docs/design/wake.md:134`, warning, NEEDS-CHANGE, introduced)
- The text says "A round whose reader went away after its rows were written sends no mark".
- The check runs once (`wake.rs:322`). A reader that closes after it, with the rows still unread in the pipe, gets every mark of the round: both between the check and the first mark, and between 2 marks (the mutex keeps the watch out, by design).
- "What stays open" names only a reader that took the rows. A reader that left them unread is not named.
- This is from reading the code. I wrote no case: it needs a stand-in service that holds the first mark.
- Fix lands in `wake.md:132-137` and `limits.md`.

**F4. A story case is flaky on the product as it is** (`crates/inbox-cli/tests/reader_gone.rs:452`, warning, NEEDS-CHANGE, introduced)
- `reader_gone::a_hook_ended_by_sigkill_leaves_a_wake_that_ends_without_marking` went red once in 4 runs: `panicked at crates/inbox-cli/tests/reader_gone.rs:452:5: assertion failed: !holds_a_lock(wake)`.
- That run had mutant r2 in place, but r2 only changes `deliver`, and this case sends no post, so the mutant is not the cause.
- Likely cause, from reading: `gone()` counts a zombie thread-group leader as ended, while another thread of the wake is still exiting and still holds the file table and the flock.
- Fix lands in the case: wait for `!holds_a_lock(wake)`, or for `/proc/<pid>` to be gone.

**F5. Mutant s2 lives through every case** (`adapters/claude-code/scripts/wake.sh:106`, warning, CONFIRMED, introduced)
- Mutant: the watch compares the parent with `1` instead of `$harness`. Every story case stays green (only my already-red case fails).
- Under it, every hook whose harness is alive is ended at the first look, 2 s after it starts. No case keeps a hook with a live harness past one look.
- Fix lands: a case with a stand-in `ps` on PATH that logs its runs. After 2 runs, assert the hook and its wake are still alive.

**F6. The hook's read loop is quadratic in the size of the rows** (`adapters/claude-code/scripts/wake.sh:132-143`, note, CONFIRMED, introduced)
- Measured with a scratch probe under bash, against `cat`, which took milliseconds:

| rows | read loop |
|---|---|
| 6.6 kB | 0.017 s |
| 2.0 MB | 1.8 s |
| 8.0 MB | 24.8 s |

- The bytes the session gets are the same as with `cat`.
- What reaches it: one wake returns every row in `Sent`. 100 posts with bodies near the 64 KiB bound are about 6.5 MB, so the session is woken tens of seconds late. Nothing is lost.

**F7. The changed assertion in `wake.rs` is weaker** (`crates/inbox-cli/tests/wake.rs:554`, note, CONFIRMED, introduced)
- "1 request" became "every request is a Pending read". A wake that sends a second, needless read now passes.
- The change has a reason (the watch's timing varies). An upper bound of 2 reads would keep that defect caught.

5. Mutants:
- m5b (check after the write removed): killed only by `adversary_reader_gone::rows_in_a_full_pipe_whose_reader_closes_unread_are_not_marked`.
- r2 (the round's guard dropped before the marks): survived. The one red in that run was the flaky case of F4, unrelated to the mutant.
- s1 (`exec 5>&-` removed, descriptor 5 never closed): killed by 7 cases. 15 more hung to the 60 s nextest bound and were killed there (TIMEOUT).
- s2 (parent compared with 1): survived.
- s3 (watch period 30 s): killed by `adversary_claude_code_adapter::a_harness_that_is_gone_gets_no_post_marked_delivered`.
- s4 (no watch at all): killed by the same case.

6. Not checked, out of time:
- Direct splice probes: a pipe closed with unread bytes, a third process holding the read end, a FIFO opened read-write, SIGPIPE at default.
- Descriptor count over 200 checks (by reading, each check closes its own pipe), and memory and CPU over 60 s.
- The hook under each signal (TERM, INT, HUP, STOP then KILL), the watch killed alone, the wake killed alone.
- A planted FIFO or link in the directory name; `mkfifo` or `ps` missing, or a `ps` with odd output; a parent id of 1 at the start; a harness that starts the hook through `sh -c`.
- Byte-for-byte output on a terminal, file, `/dev/null` or socket against 0.6.0; `--follow` under `script`.
- The 6+2 cases 20 times in a row. Whether `a_signal_to_the_hook_ends_its_wake` can reach 60 s without a mutant.
- Measured times behind "about a second", "500 ms" and "2 s" in the documents.

7. Paths written outside the worktree, all under <scratch>/bl/reader/adversary/:
- `copy/` (its `target` is deleted). It holds:
  - copy/crates/inbox-cli/tests/adversary_reader_gone.rs
  - copy/crates/inbox-cli/tests/main.rs
  - copy/crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_3.rs
  - copy/crates/inbox-xtask/tests/tools/mod.rs
- Logs: run-cli-alone.log, run-xtask-alone.log, mutant-m5b-no-premark-check.log, mutant-r2-guard-dropped-before-marks.log, mutant-s1-fd5-open.log, mutant-s2-parent-1.log, mutant-s3-period-30.log, mutant-s4-no-watch.log, mutants-rs.done, mutants-sh.done
- Scripts and saved originals: mutants-rs.sh, mutants-sh.sh, wake.rs.orig, wake.sh.orig
- Probe: probe-read.sh and the probe/ directory (about 8 MB of rows: line and rows files)

```findings
[
  {"file": "adapters/claude-code/scripts/wake.sh", "line": 102, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A post that arrives in the up to 2 s between a harness's death and the watch's next look is read by the still-living hook, marked delivered and handed to no one, and no document names that window as open."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 321, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Removing the check after the write (m5b) leaves every story case green; only the adversary full-pipe case kills it."},
  {"file": "docs/design/wake.md", "line": 134, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The design says a round whose reader went away after its rows were written sends no mark, but the check runs once, so a reader that closes after it with the rows unread still gets every mark of the round."},
  {"file": "crates/inbox-cli/tests/reader_gone.rs", "line": 452, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The story's SIGKILL case went red once on a path no mutant touched, because gone() counts a zombie thread-group leader as ended while another thread still holds the flock."},
  {"file": "adapters/claude-code/scripts/wake.sh", "line": 106, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "A watch that compares the parent with 1 instead of the first parent, which ends every hook 2 s after its start, survives every case."},
  {"file": "adapters/claude-code/scripts/wake.sh", "line": 132, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The hook's read loop is quadratic in the size of the rows (2 MB in 1.8 s, 8 MB in 24.8 s under bash, against milliseconds for cat), so a large backlog wakes the session tens of seconds late."},
  {"file": "crates/inbox-cli/tests/wake.rs", "line": 554, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The changed assertion no longer catches a wake that sends a second, needless Pending read, which the old count of one request caught."}
]
```
