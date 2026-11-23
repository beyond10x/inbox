---
format: aep.planning-md/3
id: review-result:adversary-unreadable-epochs-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the unreadable epochs file: 6 findings, 2 red cases'
relations:
- reviews: story:an-unreadable-epochs-file-keeps-its-other-lines
revision: 1
---
unit: story:an-unreadable-epochs-file-keeps-its-other-lines, tree <tree> at 0054dca (branch backlog/epochs-file-unreadable), base 9c88a0d
verdict: NEEDS-CHANGE
cases: executed 270→275, red 2
origin: introduced 6 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (<scratch>/bl/epochs/adversary/)
needs-coordinator: move the case file and its `mod` line from the copy into the tree

Two reachable cases break the acceptance ("exit status 4 and one line that names the file"). Neither one loses a line: the epochs file stayed byte-identical in every case I built. Breach to report myself: I ran `git init -q .` once inside my scratch copy by mistake and deleted that `.git` at once. Nothing in the tree was touched (`git -C <tree> status` is clean).

**1. Diff of the copy against the tree** (the tree has no commit for my files, so this is `git diff --no-index` and `diff -rq`)
- `crates/inbox-cli/tests/main.rs`: 1 insertion, `mod adversary_unreadable_beside;` in sorted order.
- `crates/inbox-cli/tests/adversary_unreadable_beside.rs`: new, 256 lines.
- No other path differs. The two mutated sources were put back and `cmp` confirmed it ("restored").

**2. Cases added**, all in `<scratch>/bl/epochs/adversary/copy/crates/inbox-cli/tests/adversary_unreadable_beside.rs`. Times are from the suite run.
- `a_repost_answered_503_over_an_unreadable_epochs_file_exits_4`: RED, 0.39 s. The epochs file is not UTF-8 and a stand-in answers the post `503 {"refused":"stopping"}`. The file stays byte-identical, but the exit status is 1, not 4. First run, verbatim:
  `assertion left == right failed: Run { status: 1, stdout: "", stderr: "inbox repost: …/alice.outbox.epochs: stream did not contain valid UTF-8\ninbox repost: 01M83TK1Y2C51RFSQ3J2BKAJPX to agent:test/bob: refused: stopping (503)\ninbox: refused: stopping (503)\n" } left: 1 right: 4`
- `a_repost_over_an_epochs_file_it_cannot_read_names_it_in_one_line`: RED, 1.17 s. The epochs file is a FIFO, then a file of 64 MiB + 1. First run, verbatim:
  `fifo: exit 4, 3 lines name the file: ["inbox: …/alice.outbox.epochs: it is no regular file; read as holding nothing", "inbox: …/alice.outbox.epochs: it is no regular file", "inbox: …/alice.outbox.epochs: it is no regular file; read as holding nothing"]`
  The same three lines appear for "over its bound" ("it holds more than 67108864 bytes").
- `a_post_over_an_unreadable_epochs_file_writes_one_line_without_its_content`: GREEN, 1.07 s. It covers all 4 states: exactly one line on standard error, exit 4, and no byte of the file's content in the line.
- `an_open_over_an_unreadable_opened_file_opens_the_inbox_and_prints_it`: GREEN, 0.63 s. Exit 4, the outcome is printed, the file is unchanged, and a later post to bob arrives, so the service did open the inbox.
- `a_post_over_an_epochs_path_of_another_kind_exits_4_and_leaves_it`: GREEN, 0.38 s. It tries a directory, a socket, a FIFO with a writer, a link to `/dev/null`, a link loop and a link to a directory. Each exits 4 within 2 s, with one line, and the inode and mode are unchanged.
- The first run had 2 more reds that were my fixture's errors, not findings: the socket path was over SUN_LEN, and I expected the outcome `opened` where the service answers `reopened`. I fixed both and both went green on the rerun.

