---
format: aep.planning-md/3
id: review-result:acceptance-round-2
kind: review-result
status: active
title: acceptance critic, round 2, stage 2 stories
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
story:cli-client — the acceptance's sequence leaves out `inbox open` by the recipient, and a freshly started service holds no inbox, so `inbox post` as written is refused with `UnknownInbox` and `inbox pending` never prints the message even when the work is correct; the acceptance needs to name the open step before the post — .engineering/planning/story/cli-client.md:44 ("`inbox post` to a running `inbox serve` followed by `inbox pending` for the recipient prints the posted message"); ess/domains/messaging.yaml:312,428-430

Round-1 status: 3 of the 4 round-1 findings are fixed. rust-workspace-and-model now names `task projections` first and a drift line naming `generated/rust`. service-acceptance drops the expiry build switch, and `task check` now runs `task acceptance`. In http-service both requests are now "correctly signed", and `signature-invalid` is decided before the row is read, ahead of `duplicate` (messaging.yaml:413-421), so the second post really is answered `SignatureInvalid`. The cli-client finding was only partly fixed: the restart sequence moved to service-acceptance, and that version does include `inbox open`. But cli-client's new acceptance has the same missing open step, so I kept it as a warning.

What I read: all 9 of the 9 stories in full with `aep plan artifact show story:<id>`. I also ran `aep plan artifact kinds`, `aep plan artifact lifecycle story` and `aep plan artifact validate` (valid, 18 artifacts), and read review-result:acceptance-round-1, Taskfile.yml, the commands section of ess/domains/messaging.yaml and ess/scenarios/post-read-answer.yaml. I checked the "before" state of declared-coverage by running `ess verify conform synthesize --path ess --scenarios ess --suite-format 5` (ess 0.96.0) with its output in a scratch directory. It exits 1 with 97 refusals, 47 of them covered, and 50 top-level `ESS-SYNTH-013` refusals uncovered, which matches the story.

What I could not establish:
- core-and-memory-backend says that before the story the generated `handle` "answers `PostMessage` with an unmet obligation". I did not synthesize the crate, so I have not seen whether an unmet obligation surfaces as an answer or as a compile or startup failure. Either way the before and after states differ.
- rust-workspace-and-model says that before the story `task check` "passes in that state". That only holds if the added field is optional: a required field would also break `conform-interpreted` before the story. The "names `generated/rust`" clause still tells the two cases apart, so I did not record this as a finding.
- Out of my lane (scope): an empty `tests/acceptance` passes service-acceptance's acceptance, because the acceptance names neither the signed exchange nor the restart repost that its outcome lists.
- Out of my lane (scope/design): declared-coverage's acceptance requires 0 uncovered refusals, but its outcome lets refusals that need a model change move to a separate story. In http-service, the read rule, the wait and the expiry task have no acceptance. In signature-verifier, the delegation and lapse checks have none.

```findings
- file: .engineering/planning/story/cli-client.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance's sequence leaves out `inbox open` by the recipient, and a freshly started service holds no inbox, so `inbox post` as written is refused with `UnknownInbox` and `inbox pending` never prints the message even when the work is correct; the acceptance needs to name the open step before the post"
```
