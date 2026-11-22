---
format: aep.planning-md/3
id: review-result:stage-3-scope-round-1
kind: review-result
status: active
title: Scope critic on the stage 3 stories, round 1
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
approve

**What I read.** 10 artifacts: the parent `epic:two-hosts-and-wake`, the 8 drafted stories, and the neighbouring `epic:human-inbox`, `epic:jetstream-backend` and `epic:authority-in-daily-use`. I ran `aep plan artifact show` on each of them, ran `aep plan artifact graph`, and read `docs/design/inbox.md` lines 355-400 (Stages, Risks).

Eight promises extracted from the parent (`.engineering/planning/epic/two-hosts-and-wake.md`), eight traced to a story:
1. A session on each of two hosts exchanges a request and a reply through one `inbox serve`. Claimed by the-service-binds-a-reachable-address and the-stage-3-acceptance-checks.
2. The service is reached over TLS with a certificate the clients pin. Claimed by the-service-speaks-tls-with-a-pinned-certificate.
3. A reachable listen address or a tunnel. Claimed by the-service-binds-a-reachable-address, whose Open 2 covers both the bind and the tunnel.
4. An idle session is woken by a post, through a harness adapter holding the wait open as a background task. Claimed by inbox-wake-returns-on-a-post and by the claude-code adapter story.
5. The three adapters `claude_code`, `metaharness` and `terminal`. Claimed one story each.
6. The client reposts its outbox by itself after a reconnect. Claimed by the-client-reconnects-after-a-restart.
7. The restart rule is checked across hosts: after a restart, the sender's repost arrives once. Claimed by the-stage-3-acceptance-checks, on top of the reconnect story.
8. Acceptance: an idle session on the second host takes its next turn within 60 s of a post from the first host. Claimed by the-stage-3-acceptance-checks (Open 1 C and Open 2 A, with the real two-host run recorded as operator evidence) and by the adapter stories.

I found no reach beyond the parent and no two stories claiming one outcome.
- **Terminal adapter.** The parent's Context names `terminal` among the stage 3 wake adapters. The story's Open 1 C records the overlap with `epic:human-inbox` stage 4 openly.
- **Wake command and design page.** `inbox wake` and `docs/design/wake.md` implement the parent's "holds `inbox wait` open".
- **Listener outside loopback.** The story holds for the operator's order, as the parent's transport line and `AGENTS.md` require.
- **Not drafted, by design.** The replay-window finding and `story:message-id-per-sender` are left out and named as Open items.
- **Neighbouring epics.** Nothing is drafted that stages 4, 5 or 6 own.

**What I could not establish.** The correctness of the acceptances is out of my lane. This includes the 60 s bound resting on operator evidence, and the Open 1 default of the-service-binds-a-reachable-address ("no default: waits for the order"), which is for plan-critic-acceptance and plan-critic-design. The `depends_on` edges and parallel-safety of file overlaps (`client.rs`, `listener.rs` and `wake.md` are touched by several stories) are likewise not mine.

```findings
[]
```
