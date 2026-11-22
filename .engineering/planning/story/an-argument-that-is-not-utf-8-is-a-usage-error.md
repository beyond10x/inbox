---
format: aep.planning-md/3
id: story:an-argument-that-is-not-utf-8-is-a-usage-error
kind: story
status: implemented
title: An argument that is not UTF-8 is a usage error
summary: An argument of inbox that is not UTF-8 ends the client with a panic and exit status 101, outside the documented statuses 0 to 4.
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T18:27:22Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T18:27:22Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T20:27:26Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Problem

An argument of `inbox` that is not UTF-8 can end the client with a panic and the exit status
101: the scan for `--json` on the error path reads the arguments with `std::env::args()`.
`AGENTS.md` and the reference document of the command line state the exit statuses 0 to 4,
and 2 for wrong usage. The panic message shows the argument in its escaped form, so no control
sequence reaches the terminal.

Found by adversary pass 1 on
story:the-client-prints-no-control-sequence-of-another-principal (finding F7, pre-existing).

## Acceptance

An argument that a command reads as text (a command name, an option name, an address, an id, a
body given on the command line) and that is not UTF-8 ends `inbox` with the exit status 2 and
one line of printable ASCII on standard error that names the argument in an escaped form, and
sends no request (state transition: exit status 101 with a panic before, 2 after).

The value of an option that takes a file name is not text: a file name that is not UTF-8 is
accepted there as before, in both spellings (`--option value` and `--option=value`).

## Decision on the acceptance

The first wording was "an argument of any command". Adversary pass 1
(review-result:adversary-an-argument-that-is-not-utf-8-pass-1, finding 1) showed that it
refused file names the client took before. The coordinator narrowed it on 2026-11-22: a
working invocation keeps working.

## Scope

- `crates/inbox-cli/src/main.rs`: the arguments are read with `std::env::args_os()`.
- Cases in `crates/inbox-cli/tests/`.

## Measured

- 26 new cases in `inbox-cli` (143 cases, none ignored): 10 of the unit in
  `tests/argument_not_utf_8.rs`, 9 of adversary pass 1 and 7 of pass 2; each under 1 s.
- Adversary pass 1: 3 findings, closed. Pass 2: 4 findings, 3 closed and 1 that existed before
  filed as the draft story:a-variable-that-is-not-utf-8-is-a-usage-error.
- `task check` on the branch merged with `main`, 2026-11-22T20:25:04Z to 2026-11-22T20:26:47Z, load average 58 to 77
  on 20 CPUs: exit 0, `668 tests run: 668 passed, 5 skipped`, the test step 33 s of
  its bound of 120 s.
