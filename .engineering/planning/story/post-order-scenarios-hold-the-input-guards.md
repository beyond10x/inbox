---
format: aep.planning-md/3
id: story:post-order-scenarios-hold-the-input-guards
kind: story
status: draft
title: The conformance suite holds the input-guarded refusals of a post before the row refusals
summary: '3 changed cores pass task conform: row refusals before no-authority, already-expired or decision-without-options; unknown-inbox before not-sender; a duplicate rule that also compares on_behalf_of.'
tags:
- from-finding
relations:
- informed_by: review-result:adversary-post-refusal-order-pass-1
revision: 1
---
## Outcome

The conformance suite holds that the input-guarded refusals of `PostMessage` answer before a
row refusal, and that `duplicate` reads the sender alone. A core that answers a row refusal
before `no-authority`, `already-expired` or `decision-without-options`, that answers
`unknown-inbox` or `inbox-closed` before `not-sender`, or whose `duplicate` also requires the
same `on_behalf_of`, fails `task conform`.

## Acceptance

Each of the 3 changed cores below, built in a copy of the tree, fails `task conform` on the core
lane, where before this story each passes it (111 of 111):

- the row refusals answer before `no-authority`, `already-expired` and
  `decision-without-options`;
- `unknown-inbox` and `inbox-closed` answer before `not-sender`;
- `duplicate` requires the same `from` and the same `on_behalf_of`, so a sender that reposts
  its own message id for another principal is answered `id-taken`.

## Context

Found by the adversary of story:post-refusal-order-scenarios in stage 2, wave 8
(review-result:adversary-post-refusal-order-pass-1, findings 1 and 4). Both gaps were there
before that story.

Since wave 8 the suite holds `signature-invalid`, `unknown-option`, `authority-invalid` and
`request-without-shape` against each of the 5 row refusals, and `not-sender` against
`duplicate`, `duplicate-key` and `id-taken`. The other pairs of an input-guarded refusal and a
row refusal are held by no scenario. The case
`every_other_input_refusal_answers_before_unknown_inbox` of
`crates/inbox-core/tests/core/precedence.rs` holds 3 of them against `unknown-inbox` in
`cargo test -p inbox-core`. A second backend is held by the conformance suite and not by the
cases of the core, so the order belongs in scenarios.

The adversary wrote 5 scenarios in a copy of the tree with the helper
`before_every_row_refusal` of `tools/scenarios.py`: one per guard for `no-authority`,
`already-expired` and `decision-without-options` against the 5 row refusals, one for
`not-sender` against `unknown-inbox` and `inbox-closed`, and one in which the sender reposts
its own id for another principal and is answered `duplicate`. ESS 0.97.1 synthesis accepted
all 5 and the interpreted target passed them (116 of 116). They are not in the repository.

The order of an input-guarded refusal before an external refusal cannot be held by the suite:
a step forces only the external outcome it expects, and synthesis refuses a step that expects
an external refusal while an input guard holds (`ESS-AUTHOR-041`). The case
`an_input_refusal_answers_before_an_external_refusal` of the core holds it.

Each new scenario moves the totals of `task coverage` and of both lanes of `task conform`.

## Scope

- `tools/scenarios.py`
- `ess/scenarios/` (the generated files), `ess/ess-inputs.yaml`
- `crates/inbox-conformance/unsupported.tsv` (when the serve lane cannot run one)
- `docs/design/inbox.md` (the section on the order of the refusals)
- `AGENTS.md`, `website/docs/status.md` (the counts)
