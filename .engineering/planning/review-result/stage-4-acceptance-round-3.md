---
format: aep.planning-md/3
id: review-result:stage-4-acceptance-round-3
kind: review-result
status: active
title: Acceptance critic on the stage 4 stories, round 3
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

My round 2 finding is fixed. One new defect: check 6 of the own-key story (`story:the-chat-bridge-runs-with-its-own-key-under-a-delegation`) asks the bridge to tell a lapsed key from a lapsed delegation, and nothing it can see shows the difference.

**Round 2 finding (asks-once check 7, waiting on the real clock): fixed.**
- The bridge now reads its time from `inbox bridge --clock-file`, a clock file of its own, and the case moves that file. The case waits only on the stand-in's count of polls (`.engineering/planning/story/the-chat-bridge-asks-a-decision-once.md:86-102`).
- Both sides can fail: the entry must still be there at `expires` plus 59 min, and gone at `expires` plus 1 h 1 s.
- What the check cites exists:
  - `FileClock` in `crates/inbox-cli/src/clock_file.rs:38`;
  - `#[cfg(debug_assertions)]` and `hide = true` on the service's `clock_file` flag (`crates/inbox-cli/src/main.rs:277-282`);
  - the 60 s window of the service's clock file (`docs/design/service.md:369-370`).
- Check 8 no longer conflicts with it.

**Other parts the revision changed: no finding.**
- The thread-reply story: check 7 now points to check 6 of the own-key story, the old check 9 is gone, and Open 5 is a pointer.
- The delegation story's Open 5 and its Scope line for `main.rs`.
- The two changed references in the posts story and the operator-run story.

New finding:

story:the-chat-bridge-runs-with-its-own-key-under-a-delegation — check 6 asks for one line naming the delegation id when the delegation has lapsed and another naming the key id when the key has lapsed, but both causes give the same `401 {"refused":"unauthenticated"}`, whose "body says nothing more", and the bridge's secret key file has no `valid_until`. So the bridge cannot tell the two apart, and the check's "fails when it names neither id" lets one line naming both pass. Settle it by asking for one line, whatever the cause, that names the key id, the delegation id, `inbox keyfile add-key`, `inbox keyfile add-delegation`, and that the service reads the key file at its start; or by naming what the bridge reads to tell the two causes apart. Open 4 A says "the key id (or the delegation id)" and needs the same change. — .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:88

Evidence for that finding:
- the uniform `401`: `docs/design/service.md:63-67`;
- the secret key file format: `crates/inbox-cli/src/keys.rs:5-8`;
- Open 3 A of the delegation story, which gives a request no delegation covers the same `401`.

**What I read:** all 8 artifacts, the 7 stories and the decision blocker. I ran `git diff HEAD~1` on `.engineering/planning/story/` and `.engineering/planning/decision-blocker/` to see the round 2 revision, which touched 6 stories and no edge. I read the changed sections in full. In the tree I checked `crates/inbox-cli/src/clock_file.rs`, `crates/inbox-cli/src/main.rs:272-284`, `docs/design/service.md` (the request signature and the expiry task) and `crates/inbox-cli/src/keys.rs`.

**What I could not establish:**
- Which `expires` the bridge's record keeps for a decision taken on silence. The service sets the row's `expires` to `keep_until` at that point, and asks-once check 7 assumes the envelope's own `expires`. The check pins that behaviour, so this is not a finding.
- Nothing outside my lane.

```findings
- file: .engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md
  line: 88
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 6 asks for one line naming the delegation id when the delegation has lapsed and another naming the key id when the key has lapsed, but both causes give the same `401 {\"refused\":\"unauthenticated\"}`, whose \"body says nothing more\", and the bridge's secret key file has no `valid_until`, so the bridge cannot tell the two apart, and the check's \"fails when it names neither id\" lets one line naming both pass; settle it by asking for one line, whatever the cause, that names the key id, the delegation id, `inbox keyfile add-key`, `inbox keyfile add-delegation`, and that the service reads the key file at its start, or by naming what the bridge reads to tell the two causes apart; Open 4 A says \"the key id (or the delegation id)\" and needs the same change"
```
