---
format: aep.planning-md/3
id: story:conformance-gate
kind: story
status: implemented
title: The conformance suite run against the core in process, as a gate
summary: '`task conform` runs the synthesized suite against `inbox-core` and fails on a failed, errored or unlisted unsupported scenario.'
relations:
- decomposes: epic:service-in-memory
- depends_on: story:core-and-memory-backend
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/inbox-conformance
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-21T11:42:34Z", actor: "agent:inbox", revision: 15, decided_on: {"recorded":{"test_result":1,"review_outcome":17,"verification":1}}}
---
## Outcome

`crates/inbox-conformance` interprets the synthesized `ess-conformance` suite against a `Target`
and has one target: the generated `handle` over the 2 obligations of `inbox-core` and
`MemoryBackend`, in process. The target supplies both ports that answer an `external:` branch
itself: the generated context port (`TryContext`) for the 3 branches of `Answer`, and the
`PostExternals` trait of `inbox-core` for the 3 branches of `PostMessage`. The caller is the
step's caller, and each of the 6 branches is answered as the step's expected outcome says.
Every other rule of `PostMessage` and `Expire` on this path is the obligation code of
`inbox-core`. The answers `inbox-core` itself gives to the 6 branches (`unknown-option` and
`shape-mismatch` computed, the other 4 asked of a verifier) are not on this path: the tests of
story:core-and-memory-backend check them, and the wire target of story:service-acceptance
runs them.

`task conform` synthesizes the suite, runs it, writes the report with
`ess verify conform report`, and gates on it (`inbox-conformance gate --suite --report
--unsupported`). The gate fails unless the report belongs to that suite (`suite.digest`), its
counts agree with its id lists, 0 scenarios failed, 0 errored, none was skipped, at least 1
passed, and the unsupported scenarios are exactly those
`crates/inbox-conformance/unsupported.tsv` names with their reasons. `task check` runs it.

## Acceptance

`task conform` exits 0 with 0 failed and 0 errored against `inbox-core`, and fails after the
`duplicate-key` check is removed from the core's `PostMessage`, with the scenario
`inbox.messaging/authored/repeated-dedup-key` reported as failed, where before this story the
suite runs on the ESS interpreted target only. On the failure the gate command exits 1 and
`task conform` exits 201, the status go-task gives a failed command; the story said "exits 1"
when it was accepted.

## Context

Stage 1 result on the interpreted target with ESS 0.96.0: 92 scenarios (23 authored), 92 passed.
The interpreted run stays as `task conform-interpreted`; a scenario that passes there and fails
here is a defect of the core or a gap of the model, and the story says which for each.

Since wave 2 the entry is `inbox_core::handle_with_events`: it takes the backend's `Exclusive`
section, builds the generated system over a clone of the backend and the step's context, and runs
the generated `handle`. The target calls this entry once per step with its own context type,
which implements `TryContext` and `PostExternals`. The order in which `PostMessage` refuses is in
the Context of story:core-and-memory-backend.

As built in wave 3 of stage 2:

- On ESS 0.96.0: `92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported`,
  `conformance_status: inconclusive`. No scenario passes on the interpreted target and fails
  here; `unsupported.tsv` is empty. The status stays `inconclusive` until story:declared-coverage;
  `gate::passing_status` names the status the gate requires and changes with that story.
- The runner reads `ess-conformance/52` only; another suite version, and a suite with a
  top-level or `provenance` key the runner does not know, exits 2. A key it does not know inside
  a scenario makes that scenario unsupported, never passed.
- One point where the gate is weaker than ESS's own runner: in a view row, a top-level member
  that is absent and one that is `null` are one value (`value::field_matches`). ESS's runner
  compares that field exactly. The generated Rust server leaves an unset `Optional` member out
  under every `presence:` value while the synthesized suite claims `null` for it, so the exact
  comparison fails 7 of the 92 scenarios on the unchanged core and no change of `ess/` makes
  them pass. This is filed as beyond10x/ess#799. When that issue is closed the rule goes and
  the ignored case `ess_contains_null_at_the_top_level_needs_the_member_published` in
  `tests/adversary.rs` runs. The wire target of story:service-acceptance reads the same rows.
- Adversary pass 1 returned 8 findings: 5 introduced and fixed, 1 declined (the point above,
  finding 3), 2 that were there before the unit and are backlog drafts:
  story:read-decision-expires-on-silence-scenario (a read decision deleted at expiry in place of
  decided on silence passes the suite) and story:external-refusal-order-cases (the order of the
  3 external refusals of `PostMessage` is seen by no conformance run).
  2 cases of that pass were deleted: their fixture declared 1 of the 4 types its root reaches,
  which ESS answers with an error and not a failure; a case of the unit holds the same rows.
- Adversary pass 2, on the first correction, returned 6 findings: 4 introduced and fixed, 2
  that were there before the unit and are backlog drafts:
  story:silence-leaves-the-other-fields-scenario (fields stored wrongly after `Expire` decides
  on silence pass the suite) and story:expire-publishes-once (`Expire` publishing its event
  twice passes). The 4 fixed: `satisfies` fails on a view with no rows, as ESS's runner decides
  it; the gate holds the report's outcome lists to the scenario ids of the suite, each once,
  and with `--implementation` to the implementation's name; a suite in which an object states
  a member name twice is refused with exit 2; 11 cases hold the lines of the first correction
  (the adversary's mutant script: 54 mutants, 48 killed, 6 surviving, the 6 it names as
  equivalent).
- One case holds a reading ESS's runner does not share: a plain snapshot followed by
  `expect_complete_subject_unchanged` is an error here, where ESS compares the rows. The
  synthesizer always pairs the two steps, so no scenario of the suite reaches it.

## Scope

- `crates/inbox-conformance/` (new)
- `Taskfile.yml`: task `conform`; `check` gains it
- `AGENTS.md`: Commands row
