---
format: aep.planning-md/3
id: review-result:stage-4-parallel-safety-round-3
kind: review-result
status: active
title: Parallel-safety critic on the stage 4 stories, round 3
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

Round 2 finding: settled.
- Check 6 of `story:the-chat-bridge-runs-with-its-own-key-under-a-delegation` no longer reads a later story. Its text is "This story builds that end; no earlier story has it", and its Scope lists `failure.rs` and `bridge.rs` for it.
- In the thread-reply story, the old check 9 is gone. Check 7 now says the end on a refused key "is not this story's: check 6 of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation". Open 5 is a pointer ("Moved ... Open 4 of ... which builds it"). Its Scope gives the answer-A return path.
- The own-key story depends on the delegation story, and the thread-reply story depends on the own-key story. So the producer of the end-on-401 now lands before everything that cites it.

Round 2 notes: I raised none.

Changes read, nothing new found
- Order: `aep plan artifact waves --status draft` still puts the 7 stories in 7 separate waves (1, 4, 5, 6, 7, 8, 9). `graph` shows the same edges as round 2, and `blocked` shows the same blocker on stories 4, 5 and 6. The 7 stay totally ordered.
- Asks-once story: the new `crates/inbox-cli/src/clock_file.rs` and the `--clock-file` flag of `inbox bridge` (`main.rs`, `reference.rs`, `generated/cli/inbox.json`) are listed in its Scope, which says "this story introduces the clock of the bridge, no earlier bridge story reads one". No other story of the set lists `clock_file.rs`. The `main.rs`, `reference.rs` and `generated/cli/inbox.json` it shares with later stories are ordered by the edges. Checks 7 and 8 read the stand-in of story 1 and the service's `--clock-file` that exists today. They read nothing a later story creates.
- Thread-reply story: it adds no flag in `main.rs`, only help text. The key and delegation flags it uses belong to the own-key story (Open 1 A and Open 3), which lands before it. The edge to the asks-once story stays, so it lands after the asks-once story's `bridge.rs` changes.
- Own-key story: Open 3 and Open 4 and its Scope are consistent with the delegation story's Scope. That Scope says the flag naming the inbox is "a flag of every client command" and `inbox bridge` takes that one flag as `--for` (default A). The own-key story adds only `--authority` in `main.rs`, and `--for` only under Open 3 B. The delegation story lands first, so the two do not race on `main.rs`. The delegation story's Open 5 now points to Open 4 of the own-key story.
- Posts story and operator-run story: I found no stale reference to the old check 9 or Open 5 of the thread-reply story in either body.

What I read: the 7 stories and the blocker, plus `aep plan artifact waves --status draft`, `graph` and `blocked`. I read the changed parts of the own-key story (check 6, Scope, Open 3, Open 4), the thread-reply story (checks 7 and 9, Scope, Open 5), the asks-once story (summary, checks 7 and 8, Scope), the delegation story (Scope, Open 5), and the cross-references in the posts and operator-run stories. I checked them by `grep`, not in full.

Placement of the 7 stories: 7 cited (at least one cited path each; `bridge.rs`, `bridge*.rs` tests and `docs/design/bridge.md` inferred and not yet existing, created only by story 1), 0 only inferred, 0 unplaceable.

Reads traced to a producer: 10, all recorded by `depends_on` edges or the chain they run through, 0 unrecorded. The new read is the own-key story's `thread` reads of the asks-once story. It is ordered after it.

What I could not establish:
- Whether the `--clock-file` flag of `inbox bridge` reuses `ServeArgs::clock_file` code in `main.rs` that story 1 also edits. Story 1 comes first, so it is ordered either way.
- Under non-default answers (a new crate for the bridge, `ess/` edits) the surfaces change, as in round 1.
- Nothing is out of my lane.

```findings
[]
```
