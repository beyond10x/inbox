---
format: aep.planning-md/3
id: review-result:adversary-the-terminal-adapter-shows-a-post-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the terminal adapter: 4 findings, no red case against the product'
relations:
- reviews: story:the-terminal-adapter-shows-a-post
revision: 1
---
unit: story:the-terminal-adapter-shows-a-post round 2, commit 56168b6 (branch stage-3/terminal-adapter), attacked in a copy made by `git archive HEAD`
verdict: NEEDS-CHANGE (one warning on the docs, one warning on a pass 1 case; no red case against the product)
cases: executed 266→266, red 0 (one probe case ran alone and was red, then taken out; see part 2)
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s3/term/adversary2/ (files listed in part 6)
needs-coordinator: whoever owns adversary_terminal.rs needs to make the change in F1. F2 is half covered by your uncommitted change to first-use.md in the tree; line 117 is still open.

1. Diff stat of the worktree
I wrote nothing in the tree. `git diff --stat` there shows 6 files that I did not change and that are not committed: AGENTS.md, CHANGELOG.md, README.md, crates/inbox-cli/tests/adversary_terminal.rs, docs/checks.md, website/docs/first-use.md. There is also one new file, .engineering/planning/story/a-character-a-reader-cannot-see-is-shown-as-its-escape.md. All of these are another session's work. After the last mutant my copy was compared with the tree's `crates/` and matches it, except for your uncommitted edit to adversary_terminal.rs. I did not leave adversary_terminal_2.rs: no case stayed red, and no case killed a mutant the existing cases let live.

2. Cases
- I wrote a probe case, `probe_the_pass_1_lock_case_with_the_adapter_held_back`. It ran alone and was red. It is not kept; a copy is at adversary2/probe_case.rs.txt.
- It runs the steps of the pass 1 case `the_adapter_does_not_keep_the_lock_once_the_inbox_is_opened_with_another_adapter`, with one change: the adapter gets SIGSTOP from the post until the plain wake ends. That stands in for the scheduler of a loaded host holding the adapter back. The product is unchanged and the assertion is that case's.
- Red output, verbatim (1.74 s):
```
panicked at crates/inbox-cli/tests/adversary_terminal_2.rs:78:5:
assertion `left != right` failed: the plain wake met the lock: inbox wake: another wake of agent:test/bob runs on this host
 / adapter Some(1) inbox wake: agent:test/bob was opened on this host with the adapter claude_code, and `inbox wake --follow` is the adapter terminal: run `inbox open --adapter terminal` to show its messages in this terminal
  left: 5
  right: 5
```
- After SIGCONT the product does what it should: the adapter ends with 1 and its line, and the second post stays `Sent`. The red comes only from which process the scheduler runs first.

3. Suite runs
I added no case, so there was no suite run after one. The base count of 266 is from the implementor's report. All mutant runs used `cargo nextest run -p inbox-cli --locked --no-fail-fast -E <filter>`. On the restored copy the selected cases were green each time.

4. Findings

F1. crates/inbox-cli/tests/adversary_terminal.rs:328. Concurrency, warning, CONFIRMED, introduced.
- What was measured:
  - The pass 1 case can go red with a correct product (the probe in part 2).
  - Under mutants M1 and M7 the case does not fail. It hangs until nextest kills it at 60 s: `TIMEOUT [60.006s] adversary_terminal::the_adapter_does_not_keep_the_lock_...`. The reason: `world.client("bob", &["wake"])` has no bound, and once the second post is `Delivered` it waits for ever.
- What reaches it: a slow host. This host runs at a load average of about 80, and nothing orders the adapter's exit before the plain wake reaches its lock.
- The fix, in the case: end the adapter first and assert status 1 (`adapter.end()`). Then run the plain wake as `Running::spawn(&mut world.client_command("bob", &["wake"])).end()`, which is bounded by HANG, and assert exit 0 and that it shows `second`. That is the order of terminal_adapter.rs:322-339.

F2. website/docs/first-use.md:117 (the list at :107-111). Contract drift, warning, NEEDS-CHANGE, introduced.
- At 56168b6, "What the commands print" names only the control characters and U+2028/U+2029 as escaped.
- It says "Text without control characters is printed as it was written". That is now false: U+202E and the other bidirectional controls are format characters, not control characters, and `visible` escapes them (output.rs:149-166).
- Your uncommitted change in the tree adds them to the list. Line 117 is unchanged and still contradicts it.
- The fix is in the doc: "Text without these characters is printed as it was written".

F3. docs/design/wake.md:449. Judgement, note, CONFIRMED, pre-existing.
- A sender who writes the 8 characters `\u{202e}` gets the same bytes on the terminal as a real U+202E shown as its escape. `escaped` (output.rs:191) does not escape the backslash; I read this in the code and did not run a case. At base 349bf9f the same holds for `\u{1b}` and `\u{7}`.
- Neither the design nor first-use.md says that a reader cannot tell them apart. Only the planning story the-client-prints-no-control-sequence-of-another-principal.md:72 says it.
- The fix is one sentence in `## terminal` and in "What the commands print". Escaping the backslash would change text that has none of these characters, so the code should stay as it is.

