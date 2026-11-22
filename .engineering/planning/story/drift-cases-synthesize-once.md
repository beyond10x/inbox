---
format: aep.planning-md/3
id: story:drift-cases-synthesize-once
kind: story
status: draft
title: The drift cases synthesize the specification once per test run
revision: 2
---
## Outcome

The drift cases of `crates/inbox-xtask` synthesize `ess/` once per test run, not once per case.

## Acceptance

One run of `task test` calls ESS synthesis for the unchanged `ess/` at most once for the cases of
`crates/inbox-xtask/tests/drift.rs`, and no case of that file is among the 10 slowest the step
prints at more than 1 s. A case that changes the specification before it synthesizes still runs
its own synthesis and still fails as it does today.

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

- `crates/inbox-xtask/tests/drift.rs`
- `crates/inbox-xtask/src/` (where `generate` and `drift` call the synthesis)
