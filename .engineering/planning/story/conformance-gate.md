---
format: aep.planning-md/3
id: story:conformance-gate
kind: story
status: active
title: The conformance suite run against the core in process, as a gate
summary: '`task conform` runs the synthesized suite against `inbox-core` and fails on a failed, errored or unlisted unsupported scenario.'
relations:
- decomposes: epic:service-in-memory
- depends_on: story:core-and-memory-backend
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/inbox-conformance
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`crates/inbox-conformance` interprets the synthesized `ess-conformance` suite against a `Target`
and has one target: the generated `handle` over the 2 obligations of `inbox-core` and
`MemoryBackend`, in process. The target supplies the generated context port (`TryContext`)
itself: the caller is the step's caller, and each `external:` branch is answered as the step's
expected outcome says. The context of `inbox-core`, which computes `unknown-option` and
`shape-mismatch` and asks a verifier, is not on this path: its own tests check it, and the wire
target of story:service-acceptance runs it.

`task conform` synthesizes the suite, runs it,
writes the report with `ess verify conform report`, and fails unless 0 scenarios failed, 0
errored and the unsupported scenarios are exactly those `crates/inbox-conformance/unsupported.tsv`
names with their reasons. `task check` runs it.

## Acceptance

`task conform` exits 0 with 0 failed and 0 errored against `inbox-core`, and exits 1 after the
`duplicate-key` check is removed from the core's `PostMessage`, where before this story the
suite runs on the ESS interpreted target only.

## Context

Stage 1 result on the interpreted target with ESS 0.96.0: 92 scenarios (23 authored), 92 passed.
The interpreted run stays as `task conform-interpreted`; a scenario that passes there and fails
here is a defect of the core or a gap of the model, and the story says which for each.

## Scope

- `crates/inbox-conformance/` (new)
- `Taskfile.yml`: task `conform`; `check` gains it
- `AGENTS.md`: Commands row
