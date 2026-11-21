---
format: aep.planning-md/3
id: review-result:acceptance-round-1
kind: review-result
status: active
title: acceptance critic, round 1, stage 2 stories
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
needs-revision
story:rust-workspace-and-model — the acceptance also passes before this story: adding a field to `ess/domains/messaging.yaml` without regenerating already makes today's `task check` fail through `projections-drift` on `generated/openapi`/`generated/schema`, so the acceptance has to say the drift line names `generated/rust`, or that `task projections` ran first — .engineering/planning/story/rust-workspace-and-model.md:40 ("`task check` fails with a drift line after a field is added to `ess/domains/messaging.yaml` without `task generate`"); Taskfile.yml:16,30-34
story:cli-client — the restart sequence leaves out `inbox open` by the owner after the restart, and the spec refuses a post to an unknown inbox with `UnknownInbox`, so followed as written `inbox repost` is refused and `inbox pending` never prints the message, even when the work is correct — .engineering/planning/story/cli-client.md:34-35; ess/domains/messaging.yaml:398-400
story:service-acceptance — the failing case, "built with the expiry task switched off", names a build switch no drafted story creates, and only `tests/acceptance` can see it (the suite sends `Expire` itself, and `Expire` is granted only to the Sweeper), yet the body never adds task `acceptance` to `task check`, so the `task check` it names gives the same result with or without the expiry task — .engineering/planning/story/service-acceptance.md:37-38,52; ess/domains/messaging.yaml:228-233
story:http-service — changing one body byte breaks both the request signature (SHA-256 of the body) and the envelope signature, and the body does not say what the request-level refusal answers, so a correct build that refuses at the request layer with a different answer fails the expected `SignatureInvalid` — .engineering/planning/story/http-service.md:41-43, request-signing paragraph under ## Context

What I read: all 9 of the 9 given stories plus the parent epic, in full, with `aep plan artifact show <id>`. I also ran `aep plan artifact kinds`, `aep plan artifact lifecycle story` and `aep plan artifact validate` (valid, 14 artifacts), and read Taskfile.yml, ess/domains/messaging.yaml (refusals of PostMessage, Pending, Settled, Sweeper), ess/scenarios/post-read-answer.yaml, the route list of generated/openapi/openapi/inbox-service.yaml and docs/design/inbox.md (the numbers 50 / ESS-SYNTH-013 are confirmed at lines 111 and 118).

What I could not establish:
- Whether the wire target treats `Expire` steps as unsupported or sends them under some service identity. Either way, switching the expiry task off does not change the suite.
- Out of my lane (scope): the epic context says the two-session exchange and the restart repost are "each carried by its own story". service-acceptance carries both in its outcome, its acceptance checks neither, and cli-client also checks the restart.
- Out of my lane (scope/design): declared-coverage's acceptance requires 0 uncovered refusals, while its outcome lets refusals that need a model change be split into another story. Also, in http-service the read rule, the wait and the expiry task have no acceptance, and signature-verifier's delegation and lapse checks have none.

```findings
- file: .engineering/planning/story/rust-workspace-and-model.md
  line: 40
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance also passes before this story: adding a field to `ess/domains/messaging.yaml` without regenerating already makes today's `task check` fail through `projections-drift` on `generated/openapi`/`generated/schema`, so the acceptance has to say the drift line names `generated/rust`, or that `task projections` ran first"
- file: .engineering/planning/story/cli-client.md
  line: 34
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the restart sequence leaves out `inbox open` by the owner after the restart, and the spec refuses a post to an unknown inbox with `UnknownInbox`, so followed as written `inbox repost` is refused and `inbox pending` never prints the message, even when the work is correct"
- file: .engineering/planning/story/service-acceptance.md
  line: 37
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the failing case, \"built with the expiry task switched off\", names a build switch no drafted story creates, and only `tests/acceptance` can see it (the suite sends `Expire` itself, and `Expire` is granted only to the Sweeper), yet the body never adds task `acceptance` to `task check`, so the `task check` it names gives the same result with or without the expiry task"
- file: .engineering/planning/story/http-service.md
  line: 41
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "changing one body byte breaks both the request signature (SHA-256 of the body) and the envelope signature, and the body does not say what the request-level refusal answers, so a correct build that refuses at the request layer with a different answer fails the expected `SignatureInvalid`"
```
