---
format: aep.planning-md/3
id: review-result:stage-3-scope-round-2
kind: review-result
status: active
title: Scope critic on the stage 3 stories, round 2
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

**What I read (round 2, a new reading).** 9 artifacts: the parent `epic:two-hosts-and-wake` (revision 2, unchanged) and the 8 revised stories, all read whole with `aep plan artifact show` in the same tree. The neighbouring epics and the graph I read in round 1 are unchanged and I did not re-read them. Part of the read was a copy of the command output saved under the session tool-results directory.

I extracted the parent's promises first and traced each one to a story.

Eight promises extracted from the parent (`.engineering/planning/epic/two-hosts-and-wake.md`), eight traced to a story:
1. A session on each of two hosts exchanges a request and a reply through one `inbox serve`. Claimed by the-service-binds-a-reachable-address (the second host reaches the one service) and the-stage-3-acceptance-checks (the exchange between two separated clients).
2. The service is reached over TLS with a certificate the clients pin. Claimed by the-service-speaks-tls-with-a-pinned-certificate.
3. A reachable listen address or a tunnel. Claimed by the-service-binds-a-reachable-address (Open 2 A, B and C).
4. An idle agent session is woken by a post, with the adapter holding the wait open as a background task. Claimed by inbox-wake-returns-on-a-post and the-claude-code-adapter-wakes-an-idle-session.
5. The three adapters `claude_code`, `metaharness` and `terminal`. Claimed one story each.
6. The client reposts its outbox by itself after a reconnect. Claimed by the-client-reconnects-after-a-restart.
7. The restart rule is checked across hosts: after a restart the sender's repost arrives once. Claimed by the-stage-3-acceptance-checks, on top of the reconnect story.
8. Acceptance: an idle session on the second host takes its next turn within 60 s of a post from the first host. Claimed by the-stage-3-acceptance-checks (Open 1 C and Open 2 A, with the operator run on two real hosts recorded as evidence of the epic) and by the adapter stories' operator-run acceptances.

The revision added nothing the parent did not ask for. The new additions are all traceable to a parent promise.
- **One-wake-per-inbox guard** in inbox-wake-returns-on-a-post (exit status 5, a lock beside the outbox file). It serves the parent's "adapter holds the wait open as a background task". The claude_code adapter story now relies on it and writes no guard of its own, so the guard has one owner and is not claimed twice.
- **Ordering notes, scope lists and shared-document headings.** The `depends_on` edges (the reconnect story after the TLS story, the wake story after the reconnect story, the terminal story after the wake story, the bind story after the terminal story) and the per-adapter headings in `wake.md` and `first-use.md` are ordering and scope. They add no outcome.
- **Operator-run evidence.** The bind story and the acceptance-checks story both point at one operator run on two hosts. Each states it is the same run, so this is one shared piece of evidence and not two claims of the same outcome.
- **Terminal adapter.** The parent's Context names `terminal` among the stage 3 adapters, so the story is in scope. Its overlap with `epic:human-inbox` stays named openly in Open 1 C. Stage 4 work (the chat bridge) is not drafted.
- **Silent narrowing.** None. The metaharness story's Open 1 C records a possible exit of that adapter from the epic, and the replay-window finding and `story:message-id-per-sender` stay named Open items.

**What I could not establish.** The checkability of the acceptances, the soundness of the new `depends_on` edges and the file-overlap ordering (for example whether the bind story waiting on the terminal story is needed) belong to plan-critic-acceptance, plan-critic-design and plan-critic-parallel-safety. They did not set this verdict.

```findings
[]
```
