---
format: aep.planning-md/3
id: story:declared-coverage
kind: story
status: active
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
- confidence: cited
  path: ess/domains/messaging.yaml
- confidence: inferred
  path: ess/ess-inputs.yaml
- confidence: inferred
  path: ess/scenarios
- confidence: cited
  path: generated
- confidence: inferred
  path: tools/scenarios.py
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`ess verify conform synthesize --suite-format 5` exits 0 on `ess/`: each of the generated
scenarios synthesis refuses is claimed by an authored scenario's `covers:` list. The story adds
scenarios, makes `task check` run `task coverage`, and makes one change to the model: the views
`Pending` and `Unanswered` no longer carry the fields `reply` and `decided_option` (the host
operator's decision of 2026-11-21, under Context).

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

As settled in wave 3 of stage 2: `task conform` runs the suite against the core in process and
its gate holds the report to the suite (`crates/inbox-conformance`). A scenario this story adds
that fails or errors against the core is reported with its id and is not fixed here: the core,
the runner and `crates/inbox-conformance/unsupported.tsv` are not in this story's scope. The
report of `task conform` stays `inconclusive` after this story, because that task synthesizes
the suite without declared coverage; whether `task conform` runs the suite `task coverage`
writes is decided at the opening of story:service-acceptance, which owns the runner and the
`conform` task. This story reports what that suite's run on the interpreted target answers as
`conformance_status`.

The host operator's decision of 2026-11-21, on the unit's report: scenarios cover 42 of the 50
refusals, and the other 8 are the invariants of `reply` and `decided_option` at the views
`Pending` and `Unanswered`. No view row can carry either field there. `Pending` holds a message
in `Sent` or `Delivered` and `Unanswered` one in `Read`. Both fields are set only by the
outcomes `Answer/answered` and `Expire/decided-on-silence`, and each of them moves the message
to the terminal state `Answered`; `PostMessage/posted` creates the message with both fields
empty, and no other outcome writes them. So no sequence of commands leaves a row of either view
with either field set, and no scenario that claims the 8 would be true. The decision: remove the
2 fields from the 2 views in `ess/domains/messaging.yaml` (4 lines) and regenerate `generated/`
with the tasks, in this story; that replaces the paragraph above that stops the story at a
model change, for these 8 refusals only. The rows of `Pending` and `Unanswered` on the wire lose
2 members that were always null, a change against release 0.1.0 that the changelog names. If
the regeneration changes anything beyond the rows of these 2 views, the story stops and the
host operator is told. Before the merge the unit shows `task coverage` at exit 0 and a
`conformance_status` other than `inconclusive` for the suite that task writes.

## Scope

- `ess/scenarios/`, `ess/ess-inputs.yaml`
- `ess/domains/messaging.yaml`: the fields `reply` and `decided_option` leave the views `Pending`
  and `Unanswered`
- `generated/`: as `task generate` and `task projections` rewrite it
- `tools/scenarios.py`
- `Taskfile.yml`: `check` gains `coverage`
- `docs/design/inbox.md`: the coverage numbers
