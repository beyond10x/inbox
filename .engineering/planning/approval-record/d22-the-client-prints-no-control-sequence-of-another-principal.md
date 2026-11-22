---
format: aep.planning-md/3
id: approval-record:d22-the-client-prints-no-control-sequence-of-another-principal
kind: approval-record
status: draft
title: 'D22: the client''s output of control sequences is fixed as its own story with the collect fix'
relations:
- decides: story:the-client-prints-no-control-sequence-of-another-principal
revision: 1
---
## Decision

story:the-client-prints-no-control-sequence-of-another-principal is accepted and built on the
branch of story:collect-returns-the-settled-record, as its own story with its own commit, and
published together with that story.

## Who decided

The default of the question D22, taken by the coordinator on 2026-11-22 at 16:38 CET: the
question was put to the operator through the host operator at 16:24 CET with the default A and
a wait of 15 minutes, and no answer came in that time.

The options put to the operator:

- A: its own story on a local branch on top of the collect fix, built now, published together
  with it.
- B: the collect fix is published as it is; this finding waits, unpublished.
- C: folded into the story of the collect fix.

## What it grants

- The creation of the story and its move from `draft` to `active`, run by the coordinator.
- Its commits on the branch `fix/collect-record`. Sub-agents on Opus or lower.

## What it does not grant

- No publication before the operator's relay has seen the finished fix.
- No release, no tag, no deployment.
