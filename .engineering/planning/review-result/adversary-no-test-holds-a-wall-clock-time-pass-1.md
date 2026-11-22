---
format: aep.planning-md/3
id: review-result:adversary-no-test-holds-a-wall-clock-time-pass-1
kind: review-result
status: active
title: Adversary pass 1 on story:no-test-holds-a-wall-clock-time
relations:
- reviews: story:no-test-holds-a-wall-clock-time
revision: 1
---
unit: story:no-test-holds-a-wall-clock-time, tree <tree> at 062e614 (`git diff main`) plus my added test file
verdict: NEEDS-CHANGE
cases: executed 151→154 (package inbox-cli), red 3
origin: introduced 7 / pre-existing 3 / undecided 0
wrote-outside-worktree: 11 paths (listed in part 6)
needs-coordinator: none

**Summary.** The unit does not meet its own acceptance statement or its changelog line. Two of its guards do not catch the defects they are named for.
- `due_mark.rs` is in the story's scope. It holds a 2 s life on the system clock, and a 3 s delay turns it red.
- The case that replaced the timed epochs bound passes with the original quadratic read put back.
- Three of my cases stay red. All 7 mutants and plants I put in implementation or test files are restored. `git status --short` shows only `crates/inbox-cli/tests/adversary_clock_file.rs` (new) and one `mod` line in `crates/inbox-cli/tests/main.rs`.
- Mutants and plants went into the tree and were restored, as your brief allows. The adversary procedure says to mutate a copy; I followed the brief.

1. git --no-pager diff --stat
```
 crates/inbox-cli/tests/main.rs | 1 +
 1 file changed, 1 insertion(+)
```
Plus the untracked `crates/inbox-cli/tests/adversary_clock_file.rs`. Both are test files; no non-test path is touched.

2. Cases added, in `crates/inbox-cli/tests/adversary_clock_file.rs`. All 3 are red now. Run alone first with `cargo nextest run -p inbox-cli --locked -E 'test(/^adversary_clock_file::/)'`: 3 run, 3 failed, 151 skipped, exit 100.

- `a_clock_file_that_is_a_fifo_with_no_writer_stops_serve_before_it_binds_with_exit_4`. Asserts exit 4 within 10 s for a FIFO clock file, as service.md promises for a file that "cannot be read". Red:
  `assertion left == right failed: inbox serve with a FIFO as its clock file (None: still running 10 s later, killed)  left: None  right: Some(4)`
- `a_clock_file_replaced_by_a_fifo_while_serving_ends_serve_with_exit_4`. Asserts exit 4 within 10 s after the file is renamed over by a FIFO. Red:
  `left: None  right: Some(4)`
  The service hangs, because the reading blocks in `open(2)` while holding the clock's lock.
- `a_clock_file_over_64_bytes_is_refused_with_the_cause_service_md_names`. Asserts the cause the service.md table gives. Red:
  `stderr: "inbox: --clock-file clock: it is longer than 64 bytes, so no instant\n"`

3. Suite run after the cases existed: `ESS_TOOLCHAIN=0.97.3 cargo nextest run -p inbox-cli --locked --no-fail-fast`
```
Summary [  16.981s] 154 tests run: 151 passed, 3 failed, 0 skipped
FAIL adversary_clock_file::a_clock_file_over_64_bytes_is_refused_with_the_cause_service_md_names
FAIL adversary_clock_file::a_clock_file_replaced_by_a_fifo_while_serving_ends_serve_with_exit_4
FAIL adversary_clock_file::a_clock_file_that_is_a_fifo_with_no_writer_stops_serve_before_it_binds_with_exit_4
exit=100
```
Slowest: 4.26 s (`an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires`). My two FIFO cases take 10.2 s each, which is their hang bound.

4. Findings. Each one says what was measured, what reaches it, and where the fix lands.

