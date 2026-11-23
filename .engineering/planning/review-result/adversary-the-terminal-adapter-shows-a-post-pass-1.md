---
format: aep.planning-md/3
id: review-result:adversary-the-terminal-adapter-shows-a-post-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the terminal adapter: 7 findings, 3 red cases'
relations:
- reviews: story:the-terminal-adapter-shows-a-post
revision: 1
---
unit: story:the-terminal-adapter-shows-a-post round 1 (`inbox wake --follow`), tree inbox-s3-term HEAD 68aee9d (story diff 5f068b4..HEAD), attacked in the copy <scratch>/s3/term/adversary/copy
verdict: NEEDS-CHANGE
cases: executed 253→262, red 3
origin: introduced 6 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s3/term/adversary/ (the copy, logs, scripts; full list in part 6)
needs-coordinator: whether the record check should run in every round or the design sentence should be narrowed (finding 1)

Summary: 3 of my cases fail against the round. The worst failure: the adapter checks this host's record of `inbox open` only when it starts. If the inbox is then opened with `--adapter claude_code`, the adapter keeps the lock, and the Claude Code plugin's `inbox wake` exits 5. The plugin takes that silently, so the session is never woken. The design says the record check exists to prevent exactly this. The implementor's 7 cases let 6 of my mutants live; my green cases kill 5 of them. The 7 cases ran 20 times together with 0 reds; the slowest single case took 4.0 s.

## 1. Proof of what I touched
I edited the copy, not the tree. Its diff against the tree (`diff -rq`, ignoring target, .git and node_modules):
- Only in copy: crates/inbox-cli/tests/adversary_terminal.rs (new).
- Differs: crates/inbox-cli/tests/main.rs (one line `mod adversary_terminal;`, in sorted order before `adversary_tls`).
- Only in the tree (untracked there, not mine): .engineering/drafts, .out.

Both paths are test files. I made the mutants by perl edits to copy/src/wake.rs and copy/src/output.rs. After the last run both are byte-equal to the tree's HEAD (checked with cmp).

One slip: I ran `git init` in the copy once by mistake and removed its `.git` at once. I committed nothing and touched no git state of the tree.

## 2. Cases added
All in copy/crates/inbox-cli/tests/adversary_terminal.rs, formatted with `rustfmt --edition 2024`; clippy `-D warnings` passes (exit 0).

Red now (they show findings):

1. `the_adapter_does_not_keep_the_lock_once_the_inbox_is_opened_with_another_adapter` (2.4 s): `--follow` runs on a terminal record; `inbox open --adapter claude_code` runs; a note is posted; then plain `inbox wake` is run. Red at :329, verbatim:
```
assertion `left != right` failed: the plugin's wake is locked out by the terminal adapter, which showed the post: inbox wake: another wake of agent:test/bob runs on this host
 true
  left: 5
 right: 5
```
2. `a_senders_bidirectional_override_does_not_reach_the_terminal_raw` (1.1 s). Red at :297, verbatim:
```
U+202E reached the terminal raw: "01M83JTP12VSFT3KZXR1TT5Z8V note Sent from agent:test/alice to agent:test/bob thread 01M83JTP12VSFT3KZXR1TT5Z8V expires 2026-11-24T06:52:21Z\n    pay \u{202e}0001\u{202c} now \u{2066}x\u{2069} end\n"
```
3. `a_row_still_in_sent_after_a_mark_that_succeeded_is_not_read_again_at_once` (0.2 s): a stand-in answers every mark `200 delivered` and still lists the row. The case checks only a lower bound (gap of at least 100 ms), so a slow host cannot turn it red. Red at :459 (the output below is from its first run, at :446 before a refactor), verbatim:
```
reads of the same row at once, the row shown 6 times: [671.213µs, 11.565981ms, 10.644082ms, 1.670812ms, 488.589µs]
```

Green now; each kills a mutant the 7 cases let live:

4. `a_round_of_two_rows_on_a_terminal_rings_one_bell` (1.2 s, under `script`): kills "BEL once per row".
5. `json_follow_on_a_terminal_rings_no_bell` (1.4 s, under `script`): kills "BEL in --json".
6. `the_plain_wake_on_a_terminal_rings_no_bell` (1.8 s, under `script`): kills "BEL in the plain wake".
7. `a_row_whose_mark_is_refused_is_read_again_only_after_a_pause` (1.0 s, lower bound only): kills "no pause after a failed mark".
8. `a_standard_output_no_one_reads_exits_4_before_the_check_of_the_adapter` (0.8 s): kills "adapter check before the standard-output check".
9. `sigterm_during_a_held_read_leaves_no_lock_behind` (2.0 s): checks the design's claim that the kernel releases the lock.

Order of work: my first run of the cases alone had a false red in case 9. I counted the message id with `matches`, and the id also appears as the thread. I fixed the count before the suite ran.

