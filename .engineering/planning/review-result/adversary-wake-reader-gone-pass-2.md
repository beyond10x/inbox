---
format: aep.planning-md/3
id: review-result:adversary-wake-reader-gone-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the wake whose reader is gone: 2 findings, 2 red cases'
relations:
- reviews: story:a-wake-whose-reader-is-gone-marks-nothing
revision: 1
---
unit: story:a-wake-whose-reader-is-gone-marks-nothing, round 2 (adapters/claude-code/scripts/wake.sh); tree inbox-bl-reader HEAD 01720d2, attacked in a git-archive copy
verdict: NEEDS-CHANGE
cases: executed 116→118, red 2 (inbox-xtask; `<before>` is my second suite run with my 2 cases deselected: 118 run minus 2. The copy has no git, so `the_plugin_declares_an_async_rewake_stop_hook_that_runs_the_script` is also red, as the brief expects. It is not counted.)
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/bl/reader/adversary2/ (all scratch is under it)
needs-coordinator: move the 2 test-file changes from the copy into the tree

1. Diff (the copy has no git, so this is `diff -r` of the copy against the tree)
- New: crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_4.rs
- Changed: crates/inbox-xtask/tests/tools/mod.rs (one line added, `mod adversary_claude_code_adapter_4;`, in sorted order)
- Both are test files. Nothing was written in the tree.
- The tree has an untracked .engineering/evidence/story/a-wake-whose-reader-is-gone-marks-nothing/. It is not mine.

2. Cases added
Both live in <scratch>/bl/reader/adversary2/copy/crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_4.rs. Both are red now on the story's hook and green on the 0.6.0 hook (`git show 9c88a0d:…/wake.sh`, swapped into the copy and then restored; cmp confirmed the restore). Each takes under 0.6 s.

a) `a_full_tmpdir_does_not_drop_a_row_the_wake_marked`
- It asserts: if the wake marked its row, the frame holds the row or the hook exits 1.
- The case stands in for a full or over-quota `$TMPDIR` with `trap '' XFSZ; ulimit -f 0`, so every write to `text` fails.
- Red output, run alone:
  `panicked at …adversary_claude_code_adapter_4.rs:81:5: the wake marked its row and the hook handed on nothing and said nothing: exit Some(0), standard error ""`
- Probe, 41-line row: with a 0 KiB limit the new hook exits 0, says nothing, and the row is marked. With 1 KiB it exits 2 and hands on 19 of 40 body lines, the row is marked, and there is no note line.
- The 0.6.0 hook in the same probe: the wake's own write fails, no mark, exit 1 with its line, or exit 2 with the "may not be marked" note.
- Cause: wake.sh:137 and :139 (`printf … >&6`) are never checked. `true > "$text"` at :91 succeeds on a full disk, because an empty file takes no block.
- What reaches it: any full or quota-limited `$TMPDIR`. On this host `/tmp` has a per-user quota (machine rules).
- Fix in wake.sh: when a write to `text` fails, close descriptor 4 at once and end the wake before it marks (the wake then sees its reader gone and exits 4). Or hold the rows without a file. Then add a row to docs/design/wake.md (exit table near :390) and to website/docs/limits.md.

b) `a_signal_inside_a_line_keeps_the_bytes_read_before_it`
- Setup: the stand-in writes `m-1 note\n    first body\nm-2 note\n    second bo` and blocks. SIGTERM goes to the hook.
- It asserts the frame holds `    second bo`.
- Red output, run alone:
  `panicked at …:117:5: the bytes `    second bo` the wake wrote are not in the frame: "inbox …: 2 message(s) follow … \nm-1 note\n    first body\nm-2 note\ninbox …: end\n"`
- Under bash, a `read` that a trapped signal interrupts throws away the bytes of the line it held.
- Probe with a stand-in that finishes its line after TERM: the frame reads `m-1 head\nial`, the count goes from 1 to 2 messages, and that row was marked. 0.6.0 hands on `  partial`.
- What reaches it: SIGTERM or SIGHUP from the harness, or from the watch, while the wake's write of a round over 64 KiB is cut mid-line. The real wake dies at once on SIGTERM, so the row whose line is cut is not marked. The damage is a frame that loses bytes and miscounts, not a lost mark. Severity: warning.
- A second probe sent TERM while the read loop was still working through rows already in the FIFO (12 tries). No loss there, so it seems to need a `read` that is blocked.
- Contract drift: wake.sh:128-131 says "the bytes are the wake's", and docs/design/wake.md:391 says "rows the wake printed before it ended are handed on in the frame". Both are false for that line.
- Fix: in the `ended` branch at wake.sh:140, keep `$line` when it is not empty (bash fills it before the interrupt; dash may not), or document the loss.

