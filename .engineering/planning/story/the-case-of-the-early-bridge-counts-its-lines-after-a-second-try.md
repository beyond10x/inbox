---
format: aep.planning-md/3
id: story:the-case-of-the-early-bridge-counts-its-lines-after-a-second-try
kind: story
status: implemented
title: The case of the early bridge counts its lines after a second try
summary: bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying counts the start lines before the second try, so a line on every try passes it.
scope:
- confidence: cited
  path: crates/inbox-cli/tests/adversary_early_bridge.rs
- confidence: cited
  path: crates/inbox-cli/tests/bridge.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T17:33:02Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T17:33:02Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T18:22:22Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

The case `bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying`
fails when the bridge prints its start line on every try.

Today the case counts the lines of the bridge's standard error about 27 ms after the first
line. The first pause of the bridge is 100 ms, so the count is taken before the second try,
and "one line for the run of failures" passes when the line is printed on every try (a
mutant of `say_once` in `crates/inbox-cli/src/wake.rs`: `if *failures == 0` replaced by `if
true`). The case
`adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`
is red under that mutant: it waits 1600 ms before it counts.

## Acceptance

- The case counts its lines after at least 2 tries of the bridge (state transition: with the
  start line printed on every try the case moves from green to red).
- The case stays under 5 s; `task check` is green.

## Context

Note 2 of review-result:adversary-closed-port-pass-1 (origin undecided: the story of that
pass changed only where the case gets its address). There before that story. It waits until
the planned waves are delivered.

## Scope

Written after the build, from the diff against `main` at a1881d9: every path below is read
from it (cited).

3 files of cases (no file under a `src/` directory changes):

- `crates/inbox-cli/tests/bridge.rs`: the case
  `a_bridge_started_before_the_service_answers_says_so_and_keeps_trying` gets a listener of
  its own (`Unanswered`, with `read_request`) that accepts each connection, closes it without
  an answer and counts the connections; the case counts the lines of the bridge after the
  3rd connection.
- `crates/inbox-cli/tests/adversary_early_bridge.rs` (new): the 2 cases of the adversary pass
  (each try of a bridge that waits is its open; the bridge pauses between its tries).
- `crates/inbox-cli/tests/main.rs`: the `mod` line of that file.

Read and not changed: `crates/inbox-cli/src/wake.rs` (`say_once`, the pause between tries),
`crates/inbox-cli/tests/adversary_wake_2.rs` (the stand-in the listener is modelled on),
`crates/inbox-cli/tests/adversary_bridge.rs` (the 2 cases that keep the closed port),
`crates/inbox-cli/tests/support`.

Not touched: every product crate, `ess/`, `generated/`, every document, `CHANGELOG.md`,
`AGENTS.md` beyond the row of the test step that every story record writes.

## Decisions

- The story was filed by an adversary pass of the bridge and accepted by the operator
  (2026-11-24). No panel of plan critics read the story.
- How the case waits. The options were: (A) a listener of the case that accepts each
  connection, closes it without an answer and counts the connections, so the case counts the
  lines after the 3rd try; (B) a fixed wait before the count, as
  `adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`
  has it (1600 ms). A is taken: the file's rule is that a bounded wait only ends a hang, and a
  fixed wait is as long on a fast host as on a slow one.
- The count is taken after the 3rd connection, not the 2nd. The line of a try is written
  after that try failed and before the pause that follows it, so the 3rd connection follows
  the write of the line of try 2. With the wait on 2 connections and the line printed on
  every try the case was green 3 of 20 times in the adversary pass; with the wait on 3, red
  40 of 40.
- The wait also ends when the bridge ended, so a bridge that gives up fails the case at once
  on its `running` assertion and not at the bound of 10 s.
- The case no longer uses a closed port. A refused connect and a connection closed without
  an answer are both a failed transport and reach the same line through the same `say_once`;
  the refused connect is still held by 2 cases of `adversary_bridge.rs`.
- The listener lives in `bridge.rs`, with a copy of the request reader of
  `adversary_wake_2.rs`: `tests/support` is not changed by this story.
- The 2 findings of the adversary pass were there before the story (2 mutants of the product
  that every case of the crate let live: a failed open taken as the start done; the pause
  between tries removed). The pass wrote one case for each; both are taken into this change,
  since each is test code and kills its mutant. The options were to take them (taken) or to
  file 2 draft stories.
- No line of the product changes; no document and no `CHANGELOG.md` entry.
- Not decided here: whether the case with the fixed wait of 1600 ms gets the same listener.
  That would move the listener into `tests/support`.

## Measured

- Before the change, on a1881d9 on the build node, with the line of a try printed on every
  try (`if *failures == 0` replaced by `if true` in `say_once`, not committed): the case is
  green in 0.027 s; `adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`
  is red in 1.631 s.
- Round 1: `git diff --stat` names `crates/inbox-cli/tests/bridge.rs`, 95 lines added, 11
  removed. With that mutant the case is red in 0.340 s (`left: 3, right: 1`: 3 lines for 3
  tries); without it green in 0.341 s. The case 20 of 20 green in a row, 0.331 to 0.390 s
  (0.03 s before: the wait for 3 tries adds the pauses of 100 and 200 ms). No case over 5 s.
- Round 1, the wait itself: with the wait on 1 connection and the mutant in, 4 of 5 green
  (the defect of the old case) and 1 red with 0 lines. A listener that answers `200` with an
  empty object: red in 0.082 s on the `running` assertion, not at the bound.
- Round 1, the crate: `cargo nextest run -p inbox-cli --locked` twice, 458 passed both times
  (12.1 s and 12.3 s). `task check` on the build node: 1159 passed, 8 skipped, the test step
  19 s, exit 0.
- Adversary pass 1 (review-result:adversary-early-bridge-case-pass-1): nothing introduced by
  the story; 2 findings that were there before, each a mutant of the product that all 458
  cases of the crate let live, and one case written for each (both taken):
  `adversary_early_bridge::each_try_of_a_bridge_that_waits_for_the_service_is_its_open` (red
  when a failed open is taken as the start done; 0.514 s) and
  `adversary_early_bridge::a_bridge_that_waits_for_the_service_pauses_between_its_tries` (red
  when the pause between tries is removed; holds only a lower bound of 700 ms for the 4th
  try; 20 of 20 green, 0.911 to 1.137 s).
- Adversary pass 1, the story's case: with the mutant of the story red 20 of 20 alone (0.339
  to 0.437 s) and 20 of 20 with a second job running in the same tree (0.331 to 0.469 s),
  each time with 3 lines. Without mutants 50 of 50 green, 0.331 to 0.355 s. With the wait on
  2 connections and the mutant in: green 3 of 20; on 1 connection: green 19 of 20. The line
  never printed: red with 0 lines. The first pause made 10 s: red at 10.0 s with the line of
  the bound. A bridge that gives up after 2 tries: red in 0.140 s on the `running` assertion.
  The crate at a1881d9 12.8 s and 12.1 s, with the change 12.2 s and 12.6 s, with the 2 cases
  of the pass (460 cases) 12.1 s.
- Adversary pass 1, seen beside the story: with `--no-fail-fast` the exit status of a
  `--stress-count` run of cargo-nextest follows its last iteration (19 passed, 1 failed,
  exit 0); the summary line of a stress run is what counts.
- The pass did not reach: a look for a bridge process left after a red run (the cases run
  on the build node); a run of the whole crate under 3 of its mutants.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T18:17:14Z to 2026-11-24T18:17:53Z, on the build
  node of the pool: exit 0, `1161 tests run: 1161 passed, 8 skipped`, the test step 18 s of
  its bound of 120 s.
