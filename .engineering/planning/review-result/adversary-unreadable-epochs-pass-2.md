---
format: aep.planning-md/3
id: review-result:adversary-unreadable-epochs-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the unreadable epochs file: 3 notes, 2 red cases'
relations:
- reviews: story:an-unreadable-epochs-file-keeps-its-other-lines
revision: 1
---
unit: story:an-unreadable-epochs-file-keeps-its-other-lines, tree at 9df65f6 (branch backlog/epochs-file-unreadable)
verdict: NEEDS-CHANGE (2 notes, no blocker; the round 2 claims hold)
cases: 5 added, 2 red. Runs: story + pass 1 + mine, 13 + 5 cases
wrote-outside-worktree: only <scratch>/bl/epochs/adversary2/
needs-coordinator: move <scratch>/bl/epochs/adversary2/copy/crates/inbox-cli/tests/adversary_unreadable_beside_2.rs and its line `mod adversary_unreadable_beside_2;` (after `mod adversary_unreadable_beside;` in tests/main.rs) into the tree.

I wrote nothing in the tree. `git status` there shows one untracked directory, `.engineering/evidence/story/an-unreadable-epochs-file-keeps-its-other-lines/`. It is not mine. Both source files were put back after the mutants (`cmp` confirmed it), and `copy/target` is deleted. The case file is formatted with `rustfmt --edition 2024`, and `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0.

Cases in adversary_unreadable_beside_2.rs (time when green):
- `a_repost_over_an_unreadable_epochs_file_exits_by_the_answer_as_the_table_says`: GREEN, 0.9 to 2.9 s. 6 answers × 3 epochs states, 18 rows. The answers: 408, 500, 503, UnknownInbox, 413, and a final declared refusal (409 Expired). The states: empty readable, not UTF-8, FIFO.
  - Readable: every row exits 1, and no line names the epochs file.
  - Unreadable: 408, 5xx, UnknownInbox and 413 exit 4, with exactly 1 line naming the file. The outbox and epochs files are unchanged.
  - Final refusal over an unreadable file: exits 1 and drops the envelope. Not UTF-8 gives 0 lines; a FIFO gives the 0.6.0 warning "read as holding nothing", once.
  - No envelope was lost and none was reported posted when it was not.
- `a_repost_ended_by_a_transport_failure_exits_3_and_names_the_local_failure_once`: GREEN, 0.7 to 2.3 s. The transport failure comes after 0, 1 and 2 posts, in the same 3 states.
  - Every row exits 3.
  - Unreadable: exactly 1 line names the file, and the outbox and epochs files are unchanged.
  - Readable: 0 lines name the file.
- `two_unreadable_files_beside_one_outbox_are_each_named_once`: GREEN, 0.5 to 1.4 s. `.opened` and `.epochs` are both FIFOs.
  - `inbox open`: exit 4; 1 failure line for `.opened`; the reconnect writes 1 warning for `.epochs`.
  - `inbox repost`: exit 4; 1 failure line for `.epochs`; the reconnect writes 1 warning for `.opened`.
  - Neither file's line was dropped because of the other.
- `a_link_at_the_staged_epochs_file_reaches_the_refused_post_path_without_a_race`: RED (finding 1).
- `a_failed_repost_leaves_the_outbox_file_as_it_was_as_the_design_says`: RED (finding 2).

Findings

1. commands.rs:705. The path round 2 called "reachable only by a race" can be reached without one.
   - What I did: planted a link `alice.outbox.epochs.new -> alice.outbox.epochs`, then ran `inbox post` against a stand-in that answers 413.
   - What happens: `keep` reads the readable file. `replace()` writes through the link into the epochs file, then renames the link over it, so the epochs path becomes a link to itself. The 413 then asks `clear_unsent`, which reads ELOOP.
   - Red line, verbatim: `status: 1, stderr: "inbox post: …/alice.outbox.epochs: Too many levels of symbolic links (os error 40)\ninbox: refused: too large (413)\n"  left: 1  right: 4`
   - Side effects: the regular epochs file is unlinked, so every other entry is lost. That loss is pre-existing: 0.6.0 has the same `replace()`, which follows a link at `<path>.new`. After this run, every change exits 4 until the link is removed.
   - Fix lands in one of 2 places. Option 1: make `replace` open the staged file with `create_new` / `O_NOFOLLOW` after removing a stale one (outbox.rs:629). That removes the trigger and the data loss. Option 2: send the failed `clear_unsent` of `post` into the exit, as `repost` now does.
   - Severity note: it needs a link planted in the outbox's 0700 directory.
2. docs/design/service.md:142 and CHANGELOG.md:16 say a command that fails on an unreadable epochs file "changes neither that file nor the outbox file" ("leaves the file and the outbox file as they are").
   - What I did: `inbox repost` over a not-UTF-8 file, 2 kept envelopes. The first arrives (its record fails); the second is refused for good.
   - Result: exit 4, and the outbox file lost the second envelope.
   - Red: the outbox file holds 1 line where 2 were expected.
   - The code is right: a final refusal drops the envelope, and only the outbox file changes. The same happens when `kept` drops expired envelopes before a failing change.
   - Fix lands in the 2 documents: name the final refusal's drop (and the expiry) as the exception.

Claims checked and holding:
- Exit 4 for 408, 5xx and UnknownInbox; exit 3 for a transport failure, with exactly 1 line for a local failure before it.
- 1 line per failed file; read-only `inbox outbox` writes 2 warnings, as in 0.6.0 (the story's case passes).
- The over-bound file is really sparse here: ext4, 0 blocks.

What I found by reading the code (attack 1): only `kept()` holds a warning back. Its only callers are `outbox` and `repost_at`, and both call `speak()` on every path, including the `?` of `kept` and of `client.epoch()`. The reconnect inside other commands never reaches `kept` over an unreadable epochs file, because `due` reads it as empty, so nothing is due. `record_epoch` and `mark_unsent` hold a warning only when the envelope left the outbox between reads (an expiry or another process), and `repost_at` speaks it at the end. `failed` is keyed by `beside(self.path)`, the same spelling as every read in the run, so a second spelling cannot occur inside one run.

Order of lines against 0.6.0 (attack 3): in `repost`, the held warning now comes after the posts. It still comes before the report lines. Only the "inbox repost: <local>" line of a transport-ended run changes places, and only when that local failure is on the outbox file. I did not count that as a finding. No warning is printed twice; `speak` drains the list.

Mutants (each run against the story's cases, the epoch.rs cases, pass 1's 5 and my 5; 29 cases):
- m1, 1 byte written at offset 64 MiB into the over-bound epochs file (brief item 6): killed by the story's post, repost and read-only cases, through the blocks comparison.
- m2, `speak` ignores `failed`: killed by the story's repost case, pass 1's one-line case and 3 of mine.
- m3, the "local failure before transport" line dropped: SURVIVED the story and pass 1. Killed only by my transport case.
- m4, `speak` only when the run had a local failure (the held warning lost on a refusal-only run): SURVIVED the story and pass 1. Killed only by my table case (FIFO + final refusal).
- m5, `read_or_nothing` ignores `failed`: killed by the story's repost and open cases, pass 1's case and my two-files case.
- m6, a failed `mark_unsent` ignored on a transport failure: SURVIVED the story and pass 1. Killed only by my transport case.

Brief item 6 asked for 3 mutants of my own; I ran 5 (m2 to m6), and 3 of them survive the story's cases. Round 2's claim about the transport line has no case in the story.

Repeat: the 13 cases of the story and pass 1 together, 10 times, with a script that does not take the slot prefix.
- 0 red.
- Suite wall time 1.5 to 4.2 s.
- Slowest case: `unreadable_beside::a_close_over_an_unreadable_file_of_the_opened_inbox_fails_with_4_and_leaves_it_as_it_was`, 2.92 s, at a 1-minute load average of about 80 to 84.

Not checked:
- `wake` and `wait` over an unreadable `.opened`.
- UTF-8 BOM and CRLF in the epochs file.
- A file that passes its bound between `keep` and the answer (by reading: `keep` rewrites the file as a regular UTF-8 file of mode 0600, far below the bound, so only another process can get it there).
- Path spellings across 2 outboxes in one run (impossible: one `Outbox` per run).
- My stand-in in the transport case relies on the listener closing before the next connect. That held in every run, but it is a timing assumption.

```findings
[
  {"file": "crates/inbox-cli/src/commands.rs", "line": 705, "category": "acceptance", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A link planted at <outbox>.epochs.new makes replace() turn the epochs path into a self-link during keep, so the post answered 413 reaches the clear_unsent failure without a race and exits 1 instead of 4; the regular epochs file and its other entries are unlinked by the pre-existing link-following replace() (outbox.rs:629)."},
  {"file": "docs/design/service.md", "line": 142, "category": "contract-drift", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The design and CHANGELOG.md:16 say a command failing on an unreadable epochs file leaves the outbox file as it is, but inbox repost drops an envelope refused for good (and kept drops expired ones) in the same run that exits 4."},
  {"file": "crates/inbox-cli/tests/unreadable_beside.rs", "line": 186, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Three mutants of round 2 survive the story's and pass 1's cases (the transport-branch local line dropped, speak skipped on a run without a local failure, a failed mark_unsent ignored on a transport failure); only adversary_unreadable_beside_2.rs kills them."}
]
```
