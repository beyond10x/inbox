---
format: aep.planning-md/3
id: review-result:adversary-inbox-wake-returns-on-a-post-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on inbox wake (rounds 2 and 3): 7 findings, 4 red cases, 4 surviving mutants with 3 cases'
relations:
- reviews: story:inbox-wake-returns-on-a-post
revision: 1
---
unit: story:inbox-wake-returns-on-a-post, adversary pass 2 on rounds 2 and 3, commit b17c77c (attacked in a git-archive copy, nothing written in the tree)
verdict: CONFIRMED (4 red cases of my own; 4 mutants survived the suite, and 3 green cases of mine catch them)
cases: executed 359→367, red 4
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 scratch directory, <scratch>/s3/wake/adversary2/ (every path is in part 6)
needs-coordinator: F1 (whether a body without Content-Length that ends with the connection counts as whole) and F4 (round 3 decision 4 asked for the check before the lock and the first request; the code and wake.md check before the write)

1. Diff of the copy against b17c77c (the copy has no git, so this is `diff -rq` against a fresh archive):
```
Only in copy/crates/inbox-cli/tests: adversary_wake_2.rs   (434 lines, new)
Files ref/crates/inbox-cli/tests/main.rs and copy/.../main.rs differ   (+1 line: mod adversary_wake_2;)
```
Both are test paths, so there is no charter violation. The mutants were applied to the copy's src/ files by script and copied back from `orig/` after each one. After the last one, a diff -r against a fresh archive showed only the 2 test paths.

