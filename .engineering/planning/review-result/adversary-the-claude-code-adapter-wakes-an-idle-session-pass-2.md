---
format: aep.planning-md/3
id: review-result:adversary-the-claude-code-adapter-wakes-an-idle-session-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the claude_code adapter: 2 defects of the hook, 2 gaps of the cases'
relations:
- reviews: story:the-claude-code-adapter-wakes-an-idle-session
revision: 1
---
unit: story:the-claude-code-adapter-wakes-an-idle-session round 2, commit 7da0174 (archive copy at <scratch>/s3/cc/adversary2/copy)
verdict: NEEDS-CHANGE
cases: executed 352→357, red 2 (and 1 more red in the copy only: `the_plugin_declares_an_async_rewake_stop_hook_that_runs_the_script` reads `git ls-files -s`, and the copy has no git)
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the scratch directory, part 6)
needs-coordinator: none

Two defects in the hook, both small:
- **F1:** the design's 8,000-byte bound on what the hook writes to standard error does not hold on this host for addresses with non-ASCII characters.
- **F2:** an `inbox` that has no `wake` command gets a wrong reason.

I could not break the printer's one-unindented-line rule or forge a plugin line at column 0.

**1. diff --stat** (the copy has no git; compared by hand against `git archive 7da0174`)
- New: `crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_2.rs` (206 lines).
- New: `crates/inbox-cli/tests/adversary_printer.rs` (96 lines).
- `crates/inbox-xtask/tests/tools/mod.rs`: +1 line, `mod adversary_claude_code_adapter_2;`.
- `crates/inbox-cli/tests/main.rs`: +1 line, `mod adversary_printer;`.
- All 4 are test files; I wrote nothing in the tree. After the mutant runs, the copy's `wake.sh` is byte-equal to `7da0174:adapters/claude-code/scripts/wake.sh` (cmp).

**2. Cases added**
All are under 0.4 s and none sleeps. The first run of each was alone, before the suite (logs `red-alone-1.log`, `red-alone-2.log`).

In `adversary_claude_code_adapter_2.rs`:
- `head_lines_of_wide_characters_keep_the_frame_within_8000_bytes`: RED.
  - Setup: 25 rows, each sender address 400 × U+00E4 (2 bytes), hook run with `LC_ALL=C.UTF-8`.
  - Red line: `the frame is 12513 bytes, over the 8,000 bytes of docs/design/wake.md; its longest line is 827 bytes`.
- `an_inbox_without_the_wake_command_is_not_reported_as_a_wrong_url`: RED.
  - Setup: the stand-in prints clap's `error: unrecognized subcommand 'wake'` and exits 2.
  - Red line: `an `inbox` without `wake` is reported as a wrong URL: inbox plugin: `inbox wake` exited 2: wrong usage: INBOX_URL is not a loopback http:// or https:// URL, or the pin does not go with it. …`
- `five_names_taken_are_passed_over`: green. It kills mutant X2.
- `rows_of_6001_bytes_are_handed_on_as_head_lines`: green. It kills mutant X5.

In `adversary_printer.rs`:
- `a_decision_of_any_text_is_one_unindented_line_in_pending_and_through_the_hook`: green.
- What it runs: the real `inbox serve`, keys made at run time, then `inbox pending` and the real `wake.sh` with the built `inbox` on `PATH`.
- The input: one decision whose body and 8 option texts hold `\n`, `\r\n`, NEL, VT, FF, U+2028, U+2029, a tab at the start of a line, a text of only line ends, and a text that ends with a line end, each before a forged head line.
- What it asserts: one unindented line in `pending`; in the hook, `1 message(s) follow` and one unindented line between the begin and end lines.

**3. Suite run** (after the cases existed)
- Command: `SLOT_TIMEOUT=20m cargo nextest run -p inbox-xtask -p inbox-cli --locked --no-fail-fast` in the copy.
- Result: `Summary [ 39.334s] 357 tests run: 354 passed, 3 failed, 0 skipped`, exit 100.
- The 3 failures: my 2 red cases, and the git case (no git in the copy).
- Split: inbox-cli 246 (245 + 1), inbox-xtask 111 (107 + 4). The 245 and 107 before my cases match `docs/checks.md`.
- Slowest: `drift::an_optional_field_added…` 12.2 s, `test_step::a_run_over_the_bound…` 8.2 s; none of them is mine.
- `cargo clippy -p inbox-xtask -p inbox-cli --all-targets --locked -- -D warnings`: exit 0. Both files formatted with `rustfmt --edition 2024`.

