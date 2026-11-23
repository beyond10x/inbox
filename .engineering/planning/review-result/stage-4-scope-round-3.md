---
format: aep.planning-md/3
id: review-result:stage-4-scope-round-3
kind: review-result
status: active
title: Scope critic on the stage 4 stories, round 3
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

My one open item from round 2 is settled. The text the revision changed adds nothing the epic did not ask for, leaves no promise of the epic unclaimed, and does not claim any outcome twice.

- **Round 2 finding (the bridge's end on `401` claimed twice): settled.** story:the-chat-bridge-runs-with-its-own-key-under-a-delegation now owns that end whole:
  - its check 6 covers both the lapsed delegation and the lapsed key, and says "This story builds that end; no earlier story has it" (lines 88-95);
  - its Open 4 holds the choice (lines 140-147), and its Scope says it owns the end (lines 154-156).

  In story:the-chat-bridge-turns-a-thread-reply-into-an-answer, the old check 9 is gone and check 7 points to the own-key story's check 6 (lines 85-87). Its Open 5 is only a pointer (lines 160-163). Its Scope says the check and the Open item come back to it on answer A of the blocker (lines 183-187). So the end is claimed exactly once under each answer.
- **Round 2 note (title and summary of the asks-once story): settled.** story:the-chat-bridge-asks-a-decision-once lines 6-7 now say "under Open 2 A only, one thread line says how it ended".
- **The changed references: consistent with the blocker.**
  - the delegation story's Open 5 now points to Open 4 of the own-key story (line 187);
  - its Scope line for `main.rs` leaves the question of one flag or two to Open 3 of the own-key story (lines 210-214);
  - the posts story's key line now names decision-blocker:delegation-grants-inbox-ownership (lines 150-151);
  - the operator-run story names the blocker's option B and the own-key story (lines 50-51).
- **The asks-once story's new `inbox bridge --clock-file`** (checks 7-8, Scope lines 189-194) is a hidden flag of debug builds that only the test of check 7 needs. It does not reach beyond the epic.
- **Coverage is unchanged since round 2:** all 16 promises I listed from epic:human-inbox trace to a story. The poller of epic:26 stays an omission the drafter names in Open 3 of the operator-run story.

What I read: the 4 revised stories (own-key revision 5, thread-reply revision 6, asks-once revision 8, delegation revision 10) and the 2 changed references (posts revision 6, operator-run revision 5), with `aep plan artifact graph` (18 edges from the set, none changed) and `aep plan artifact blocked` (the blocker still blocks the delegation, own-key and thread-reply stories).

What I could not establish:
- Still unknown, as in rounds 1 and 2: which operations the chat connector of `connectors` has. Every bridge story rests on that.

```findings
[]
```
