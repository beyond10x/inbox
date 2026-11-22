---
format: aep.planning-md/3
id: review-result:adversary-pass-1-on-the-client-prints-no-control-sequence-of-another-principal
kind: review-result
status: active
title: Adversary pass 1 on story:the-client-prints-no-control-sequence-of-another-principal
relations:
- reviews: story:the-client-prints-no-control-sequence-of-another-principal
revision: 1
---
unit: story:the-client-prints-no-control-sequence-of-another-principal, the branch `fix/collect-record` after round 1 of the implementor
verdict: CONFIRMED
cases: executed 104→111 (the package `inbox-cli`), red 0
origin: introduced 6 / pre-existing 1 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with each
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added
`crates/inbox-cli/tests/adversary_control_sequences.rs` (7 cases) and one `mod` line in
`crates/inbox-cli/tests/main.rs`. Every mutation ran in a copy outside the tree.

**2. Cases added (7), all green.** A body with all 65 C0, DEL and C1 characters but the line end,
through `pending` in text and with `--json`; a refusal of the service and a transport failure
with `--json`; a reply printed after the outbox file could not be written (exit 4); every
member of a row a service sets, and a `read` outcome; an answer that is not UTF-8 or not JSON;
one body of 65536 escape characters (linear time; 2 s alone, 12 s at a load average of 122); a
body that ends in a line end before its reply, compared with the lines the binary of the commit
before prints.

**3. Suite run.** `cargo nextest run -p inbox-cli --locked --no-fail-fast`: 111 run, 111 passed,
0 skipped. Without the 7 cases: 104.

18 mutants against the package. 13 are killed by the cases of the unit. 4 pass the cases of the
unit and are killed only by the cases of this pass (F1 to F4). 1 is equivalent (the scan case
without `dbg!(`).

**4. Findings.**

F1. `crates/inbox-cli/src/main.rs`: no case of the unit puts a control character into the
`--json` document of a failure; the document written through the text sink passes them.

F2. `crates/inbox-cli/src/main.rs`: no case of the unit puts one into the text printed after a
local failure; that text written through the document sink passes them.

F3. `crates/inbox-cli/src/output.rs`, `indent`: a split that adds an empty indented line after a
body ending in a line end passes the cases of the unit.

F4. `crates/inbox-cli/src/output.rs`, `visible`: leaving U+009F out passes the cases of the unit,
whose text cases use 2 of the C1 values.

F5. `crates/inbox-cli/tests/control_sequences.rs`: the case the story names checks that no
control character is printed and not that the reply and the reason are shown.

F6. `AGENTS.md`, the row of `crates/inbox-cli`: it says every write goes through
`src/output.rs`; clap writes help, the version and usage errors itself, and the standard hook
writes a panic.

F7. `crates/inbox-cli/src/main.rs`: an argument that is not UTF-8 ends the client with a panic
(exit 101) where the documents state the exit statuses 0 to 4. The panic message shows the
argument escaped. Pre-existing, outside the story.

**5. Attacked and not broken.** The output of the binary before and after the unit on a fixture
without control characters: 28 comparisons in text and with `--json`, each identical. No other
writer to either stream in `src/`. Every member of a row, the outcome text, the `refused` text
and the detail of a transport failure are escaped. An answer that is not UTF-8, a lone surrogate
and an escape character in the status line give exit 3 with nothing raw. A backslash before a C1
character reads back as the value sent. `wait` prints once.

**What the coordinator did.**

- F1 to F4: the 7 cases of the pass stay in the suite.
- F5: the named case also requires the reply and the reason shown in their escaped form.
- F6: the row states that the client's own code writes through the 3 sinks, what the scan case
  looks for, and that clap and the panic hook write themselves.
- F7: filed as the draft story:an-argument-that-is-not-utf-8-is-a-usage-error.

```findings
- file: crates/inbox-cli/src/main.rs
  line: 505
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the --json failure document has no case with a control character; printing it through print_text (invalid JSON) passes all 9 of the unit's cases
- file: crates/inbox-cli/src/main.rs
  line: 507
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the text of Failure::After has no case with a control character; printing it through print_document (ESC raw) passes the unit's suite
- file: crates/inbox-cli/src/output.rs
  line: 219
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: indent with split instead of split_inclusive adds an empty indented line after a body ending in a line end and passes the unit's suite, because its byte-for-byte case has that line end only at the end of the output
- file: crates/inbox-cli/src/output.rs
  line: 168
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: leaving U+009F out of visible passes the unit's suite; its text cases use only the C1 values U+009B and U+0085
- file: crates/inbox-cli/tests/control_sequences.rs
  line: 68
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the acceptance case the story names checks only for absence and stays green when visible drops control characters instead of showing them
- file: AGENTS.md
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the inbox-cli row says every write to stdout and stderr goes through output.rs, but clap writes help, version and usage errors and the panic hook writes panics, and the source-scan case has no positive control
- file: crates/inbox-cli/src/main.rs
  line: 491
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: an argument that is not UTF-8 makes std::env::args() panic with exit 101 instead of the usage status 2, outside the documented exit statuses 0 to 4
```
