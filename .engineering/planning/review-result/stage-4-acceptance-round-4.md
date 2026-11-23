---
format: aep.planning-md/3
id: review-result:stage-4-acceptance-round-4
kind: review-result
status: active
title: Acceptance critic on the stage 4 stories, round 4
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

My round 3 finding is fixed, and the revision added no new defect in my lane, so I approve the set as it stands.

The finding was that check 6 of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation asked the bridge to tell a lapsed key from a lapsed delegation, which a uniform `401` does not let it do. Now (`.engineering/planning/story/the-chat-bridge-runs-with-its-own-key-under-a-delegation.md:88-97`):
- The case runs both causes and expects the same single line for each.
- That line names the key id, the delegation id, `inbox keyfile add-key`, `inbox keyfile add-delegation`, and that the service reads the key file at its start.
- The case fails when the bridge goes on, exits 0, or its line lacks one of the 2 ids or one of the 2 commands.
- This is something the bridge can do and something a case can fail. Open 4 A now has the same wording and gives the reason the bridge cannot tell the causes apart.

The new last sentence of Open 5 in story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal only points to Open 4 of the own-key story and agrees with it. It changes no acceptance and no check.

Read: the 2 changed artifacts, through `git diff HEAD~1 -- .engineering/planning/story/ .engineering/planning/decision-blocker/`. The other 5 stories and the blocker did not change since round 3.

Not established, and not a finding:
- The fail condition of check 6 does not list a line that leaves out "the service reads the key file at its start", or a line that differs between the two causes, though the check asks for both.
- `inbox keyfile add-key` takes only a `valid_until` after now (`crates/inbox-cli/src/keys.rs:372`). To get a lapsed key entry, the case moves the service's clock file within its 60 s window or writes the entry itself. Either way works.

```findings
[]
```
