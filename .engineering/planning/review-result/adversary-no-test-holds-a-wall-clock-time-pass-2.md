---
format: aep.planning-md/3
id: review-result:adversary-no-test-holds-a-wall-clock-time-pass-2
kind: review-result
status: active
title: Adversary pass 2 on story:no-test-holds-a-wall-clock-time
relations:
- reviews: story:no-test-holds-a-wall-clock-time
revision: 1
---
unit: story:no-test-holds-a-wall-clock-time, pass 2, tree <tree> at 6805268 (round 2 = `git diff 062e614 6805268`)
verdict: NEEDS-CHANGE
cases: executed 154→154 (package inbox-cli; no case added, 4 probe cases ran green and were moved to scratch), red 0
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 13 paths (part 6)
needs-coordinator: none

Summary: nearly all of round 2 holds. The one thing that still needs a change: a pairwise read of the epochs file escapes the step counter if it takes each id's text once and compares the strings. The step case passes it in 14.0 s, while the code comment and the changelog say every comparison is counted. Two other mutants survive, both small (F2, F3). Under 3 s plants every changed case passed. My pass-1 cases are green now.

1. git --no-pager diff --stat
(empty: every plant and mutant is restored, and the probe file was moved out of the tree to scratch.)

2. Cases added: none kept. I wrote 4 probe cases in `crates/inbox-cli/tests/adversary_clock_file_2.rs` and ran them alone:
- a symbolic link to a FIFO;
- a FIFO whose writer holds it open;
- a directory, then a symbolic link to `/dev/null`;
- a symbolic link to a regular file.

All 4 passed: `4 tests run: 4 passed, 154 skipped`. The first three are refused with exit 4 and `it is no regular file`; the regular file is read through. They show the implementor's F6 claim holds and are not findings, so the file is now at `.../adversary2/probe-adversary_clock_file_2.rs` and the `mod` line is removed.

3. Suite run on the clean tree: `ESS_TOOLCHAIN=0.97.3 cargo nextest run -p inbox-cli --locked --no-fail-fast` → `Summary [ 9.048s] 154 tests run: 154 passed, 0 skipped`, exit 0. My 3 pass-1 cases are green, which confirms the F6 and F7 fixes.

4. Findings

**F1. crates/inbox-cli/src/outbox.rs:289** (mutant, warning, NEEDS-CHANGE, introduced)
- What it is: the `LineId` comment says a read in pairs "is counted at the comparison, however it is written", and CHANGELOG says "counts the steps of the read, at each comparison of two entries". That holds only for comparisons of `LineId` values.
- What was measured: mutant M4c. It takes each id's text once (one step per line, through the sanctioned `text()`), then compares the `&str` values in pairs:
  ```
  - let mut named: HashMap<LineId<'a>, (usize, &'a str)> = HashMap::new(); for line in lines { ... }
  + let lines: Vec<(&'a str, &'a str)> = lines.map(|line| (line.id.text(), line.digits)).collect();
  + .filter(|(id, _)| lines.iter().filter(|(other, _)| other == id).count() == 1)
  ```
  `outbox::tests::an_epochs_file_of_20000_lines_is_read_in_steps_linear_in_its_lines`: PASS in 14.037 s.
- What reaches it: any later change to `Epochs::parse` that compares text instead of `LineId`.
- Fix (in `outbox.rs` and the CHANGELOG line): say honestly what is counted, or keep ids out of reach as text until after the counting.
- For contrast, M4 (the F3 shape over `other.id == line.id`) is caught in 16 ms: `the read of the epochs file took more than 6019 steps`.

**F2. crates/inbox-cli/src/outbox.rs:303** (mutant, note, CONFIRMED, introduced)
- What it is: nothing checks that a comparison of two `LineId`s counts a step, and that step is what catches M4.
- What was measured: mutant M9, `fn eq(..) { self.0 == other.0 }` with `step();` removed. 29 cases green: `outbox::tests::*`, `adversary_w8_repost::*`, `epoch::*`.
- The `HashMap` calls `eq` only when hashes meet, so the step bounds hold either way.
- Fix: a unit case in `outbox.rs` that compares two `LineId`s and asserts `STEPS` rose by 1.

**F3. crates/inbox-cli/tests/adversary_w8_repost.rs:375** (mutant, note, CONFIRMED, introduced)
- What it is: with the 3 s bound replaced by `wait_until_finished` (10 s), the case can no longer tell a stand-in that keeps its 1 s wait from one that waits the 10 s default.
- What was measured: mutant M8, `support::accept_within(&listener, wait.max(support::STAND_IN_WAIT))` in `stand_in_within`. `a_stand_in_whose_client_never_connects_ends_its_thread_and_fails_the_case`: PASS in 10.022 s, a race with the 10 s bound.
- `adversary_w8_repost2.rs:341` and `adversary_control_sequences.rs:355` have the same shape; I did not run mutants on them.
- Fix: assert that the thread's message names its 1 s wait (as the raw stand-in case already does), and leave the time unbounded.

Mutants and plants

