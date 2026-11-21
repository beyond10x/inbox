---
format: aep.planning-md/3
id: review-result:adversary-conformance-gate-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:conformance-gate (df1ecc5): 8 findings, 8 red cases'
relations:
- reviews: story:conformance-gate
revision: 1
---
8 findings (6 introduced, 2 pre-existing)

```
unit: story:conformance-gate, df1ecc5 on impl/conformance-gate (base c582901), worktree plus the uncommitted crates/inbox-conformance/tests/adversary.rs
verdict: CONFIRMED
cases: executed 20→41, red 8
origin: introduced 6 / pre-existing 2 / undecided 0
wrote-outside-worktree: 1 directory, the scratch directory the brief assigned (<scratch> below)
needs-coordinator: none
```

## 1. Diff

`git --no-pager diff --stat` is empty. `git status --short` shows one entry: `?? crates/inbox-conformance/tests/adversary.rs`, an untracked test file. No implementation, Taskfile.yml or AGENTS.md path was touched.

## 2. Cases added (`crates/inbox-conformance/tests/adversary.rs`, 21 cases)

Red cases. Each was run alone with `cargo test -p inbox-conformance --test adversary -- --exact <name>` before the suite, and each exited 101:

- `ess_a_snapshot_that_finds_no_row_does_not_hold_the_subject_unchanged` (:105): `left: Passed right: Failed`
- `ess_value_invariants_need_an_occurrence_in_the_witness_row` (:142): `left: Passed right: Failed`
- `ess_value_invariants_hold_of_every_row_of_the_view` (:158): `left: Passed right: Failed`
- `ess_contains_null_at_the_top_level_needs_the_member_published` (:173): `left: Passed right: Failed`
- `an_unknown_top_level_key_of_the_suite_is_not_ignored`: `a suite with the unknown top-level key `fixtures` is admitted and its scenarios run`
- `an_unknown_provenance_key_is_not_ignored`: `... unknown provenance key `scenario_clock` is admitted and its scenarios run`
- `the_report_passed_count_must_agree_with_the_passed_list`: `counts.passed 3 against outcomes.passed of 1 is read`
- `the_report_total_must_be_the_sum_of_its_outcomes`: `a report of 92 scenarios with 1 accounted for passes the gate`

13 cases are green now. Each turns red on the runner mutant that stops checking the thing it names: 11 `catches_*` cases, plus `catches_a_skipped_scenario` and `catches_a_run_that_passed_nothing_where_every_unsupported_one_is_listed`.

## 3. Suite run, after the cases existed

`CARGO_INCREMENTAL=0 cargo test -p inbox-conformance --locked --no-fail-fast` exited 101: lib 4 passed; tests/adversary.rs 13 passed, 8 failed (the 8 above); tests/runner.rs 16 passed.

`cargo clippy -p inbox-conformance --all-targets --locked -- -D warnings` exits 0. The file is rustfmt-clean.

## 4. Findings (they cover df1ecc5)

F1. `value_invariants` is weaker than ESS. `crates/inbox-conformance/src/runner.rs:572`, CONFIRMED, introduced, warning. The runner checks only the witness row's values. A witness row with no occurrence (decision null, absent, or an empty options list) passes. ESS 0.96.0 `value_invariant.rs` `Contract::observe` checks every row of the view, and returns Unsatisfied "no selected row holds an actual occurrence". This touches 15 scenarios. What reaches it: no wrong core found; the mutant that drops the stored `decision` is killed by a `contains` step on `Due`.

F2. A snapshot that selects no row passes. `runner.rs:485`, with the snapshot accepting an absent row at :462. CONFIRMED, introduced, warning. `snapshot_subject` and `expect_subject_unchanged` pass when no row matches, both times (`(None, None) => true`). ESS `snapshot_subject` requires "exactly one subject row", and errors on an empty subject. What reaches it: on the 0.96.0 suite every snapshot (8) comes right after a `contains` on the same subject, so a core cannot use it today.

