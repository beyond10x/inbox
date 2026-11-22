---
format: aep.planning-md/3
id: story:test-builds-optimize-the-dependencies
kind: story
status: implemented
title: Test builds compile the dependencies optimized, so the test step holds its bound on a loaded host
summary: The test step ended at its 120 s bound twice at a load average of 98 to 130 on 20 CPUs; with dependencies at opt-level 2 the same run takes 39 s where it took 108 s.
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T19:31:51Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T19:31:51Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T19:49:41Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The test step of the gate holds its bound of 120 s on a loaded host: test builds compile every
dependency optimized, so the cases that sign, verify, hash and parse use less CPU. The crates
of the workspace stay unoptimized, so a change of one of them builds as fast as before.

## Acceptance

- `.cargo/config.toml` sets `[profile.dev.package."*"] opt-level = 2` beside `[profile.dev]
  debug = false`; a case of `inbox-xtask` fails when either setting is gone (state transition:
  the case is red on the file as `main` holds it and green with the setting).
- On one tree and one host, at a load average above 100 on 20 CPUs, the run of the test step
  takes less than half the time it takes without the setting; both times are in the Scope.
- `task check` exits 0; the time of the CI job `inbox / task check` before and after is in the
  report of the unit.

## Context

A defect under the rule "Tests stay fast" of the folder this repository is developed in: the
test step ends within 120 s, and raising the bound or waiting longer is not a fix. On
2026-11-22 the step ended at its bound twice on a host with a load average of 98 to 130
on 20 CPUs (633 cases, 5 threads under the build pool), and took 17 to 37 s on the same host
unloaded. The pool gives a job 5 test threads, so the time of the run is the summed time of the
cases divided by 5; on a loaded host each case waits for a CPU in proportion to the CPU it
needs. Most of that CPU was unoptimized code of dependencies: `ed25519-dalek`,
`curve25519-dalek`, `sha2` and the JSON parser.

## Scope

- `.cargo/config.toml`: `[profile.dev.package."*"] opt-level = 2`.
- `crates/inbox-xtask/tests/tools/cargo_config.rs` (new), case
  `test_builds_optimize_the_dependencies_and_not_the_workspace`, and `tests/tools/mod.rs`.
- `AGENTS.md` (the rows of `.cargo/config.toml`, of the module `tools` and of `task test`),
  `docs/checks.md` (80 cases of the repository tasks), `CHANGELOG.md`.

## Measured

- The case on the file as `main` held it: red, "left: None, right: Some({"opt-level": "2"})".
  With the setting: green.
- One tree, one host, whole workspace, 633 cases, 5 threads, load average 105 to 146 on 20
  CPUs: 107.9 s and 506.5 s summed over the cases without the setting; 38.7 s and 168.6 s with
  it. The first build of the dependencies with the setting: 3 min 05 s at that load.
- `task check` on the branch merged with `main` at release 0.3.1, 2026-11-22T19:43:45Z to
  19:49:12Z, load average 73 to 83: exit 0, `634 tests run: 634 passed, 5 skipped`, the test
  step 39 s of its bound of 120 s. The run before the merge, 19:34:14Z to 19:41:13Z at 96 to
  117: exit 0, the test step 57 s.
- The release tree of 0.3.1 without the setting, the same hour: the test step 91 s at a load
  average of 89 to 108, and at its bound twice before that (98 to 130).
