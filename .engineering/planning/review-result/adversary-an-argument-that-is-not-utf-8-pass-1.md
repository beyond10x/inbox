---
format: aep.planning-md/3
id: review-result:adversary-an-argument-that-is-not-utf-8-pass-1
kind: review-result
status: active
title: Adversary on the story of an argument that is not UTF-8, pass 1
relations:
- reviews: story:an-argument-that-is-not-utf-8-is-a-usage-error
revision: 1
---
unit: story:an-argument-that-is-not-utf-8-is-a-usage-error, uncommitted working tree on base 7cd7416 (branch fix/argument-not-utf-8), <tree>
verdict: NEEDS-CHANGE
cases: executed 119→128 (crate total, from nextest's "Starting 9 tests ... (119 tests skipped)"; 119 is also the count in docs/checks.md), red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 paths
needs-coordinator: should path-valued options (--out, --secret-key-file, --outbox, --key-file, --file, --operator-secret-key-file, --body-file) keep accepting a file name that is not UTF-8? The acceptance says "an argument of any command", and the code obeys that. But the story's premise ("reads its arguments with std::env::args()") was false for those options, so they worked before this change.

**1. git diff --stat** (the unit's own uncommitted changes plus mine)
```
 ...-argument-that-is-not-utf-8-is-a-usage-error.md |  7 ++--
 AGENTS.md                                          |  2 +-
 CHANGELOG.md                                       | 10 ++++++
 crates/inbox-cli/src/main.rs                       | 38 ++++++++++++++++++++--
 crates/inbox-cli/src/reference.rs                  |  2 +-
 crates/inbox-cli/tests/main.rs                     |  2 ++
 docs/checks.md                                     | 14 +++++++-
 generated/cli/inbox.json                           |  2 +-
```
I touched two test files and nothing else:
- `crates/inbox-cli/tests/main.rs`: one line, `mod adversary_argument_not_utf_8;`. The other `+` line in that file is the unit's.
- `crates/inbox-cli/tests/adversary_argument_not_utf_8.rs`: new and untracked.

I touched no non-test path.

**2. Cases added**, in `crates/inbox-cli/tests/adversary_argument_not_utf_8.rs`. All 9 were run alone first (`-E 'test(/adversary_argument_not_utf_8/)'`, 9 run, 119 skipped). None takes over 1 s.

- `a_path_option_that_is_not_utf_8_still_names_a_file`: **FAIL** (0.20 s). Runs `key new --out <dir>/caf\xE9.key --key-id .. --principal ..` and expects exit 0 and the file made. Red output:
  ```
  assertion `left == right` failed: key new --out <file name that is not UTF-8>: "inbox: usage: argument 4 is not UTF-8: \"/…/target/tmp/inbox-cli-adv-path-out-1058113-0/caf\\xE9.key\"\n"
    left: Some(2)
   right: Some(0)
  ```
- `the_secret_key_file_option_takes_the_path_its_variable_takes`: **FAIL** (0.42 s). The same key file, renamed to `caf\xE9.key`:
  - through `INBOX_SECRET_KEY_FILE`, the key is read and the client reaches the transport step (exit 3);
  - through `--secret-key-file`, it is refused.

  Red output:
  ```
  assertion `left == right` failed: the option form "inbox: usage: argument 3 is not UTF-8: \"/…/caf\\xE9.key\"\n" answered other than the variable form "inbox: transport: 127.0.0.1:46345: 127.0.0.1:46345: Connection refused (os error 111)\n"
    left: Some(2)
   right: Some(3)
  ```
- `a_refused_argument_with_letters_that_are_not_ascii_is_written_in_printable_ascii`: **FAIL** (0.06 s). The argument `grüße\xFF` should give one line of printable ASCII only. Red output:
  ```
  not-ascii: a byte that is not printable ASCII reached standard error: "inbox: usage: argument 2 is not UTF-8: \"grüße\\xFF\"\n"
  ```
- `a_line_feed_in_a_refused_argument_does_not_start_a_second_line`: PASS (0.10 s). The argument holds `\xFF\ninbox: refused: forged (403)\n\x85` and U+2028. The output stays one line and holds `\n` as text.
- `the_json_flag_after_the_refused_argument_gets_the_usage_document`: PASS (0.13 s). `open \xFF --json` prints exactly `{"error":"usage","detail":"argument 2 is not UTF-8: \"\\xFF\""}`.
- `a_long_refused_argument_stays_one_printable_line`: PASS (0.66 s). A 96 KiB argument of `\x01\xFF\n` stays one line.
- `a_program_name_that_is_not_utf_8_does_not_panic`: PASS (0.20 s). argv[0] is `inbox\xFF`, followed by a clap usage error: exit 2, no panic. The base panicked here, because clap's error path called `std::env::args()`.
- `an_inbox_url_that_is_not_utf_8_is_a_usage_error`: PASS (0.38 s). `INBOX_URL` set to `http://127.0.0.1:1/\xFF\x1b[2J` gives exit 2, no panic, and no ESC or high byte on standard error.
- `a_utf_8_argument_that_is_not_ascii_reaches_the_command`: PASS (0.17 s). `key new --out grüße.key` makes the file. The unit's own case 7 only checks for the absence of "not UTF-8" on a run that clap refuses anyway, so it could not catch a mutant that refuses non-ASCII arguments; this case would.

**3. Suite run**, after the cases existed:
`cargo nextest run -p inbox-cli --locked -E 'test(/argument_not_utf_8/)' --no-fail-fast`
```
    Starting 16 tests across 2 binaries (112 tests skipped)
     Summary [   1.153s] 16 tests run: 13 passed, 3 failed, 112 skipped
        FAIL [   0.141s] ( 1/16) inbox-cli::main adversary_argument_not_utf_8::a_path_option_that_is_not_utf_8_still_names_a_file
        FAIL [   0.182s] ( 3/16) inbox-cli::main adversary_argument_not_utf_8::a_refused_argument_with_letters_that_are_not_ascii_is_written_in_printable_ascii
        FAIL [   0.864s] (16/16) inbox-cli::main adversary_argument_not_utf_8::the_secret_key_file_option_takes_the_path_its_variable_takes
error: test run failed
exit=100
```
All 7 of the unit's own cases pass.

**4. Findings** (they cover the working tree on 7cd7416)

F1, `crates/inbox-cli/src/main.rs:488` (`arguments()`; also :520). Category acceptance, warning, NEEDS-CHANGE, introduced.
- **What was measured:** a file name that is not UTF-8, given to any `PathBuf` option, is now refused with exit 2. The two red path cases above show it.
- **Why introduced:** read from the base with `git show HEAD:crates/inbox-cli/src/main.rs`, not run.
  - The base parsed with `Cli::try_parse()`, which reads `args_os()`, and clap's `PathBuf` parser takes any `OsStr`.
  - `std::env::args()` ran only on clap's error path (base line 498), so these options worked.
  - In the current tree, the `INBOX_SECRET_KEY_FILE` form of the same path still works, which shows that clap and the client handle such a path.
- **What reaches it:** a user passing a Latin-1 file name on Linux, e.g. `inbox post --body-file "$f"`. This is rare but real.
- **Fix to name (not applied):** keep `Cli::try_parse_from(std::env::args_os())`. Only on clap's `ErrorKind::InvalidUtf8`, find the first argument that is not UTF-8 and print the escaped line; use `args_os()` for the `--json` scan.
- **Decision for the coordinator:** this collides with the acceptance text "any command", see needs-coordinator.

F2, `crates/inbox-cli/src/main.rs:500`. Category boundary, note, CONFIRMED, introduced.
- **What was measured:** `{raw:?}` on an `OsString` uses `escape_debug`, so printable letters outside ASCII in the refused argument reach standard error raw, e.g. `"grüße\xFF"`. This contradicts the claim that the line holds only printable ASCII; the unit's own helper asserts that claim, but only for ASCII inputs.
- **No terminal hazard:** control characters, C1, U+2028, bidi controls and the line feed are all escaped. The line-feed and C1 cases pass.
- **What reaches it:** a path that mixes UTF-8 letters with a Latin-1 byte.
- **Fix to name:** `raw.as_bytes().escape_ascii()`, if the ASCII claim is wanted. Otherwise drop the claim.

F3, `CHANGELOG.md:88` and `docs/checks.md:163`. Category contract-drift, note, NEEDS-CHANGE, introduced.
- **What was measured:** two statements are only partly true:
  - The CHANGELOG says an option value that is not UTF-8 "Before, the client panicked and ended with the status 101".
  - docs/checks.md says the client "read its arguments with `std::env::args()`".
- **What the base actually did:** clap read `args_os()`. A path option worked. A `String` value made clap fail, and the panic came from the `--json` scan on that error path.
- **What reaches it:** readers of the release notes are told a working feature was a crash.

**5. What I attacked and could not break**
- Line forging: a line feed, CR, ESC, OSC, C1 (raw and as UTF-8 U+009B/U+0085) and U+2028 in the refused argument all come out escaped on one line.
- A 96 KiB argument stays one line. It is unbounded in length, but the kernel caps it at 128 KiB.
- `--json` before or after the bad argument gives the exact document.
- argv[0] that is not UTF-8: no panic (the base panicked here).
- `INBOX_URL` that is not UTF-8: exit 2 with clap's own message, no raw byte.
- A UTF-8 argument outside ASCII reaches the command.
- Mutants of `arguments()`: I reasoned about these from the unit's literal assertions and did not run them on a mutated copy. Counting from 0, dropping the escape (lossy) and skipping the first argument would each be caught by the unit's positional, `\xFF` and before-subcommand cases.
- Not tested, so no claim: `INBOX_BACKEND` that is not UTF-8 reads as unset (`std::env::var(..).ok()` at main.rs:380; this is pre-existing), and `--help` or `--version` next to a bad argument.

**6. Paths written outside the worktree**
- <scratch>/d30/utf8/adversary/own-cases.log
- <scratch>/d30/utf8/adversary/suite.log

The tests' own temporary directories under the worktree's `target/tmp` are removed on drop.

```findings
- file: crates/inbox-cli/src/main.rs
  line: 488
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a file name that is not UTF-8 given to a PathBuf option (key new --out, --secret-key-file, --body-file and others) worked before through clap's args_os and is now refused with exit 2, while the same path through INBOX_SECRET_KEY_FILE is still accepted
- file: crates/inbox-cli/src/main.rs
  line: 500
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the Debug escape leaves printable letters outside ASCII raw, so a refused argument like grüße\xFF puts non-ASCII bytes on the one line claimed to be printable ASCII only
- file: CHANGELOG.md
  line: 88
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: CHANGELOG.md:88 and docs/checks.md:163 say every argument that is not UTF-8, option values included, made the client panic because it read std::env::args(), but clap read args_os and path options worked; the panic came only from the --json scan on clap's error path
```
