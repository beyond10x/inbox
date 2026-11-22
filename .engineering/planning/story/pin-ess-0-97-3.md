---
format: aep.planning-md/3
id: story:pin-ess-0-97-3
kind: story
status: implemented
title: The repository is pinned to ESS 0.97.3
summary: ESS 0.97.3 fixes the cost of synthesis and changes read-grant scenarios; every pin of the repository moves from 0.97.1.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:18:54Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T13:18:55Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T14:20:49Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The repository is pinned to ESS 0.97.3 everywhere it names an ESS release: the manifest
`ess/ess-inputs.yaml`, the CI job that installs `ess`, the generated crate, the projections,
the conformance suite and its runner, and the documents that state the numbers of the gate.

ESS 0.97.3 holds fixes of the synthesis cost. It changes two things a repository meets:
`--component <name>` reports only that component, and a read-grant scenario no longer copies
the reads of other views, so a synthesized suite changes.

## Acceptance

- `ess/ess-inputs.yaml` requires `ess 0.97.3`; `task spec` passes with `--strict-requires`.
- The CI job installs ESS 0.97.3 from its release archive, held to its SHA-256.
- `generated/` is what ESS 0.97.3 writes from `ess/`: `task drift` and `task projections-drift` pass.
- `task check` exits 0. Every count a document states is the count ESS 0.97.3 prints.
- The time of `task coverage` before and after is in the report of the unit.

## Context

The operator ordered on 2026-11-22 that every repository upgrades to ESS 0.97.3
(approval-record:ess-0-97-3-ordered).

## Scope

- `ess/ess-inputs.yaml` (`requires: ess 0.97.3`), `crates/inbox-xtask/src/scenarios.rs` (the
  manifest head it writes), `crates/inbox-xtask/tests/tools/scenarios.rs`.
- `.github/workflows/check.yml`: `ESS_VERSION` and `ESS_SHA256` of the release archive.
- `crates/inbox-conformance/unsupported.tsv`: the reasons of 6 read-grant scenarios of
  `Replies`, `Settled` and `Unanswered`, which no longer read other views.
- `ess/domains/messaging.yaml`: 4 `# SYNTHESIS:` comments state what ESS 0.97.3 does. The 3
  examples and the external branch of `Answer` stay: each was measured as still needed.
- `generated/`, the projections and the data of the landing page figure are unchanged.
- `docs/design/inbox.md`, `docs/checks.md`, `AGENTS.md`, `README.md`, `CHANGELOG.md`,
  `website/docs/design.md`, `website/docs/how-it-is-built.md`, and comments in
  `crates/inbox-conformance/src/suite.rs`, `crates/inbox-core/src/obligations.rs`,
  `crates/inbox-core/tests/adversary.rs`, `crates/inbox-core/tests/post_message_id.rs`.

## Measured

- The synthesis of `task coverage`, 3 runs each on the same sources: 56.5, 54.1 and 55.0 s of
  wall time (42.4, 40.5, 41.7 s of user CPU) on ESS 0.97.1; 25.7, 31.4 and 27.6 s (20.1, 20.6,
  20.3 s) on ESS 0.97.3.
- `ess verify conform mutate --class error-swap` on the interpreted target under a bound of
  600 s on ESS 0.97.3: ended at the bound with no output (405 s of user CPU).
- `task check`: exit 0, `524 tests run: 524 passed, 5 skipped`, the test step 88 s of its bound
  of 120 s. Two runs before it ended red in the test step only, at a load average of 113 to
  150 on 20 CPUs: the case `tools::test_step::a_run_over_the_bound_leaves_no_test_process_running`
  (a fixture with a bound of 2 s), then the step at 121 s.