| Change | Result |
|---|---|
| M4: quadratic read over `LineId ==` | caught by `an_epochs_file_of_20000_lines_is_read_in_steps_linear_in_its_lines` (`took more than 6019 steps`, outbox.rs:273) |
| M4c: quadratic read over `text()` once per line | survived (F1) |
| M5: counter counts nothing (`steps.set(taken - 1)`) | caught by the same case: `0 steps for 2001 lines: fewer than one per line` (outbox.rs:684) |
| M7: `support::blocked_in_write` returns true at once | caught by both `adversary::sigterm_while_the_first_line_is_written_stops_serve_with_status_0` (adversary.rs:741) and `adversary2::sigterm_ends_serve_when_its_standard_error_is_a_full_pipe` (adversary2.rs:359): `blocked_in_write answers true before any thread writes to the pipe` |
| M9: `LineId::eq` not counting | survived (F2) |
| M10: `is_file` refusal removed | caught by `clock_file::tests::each_bad_form_of_the_file_is_named_and_its_text_is_not` (`left: Err("it is empty") right: Err("it is no regular file")`) |
| M8: stand-in ignores its 1 s wait | survived (F3) |

- My pass-1 FIFO cases stay green under M10, because a FIFO opened with `O_NONBLOCK` and no writer reads as empty and still gets exit 4.
- M5, M7, M10 and M8 shared one build; each touches only the cases listed for it. M4, M4c and M9 each had a build of their own.

3 s plants in the test's own thread, all green:
- `due_mark` before the post: PASS 4.22 s.
- `adversary.rs` full-pipe case after SIGTERM: PASS 3.40 s.
- `adversary2.rs` full-pipe case after SIGTERM: PASS 3.13 s.
- the `adversary_w8_repost` stand-in thread delayed 3 s (all 8 cases of that file): PASS, the never-connects case in 4.03 s.
- `checks.rs` before the post: PASS 4.23 s.

5. Attacked and could not break
- `outbox.rs` as shipped code answers like `main` for every form you listed: duplicate ids, two epochs for one id, empty lines, no tab, CRLF, no final newline, case or whitespace variants, 0 lines, non-UTF-8. I read `git diff main -- outbox.rs` instead of building a `main` binary:
  - `LineId`'s `Hash` and `Eq` are those of `&str`;
  - the split is `text.lines()` and `split_once('\t')`, as on main;
  - the empty-id test moved after the count filter, which gives the same result;
  - `read_to_string(...).unwrap_or_default()` is unchanged.
  No difference, so no case.
- The release build carries no counter. `STEPS`, `MOST` and the assert are `cfg(test)`, `step()` is empty in any other build, and `LineId` stays as a plain wrapper. `cargo check --release --locked -p inbox-cli`: Finished, no warning in `inbox-cli`.
- `libc`:
  - It is used unconditionally (the `clock_file` module is compiled in release too, though unreachable there).
  - `libc::O_NONBLOCK` exists on every unix target, and the crate was unix-only before (`std::os::unix` in `outbox.rs`).
  - `libc` was already in the lock file through `signal-hook`; Cargo.lock gains one dependency line.
- Not verified: whether `O_NONBLOCK` changes a read of a regular file on a network or FUSE filesystem. Local filesystems on Linux ignore it.
- `wait_until_finished` turns no failure into a pass:
  - In the two `should_panic` cases, a timeout panics with a message that does not match the expected one, so the case still fails.
  - The raw stand-in case still asserts its message.
  - The escape case still asserts that no character is raw and that the body is shown whole.
- Every changed case holds no time a 3 s delay breaks: the `expires` margins of 30 s span one process start, and the clock file stays frozen until the case moves it.
- Docs:
  - The service.md table rows match the code and my probes.
  - AGENTS.md matches the code.
  - The CHANGELOG list of what is left in `crates/inbox-cli/tests/` is right in substance. It is loose in wording: `requests()` 2 s, a 1 s `requests_within` in a `should_panic` case and the 200 ms after SIGTERM are short waits whose running out cannot fail the case, which is neither "a lower bound" nor "10 s or more".
- Not checked: an arm runner, a cross build, and the 5-of-5 acceptance runs under load (no load test).

6. Paths written outside the worktree
- <scratch>/d30/clock/adversary2/ (the directory)
- …/adversary2/src.patch
- …/adversary2/tests.patch
- …/adversary2/docs.patch
- …/adversary2/probe-adversary_clock_file_2.rs
- …/adversary2/run1-probes.log
- …/adversary2/run2-group1.log
- …/adversary2/run3-m4-eq.log
- …/adversary2/run4-m4-text.log
- …/adversary2/run5-m9-eq.log
- …/adversary2/run6-plants.log
- …/adversary2/run7-acceptance-plant.log
- …/adversary2/run8-suite.log

The pass-1 script `…/adversary/acceptance-check.sh` was reused and not changed.

7. findings
```findings
[
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 289, "category": "mutant", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A pairwise read that takes each id's text once through text() and compares the strings passes the step case in 14.0 s, so the comment's 'counted at the comparison, however it is written' and the changelog's 'at each comparison of two entries' are untrue."},
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 303, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Nothing checks that LineId::eq counts a step: with the step removed, all 29 outbox, epoch and w8_repost cases stay green."},
  {"file": "crates/inbox-cli/tests/adversary_w8_repost.rs", "line": 375, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "With the 3 s bound replaced by the 10 s hang bound, a stand-in that ignores its 1 s wait passes the never-connects case in 10.02 s, a race with the bound."}
]
```
