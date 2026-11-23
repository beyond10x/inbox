---
format: aep.planning-md/3
id: approval-record:four-backlog-stories-accepted
kind: approval-record
status: draft
title: 4 backlog stories are accepted by the host operator under the operator's delegation
relations:
- decides: story:a-wake-whose-reader-is-gone-marks-nothing
- decides: story:a-character-a-reader-cannot-see-is-shown-as-its-escape
- decides: story:the-tls-terminate-test-holds-on-a-loaded-host
- decides: story:an-unreadable-epochs-file-keeps-its-other-lines
revision: 1
---
## Decision

4 draft stories of the backlog are accepted and built as waves, one pull request each. Every
item under "Open" of an accepted story is taken at the default the story names; where a story
names none, the coordinator decides and writes the decision into the story.

## Who decided

The host operator's session, under the delegation the operator gave it on 2026-11-23 at 03:57
CET for decisions that are not about secrets, money or architecture. The coordinator put the
question on 2026-11-23 with 3 lettered options; the answer was option B: the 4 backlog waves, and
beside them planning only for stage 4.

## What it grants

- The move of the 4 stories from `draft` to `active`, run by the coordinator.
- Branches, their commits, pull requests and the merge to `main` after a green gate and green
  required checks. Sub-agents on Opus or lower.

## What it does not grant

- No code of stage 4 and no acceptance of a stage 4 story: those wait for the operator.
- No listener outside loopback and no deployment.
