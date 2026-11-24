---
format: aep.planning-md/3
id: review-result:adversary-early-bridge-case-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the case of the early bridge (round 1)
relations:
- reviews: story:the-case-of-the-early-bridge-counts-its-lines-after-a-second-try
revision: 1
---
ADVERSARY PASS 1

```
unit: story:the-case-of-the-early-bridge-counts-its-lines-after-a-second-try, round 1 at 62b8c97c (base a1881d9), in the adversary tree
verdict: CONFIRMED (2 warnings, both pre-existing; nothing introduced by the story)
cases: executed 458→460, red 0
origin: introduced 0 / pre-existing 2 / undecided 0
wrote-outside-worktree: 34 files in the scratch directory (32 logs, numbered 01 to 26, and 2 read-only scripts, times.sh and fails.sh); no build directory
needs-coordinator: should finding 1 go into this story (its fix is a few lines in this story's listener) or into a backlog draft with finding 2
```

0 pre-existing findings are in the private file. Neither finding is a security flaw, so the private file was not written.

## 1. Diff stat of the adversary tree (`git --no-pager diff --stat`)

```
 crates/inbox-cli/tests/main.rs | 1 +
 1 file changed, 1 insertion(+)
```

Untracked: `crates/inbox-cli/tests/adversary_early_bridge.rs` (162 lines). Every path is a test file. `src/wake.rs`, `src/bridge.rs` and `tests/bridge.rs` match 62b8c97c again; each mutant was edited back by hand.

## 2. Cases added (`crates/inbox-cli/tests/adversary_early_bridge.rs`, one `mod` line in `tests/main.rs`, sorted)

Both cases are green on the tree now. Each one kills a mutant that the whole crate lets live. They use a listener of their own (`Tries`). It keeps the request line of each connection, reads what arrives and closes without an answer. Its thread ends when it is dropped or after `HANG`.

- `a_bridge_that_waits_for_the_service_pauses_between_its_tries`:
  - Waits for 4 tries and holds only a lower bound: the 4th try comes at least 700 ms (100 + 200 + 400) after the bridge starts.
  - Red under mutant M5 (the pause removed in `say_once`, `wake.rs:679`). Run of that case file alone, captured after the file was written:
    ```
    FAIL [   0.291s] (2/2) inbox-cli::main adversary_early_bridge::a_bridge_that_waits_for_the_service_pauses_between_its_tries
    panicked at crates/inbox-cli/tests/adversary_early_bridge.rs:134:5:
    4 tries within 285.940506ms of the bridge's start: no pause of 100, 200 and 400 ms between them
    Summary [   0.293s] 2 tests run: 1 passed, 1 failed, 458 skipped
    ```
    The first version of the file gave the same result: `4 tries of the start within 232.729762ms`.
  - Green 20 of 20 without mutants. Fastest 0.911 s, slowest 1.137 s.
- `each_try_of_a_bridge_that_waits_for_the_service_is_its_open`:
  - Asserts that each of the first 3 tries is `POST …/open-inbox`.
  - Red under mutant M8a (`Err(Failure::Transport(_)) => break,` added after `wake.rs:436`):
    ```
    FAIL [   0.301s] (1/2) inbox-cli::main adversary_early_bridge::each_try_of_a_bridge_that_waits_for_the_service_is_its_open
    panicked at crates/inbox-cli/tests/adversary_early_bridge.rs:157:9:
    a try that is not the open of the start: ["POST /messaging/commands/open-inbox HTTP/1.1", "GET /messaging/views/pending?inbox=human%3Aperson&wait=0&undelivered=1 HTTP/1.1", "GET /messaging/views/pending?inbox=human%3Aperson&wait=0&undelivered=1 HTTP/1.1"]
    ```
  - Green: 0.514 s in the suite run.

## 3. Suite run, after the cases existed

`cargo nextest run -p inbox-cli --locked` (on build-1d):
```
Summary [  12.094s] 460 tests run: 460 passed, 0 skipped
exit 0
```

The "before" count of 458 comes from two sources: the implementor's `cases:` line, and my runs 19-1 and 19-2 with `-E 'not test(/^adversary_early_bridge::/)'`, which show `458 tests run: 458 passed`.

`cargo clippy -p inbox-cli --all-targets --locked -- -D warnings`: exit 0. The new file was formatted with `rustfmt --edition 2024`.

## 4. Findings