**3. Suite**, run after the cases existed: `SLOT_TIMEOUT=10m cargo nextest run -p inbox-cli --locked --no-fail-fast` gave `Summary [24.373s] 275 tests run: 273 passed, 2 failed, 0 skipped`, exit 100. The 2 that failed are the red cases above. The before count of 270 comes from the implementor's `impl/full1.log`.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exited 0.

**Findings**

F1. `crates/inbox-cli/src/commands.rs:916`. acceptance, warning, NEEDS-CHANGE, introduced.
- Measured: `repost_at` sends the failed `mark_unsent` to `settle_or_warn`, which only warns, instead of to `local`. A repost that has to mark an envelope unsent (answered 408, 5xx or `UnknownInbox`) over an unreadable epochs file exits 1, not 4.
- Documents it contradicts: the story's acceptance, `docs/design/service.md` ("exits 4 after the report"), the AGENTS.md row, and the doc comment of `repost`.
- What reaches it: an `inbox repost` after a restart, before the recipient opened its inbox again (`UnknownInbox`), or while the service stops (503).
- Fix: put that failure into `local`, as `record_epoch` and `clear_unsent` already are at :961.
- On origin: the base also exits 1 here, but it overwrote the file. The 4 is this unit's new contract, so I counted it as introduced.

F2. `crates/inbox-cli/src/outbox.rs:492`. acceptance, warning, NEEDS-CHANGE, introduced.
- Measured: for a FIFO or an over-bound file, `inbox repost` prints 3 lines naming the file. The first comes from `held_nothing` in `kept`. The second is the failure. The third comes from `read_or_nothing` in `Outbox::due` (:298), run by the reconnect after the repost.
- Two of the three say "read as holding nothing" beside an exit 4. The acceptance says one line.
- Fix: no warning on the read paths of a command that then fails on the file. Or the design names the 3 lines.

F3. `crates/inbox-cli/tests/unreadable_beside.rs:263`. mutant, note, CONFIRMED, introduced.
- Mutant: `opened_with` reads `.opened` before `sent_open` (commands.rs:173). It passes all 5 of the story's cases, because the case for `open` checks only the exit status, the line and the file.
- My case `an_open_over_an_unreadable_opened_file_opens_the_inbox_and_prints_it` kills it.

F4. `crates/inbox-cli/tests/unreadable_beside.rs:132`. mutant, note, CONFIRMED, introduced.
- Mutant: the old "read as holding nothing" warning written beside the failure (outbox.rs:490). It passes all 5 story cases, because `wrong()` asks only that *some* line names the path and reason, not that it is one line.
- My cases `a_post_over_an_unreadable_epochs_file_writes_one_line_without_its_content` and `a_post_over_an_epochs_path_of_another_kind_exits_4_and_leaves_it` kill it.

F5. `AGENTS.md:58`. contract-drift, note, NEEDS-CHANGE, introduced.
- The `crates/inbox-cli/` row still says a file that is no regular file or over its bound "is read as holding nothing … (a change of the outbox file fails instead)".
- Earlier sentences of the same row now say a change of the epochs or opened file fails too. The parenthesis should name all three files.

F6. `crates/inbox-cli/tests/unreadable_beside.rs:164`. judgement, note, CONFIRMED, introduced.
- `a_change_of_an_unreadable_epochs_file_fails_with_4_and_leaves_it_as_it_was` ran 2.0 to 6.9 s over 20 runs at a load average of about 100. It was over 5 s in 2 of 20 runs (6.92 s and 6.59 s).
- Cause: it makes and reads back (`held()` → `fs::read`) a 64 MiB + 1 file twice. It stays under the 60 s bound.

