---
format: aep.planning-md/3
id: review-result:adversary-chat-bridge-posts-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the chat bridge that posts: 7 findings, 3 red cases'
relations:
- reviews: story:the-chat-bridge-posts-a-pending-message
revision: 1
---
unit: story:the-chat-bridge-posts-a-pending-message, tree inbox-bl-s4-post at HEAD 2b88029 (wip on e84f170); cases run in a copy of HEAD
verdict: NEEDS-CHANGE
cases: executed 294→299, red 3
origin: introduced 6 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s4/post/adversary/ (copy and logs, listed in part 6)
needs-coordinator: move copy/crates/inbox-cli/tests/adversary_bridge.rs and its `mod adversary_bridge;` line in tests/main.rs into the tree

**1. Diff of the tree**

`git diff --stat` of the tree is empty. I wrote nothing in the tree. `git status` shows one untracked file that is not mine: `.engineering/planning/approval-record/d89-the-chat-bridge-posts-under-a-standing-grant-of-connectors.md`.

In the copy I added one test file, `crates/inbox-cli/tests/adversary_bridge.rs`, and one line in `tests/main.rs`. I mutated `src/bridge.rs` and `src/output.rs` in the copy for one run and then restored them from HEAD; a `diff -r` against `git archive HEAD` shows no difference in `src`.

**2. My cases**

All 5 are in `copy/crates/inbox-cli/tests/adversary_bridge.rs`. Each uses its own POSIX sh stand-in `connectors` and a real `inbox serve` on loopback. The first run of the 5 alone was `run1.log`. Times are from the green-source suite run (`run4-suite.log`).

- **`a_signal_to_the_bridge_leaves_no_connectors_child_running`** (group 4). Red, 1.0 s.
  - What it does: the stand-in holds its answer, the case sends SIGTERM to the bridge, waits for the bridge to exit, then checks whether the child still runs.
  - Red line: `the connectors child 189979 still runs after the bridge ended by SIGTERM`.
  - The lock is freed; only the child is left running.
- **`a_grandchild_holding_the_pipes_does_not_hold_the_post_of_an_ended_child`** (group 4). Red, 5.8 to 6.4 s.
  - The time is over 5 s only while red: the case gives the bridge 5 s. A fixed bridge passes in well under 1 s.
  - What it does: the stand-in answers `applied`, exits 0, and leaves a background process that holds its stdout and stderr.
  - Red line: `the note is not Delivered 5 s after connectors exited 0 with `applied`: a grandchild holds its standard output`.
- **`a_bridge_started_before_the_service_answers_waits_for_it`** (groups 6 and 7). Red, 1.4 to 2.0 s.
  - What it does: `INBOX_URL` points at a closed loopback port.
  - Red line: `the bridge ended with Some(ExitStatus(unix_wait_status(768))) while the service did not answer: inbox bridge: transport: 127.0.0.1:44993: 127.0.0.1:44993: Connection refused (os error 111)`. That is exit 3 at the first request, where the rounds of `wake --follow` try again until the service answers.
- **`a_body_of_3001_bytes_is_cut_to_3000_and_one_of_3000_is_whole`** (group 8). Green, 1.2 s.
  - It kills mutant M15, which none of the story's cases catch.
- **`after_a_failed_post_the_next_invocation_is_the_same_message`** (group 8). Green, 1.2 s.
  - It kills mutant M14, which none of the story's cases catch.

**3. Suite runs** (logs in the scratch directory)

- Mutant run, `run2-mutants.log`: M14 and M15 applied together, filter `test(/bridge/) | test(/chat_post|a_body_over_the_bound|a_run_of_3|head_line_of_a_chat/)`.
  - Result: `20 tests run: 15 passed, 5 failed`.
  - All 14 story cases (`bridge::*`, `bridge::tests::*`, and the 3 `output::tests` cases of the chat post) passed.
  - Killed only by my cases: the order case shows `left: ["inbox-…SSB", "inbox-…QEQ", "inbox-…SSB"]` against the expected A, A, B; the cut case shows a block of 3001 `b`.
- `run3-suite.log`: by mistake this ran on a build that still held the mutants. The restored files carried the archive's older mtime, so cargo did not rebuild them. One useful number from it: with both mutants in, the whole crate suite without my file ran `294 tests run: 294 passed`. No case anywhere in the crate kills M14 or M15. That run is the source of `executed 294`; it excludes `adversary_bridge::*`.
- Valid run, `run4-suite.log`, after touching the sources: `cargo nextest run -p inbox-cli --locked --no-fail-fast` gave `299 tests run: 296 passed, 3 failed`, exit 100. The 3 failures are my red cases. Step time 52.9 s.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0.
- The copy's `target` directory is deleted.