**F1, `crates/inbox-cli/src/wake.rs:436`** (CONFIRMED, pre-existing, warning)
- **What was measured:** with M8a, the bridge takes a failed open as "started" and goes on to read `Pending` of an inbox it never opened. The whole crate stays green: run 13 gave `459 tests run: 458 passed, 1 failed`, and the 1 failure was my pause case, for a timing reason. The story's case passed in 0.199 s, and both closed-port cases in `adversary_bridge.rs` passed.
- **Origin:** the a1881d9 case under M8a is green, as are both closed-port cases. Run 17 gave `3/3 stress run iterations: 3 passed`.
- **Why it matters:** the doc comment says the bridge "keeps trying instead of ending at the open of its start". The case counts connections and never looks at what they ask, although `read_request` already reads every request.
- **What reaches it:** the scenario the case names, a bridge started before the service, for example by a service manager. With this regression, a service that came up later would answer `409 not open`. The reconnect would find no record of an open and end the bridge. That last step was traced in the code, not run.
- **Fix:** keep the request line in `Unanswered` and assert that the first 3 tries are the open, or take my case.

**F2, `crates/inbox-cli/src/wake.rs:679`** (CONFIRMED, pre-existing, warning)
- **What was measured:** with M5 (`std::thread::sleep(pause(*failures))` replaced by `let _ = pause(*failures);`), a bridge or wake that waits for a service that does not answer connects again at once, in a loop. Run 10b: `458 tests run: 458 passed`.
- **Origin:** the a1881d9 case is green under M5 (run 16, 5 of 5), and the other 457 cases are the same files.
- **What reaches it:** every bridge or wake while the service is down. The existing pause cases (`adversary_wake::an_empty_answer_that_comes_at_once_is_not_asked_again_without_a_pause`, `wake::an_answer_at_once_with_only_a_delivered_row_is_followed_by_a_growing_pause`) hold the `quick` pause of `held`, not this sleep.
- **Fix:** my pause case.

## 5. What I attacked and could not break (the brief's items, in order)

1. **Mutant M1 (`if true` in `say_once`):**
   - Red 20 of 20 alone: 0.339 to 0.437 s.
   - Red 20 of 20 again with my second job (the bridge lane, 3 iterations) running at the same time; the sync line confirms that job was running in the tree. Times 0.331 to 0.469 s.
   - Every red run had `left: 3`: the lines of tries 1 to 3, so the line of try 3 is already there when the count is read.
   - Order in `wake.rs`: line 438 `say_once` → 674 `warn` (an unbuffered write to the stderr file) → 679 pause → 435 `started` → `client.rs:173` connect. Nothing connects before the line is written, and the line of try 2 is written 200 ms before connection 3. No gap.
2. **Other product mutants:**
   - `if false`: red, `left: 0`, at `bridge.rs:1094`.
   - `== 1` (line on the second failure only): the story's case is green. It is killed by `wake::a_502_of_a_proxy_is_tried_again_…` and `adversary_wake::each_run_of_failures_after_an_answer_is_said_again`, which share `say_once`.
   - `% 2 == 0` (line on every second failure): the case is red in 19 of 20 alone, but was green in the crate run. That catch is a race with the line of try 3; waiting on the 3rd try is a decided item. The mutant is killed for sure by `wake::a_service_that_answers_503_408_or_drops_the_connection_is_asked_again` and the 1600 ms case.
   - Pause removed: the story's case is green, which is right for it (one line, still trying; 0.046 to 0.059 s). See F2.
   - `FIRST_PAUSE` 10 s: red at 10.022 s with `the 3rd try of the bridge, or its end: not within 10 s` (`bridge.rs:40`). Under 60 s.
   - Giving up after 2 tries: red at 0.140 s on `assert!(bridge.running())` (`bridge.rs:1091`), with stderr in the message. It does not wait for `HANG`.
   - A closed connection read as an answer: covered by the implementor's 200 `{}` mutant (red). The variant at the start loop is F1.
3. **The 2 failure paths:**
   - Refused connect: `client.rs:133`, `Failure::Transport("…: Connection refused")`.
   - Accepted, then closed: `client.rs:206-211`, `Reply::read` "the answer has no end of its head", `Failure::Transport`.
   - Both reach `tried_again` (`wake.rs:105`) and the same `say_once` (`wake.rs:438`).
   - The refused path is still covered by `adversary_bridge::a_bridge_started_before_the_service_answers_waits_for_it` (still running after 1 s) and `adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal` (running at 1.6 s, exactly 1 line, ends on SIGTERM). Neither asserts the text of the line any more; the format is the same `say_once` line, so I filed no gap.