**4. Findings** (all cover 7da0174)

F1. The size bound is false in a UTF-8 locale (`docs/design/wake.md:279`, `wake.sh:96`). NEEDS-CHANGE, note, introduced.
- Measured: the design says "GNU `cut -c` counts bytes, so there it is also at most 8,000 bytes". GNU coreutils 9.12 (this host) counts characters in a UTF-8 locale: `printf 'a\303\244bc' | cut -c 1-2` gives `aä`.
  - The frame is 12,513 bytes for 2-byte characters.
  - With 4-byte characters (U+1F600 cut whole, checked), the 20 head lines alone are about 11,400 UTF-16 code units. That is over the 10,000-character cap if Claude Code counts JS string length (not verified).
  - Counted in code points, the bound of 8,000 still holds.
  - The round's own size case runs with `env_clear()` (C locale) and only ASCII, so it cannot see this.
- What reaches it: the hook inherits the terminal's UTF-8 locale. It also needs a sender principal (`from` or `on_behalf_of`) of more than about 150 non-ASCII characters. Principals are named by the operator in the key file, with no bound and no alphabet. Not checked: the forge's coreutils, which may count bytes and keep the case green there.
- Fix: `cut -b 1-300` (POSIX; bytes everywhere; UTF-16 units ≤ bytes). Mutant X12 applies it: my case goes green and every other case stays as it was.

