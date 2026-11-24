---
format: aep.planning-md/3
id: review-result:adversary-variable-not-utf-8-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the variable that is not UTF-8 (round 1)
relations:
- reviews: story:a-variable-that-is-not-utf-8-is-a-usage-error
revision: 1
---
ADVERSARY PASS 1

```
unit: story:a-variable-that-is-not-utf-8-is-a-usage-error, round 1, commit b88cb667 (base a1881d9), the adversary tree plus 1 new case file
verdict: CONFIRMED (one red case, pre-existing, a note; 3 mutants that the story's cases miss and the new cases kill)
cases: executed 476→486, red 1
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 27 files in the scratch directory (logs and 2 wait scripts, part 6)
needs-coordinator: none
```

I found no defect in the story's behaviour. The only red case is a pre-existing `--json` quirk (finding F1). The other 3 findings are mutants that only the new cases catch. 0 pre-existing findings are in the private file, and I did not write that file.

## 1. Diff
```
 crates/inbox-cli/tests/main.rs | 1 +
 1 file changed, 1 insertion(+)
?? crates/inbox-cli/tests/adversary_variable_not_utf_8.rs   (new, uncommitted)
```
Only test files changed. All 11 mutants were edited back by hand. `git diff b88cb667 -- crates/inbox-cli/src` is empty.

## 2. Cases added (`crates/inbox-cli/tests/adversary_variable_not_utf_8.rs`, 10 cases, `mod` line sorted in `tests/main.rs`)
Each run of the binary is killed after 10 s, so a `serve` that wrongly listens fails fast instead of at the runner's 60 s.