4. **The listener:**
   - No busy wait: `accept_within` polls every 10 ms, bounded at 20 ms per call.
   - The `Drop` of `Unanswered` also runs while a failing assertion unwinds. The thread ends within 20 ms, or after a read in progress (at most `HANG`). It is detached and does not hold the end of the process.
   - The count is read with `SeqCst` on both sides. A second client would be counted, which is fine.
   - `Bridge::drop` kills the bridge and waits for it. Before its open the bridge has no child.
   - I could not look for the process by PID on build-1d: the case prints no PID, and the tests run on the node.
5. **Times:**

   | Measured | Result |
   |---|---|
   | The case 50 times, no mutants | 50 of 50 green, 0.331 to 0.355 s |
   | The case at a1881d9 | 0.027 s in both crate runs |
   | The case now | 0.333 s and 0.342 s in the crate runs |
   | Crate at a1881d9 (458 cases) | 12.767 s and 12.073 s (wall 15.5 s and 13.9 s) |
   | Crate now (458 cases) | 12.193 s and 12.582 s (wall 16.1 s and 15.4 s) |
   | Crate with my 2 cases (460) | 12.094 s |
   | Bridge lane | 103 cases, about 11 s per iteration |

   No case over 5 s.
6. **Mutants of the case**, with M1 in:
   - Wait on `>= 2`: green in 3 of 20 (`left: 2` in the 17 red runs).
   - Wait on `>= 1`: green in 19 of 20 (`left: 0` in the red one).
   - So the wait on 3 is the case's margin.
   - The count assertion made `>= 1`: green 5 of 5.
   - The `connectors` assertion: first I started the poll before the open. The case stayed green, because the poll calls nothing while the record is empty. A one-line mutant instead runs `connectors` before each open: red at `bridge.rs:1105` (`left: 3`). With the assertion removed it is green, so that assertion is the only guard against that mutant.
7. **The doc comments, sentence by sentence:**
   - They match what the case does.
   - One imprecision in `Unanswered`: "stops when it is dropped, or after three times HANG at most". The thread actually stops within 20 ms of the drop, or after the read in progress (up to `HANG`), so the true bound is 4 times `HANG`. A note only, not a finding.

**Nextest exit status:** I saw the same as the implementor, and found the cause. With `--no-fail-fast`, the exit status of a stress run follows its last iteration. Run 05: `20/20 stress run iterations: 19 passed, 1 failed`, iteration 19 failed and 20 passed, `exit 0`. Without `--no-fail-fast` the run stops at the first failed iteration and exits 100 (run 05b: `4/20 … 3 passed, 1 failed`, exit 100).

**Stale builds:** I edited only after each job had ended, and each sync line named the expected number of changed files. I saw no stale binary.

**Not reached:**
- The PID check of a bridge left after a red run (the node is remote).
- A mutant run of the whole crate for M1, `FIRST_PAUSE` 10 s and the give-up mutant; for these only the case alone ran.

**Builds without `slot`:** none. Every cargo call went through the shim to build-1d, and each log starts with `slot: synced to build-1d`. One `cargo nextest --version` call (it compiles nothing) went through the shim, gave no output within 120 s and was left in the background.

## 6. Paths written outside the adversary tree

All in the scratch directory, 34 files:
- Logs `01-base-stress50.log` to `26-m9b-calls-assert-removed.log`, 32 files: 01 to 26 are each one file, except 03 (`03a`, `03b`), 05 (`05`, `05b`), 09 (`09`, `09b`), 10 (`10`, `10b`), 18 (`18-base-crate-1`, `18-base-crate-2`) and 19 (`19-now-crate-1`, `19-now-crate-2`).
- `times.sh` and `fails.sh`: read-only log summaries, never run under `slot`.

No `pre-existing.md`.

## 7. Findings

```findings
[
  {"file": "crates/inbox-cli/src/wake.rs", "line": 436, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A bridge that takes a transport failure of its open as the start done passes all 458 inbox-cli cases, the story's case included, because the case counts connections and never checks that each try is the open."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 679, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "With the pause of say_once removed, a bridge or wake reconnects in a tight loop to a service that does not answer, and all 458 inbox-cli cases stay green."}
]
```
