---
format: aep.planning-md/3
id: review-result:stage-4-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic on the stage 4 stories, round 2
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

Round 1 findings
- Finding 1 (delegation story shared files, no edge) is settled. `an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal` now has `depends_on` the asks-once story (edge in `aep plan artifact graph`). Its Scope names the shared files (`main.rs`, `reference.rs`, `generated/cli/inbox.json`, `AGENTS.md`, `website/docs/status.md`) as the reason. It also says the generated JSON is regenerated with `task cli-reference`, not merged. The store now puts it in wave 6, after stories 1 to 3.
- Finding 2 (unconditional edge to the delegation story on an undecided answer) is settled. `decision-blocker:delegation-grants-inbox-ownership` blocks stories 4, 5 and 6 (`aep plan artifact blocked`). The Scope of the thread-reply story says what happens to the edge on answers A, B and C. Its direct edge to the asks-once story keeps it ordered on answer A.

Round 2 finding
- `story:the-chat-bridge-runs-with-its-own-key-under-a-delegation`, acceptance check 6, reads a behaviour that a later story creates.
  - Quoted words: "the bridge ends as Open 5 of story:the-chat-bridge-turns-a-thread-reply-into-an-answer says (default: exit 1 and one line that names the delegation id and `inbox keyfile add-delegation`)". Citation: `.engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:86-88`.
  - Producer: check 9 and Open 5 of the thread-reply story (`the-chat-bridge-turns-a-thread-reply-into-an-answer.md:88-89, 162`) are the only places that give the bridge an end on `401`. That story is built after the own-key story (edge `depends_on`, wave 8 after wave 7).
  - Effect: the check cannot pass when the own-key story is built. Its implementor would invent the end-on-401 behaviour, or the story would report green on a stand-in that story 6 later contradicts.
  - Cited surface: both bodies, the reader's acceptance line and the producer's check 9.
  - What settles it: the existing edge already orders the two the wrong way for this read. Either move the end-on-`401` behaviour and its Open 5 into the own-key story and have story 6 cite it, or drop check 6 from the own-key story's acceptance. As stated it is a reader before its producer.

Ordering of the set
- The 7 stories are totally ordered by `depends_on` edges: 1, then 2, then 3, then 4, then 5, then 6, then 7. Story 6 also has a direct edge to story 3, and story 7 to story 3.
- `aep plan artifact waves --status draft` puts each of the 7 in its own wave (1, 4, 5, 6, 7, 8, 9). No two share a wave, so no pair is unordered.
- The shared files (`bridge.rs`, `main.rs`, `reference.rs`, `generated/cli/inbox.json`, `tests/main.rs`, `tests/support/`, `docs/design/bridge.md`, `first-use.md`, `limits.md`, `status.md`, `AGENTS.md`, `wake.md`, `wake.rs`, `outbox.rs`, `commands.rs`) are all between ordered stories.
- The own-key story shares `wake.rs`, `outbox.rs`, `commands.rs` and `docs/design/wake.md` with story 1, and `main.rs` with stories 1 to 4. It is ordered after all of them through story 4.
- The other draft stories in `waves` (`metaharness`, `binds-reachable`) are outside the set, so I did not assess them.

What I read: 7 stories and the blocker in full, plus `aep plan artifact waves --status draft`, `graph` and `blocked`.

Placement of the 7 stories: 7 cited (at least one cited path each; central files such as `bridge.rs` inferred), 0 only inferred, 0 unplaceable. `bridge.rs` and `docs/design/bridge.md` do not exist yet, and only story 1 creates them.

Reads traced to a producer: 9.
- Story 2 reads story 1's record and stand-in.
- Story 3 reads story 2's read mark.
- Story 5 reads story 4's header, story 1's chat-post record and adapter check, and story 3's `thread` reads.
- Story 6 reads story 5's key and delegation, and story 3's silence line.
- Story 7 reads stories 3 and 6.
- 8 of the 9 are recorded by `depends_on` edges or the chain they run through. The 1 unrecorded read, in the wrong direction, is the finding above.

What I could not establish: whether the header of story 4 can travel through `client.rs`, `main.rs`, `commands.rs` and `keys.rs`. Its Scope now lists `commands.rs` and `keys.rs` "when the inbox acted for has to travel", which is fine because story 4 is ordered after stories 1 to 3. Out of my lane and not counted: whether the "Open 5" default belongs in story 5's acceptance at all, which is for the acceptance critic.

```findings
- file: .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md
  line: 86
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "Acceptance check 6 reads the end of the bridge on a 401 that Open 5 and check 9 of the thread-reply story create, but that story is built after this one, so the check cannot pass when this story is built; move the behaviour into this story or drop the check from it."
```