**4. Findings**

- **F1, signal leaves the child running** — `crates/inbox-cli/src/bridge.rs:367`. CONFIRMED, introduced, warning.
  - Measured: `a_signal_to_the_bridge_leaves_no_connectors_child_running`.
  - The bridge spawns `connectors` in its own process group and dies by the default action of SIGTERM. The child is never signalled.
  - What reaches it: any `systemctl stop` or `kill` of a bridge during a post. A `connectors` that hangs then lives forever. A child that finishes makes a post nobody records or marks; the next bridge posts it again with the same key.
  - Fix: on SIGTERM or SIGINT, end the child (its own process group, killed in the signal path) before exiting. Or document it in `docs/design/bridge.md` and `limits.md`, which today only say "a connectors that never ends holds the bridge".
- **F2, grandchild holds the post** — `crates/inbox-cli/src/bridge.rs:373-384`, and `taken` at :404. CONFIRMED, introduced, warning.
  - Measured: `a_grandchild_holding_the_pipes_does_not_hold_the_post_of_an_ended_child`.
  - `invoke` reads both pipes to EOF before `wait()`. A helper process that inherits the pipes holds a finished post until the helper ends.
  - The design says the rest is read and dropped "so the child is not held". The reverse happens: the bridge is held by any holder of the pipe.
  - What reaches it: a real `connectors` that starts a background helper or agent; not shown for a released `connectors`.
  - Fix: wait for the child and stop reading at its exit (a bounded drain after exit), or close inherited descriptors.
- **F3, bridge ends at start when the service does not answer** — `crates/inbox-cli/src/bridge.rs:213`, `Surface::started` with `commands::open(client, ADAPTER)`, called at `wake.rs:244`. CONFIRMED, introduced, warning.
  - Measured: `a_bridge_started_before_the_service_answers_waits_for_it`.
  - The open at the start is outside `held`'s tries. A Transport failure ends the bridge with 3. `inbox wake --follow` against the same dead service waits, with "trying again until the service answers".
  - `docs/design/bridge.md` "The rounds" says the bridge runs "the tries and pauses" of the wake.
  - What reaches it: a bridge started with or before `inbox serve` (boot, a service restart).
  - Fix: run the start open through `tried_again` and `pause`, as `held` does.
- **F4, mutant M14 survives the story's cases** — `crates/inbox-cli/src/bridge.rs:249`. CONFIRMED, introduced, warning.
  - Mutant: `return Ok((first, marked));` → `continue;`, so a failed post no longer ends the round.
  - All story cases stay green, because every failure case has one message in `Pending`.
  - Killed by `after_a_failed_post_the_next_invocation_is_the_same_message`.
- **F5, mutant M15 survives the story's cases** — `crates/inbox-cli/src/output.rs:207`. CONFIRMED, introduced, note.
  - Mutant: `if taken + piece.len() > MOST_CHAT_BODY_BYTES {` → `> MOST_CHAT_BODY_BYTES + 1`.
  - Why it survives: no story case puts a 1-byte piece across 3000/3001. The pieces used are `é` (2 bytes), `&amp;`, `&lt;`, `&gt;` and the 6-byte `\u{1b}`.
  - Killed by `a_body_of_3001_bytes_is_cut_to_3000_and_one_of_3000_is_whole`.
- **F6, the record grows without bound** — `crates/inbox-cli/src/outbox.rs:422-438`, `record_chat_post`. CONFIRMED (by reading), introduced, note. No case: one would need a planted file of 64 MiB.
  - Each post reads, parses and rewrites the whole record under the outbox lock, and no line is ever removed.
  - That is O(n) per post. After about 700 thousand posts (about 90 bytes per line, bound 64 MiB) every post ends the bridge with 4: posted, not marked. Each restart posts the same message again and dies again.
  - The documents state "No entry is ever removed yet" and the bound, but not that the bridge then stops serving for good.
  - Fix: pruning, which the later story that reads the record could add, or a line in `limits.md`.
- **F7, the staged file of `replace` (asked in group 5)** — `crates/inbox-cli/src/outbox.rs:622`, the shared `replace`. INFEASIBLE, pre-existing, note.
  - The staged `<record>.new` is opened with create and truncate, without `O_NOFOLLOW`, `O_EXCL` or a mode fix.
  - A link planted there is written through to its target, and the link is then renamed over the record. A leftover `.new` of another mode gives the record that mode, not 0600.
  - Nobody reaches this but an actor who can already write the owner's outbox directory; a crash leaves a `.new` of mode 0600. The rule in the unmerged branch covers it.

