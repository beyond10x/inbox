---
format: aep.planning-md/3
id: review-result:stage-4-scope-round-2
kind: review-result
status: active
title: Scope critic on the stage 4 stories, round 2
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
needs-revision

Both round 1 findings are settled; one new warning remains, about two stories that both build how the bridge ends on `401`.

- **Round 1, finding 1 (poller gap): settled.** story:the-operator-runs-the-chat-bridge-on-the-real-channel now says the poller is "Not in this set", because nothing in this repository names it (lines 29-31), and holds it as Open 3 (lines 101-103). That is an omission the drafter names, so it is not a gap.
- **Round 1, finding 2 (thread line on silence): settled in the body.** In story:the-chat-bridge-asks-a-decision-once:
  - The Outcome calls the line "an addition the epic does not ask for" (lines 45-47).
  - The Acceptance now rests on one post across a restart (lines 51-55), and check 5 on `answered_by: silence` (lines 74-78).
  - The line is only under Open 2 A, in check 6 (line 79) and Open 2 (lines 144-150). Open 2 C, "nothing", drops check 6.
  - The operator-run story's check 7 makes it conditional too (lines 70-71).
  - The title and summary (lines 6-7) still promise the line without a condition. I give no finding for that, since the body decides and the brief sets the bar at a defect. It is a one-line fix if the decomposer touches the file again.

New finding:

story:the-chat-bridge-runs-with-its-own-key-under-a-delegation — check 6 has the bridge end on a `401` for a lapsed or missing delegation "as Open 5 of story:the-chat-bridge-turns-a-thread-reply-into-an-answer says". That story's check 9 claims the same end for a lapsed delegation. So the delegation case is claimed twice, and it is decided in an Open item of a story built one wave later. To settle it, move Open 5 and the delegation half of the exit into this story, and leave only the key half in the answer story's check 9 — .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:86-88; .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:88-91, :162-166

What I read: 9 artifacts in full: the epic (unchanged, revision 1), all 7 stories and decision-blocker:delegation-grants-inbox-ownership. Commands: `aep plan artifact graph`, `aep plan artifact blocked` and `aep plan artifact waves --status draft`, plus an `rg` for `401` across the set.

The epic has 16 promises, and all 16 now trace:
- **Person inbox, long-running client, reads `Pending`, posts each message, marks it delivered** (epic:12-13): posts-a-pending-message.
- **Read on a reaction** (epic:14): marks-a-message-read-on-a-reaction.
- **Thread reply becomes a reply envelope** (epic:14-15): turns-a-thread-reply-into-an-answer.
- **"signed with the bridge's key ... under a delegation"** (epic:15): the blocker, an-agent-with-a-delegation and runs-with-its-own-key. Option A says openly that it drops this.
- **Decision appears in the channel; the reply answers it** (epic:19-21): posts, turns-a-thread-reply and operator-run.
- **Through `connectors`, no token of its own** (epic:25-26): posts.
- **The poller** (epic:26): an omission the operator-run story names in its Open 3.
- **`slack` and `terminal`** (epic:26-27): `slack` in posts Open 4 A, plus runs-with-its-own-key check 3. `terminal` is covered by story:the-terminal-adapter-shows-a-post, outside the set.
- **Asked once, silence answers** (epic:29-30): asks-a-decision-once.

The new story runs-with-its-own-key traces to epic:12-15: it reads the `Pending` view of the person's inbox with the bridge's own key, under a delegation. Apart from the finding above, no two stories claim one outcome and no story reaches into stage 5, stage 6, the metaharness adapter or the reachable-address story.

What I could not establish:
- Out of my lane (design): the Context of story:the-chat-bridge-posts-a-pending-message (line 150) still names "Open 1 of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal" as where the key is decided. The decision now sits in the decision blocker.
- Still not established, as in round 1: which operations the chat connector of `connectors` has. Every bridge story rests on that.

```findings
- file: .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md
  line: 86
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Check 6 has the bridge end on a 401 for a lapsed or missing delegation 'as Open 5 of story:the-chat-bridge-turns-a-thread-reply-into-an-answer says', and that story's check 9 claims the same end for a lapsed delegation, so the delegation case is claimed twice and decided in an Open item of a story built one wave later; to settle it, move Open 5 and the delegation half of the exit into this story and leave only the key half in the answer story's check 9"
```
