---
format: aep.planning-md/3
id: review-result:stage-4-parallel-safety-round-4
kind: review-result
status: active
title: Parallel-safety critic on the stage 4 stories, round 4
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

My round 3 verdict holds for the set as it stands now.

What changed, read from `git show eb3fcf9 -- .engineering/planning/story/`:
- `story:the-chat-bridge-runs-with-its-own-key-under-a-delegation`: check 6 now runs 2 causes with one line. Open 4 A is worded to match. Scope, `scope:` paths and edges are untouched.
- `story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal`: only the last sentence of Open 5 changed. It still points to Open 4 of the own-key story.

Parallel-safety findings:
- No edge, scope entry or `Scope` section changed in either story.
- `aep plan artifact waves --status draft` still puts the 7 stories in 7 separate waves (1, 4, 5, 6, 7, 8, 9). No two share a wave, so no pair is unordered, and the total order is unchanged.
- The producer and reader order from round 2 and 3 stays right. The own-key story builds the end of the bridge on a refused key (check 6, Open 4), and the delegation story, which comes before it, only points to it.
- Check 6 reads the `401` that story 4 produces, and that story is ordered before. It reads the "service reads the key file at its start" fact from the delegation story's Open 5 and `docs/design/service.md`. Both exist before this story is built.
- Out of my lane and not counted: the Scope paragraph of the thread-reply story for answer A still describes the returned check as naming "the key id and `inbox keyfile add-key`" only. It could be reworded to the one-line form of Open 4. That is for the acceptance lane.

What I read: `git show eb3fcf9` (stat and the story diffs), `aep plan artifact waves --status draft`, and the round 4 brief.

Placement of the 7 stories: 7 cited, 0 only inferred, 0 unplaceable, unchanged from round 3. Reads traced to a producer: 10, all recorded by edges or the chain, 0 unrecorded.

What I could not establish: nothing new.

```findings
[]
```
