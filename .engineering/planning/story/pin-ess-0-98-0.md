---
format: aep.planning-md/3
id: story:pin-ess-0-98-0
kind: story
status: implemented
title: The repository is pinned to ESS 0.98.0
summary: The host's ess is 0.98.0 since 2026-11-23; every pin of the repository moves from 0.97.4.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T13:18:00Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T13:18:00Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T14:15:19Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The repository is pinned to ESS 0.98.0 everywhere it names the ESS release it uses: the
manifest `ess/ess-inputs.yaml`, the CI job that installs `ess`, the manifest head `task
scenarios` writes, and the documents and comments that state which release the numbers of the
gate come from.

ESS 0.98.0 was published on 2026-11-23. Its release states 2 breaking changes of validation
(an `exists: false` branch declared below a refusal it answers first is refused; an ordering
comparison over an enum is refused) and that synthesis writes more scenarios for creating
branches that read or create related rows and for row-set refusals.

Since that day the shared `ess` of the build host is 0.98.0, so `task check` stops in its step
`spec` on a tree pinned to 0.97.4 unless `ESS_TOOLCHAIN=0.97.4` is set.

## Acceptance

- `ess/ess-inputs.yaml` requires `ess 0.98.0`; `task spec` passes with `--strict-requires`
  and without `ESS_TOOLCHAIN`.
- The CI job installs ESS 0.98.0 from its release archive, held to its SHA-256.
- `generated/` is what ESS 0.98.0 writes from `ess/`: `task drift` and `task projections-drift`
  pass. The unit states which generated files changed against 0.97.4, or that none did.
- The unit states whether the suite of `task coverage` keeps its bytes against 0.97.4, and the
  number of scenarios before and after.
- `task check` exits 0 without `ESS_TOOLCHAIN`. Every count a document states is the count
  ESS 0.98.0 prints.
- The time of the synthesis of `task coverage` before and after is in the record of the unit.

## Context

The operator's word of 2026-11-22, relayed by the host operator's session: "upgrade ess.
continue with most recent version".

Measured by the coordinator before the unit started, on a copy of `ess/` and `generated/` of
`main` (e84f170): ESS 0.98.0 validates the specification (`inbox v1 — 3 file(s), 44
scenario(s), valid`); the suite of `ess verify conform synthesize --suite-format 5` has the
same SHA-256 on 0.97.4 and 0.98.0 (`79acdfc8...`, 119 scenarios); `ess generate --check` passes
for `schema`, `openapi`, `asyncapi` and `docs`; one synthesis took 23 s on 0.97.4 and 22 s on
0.98.0 on a loaded host. Not measured: `task drift` (the Rust library and server under
`generated/rust/`), `task conform`, `task check`.

## Scope

- `ess/ess-inputs.yaml`.
- `.github/workflows/check.yml` (`ESS_VERSION`, `ESS_SHA256`).
- `crates/inbox-xtask/src/scenarios.rs` and its case (the manifest head).
- `generated/` where ESS 0.98.0 writes other bytes.
- `README.md`, `AGENTS.md`, `CHANGELOG.md`, `docs/checks.md`, `docs/design/inbox.md`,
  `website/docs/design.md`, `website/docs/how-it-is-built.md`; comments in `crates/`.

## Measured

- Red first, on the tree pinned to 0.97.4 with the case already edited: `cargo nextest run
  --locked -p inbox-xtask -E 'test(/a_manifest_out_of_its_form_is_edited|scenarios_drift|repository/)'`
  ended with 1 passed, 2 failed. One failure is the edited case, the other
  `synthesis::the_entry_of_the_repository_is_made`, which `ess` refuses with "requires ess
  0.97.4 and this is ess 0.98.0". The story named only the step `spec`; the step `test` failed
  too.
- The suite of `task coverage`: SHA-256 `79acdfc8a8157eb24f796589df7d71f71741bec8e3d4d564a73d452840dd11f5`
  on 0.97.4 (3 runs) and on 0.98.0 (3 runs), 119 scenarios before and after.
- `generated/`: `task projections` and `task generate` on 0.98.0 changed 0 of the 96 tracked
  files. The 4 files `generated/{openapi,asyncapi,schema,docs}/.ess-output/state.json` keep
  `"producer":"ess 0.97.4"`; they were not edited by hand.
- Refusal lines of synthesis: 98 on both releases (6 SYNTH-001, 2 SYNTH-003, 47 SYNTH-004, 1
  SYNTH-008, 42 SYNTH-013); the diff of standard output and standard error is empty.
- Synthesis, 3 runs each: 0.97.4 took 19.8, 21.6 and 25.6 s (17.6, 17.8 and 20.2 s of CPU
  time) at a load average of 34 to 46 on 20 CPUs; 0.98.0 took 24.9, 27.8 and 23.9 s (17.5, 18.2
  and 18.1 s of CPU time) at a load average of 60 to 75.
- The 2 breaking changes of 0.98.0: a copy of `ess/` with `unknown-inbox` below `duplicate-key`
  is refused (`conflicting_declaration`, ESS-COMMAND-004); the specification has that branch
  above `duplicate`. It has no ordering comparison over an enum.
- The asset `ess-0.98.0-x86_64-unknown-linux-gnu.tar.gz`: the release's API record and its
  `SHA256SUMS` both give `c6c9a3a3d6956ce1a237f0340905dfadc431d31b1a15b104a6d2e64bb5acaa8e`. The
  release tag is `0.98.0`, without `v`.
- `task check` of the unit alone, on `main` at e84f170 with the pin, 2026-11-23T13:35:21Z to
  13:52:06Z, without `ESS_TOOLCHAIN`: exit 0, 894 passed, 8 skipped, the test step 70 s.
- 1 case changed, 0 added.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T13:56:49Z to 2026-11-23T14:14:59Z, load average 74 to 102
  on 20 CPUs: exit 0, `928 tests run: 928 passed, 8 skipped`, the test step 68 s of
  its bound of 120 s.