F3. Absent and `null` are treated as one value at the top level. `crates/inbox-conformance/src/value.rs:69`. CONFIRMED, introduced, note. The runner treats them alike for a top-level field in `contains`, `excludes` and subject matching. ESS `fields_match` compares a plain field exactly, and reads absent as null only on a dotted leaf path. The suite has 160 top-level `null` claims (`answered_by`, `decided_option`, `refusal`, `reply`). A core that leaves those members out passes this gate and fails ESS's runner.

F4. Unknown top-level and provenance keys are silently ignored. `crates/inbox-conformance/src/suite.rs:292`. INFEASIBLE, introduced, warning. This contradicts the unit's claim that a key the runner does not know makes the scenario unsupported, and the crate doc's "nothing the suite states is silently ignored". What reaches it: nothing today; the suite version is pinned to /52, and that suite has `provenance`, `events`, `scenarios` and no other top-level key.

F5. The gate does not check two counts. `crates/inbox-conformance/src/gate.rs:101`. INFEASIBLE, introduced, note. It does not compare `counts.passed` with `outcomes.passed`, nor `total` with the sum of the categories. It also ignores `conformance_status` and `suite.digest`. A hand-written report with total 92 and passed 1 passes the gate (exit 0). This contradicts the claim "fails on counts that disagree with the id lists". What reaches it: nothing in `task conform`; `ess verify conform report` refuses a missing, duplicate, foreign or unknown-status result and a digest mismatch (all exit 2), so the task stops before the gate.

F6. The crate's own suite misses 12 runner mutants. Starting at `runner.rs:303`. CONFIRMED, introduced, warning. Each mutant makes one comparison always hold, and the unit's 20 cases stay green: error name (two variants), event name, payload shape, `expect_no_events`, not-granted on a command that ran, not-granted on a read that was served, `excludes`, `subject_unchanged`, the fields of `complete_subject_unchanged`, the gate's `skipped` check, the gate's `passed == 0` check. `task conform` cannot catch a more lenient runner either, because the correct core still passes. Each mutant is caught by one `catches_*` case. Log: `<scratch>/mutants/runner1.out`, from `runner_mutants.py`.

F7. A decision that was read is deleted at expiry in place of decided on silence, and nothing notices. `crates/inbox-core/src/obligations.rs:252`. CONFIRMED, pre-existing, warning. Mutant `expire-silence-not-after-read-2` drops `MessageState::Read` from `open`. `task conform` passes, `cargo test -p inbox-core` exits 0, and `conform-interpreted` never runs the core. The specification's scenarios lack one: a decision with `default_on_silence` that is MarkRead, then expired, should give `decided-on-silence`. The specification's guard is `state: [Sent, Delivered, Read]`. `decision-taken-on-silence` only does MarkDelivered.

F8. Reversing the order of PostMessage's 3 external refusals passes everything. `obligations.rs:165`. CONFIRMED, pre-existing, note. `task conform` and `cargo test -p inbox-core` both pass, and `conform-interpreted` does not run the core. No conformance run can see this order: StepContext, like ESS's forced outcome, takes one branch. The core's tests have no case where two externals hold at once, for example an unknown option and a bad signature.

## 5. What held

