---
format: aep.planning-md/3
id: story:the-gate-fails-on-a-breaking-change-of-the-specification
kind: story
status: draft
title: The gate fails on a breaking change of the specification
summary: 'task check compares no two revisions of ess/: a breaking change of the specification passes the gate and CI.'
revision: 1
---
## Outcome

`task check` fails when a change of `ess/` is breaking or of unknown compatibility against the
specification of `main`, and CI fails on the same change, so a narrowing is not merged as if
it were additive.

Today no step of `task check` compares two revisions of the specification. For
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal, which changed 3 summary
sentences of `ess/domains/messaging.yaml`, the comparison was run by hand: `ess verify diff
--from <the specification before> --to ess --compatibility --fail-on breaking-or-unknown`
answered `compatibility: 0 breaking, 0 unknown, 3 compatible` (2026-11-24, ESS 0.98.0).

## Acceptance

- A step of `task check` runs `ess verify diff --compatibility --fail-on breaking-or-unknown`
  from the specification of the merge base with `main` to the one of the tree and fails on a
  breaking or unknown item with the line of `ess`; a tree whose `ess/` equals the one of
  `main` passes without a comparison (state transition: a breaking change of the
  specification moves the gate from exit 0 to a non-zero exit).
- A planted breaking change (a removed command outcome) makes the step fail, and the same
  tree without it passes; both runs are named in the story's Measured.
- An acknowledged breaking change has one place in the repository where it is written, and
  the step reads it.
- The step is timed before and after; the gate's test step stays within 120 s; CI runs the
  same step (the local gate equals CI).

## Context

A gap that was there before the story named above, found in its round 1 and filed as a draft.
It waits until the planned waves are delivered. The technique is the spec diff in the gate of
the hardening catalogue of ESS.
