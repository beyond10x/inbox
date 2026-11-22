---
format: aep.planning-md/3
id: review-result:stage-3-design-round-1
kind: review-result
status: active
title: Design critic on the stage 3 stories, round 1
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
story:inbox-wake-returns-on-a-post — the "one wake per inbox" rule is split between this story's Open 2, whose default A allows two waits and puts no guard in `inbox wake`, and the claude_code adapter's gate claim that its files "start no second one while one runs", so under the default each adapter writes its own guard in shell and wake.md describes no shared one; Open 2 has to name that need and say which story owns the guard (B puts it in `inbox wake` for all three adapters) — .engineering/planning/story/inbox-wake-returns-on-a-post.md:78-80, .engineering/planning/story/the-claude-code-adapter-wakes-an-idle-session.md:41,61
story:the-client-reconnects-after-a-restart — Open 1 option B ties reconnect to `inbox wake`, the command story:inbox-wake-returns-on-a-post adds, and that story already has a depends_on edge to this one, so picking B turns the edge into a cycle; B has to say it reverses that edge, or be dropped — .engineering/planning/story/the-client-reconnects-after-a-restart.md:70-71, aep plan artifact graph
What I read: 8 stories and the epic (`aep plan artifact show`), `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact validate` (prints "valid"; its 10 notes are about older review-results, not this set), plus story:no-test-holds-a-wall-clock-time. I walked 23 edges, including ones outside the set (story:no-test-holds-a-wall-clock-time, then story:message-id-per-sender to story:post-does-not-tell-a-message-id-exists to its review-result). There is no cycle and no queue: two separate roots (tls, then binds; reconnect, then wake, then 3 adapters) meet only in the stage-3 checks. I traced 8 acceptance reads to a producer in the set (binds reads tls; wake reads reconnect; claude_code, metaharness and terminal read wake; stage-3 checks read tls, reconnect and wake), and an edge records all 8. The stage-3 checks story is integration on top of feature stories that each prove themselves, not a horizontal slice. The tls/binds split keeps the gated part (the operator's order for a listener outside loopback) separate, so it is not a split abstraction.
What I could not establish: (1) the terminal story's default (Open 1 B, Open 2 A) makes a never-exiting mode of `inbox wake` in the same files as story:inbox-wake-returns-on-a-post. The depends_on edge records this, but whether this is one command split across two stories or a sound extension depends on how Open 1 is settled. Noted, not a finding. (2) Out of my lane, for parallel-safety: client.rs, main.rs, commands.rs, reference.rs, generated/cli/inbox.json, docs/design/wake.md and website/docs/limits.md and first-use.md are each edited by several stories. (3) Out of my lane, for scope: the epic's second-host and real-session acceptance is held only by the operator run of the stage-3 checks Open 1 C; the metaharness resume mechanism is not known in the tree.

```findings
- file: .engineering/planning/story/inbox-wake-returns-on-a-post.md
  line: 78
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the \"one wake per inbox\" rule is split between this story's Open 2, whose default A allows two waits and puts no guard in `inbox wake`, and the claude_code adapter's gate claim that its files \"start no second one while one runs\", so under the default each adapter writes its own guard in shell and wake.md describes no shared one; Open 2 has to name that need and say which story owns the guard (B puts it in `inbox wake` for all three adapters)"
- file: .engineering/planning/story/the-client-reconnects-after-a-restart.md
  line: 70
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Open 1 option B ties reconnect to `inbox wake`, the command story:inbox-wake-returns-on-a-post adds, and that story already has a depends_on edge to this one, so picking B turns the edge into a cycle; B has to say it reverses that edge, or be dropped"
```
