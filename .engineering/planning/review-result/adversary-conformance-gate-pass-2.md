---
format: aep.planning-md/3
id: review-result:adversary-conformance-gate-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:conformance-gate (3ff527d): 6 findings, 4 red cases'
relations:
- reviews: story:conformance-gate
revision: 1
---
6 findings (4 introduced, 0 of them regressions; 2 pre-existing). 4 red cases.

Covers `3ff527d` on `impl/conformance-gate` (base `c582901`), plus the uncommitted `crates/inbox-conformance/tests/adversary2.rs`.
`git status --short`: `?? crates/inbox-conformance/tests/adversary2.rs`. `git diff --stat` is empty, so no tracked file was touched.
Cases executed: 39 → 43 (lib 4 + adversary 18, 1 ignored + runner 17 = 39, from the implementor's counts and re-measured with adversary2 deselected; then + adversary2 4). Red: 4.

## Findings

**1. `satisfies` passes on a view with no rows.** `crates/inbox-conformance/src/runner.rs:535`. CONFIRMED / INFEASIBLE, introduced (by the unit's first commit, not by the correction), warning.
- ESS 0.96.0 (`runner.rs` `satisfies`) requires "Every row satisfies the predicate, and there is at least one". An empty view is `Unsatisfied("the view holds no rows")`.
- The runner loops over the rows and returns `Ok` when there are none. The crate doc says it compares `satisfies` "as the ESS 0.96.0 runner compares it".
- Case `adversary2.rs::ess_satisfies_needs_at_least_one_row`, run alone, exit 101:
  `panicked at crates/inbox-conformance/tests/adversary2.rs:63:5: ScenarioResult { id: "probe", status: Passed, message: None }  left: Passed  right: Failed`
- What reaches it: all 30 `satisfies` steps of the suite are the only expectation on their read (`<scratch>/satisfies.py`). No wrong core found: the `value_invariants` scenarios on the same 5 views now need a row.
- Fix: fail when `rows` is empty.

**2. The gate binds a report to the suite's bytes only.** `crates/inbox-conformance/src/gate.rs:174` (`bound`), `src/main.rs:95`. INFEASIBLE, introduced (by the correction's answer to F5), note.
- The report below has the suite's digest, its counts agree with its lists, its total adds up and its status is `inconclusive`. The gate passes it although it accounts for 1 of the suite's 2 scenarios, or lists one id twice.
- `adversary2.rs::gate_refuses_a_report_that_does_not_account_for_the_scenarios_of_its_suite`, exit 101. The control with both ids exits 0; then:
  `1 scenarios: 1 passed ... conformance_status: inconclusive` and `a report of 1 scenario passes the gate on a suite of 2  left: Some(0)  right: Some(1)`
- `adversary2.rs::gate_refuses_a_report_that_lists_one_scenario_twice`, exit 101:
  `a report passing `a` twice and never naming `b` passes the gate  left: Some(0)  right: Some(1)`
- Probe: `target/debug/inbox-conformance gate --suite .out/conform/suite.json --report .out/conform/report.json ...` exits 0. That report is the interpreted target's (`implementation: interpreted`) and sits in the same directory.
- What reaches it: nothing in `task conform`. `ess verify conform report` refuses missing, duplicate and foreign results, and the task always passes the freshly written `core-report.json`.
- Fix: read the suite's scenario ids and require the report's ids to be exactly those, once each; check `implementation`.

**3. The correction's new lines are mostly untested.** `crates/inbox-conformance/src/invariants.rs:405` and others. CONFIRMED, introduced (by the correction), warning.
- Each mutant below makes one comparison always hold, in a tree copy. `cargo test -p inbox-conformance --lib --test runner --test adversary` stays at exit 0 for each. Log: `<scratch>/mutants/runner.out`, script `runner_mutants.py`.
- gate: the digest check in `bound`, the status check in `bound`, `passing_status` always `inconclusive`, and `main` dropping `gate::bound` altogether. The F5 binding has no test at all.
- suite: `asserted == 0` no longer broken; a value type that is not a newtype over text admitted; unknown keys in a struct field, a newtype, an invariant, an enum and a struct declaration.
- invariants:
  - the three closure checks (undeclared, unreached, Binary64);
  - shape failures dropped (a malformed non-witness row passes);
  - required member missing; primitive, `Text`, `Integer` and `Boolean` admits;
  - list not an array; prefix; struct not an object; undeclared struct member; enum label; `represents_absence`.
- runner: the witness taken last instead of first; a witness event without the field passes; subject selection ignoring the value; several subject rows taking the first; subject selection reading absent as null; unchanged compared by `view_equal`; complete/non-complete mismatch.
- Equivalent mutants, not counted: each of the two duplicated total checks and the two duplicated `broken` checks alone (removing both is killed); `inv-terminal-not-text` and `run-observe-broken-ok` (unreachable after `shape`).
- The behaviour itself holds: my probes of the same paths in a copy answer as ESS does (see "Held"). What is missing is a case that pins it.

**4. A key stated twice in a step is silently ignored.** `crates/inbox-conformance/src/suite.rs:942` (`member` takes the first). INFEASIBLE, introduced, note.
- `Value::Object` keeps a duplicate key, and `only` admits it. ESS reads the suite with serde, which refuses a duplicate field.
- `adversary2.rs::a_key_stated_twice_in_a_step_is_not_ignored`, exit 101:
  `a step stating `expectation` twice is admitted and its second expectation ignored: Ok([Query {..}, ExpectView { .., expectation: Counts { at_least: Some(0), at_most: None } }])`
- What reaches it: nothing; the synthesizer writes no duplicate keys.

**5. Fields stored wrongly after `Expire` decides on silence pass everything.** `crates/inbox-core/src/obligations.rs:267`. CONFIRMED, pre-existing (the core and `ess/` are unchanged by the unit), warning.
- Five mutants, each adding one line after `decided_option` is written: `decision = None`, `default_on_silence = None`, `body` emptied, `created = keep_until`, `thread = message_id`.
- Each gives `task conform` 92 passed (gate 0), and `cargo test -p inbox-core` exits 0. `conform-interpreted` never runs the core. Log: `<scratch>/mutants/core.out`.
- The missing scenario: after decided-on-silence, a read of the message row that claims its unchanged fields (body, created, decision, default_on_silence, and a thread that differs from the message id). `decision-taken-on-silence` step 22 claims only answered_by, decided_option, expires, message_id and state, and uses thread == message_id.

**6. `Expire` publishing its event twice passes everything.** `crates/inbox-core/src/lib.rs:107`. CONFIRMED, pre-existing, note.
- Mutant `events-doubled-expire-only` duplicates the last event in `handle_with_events` when the command is Expire. `task conform` 92 passed; `cargo test -p inbox-core` exits 0.
- The same doubling on PostMessage or OpenInbox is killed, through the capture error "published ... 2 times".
- The missing scenario: Expire held to one `DecisionTakenOnSilence` / `MessageExpired`. No step of suite/52 bounds the number of occurrences.

## Pass 1

- F1: closed. Probes in a copy (`<scratch>/probe.log`): 0 rows → failed; second row with letter `C`, an extra member or a missing `text` → failed; witness is the first MessagePosted, also within one command → failed; two witness rows, one with an occurrence → passed. All as ESS. The untested shape lines are finding 3.
- F2: closed. Subject by `state` with 2 rows → failed, with 1 → passed; another row changing → passed; null→absent and an added member → failed. Not checked: `admit_row` on a complete snapshot (documented in `lib.rs`; the Inboxes rows are held by 50 `contains`).
- F4: closed for top level, provenance, scenario, step, predicate, expected event and shape, view claim and the `value_invariants` keys. Remaining gap: finding 4.
- F5: partly. The counts, total, digest and status checks work, but none of the binding has a test (finding 3) and it binds only bytes (finding 2).
- F6: closed. Of pass 1's 26 mutants rerun on `3ff527d`, all 22 that still apply are killed by the crate suite (`<scratch>/mutants/pass1.out`). The 4 that no longer apply are replaced code, covered by the new campaign (`inv-count`, `inv-alphabet`, `run-unchanged` killed).

## Held

- `slot task conform`, twice in a row and after a stale interpreted `report.json`: `92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported`, `conformance_status: inconclusive`, exit 0.
- Regression count: every scenario passed before and after, so all 1,800 steps ran both times (execute 332, expect_outcome 324, query 248, expect_view 216, …). No comparison got weaker. `value_invariants` (15) and snapshots (8+6) got stricter. The one relaxation is "exactly one identity row" → ESS's "an eligible row with an occurrence".
- Core mutants killed by the gate: answered_by Principal, state left unchanged, all events doubled, PostMessage events doubled, inbox list doubled, an extra message row, and 8 dropped grants (Expire, Due and Messages for the Sweeper; Settled, Unanswered, Replies, Collect, CloseInbox and Thread for the Client).
- The gate exits 2 on a missing `--suite`; a changed suite gives a digest mismatch; an unknown `conformance_status` fails.
- Taskfile has no comment and passes `--suite`. The AGENTS.md row matches what the task does. No `unsafe`; no name, e-mail or home path in the diff; no edit under `generated/`, `ess/` or `crates/inbox-core/`.
- `cargo fmt --check` exit 0; `cargo clippy -p inbox-conformance --all-targets --locked -- -D warnings` exit 0.
- No tool call was denied.

Written outside the worktree: only the scratch directory the brief named (`<scratch>` = `.../w3/conformance-gate/adversary2`): scripts (`census.py`, `satisfies.py`, `views.py`, `show.py`, `runner_mutants.py`, `core_mutants.py`, `probe.rs`, `red_alone.sh`, `suite.sh`), logs (`red_alone.log`, `red_alone_2.log`, `suite.log`, `probe.log`) and `mutants/`. Both tree copies are deleted. Builds of my cases also wrote under the worktree's own `target/`.

```findings
- file: crates/inbox-conformance/src/runner.rs
  line: 535
  category: contract-drift
  severity: warning
  origin: introduced
  message: 'satisfies passes on a view with no rows, where ESS 0.96.0 requires at least one row; 30 suite steps are satisfies alone on their read'
- file: crates/inbox-conformance/src/gate.rs
  line: 174
  category: acceptance
  severity: note
  origin: introduced
  message: 'the gate binds a report to the suite bytes only: a report with the suite digest that accounts for 1 of 2 scenarios, lists an id twice, or is the interpreted target report passes'
- file: crates/inbox-conformance/src/invariants.rs
  line: 405
  category: mutant
  severity: warning
  origin: introduced
  message: 'most lines the correction added survive mutation under cargo test -p inbox-conformance: the whole gate binding, the shape checks, closure, witness order, subject selection and unchanged exactness'
- file: crates/inbox-conformance/src/suite.rs
  line: 942
  category: acceptance
  severity: note
  origin: introduced
  message: 'a key stated twice in a step is admitted and the second value silently ignored, against nothing the suite states is silently ignored'
- file: crates/inbox-core/src/obligations.rs
  line: 267
  category: mutant
  severity: warning
  origin: pre-existing
  message: 'after decided-on-silence a core that clears decision or default_on_silence, empties body or rewrites created or thread passes task conform and cargo test -p inbox-core; the suite lacks a read of those fields after silence'
- file: crates/inbox-core/src/lib.rs
  line: 107
  category: mutant
  severity: note
  origin: pre-existing
  message: 'Expire publishing its event twice passes task conform and cargo test -p inbox-core; no scenario holds Expire to one occurrence'
```
