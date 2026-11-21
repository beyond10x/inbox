---
format: aep.planning-md/3
id: story:post-refusal-order-scenarios
kind: story
status: draft
title: Authored scenarios hold the order in which PostMessage refuses
summary: Scenarios with two refusals holding at once, so the suite fails an implementation that reads a row before it has checked the signature.
tags:
- from-finding
relations:
- informed_by: story:core-and-memory-backend
revision: 1
---
## Outcome

The authored scenarios hold the order in which `PostMessage` refuses when two refusals apply to
one post, so that the conformance suite fails an implementation that reads a row before it has
checked the signature. On ESS 0.96.0 the suite of 92 scenarios holds one such pair
(`repost-after-restart-arrives-once`: `duplicate` before `duplicate-key`); an implementation that
answers `unknown-inbox`, `duplicate`, `duplicate-key` or `inbox-closed` before
`signature-invalid` passes all 92.

New scenarios in `tools/scenarios.py`, each with two refusals holding and the one the
interpreted target answers expected:

- each of the 3 external refusals (`unknown-option`, `signature-invalid`, `authority-invalid`)
  against each of the 4 row refusals (`unknown-inbox`, `duplicate`, `duplicate-key`,
  `inbox-closed`);
- one input-guarded refusal against each of the 4 row refusals;
- the pairs among the 4 row refusals that the interpreted target decides and no scenario holds
  yet.

`docs/design/inbox.md` says in one paragraph which order holds and that the contract text of
`PostMessage` in `generated/rust/inbox/PLAN.md` states another one.

## Acceptance

`task conform-interpreted` runs a scenario that expects `signature-invalid` for a post to an
inbox that does not exist, where before this story no scenario of the suite combines an external
refusal of `PostMessage` with a row refusal.

## Context

Found in stage 2, wave 2, while the coordinator checked story:core-and-memory-backend. The
unit's first version followed the contract text in `PLAN.md` (`duplicate` and `unknown-inbox`
before the input-guarded refusals, the external branches last) and passed the 92 scenarios. The
coordinator measured the interpreted target with scenarios in a scratch copy of `ess/`:

- 12 scenarios, an external refusal expected while a row refusal also holds: all 12 synthesize
  and pass;
- 4 pairs, `request-without-shape` against a row refusal: synthesis refuses the row refusal as
  the expected outcome (`ESS-AUTHOR-041`, "`request-without-shape` ... answers it first") and
  accepts `request-without-shape`, which then passes;
- 3 scenarios, an external refusal expected while an input-guarded refusal declared before it
  holds: synthesis refuses all 3 (`ESS-AUTHOR-041`).

`crates/inbox-core` follows the measured order and holds these pairs as cargo tests
(`crates/inbox-core/tests/core/precedence.rs`). This story puts them where every implementation
is checked: story:conformance-gate and story:service-acceptance run the suite.

The scenarios add to the generated scenarios that synthesis refuses only if a new scenario's
`covers:` claim is wrong; story:declared-coverage owns the count of uncovered refusals.

## Scope

- `tools/scenarios.py`
- `ess/scenarios/` (new files), `ess/ess-inputs.yaml`
- `docs/design/inbox.md`
- `AGENTS.md` (the scenario counts in Layout and Commands)
