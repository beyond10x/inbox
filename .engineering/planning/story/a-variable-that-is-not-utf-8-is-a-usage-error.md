---
format: aep.planning-md/3
id: story:a-variable-that-is-not-utf-8-is-a-usage-error
kind: story
status: implemented
title: An INBOX variable that is not UTF-8 is a usage error, not an unset variable
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_variable_not_utf_8.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: cited
  path: crates/inbox-cli/tests/variable_not_utf_8.rs
- confidence: cited
  path: docs/checks.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T17:32:53Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T17:32:54Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T18:27:31Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Problem

`inbox serve` reads `INBOX_BACKEND` with `std::env::var(..).ok()`. A value that is not UTF-8
reads as unset, so the service starts with the backend `memory` without a word. An unknown
backend name in UTF-8 is refused with the exit status 2.

Found by adversary pass 2 on story:an-argument-that-is-not-utf-8-is-a-usage-error
(review-result:adversary-an-argument-that-is-not-utf-8-pass-2, finding 4, pre-existing).

## Acceptance

`inbox serve` with an `INBOX_BACKEND` that is not UTF-8 ends with the exit status 2 and one
line of printable ASCII on standard error that names the variable, and opens no listener
(state transition: the service starts with `memory` before, exit status 2 after). The same
holds for every other `INBOX_*` variable the client or the service reads as text.

## Scope

Written after the build, from the diff against `main` at a1881d9: every path below is read
from it (cited).

Product, one file:

- `crates/inbox-cli/src/main.rs`: `inbox serve` reads `INBOX_BACKEND` with `var_os` and
  refuses a value that is not UTF-8 before `serve::serve` is called; `Scan` also records the
  subcommands named on the line and the long options given; `scan` takes the root command;
  new `refused_variable` (which variable clap failed on), `ascii_escaped` (the escapes,
  shared with `not_utf_8`) and `variable_not_utf_8` (the line); the path of a clap error
  names an argument first and a variable only when clap's error is `InvalidUtf8`; one unit
  case.

Cases:

- `crates/inbox-cli/tests/variable_not_utf_8.rs` (new): the 17 cases of the story.
- `crates/inbox-cli/tests/adversary_variable_not_utf_8.rs` (new): 9 cases of the adversary
  pass.
- `crates/inbox-cli/tests/main.rs`: the 2 `mod` lines.

Documents and generated text:

- `crates/inbox-cli/src/reference.rs` and `generated/cli/inbox.json` (written by `task
  cli-reference`): the meaning of exit status 2.
- `website/docs/first-use.md`: the row of exit status 2.
- `AGENTS.md`: the sentence on an argument that is not UTF-8.
- `docs/checks.md`: one entry.
- `docs/design/service.md`: the order of the refusals of a client command and of `inbox
  serve`.
- `CHANGELOG.md`: one bullet under Fixed.

Read and not changed: `crates/inbox-cli/src/output.rs` (`MOST_SHOWN_CHARACTERS`,
`quoted_cut`), `crates/inbox-cli/tests/adversary_argument_not_utf_8.rs`,
`crates/inbox-cli/tests/support`.

Not touched: `crates/inbox-server`, `crates/inbox-core`, `ess/`, `generated/rust/`,
`adapters/`, `XDG_STATE_HOME`, `HOME` and every variable without the `INBOX_` prefix.

## Decisions

- The story was filed by adversary pass 2 of
  story:an-argument-that-is-not-utf-8-is-a-usage-error and accepted by the operator
  (2026-11-24). No panel of plan critics read the story.
- Which variables. The 4 that a command reads as text are refused when they are not UTF-8:
  `INBOX_BACKEND` (`inbox serve`), `INBOX_OPERATOR_PUBLIC_KEY` (`inbox serve`), `INBOX_URL`
  and `INBOX_TLS_PIN` (the client commands). The 2 path variables,
  `INBOX_SECRET_KEY_FILE` and `INBOX_OUTBOX`, keep a name that is not UTF-8: such a path is
  legal. The options were to refuse all 6 or the 4; the 4 are taken.
- The line. `inbox: usage: <variable> is not UTF-8: "<shown>"`, the form of the sibling's
  line for an argument, with the same escapes (one helper, `ascii_escaped`). The value is cut
  after its first 64 bytes and the count of the rest follows (` (and 1 more byte)`, ` (and
  <n> more bytes)`). The options were the whole value, as the sibling shows an argument, or
  the cut; the cut is taken, since a variable can hold any length. A pin and a public key
  are not secrets, so the value is shown.
- A variable the command does not read stays ignored: a variable whose flag is on the line
  (clap applies a variable only when its flag is absent), a client variable beside `inbox serve`, and
  `INBOX_BACKEND` beside a client command. A working invocation keeps working.
