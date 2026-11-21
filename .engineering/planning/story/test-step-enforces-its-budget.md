---
format: aep.planning-md/3
id: story:test-step-enforces-its-budget
kind: story
status: draft
title: The test step kills a test at 60 s and fails over 120 s
summary: 'cargo test in task check has no bound per test: a case that waits on a silent peer stood 38 minutes in a changed copy and held a build slot.'
tags:
- budget
revision: 1
---
## Outcome

The test step of the gate enforces its budget itself. A test that runs longer than 60 s is
killed by the runner and counts as failed, with its name printed; the step fails when it runs
longer than 120 s, in `task check` on a workstation and in the CI job alike; the step prints
its ten slowest tests.

Today `task check` runs `cargo test --workspace --locked`, which has no bound per test. A case
that waits on a peer that never answers waits forever.

## Acceptance

A case added for the proof that sleeps 90 s makes `task check` fail within 75 s of the start
of that case and name it, where before this story the step waits the 90 s and passes. With the
case removed, the step prints its ten slowest tests and ends under 120 s.

## Context

Rule of the folder this repository is developed in, 2026-11-21 ("Tests stay fast"): the runner
kills a test at 60 s, the step fails over 120 s, and a repository whose runner does not do so
gets that change as its first budget story, ahead of the next feature.

Measured on 2026-11-21 in wave 8 of stage 2: an adversary ran `cargo test -p inbox-cli -p
inbox-server` on a changed copy of the client. The case
`the_services_own_400_401_408_413_503_keep_the_envelope_on_post_and_on_repost` of
`crates/inbox-cli/tests/cases.rs` waited on a stand-in service for a request the changed client
never sent. The run stood for 38 minutes and held one build slot until the coordinator ended
the test binary. The whole gate on the unchanged tree took 58 to 80 s in 9 runs that day.

Known ways, not decided here: `cargo-nextest` with `slow-timeout` and `terminate-after` in
`.config/nextest.toml` (a pinned binary in CI, with its checksum, like `task` and `ess`); or a
bound in the harness of the cases themselves. `cargo nextest` does not run doc tests; the
workspace has none that the gate counts, which the unit checks.

Also in scope: the stand-ins of `crates/inbox-cli/tests/support/mod.rs` and
`crates/inbox-server/tests/support/mod.rs` bound their wait for a connection, so that a case
whose client sends nothing fails with a message in seconds.

## Scope

- `Taskfile.yml` (the test step of `task check`, `task acceptance`)
- `.config/nextest.toml` (new, if that way is taken)
- `.github/workflows/check.yml` (the runner's install, the step's bound)
- `crates/inbox-cli/tests/support/mod.rs`, `crates/inbox-server/tests/support/mod.rs`
- `AGENTS.md` (the row of `task check`)
