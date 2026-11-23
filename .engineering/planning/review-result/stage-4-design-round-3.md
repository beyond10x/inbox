---
format: aep.planning-md/3
id: review-result:stage-4-design-round-3
kind: review-result
status: active
title: 'Plan critic, design, round 3 on the stage 4 draft stories: approve, no finding'
relations:
- reviews: story:the-chat-bridge-posts-a-pending-message
- reviews: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
- reviews: story:the-chat-bridge-marks-a-message-read-on-a-reaction
- reviews: story:the-chat-bridge-asks-a-decision-once
- reviews: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
- reviews: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
- reviews: story:the-operator-runs-the-chat-bridge-on-the-real-channel
revision: 1
---
approve

**Round 2 findings, both settled:**
- **Finding 1 is settled:** the own-key story's check 6 needed the end on `401` that the thread-reply story built, which lands after it. The own-key story now builds that end itself. Its check 6 says "This story builds that end; no earlier story has it", and its Open 4, its `failure.rs` scope line and its Scope paragraph back that up (.engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:88-95, :141-147, :155-157). The thread-reply story's old check 9 is gone. Its check 7 and its Open 5 now point to the own-key story, and its Scope says the check comes back to it on answer A of the blocker (.engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:85-87, :160-163, :183-187). The delegation story's Open 5 now points at Open 4 of the own-key story. The dependency now runs the same way as the edges.
- **Finding 2 is settled:** the thread-reply Scope claimed the flags of the key and delegation id. Its `main.rs` line now reads "this story adds no flag" (.engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:195-198).
- **My round 2 note, one flag or two, is also settled:** it is Open 3 of the own-key story, default one flag. The delegation story's `main.rs` line names the same flag `--for <address>` and points to that Open item. The own-key story's `main.rs` line takes `--for` only under Open 3 B.

**The rest of the revision raises no new finding in my lane:**
- **Bridge clock (asks-once):** the asks-once story introduces the bridge's clock (`--clock-file`, `FileClock` of `crates/inbox-cli/src/clock_file.rs`, which exists). No earlier bridge story reads that clock. The posts story's "not due" is the `Pending` view's own filter on the server (crates/inbox-cli/src/wake.rs:2), and the reaction story reads the service's clock.
- **Operator-run story:** its changed reference now names the blocker.
- **Edges and order:** no edge changed. No cycle, and the order (posts, reaction, asks-once, delegation, own-key, thread-reply, operator run) matches every acceptance read.

**What I read:**
- The diff of the revision (`git diff HEAD~1 HEAD` on `.engineering/planning/story/`).
- `aep plan artifact graph`: 11 `depends_on`/`blocks` edges touching the set.
- `aep plan artifact blocked` (the blocker is open and blocks the delegation, own-key and thread-reply stories) and `aep plan artifact validate` (valid).
- `crates/inbox-cli/src/main.rs:279-282` and `crates/inbox-cli/src/clock_file.rs:38-78` for the cited clock.
- I traced 22 acceptance reads to a producer in the set. An edge, direct or transitive, records all 22.

**What I could not establish:** none.

```findings
[]
```

Files: .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md, .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md, .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md