**5. Attacked and not broken**

- Key: `inbox-<message id>` only. Message ids are restricted to the Crockford alphabet by `MessageId`, so a sender cannot put text into an argument.
- Text of a post, by reading `chat_post`:
  - every `<`, `>` and `&` is escaped;
  - runs of 3 or more backticks are escaped in the body, and every backtick in the head line;
  - CR, U+2028 and U+2029 are escaped;
  - the cut falls at whole pieces, never inside an escape or a multi-byte character;
  - an `&` already followed by `lt;` becomes `&amp;lt;`.
  
  I found no way to leave the block. The JSON goes through `push_text`.
- JSON from the child: `json::parse` has a depth bound of 64, so a document nested 10000 deep is refused, not a stack overflow. Two documents, a `ts` that is a number or empty, and `retry_after_seconds` out of range or not a whole number are refused, as the unit cases cover.
- Binding checks: empty values, white space, control characters and a connector starting with `-`. The channel and the user are never in argv.
- Lines on stderr all go through `output::warn`.
- Lock: Rust opens files with `O_CLOEXEC`, so the child does not inherit it.
- `wake.rs` after the refactoring: the order of the checks and the line texts are the same as at e84f170 (the refusal message is identical). The `wake`, `reader_gone` and `follow` cases are green in the run of 299.
- `limits.md`, `status.md` and the CHANGELOG bullet name no story id, stage, wave or test count.

Not reached:
- Group 3 in part: `--allowed-chat-user` and channel values with U+202E or very long values were not run.
- Group 5 in part: a record that is not UTF-8, and a FIFO or link at the record name (read path, by reading only).
- Group 6 in part: loopback look-alike URLs (pre-existing client check), `PATH` unset or empty, a relative entry, a file with mode `0o001`, two bridges with two outbox files.
- Group 9 in part: `AGENTS.md` rows and `generated/cli/inbox.json` against the help text, not compared line by line.
- Mutants not run: a `status` 200, `ok` and empty-`ts` mutant, and the head-line backtick mutant. By reading, the unit cases `only_an_applied_answer_with_a_reference_is_a_post_made` and `the_head_line_of_a_chat_post_holds_no_backtick_and_no_line_end_of_a_sender` assert those exact shapes.

**6. Paths written outside the worktree**

- <scratch>/s4/post/adversary/copy/ (the copy; `target` deleted)
- <scratch>/s4/post/adversary/copy/crates/inbox-cli/tests/adversary_bridge.rs (the cases to move)
- <scratch>/s4/post/adversary/copy/crates/inbox-cli/tests/main.rs (the `mod adversary_bridge;` line)
- <scratch>/s4/post/adversary/run1.log, run2-mutants.log, run3-suite.log, run4-suite.log
- <scratch>/s4/post/adversary/waitlog.sh, suite.sh, suite2.sh
- Each case's own world directory is made through `support::Dir`, the same as the story's cases.

```findings
- file: crates/inbox-cli/src/bridge.rs
  line: 367
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a SIGTERM to the bridge during a post leaves its connectors child running, as a_signal_to_the_bridge_leaves_no_connectors_child_running shows.
- file: crates/inbox-cli/src/bridge.rs
  line: 373
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: invoke reads both pipes to EOF before wait, so a grandchild that holds them keeps a post that ended with applied from being recorded and marked.
- file: crates/inbox-cli/src/bridge.rs
  line: 213
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the open at the start is not tried again, so a bridge started while the service does not answer exits 3, where the rounds of wake --follow wait for it.
- file: crates/inbox-cli/src/bridge.rs
  line: 249
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: replacing the end of the round after a failed post with continue survives every case of the crate, and the order case in adversary_bridge.rs kills it.
- file: crates/inbox-cli/src/output.rs
  line: 207
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: moving the cut of the body by one byte survives every case of the crate, and the 3000/3001 byte case in adversary_bridge.rs kills it.
- file: crates/inbox-cli/src/outbox.rs
  line: 422
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the record of the chat posts is rewritten whole on each post and never pruned, so after about 700 thousand posts every post ends the bridge with 4 and is posted again on each restart.
- file: crates/inbox-cli/src/outbox.rs
  line: 622
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: the shared replace opens the staged .new without O_NOFOLLOW or O_EXCL and sets mode 0600 only when it creates it, which only a writer of the owner's outbox directory can use.
```
