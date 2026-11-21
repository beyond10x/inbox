---
format: aep.planning-md/3
id: review-result:adversary-service-acceptance-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:service-acceptance (5e5058c): 7 findings (2 warnings, 5 notes), 4 red cases, 8 of 12 changed services green in every lane'
relations:
- reviews: story:service-acceptance
revision: 1
---
7 findings (2 warning, 5 note). No blocker.

```
unit: story:service-acceptance, commit 5e5058c on f883cbd, worktree inbox-s2-w5-acc (findings cover HEAD plus my 2 untracked test files)
verdict: CONFIRMED
cases: executed 330→334, red 4 (plus 1 ignored acceptance case: passes on the real binary, fails on a mutant)
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the assigned scratch directory, part 6)
needs-coordinator: none
```

## 1. Diff stat

`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short` shows only 2 untracked test files:
- `crates/inbox-conformance/tests/adversary3.rs`
- `tests/acceptance/tests/adversary.rs`

Nothing is staged or committed.

## 2. Cases added (each run alone first)

**`crates/inbox-conformance/tests/adversary3.rs`**: 4 cases, all fail now. First run: `cargo test -p inbox-conformance --test adversary3`.

- `a_declared_outcome_under_a_status_the_contract_does_not_give_it_fails_the_scenario`
  - A stand-in service answers OpenInbox with HTTP 500 (also tried 409) and a body naming `opened`.
  - Output: `an opened answered with HTTP 500 is recorded as ScenarioResult { id: "probe", status: Passed, message: None }`
- `the_gate_refuses_a_listed_reason_that_is_not_the_reason_of_the_run`
  - Takes the 73 ids of the real `unsupported.tsv` and sets every reason to "the service was not running".
  - Output: `the gate passes 73 listed scenarios whose every reason is 'the service was not running'`
- `ess_reads_a_row_path_with_an_empty_member_and_holds_null_there`
  - Output: `the runner does not interpret what ESS interprets: Some("step 1: the row path 'signature..key_id' names an empty member, which no row carries")`
- `ess_refuses_coverage_lists_out_of_order`
  - Output: `the runner reads a coverage block ESS refuses`

**`tests/acceptance/tests/adversary.rs`**: 1 case, ignored like the 3 checks.

- `after_a_restart_the_second_repost_is_answered_duplicate`
  - Against the real binary it passes: `test result: ok. 1 passed`.
  - Against the duplicate-check-dropped mutant it fails:
    ```
    left: ["posted", "posted"]
    right: ["posted", "duplicate"]
    ```
  - The 3 existing checks against that same mutant: `test result: ok. 3 passed`.

## 3. Suite run, after the cases existed

Command: `CARGO_INCREMENTAL=0 cargo test -q --locked --workspace --no-fail-fast`, exit 101.

- Totals: 330 passed, 4 failed, 9 ignored. The 4 failures are the adversary3 cases above.
- Last lines: `error: 1 target failed: -p inbox-conformance --test adversary3`.
- A first run without `--no-fail-fast` stopped at adversary3 after 107 cases. The full run is the one counted.

## 4. Findings

**F1. warning, CONFIRMED, introduced.** `crates/inbox-conformance/src/wire_target.rs:472` (`fn executed`)
- The wire target reads the outcome from the body only and never checks the HTTP status.
- `generated/openapi` gives each outcome its status (posted 202, duplicate 409, and so on). The `inbox` client exits 3 on a 500.
- So the serve lane records `passed` for an answer that a client of the same service sees as a failure.
- What reaches it: any server or backend change that maps statuses wrongly. The serve lane is the stage's only end-to-end check of the HTTP contract.
- Fix: hold the status to the contract's status for that outcome.

**F2. warning, CONFIRMED, introduced.** `tests/acceptance/tests/checks.rs:214`
- Check 3's claim "Pending holds the message once" cannot fail. `MemoryBackend` keys each message by its id, so a second post replaces the row instead of adding one.
- With both duplicate guards of `PostMessage` removed (mutant), the check still passes.
- Only the serve lane's `repost-after-restart-arrives-once` notices.
- Fix: assert that the second repost answers `duplicate`, as in `tests/acceptance/tests/adversary.rs`.

**F3. note, CONFIRMED, introduced.** `crates/inbox-conformance/src/wire_target.rs:811` (union read) and the 20 serve-lane passes
- 8 of the 12 mutants stay green in all 3 lanes: core, serve and acceptance. Each is caught only by its own crate's `cargo test`, so `task check` still goes red.
- Why the serve lane cannot see them:
  - every request it sends is validly signed;
  - every key and delegation it writes is valid until 9999;
  - its `authority-invalid` case withholds the delegation entirely, so the pair check is never hit;
  - the union read hides the read rule;
  - none of the 20 passed scenarios contains `expect_not_granted`.

| mutant | core | serve | acceptance | crate test |
|---|---|---|---|---|
| read rule dropped | green | green | green | red |
| request signature unchecked | green | green | green | red |
| request window 60 s → 10 y | green | green | green | red |
| envelope signature unchecked | green | **red** (2) | green | red |
| envelope key/`from` binding unchecked | green | green | green | red |
| delegation lapse unchecked | green | green | green | red |
| delegation pair unchecked | green | green | green | red |
| key lapse unchecked | green | green | green | red |
| Messages/Due served to a client | green | green | green | red |
| duplicate checks dropped | **red** | **red** | green | red |
| sweep never runs | green | green | **red** (check 2) | red |
| `MessageHandled` left out of `published` | green | **red** (3) | green | **green** |

**F4. note, CONFIRMED, introduced (exposed: the list was empty at base).** `crates/inbox-conformance/src/gate.rs:286`
- The gate checks only the ids in `unsupported.tsv`. The reason column is never compared with anything.
- Today all 73 reasons equal the run's messages. Nothing keeps it that way.
- Fix: the gate reads the results document and compares each scenario's message with its listed reason.