F1. due_mark.rs:44 (acceptance, blocker, NEEDS-CHANGE, pre-existing)
- What it is: `due_mark::a_due_message_stays_listed_and_only_its_own_read_is_refused` was added on main. It gives a post `expires` = system clock + 2 s and needs the post to arrive before that. The class is 2, it is inside the story's scope `crates/inbox-cli/tests/`, and the inventory does not list it.
- Measured: a planted 3 s sleep after `expires` is computed, standing in for a slow client start. Red:
  `panicked at crates/inbox-cli/tests/support/mod.rs:84:9 ... status: 1, stderr: "inbox: refused: inbox.messaging.AlreadyExpired (422)\n"`
- What reaches it: every `task test` run on this host.
- Why pre-existing: `git diff main` is empty for this file and for the server, core and verify crates. Not run on the base commit itself.
- Fix: the case starts `inbox serve --clock-file` and moves the file past `expires`. Its `wait --seconds 1` bound is monotonic and can stay.

F2. CHANGELOG.md:19 (contract-drift, warning, NEEDS-CHANGE, introduced)
- What it is: "No test holds a wall-clock time where a slow host fails it" is false of the tree. The story's acceptance statement covers all of `cargo test --workspace`.
- Measured: a 3 s plant in the test's own thread turned each of these red:
  - F1.
  - F8 and F9.
  - `crates/inbox-server/tests/read_rule.rs:206`, red with `left: [] right: ["01JD0000000000000000000001"]`. This one confirms the inventory's own class 2; it is not a new finding.
- The inventory itself leaves about 12 more cases of class 1 to 3.
- Fix: narrow the changelog line to the two cases the unit fixed, or fix the rest.

F3. outbox.rs:605 (mutant, blocker, NEEDS-CHANGE, introduced)
- What it is: `an_epochs_file_of_20000_lines_is_read_in_steps_linear_in_its_lines` counts only where `step()` was placed, not the comparisons.
- Measured: I put back wave 8's original quadratic `Epochs::parse` (from a9bd783^), with steps per line as the current code places them. Neither case caught it:
  - the unit case: PASS in 22.166 s;
  - `adversary_w8_repost::an_epochs_file_of_20000_lines_is_read_and_its_entry_is_found`: PASS in 54.521 s.

  The mutant, in 3 lines:
  ```
  - let mut named: HashMap<&str,(usize,&str)> = ...; for line in text.lines() { step(); ... }
  + let named: Vec<(&str,&str)> = text.lines().inspect(|_| step()).map(..).collect();
  + .filter(|(id, _)| named.iter().filter(|(other, _)| other == id).count() == 1)
  ```
- The case's comment "where a read in pairs takes 100 times" is untrue. The unit removed the 2 s bound, the only check that caught F3, which took 7.67 s.
- Fix: count at the comparison (a counting key type or comparator used by the map), or assert the structure. Either lands in `outbox.rs`.

F4. outbox.rs:615 (mutant, warning, NEEDS-CHANGE, introduced)
- What it is: the step case has upper bounds only.
- Measured: the mutant `fn step() { STEPS.with(|s| s.set(s.get())) }` passes (0.362 s).
- Fix: add a lower bound, for example `long >= 20_001`.

F5. adversary.rs:729 (mutant, note, CONFIRMED, introduced)
- What it is: the guard `blocked_in_write` is not tested by anything.
- Measured: the mutant `return true;` as its first statement leaves `sigterm_while_the_first_line_is_written_stops_serve_with_status_0` green (0.513 s). The filler fills 64 KiB long before the child execs.
- Fix: in the case, assert `!blocked_in_write(filled)` before the filler starts.

F6. clock_file.rs:100 (boundary, note, INFEASIBLE, introduced)
- What it is: `File::open` blocks on a FIFO. Start and serving both hang instead of exit 4 (the two red cases in part 2).
- What reaches it: nothing found. No caller passes a FIFO, and the debug-only flag is used with regular files.
- Fix: check `metadata().is_file()` before the open, or open non-blocking.