3. Suite run, after the cases existed
Command: `cargo nextest run -p inbox-xtask --locked --no-fail-fast` in the copy
Result: `Summary [17.475s] 118 tests run: 115 passed, 3 failed`
The 3 failures are my 2 cases plus `the_plugin_declares_…` (copy without git).
`cargo clippy -p inbox-xtask --all-targets --locked -- -D warnings`: clean, after I removed one `mut`.

Ten runs in a row of the reader cases, no slot prefix (xtask `test(claude_code)` with my cases and the git case left out, 35 cases; inbox-cli `test(reader_gone) | test(/^wake::/)`, 26 cases):
- 0 reds in 10 × 61 case runs.
- Load average 74.9 to 94.5.
- Slowest case seen: `a_hook_whose_harness_is_alive_outlives_the_first_look_of_its_watch`, 2.65 s.
- Step times: xtask 2.9 to 3.4 s, cli 2.9 to 8.1 s.

Mutants (each run on xtask `test(claude_code)` and the cli filter):
- M1 printf to `> "$text"` on every line (truncates): killed by rows_without_a_last_line_end_are_handed_on_byte_for_byte and 11 more
- M2 `printf '%s'` without the line end: killed by rows_without_a_last_line_end_are_handed_on_byte_for_byte and 12 more
- M3 `text=$dir.text` (file beside the directory): killed by a_hook_whose_harness_is_alive_outlives_the_first_look_of_its_watch, rows_without_a_last_line_end_are_handed_on_byte_for_byte and 7 more
- M4 the `elif [ -n "$line" ]` branch disabled (last line without a line end dropped): killed by rows_without_a_last_line_end_are_handed_on_byte_for_byte
- M5 `[ -s ]` changed to `[ -e ]`: killed by a_wake_that_ended_with_0_and_no_rows_leaves_the_session_alone and 5 more
- M6 the trap removes only `rows`, not the directory: killed by a_name_that_exists_is_passed_over_and_never_used and 8 more
- The inbox-cli reader cases (26) stayed green under every mutant. They do not run the hook, so they kill none of these.
- `cat` of the FIFO instead of the file: not run. It blocks on open, so it would hang until the 60 s kill instead of giving a clean signal.

4. Judgement findings
Only the two above (findings a and b), against HEAD 01720d2 in the copy.

5. Attacked and not broken
- Bytes against 0.6.0, under bash: last line without a line end, a line of only spaces, empty lines, a backslash at the end of a line, `-n` / `-e` / `%s` as whole lines, CRLF, bytes that are not UTF-8, a 1.1 MB line, exactly 6000 and 6001 bytes, the 300-byte cut inside a 2-byte character. Every frame byte was identical.
- A NUL byte is dropped by bash `read` (1 byte off). It is infeasible: output.rs escapes control characters, so the real wake never writes a NUL.
- Descriptors: the wake holds 0, 1 (the FIFO), 2 and its script fd only. The watch holds 0, 1 and 2 on /dev/null. Neither holds 3, 4, 5 or 6.
- File and directory: `text` is 600 and the directory 700 under the script's own `umask 077`, whatever umask it inherits. The directory is freshly made by `mkdir`, so a planted link or FIFO cannot sit at `text`.
- SIGINT to a hook started with `&` from a non-interactive shell is ignored on entry, so it cannot be trapped. That is a probe artifact, not reached through the harness.

Not reached:
- dash
- a read-only `$TMPDIR`
- what a SIGKILL of the hook leaves behind (by reading the code: the directory with `text` holding rows the wake did not mark, mode 600; the next hook has another pid and does not trip over it)
- a SIGKILL of the wake
- the documents of item 6 beyond wake.md:391, including the sentence in first-use.md
- item 7 (a third process opening the FIFO read-write; `mkfifo` or `ps` missing from PATH; odd `ps` output; parent id 1 at the start)
- an inbox-cli case file (none written)

6. Paths written outside the tree, all under <scratch>/bl/reader/adversary2/:
- copy/ (its target/ is deleted)
- case4.log, runs.log, mutants.log, repeat.log
- wake.sh.new
- probe/ holding bytes.sh, full.sh, sig.sh, sig2.sh, runs.sh, mutants.sh, repeat.sh, old.sh

```findings
[
  {"file": "adapters/claude-code/scripts/wake.sh", "line": 137, "category": "boundary", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A write to `text` that fails (full or over-quota TMPDIR) is not checked, so the wake marks rows the hook dropped, and with every write failed the hook exits 0 and says nothing; 0.6.0 marked nothing in that case."},
  {"file": "adapters/claude-code/scripts/wake.sh", "line": 140, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "SIGTERM or SIGHUP while `read` waits inside a line drops the bytes read so far, so the frame loses them and can count a fragment as a message, against the script's 'the bytes are the wake's' and docs/design/wake.md:391."}
]
```