## 3. Suite run (after the cases existed)
`cargo nextest run -p inbox-cli --locked --no-fail-fast`, exit 100:
```
FAIL ( 73/262) adversary_terminal::a_row_still_in_sent_after_a_mark_that_succeeded_is_not_read_again_at_once
FAIL ( 78/262) adversary_terminal::a_senders_bidirectional_override_does_not_reach_the_terminal_raw
FAIL ( 84/262) adversary_terminal::the_adapter_does_not_keep_the_lock_once_the_inbox_is_opened_with_another_adapter
Summary [  31.211s] 262 tests run: 259 passed, 3 failed, 0 skipped
```
The number before my cases (253) comes from the implementor's report.

20 runs of the 7 cases of terminal_adapter.rs together: 20 of 20 exit 0, 0 reds. The slowest cases: `a_message_delivered_before_the_start_is_not_shown` 4.01 s, `a_terminal_gets_one_bell_…` 3.45 s.

## 4. Findings

1. **The record is checked only at the start.**
   - Measured: case 1 red at adversary_terminal.rs:329. The adapter shows the post and marks it delivered, and the plugin's wake exits 5.
   - The cause: the check runs once, at crates/inbox-cli/src/wake.rs:139-141, before the loop of rounds.
   - Contradicted: docs/design/wake.md:446 ("So a terminal does not take the lock of a Claude Code session's plugin").
   - What reaches it: someone runs `--follow`, then follows the first-use page's `### claude_code` section (`inbox open --adapter claude_code`) with the same key and outbox. adapters/claude-code/scripts/wake.sh maps exit 5 to exit 0, so the session is never woken and nothing says why.
   - Fix lands in wake.rs: re-read the record each round, before writing the rows, and end with `OtherAdapter`. Or narrow the design sentence to the start only.
   - NEEDS-CHANGE, introduced.

2. **Bidirectional overrides reach the terminal raw.**
   - Measured: case 2 red at :297. U+202A-202E and U+2066-2069 are not escaped.
   - The cause: crates/inbox-cli/src/output.rs:148, `visible`, escapes only control characters (Cc) and U+2028/2029; these are format characters (Cf).
   - Origin: the same `visible` is in base 5f068b4 (output.rs:122), so `inbox wake` and `inbox pending` show them too. The new mode puts them on a live terminal.
   - Fix lands in `output::visible`.
   - CONFIRMED, pre-existing.

3. **A row that stays in `Sent` after a successful mark spins.**
   - Measured: case 3 red at :459. With no failed mark, no round pauses: the row was shown 6 times with gaps under 12 ms, so output grows without end.
   - The cause: wake.rs:158-163 resets the pause after every round whose marks all succeed.
   - What reaches it: only a service that answers a mark 2xx and does not keep it. Nothing found in this repository's service, so this is a state I built.
   - Fix: count a row that comes back after its mark as a failed round for the pause.
   - INFEASIBLE, introduced.

4. **status.md still lists the terminal adapter as missing.** website/docs/status.md:76 (under "Missing") names "showing a message in a terminal: the `metaharness` and `terminal` adapters". The new row 31 says it is built. The sentence was true at base. NEEDS-CHANGE, introduced.

5. **The design's "What it leaves out" is stale.** docs/design/wake.md:185 still says "No adapter but `claude_code`, which is built". The implementor named it. NEEDS-CHANGE, introduced.

6. **The first-use commands exit 2 as written.**
   - The `### terminal` section says "In a terminal of its own" (website/docs/first-use.md:254) and gives `inbox open --adapter terminal` and `inbox wake --follow` without `--secret-key-file`.
   - The page's earlier walkthrough exports only `INBOX_URL` and passes `--secret-key-file` per command.
   - Run as written, both commands exit 2: `usage: the caller's key: pass --secret-key-file <path> or set INBOX_SECRET_KEY_FILE`.
   - Fix: add the exports, as the `### claude_code` section does.
   - NEEDS-CHANGE, introduced.

7. **The 7 cases do not observe 6 behaviours, and the `script` case has weak spots.**
   - The cases in crates/inbox-cli/tests/terminal_adapter.rs let these mutants live: BEL per row, BEL in --json, BEL in the plain wake, no pause after a failed mark, adapter check before the standard-output check, pause not reset. The cause: the bell case has one row per round, and every other case writes to a pipe or a file.
   - The `script` case at :386 goes red, not skipped, on a host without `script`: `expect("the process starts")` panics.
   - With BSD `script` (no `-c`/`-e`) it fails after 10 s at "the binary runs under script".
   - It reads /proc, so it is Linux only. I reasoned these three from the code and did not run them on another host.
   - It leaves no process behind: on a panic, Drop kills `script`, and the pty hang-up ends `inbox`.
   - CONFIRMED, introduced.

