---
format: aep.planning-md/3
id: review-result:stage-4-scope-round-1
kind: review-result
status: active
title: Scope critic on the stage 4 stories, round 1
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

I found two scope defects. Both are warnings: one gap in the parent and one story that does more than the epic asks. Neither is a blocker.

epic:human-inbox — "It replaces a per-session poller of the chat channel" is promised, and no drafted story claims it or names it as left out (it appears only as a quote in the Context of story:the-chat-bridge-posts-a-pending-message:116); story:the-operator-runs-the-chat-bridge-on-the-real-channel would most naturally take it or record it as outside the repository — .engineering/planning/epic/human-inbox.md:26

story:the-chat-bridge-asks-a-decision-once — the Outcome and Acceptance make the bridge write a line into the thread saying which option silence took, and check 5 adds an "expired, not answered" line for a decision without a default, a request and a note; the epic does not ask for either, since it promises only that "the person is asked once, and at `expires` the service answers by silence with the named option"; to settle it, mark the line as an addition under the story's Open 2 and rest the acceptance on one post plus `answered_by: silence` (story:the-operator-runs-the-chat-bridge-on-the-real-channel:35-36 inherits "its thread said which option") — .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md:42-45, :55-57, :76-77; .engineering/planning/epic/human-inbox.md:29-30

What I read: the epic and all 6 stories in full, plus story:the-terminal-adapter-shows-a-post and approval-record:stage-3-six-stories-on-default (both outside the set). Commands: `aep plan artifact show` on each, `aep plan artifact graph`, and reads of docs/design/inbox.md (Stages), docs/design/wake.md (`## terminal`), website/docs/status.md (Planned) and AGENTS.md. I listed 16 promises from the epic before reading any story, and 15 trace to a story:
- epic:12-15 (person inbox, long-running client, reads `Pending`, posts each message, marks it delivered): the-chat-bridge-posts-a-pending-message
- read on a reaction: the-chat-bridge-marks-a-message-read-on-a-reaction
- thread reply from the allowed chat user becomes a reply envelope: the-chat-bridge-turns-a-thread-reply-into-an-answer
- "signed with the bridge's key ... under a delegation": an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal, its option B. Its Open 1 A says openly that the outcome is then dropped, so this is not a gap.
- epic:19-21 (decision appears in the channel; reply moves `Read` to `Answered` with `answered_by: principal`): posts-a-pending-message, turns-a-thread-reply-into-an-answer, and the-operator-runs-the-chat-bridge-on-the-real-channel for the real channel
- epic:25-26 (through `connectors`, no provider token of its own): posts-a-pending-message, Outcome
- epic:26-27 (`slack` and `terminal` adapters): `slack` in posts-a-pending-message Open 4 A; `terminal` is covered outside the set by story:the-terminal-adapter-shows-a-post (implemented on option B) and docs/design/wake.md:498
- epic:29-30 (asked once; silence answers at `expires`): the-chat-bridge-asks-a-decision-once
- not traced: the per-session poller (finding 1)

No two stories claim the same outcome, and no promise is narrowed without saying so. Notes, requests and decisions are all posted and marked read; a note takes no answer, as the model says. Nothing reaches stage 5, stage 6, the metaharness adapter or the reachable-address story.

What I could not establish:
- Out of my lane (design or acceptance): the posts, read, asks-once and answer stories (stories 1, 3, 4, 5) each put an ESS document with `UNMAPPED:` markers under `ess/`, but `task spec` fails on every `UNMAPPED` in `ess/` (AGENTS.md:15 and :78). This only works if Open 6 ii places the document outside the tree that `task spec` reads.
- Possible blocker artifact, not established: the operations of `connectors` that stories 1, 3 and 5 rely on are unverified (read from `--help` only): post, read reactions, read a thread, and whether a write needs a proof per write. This should become a dependency-blocker on `connectors` only once the operations are checked and found missing. I filed no finding for it.
- What the "per-session poller" is: neither the tree nor the planning store names it outside the epic and its one quote.

```findings
- file: .engineering/planning/epic/human-inbox.md
  line: 26
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"It replaces a per-session poller of the chat channel\" is promised, and no drafted story claims it or names it as left out (it appears only as a quote in the Context of story:the-chat-bridge-posts-a-pending-message:116); story:the-operator-runs-the-chat-bridge-on-the-real-channel would most naturally take it or record it as outside the repository"
- file: .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md
  line: 42
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The Outcome and Acceptance make the bridge write a line into the thread saying which option silence took, and check 5 adds an 'expired, not answered' line for a decision without a default, a request and a note; the epic does not ask for either, since it promises only that 'the person is asked once, and at expires the service answers by silence with the named option'; to settle it, mark the line as an addition under the story's Open 2 and rest the acceptance on one post plus answered_by: silence"
```
