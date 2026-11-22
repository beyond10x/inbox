---
format: aep.planning-md/3
id: story:pin-ess-0-97-4
kind: story
status: implemented
title: The repository is pinned to ESS 0.97.4
summary: ESS 0.97.4 removes a slowdown of synthesis; every pin of the repository moves from 0.97.3, and PostMessage declares unknown-inbox above duplicate.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T20:41:31Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T20:41:32Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T21:42:49Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The repository is pinned to ESS 0.97.4 everywhere it names an ESS release: the manifest
`ess/ess-inputs.yaml`, the CI job that installs `ess`, and the documents and comments that
state which release the numbers of the gate and the notes on synthesis come from.

ESS 0.97.4 removes a slowdown of synthesis that began with 0.93.0. Its release states no
breaking change and that suites keep their bytes against 0.97.3.

In the same change `PostMessage` declares its outcome `unknown-inbox` above `duplicate`.
The precedence already answers `unknown-inbox` first where both hold (a comment in the
file says so), and the next ESS release refuses a declaration order that disagrees with it
(`conflicting_declaration`, ESS-COMMAND-004), as the ESS session told this one on 2026-11-22.

## Acceptance

- `ess/ess-inputs.yaml` requires `ess 0.97.4`; `task spec` passes with `--strict-requires`.
- The CI job installs ESS 0.97.4 from its release archive, held to its SHA-256.
- `generated/` is what ESS 0.97.4 writes from `ess/`: `task drift` and `task projections-drift`
  pass. The unit states which generated files changed against 0.97.3, or that none did, and
  the same for the reorder of the 2 outcomes.
- `task check` exits 0. Every count a document states is the count ESS 0.97.4 prints.
- The time of the synthesis of `task coverage` before and after is in the record of the unit.

## Context

The operator ordered on 2026-11-22 that every repository moves its pins from ESS 0.97.3 to
0.97.4 (approval-record:ess-0-97-4-ordered).

## Decisions taken while building

- A statement that names ESS 0.97.3 as the release a refusal or a count was measured on stays
  at 0.97.3 where it was not measured again (`ess/domains/messaging.yaml` `SYNTHESIS` notes,
  `docs/design/inbox.md`, `docs/checks.md`, 4 comments in `crates/`). `docs/design/inbox.md`
  says so above its list, with the 2 reasons the list still holds: the same bytes of
  `generated/` and of the suite, and the release notes of 0.97.4 ("Suites, refusals and notes
  keep their bytes").
- The version the repository uses, and every count the gate prints, name 0.97.4.
- That the next ESS release refuses the old declaration order was not measured here: it is the
  ESS session's notice. On 0.97.4 both orders validate.

## Scope

- `ess/ess-inputs.yaml`, `ess/domains/messaging.yaml` (`PostMessage`, 2 outcomes reordered).
- `.github/workflows/check.yml` (`ESS_VERSION`, `ESS_SHA256`).
- `crates/inbox-xtask/src/scenarios.rs` and its case (the manifest head).
- `generated/` (35 files, from the reorder).
- `README.md`, `AGENTS.md`, `CHANGELOG.md`, `docs/checks.md`, `docs/design/inbox.md`,
  `website/docs/design.md`, `website/docs/how-it-is-built.md`; comments in
  `crates/inbox-core/`.

## Measured

- The synthesis of `task coverage`, 3 runs each on a loaded host (load average 41 to 63 on 20
  CPUs), same specification: ESS 0.97.3 24.4 to 26.1 s (20.4 to 20.6 s of CPU time), ESS 0.97.4
  19.7 to 20.9 s (18.9 to 19.4 s of CPU time).
- The suite of both releases has the SHA-256 `86c2f38cc074e1e9daaf03473d4c98af35a76a6a141a5ea72bcb53aad7e98f88`.
  The 96 files of `generated/` keep their bytes on 0.97.4.
- The reorder of the 2 outcomes of `PostMessage`: 35 files of `generated/` change; the suite's
  SHA-256 becomes `79acdfc8a8157eb24f796589df7d71f71741bec8e3d4d564a73d452840dd11f5`; it differs in
  `provenance.contract_digest` and `provenance.spec_digest` only; 119 scenarios before and
  after, none added, removed or changed.
- 90 capabilities, 87 generated, 3 obligations, on both releases.
- 3 sub-agents worked on the unit and each ended early. The coordinator checked the tree
  against the logs, changed the remaining version statements and ran the gate.
- `task check` on the branch merged with `main`, 2026-11-22T21:37:49Z to 2026-11-22T21:42:18Z, load average 123 to 148
  on 20 CPUs: exit 0, `691 tests run: 691 passed, 5 skipped`, the test step 53 s of
  its bound of 120 s.
