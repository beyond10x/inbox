---
format: aep.planning-md/3
id: review-result:adversary-post-refusal-order-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:post-refusal-order-scenarios (fbf2a82): 4 findings (2 warnings, 2 notes), 2 introduced, 2 there before the unit'
relations:
- reviews: story:post-refusal-order-scenarios
revision: 1
---
## What was reviewed

story:post-refusal-order-scenarios at `fbf2a82` on base `44c37f6` (branch `impl/w8-post-order`),
by one adversary on Opus, pass 1 of at most 2, on 2026-11-21. No second pass ran: the correction
changes sentences of one section of `docs/design/inbox.md` and no scenario, rule or source line
of a crate.

## Verdict

The claims of the unit hold. 4 findings: 1 warning and 1 note introduced by the unit, both in
the new section of `docs/design/inbox.md`; 1 warning and 1 note there before it, both gaps of
the suite. 4 changed cores were built; each passed the core lane with the unit's 111 scenarios.

## Findings

- H1 (warning, there before the unit; `tools/scenarios.py`): no scenario holds that
  `no-authority`, `already-expired` or `decision-without-options` answer before a row refusal,
  or that `not-sender` answers before `unknown-inbox` and `inbox-closed`. A core that answers
  the row refusals before those 3 guards passed the core lane (111 of 111); a core that answers
  `unknown-inbox` before `not-sender` passed both lanes.
- H2 (warning, introduced; `docs/design/inbox.md`, the section on the order of the refusals):
  the sentence on `ESS-AUTHOR-041` stood in the list of what the suite holds. Synthesis
  refusing a scenario checks the scenario and no implementation, as H1 shows.
- H3 (note, introduced; the same section): the section states that the input-guarded refusals
  answer before the 3 external ones. The suite cannot hold that order, and the section named
  cases of the core only for the order among the 3 external refusals. A core that answers the
  external refusals first passed the core lane with the adversary's scenarios too (116 of 116).
- H4 (note, there before the unit; `tools/scenarios.py`): a `duplicate` rule that requires the
  same `on_behalf_of` as well as the same `from` passed both lanes. The scenario
  `a-relayed-message-id-is-its-senders` fails only a rule that reads `on_behalf_of` in place of
  `from`.

## Changed cores (4 built in a copy of the tree)

| Change | The unit's 111 scenarios, core lane | The adversary's 5 scenarios |
|---|---|---|
| the row refusals before 3 input guards | passes | fails, 3 scenarios |
| `unknown-inbox` before `not-sender` | passes | fails, 1 scenario |
| `duplicate` also compares `on_behalf_of` | passes | fails, 1 scenario |
| the external refusals before the input guards | passes | passes |

On the serve lane the first and the last fail 1 scenario, `envelope-must-fit-its-kind`, for
another reason than a row.

## What held

- Every step of the 6 new scenario files holds the pair its comment names and no other: at
  each step both refusals of the pair hold in the model.
- All 9 pairs of row refusals one post can make hold are in the suite; `unknown-inbox` and
  `inbox-closed` cannot hold together.
- A core that stores a refused post fails the scenarios: each one checks the views afterwards.
- A core that reads the inbox before the signature only when the inbox is closed fails step 4
  of the signature scenario.
- The quote of `generated/rust/inbox/PLAN.md` and its line number are accurate.

## Cases

5 scenarios, written in a copy of the tree and not in the repository: 3 for the 3 input
guards against the 5 row refusals, 1 for `not-sender` against the 2 inbox refusals, 1 for a
repost by the sender for another principal. ESS 0.97.1 synthesis accepted all 5; the
interpreted target passed 116 of 116. They are the text of
story:post-order-scenarios-hold-the-input-guards (a draft).
