---
format: aep.planning-md/3
id: story:drift-cases-synthesize-once
kind: story
status: implemented
title: The drift cases synthesize the specification once per test run
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T15:01:13Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T15:01:13Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T15:35:40Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The drift cases of `crates/inbox-xtask` synthesize `ess/` once per test run, not once per case.

## Acceptance

One run of `task test` calls ESS synthesis for the unchanged `ess/` at most once for the cases of
`crates/inbox-xtask` (state transition: 16 calls per run before, 1 after), and no case of
`crates/inbox-xtask/tests/drift.rs` that leaves `ess/` unchanged is among the 10 slowest the
step prints at more than 1 s. A case that changes the specification before it synthesizes still
runs its own synthesis and still fails as it does today.

Changed when the unit was implemented: the bound of 1 s named every case of the file before.
The case `drift::an_optional_field_added_to_the_specification_is_drift` changes `ess/`, so it
runs one synthesis of its own (2.5 to 3.9 s) and the two clauses could not both hold for it.

## Context

Found by the implementor of story:test-step-enforces-its-budget on 2026-11-22, existing before
that story. Each `drift::*` case runs ESS synthesis itself: 2.1 to 2.2 s per case in a run of
`task test` with 8 threads, 5.9 to 6.3 s per case with 5 threads on a loaded host (4 of the 10
slowest cases of the step were drift cases in the first, 5 of the 6 cases over 5 s in the
second).

This is the mistake the rule "Tests stay fast" of the folder this repository is developed in
exists for: an expensive step inside each test, so its cost is multiplied by the number of
tests. The 2 accepted forms: a committed generated file with a digest of its sources, or one
fixture built once per test run and read by every test, as a cache on disk keyed by the digest
of its sources under `target/`. An in-process global is not that form under cargo-nextest, which
runs each test in a process of its own.

## Scope

- `crates/inbox-xtask/src/lib.rs`: `generate_with` and `drift_with` take the synthesis as a
  function; `generate`, `drift` and `synthesize` call them and behave as before.
- `crates/inbox-xtask/tests/synthesis.rs` (new, 5 cases): the cache on disk,
  `target/tmp/synthesis/<key>/`, keyed by the SHA-256 of the arguments, the `ess` binary (path,
  length, modification time, version) and every file under `ess/`. An entry appears by a
  rename under a file lock.
- `crates/inbox-xtask/tests/drift.rs`, `tests/adversary.rs`, `tests/adversary2.rs`,
  `tests/main.rs`: the cases read the cache; a case that runs the binary gets a POSIX sh
  stand-in for `ess` first on `PATH`, which answers from the cache only for the unchanged
  `ess/` and the arguments of the synthesis, and runs the real `ess` otherwise.
- `crates/inbox-xtask/Cargo.toml`, `Cargo.lock`: the dev-dependency `sha2`.
- `Taskfile.yml`, task `test`: one case makes the cache entry before the bounded run.
- `AGENTS.md`, `docs/checks.md`, `CHANGELOG.md`.

## Measured

- Synthesis calls in one run of `task test`, counted by a logging stand-in for `ess`: 18 before
  (16 of the unchanged `ess/`), 3 after (1 of the unchanged `ess/`, in the setup step).
- The cases of `crates/inbox-xtask`: 74 cases with 159.3 s summed and 39.5 s of wall time before
  (load average 101 to 113 on 20 CPUs), 79 cases with 41.5 s summed and 13.6 s after (64 to 67).
- `task test` on the base: 81 s of its bound of 120 s, 6 drift cases among the 10 slowest at 7.3
  to 14.9 s (load average 70 to 87).
- `task check` on top of story:the-step-case-passes-on-a-loaded-host: exit 0, `529 tests run: 529
  passed, 5 skipped`, the test step 38 s of its bound of 120 s (load average 52 to 58). In that
  run the case that changes `ess/` took 11.5 s and was the slowest of the step; no other case of
  `tests/drift.rs` was among the 10 slowest.