**F5. note, INFEASIBLE, introduced.** `crates/inbox-conformance/src/suite.rs:729`
- A row path with an empty member (`signature..key_id`, `signature.`) makes the scenario unsupported. ESS 0.96.0 interprets it and holds `null` there (measured).
- Synthesis never writes such a path, and the error goes the safe way: the core gate fails.

**F6. note, INFEASIBLE, introduced.** `crates/inbox-conformance/src/suite.rs:38`
- The doc comment says the runner reads `coverage` "as ESS's own reader does". Measured, the runner accepts 8 suites that ESS refuses:
  - `authored`/`generated` lists out of order;
  - a `covered_by` that is not an id, or names no authored scenario;
  - a `refused` entry or `selection` with missing fields;
  - empty `authored_sources`;
  - a count written `24.0`;
  - an empty events catalog. This one silently drops every `absent_events` check; that code predates the unit.
- No effect on the pipeline: `ess verify conform report` refuses all 8 downstream.
- Fix: correct the doc comment, or add the checks.

**F7. note, INFEASIBLE, introduced.** `crates/inbox-conformance/src/wire_target.rs:378`
- The story's Outcome says a step expecting `unknown-option`/`shape-mismatch` from an envelope with no such mismatch is unsupported. The wire target has no such reason: it sends the envelope as stated, so such a step would fail, not be unsupported.
- No scenario in the suite reaches it: all 3 such steps carry the mismatch and pass.

## 5. What held

- `unsupported.tsv` equals the run's messages: 73 of 73. Reason counts are 64, 35, 9, 7.
- The 10 unsupported scenarios I checked by hand list reasons that are true of them.
- `Messages/grant/read/denied` and `Inboxes/grant/read/denied` are honestly unsupported: a request cannot be the Sweeper, and a request with no actor gets 401, not the declared not-granted refusal.
- Reading `/65`: ESS and the runner agree on 18 altered suites, including coverage absent, a 9th member, a missing member, the `knowledge` value, counts off by one, an extra or missing count, a scenario in neither list, in both or not in the suite, a `refused` entry that is text, and format `/52`.
- Dotted row paths agree with ESS on 9 cases: nested present, absent, `null`, through text, through a list.
- Check 2 timing: 10 runs, all passed, 3.84 to 4.03 s.
- Each task alone on a clean `.out/`, run in the tree copy: `conform-interpreted`, `coverage`, `conform` and `acceptance` all exit 0 with the claimed lines. Serve lane: 20 passed, 73 unsupported, `conformance_status: failed`.
- The `awk` count and pipe: `set: [pipefail]` is on. A `FAILED` line counts only the passed tests; a compile error counts 0.
- Gate: one line more or less, lines swapped, the core list given for the serve report and the reverse, a wrong `--implementation`, the same id twice, an empty reason, and the status rule (`inconclusive` and `failed` refused for the core report). I checked these by reading the code, not by running them.
- No `inbox serve` left running: checked by binary path and by command line.
- The diff has no `unsafe`, no home path, name or secret, and no edit under `generated/`, `ess/` or `docs/`.
- No permission check denied a tool call.

## 6. Paths written outside the worktree

The assigned scratch directory `<scratch>/w5/service-acceptance/adversary/`, 38 MB:
- scripts: `dump.py`, `externals.py`, `kinds.py`, `suite_diff.py`, `dotted.py`, `mutate.py`, `lanes.sh`, `all_mutants.sh`, `timing.sh`, `tasks.sh`
- logs: `*.log`
- `ess/` (the base report, `variants/`, `dotted/`)
- `m/` (`build0.log`, `build-dup.log`, `summary.log`, `runs/<mutant>/` with results, reports and lane logs)

The tree copy `m/tree` and its `target/` are deleted. Inside the worktree, the test runs left their own directories under `target/tmp/` (`adversary3-*`).

## 7. Findings block

```findings
- file: crates/inbox-conformance/src/wire_target.rs
  line: 472
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'The wire target ignores the HTTP status of a command answer, so a service answering 500 or 409 with the expected outcome in its body passes a scenario the inbox client of that service sees fail.'
- file: tests/acceptance/tests/checks.rs
  line: 214
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'Check 3 asserts Pending holds the message once, which a store keyed by message id makes true without any duplicate check; with both PostMessage duplicate guards removed all 3 checks pass.'
- file: crates/inbox-conformance/src/wire_target.rs
  line: 811
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Eight of twelve service mutants (request signature, request window, key-to-from binding, delegation pair and lapse, key lapse, read rule, Messages and Due served to a client) stay green in the core lane, the serve lane and the acceptance checks; only crate unit tests catch them.'
- file: crates/inbox-conformance/src/gate.rs
  line: 286
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'The gate checks only the ids of unsupported.tsv, so a list whose 73 reasons are all false still passes.'
- file: crates/inbox-conformance/src/suite.rs
  line: 729
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'A row path with an empty member makes the scenario unsupported where ESS 0.96.0 interprets it and holds null, contrary to the claim that dotted paths are compared as ESS compares them.'
- file: crates/inbox-conformance/src/suite.rs
  line: 38
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'The coverage reader is documented as reading the block as ESS does but accepts eight measured suites ESS refuses, among them unsorted origin lists and a covered_by naming no authored scenario; ess verify conform report refuses them downstream.'
- file: crates/inbox-conformance/src/wire_target.rs
  line: 378
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'The story says a step expecting unknown-option or shape-mismatch from an envelope without that mismatch is unsupported, but the wire target sends the envelope as stated and has no such reason; no scenario of the suite reaches it.'
```