F2. An `inbox` without `wake` is reported as a wrong `INBOX_URL` (`wake.sh:113`). CONFIRMED, note, introduced.
- Measured: the case above. The released 0.4.0 has no `wake` (it came in 4467db3, after the `dcad381` release). clap exits 2 (`main.rs:763`), and the hook names a wrong URL or pin.
- What reaches it: an operator with the release on `PATH` who loads the plugin from a checkout. The website's own command is `claude --plugin-dir /path/to/inbox/adapters/claude-code`.
- Fix: in the `2)` text of `wake.sh` (and the design's table row), also name "or this `inbox` has no `wake` command (older than the plugin)".

F3. No plugin case ran the hook against the real printer, and the pass-1 cases changed in round 2 pass by fixture (`adversary_claude_code_adapter.rs:212`, `:250`; `claude_code_adapter.rs:428`). CONFIRMED, note, introduced.
- The stand-in rows are written by hand "as the fixed printer writes them". A printer that puts sender text back at column 0 leaves all 27 plugin cases green.
- `a_decision_option_cannot_forge_the_plugins_note_line` now passes by the random mark alone. A column-0 copy of the note also passes, because it carries the other run's mark and the forged run has no note line, so the count is 2 against 3.
- `one_message_reads_as_one_message_to_the_session` holds an indented stand-in. Its `headers` filter uses one space.
- The gap is closed by `adversary_printer.rs` (green); keep that case.

F4. Two boundary mutants of the round survive its cases (`claude_code_adapter.rs:502`, `:452`). CONFIRMED, note, introduced.
- The bound of 100 tries changed to 3 (X2) survives: only 2 names are taken in the case.
- `-gt 6000` changed to `-gt 6001` (X5) survives.
- My 2 green cases catch both.

**Mutants** (against the 27 plugin cases plus mine; logs `mutant-*.log`, summary `mutants.out`)
- B1, count with `'^ '`: survived. It is equivalent: the printer writes no line that starts with exactly 1 space unless the service sends a row with an empty `message_id` and `kind`.
- B2, size test `-ge`: caught by `rows_over_their_bound_…` (its 6,000-byte part).
- B3, 20 changed to 21: caught by `rows_over_their_bound_…`.
- B4, end line before the rows: caught by 8 cases.
- B5, note without the mark: caught by `a_mark_that_failed_…`, the size case and `a_decision_option_cannot_forge_…`.
- B6, mark from 4 bytes (`-N 4`, length 8): caught by 7 cases.
- X2, tries bound 3: survived the round's cases; caught by `five_names_taken_are_passed_over`.
- X5, `-gt 6001`: survived the round's cases; caught by `rows_of_6001_bytes_…`.
- X12, `cut -b`: this is the fix of F1; only F1's case changes (it goes green).
- X13, count taken after `sed 1,20p`: caught by the size case.
- Printer mutants, judged from the cases and not run:
  - `inline` dropped for `on_behalf_of` or the reply's `from` alone: caught by `a_line_end_in_any_member_…` (it loops over both).
  - `first_and_rest` indenting by 2: caught by the exact `contains` and `ends_with` in the option and refusal cases.
  - U+2029 dropped from `visible`: caught by the exact assert of `a_unicode_line_…`.
  - The final `trim_end` dropped: caught by the `ends_with` of the refusal case.

**5. Attacked and not broken**
- **Printer:** every line end (`\n`, `\r`, `\r\n`, NEL, VT, FF, U+2028, U+2029), a leading tab, text that is only line ends, and a trailing line end stay indented or escaped, through the binary.
- **Members that are not text** (absent, a number, an object, an option list that is not a list, a row that is not an object) print as empty and never break a line. `in_sent` drops a row that is not an object.
- **Column 0:** a sender cannot put text there. Every head line starts with the service's `message_id`, and `trim_end` drops only trailing white space.
- **Other printers:** `read`, `refuse` and `answer` print an outcome and ids. The reconnect line and the error lines go to the wake's standard error, which the hook throws away.
- **Frame:** no line of the plugin can be forged. At worst a sender shows an indented forged head line or an indented end line of an old mark, and the begin line names the rule. I raise no judgement finding on it.
- **Mark:** it is made before the wake in every path. It is in no argument of a child (`printf` is a builtin of dash, bash, busybox ash and zsh), not in the environment, and not on standard output.
  - `od` printing upper-case or an offset, `od` missing, or `/dev/urandom` failing gives exit 1 with no wake.
- **Directory:**
  - TMPDIR not writable or missing: 100 tries, then exit 1.
  - A file, a dangling link or another user's directory under the name is passed over.
  - The EXIT trap is set only after a successful `mkdir`, so `rm -rf` never sees an unset `dir`.
  - A signal before the trap leaves the directory, which the design covers with "unless it is killed".
- **Exit-status table:** every row matches the script. With the real `inbox wake`, exit 2 without rows is not reachable: it prints only when it has at least one row in `Sent`, and a head line is never white space alone.
- **Exit-4 partial write:** a write cut inside a character makes GNU grep print `binary file matches` inside the frame in the size branch. Not reachable by a sender; I raise no finding.
- **Documents:** the cited statements about the harness are on hooks.md (lines 425, 456, 821, 904, 1095, 4210, 4230, 4287). `first-use.md` names no story id, stage, wave or test count. "What the commands print" is true of `pending` and the hook (my case). The counts in `docs/checks.md` match.
- **POSIX:** read only, not run (no dash on this host, none fetched). `until`, `${#mark}`, `*[!0-9a-f]*`, `$(( ))`, `grep -c -v`, `sed -n`, `cut -c`, `od -A n -N 8 -t x1`, `command -v`, `kill -0`/`-TERM`, and the `trap` with `wait` are all POSIX or XSI, and dash runs them.

Not checked:
- dash and busybox (not on this host).
- The forge's coreutils `cut -c` behaviour.
- Whether Claude Code counts its cap in UTF-16 units.
- The git-mode case.

**6. Paths written outside the worktree**
- `<scratch>/s3/cc/adversary2/`:
  - `copy/` (the archive plus my 4 test-file changes; `copy/target` deleted);
  - `mutate.sh`, `wake.sh.orig`, `probe-rows`;
  - the logs `build1.log`, `red-alone-1.log`, `red-alone-2.log`, `suite.log`, `clippy.log`, `mutants.out` and `mutant-*.log` (10);
  - `list-cli.txt` and `list-xtask.txt` (empty, a failed grep).

```findings
[
{"file":"docs/design/wake.md","line":279,"category":"boundary","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"GNU coreutils 9.12 cut -c counts characters in a UTF-8 locale, so 20 head lines of multi-byte addresses make the frame 12,513 bytes against the stated 8,000, and cut -b 1-300 restores the bound"},
{"file":"adapters/claude-code/scripts/wake.sh","line":113,"category":"acceptance","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"an inbox on PATH without the wake command, such as the released 0.4.0, exits 2 and the hook tells the operator INBOX_URL is not a loopback URL or the pin does not fit"},
{"file":"crates/inbox-xtask/tests/tools/adversary_claude_code_adapter.rs","line":212,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"no plugin case ran the hook against the real printer, and the pass-1 forgery cases changed in round 2 now pass by the random mark or by hand-written rows whatever the printer does"},
{"file":"crates/inbox-xtask/tests/tools/claude_code_adapter.rs","line":502,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the mutants tries bound 100 to 3 and size test -gt 6000 to -gt 6001 survive every case of the round"}
]
```