F4. crates/inbox-cli/src/wake.rs:237. Mutant, note, INFEASIBLE, introduced.
- Mutant M3: ids of failed marks are also kept in `marked`. It survived: `54 tests run: 54 passed`.
- Its only effect is that the pause does not reset after a failed mark is followed by a mark that succeeds. Under the mutant every gap is at least as long as without it, so no lower bound on time can see it, and the brief rules out an upper bound. The design already says this at wake.md:477-478.

5. Attacked and not broken
- A torn or empty read of `<outbox>.opened`: the file is replaced by write and rename (outbox.rs:569), so a read sees the whole old file or the whole new one. A failed read counts as no record and the mode goes on; it never ends with 1 on a record of `terminal` or no record.
- `inbox open --adapter claude_code` during a held read: it wakes the wait (service.rs:261, any command), finds no row and waits again, as the design says. A row that ended a read answered before the open is also not shown and stays `Sent`. Two rows in one round: one check covers both. With `--json --follow` the end is the error document on a line of its own, with no half round and no bell.
- The reconnect after a restart opens with the adapter of the record and leaves a newer record alone (`Recorded::Other`). `inbox wake` without the flag never calls `terminal_only`; mutant M8 is caught.
- The pause for a row that comes back: a sender cannot make it grow on a healthy service. A repost of a delivered id answers Duplicate, another sender's id answers MessageIdTaken, and alternating posts have different ids. `marked` holds only ids whose mark succeeded, and a row that comes back is still shown and marked, as the design says.
- `visible`: text without these characters comes back as `Cow::Borrowed`, so output that a program parses is unchanged. Every write to standard output or standard error goes through `visible` or `plain_document`; no source file in src/ writes with `println!` or `print!`.
- first-use.md `### terminal`, run as written against `inbox serve` on 127.0.0.1 under `script`: the row was shown, followed by one `\a`, and pending showed `Delivered`.
- The changed sentences of wake.md `## terminal`, AGENTS.md, status.md, checks.md, the `--follow` help in main.rs and generated/cli/inbox.json all match the code.

Mutants
- M1, the round check after `deliver`: caught by terminal_adapter::a_record_of_another_adapter_ends_the_adapter_at_its_next_round_without_a_row. The pass 1 case timed out at 60 s.
- M2, the check only when a row came back: caught by the same case and by the pass 1 case.
- M3, ids of failed marks kept in `marked`: survived (F4).
- M7, the round check before the held read: caught by the round 2 case. The pass 1 case timed out at 60 s.
- M8, the round check also without the flag: caught by the round 2 case and by terminal_adapter::an_inbox_opened_with_another_adapter_ends_the_terminal_adapter_with_1_and_no_request.
- M4a (202a→202b), M4b (202e→202d), M4c (2066→2067), M4d (2069→2068): all caught by output::tests::every_bidirectional_control_is_shown_escaped. M4b, M4c and M4d are also caught by adversary_terminal::a_senders_bidirectional_override_does_not_reach_the_terminal_raw.

Not checked
- The workspace `task test` and clippy: I added no file.
- Rename and replace races of the record under real concurrent writers (I only read the code).
- `--json` output under a reconnect in the round that ends with 1.

6. Paths written outside the worktree
All are in <scratch>/s3/term/adversary2/:
- copy/ (its target/ is deleted)
- probe.log and probe_case.rs.txt
- mutants.sh, mutants.log and mutants.out
- 9 mutant logs: mutant-m1-check-after-deliver.log, mutant-m2-check-only-again.log, mutant-m3-failed-ids-marked.log, mutant-m7-check-before-read.log, mutant-m8-check-without-flag.log, mutant-m4a-202a.log, mutant-m4b-202e.log, mutant-m4c-2066.log, mutant-m4d-2069.log
- wake.rs.orig and output.rs.orig
- firstuse.sh and firstuse/ (keys, keys.json, serve.out and serve.err, follow.sh, follow.out and follow.err, post.out and post.err, pending.out, state/)

```findings
[
  {"file": "crates/inbox-cli/tests/adversary_terminal.rs", "line": 328, "category": "concurrency", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The pass 1 lock case runs the plain wake right after the post, with no bound and no wait for the adapter to end, so it exits 5 with a correct product when the adapter is scheduled late, and it hangs to the 60 s kill instead of failing under mutants M1 and M7."},
  {"file": "website/docs/first-use.md", "line": 117, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "What the commands print says text without control characters is printed as it was written, but visible now escapes the bidirectional controls, which are format characters, and the list at 107-111 of the commit does not name them."},
  {"file": "docs/design/wake.md", "line": 449, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A sender's literal text \\u{202e} prints the same bytes as an escaped U+202E because the backslash is not escaped, and neither the design nor first-use.md says a reader cannot tell them apart."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 237, "category": "mutant", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The mutant that keeps the ids of failed marks in marked survives, because it only stops the pause from resetting and only an upper bound on time could see that."}
]
```