F7. docs/design/service.md:288 (contract-drift, note, CONFIRMED, introduced)
- What it is: the table gives the cause `it does not hold one RFC 3339 instant` for a file over 64 bytes. The code says `it is longer than 64 bytes, so no instant`.
- Measured: my third red case in part 2.
- Fix: one doc line, or the code's text.

F8. crates/inbox-server/tests/cases.rs:521 (acceptance, warning, CONFIRMED, pre-existing)
- What it is: `a_request_not_arrived_within_the_request_timeout_is_408` holds a 300 ms timeout to `< 900 ms`. Line 532 holds a stop to `< 2 s`. Both are class 1 and both are missing from the inventory.
- Measured: a 3 s plant gives `assertion failed: Monotonic::now() - started < Duration::from_millis(900)`.
- Origin: the server crate is unchanged by the unit.

F9. crates/inbox-server/tests/adversary_deadline_wait.rs:256 (acceptance, warning, CONFIRMED, pre-existing)
- What it is: `one_wait_of_a_second_on_a_note_due_at_the_reading_reads_its_view_once` asserts `held < 3 s`. It is class 1, came from main, and is not classed in the inventory.
- Measured: a 3 s plant gives `held 4.018921084s` at line 257 with the plant in (256 without it).

F10. tests/acceptance/tests/checks.rs:132 (judgement, note, CONFIRMED, introduced)
- What it is: the life of 30 s is fixed from `start`. The post's `created` comes from the system clock, and the service refuses a post where `expires <= created`. So the post must be made within 30 s of real time after `start`. `crates/inbox-cli/tests/clock_file.rs:116` has the same shape.
- Measured: a 31 s plant before the post gives
  `panicked at tests/acceptance/src/lib.rs:69:9 ... status: 1, stderr: "inbox: refused: inbox.messaging.AlreadyExpired (422)\n"`
- The 60 s runner bound binds first, so this is small.
- Fix: compute `expires` from the system clock just before the post. That leaves one process start inside the margin instead of about 8.

Mutants and plants:

| Mutant | Result |
|---|---|
| M1: `FileClock::now` falls back to the system clock on a bad reading | caught by `clock_file::a_clock_file_that_goes_bad_while_serving_ends_serve_with_exit_4` (open exit 0, expected 3). The unit cases miss it. |
| M2: step back accepted (`if false && at < *last`) | caught by `clock_file::tests::a_reading_follows_the_file_forward_and_refuses_a_step_back` and `clock_file::a_clock_file_that_goes_bad_while_serving_ends_serve_with_exit_4` |
| M3: `conflicts_with = "clock_start"` removed | caught by `clock_file::a_clock_file_that_holds_no_instant_stops_serve_before_it_binds` (serve started and printed 23 bytes) |
| M4: the epochs read made quadratic (F3 put back) | survived (F3) |
| M5: step counter not counting | survived (F4) |
| M6: the sweep disabled (`listener.rs:138`, `sweep_now` dropped) | caught by `checks::a_read_note_and_an_expired_unanswered_request_are_in_no_view`: "the request that expired at ... is in ["Outbox", "Thread"]" (checks.rs:195) |
| M7: `blocked_in_write` returns true at once | survived (F5) |

M1, M3, M5 and M7 ran in one build together with the `due_mark` plant; M2 and M4 ran in a second build. Each mutant is attributed to the cases it alone touches.

