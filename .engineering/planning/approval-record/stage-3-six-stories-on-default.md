---
format: aep.planning-md/3
id: approval-record:stage-3-six-stories-on-default
kind: approval-record
status: draft
title: 6 stories of stage 3 are accepted on the default of the question put to the operator
relations:
- decides: story:the-service-speaks-tls-with-a-pinned-certificate
- decides: story:the-client-reconnects-after-a-restart
- decides: story:inbox-wake-returns-on-a-post
- decides: story:the-claude-code-adapter-wakes-an-idle-session
- decides: story:the-stage-3-acceptance-checks
- decides: story:the-terminal-adapter-shows-a-post
revision: 1
---
## Decision

6 stories of epic:two-hosts-and-wake are accepted and built one after another, each on a
branch with a draft pull request. Every item under "Open" of an accepted story is taken at the
default the story names.

## Who decided

The coordinator, on a default. The question went to the host operator's session on 2026-11-22
at 19:55 CET with 3 lettered options and the default A; the folder rule "A default proceeds"
lets a session take the named default after 15 minutes without an answer, on a reversible
step. No answer came by 20:10 CET. The operator can reverse it: nothing of it reaches `main`.

## What it grants

- The move of the 6 stories from `draft` to `active`, run by the coordinator.
- Branches, their commits and draft pull requests. Sub-agents on Opus or lower.
- The order `aep plan artifact waves` gives: TLS with a pinned certificate, the reconnect, `inbox
  wake`, the claude_code adapter, the acceptance checks, the terminal adapter.

## What it does not grant

- No merge to `main`, no release, no tag, no deployment.
- No listener outside loopback: story:the-service-binds-a-reachable-address stays a draft and
  needs the operator's own order. story:the-metaharness-adapter-wakes-an-idle-session stays a
  draft.