2. Cases in `<scratch>/s3/wake/adversary2/copy/crates/inbox-cli/tests/adversary_wake_2.rs`. The red lines are from the first run of the new cases alone (red1.log, red2.log).
- `a_whole_404_page_without_content_length_ends_the_wake_with_3`: RED, 0.4 s. "inbox wake still runs after 3 reads answered by a whole 404 page without Content-Length; it said: inbox wake: transport: …: the answer with the status 404 is not JSON: at byte 0: expected a value; trying again until the service answers; left: None right: Some(3)"
- `a_reopen_that_gets_no_answer_does_not_end_the_wake_as_never_opened`: RED, 0.3 s. "it said: inbox: reconnect at the start 5555… of the service: the open of agent:test/bob again failed: transport: …: the answer has no end of its head; the next command tries again / inbox wake: agent:test/bob is not open at the service, and no `inbox open` of it on this host opened it again: run `inbox open` first; left: Some(1) right: None"
- `a_standard_output_open_only_for_reading_marks_nothing_and_exits_4`: RED, 0.15 s. "inbox wake with a standard output it cannot write marked the row (exit, marks); left: (Some(0), 1) right: (Some(4), 0)"
- `a_wake_whose_standard_output_no_one_reads_exits_4_before_its_first_request`: RED, 10.1 s (it runs to the hang bound when red; when green it ends at once). "left: (None, 14) right: (Some(4), 0)": 14 reads of Pending in 10 s with stdout on /dev/null, holding the lock.
- `a_500_is_tried_again_and_a_499_ends_the_wake_with_3`: GREEN, 0.27 s. Red on C1 and C1b.
- `a_service_that_is_never_open_after_an_open_ends_the_wake_with_1`: GREEN, 0.37 s. Red on C2.
- `a_repost_answered_404_by_another_server_marks_the_envelope_unsent`: GREEN, 0.25 s. Red on C4: "the envelope a 404 of another server met is marked unsent: \"\"".
- rustfmt --edition 2024 was run on the file. `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0.

3. Suite runs after the cases existed: `SLOT_TIMEOUT=20m cargo nextest run -p inbox-cli -p inbox-server --locked --no-fail-fast`, each exit 100.
- suite1 (2 cases present then): 361 run, 359 passed, 2 failed (my 2).
- suite2 (7 cases): `Summary [38.541s] 366 tests run: 362 passed, 4 failed`, the 4 red cases above.
- suite3 (6 cases, while I reworked the repost case): 365 run, 361 passed, 4 failed, the same 4.
- The repost case was added last and run alone (green) and against C4 (red). I did not run a full suite with all 8 cases: 359 + 8 = 367.
- The 359 before is the count with my file deselected in the mutant runs ("359 tests run … 2 skipped").
- Stability: no existing case was red in one run and green in another, over the 3 suite runs plus 11 mutant runs.
- No case of tests/wake.rs, adversary_wake.rs or undelivered.rs ran over 3 s. The slowest were adversary_wake spin at 2.05 s and the acceptance case at 1.90 s.

4. Findings (all cover b17c77c)
F1 `crates/inbox-cli/src/client.rs:269`, contract-drift, warning, NEEDS-CHANGE, introduced.
- Measured: `whole` is true only with a Content-Length. A 404 HTML page without one, which ends with its connection (the client sends Connection: close), counts as cut: it is a Transport failure, so the wake tries it again forever.
- This contradicts wake.md:123 ("a whole answer whose body is not JSON … exits 3") and first-use.md:184 ("a 404 of another program on the URL ends it with 3").
- What reaches it: an INBOX_URL that names another loopback HTTP server that answers without Content-Length.
- Fix: count an answer without Content-Length and without chunked Transfer-Encoding as whole at EOF, or document it in wake.md's table.

F2 `crates/inbox-cli/src/wake.rs:219`, concurrency, warning, CONFIRMED, introduced.
- Measured: after a 409 with a new epoch, the reconnect's open fails in transport. The reconnect itself says "the next command tries again", but the wake exits 1 with a line saying no `inbox open` on this host opened it and to run `inbox open`. Both are false: the record exists, and the failure is the kind the table says is tried again.
- The same guard also ends the wake when two real restarts come in a row: a 409 at a third start right after a successful reopen.
- What reaches it: a service that goes away again between its 409 and the open, for example a crash loop or two restarts in a row.
- Fix: when the reconnect's open failed in transport (its `gone`), take the pause of a failure and read again, instead of NotOpen. Compare epochs in the `reopened` guard.

F3 `crates/inbox-cli/src/output.rs:72`, acceptance, warning, CONFIRMED, introduced.
- Measured: with descriptor 1 open read-only (`inbox wake 1<file`), the duplicate and the fstat succeed and it is not /dev/null, so `reaches_a_reader` passes. Each write then fails with EBADF, which std's stdout counts as written. The row is marked delivered, and the wake exits 0 with no byte written.
- This is against wake.md:81 ("a standard output that cannot be written … without a mark, exit 4").
- What reaches it: a parent that hands over fd 1 read-only. That is uncommon; the class is the same as pass 1's F4.
- Fix: write the rows through a `File` made from a duplicate of fd 1 instead of `std::io::stdout()`, or check that `fcntl(1, F_GETFL)` has write access.

F4 `crates/inbox-cli/src/wake.rs:133`, acceptance, warning, NEEDS-CHANGE, introduced.
- Measured: round 3 decision 4 says the check that fd 1 is open runs "before the lock and the first request" and exits 4. The code checks only when rows are to be written.
- With fd 1 closed or on /dev/null, the wake takes the lock and reads Pending 14 times in 10 s on an empty inbox. Meanwhile a working wake of that inbox exits 5, and the next message is then not delivered (exit 4).
- wake.md:85 documents the code's choice, so this is a mismatch between the brief and the build.
- Fix: call `output::reaches_a_reader` before `lock()`, or the coordinator accepts the change.

F5 `crates/inbox-cli/src/wake.rs:87`, mutant, warning, CONFIRMED, introduced.
- Measured: `status >= 500` changed to `>= 501`, or to `>= 499`, survives the full suite of both packages. My case `a_500_is_tried_again_and_a_499_ends_the_wake_with_3` catches both.
```
-    status == 408 || status >= 500
+    status == 408 || status >= 501
```

F6 `crates/inbox-cli/src/wake.rs:219`, mutant, warning, CONFIRMED, introduced.
- Measured: dropping the `reopened` guard survives the suite. Against a stand-in that answers 409 from a new start every time and takes every open, the wake would open and read forever. My case `a_service_that_is_never_open_after_an_open_ends_the_wake_with_1` catches it.
```
-                if !opened || reopened {
+                if !opened {
```

F7 `crates/inbox-cli/src/commands.rs:556`, mutant, warning, CONFIRMED, introduced.
- Measured: dropping `Unexpected` from `unsent` survives the suite. It is equivalent for `post`, because only the Refused branch reads `not_reached`. It is live for `repost`: a kept envelope with no epochs line that meets a 404 of another server is no longer marked unsent, while on main (a Transport failure) it is. My case `a_repost_answered_404_by_another_server_marks_the_envelope_unsent` catches it.
```
-        Err(Failure::Transport(_) | Failure::Certificate(_) | Failure::Unexpected { .. }) => true,
+        Err(Failure::Transport(_) | Failure::Certificate(_)) => true,
```

Mutants, each run with `cargo nextest run -p inbox-cli [-p inbox-server] --locked --no-fail-fast`, my file deselected (server mutants on both packages, client mutants on inbox-cli; logs in mut/):
- S1, the open check only before the hold (the loop's check made first-time-only, the check at the bound dropped): caught by `undelivered::a_read_with_undelivered_is_never_held_for_an_inbox_that_is_not_open` and `wake::a_wake_of_an_inbox_that_is_not_open_ends_with_1…`.
- S2, the `state == Sent` filter dropped in `undelivered_rows`: caught by 3 cases of undelivered.rs.
- S3, the due comparison flipped at equality in `not_due`: caught by `adversary_deadline_wait::one_wait_of_a_second_on_a_note_due_at_the_reading_reads_its_view_once` ("held 11ms"). `not_due` is shared with the plain wait; undelivered.rs has no case at the equality itself.
- C1 (`>= 501`) and C1b (`>= 499`): survived. Caught by my 500/499 case.
- C2, the `reopened` guard dropped: survived. Caught by my never-open case.
- C3, NotOpen mapped to exit 3: caught by `wake::a_wake_of_an_inbox_that_is_not_open_ends_with_1…`.
- C4, Unexpected dropped from `unsent`: survived. Caught by my repost case.
- C5, QUICK set to 0: caught by `adversary_wake::an_empty_answer_that_comes_at_once…` and `wake::an_answer_at_once_with_only_a_delivered_row…`.
- C6, the /dev/null test dropped with the duplicate kept: caught by `adversary_wake::a_wake_started_with_standard_output_closed_marks_nothing` and `wake::a_wake_whose_standard_output_is_dev_null…`.
- C7, the client's `in_sent` guard dropped (an extra one): caught by `wake::the_rows_are_written_before_the_first_mark…` and `wake::an_answer_at_once_with_only_a_delivered_row…`.

5. Attacked and not broken (code read unless I say otherwise)
- Authentication runs before `wait()` and the open check in every path (service.rs:75 before :90).
- The signed target is `path?query` as received, so adding, removing or reordering `undelivered` breaks the signature.
- The key given twice is 400. A percent-encoded key or value decodes to the same parameter, which is harmless. `undelivered=true`, an empty value and `01` are 400 (and tested).
- The open check is on the caller's own inbox, not on the query's `inbox=`. Since every principal's address is its inbox, nothing about another principal's inbox is learned. With `inbox=bob`, alice gets only her own rows to bob in Sent, which the read rule gives her without the parameter too.
- Without the parameter, `wait()` and `waited()` take the same paths as before. I did not build main to compare the bytes.
- The due side at equality is the same function the marks use (S3 is caught).
- The service does not spin: it sleeps on the command generation.
- The reaches_a_reader duplicate is dropped at once, so no descriptor leaks. It runs once per wake.
- /dev/null through a symbolic link is caught (it fstats the fd). A closed pipe gives EPIPE and exit 4.
- A 409 without an epoch, or at the recorded epoch, ends the wake after 1 read.
- A record only for alice, with a wake of bob, ends with 1.

Not checked:
- main's binary byte-for-byte (part of point 5).
- `max_connections` filled with held reads that carry the parameter.
- A close followed by an open inside the held window (a race where the wake exits 1 although the inbox is open again).
- /dev/zero as standard output (it passes the check and marks the row; I judged it too far-fetched to raise).
- A socket on fd 1 whose peer is gone.
- The service's stop with the parameter, beyond reading the code.
- CHANGELOG, first-use, status and AGENTS against the code, beyond the sentences named in F1 and F4.

6. Paths written outside the worktree, all under `<scratch>/s3/wake/adversary2/`:
- `copy/`: the archive of b17c77c with my 2 test paths. Its `target/` is deleted.
- `orig/` (service.rs, wake.rs, failure.rs, commands.rs, output.rs) and `mut/` (S1-S3, C1-C7 .diff and .log files, and C1/C1b/C2/C4 -2.log).
- Scratch scripts in POSIX sh, not for the repository: `mutants.sh`, `mut2.sh`, `mut3.sh`.
- Outputs: `mutants.out/.stdout`, `mut2.out/.stdout`, `mut3.out/.stdout`.
- Logs: `build1.log`, `red1.log`, `red2.log`, `suite1.log`, `suite2.log`, `suite3.log`, `clippy.log`, `clippy2.log`, `g-unit.log`, `g-c4.log`.
- `code.diff`, `main-commands.rs`.
- Case drafts: `caseC.rs.txt` to `caseG.rs.txt`, and `adversary_undelivered.rs.txt`, a server case I did not use because S3 was caught.

7.
```findings
[
  {"file": "crates/inbox-cli/src/client.rs", "line": 269, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A whole non-JSON answer without Content-Length (a 404 page of another server ending with its connection) is counted as cut, so the wake tries it again forever instead of exiting 3 as wake.md:123 and first-use.md:184 say."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 219, "category": "concurrency", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "When the reconnect's open after a 409 fails in transport, or a second restart follows a reopen, the wake exits 1 saying no inbox open on this host opened it and to run inbox open, though the record exists and the failure is one the table tries again."},
  {"file": "crates/inbox-cli/src/output.rs", "line": 72, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "With descriptor 1 open read-only, reaches_a_reader passes, std's stdout counts the EBADF writes as written, and the wake marks the row delivered and exits 0 with nothing written."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 133, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Round 3 decision 4 puts the standard-output check before the lock and the first request, but the wake checks only before writing rows, so a wake on /dev/null or a closed fd 1 holds the lock and reads Pending until a message comes, which it then does not deliver."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 87, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "passing() moved to >= 501 or >= 499 survives the suite of both packages; adversary_wake_2::a_500_is_tried_again_and_a_499_ends_the_wake_with_3 catches both."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 219, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping the reopened guard survives the suite; adversary_wake_2::a_service_that_is_never_open_after_an_open_ends_the_wake_with_1 catches it."},
  {"file": "crates/inbox-cli/src/commands.rs", "line": 556, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping Unexpected from unsent survives the suite and stops repost from marking unsent an envelope that met a 404 of another server; adversary_wake_2::a_repost_answered_404_by_another_server_marks_the_envelope_unsent catches it."}
]
```
