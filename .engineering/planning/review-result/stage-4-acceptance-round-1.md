---
format: aep.planning-md/3
id: review-result:stage-4-acceptance-round-1
kind: review-result
status: active
title: Acceptance critic on the stage 4 stories, round 1
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

The question you asked: the 4 stories cannot meet their acceptance as written. Check 1 of each says the bridge's ESS document goes "in `ess/`", with every relation the sources leave open "left as an `UNMAPPED:` marker". `task spec` is the first step of `task check` (`Taskfile.yml:15`). It runs `grep -rn UNMAPPED ess` and exits 1 on any match (`Taskfile.yml:58`). The operator's rule says the same: "`ess/` holds no `UNMAPPED:` marker" (`AGENTS.md:146`; `docs/design/inbox.md:234`). So check 1 can pass, since `ess specify validate` exits 0 on the document, while the gate goes red and the story cannot land. An `ess specify validate` exit 0 does not stand in for `task spec`.

Each check 1 needs one of three changes:
- the relation is written out as the operator answers its Open item;
- a rule ESS cannot express becomes an `external:` outcome listed in `docs/design/inbox.md`, "What the specification cannot say";
- the draft with its markers stays outside `ess/` (for example in `docs/design/bridge.md`) until the answer comes.

Story 1 also has a marker tied to no Open item: "Whether it is ever removed: `UNMAPPED:`" (line 176).

Findings:

story:the-chat-bridge-posts-a-pending-message — check 1 puts `UNMAPPED:` markers in an ESS document under `ess/`, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold — .engineering/planning/story/the-chat-bridge-posts-a-pending-message.md:74
story:the-chat-bridge-marks-a-message-read-on-a-reaction — check 1 extends the document in `ess/` with `UNMAPPED:` markers, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold — .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md:56
story:the-chat-bridge-asks-a-decision-once — check 1 keeps the ChatPost-to-Message relation as "`UNMAPPED:` until then" in `ess/`, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold — .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md:62
story:the-chat-bridge-turns-a-thread-reply-into-an-answer — check 1 extends the document in `ess/` with `UNMAPPED:` markers, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold — .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:62
story:the-chat-bridge-posts-a-pending-message — check 4 ("no text of a sender reaches the chat provider as the provider's own markup") names no input and no expected text, and Open 7 leaves the escapes "not verified", so a stand-in case cannot tell a pass from a fail — .engineering/planning/story/the-chat-bridge-posts-a-pending-message.md:91
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal — the acceptance joins two independent outcomes (the agent's read answered the person's `Pending` rows, and its `Answer` moving `Read` to `Answered`), so one can pass while the other fails — .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:59
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal — the Outcome says the agent marks the person's messages delivered and read, but no check moves a message by the agent's `MarkDelivered` or `MarkRead`, and story 5 check 4 relies on that — .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:43
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal — check 5 ("names how many of them it then supports") sets no expected number, so it also passes when none of the 71 scenarios becomes supported — .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:82
story:the-chat-bridge-marks-a-message-read-on-a-reaction — check 2 ("no later poll asks about it") can only fail under Open 4 A, one read per post, while the default Open 4 B reads the whole channel per poll, so under the default it always passes — .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md:61
story:the-chat-bridge-marks-a-message-read-on-a-reaction — check 4's "is no failure of the bridge" names nothing to observe (no stderr line, the bridge keeps running, the record holds the post as read) — .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md:67
story:the-chat-bridge-marks-a-message-read-on-a-reaction — check 6 ("a reaction taken away after the poll changes nothing") cannot fail, because the model has no command or transition out of `Read`, and the check names no bridge behaviour that would break it — .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md:74
story:the-chat-bridge-asks-a-decision-once — the acceptance joins three outcomes (one post across restarts, `answered_by: silence` after `expires`, one thread line), and the middle one is already true before this story because `Expire` exists — .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md:52

What I read: all 6 of the 6 ids, in full, from their files under `.engineering/planning/story/`. I also ran `aep plan artifact kinds` and `aep plan artifact lifecycle story`. In the tree I read `AGENTS.md` (lines 1-30 and 135-155), the `check`, `spec` and `coverage` tasks in `Taskfile.yml`, `docs/design/inbox.md:225-245`, `ess/system.yaml`, `ess/components.yaml`, `ess/ess-inputs.yaml`, `ess/domains/messaging.yaml:632-700` and `ess specify validate --help`.

What I could not establish:
- Whether `ess specify validate --path ess --strict-requires` reads a document that sits under `ess/` but is not listed in `ess-inputs.yaml` (the not-synthesized option, Open 6 ii). I did not run it.
- Story 4 check 7 (retention of the record): the bridge's own clock compared with the service's `--clock-file` is not stated. This is an unease, not a finding.
- Out of my lane, for design and scope:
  - stories 1, 3, 4 and 5 are scoped "for the defaults" of Open items the operator has not answered;
  - story 5 depends on story 2, which is a change of architecture waiting for the operator.
- Whether a blocker artifact is needed: no. The `UNMAPPED` conflict is fixed in the text of each check 1.

```findings
- file: .engineering/planning/story/the-chat-bridge-posts-a-pending-message.md
  line: 74
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 1 puts `UNMAPPED:` markers in an ESS document under `ess/`, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold"
- file: .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md
  line: 56
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 1 extends the document in `ess/` with `UNMAPPED:` markers, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold"
- file: .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md
  line: 62
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 1 keeps the ChatPost-to-Message relation as \"`UNMAPPED:` until then\" in `ess/`, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold"
- file: .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md
  line: 62
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 1 extends the document in `ess/` with `UNMAPPED:` markers, which `task spec` in `task check` refuses, so the check and a green gate cannot both hold"
- file: .engineering/planning/story/the-chat-bridge-posts-a-pending-message.md
  line: 91
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 4 (\"no text of a sender reaches the chat provider as the provider's own markup\") names no input and no expected text, and Open 7 leaves the escapes \"not verified\", so a stand-in case cannot tell a pass from a fail"
- file: .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md
  line: 59
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance joins two independent outcomes (the agent's read answered the person's `Pending` rows, and its `Answer` moving `Read` to `Answered`), so one can pass while the other fails"
- file: .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md
  line: 43
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the Outcome says the agent marks the person's messages delivered and read, but no check moves a message by the agent's `MarkDelivered` or `MarkRead`, and story 5 check 4 relies on that"
- file: .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md
  line: 82
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 5 (\"names how many of them it then supports\") sets no expected number, so it also passes when none of the 71 scenarios becomes supported"
- file: .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md
  line: 61
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 2 (\"no later poll asks about it\") can only fail under Open 4 A, one read per post, while the default Open 4 B reads the whole channel per poll, so under the default it always passes"
- file: .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md
  line: 67
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 4's \"is no failure of the bridge\" names nothing to observe (no stderr line, the bridge keeps running, the record holds the post as read)"
- file: .engineering/planning/story/the-chat-bridge-marks-a-message-read-on-a-reaction.md
  line: 74
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "check 6 (\"a reaction taken away after the poll changes nothing\") cannot fail, because the model has no command or transition out of `Read`, and the check names no bridge behaviour that would break it"
- file: .engineering/planning/story/the-chat-bridge-asks-a-decision-once.md
  line: 52
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the acceptance joins three outcomes (one post across restarts, `answered_by: silence` after `expires`, one thread line), and the middle one is already true before this story because `Expire` exists"
```
