---
format: aep.planning-md/3
id: review-result:stage-4-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic on the stage 4 stories, round 2
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

All 12 of my round 1 findings are fixed in the text as it stands. One new finding holds: in `story:the-chat-bridge-asks-a-decision-once`, check 7 waits on the real clock, which its own check 8 rules out.

Round 1 findings, each fixed:
- **The 4 blockers on `UNMAPPED:` under `ess/`:** fixed in all 4 stories. Each check 1 now drafts the nouns in `docs/design/bridge.md`, outside `ess/`. A relation enters `ess/` only after the operator answers it, with no marker. Each check 1 requires that `grep -rn UNMAPPED ess` prints nothing and that `task spec` exits 0 after the story. So a green gate and the check can both hold. Lines:
  - `story:the-chat-bridge-posts-a-pending-message.md:72-80`
  - `story:the-chat-bridge-marks-a-message-read-on-a-reaction.md:54-59`
  - `story:the-chat-bridge-asks-a-decision-once.md:59-64`
  - `story:the-chat-bridge-turns-a-thread-reply-into-an-answer.md:60-65`
- **Posts story, check 4 (sender markup):** fixed (lines 93-100). It now names the inputs (the channel-wide mention, the delimiter, ESC, U+202E), the expected text and the cases that fail. The `--describe` flag it relies on exists in `connectors invoke --help`.
- **Delegation story, acceptance:** fixed. It is now one outcome, the agent's `Answer` (lines 64-69).
- **Delegation story, marks by the agent:** fixed. Checks 2 and 3 cover the agent's `MarkDelivered` and `MarkRead` (lines 75-79).
- **Delegation story, check 5 (no expected number):** fixed. It is replaced by check 8 with fixed numbers (lines 94-98). I confirmed both: `wc -l` gives 87, and `grep -c` gives 71.
- **Reaction story:**
  - check 2 now counts exactly one `read` through the forwarder, under either answer to Open 4 (lines 60-65);
  - check 4 names what to observe (lines 69-75);
  - check 6 now fails a bridge that marks a post again at every poll (lines 80-83).
- **Asks-once story, acceptance:** fixed. It is now one transition, `Sent` to `Delivered` of a reposted message with no second post (lines 51-55).

New finding:

story:the-chat-bridge-asks-a-decision-once — check 7 waits for the system clock to pass an `expires` "2 s ahead" ("waits until the record no longer holds the message id"), so the case depends on a real instant, which check 8 ("holds no wall-clock time") and the implemented story:no-test-holds-a-wall-clock-time ("no test sleeps until an instant passes", "none depends on a real instant") forbid, and checks 7 and 8 cannot both pass as written; settle it by giving the bridge a clock the case can move (as `--clock-file` does for the service), or by setting the service's clock file behind the system clock so the `expires` is already past for the bridge when the case starts — .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md:84

What I read: all 8 artifacts in full from `.engineering/planning/story/` and `.engineering/planning/decision-blocker/`: the 7 story ids and `decision-blocker:delegation-grants-inbox-ownership`. I also ran `aep plan artifact blocked`; the blocker blocks stories 4, 5 and 6, as the brief says. In the tree I confirmed:
- `wc -l` and `grep -c` on `crates/inbox-conformance/unsupported.tsv`: 87 and 71;
- the `Forwarder` in `crates/inbox-cli/tests/wake.rs`;
- the `clock_file` flag in `crates/inbox-cli/src/main.rs`;
- `connectors invoke --help` for `--describe`;
- story:no-test-holds-a-wall-clock-time (its acceptance and the decision on it).

The new own-key story (`story:the-chat-bridge-runs-with-its-own-key-under-a-delegation`) has one acceptance statement, a named before state, and 7 checks that can each fail. I have no finding on it.

What I could not establish:
- Whether the `Inboxes` view in own-key story check 3 can be read by the caller the case uses. I did not run it.
- Outside my lane, it does not set the verdict: operator story check 6 is followed by a blank line before check 7 (line 65). That is style only.

```findings
- file: .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md
  line: 84
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 7 waits for the system clock to pass an `expires` \"2 s ahead\" (\"waits until the record no longer holds the message id\"), so the case depends on a real instant, which check 8 (\"holds no wall-clock time\") and the implemented story:no-test-holds-a-wall-clock-time (\"no test sleeps until an instant passes\", \"none depends on a real instant\") forbid, and checks 7 and 8 cannot both pass as written; settle it by giving the bridge a clock the case can move (as `--clock-file` does for the service), or by setting the service's clock file behind the system clock so the `expires` is already past for the bridge when the case starts"
```
