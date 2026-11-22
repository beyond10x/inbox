---
format: aep.planning-md/3
id: review-result:stage-3-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic on the stage 3 stories, round 1
relations:
- reviews: story:the-service-speaks-tls-with-a-pinned-certificate
- reviews: story:the-client-reconnects-after-a-restart
- reviews: story:the-service-binds-a-reachable-address
- reviews: story:inbox-wake-returns-on-a-post
- reviews: story:the-claude-code-adapter-wakes-an-idle-session
- reviews: story:the-metaharness-adapter-wakes-an-idle-session
- reviews: story:the-terminal-adapter-shows-a-post
- reviews: story:the-stage-3-acceptance-checks
revision: 1
---
needs-revision
story:the-claude-code-adapter-wakes-an-idle-session — the acceptance says the woken session "handles the message", and nothing marks that as done or not done; name the record that shows it, such as the message moving from `Delivered` to `Read` in the owner's `Pending`/`Unanswered`, or a reply showing up in the sender's `Outbox` — .engineering/planning/story/the-claude-code-adapter-wakes-an-idle-session.md:23
story:the-service-binds-a-reachable-address — the acceptance says `inbox serve --listen` "binds when a certificate is given", but no check is named for that clause: Open 5's default C keeps only the refusals in the gate and leaves the outside-loopback bind to an operator run, which this acceptance does not name as its evidence and which story:the-stage-3-acceptance-checks Open 1 C records as evidence of the epic; name that operator run as the evidence of this story, or switch Open 5 to A (bind `0.0.0.0:0`, connect through loopback) — .engineering/planning/story/the-service-binds-a-reachable-address.md:23

```findings
- file: .engineering/planning/story/the-claude-code-adapter-wakes-an-idle-session.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says the woken session \"handles the message\", and nothing marks that as done or not done; name the record that shows it, such as the message moving from `Delivered` to `Read` in the owner's `Pending`/`Unanswered`, or a reply showing up in the sender's `Outbox`"
- file: .engineering/planning/story/the-service-binds-a-reachable-address.md
  line: 23
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says `inbox serve --listen` \"binds when a certificate is given\", but no check is named for that clause: Open 5's default C keeps only the refusals in the gate and leaves the outside-loopback bind to an operator run, which this acceptance does not name as its evidence and which story:the-stage-3-acceptance-checks Open 1 C records as evidence of the epic; name that operator run as the evidence of this story, or switch Open 5 to A (bind `0.0.0.0:0`, connect through loopback)"
```
