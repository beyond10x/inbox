---
format: aep.planning-md/3
id: review-result:stage-4-design-round-2
kind: review-result
status: active
title: Design critic on the stage 4 stories, round 2
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

Round 1: both of my findings are settled in the text as it stands.
- Finding 1 (no story owned the bridge's switch to its own key) is settled. story:the-chat-bridge-runs-with-its-own-key-under-a-delegation now owns it: every request carries the header, and the lock, `.opened` and `.chat` records are keyed by the person's address (checks 1-5, .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:63-85). The thread-reply story hands it off (.engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:111-115).
- Finding 2 (the operator wait was not in the graph) is settled. decision-blocker:delegation-grants-inbox-ownership `blocks` the delegation, own-key and thread-reply stories (.engineering/planning/decision-blocker/delegation-grants-inbox-ownership.md:8-10; `aep plan artifact blocked`). The thread-reply story now says what happens to its edge on answers A, B and C (.engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:180-185).

New findings:

story:the-chat-bridge-runs-with-its-own-key-under-a-delegation — check 6 needs the bridge to end on `401` "as Open 5 of story:the-chat-bridge-turns-a-thread-reply-into-an-answer says". That exit is built by the thread-reply story (its check 9 and Open 5), and the thread-reply story `depends_on` this one, so the check reads behaviour that lands one story later and cannot pass when this story is built. To settle it, move the `401` exit (Open 5 and check 9) into this story or into story:the-chat-bridge-posts-a-pending-message, or drop check 6 here. — .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:86-88; .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:88-91, :162-166; .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:186-187; `aep plan artifact graph` (edge from thread-reply to own-key)

story:the-chat-bridge-turns-a-thread-reply-into-an-answer — its Scope still claims `main.rs` for "the flags of the key and the delegation id of the bridge". The own-key story's Open 1 A now owns those flags (`--for`, `--authority`), and this story's own Open 6 calls the delegation-id flag "the flag of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation". The Scope line should name only the flags this story adds. — .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:192, :169-170; .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:119-122

What I read: the 7 stories and the blocker, whole bodies (files with line numbers; `git diff --stat 57a16f1 0c86200`), plus `aep plan artifact graph`, `aep plan artifact waves --status draft`, `aep plan artifact blocked` and `aep plan artifact validate` (valid; its only notes are about 10 older review-results with no findings block, none of them in this set).
- I walked 18 edges: 7 `decomposes`, 8 `depends_on`, 3 `blocks`. Outside the set I checked `epic:human-inbox` (no edges out) and the blocker.
- No cycle in the declared edges. The order is 1 -> 2 -> 3 -> delegation -> own-key -> thread-reply -> operator run, plus thread-reply -> 3 and operator run -> 3.
- I traced 22 acceptance reads to a producer in the set. An edge, direct or transitive, records 21 of them. The 22nd is the own-key check 6 above, which reads against the edge.

What I could not establish:
- Whether the delegation story's client flag "the flag that names that inbox" (`main.rs`, .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:211) and the own-key story's bridge flag `--for <address>` are one flag or two. Neither body says. If the client flag is global, the bridge gets it without the own-key story.
- The set is now a single queue of 7. Every edge has a reason written beside it, and the coordinator settled the delegation -> asks-once edge (shared files), so I report no serialising chain.
- The operator-run story's check 2 still says "Open 1 B of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal" instead of naming the blocker. That reference still resolves, and wording is out of my lane.
- Out of my lane (parallel safety): `aep plan artifact waves --status draft` reports 180 collisions.

```findings
- file: .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md
  line: 86
  category: design
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 6 needs the bridge to end on 401 as Open 5 of story:the-chat-bridge-turns-a-thread-reply-into-an-answer says, but that exit is built by the thread-reply story (its check 9 and Open 5), which depends_on this one, so the check reads behaviour that lands one story later and cannot pass when this story is built"
- file: .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md
  line: 192
  category: design
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "its Scope still claims main.rs for the flags of the key and the delegation id of the bridge, which the own-key story's Open 1 A now owns (--for, --authority) and which this story's own Open 6 calls the flag of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation"
```

Files: .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md, .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md, .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md, .engineering/planning/decision-blocker/delegation-grants-inbox-ownership.md