- Core mutants killed by `task conform` (29 kinds): each PostMessage input guard removed; `already-expired` Le changed to Lt; unknown-option never asked; signature and authority branches crossed; duplicate, duplicate-key and inbox-closed each removed; duplicate moved after inbox-closed; duplicate-key answering duplicate; MessagePosted `to`, `kind` and `thread` wrong; stored thread, decision, reply_shape, default_on_silence, authority, on_behalf_of, created and dedup_key each wrong; Expire's not-due removed or changed from Gt to Ge; Expire removing nothing; keep_until, decided_option and answered_by each dropped; DecisionTakenOnSilence `to` wrong; the backend hiding a message row, doubling one, or hiding closed inboxes.
- Core mutants that pass `task conform` but are caught by `cargo test -p inbox-core`, which `task check` runs: no-authority after already-expired, externals before the input guards, externals after the row refusals, duplicate after unknown-inbox, duplicate-key after inbox-closed.
- What the core cannot get wrong because the generated code fixes it: a wrong error name, an event emitted twice, the grants.
- `expire-silence-also-for-non-decision` survives everything, but is equivalent in the service: the unknown-option external refuses a `default_on_silence` without a decision.
- The gate refuses a report/1, a document that is not JSON, a tsv with a comment or an empty line, a listed scenario that passed, and all-unsupported or 0-passed runs.
- Suite versions /51 and /53 exit 2. An unknown step key, expectation key, predicate operator or compare operator makes the scenario unsupported.
- Reversing the scenario order, or running every scenario twice: 0 not passed, so no state carries over between scenarios.
- `run` exits non-zero on any failure, so no stale results or report can reach the gate.
- The expansion of `absent_events` and `unpublished_events` matches ESS `expand_steps`.
- The diff has no person's name, home path, `unsafe`, or comment in a manifest or the Taskfile, and no edit under generated/, ess/ or inbox-core.
- The task adds about 5 s for a second synthesis whose output is byte-identical to the one `conform-interpreted` already does, and 0.08 s for the run.

## 6. Written outside the worktree

Everything is under the scratch directory the brief named: scripts (`probe_gate.py`, `core_mutants.py`, `runner_mutants.py`, `reorder.py`, `refuse.py`, `red_alone.sh`), logs (`conform-1.log`, `red_alone.log`, `suite.log`, `suite2.log`, `clippy.log`, `probe_gate.log`), directories `gate-probe/` and `mutants/` (per-mutant logs, `core-mutants.tsv`, `run1-3.out`, `runner1.out`, `pristine/`, `pristine-runner/`). Both tree copies and the reordered suite copies are deleted. No tool call was denied.

## 7. Findings block

```findings
- file: crates/inbox-conformance/src/runner.rs
  line: 572
  category: contract-drift
  severity: warning
  origin: introduced
  message: 'value_invariants checks only the witness row and passes one holding no occurrence, where ESS checks every row and fails without an occurrence'
- file: crates/inbox-conformance/src/runner.rs
  line: 485
  category: boundary
  severity: warning
  origin: introduced
  message: 'a subject snapshot that selects no row and still selects none passes expect_subject_unchanged, where ESS requires exactly one subject row'
- file: crates/inbox-conformance/src/value.rs
  line: 69
  category: contract-drift
  severity: note
  origin: introduced
  message: 'a top-level null claim in contains matches a row that leaves the member out, where ESS compares a plain field exactly'
- file: crates/inbox-conformance/src/suite.rs
  line: 292
  category: acceptance
  severity: warning
  origin: introduced
  message: 'an unknown top-level or provenance key of the suite is ignored and its scenarios run, against the claim that an unknown key makes a scenario unsupported'
- file: crates/inbox-conformance/src/gate.rs
  line: 101
  category: acceptance
  severity: note
  origin: introduced
  message: 'the gate never compares counts.passed with outcomes.passed nor total with the sum of outcomes, so a report of 92 with 1 passed is accepted'
- file: crates/inbox-conformance/tests/runner.rs
  line: 1
  category: mutant
  severity: warning
  origin: introduced
  message: 'twelve runner mutants (error name, event name, shape, no_events, not-granted on a command or a read, excludes, subject_unchanged, complete fields, gate skipped, gate passed zero) leave the unit suite green'
- file: crates/inbox-core/src/obligations.rs
  line: 252
  category: mutant
  severity: warning
  origin: pre-existing
  message: 'a core that deletes a read decision at expiry instead of deciding it on silence passes task conform and cargo test -p inbox-core; the suite lacks a read-then-expired decision scenario'
- file: crates/inbox-core/src/obligations.rs
  line: 165
  category: mutant
  severity: note
  origin: pre-existing
  message: 'reversing the order of the three PostMessage external refusals passes task conform and cargo test -p inbox-core, and no conformance run can observe it'
```