- `every_command_refuses_exactly_the_variables_it_reads_as_text` (green, 0.83 s): 21 command lines × 6 variables set to `\xFF`, each compared with the same line without the variable. Commands that read the variable must exit 2 with exactly the naming line; every other run must match the run without it byte for byte. Kills M1, M2, M4, M8.
- `help_and_version_print_beside_every_bad_variable` (green, 0.28 s): `--help`, `--version`, `help open`, `help serve`, `open --help`, `serve --help`, `wake --follow --help`, `bridge --help` × 6 variables: exit 0, text on standard output, nothing on standard error.
- `a_flag_on_the_line_wins_over_its_bad_variable_in_every_spelling` (green, 0.20 s): `--url x`, `--url=x`, `--url ""`, `--url=`, the flag twice, the flag before the subcommand, `wake --url x --follow`; the same for `--tls-pin` (also with `--json` first) and `--operator-public-key` (separate, `=`, empty). Each run is byte for byte the run without the variable. Kills M3, M4, M9 (the story's cases miss M9).
- `after_a_double_dash_the_variable_is_named_and_a_bad_flag_value_wins` (green, 0.04 s):
  - `read -- --url` and `read -- --tls-pin` name the variable.
  - `--url <bad>` names `argument 3`; `--url=<bad>` names `argument 2`.
  - `--outbox <bad name>` still names `INBOX_URL`.
  - Kills M11 (the story's cases miss it).
- `serve_with_a_bad_variable_opens_nothing_and_writes_nothing` (green, 0.04 s): uses a real key set and a port the case picked. For `INBOX_BACKEND` and for `INBOX_OPERATOR_PUBLIC_KEY`, in text and with `--json`:
  - exit 2 and the naming line;
  - standard output is empty (text) or one document (`--json`);
  - the port can be bound afterwards;
  - the directory listing is unchanged and no `state` directory exists.
- `the_line_reads_back_as_the_first_64_bytes_and_the_count_of_the_rest` (green, 0.06 s): the case decodes the escapes back to bytes and checks them against the value. Values tried:
  - lengths 1, 64, 65 and 4096 bytes;
  - a line feed, CR, tab, ESC, DEL, 0x80, a quote, a backslash;
  - the text `\xFF` beside the byte 0xFF;
  - UTF-8 up to the cut with a character split at byte 64;
  - UTF-8 except the last byte.

  Every line is one line of printable ASCII under 400 bytes. Kills M6, M7, M10 (the story's cases miss M10).
- `the_usage_document_names_each_variable_before_and_after_the_subcommand` (green, 0.05 s): for all 4 text variables, `--json` before and after the subcommand gives exit 2 and one document with members `error`, `detail` in that order (the sibling's shape). The detail is the line without `inbox: usage: `.
- `of_2_bad_variables_the_same_is_named_over_20_runs` (green, 0.23 s): `wake` names `INBOX_URL` and `serve` names `INBOX_OPERATOR_PUBLIC_KEY`, 20 times each.
- `a_bad_variable_beside_another_usage_error` (green, 0.06 s):
  - An unknown flag or subcommand gets clap's message, byte for byte as without the variable.
  - A missing argument (`read`, `refuse m`, `post --to x`) gets the one line that names the variable: clap applies variables before it checks required arguments.
  - Kills M9.
- `a_json_word_after_a_double_dash_asks_for_no_document` (**red**, pre-existing): finding F1.

Red output, captured from the first run of the cases alone, before the suite (`cargo nextest run -p inbox-cli --locked -E 'test(/adversary_variable_not_utf_8::/)'`; the first lines say `slot: synced to build-1d … at b88cb667 + 2 changed files`):
```
thread 'adversary_variable_not_utf_8::a_json_word_after_a_double_dash_asks_for_no_document' (2051579) panicked at crates/inbox-cli/tests/adversary_variable_not_utf_8.rs:597:5:
a document though --json is the message id: "{\"error\":\"usage\",\"detail\":\"INBOX_URL is not UTF-8: \\\"\\\\xFF\\\"\"}\n"
     Summary [   0.293s] 10 tests run: 9 passed, 1 failed, 476 skipped
```
The run before that one did not compile because of a lifetime typo in my own case. Nothing executed in it.

Table of attack 1 (the case prints it). `yes` means the line names the variable:

| command | INBOX_URL | INBOX_TLS_PIN | INBOX_OPERATOR_PUBLIC_KEY | INBOX_BACKEND | INBOX_SECRET_KEY_FILE | INBOX_OUTBOX |
|---|---|---|---|---|---|---|
| open, close, post, pending, wait, wake, wake --follow, bridge, read, answer, refuse, outbox, collect, thread, replies, repost | 2, 1 line, yes | 2, 1 line, yes | as without, no | as without, no | as without, no | as without, no |
| serve | as without, no | as without, no | 2, 1 line, yes | 2, 1 line, yes | as without, no | as without, no |
| key new, tls new, keyfile add-key, keyfile add-delegation | as without, no | as without, no | as without, no | as without, no | as without, no | as without, no |

- No cell shows clap's 5 lines.
- In "as without" cells the line is the command's own refusal, 1 line: exit 2 for client commands and `serve`, exit 4 for `key`, `tls` and `add-delegation`, exit 2 for `add-key`.
- Help and version print beside every variable.

## 3. Suite runs (after the cases existed)
- `cargo nextest run -p inbox-cli --locked --no-fail-fast`, run 1: `Summary [  12.300s] 486 tests run: 485 passed, 1 failed, 0 skipped`, exit 100.
- Run 2: `Summary [  12.581s] 486 tests run: 485 passed, 1 failed, 0 skipped`, exit 100.
- The one failure in both runs is `a_json_word_after_a_double_dash_asks_for_no_document`.
- Without my file (`-E 'not test(/adversary_variable_not_utf_8::/)'`): `Summary [  12.125s] 476 tests run: 476 passed, 10 skipped`, exit 0. The implementor reported 12.915 s and 12.223 s for 476. No new case runs over 1 s; the slowest is the table at 0.83 s.
- Two runs before these used fail-fast and stopped at the red case (161 and 160 of 486). I discarded them as counts.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings`: exit 0.
- `slot --fetch generated/cli/inbox.json task cli-reference`: exit 0, no diff in the tree.
- Every job's first line names `build-1d`. I saw no stale binary.

Mutants. Each run is `cargo nextest run -p inbox-cli --locked --no-fail-fast -E 'test(/variable_not_utf_8|the_refused_variable|argument_not_utf_8/)'` over 54 cases.

| Mutant | Story's cases | New cases |
|---|---|---|
| M1 `INBOX_BACKEND` read with `var(..).ok()` | caught: 2 | caught: 3 |
| M2 check skipped for `serve` | caught: 2 | caught: 4 |
| M3 check run before clap, flags on the line ignored | caught: 4 | caught: 6 |
| M4 all 4 variables checked for every command before clap | caught: 7 | caught: 6 |
| M5 path filter removed from `refused_variable` | unit case only | **survives** |
| M6 cut at 64 removed | caught: 1 | caught: 1 |
| M7 line-break escape removed | caught: 1 | caught: 1 |
| M8 variable name replaced by the flag name | caught: 11 + unit case | caught: 7 |
| M9 `InvalidUtf8` guard dropped (main.rs:951) | **survives** | caught: 2 |
| M10 `(and 1 more bytes)` | **survives** | caught: 1 |
| M11 words after `--` counted as flags (scan) | **survives** | caught: 1 |

Notes on the mutant runs:
- The red json case was red in every run and is not counted above.
- The first M1 run had my 2 serve cases end at the runner's 60 s bound, because `.output()` waited on a listening service. I added the 10 s kill and reran M1: 6 failed, the slowest at 10.05 s.

Your question on the path filter: no case through the binary can hold it with the current argument order. In `ClientArgs`, `url` and `tls_pin` come before `secret_key_file` and `outbox`, and `serve` has no path variable. Clap therefore fails on a text variable before a path variable can come up, or it does not fail at all. M5 survived all 53 binary cases. Only a scan that wrongly judges a text argument to be a file name would expose it, and none was found. The unit case is the right holder.

## 4. Findings (tree: b88cb667 plus the new case file)
- **F1** `crates/inbox-cli/src/main.rs:947`. Verdict CONFIRMED, origin pre-existing, severity note.
  - What was measured: `inbox read -- --json` with a bad `INBOX_URL` prints a usage document on standard output. `--json` there is the message id: without the variable the same line prints no document. `wants_json` tests every argument, including those after `--`.
  - Origin: base a1881d9 has the same expression, and clap's path printed `{"error":"usage","detail":"invalid UTF-8 …"}` for this line.
  - What reaches it: a message id or thread id spelled `--json` after `--`. Rare.
  - Fix: compute `wants_json` from the arguments before a bare `--`.
- **F2** `crates/inbox-cli/src/main.rs:951`. Verdict CONFIRMED, origin introduced, severity note.
  - What was measured: dropping the `ErrorKind::InvalidUtf8` guard (M9) leaves the story's suite green. With it dropped, an unknown flag or a repeated flag beside a bad variable gets the variable's line instead of clap's message, which goes against the decided "any other clap error keeps clap's message".
  - Fix: the new cases `a_bad_variable_beside_another_usage_error` and `a_flag_on_the_line_wins_over_its_bad_variable_in_every_spelling` hold it.
- **F3** `crates/inbox-cli/src/main.rs:794` (with `:777`). Verdict CONFIRMED, origin introduced, severity note.
  - What was measured: counting words after `--` into `given` (M11) leaves the story's suite green. `inbox read -- --url` would then get clap's 5 lines instead of the `INBOX_URL` line.
  - Fix: held by `after_a_double_dash_the_variable_is_named_and_a_bad_flag_value_wins`.
- **F4** `crates/inbox-cli/src/main.rs:924`. Verdict CONFIRMED, origin introduced, severity note.
  - What was measured: the singular ` (and 1 more byte)` is untested by the story's cases (M10 survives). Only a 136-byte rest is asserted.
  - Fix: held by `the_line_reads_back_as_the_first_64_bytes_and_the_count_of_the_rest` (a 65-byte value).

## 5. Attacked and not broken
- How `refused_variable` finds the variable:
  - A flag before the subcommand, an abbreviation or an alias: clap refuses the line itself, so the output is the same as without the variable. Clap has no aliases here and does not infer abbreviations.
  - A flag after `--` and the same flag twice: correct.
  - A hyphen word as another option's value: clap refuses it.
  - `help <command>`: help prints, exit 0.
  - No argument with a variable has a parser other than `String` or `PathBuf`.
- Whether each command reads a variable: all 126 cells match.
- `serve` opens nothing: proved by facts (port free, no file, no state directory). `serve` has no `--backend` flag (clap rejects it as an unknown argument), and the documents name only `INBOX_BACKEND`.
- The line: one line of printable ASCII, under 400 bytes; the escapes read back unambiguously; the cut and the count are right.
- Pin and public key: the documents (`docs/design/service.md`, `CHANGELOG.md`, `docs/checks.md`) say the value is shown escaped. The sibling shows an argument with the same escapes, uncut. A pin and a public key are not secrets; I agree and file no note.
- `--json` before and after the subcommand: the same members as the sibling.
- 2 variables at once: a stable order over 20 runs; an argument wins over a variable.
- The sentences in `reference.rs`, `generated/cli/inbox.json`, `website/docs/first-use.md`, `AGENTS.md`, `docs/checks.md` (17 cases, counted), `docs/design/service.md` and the `CHANGELOG.md` bullet match the code and the table.
- A temporary probe, since removed: clap's help shows the variable's value lossily in `[env: …]`. Piped output carried no raw ESC byte. A terminal was not tested.

Not reached: a NUL in a value (the standard library refuses a NUL in an environment value when it starts the process; not run), and a terminal (pty) for help output.

## 6. Paths written outside the adversary tree
All 27 are in the scratch directory, which the brief assigned. No `target` directory is there.
- Logs: `01-cases-alone.log`, `02-cases-alone.log`, `03-table.log`, `04-help-probe.log`, `05-clippy.log`, `06-cli-reference.log`, `07-nextest-1.log`, `08-nextest-2.log`, `09-done.log`, `10-nextest-1.log`, `11-nextest-2.log`, `12-nextest-without.log`, `13-done.log`, `m01.log`, `m01b.log`, `m02.log` … `m11.log`.
- Scripts: `wait.sh` and `summary.sh`. They only wait and read logs, and were started without `slot`.

Build commands run without `slot` and outside the shim: none. `cargo` went through the shim; the task ran as `slot --fetch … task cli-reference`.

## 7. Findings block
```findings
[
  {"file": "crates/inbox-cli/src/main.rs", "line": 947, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A word --json after a bare -- still makes the clap error path print a usage document on standard output, which the same line without the bad variable does not."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 951, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping the InvalidUtf8 guard before refused_variable leaves the story's cases green; the new cases for an unknown or repeated flag beside a bad variable kill it."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 794, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Counting words after -- as flags on the line leaves the story's cases green although inbox read -- --url would then lose the INBOX_URL line; the new double-dash case kills it."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 924, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The singular count after a 65-byte value is untested by the story's cases; the new read-back case kills a wrong plural."}
]
```