5. Attacked and not broken:
- The release build has no flag. Every reaching line is `cfg(debug_assertions)`, release passes `clock_file: None`, and `cargo check --release --locked -p inbox-cli` exits 0 with no warning. `FileClock` is still compiled into release, but nothing can reach it.
- A clap unknown argument exits 2, as service.md says.
- Neither the debug `inbox serve --help` nor `generated/cli/inbox.json` names the clock flags.
- No bad reading answers at a fallback instant or counts as "not due". `now()` exits while holding the clock lock, so only one thread exits.
- A directory as the clock file gives exit 4 (EISDIR). The same instant twice is accepted.
- The 60 s request window is symmetric (`ring.rs`), so moving the clock 31 s keeps every later request inside it.
- Exit 4 while serving loses nothing the documented stop keeps: the memory backend keeps nothing across either.
- `/proc/self/task/*/syscall` on a GitHub runner, not verified by a run. From the kernel code, `ptrace_may_access` allows the caller's own thread group before Yama is consulted, so ubuntu-latest x86-64 should read it. Syscall number 1 means write on x86-64 only (64 on aarch64), and the existing child check has the same dependency.
- The new main cases `deadline_mark.rs`, `adversary_deadline_mark.rs` and the `argument_not_utf_8` files only wait or hold no time. `adversary_deadline_wait.rs` lines 146, 222 and 299 only wait; line 256 does not (F9).

Not checked:
- The 5-of-5 acceptance run under load (no load test).
- Symlinks, a file replaced in place mid-read, and offsets or fractions end to end (covered by unit cases only).
- The W-class entries of core, backend-testkit, conformance and xtask, one by one.
- No worktree session lease was taken; the brief named none.

6. Paths written outside the worktree:
- <scratch>/d30/clock/adversary/ (the directory)
- …/adversary/diff-src.patch
- …/adversary/acceptance-check.sh
- …/adversary/run1-own-cases.log
- …/adversary/run2-suite.log
- …/adversary/run3-groupA.log
- …/adversary/run4-groupB.log
- …/adversary/run5-server-plants.log
- …/adversary/run6-acceptance-nosweep.log
- …/adversary/run7-acceptance-late-post.log
- …/adversary/run8-check-release.log

Inside the worktree, the builds wrote into its own `target/`.

7. findings
```findings
[
  {"file": "crates/inbox-cli/tests/due_mark.rs", "line": 44, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "pre-existing", "message": "A case in the story's scope gives a post a 2 s life on the system clock; a 3 s delay before the post makes it fail with AlreadyExpired, and the inventory does not list it."},
  {"file": "CHANGELOG.md", "line": 19, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The entry claims no test holds a wall-clock time a slow host breaks, while 3 s plants turn at least 4 workspace cases red and the inventory leaves about 12 more."},
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 605, "category": "mutant", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "With wave 8's quadratic Epochs::parse put back, the step-count case passes in 22 s and the binary case in 54.5 s, because steps are counted where step() is placed and not per comparison."},
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 615, "category": "mutant", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The step-count case has only upper bounds, so a step() that counts nothing passes."},
  {"file": "crates/inbox-cli/tests/adversary.rs", "line": 729, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "blocked_in_write returning true at once leaves the SIGTERM-while-writing case green, so its guard is not tested."},
  {"file": "crates/inbox-cli/src/clock_file.rs", "line": 100, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A FIFO as the clock file blocks open(2), so inbox serve hangs at start and while serving instead of exiting 4 as service.md says; no caller passes a FIFO."},
  {"file": "docs/design/service.md", "line": 288, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The table gives 'it does not hold one RFC 3339 instant' for a file over 64 bytes, but the code says 'it is longer than 64 bytes, so no instant'."},
  {"file": "crates/inbox-server/tests/cases.rs", "line": 521, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A 300 ms request timeout held to under 900 ms (and a stop held to under 2 s at line 532) is class 1, missing from the inventory, and fails with a 3 s delay in the test thread."},
  {"file": "crates/inbox-server/tests/adversary_deadline_wait.rs", "line": 256, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A case new on main asserts a 1 s wait was held under 3 s; with a 3 s delay it held 4.02 s and failed, and the inventory does not class it."},
  {"file": "tests/acceptance/tests/checks.rs", "line": 132, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The 30 s life is fixed from the start while created comes from the system clock, so a post more than 30 s after the start is refused AlreadyExpired (shown with a 31 s plant); clock_file.rs:116 has the same shape."}
]
```