## Mutants
- BEL once per row: survived the 7 cases; caught by `a_round_of_two_rows_on_a_terminal_rings_one_bell`.
- BEL before the rows: caught by `terminal_adapter::a_terminal_gets_one_bell_after_the_rows_of_each_round`.
- BEL in --json: survived the 7; caught by `json_follow_on_a_terminal_rings_no_bell`.
- BEL in the plain wake (`try_print_text` with the bell): survived the 7; caught by `the_plain_wake_on_a_terminal_rings_no_bell`.
- Pause not reset (`failed_rounds = 0` dropped): survived every case, mine included. Killing it needs an upper time bound, which the rules forbid.
- No pause after a failed mark: survived the 7; caught by `a_row_whose_mark_is_refused_is_read_again_only_after_a_pause`.
- Adapter check after the lock: caught by `terminal_adapter::an_inbox_opened_with_another_adapter_…`.
- A missing record taken for another adapter: caught by `terminal_adapter::an_inbox_opened_with_another_adapter_…` and `each_round_writes_its_rows_before_its_marks_…`.
- Adapter check before the standard-output check (exit 1 instead of 4): survived the 7; caught by `a_standard_output_no_one_reads_exits_4_before_the_check_of_the_adapter`.

## 5. Attacked and not broken
- Controls from a sender (BEL, ESC, CSI, OSC, CR, BS, DEL, C1, U+2028/2029) and forged head lines: all escaped or indented. I read this from `visible` and `rows_text`; the implementor's case covers BEL and line ends.
- The failed-mark line on standard error goes through `visible`. It keeps `\n` raw, but its text is the message id and the service's own failure text; I found no text a sender sets in it.
- The plain wake is byte for byte unchanged: same `try_print_text` bytes, the record check runs only with the flag, and the exit is mapped as before.
- Rows are written and flushed before the marks in every round: covered by `each_round_…` (marks[1].stdout).
- A round with no row: `held` returns only when there are rows, so it never rings.
- No set of ids grows over a long run.
- SIGTERM during a held read leaves no lock behind (case 9).
- A record that is not JSON, or a duplicate record of the caller, is read as no record, so the mode goes on as the wake does. I read this from `Records::parse` and did not run it.
- generated/cli/inbox.json matches the doc comment in main.rs.

Not reached:
- SIGHUP, or a hung-up terminal under nohup.
- A reader of the pipe that goes away in a later round.
- `inbox close` or a service restart while the mode waits.
- The plugin hook started for real against a running `--follow`.
- An unknown adapter name in the record.
- A record of another principal in the same file. I did not run it; the plugin's grep matches `"adapter":"claude_code"` in any principal's record, which is plugin code from before this unit.
- The first-use page end to end against `inbox serve`: I ran only the 2 terminal commands without the key variable.

## 6. Paths written outside the tree
All under <scratch>/s3/term/adversary/:
- copy/ (the copy; its target/ is deleted)
- build.log, red-1.log, red-2.log, run-3.log, suite.log, clippy.log
- mutants.sh, mutants.log, mutants2.sh, mutants2.log, mutants/*.log (11 files)
- repeat.sh, repeat.log, repeat-1.log … repeat-20.log

I created and deleted backup/ and docprobe/ in the same place. The coordinator moves copy/crates/inbox-cli/tests/adversary_terminal.rs and the mod line in copy/crates/inbox-cli/tests/main.rs into the tree.

```findings
[
  {"file": "crates/inbox-cli/src/wake.rs", "line": 140, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The adapter checks this host's record only at its start, so after `inbox open --adapter claude_code` it keeps the wake lock, shows and marks the posts, and the plugin's wake exits 5 silently, which docs/design/wake.md:446 says the check prevents."},
  {"file": "crates/inbox-cli/src/output.rs", "line": 148, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "visible leaves the bidirectional overrides and isolates U+202A-202E and U+2066-2069 raw, so a sender's text reorders what the owner's terminal shows."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 162, "category": "property", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A round whose marks all succeed resets the pause, so a row a service keeps listing in Sent after a 2xx mark is shown and marked again with no pause, without end."},
  {"file": "website/docs/status.md", "line": 76, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The Missing list still names the terminal adapter while the new row 31 of the same page says it is built."},
  {"file": "docs/design/wake.md", "line": 185, "category": "contract-drift", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "What it leaves out still says no adapter but claude_code is built."},
  {"file": "website/docs/first-use.md", "line": 254, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The terminal section's two commands run in a terminal of its own without INBOX_SECRET_KEY_FILE or --secret-key-file, and as written both exit 2 with a usage error."},
  {"file": "crates/inbox-cli/tests/terminal_adapter.rs", "line": 382, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The 7 cases let 6 mutants live (BEL per row, BEL in --json, BEL in the plain wake, no pause after a failed mark, adapter check before the stdout check, pause not reset), and the script case panics instead of skipping on a host without util-linux script."}
]
```
