---
format: aep.planning-md/3
id: story:declared-coverage
kind: story
status: draft
title: 'Declared coverage of the suite: no generated scenario left uncovered'
summary: '`task coverage` exits 0: every scenario synthesis refuses is covered by an authored one.'
relations:
- decomposes: epic:service-in-memory
- depends_on: story:rust-workspace-and-model
- depends_on: story:conformance-gate
scope:
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: docs/design/inbox.md
- confidence: inferred
  path: ess/ess-inputs.yaml
- confidence: inferred
  path: ess/scenarios
- confidence: inferred
  path: tools/scenarios.py
revision: 8
---
## Outcome

`ess verify conform synthesize --suite-format 5` exits 0 on `ess/`: each of the generated
scenarios synthesis refuses is claimed by an authored scenario's `covers:` list. The story
changes scenarios only. `task check` runs `task coverage`.

## Acceptance

`task coverage` exits 0 with 0 uncovered refusals, where before this story it exits 1 with 50
uncovered refusals, all `ESS-SYNTH-013`.

## Context

Stage 1 numbers with ESS 0.96.0: 97 refusals at synthesis, 47 covered by authored scenarios, 50
uncovered. The 50 are type-invariant checks (`Body`, `MessageId`, `ThreadId`, `OptionLetter`) at
views for which synthesis arranges no row. The report's `conformance_status` is `inconclusive`
while they stand.

`tools/scenarios.py` writes the authored scenarios and holds the `covers:` map.

A refusal that no scenario covers without a change to the model stops the story: it reports
the refusal and changes no file under `ess/domains/`, because the model is what the other
stories of this epic implement.

The story runs after story:conformance-gate: every scenario it adds is run by `task conform`
against the core as well as on the interpreted target, and both stay at 0 failed.

## Scope

- `ess/scenarios/`, `ess/ess-inputs.yaml`
- `tools/scenarios.py`
- `Taskfile.yml`: `check` gains `coverage`
- `docs/design/inbox.md`: the coverage numbers