**Mutants** (each run against the story's 5 cases plus my 5)
- written-then-failed (the outbox file is written before the failure): killed by the story's epochs case.
- only-when-lines-grow (fails only under `post`, not under `repost`): killed by 3 of the story's cases.
- reason-dropped from the line: killed by the story's epochs case and its no-epoch case.
- other-files-path (the outbox path named in the line): killed by the story's cases.
- open-before-request: survived the story's cases. Killed only by my `open` case (F3).
- missing-as-unreadable-opened: killed by all cases.
- old-warning-beside: survived the story's cases. Killed by my 2 `post` cases (F4).
- repost-ignores-record-failure: killed by the story's epochs case, its no-epoch case and `epoch.rs`.
- mark-unsent-unread-dropped (the implementor's claim): survived all 10. It is equivalent: an `Epochs::unread()` has empty sets, so the first operand already sets `changed` on the one insert per change.

**Ran 20 times:** the story's 5 cases together (there are 4 in `unreadable_beside.rs` plus the changed one in `epoch.rs`, so 5, not the 6 the brief names). 0 red; slowest 6.92 s.

**Attacked and could not break**
- `post` in all 4 states: exit 4, one line, no request, and both the outbox and epochs files byte-identical.
- The other 6 file kinds: no hang, exit 4 within 2 s.
- `close` and `open` over `.opened` in 4 states: no write. `open` sends its request first.
- A missing file is still read as empty.
- The read-only paths (`outbox`, the `kept` step, `due`, `opened`) print the same text under the same conditions as `read_or_nothing` in 0.6.0, by reading the code.
- The 64 MiB + 1 file is really sparse on this ext4 file system: 0 blocks of 512.
- The root case of `make()` asserts with a message. I could not run it, since I am not root.
- What newly fails where 0.6.0 exited 0: post, repost, open and close in the 4 states. All are named in the story, the design and the CHANGELOG.
- After `open` exits 4, `close` exits 4 with nothing sent until the file is repaired. Each line names the file and the reason. The docs imply this but do not spell it out.

**Not checked**
- A file that turns unreadable between the read and the write under the lock.
- `replace()` following a symlink planted at `<path>.new`.
- A path with a line feed: by reading, `output::visible` keeps it raw, the same as for every other failure on the outbox file.
- BOM and CRLF in the epochs file: read only, not run. CRLF is kept by `str::lines`; a BOM makes the first line an entry of another form, which a write drops, as documented.
- `wake` and `--follow` over an unreadable `.opened`.

**Paths written outside the worktree**, all under <scratch>/bl/epochs/adversary/:
- `copy/` (the archive of HEAD plus my 2 test edits; its `target/` is deleted)
- `orig/` (outbox.rs and commands.rs saved for the mutants)
- `mutants/` (summary.txt and one log per mutant)
- `mutants.sh`, `mutants.out`, `repeat.sh`, `repeat.txt`, `repeat-run.log`, `repeat.out`
- `mine-red.log`, `mine-fixed.log`, `suite.log`, `clippy.log`

```findings
[
  {"file": "crates/inbox-cli/src/commands.rs", "line": 916, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "inbox repost over an unreadable epochs file exits 1, not 4, when a post is answered 408, 5xx or UnknownInbox, because the failed mark_unsent goes to settle_or_warn instead of to the run's local failure."},
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 492, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "inbox repost over a FIFO or over-bound epochs file writes 3 lines naming it, two of them 'read as holding nothing' (kept and the reconnect's due), beside the exit-4 failure the acceptance wants as one line."},
  {"file": "crates/inbox-cli/tests/unreadable_beside.rs", "line": 263, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A mutant whose inbox open reads .opened before its request passes every story case, because the open case checks no request and no opened inbox."},
  {"file": "crates/inbox-cli/tests/unreadable_beside.rs", "line": 132, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A mutant that writes the old 'read as holding nothing' warning beside the failure passes every story case, because wrong() asks for some line naming the file, not for one line."},
  {"file": "AGENTS.md", "line": 58, "category": "contract-drift", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The inbox-cli row still says only a change of the outbox file fails on a file that is no regular file or over its bound, while the same row now says changes of the epochs and opened files fail too."},
  {"file": "crates/inbox-cli/tests/unreadable_beside.rs", "line": 164, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The story's epochs case ran up to 6.92 s (over 5 s in 2 of 20 runs at load about 100) because it makes and reads back a 64 MiB + 1 file twice."}
]
```
