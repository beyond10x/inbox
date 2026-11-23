---
format: aep.planning-md/3
id: review-result:stage-4-design-round-1
kind: review-result
status: active
title: Design critic on the stage 4 stories, round 1
relations:
- reviews: story:the-chat-bridge-posts-a-pending-message
- reviews: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
- reviews: story:the-chat-bridge-marks-a-message-read-on-a-reaction
- reviews: story:the-chat-bridge-asks-a-decision-once
- reviews: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
- reviews: story:the-operator-runs-the-chat-bridge-on-the-real-channel
revision: 1
---
needs-revision

story:the-chat-bridge-turns-a-thread-reply-into-an-answer — under Open 1 B of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal every request of the bridge has to name the person's inbox. That covers the `Pending` rounds, `MarkDelivered`, `MarkRead` and `Thread` reads of the three earlier bridge stories, and also the outbox lock, the `.opened` record and the `<outbox>.chat.<hash>` record, all keyed by the caller's principal. No story says which one switches the bridge over to its own key. This body's Scope names only "the flags of the key and the delegation id of the bridge", and its Open 6 covers only the reply's `from`/`on_behalf_of`/`authority`. — .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:180, :162-167; .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:192-193 ("the client sends the header when it acts for another inbox"); .engineering/planning/story/the-chat-bridge-posts-a-pending-message.md:144-149, :218-222; crates/inbox-cli/src/wake.rs:186-188 (`client.caller.principal`)

story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal — the body says it "waits for the operator's answer and is not built on a default", but no `blocks` edge records that wait. So `aep plan artifact waves` puts it in wave 1, and the only trace of the operator decision in the graph is the `depends_on` edge from story:the-chat-bridge-turns-a-thread-reply-into-an-answer. That edge "holds only under its option B" and points at a story that answer A rejects. A decision artifact that `blocks` this story and the thread-reply story would record the wait. — .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:51-53; .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:110, :172-173; `aep plan artifact waves` (wave 1)

What I read: the 6 stories and the epic, whole bodies (`aep plan artifact show`, plus the files with line numbers), `aep plan artifact relations`, `aep plan artifact graph` (all 344 edges, 12 of which touch the set: 6 `decomposes`, 6 `depends_on`), `aep plan artifact validate` (valid; it reports 10 older review-results without a findings block, none of them in this set) and `aep plan artifact waves`. Outside the set I followed the edges of `epic:human-inbox`, which has none going out, and `story:no-test-holds-a-wall-clock-time`, which is implemented. No cycle: the edges run 1 <- 3 <- 4 <- 5 <- 6, 2 <- 5, and 4 <- 6 (redundant). I traced 19 acceptance reads to a producer in the set, and an edge, direct or transitive, records all 19.

What I could not establish:
- Whether the chain 1 -> 3 -> 4 -> 5 serialises more than it needs to. Every edge has a reason written beside it. The 3 -> 1 and 4 -> 3 edges are real reads. The 5 -> 4 edge is written as "edits `bridge.rs` after it", and the real read behind it (the thread line that story 4 makes first) is not stated. The trade-off is either keeping that order or splitting `crates/inbox-cli/src/bridge.rs` into modules per story. That choice is not a finding.
- Two stories own the text of a decision's chat post: story 1 (Open 7, `output.rs`) defers the options to story 5 (Open 1, `output.rs`). The deferral is written and the edges order the two, so I report no defect.
- Out of my lane (parallel safety): `aep plan artifact waves` shows collisions between the delegation story and the bridge stories on `main.rs`, `reference.rs`, `generated/cli/inbox.json`, `AGENTS.md` and `website/docs/status.md`, plus 509 collision lines that name a bridge story.

```findings
- file: .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md
  line: 180
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "under Open 1 B of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal every request of the bridge has to name the person's inbox (the Pending rounds, MarkDelivered, MarkRead and Thread reads of the three earlier bridge stories) and the outbox lock, the .opened record and the <outbox>.chat.<hash> record are all keyed by the caller's principal, yet no story says which one switches the bridge over to its own key, since this body's Scope names only the flags of the key and the delegation id and its Open 6 covers only the reply fields"
- file: .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md
  line: 51
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the body says it waits for the operator's answer and is not built on a default, but no blocks edge records that wait, so aep plan artifact waves puts it in wave 1, and the only trace of the operator decision in the graph is the depends_on edge from story:the-chat-bridge-turns-a-thread-reply-into-an-answer, which holds only under option B and points at a story that answer A rejects; a decision artifact that blocks this story and the thread-reply story would record the wait"
```

Files: .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md, .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md, .engineering/planning/story/the-chat-bridge-posts-a-pending-message.md, crates/inbox-cli/src/wake.rs
