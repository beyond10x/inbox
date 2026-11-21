---
format: aep.planning-md/3
id: review-result:adversary-service-acceptance-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:service-acceptance (b919136): 3 notes, 2 red cases, no warning'
relations:
- reviews: story:service-acceptance
revision: 1
---
3 findings, all notes. No blocker, no warning.

```
unit: story:service-acceptance, correction round b919136 on 5e5058c (base f883cbd), worktree inbox-s2-w5-acc
verdict: CONFIRMED
cases: executed 338→340, red 2
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory (the assigned scratch directory, part 6)
needs-coordinator: none
```

## 1. Diff stat

`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short` shows one untracked file, `crates/inbox-conformance/tests/adversary4.rs`. Nothing is staged or committed.

## 2. Cases added

`crates/inbox-conformance/tests/adversary4.rs`: 2 cases, both fail now. Run alone first with `cargo test -p inbox-conformance --test adversary4`:

- `ess_reads_a_refusal_without_covered_by`
  ```
  the runner refuses a suite ESS reads: Some("an entry of coverage `refused` has no `covered_by`")
  ```
- `ess_reads_a_suite_whose_scenarios_claim_no_absent_events_without_a_catalog`
  ```
  the runner refuses a suite ESS reads: Some("the suite has no `events`")
  ```

One check that cannot be a cargo test is the scratch script `gate_cases.py` (finding P3).

## 3. Suite run, after the case existed

Command: `CARGO_INCREMENTAL=0 cargo test -q --locked --workspace --no-fail-fast`, exit 101.

- Totals: 338 passed, 2 failed, 9 ignored. The 2 failures are the adversary4 cases.
- Last lines: `error: 1 target failed: -p inbox-conformance --test adversary4`.

## 4. Findings

**P1. note, INFEASIBLE, introduced.** `crates/inbox-conformance/src/suite.rs:456` (doc at :50)
- The round refuses a `refused` entry that lacks any of 11 members. The doc says ESS 0.96.0 refuses each altered suite these rules stand for.
- Measured on the real suite, one member removed at a time: ESS refuses 10 of the 11 as missing. It reads an entry without `covered_by`, whether in one entry or in all 89. The runner refuses that suite: a false refusal.
- Not reached today: every refusal in this repository's suite carries `covered_by`, because `task coverage` fails on an uncovered one.
- Fix: read an absent `covered_by` as `null`.

**P2. note, INFEASIBLE, pre-existing.** `crates/inbox-conformance/src/suite.rs:370`
- ESS writes the `events` catalog "only where a scenario refers to it". With no `absent_events` or `unpublished_events` in any scenario, ESS runs the suite without `events`. The same suite with the catalog kept is refused.
- The runner requires `events` in every suite.
- The line has been unchanged since f883cbd. Every scenario of this repository's suite has absent-event runs, so nothing reaches it today.

**P3. note, INFEASIBLE, introduced.** `crates/inbox-conformance/src/gate.rs:67` (`read_reasons`)
- `gate --results` holds the results document to the suite digest only, not to the report. The reasons are only looked up by id.
- Results whose 20 passed scenarios are all marked `failed`, or with a passed one marked `unsupported`, still give exit 0.
- The report and its results carry the same `completed_at`, which could bind them.
- Nothing reaches it through `task conform`, which writes and reads one path in sequence.
- Case: the scratch script `gate_cases.py`.

## 5. What held

**F1, status check.**
- Applying the round's rule to the projected contract gives a status to every declared outcome of the 10 commands, and the rows status (200) and the not-granted status (403) to every view.
- No outcome has 2 statuses.
- An outcome missing from the map fails, as does a renamed schema. Read in the code:
  - `duplicate` under 202 fails;
  - a declared error under another error's status fails;
  - a view under 206 or 204 fails;
  - a 403 whose body has no `actor` fails;
  - a 401 with an outcome fails.
- Nothing in it restates a rule of the specification.

**F2, check 3.** Against the duplicate-check-dropped mutant, check 3 now fails at `checks.rs:221`.
- Other mutants against the 3 checks (5 more; 6 is this pass's limit):
  - sweep-never-runs: check 2 fails (:179);
  - envelope signature unchecked, request signature unchecked, key-to-`from` binding unchecked, read rule dropped: stay green. That restates F3, so it is not a new finding.
- No list of "the defect that fails it" is in the tree, so that part of the brief could not be tried.

**F4, reasons.** Run through `gate_cases.py`:
- the real results: exit 0;
- no `--results` with entries in the list: 73 problems;
- no `--results` with the empty core list: exit 0;
- core results given for the serve report: 73 problems;
- a trailing space on either side: fails;
- another suite's digest, a missing `message`, a scenario twice: exit 2;
- a CRLF list: exit 0, since `lines()` strips the `\r`.

**F5.** An empty member in a row path now reaches nothing, as measured on ESS.

**F6.** The other new refusals agree with ESS:
- 10 of the 11 `refused` members, `selection.filter`, the sort order, a `covered_by` naming no authored scenario, `1.0`;
- `covered_by: null` is read;
- the real suite of `task coverage` is read by both.

**`adversary3.rs`.** Only formatting changed, plus the `format!` at :150 replaced by a raw string. Every assertion is intact.

**No regression of pass 1, measured in a tree copy at HEAD:**
- each task alone on a clean `.out/` exits 0: `conform-interpreted` 93/93, `coverage`, `conform` (core 93 passed `passed`; serve 20 passed, 73 unsupported, `failed`), `acceptance` 3;
- the list equals the run: 73 ids, 0 reasons differ;
- check 2: 5 of 5 passed, 3.47 to 4.01 s;
- no process left, checked by binary path.

**Repository rules.** The round's diff has no name, home path, `unsafe` or comment in a manifest, and no edit under `generated/`, `ess/`, `docs/` or `.engineering/`.

No permission check denied a tool call.

## 6. Paths written outside the worktree

The assigned scratch directory `<scratch>/w5/service-acceptance/adversary/`. Pass 2 added:
- scripts: `reverse.py`, `noevents.py`, `statuses.py`, `gate_cases.py`, `listcmp.py`, `p2_mutants.sh`, `p2_tasks.sh`
- `adv3-committed.rs`
- logs: `adv4-first.log`, `suite3.log`
- `ess2/` (`reverse/` and the `n*` suites)
- `p2/` (build, mutant, task and acceptance logs, `gate/`)

The tree copy `p2/tree` and its `target/` are deleted.

## 7. Findings block

```findings
- file: crates/inbox-conformance/src/suite.rs
  line: 456
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'The runner refuses a refused entry without covered_by, which ESS 0.96.0 reads, although the Coverage doc says ESS refuses each suite these rules stand for.'
- file: crates/inbox-conformance/src/suite.rs
  line: 370
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: 'The runner requires the events catalog in every suite, while ESS writes it only where a scenario refers to it and runs a suite without it.'
- file: crates/inbox-conformance/src/gate.rs
  line: 67
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'gate --results binds the results document to the suite digest only, so results whose statuses contradict the report still pass the gate.'
```
