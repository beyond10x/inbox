---
format: aep.planning-md/3
id: review-result:stage-4-scope-round-4
kind: review-result
status: active
title: Scope critic on the stage 4 stories, round 4
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

My round 3 verdict still holds. Commit eb3fcf9 changes how the bridge's message on a refused key or delegation is worded, not who owns it.

- **Own-key story** (`story:the-chat-bridge-runs-with-its-own-key-under-a-delegation`, revision 6): check 6 now runs both causes. Each ends with the same one line, naming both ids, both `inbox keyfile` commands, and that the service reads the key file at its start (`.engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:88-97`). Open 4 A matches (lines 143-152). The end on `401` is still claimed by this story alone. The thread-reply story still only points to it (check 7, Open 5). Under the blocker's answer A, its Scope still takes back a key-only check, which fits because there is no delegation under A.
- **Delegation story** (`story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal`, revision 11): the last sentence of Open 5 is now a pointer that describes the same line (lines 186-188). It claims nothing new.
- **Against the epic:** neither change adds anything `epic:human-inbox` did not ask for, and no claim is now made twice. All 16 promises I listed from `epic:human-inbox` still trace to a story. The per-session poller is still the gap the drafter names, in Open 3 of the operator-run story.

What I read: `git show eb3fcf9 -- .engineering/planning/` (the 2 story diffs and the 4 round 3 review results it adds). The commit changes no edge.

What I could not establish: still, as in rounds 1 to 3, which operations the chat connector of `connectors` offers. Every bridge story rests on that.

```findings
[]
```
