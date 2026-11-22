---
format: aep.planning-md/3
id: review-result:adversary-an-argument-that-is-not-utf-8-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the argument that is not UTF-8: 3 findings of pass 1 closed, 4 new'
relations:
- reviews: story:an-argument-that-is-not-utf-8-is-a-usage-error
revision: 1
---
unit: story:an-argument-that-is-not-utf-8-is-a-usage-error, round 2 of the implementor, uncommitted working tree on base 7cd7416 (branch fix/argument-not-utf-8), <tree>
verdict: CONFIRMED (one warning: the acceptance still fails for 4 spellings of the command line)
cases: executed 130→137 (130 is the implementor's reported count, and also `cargo nextest list -p inbox-cli` minus my 7), red 4
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 10 paths (part 6)
needs-coordinator: none

**1. git diff --stat** (the unit's uncommitted changes plus mine)
```
 ...-argument-that-is-not-utf-8-is-a-usage-error.md | 34 ++++++---
 AGENTS.md                                          |  2 +-
 CHANGELOG.md                                       | 15 ++++
 crates/inbox-cli/src/main.rs                       | 84 +++++++++++++++++++++-
 crates/inbox-cli/src/reference.rs                  |  2 +-
 crates/inbox-cli/tests/main.rs                     |  3 +
 docs/checks.md                                     | 22 +++++-
 generated/cli/inbox.json                           |  2 +-
```
I touched two files, both tests:
- `crates/inbox-cli/tests/main.rs`: one added line, `mod adversary_argument_not_utf_8_2;`. The other 2 `+` lines are the unit's.
- `crates/inbox-cli/tests/adversary_argument_not_utf_8_2.rs`: new, untracked.

I touched no non-test path.

**Pass-1 findings**
- F1 (file names refused): closed.
  - Pass 1's own cases pass in the suite run: `a_path_option_that_is_not_utf_8_still_names_a_file` and `the_secret_key_file_option_takes_the_path_its_variable_takes`.
  - My `the_path_options_of_every_command_take_a_file_name_that_is_not_utf_8` passes. It covers `keyfile add-key --file` and `--operator-secret-key-file=`, `keyfile add-delegation --file=` and `--operator-secret-key-file`, `post --body-file=`, `answer --body-file`, and `--json open --outbox=`.
  - The probe `key new --out=-\xFF ...` gave status=0 and made the file `-\377`.
- F2 (letters outside ASCII left raw): closed.
  - Pass 1's `a_refused_argument_with_letters_that_are_not_ascii_is_written_in_printable_ascii` passes.
  - My `every_byte_value_of_a_refused_argument_is_written_as_documented` passes: bytes 0x01..0xFF in one argument give the exact expected escape string on one line.
- F3 (CHANGELOG and docs/checks.md misstated the base): closed.
  - CHANGELOG.md:84-95 and docs/checks.md:160-179 now say that clap read `OsString`, that path options worked, and that the panic came from the `std::env::args()` scan on the error path.
  - This matches `git show main:crates/inbox-cli/src/main.rs` line 498.
  - One new imprecision is finding 2 below.

**2. Cases added** in `crates/inbox-cli/tests/adversary_argument_not_utf_8_2.rs`, each run alone first. Command: `ESS_TOOLCHAIN=0.97.3 cargo nextest run -p inbox-cli --locked --no-fail-fast -E 'test(/adversary_argument_not_utf_8_2::/)'`. Result: 7 run, 2 passed, 5 failed, 130 skipped, 0.45 s. The 5th failure was a typo in my own fixture: `--public-key AAAA` is a usage error of its own. I fixed it and reran that case alone: PASS in 0.15 s. No case takes over 1 s.

Each red case asserts the acceptance: exit 2, empty standard output, exactly one `\n`, every byte printable ASCII, the line names the argument, and the stand-in listener got no connection.
- `a_message_id_after_double_dash_that_starts_like_a_path_option_is_named`, `read -- --out=\xFF`: **FAIL**
  `adv2-dashdash-equals: not one line: "error: invalid UTF-8 was detected in one or more arguments\n\nUsage: inbox read [OPTIONS] <MESSAGE_ID>\n\nFor more information, try '--help'.\n"  left: 5 right: 1`
- `a_stray_argument_after_double_dash_and_a_path_option_name_is_named`, `read -- --out \xFF`: **FAIL**
  `adv2-dashdash-split: a byte that is not printable ASCII reached standard error: "error: unexpected argument '�' found\n\nUsage: inbox read [OPTIONS] <MESSAGE_ID>\n\nFor more information, try '--help'.\n"`
- `an_option_name_that_is_not_utf_8_after_a_path_option_is_named`, `key new --out -\xFF --key-id .. --principal ..`: **FAIL**
  `adv2-hyphen-value: a byte that is not printable ASCII reached standard error: "error: unexpected argument '-�' found\n\nUsage: inbox key new [OPTIONS] --out <OUT>\n\nFor more information, try '--help'.\n"`
- `a_path_option_of_another_subcommand_is_not_a_file_name`, `open --file=\xFF`: **FAIL**
  `adv2-other-subcommand: not one line: "error: unexpected argument '--file' found\n\nUsage: inbox open [OPTIONS]\n\nFor more information, try '--help'.\n"  left: 5 right: 1`
- `every_byte_value_of_a_refused_argument_is_written_as_documented`: PASS, 0.11 s. It kills the mutants: a lower-case hex escape, a dropped `"` or `\` escape, a dropped `\n`/`\r`/`\t` arm.
- `a_surrogate_half_and_an_overlong_encoding_are_refused_and_escaped`, `open --adapter \xED\xA0\x80\xC0\xAF`: PASS, 0.53 s.
- `the_path_options_of_every_command_take_a_file_name_that_is_not_utf_8`: PASS, 0.77 s.

**3. Suite run**, after the cases existed. Command: `ESS_TOOLCHAIN=0.97.3 cargo nextest run -p inbox-cli --locked --no-fail-fast`, 38.4 s wall.
```
    Starting 137 tests across 2 binaries
     Summary [  37.157s] 137 tests run: 133 passed, 4 failed, 0 skipped
        FAIL ( 31/137) adversary_argument_not_utf_8_2::a_message_id_after_double_dash_that_starts_like_a_path_option_is_named
        FAIL ( 33/137) adversary_argument_not_utf_8_2::a_stray_argument_after_double_dash_and_a_path_option_name_is_named
        FAIL ( 34/137) adversary_argument_not_utf_8_2::a_path_option_of_another_subcommand_is_not_a_file_name
        FAIL ( 36/137) adversary_argument_not_utf_8_2::an_option_name_that_is_not_utf_8_after_a_path_option_is_named
exit=100
```
- All 18 of the unit's and pass 1's cases pass.
- Cases over 5 s are none of mine. These pre-existing ones are: `adversary::add_key_processes_at_once_lose_no_entry` 16.8 s, `adversary::an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires` 5.0 s, `cases::each_client_command_works_against_a_running_service` 5.0 s.
- Real case count of `inbox-cli`: `cargo nextest list -p inbox-cli --locked` gives 137 (111 in `main`, 26 in `bin/inbox`). That is 130 without my 7, so the "130 cases" in docs/checks.md:21 equals the tree I was given.
- Also ran: `cargo build -p inbox-cli --locked` (4.7 s), and a POSIX sh probe of 26 command lines against `target/debug/inbox` (`probe.log`).

**4. Findings** (they cover the working tree on 7cd7416)

Finding 1: `crates/inbox-cli/src/main.rs:516` and `:523` (`refused_argument`). Category acceptance, warning, CONFIRMED, introduced.
- What was measured: the scan takes any argument that equals a path option, or starts with `<path option>=`, as the start of a file name, wherever it stands:
  - after a bare `--`;
  - under a subcommand that does not have that option;
  - before a token starting with `-`, which clap never takes as a value.

  In those places an argument read as text that is not UTF-8 is not named. The client falls through to clap's own 5-line message, which does not name the argument, and in 2 of the 4 cases prints U+FFFD bytes. Exit status 2 and "no request" still hold.
- With `--json`: `--json read -- --out=\xFF` prints the generic usage document and the same 5 lines.
- On the base, all 4 command lines panicked with exit status 101, read from main.rs:498 on main.
- What reaches it: a user who types such a line. No documented workflow passes a positional that starts with `--out=` or a file name that starts with `-`. The acceptance is stated for every argument read as text.
- Fix to name (not applied): when clap refused and the scan names nothing but some argument after argv[0] is not UTF-8, name the first such argument. Or mirror clap: stop the scan at `--`, never take a token starting with `-` as a value, and scope the path options to the subcommands named so far.

Finding 2: `CHANGELOG.md:87` and `docs/checks.md:170`. Category contract-drift, note, CONFIRMED, introduced.
- Both say "each byte that is not printable ASCII written `\xE9`".
- The code (`not_utf_8`, main.rs:536-554) writes line feed, carriage return and tab as `\n`, `\r`, `\t`, and also escapes `"` and `\`, which are printable ASCII.

Finding 3: `website/docs/first-use.md:85`. Category contract-drift, note, CONFIRMED, introduced.
- The exit status table still reads "2 | Wrong usage, including an `INBOX_URL` that is not a loopback address".
- The unit changed the same row in `reference.rs:44` and `generated/cli/inbox.json` to include "an argument read as text that is not UTF-8", so the two documents now differ.

Finding 4: `crates/inbox-cli/src/main.rs:382`. Category boundary, note, CONFIRMED, pre-existing.
- `INBOX_BACKEND` set to `\xFF` is read as unset by `std::env::var(..).ok()`, so `serve` silently takes `memory`. The probe got past the backend check to `--operator-public-key: not canonical base64`.
- `INBOX_BACKEND=bogus` is refused with exit 2 (serve.rs:62).
- main has the same line, main.rs:380.

**5. Attacked and not broken**
- No exit status 101 and no signal in any probe (26 command lines, plus the 18 existing and 7 new cases). Every refusal is 2, the accepted ones 0, 3 or 4.
- `env::args()` is no longer called anywhere in `crates/inbox-cli/src`. The `--json` scan reads `args_os`.
- The one line: every byte 0x01..0xFF, a WTF-8 surrogate half, an overlong encoding, `\n`/`\r`/ESC (pass 1 and the unit's cases), and 96 KiB (pass 1) all give one printable-ASCII line ending in `\n`.
- `--json` before and after the refused argument: one JSON document on stdout, one line on stderr, exit 2. `--json=\xFF` is named as the refused argument.
- Spellings of a path option that clap accepts all keep working:
  - `--opt value` and `--opt=value`, `--out=-<bytes>`, an empty `--out=` followed by text;
  - options of the nested subcommands `key new` and `keyfile add-key|add-delegation`.

  Given twice, or before the subcommand (path options are not global), clap refuses the line with an ASCII-only message, exit 2.
- There are no short flags or aliases on path options.
- `path_options()` finds all 7 `PathBuf` options of the definition. There is no `OsString` argument and no positional `PathBuf`. It reads only `get_long()`, so an alias or a positional path added later would be missed. No case can show this today.
- `INBOX_URL` or `INBOX_OPERATOR_PUBLIC_KEY` not UTF-8 gives exit 2 with clap's generic 5-line message, ASCII only. main behaves the same, because `env::args()` panicked only on arguments. The environment is outside this acceptance.
- `--help` beside a bad argument prints help, exit 0. A bad argument before `--help`, after `help`, or as a subcommand under `key` is named, exit 2.
- docs/checks.md "130 cases" equals the list. `generated/cli/inbox.json` equals `reference.rs` (`the_cli_reference_document_is_current` passes). AGENTS.md describes the scan as coded; finding 1 is where code and acceptance part.

**6. Paths written outside the worktree** (all under the assigned scratch directory `<scratch>/d30/utf8/adversary2/`)
- build.log
- probe.sh (POSIX sh)
- probe.log
- work/err.txt
- work/out.txt
- own-cases.log
- own-cases-2.log
- list.log
- suite.log

Also the harness's background-task output file for the suite run, under the session's claude-tmp directory. The tests' own directories under `<tree>/target/tmp` are removed on drop.

```findings
[
  {"file": "crates/inbox-cli/src/main.rs", "line": 516, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "refused_argument takes any argument equal to or prefixed by a path option as a file name regardless of a preceding --, the subcommand or clap's rule that a token starting with - is no value, so read -- --out=\\xFF, read -- --out \\xFF, key new --out -\\xFF and open --file=\\xFF end with clap's 5-line message (U+FFFD bytes in two) instead of the one escaped line."},
  {"file": "CHANGELOG.md", "line": 87, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "CHANGELOG.md:87 and docs/checks.md:170 say every byte that is not printable ASCII is written \\xE9, but the code writes line feed, carriage return and tab as \\n, \\r, \\t and also escapes the printable \" and \\."},
  {"file": "website/docs/first-use.md", "line": 85, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the exit status table of the first-use page still omits the argument that is not UTF-8 from status 2, while the unit changed the same row in reference.rs and generated/cli/inbox.json."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 382, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "INBOX_BACKEND that is not UTF-8 is read as unset by std::env::var(..).ok() and serve silently takes memory, while an unknown UTF-8 backend name is refused with exit 2."}
]
```