- Which variable is named. clap's error does not say. `refused_variable` takes the first
  argument of the subcommand the line names, in the order of its definition, that is read as
  text, has a variable, and whose long flag is not on the line. So `INBOX_URL` is named
  before `INBOX_TLS_PIN`, and for `inbox serve` `INBOX_OPERATOR_PUBLIC_KEY` before
  `INBOX_BACKEND` (clap fails before the backend is read). An argument that is not UTF-8 is
  named before any variable.
- The variable is looked for only when clap's error is `InvalidUtf8`. Every other clap error
  keeps clap's message. A missing argument beside a bad variable gets the variable's line:
  clap applies variables before it checks for required arguments.
- `refused_variable` takes the read of a variable as a parameter, so a unit case holds the
  order and the path filter without a change of the process environment. No case through the
  binary can hold the path filter: in the client arguments the 2 text variables stand before
  the 2 path variables, and `inbox serve` has no path variable.
- Short flags are not looked for on the line: no argument with a variable has one.
- The adversary pass wrote 10 cases. 9 are taken into this change. The 10th is red for a
  behaviour that was there before the story (a word `--json` after a bare `--` asks for the
  usage document): it is filed as the draft
  story:a-json-word-after-a-double-dash-asks-for-no-usage-document with the case in its
  body. The options were (A) the draft, or (B) the one-line fix in this story; A is taken:
  the behaviour is outside the story's acceptance.
- Not in this story: `XDG_STATE_HOME`, `HOME` and every variable without the `INBOX_`
  prefix; the variables of the tools and the test harness; the script of the Claude Code
  adapter, which passes its environment on unchanged. `README.md` and the pages of the wake
  and the bridge do not change.

## Measured

- Before the change, on a1881d9 on the build node, the new cases against the old product: 17
  run, 4 passed, 13 failed. `inbox serve` with an `INBOX_BACKEND` that is not UTF-8 opened a
  listener and printed its URL; a client command with an `INBOX_URL` that is not UTF-8 ended
  with clap's message of 5 lines (`error: invalid UTF-8 was detected in one or more
  arguments`, the usage, the hint) that names no variable. The 4 that passed are the working
  invocations and the argument named before a variable.
- Round 1: the 17 cases and the unit case green; the cases of the crate 458 → 476, twice in
  a row (12.9 s and 12.2 s). No new case over 1 s; the slowest new case 0.019 s.
  `generated/cli/inbox.json` differs in 1 line, the meaning of exit status 2.
- Round 1, mutants, each red against a named case and taken out by hand: `INBOX_BACKEND`
  read as unset (2 cases red); the line without the name (12); the value written raw (13);
  the cut removed (1); the check applied to a variable whose flag is on the line (2); the
  check applied to a variable of another command (2); `INBOX_BACKEND` checked for every
  command (3); `INBOX_OUTBOX` read as text (2); the path filter of `refused_variable`
  removed (1, the unit case only).
- Round 1, `task check` on the build node: 1177 passed, 8 skipped, the test step 19 s,
  exit 0.
- Adversary pass 1 (review-result:adversary-variable-not-utf-8-pass-1): no defect in the
  behaviour of the story. 10 cases written; 9 green and taken. 21 command lines against each
  of the 6 variables (126 runs): every command that reads the variable as text ends with
  exit 2 and the one line; every other run equals the run without the variable byte for
  byte; no run shows clap's 5 lines. Help and version print beside every bad variable.
- Adversary pass 1, mutants: 11 tried. 3 lived under the story's 17 cases and are red under
  the pass's cases: the `InvalidUtf8` guard dropped (2 cases red), the words after `--`
  counted as flags on the line (1), ` (and 1 more bytes)` for one more byte (1). The path
  filter removed lives under all 53 cases through the binary and is red in the unit case.
- Adversary pass 1, the 10th case, red and not taken: `inbox read -- --json` with a bad
  `INBOX_URL` prints the usage document on standard output; main at a1881d9 printed it for
  this line too, with clap's message. Filed as a draft story.
- Adversary pass 1, the crate with its cases: 486 run, 485 passed, 1 failed (the 10th case),
  12.3 s and 12.6 s; without its file 476 passed, 12.1 s. The slowest new case 0.83 s (the
  126 runs).
- After the pass, with 9 of its cases in the tree: `cargo nextest run -p inbox-cli --locked`
  on the build node, 485 passed, 0 skipped, 12.1 s.
- The pass did not reach: a NUL in a value (the standard library refuses it at the start of
  a process); the help output on a terminal, where clap shows a variable's value in `[env:
  …]`; piped, that output carried no raw escape byte.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T18:25:49Z to 2026-11-24T18:26:26Z, on the build
  node of the pool: exit 0, `1188 tests run: 1188 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
