---
format: aep.planning-md/3
id: review-result:adversary-inbox-wake-returns-on-a-post-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on inbox wake (round 1): 13 findings, 6 red cases, the acceptance case red in 3 of 4 package runs'
relations:
- reviews: story:inbox-wake-returns-on-a-post
revision: 1
---
unit: story:inbox-wake-returns-on-a-post, round 1, commit 1a5f274 (attacked in a git-archive copy of that commit, not in the implementor's tree)
verdict: CONFIRMED (blocker: the unit's own acceptance case fails at random in package runs; 6 red cases of my own)
cases: executed 207→216, red 7
origin: introduced 13 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 scratch directory with 9 kinds of path (all listed in part 6); nothing written in the tree
needs-coordinator: whether round 2's Sent-only wait also leaves out rows that are due (finding 3); whether a non-inbox HTTP answer should be tried again (finding 2 is a mismatch between the document and the code, and either side can be fixed)

1. Diff stat of the copy (`<scratch>/s3/wake/adversary/copy`, against 1a5f274):
```
 crates/inbox-cli/tests/adversary_wake.rs | 610 +++++++++++++++++++++++++++++++
 crates/inbox-cli/tests/main.rs           |   1 +
```
Both are test paths, so no charter violation. I edited the mutants into the copy's `src/wake.rs` and `src/client.rs` only, put the original back after each mutant, and checked it was back (`git status` showed only the 2 test paths).

2. Cases in `crates/inbox-cli/tests/adversary_wake.rs` (plus `mod adversary_wake;` in `tests/main.rs`). Each red line below is from the first run of that case alone.
- `an_empty_answer_that_comes_at_once_is_not_asked_again_without_a_pause`: RED. "inbox wake read Pending 176 times in 2 s against a service whose wait ends at once" (the case allows at most 50).
- `an_answer_that_is_neither_a_success_nor_a_refusal_ends_the_wake_with_3`: RED. "inbox wake still runs after 3 reads answered 404 by a server that is not the service; left: None right: Some(3)".
- `a_dangling_link_in_place_of_the_lock_file_creates_nothing_through_it`: RED. "the open of the lock file created …/elsewhere/made-through-the-link through a dangling link (2 requests sent)".
- `a_due_note_alone_does_not_end_the_wake`: RED. "inbox wake ended on the due note 01M836AYE3S580YA9DVCPV381V alone (exit 1): … inbox wake: 01M836AYE3S580YA9DVCPV381V was printed and not marked delivered: refused: inbox.messaging.Expired (502); left: Some(1) right: None".
- `a_wake_started_with_standard_output_closed_marks_nothing`: RED. "inbox wake with no standard output sent 1 MarkDelivered (exit 0): left: 2 right: 1".
- `a_restart_between_the_two_reads_of_a_round_still_opens_the_inbox_again_first`: RED. "the wake held its read on the new start for 10.001168318s without opening the inbox there" (the record of the open still holds the first start's epoch). It takes 11 s when red.
- `each_run_of_failures_after_an_answer_is_said_again`: GREEN on the unit (0.33 to 0.54 s). It is red on mutant m9.
- `a_wake_started_with_sigterm_ignored_still_ends_on_sigterm`: GREEN on the unit (0.09 to 0.17 s). It is red on mutant m10.
- `rustfmt --edition 2024` was run on the file. `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0.

3. Suite run after the cases existed: `SLOT_TIMEOUT=20m cargo nextest run -p inbox-cli --locked --no-fail-fast`, exit 100.
```
Summary [  54.864s] 216 tests run: 209 passed, 7 failed, 0 skipped
FAIL [   0.195s] adversary_wake::a_wake_started_with_standard_output_closed_marks_nothing
FAIL [   0.332s] adversary_wake::a_dangling_link_in_place_of_the_lock_file_creates_nothing_through_it
FAIL [   0.674s] adversary_wake::an_answer_that_is_neither_a_success_nor_a_refusal_ends_the_wake_with_3
FAIL [  11.358s] adversary_wake::a_restart_between_the_two_reads_of_a_round_still_opens_the_inbox_again_first
FAIL [  11.893s] wake::a_wake_on_an_empty_inbox_returns_on_a_post_after_the_end_of_a_wait_and_a_restart
FAIL [   1.920s] adversary_wake::a_due_note_alone_does_not_end_the_wake
FAIL [   2.232s] adversary_wake::an_empty_answer_that_comes_at_once_is_not_asked_again_without_a_pause
```
The 207 is the count from the implementor's suite logs (`suite-1.log` to `suite-3.log`).

4. Findings (all cover commit 1a5f274)
F1 `crates/inbox-cli/src/wake.rs:141`, boundary, warning, CONFIRMED, introduced.
- Measured: a read that ends without a row starts the next round with no pause. When both reads of a round are answered at once, the wake spins: 176 reads in 2 s against a stand-in that answers one request at a time, and more against a real service.
- What reaches it: `inbox serve --max-wait 0`. I started a service with that flag: it accepts it and starts. Any 2xx answer without `rows` reaches it too.
- Fix: a pause when a held read comes back empty faster than it asked for, or the service refuses `--max-wait 0`.

F2 `crates/inbox-cli/src/wake.rs:58`, contract-drift, warning, NEEDS-CHANGE, introduced.
- Measured: `Failure::Transport(_) => true` also covers answers that are "neither a success nor a refusal" (a 3xx, a 404 without `refused`, a body that is not JSON). The wake tries these again forever and says so in one line. `docs/design/wake.md:85-89` names only "cannot be reached, or cuts the answer" as tried again and says every other failure ends the wake.
- What reaches it: an `INBOX_URL` that names another loopback HTTP server.
- Fix: split those answers out of `Transport` and end the wake with 3, or write in the document that they are tried again. Trying again may be wanted for a proxy on loopback that answers 502, so this needs a decision.

F3 `crates/inbox-cli/src/wake.rs:138`, contract-drift, warning, CONFIRMED, introduced.
- Measured: the `wait=0` read answers rows that are due, and the wake returns on a note that is due alone. Its `MarkDelivered` is refused `Expired` and it exits 1. Every wake started after it does the same until the sweep. This is exactly what `docs/design/service.md:237-238` says the held wait prevents.
- What reaches it: an unread note passes its `expires` while no wake runs. The sweep runs every 10 s by default.
- Relation to round 2: this is about due rows, not `Delivered` rows. A Sent-only wait parameter does not fix it unless it also leaves out due rows.
- Fix: the wake ignores due rows when it decides to return, or the new service parameter leaves them out.

F4 `crates/inbox-cli/src/wake.rs:101` (also `src/output.rs` `print_checked`), acceptance, warning, CONFIRMED, introduced.
- Measured: started with descriptor 1 closed (`inbox wake >&-`), the write "succeeds" because Rust's standard output treats `EBADF` as success. The wake then marks the row delivered although nothing printed it, and exits 0. The story's rule says a closed standard output ends the wake without a mark.
- What reaches it: any parent that starts the wake with fd 1 closed.
- Fix: before the first request, check that fd 1 is open (`fcntl(1, F_GETFD)`), and exit 4 if it is not.

F5 `crates/inbox-cli/src/wake.rs:212`, boundary, note, INFEASIBLE, introduced.
- Measured: the lock file is opened with `O_CREAT` and without `O_NOFOLLOW`. A dangling link in its place is followed: the file it names is created elsewhere, and the wake goes on and sends requests.
- What reaches it: only someone who can write to the outbox directory, which the owner normally holds (made 0700). So I built this state; I did not find a real path to it.
- Fix: open the lock file with `O_NOFOLLOW`.

F6 `crates/inbox-cli/src/wake.rs:130`, concurrency, warning, CONFIRMED, introduced.
- Measured: `docs/design/wake.md:17-24` says the wake "opens the inbox again before it waits on the new start". That is false when the start changes between the `wait=0` read and the held read: the wake holds its read on the new start, where the inbox is not open, for up to 60 s, and posts meanwhile are refused `UnknownInbox`.
- What reaches it: the new start must answer before the old one is gone. That happens behind a forwarder or proxy, which is what the acceptance case builds. A plain restart on one port has downtime, which turns the held read into a transport failure.
- Fix: the held read carries the epoch the wake expects and the service answers at once when it differs, or the service answers a held wait on an inbox it does not know at once.

F7 `crates/inbox-cli/tests/wake.rs:679`, concurrency, blocker, CONFIRMED, introduced.
- Measured: the acceptance case `a_wake_on_an_empty_inbox_returns_on_a_post_after_the_end_of_a_wait_and_a_restart` failed with "the wake opened the inbox again at the new start: not within 10 s" in 3 of my 4 package runs:
  - 1 of 2 runs on the unmodified unit.
  - 2 runs with mutants m9 and m10, which are not the cause: under each mutant the case alone passed 3 of 3, and on the unit alone it passed 4 of 4.
- The window of F6 is the likely cause, but I have not proven it.
- Fix: the fix of F6. Without it, the case needs a deterministic order of restart and reads.

F8 `crates/inbox-cli/src/wake.rs:132`, mutant, warning, CONFIRMED, introduced.
- Measured: mutant m9 survives every case of `tests/wake.rs`. My case `each_run_of_failures_after_an_answer_is_said_again` catches it.
```
@@ fn held
-                failures = 0;
+
```

F9 `crates/inbox-cli/src/wake.rs:86`, mutant, warning, CONFIRMED, introduced.
- Measured: mutant m10 survives every case of `tests/wake.rs`. Nothing holds the documented "also when it was started with them ignored". My case `a_wake_started_with_sigterm_ignored_still_ends_on_sigterm` catches it.
```
@@ fn wake
-    end_on_signal()?;
+
```

F10 `website/docs/first-use.md:175`, judgement, note, CONFIRMED, introduced.
- Measured, with a probe of the binary: a wake of an inbox that was never opened (no `.opened` record) waits silently forever. Meanwhile `inbox post` to it is refused `UnknownInbox (409)`, and nothing appears on standard error.
- "it opens your inbox again first, as every command does" holds only when a record exists.
- Fix: one line on standard error when there is no record, or a sentence in `first-use.md` and `wake.md`.

F11 `docs/design/wake.md:36`, judgement, note, INFEASIBLE, introduced.
- Measured: "a mark is never sent for a row the surface did not take" overclaims. A write that fits in the pipe buffer succeeds even if the reader then exits without reading it, and the row is marked.
- Fix: soften the line in the document. Code cannot close this gap.

F12 `crates/inbox-cli/src/wake.rs:162`, judgement, note, CONFIRMED, introduced.
- Measured: the wake overrides an inherited `SIG_IGN` for SIGINT. A Ctrl-C at a non-interactive script that started `inbox wake &` ends the wake, against the POSIX convention that a background job keeps an ignored SIGINT.
- This is a design choice the document states, so it is raised only as a note.

F13 `crates/inbox-cli/tests/wake.rs:653`, judgement, note, CONFIRMED, introduced.
- Measured: the acceptance case ran 3.858 s alone once (`accept-unit-2.log`), over the story's limit of 3 s per case on this host.

5. Attacked and not broken
- Mutants caught by `tests/wake.rs`:
  - m1, marks before the flush: caught by `the_rows_are_written…` and `a_closed_standard_output…`.
  - m2, a mark for every row: caught by `the_rows_are_written…`.
  - m3, the lock after the first request: caught by the lock-file, second-wake and acceptance cases.
  - m4, a lock per outbox file instead of per address: caught by the unit case of the lock path, the lock-file case and the second-wake case.
  - m5, 408 and 503 end the wake: caught by `a_service_that_answers_503…`.
  - m6, a pin mismatch tried again: caught by `a_certificate_that_is_not_the_pinned_one…`.
  - m7, the pause never grows: caught by `the_pause_starts_short…`.
  - m8, the regular-file check dropped: caught by `a_fifo_or_a_link_to_a_device…`.
  - m11, the print error ignored: caught by `a_closed_standard_output…`.
  - m12, `Certificate` turned back into `Transport` in `client.rs`: caught by the pin case.
- Code read only:
  - The `Failure::Certificate` change leaves other commands as they were: exit 3, message `transport: …`, JSON error `transport`, and `unsent` and the reconnect's "gone" also cover `Certificate`.
  - A handshake cut by the peer stays `UnexpectedEof`, so it is a transport failure and is tried again (`inbox-server/src/tls.rs:451-455`).
  - 401 and 400 end the wake with 1.
  - The text sent to standard output and standard error is escaped (`warn` and `print_checked` go through `visible` and `plain_document`).
  - The kernel releases the `flock` on SIGKILL.
- Not checked:
  - A TLS handshake cut while the program runs.
  - SIGPIPE and SIGHUP while the program runs, and a signal in the middle of the marks.
  - A full pipe, and `/dev/full` as standard output.
  - A relative `--outbox` with a changed working directory, and an outbox file reached through a symbolic link to the file itself. From the code, that link gives 2 lock files, so 2 wakes could run, the same as `<outbox>.lock`.
  - `inbox close` during a wake, while the program runs.
  - `generated/cli/inbox.json` against the help beyond the unit's own test, and `status.md` and `limits.md` beyond a grep.
  - Mutants that re-check "0 to 4" lines (known).

6. Paths written outside the worktree, all under `<scratch>/s3/wake/adversary/`:
- `copy/`: the archive of 1a5f274 with my 2 test paths. Its `target/` and my own `.git` are deleted.
- `orig/wake.rs` and `orig/client.rs`: the originals the mutants were restored from.
- `mut/`: the m1 to m12 `.diff` and `.log` files, plus `m9-full.log` and `m10-full.log`.
- `mutants.sh`, `mut2.sh`, `accept.sh`, `probe1.sh`: scratch scripts in POSIX sh and perl, not for the repository.
- `mutants.out`, `mut2.out`, `accept.out`, `accept-{unit-1..4,m9-1..3,m10-1..3}.log`.
- `build1.log`, `red1.log`, `red2.log`, `red3.log`, `green-mk.log`, `clippy.log`, `suite.log`, `suite2.log`.
- `base.msg`.
- The probe directory `p1/`, with keys made at run time, is deleted.

7.
```findings
[
  {"file": "crates/inbox-cli/src/wake.rs", "line": 141, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "A round whose reads are both answered at once without a row is started again with no pause, so a service with --max-wait 0 or any 2xx answer without rows makes the wake spin (176 reads in 2 s against a one-at-a-time stand-in)."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 58, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Every Failure::Transport is tried again forever, including a 3xx, a 404 without refused and a non-JSON answer, while docs/design/wake.md names only an unreachable service or a cut answer as tried again."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 138, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The wait=0 read returns on a due note alone, whose mark is refused Expired with exit 1, again on every wake until the sweep, which the service's held wait was designed to prevent."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 101, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Started with descriptor 1 closed, the wake's write succeeds through std's EBADF rule and it marks the row delivered with nothing printed and exits 0."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 212, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The lock file is opened without O_NOFOLLOW, so a dangling link in its place creates the file it names elsewhere and the wake runs on, in a directory only its owner can normally write."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 130, "category": "concurrency", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "When the start changes between the wait=0 read and the held read, the wake waits up to 60 s on the new start without opening the inbox there, against docs/design/wake.md:17-24."},
  {"file": "crates/inbox-cli/tests/wake.rs", "line": 679, "category": "concurrency", "severity": "blocker", "verdict": "CONFIRMED", "origin": "introduced", "message": "The acceptance case failed with 'the wake opened the inbox again at the new start: not within 10 s' in 3 of 4 package runs here, 1 of 2 of them on the unmodified unit."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 132, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping the reset of failures after an answer survives tests/wake.rs, and adversary_wake::each_run_of_failures_after_an_answer_is_said_again catches it."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 86, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping end_on_signal() survives tests/wake.rs because no case starts a wake with SIGTERM ignored, and adversary_wake::a_wake_started_with_sigterm_ignored_still_ends_on_sigterm catches it."},
  {"file": "website/docs/first-use.md", "line": 175, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A wake of an inbox with no open record on this host waits silently forever while posts to it are refused UnknownInbox, which 'it opens your inbox again first' does not say."},
  {"file": "docs/design/wake.md", "line": 36, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "'a mark is never sent for a row the surface did not take' overclaims: a write that fits in the pipe buffer is marked even when the reader exits without reading it."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 162, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The wake overrides an inherited SIG_IGN of SIGINT, so a Ctrl-C at a script that started 'inbox wake &' ends it, against the POSIX convention for background jobs."},
  {"file": "crates/inbox-cli/tests/wake.rs", "line": 653, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The acceptance case ran 3.858 s alone once on this host, over the story's 3 s limit per case."}
]
```
