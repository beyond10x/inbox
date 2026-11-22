---
format: aep.planning-md/3
id: review-result:adversary-pass-1-on-the-test-step-budget
kind: review-result
status: active
title: adversary, pass 1, the test step's budget
relations:
- reviews: story:test-step-enforces-its-budget
revision: 1
---
unit: story:test-step-enforces-its-budget, the uncommitted working tree of `chore/test-budget` (base e649555)
verdict: NEEDS-CHANGE
cases: executed 523→524, red 1
origin: introduced 3 / pre-existing 0 / undecided 0

The report of the adversary (one agent, 2026-11-22), as handed back, with the path of the unit's
tree left out. What the coordinator did with each finding is at the end.

**1. Diff.** The adversary's only edit is in a test file, `crates/inbox-xtask/tests/tools/test_step.rs`:
lines 147–285, the constant `STUBBORN` and one case. No implementation file, no config, no
Taskfile, no manifest and no support file was touched.

**2. Case added.** `crates/inbox-xtask/tests/tools/test_step.rs::a_run_over_the_bound_leaves_no_test_process_running`,
red when handed back, 13.7 s.

- Setup: it writes a fixture package into `CARGO_TARGET_TMPDIR` and copies in the repository's
  `.config/nextest.toml`. The package has 2 tests: one ignores SIGTERM, the other waits on a
  child that ignores SIGTERM.
- Run: the real `tools/test-step.sh 2 <report> cargo nextest run --offline`.
- Asserts, in order: exit 1; both tests ran; no process of the run is still alive 1.5 s after the
  step ended; the 10 slowest tests are printed.
- Needs `cargo-nextest` on PATH. It kills any survivors by PID when it ends.

Red output of the case run alone:

```
FAIL [  13.711s] (1/1) inbox-xtask::main tools::test_step::a_run_over_the_bound_leaves_no_test_process_running
still running after the step ended: ["test-alive", "child-alive"]
test-step: the run took 12 s of its bound of 2 s, exit 137
  Cancelling due to signal: 2 tests still running
test-step: the run wrote no JUnit report
test-step: the run took longer than its bound of 2 s and was ended
```

Checked against a copy of the fixture: `grace-period = "0s"` in `slow-timeout` of
`.config/nextest.toml` gives `SIGKILL [1.909s]` for both tests and no survivors; the report is
written, the 10 slowest are printed, and the step exits 1 after 2 s.

**3. Suite run with the case.** `task test`: `524 tests run: 523 passed, 1 failed, 5 skipped`,
30 s of the bound of 120 s, exit 100. The only failure is the new case.

**4. Findings.**

1. The step's kill delay equals nextest's grace (`tools/test-step.sh:59`, `timeout --kill-after=10`).
   At the bound `timeout` sends SIGTERM to cargo-nextest, which waits its default 10 s grace
   before it sends SIGKILL to a test; `timeout`'s own SIGKILL, also at 10 s, ends nextest first.
   A test that ignores SIGTERM keeps running as an orphan, no JUnit report is written and the 10
   slowest are not printed. The step itself still fails at the bound plus 10 s. No test of the
   suite ignores SIGTERM today.
2. A test's child that ignores SIGTERM outlives its test (`.config/nextest.toml:3`). After a
   `TIMEOUT`, once the test process has died of SIGTERM, nextest sends no SIGKILL to the group.
   Measured with a limit of 2 s; with `grace-period = "0s"` the child dies with its test.
3. The "60 s" of `AGENTS.md` is 70 s for a test that ignores SIGTERM (a limit of 2 s measured at
   `TIMEOUT [12.003s]`), and the step's bound is up to 130 s. With `grace-period = "0s"`:
   `TIMEOUT [2.003s]`.

**5. Attacked, not broken.**

- `tools/test-step.sh`: exit statuses are kept; a command that exits 124 by itself returns 124; a
  missing command returns 127; a stale report is removed; a report path with a space works; the
  usage errors return 2; `&amp;` and `&lt;` decode; reordered attributes parse; zero or fewer
  than 10 cases are fine. Two `<testcase` on one line and times like `1e-3` are mishandled, and
  nextest writes neither (one case per line, 3 decimals, in the report of 523 cases). dash is not
  installed on the host of the pass: the probes ran with `sh` (bash) and `bash --posix`.
- `task acceptance`: `set: [pipefail]` holds with task 3.54.0, so a failing 4th check fails the
  step; the count is limited to `-p inbox-acceptance` and no retries are configured.
- One binary per package: every case file is declared in its `main.rs`; no test sets environment
  variables or the working directory, none reads the name of its binary; 517 + 6 = 523.
- The CI step: the archive's SHA-256 matches, `cargo-nextest` is at the top level of the archive,
  no `CARGO_TERM_COLOR` is set.

```findings
- file: tools/test-step.sh
  line: 59
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: timeout's --kill-after=10 equals nextest's 10-s grace, so at the bound nextest is SIGKILLed before it kills a test that ignores SIGTERM; the test keeps running outside the pool and no JUnit report is written, so the 10 slowest are not printed.
- file: .config/nextest.toml
  line: 3
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: with the default 10-s grace, a test's child that ignores SIGTERM outlives its test after a TIMEOUT or a cancel; grace-period = "0s" kills the whole process group (measured).
- file: AGENTS.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the rows of .config/nextest.toml and task test say a test is killed at 60 s, but a test that ignores SIGTERM ends at 70 s (2-s limit measured at 12.003 s) and the step bound reaches 130 s.
```

## What the coordinator did

All 3 findings were introduced by the unit and are closed in the unit's own commit by one
change: `slow-timeout` of `.config/nextest.toml` has `grace-period = "0s"`, and the row of that
file in `AGENTS.md` says why. The adversary's case stays in
`crates/inbox-xtask/tests/tools/test_step.rs` and passes with the change.
