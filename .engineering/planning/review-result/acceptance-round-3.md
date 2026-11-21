---
format: aep.planning-md/3
id: review-result:acceptance-round-3
kind: review-result
status: active
title: acceptance critic, round 3 read, stage 2 stories
relations:
- reviews: story:rust-workspace-and-model
- reviews: story:core-and-memory-backend
- reviews: story:signature-verifier
- reviews: story:backend-test-kit
- reviews: story:conformance-gate
- reviews: story:http-service
- reviews: story:cli-client
- reviews: story:service-acceptance
- reviews: story:declared-coverage
revision: 1
---
approve

What I read and ran: I read all 9 of the 9 stories in full with `aep plan artifact show story:<id>`, plus epic:service-in-memory, review-result:acceptance-round-1 and review-result:acceptance-round-2. I ran `aep plan artifact validate` (23 artifacts, valid), `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I read Taskfile.yml, docs/plan/stage-2.md, ess/scenarios/post-read-answer.yaml, and in ess/domains/messaging.yaml the sections for OpenInbox, PostMessage, MarkRead, Expire and the Due view. I also checked these things in a scratch copy outside the repository (ess 0.96.0):
- I added an optional field to `Message`, ran `task projections`, then `task spec`, `task projections-drift` and `slot task conform-interpreted`. The spec validated, the drift check was current, and the suite gave 92 passed, 0 failed.
- A fresh rust synthesis of that changed spec differs from the synthesis of today's spec in the model files.
- `ess generate synthesize --target rust --layout crate` gives 85 of 87 capabilities. Its binary `inbox-service-server` prints "unmet startup obligations", including `command behaviour: inbox.messaging.PostMessage`, and returns a failure exit code.
- `ess/scenarios/repeated-dedup-key.yaml` covers `PostMessage/outcome/duplicate-key`.

Round 2 items against the current revision:
- Round 2 finding on story:cli-client: resolved. The acceptance now begins "After the recipient runs `inbox open` against a running `inbox serve`, `inbox post` by the sender followed by `inbox pending` by the recipient prints the posted message" (.engineering/planning/story/cli-client.md:47-48). The post therefore no longer reaches an unknown inbox (ess/domains/messaging.yaml:312,428-430).
- Round 2 could-not-establish, core-and-memory-backend "before" state: resolved. It now reads "the behaviour of `PostMessage` is an unmet obligation and no `PostMessage` stores a message" (.engineering/planning/story/core-and-memory-backend.md:46-49). The synthesized binary lists exactly that obligation.
- Round 2 could-not-establish, rust-workspace-and-model "passes in that state": resolved. The acceptance now says "an optional field" (.engineering/planning/story/rust-workspace-and-model.md:47). My run confirms that before this story `task check` passes in that state, and that the rust synthesis changes, so the drift line naming `generated/rust` can only appear after the story.
- Round 2 out-of-lane note, an empty `tests/acceptance` passing service-acceptance: resolved. The acceptance now names the 3 checks (.engineering/planning/story/service-acceptance.md:51-54). The expiry line now says "an unanswered request" (service-acceptance.md:42). For a request that matches Expire's `expired` outcome, which deletes it (ess/domains/messaging.yaml:681-682). A decision with `default_on_silence` would have been kept until `keep_until` instead (messaging.yaml:662-672).
- Round 2 out-of-lane note on declared-coverage, where the outcome let refusals move to a separate story: resolved. The Outcome no longer allows that, and Context says such a refusal "stops the story" (.engineering/planning/story/declared-coverage.md:48). That matches the acceptance's 0 uncovered (declared-coverage.md:36-37).
- Round 2 out-of-lane note on http-service and signature-verifier: unchanged. http-service still has no acceptance for the read rule, the wait or the expiry task, and signature-verifier has none for the delegation or lapse checks. Both are still out of my lane (scope).

What I could not establish:
- I did not check what declared-coverage's "before" state is now. Round 2 ran `task coverage` and saw it exit 1 with 50 `ESS-SYNTH-013` refusals. Since then neither ess/ nor the Taskfile has changed: the only commit after stage 1 is the store commit.
- Out of my lane (scope): see the last item above.

```findings
[]
```
