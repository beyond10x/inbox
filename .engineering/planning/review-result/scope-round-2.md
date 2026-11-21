---
format: aep.planning-md/3
id: review-result:scope-round-2
kind: review-result
status: active
title: scope critic, round 2, stage 2 stories
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
story:declared-coverage — its Outcome still files every ESS gap in the `ess` repository and records the filed item in the design note, and no sentence of the parent asks for that: the revision added only the coverage half to the parent ("The stage completes the suite's declared coverage as well (story:declared-coverage)", .engineering/planning/epic/service-in-memory.md:34-36), so either the parent's Context names the filing or lines 31-32 leave the story — .engineering/planning/story/declared-coverage.md:31

**Round 1 findings, rechecked:**
- **Fixed:** the merged observations. Parent line 30 now says "both carried by story:service-acceptance because they share one harness".
- **Fixed:** the shared outbox. epic:two-hosts-and-wake lines 27-28 now give "The outbox file and the manual `inbox repost`" to stage 2 (story:cli-client).
- **Partly fixed:** declared-coverage. The coverage half is now in the parent at lines 34-36. The filing half is the one finding above.

**What I read:** 1 parent, 9 stories and 4 sibling epics (whole bodies), round 1's review-result:scope-round-1, and docs/design/inbox.md (Shape, What the specification cannot say, Stages). Commands: `aep plan artifact show` on each of those, `aep plan artifact graph`, `aep plan artifact relations`, `aep plan artifact validate` (valid, 18 artifacts), plus numbered reads of the parent and story:declared-coverage files.
- I took 15 promises from the parent and traced all 15 to a story. They are the 14 from round 1 plus the declared coverage from line 34.
- Nothing outside the set claims part of the parent. The only other edges in the graph are `reviews` edges.

**What I could not establish:**
- **story:backend-test-kit:** no parent sentence names a backend test kit. It does check "one backend contract" (parent line 13), and epic:jetstream-backend expects "the same backend test kit", so I did not raise it as reach.
- **story:http-service limits:** the connection, request-size and message-count limits, and the `503` they produce, come from no parent sentence. I read them as how the service is built, not a new outcome, and did not raise them.
- **Repost after a restart:** the parent (line 31) and epic:two-hosts-and-wake (line 29) both check it, locally and across hosts. That is not a duplicate inside this set, so it is not a finding.
- **Outside my lane (design):** story:declared-coverage cites a heading "ESS gaps" in docs/design/inbox.md, but that heading does not exist; the section is "What the specification cannot say (ESS 0.96.0)", line 96. Its summary says "or the gap is closed in ESS", which contradicts "The story changes scenarios only" at line 29.
- **Outside my lane (acceptance):**
  - service-acceptance says a message is in no view 2 sweep intervals after `expires`. That conflicts with a decision that names `default_on_silence`, which waits until `keep_until` (design note line 60-61).
  - Both conformance targets accept scenarios listed in `unsupported.tsv`. The parent says "0 failed and 0 errored", while the design note says "green".
- **Outside my lane (parallel-safety):** Taskfile.yml, AGENTS.md and crates/inbox-conformance are touched by several stories.

```findings
- file: .engineering/planning/story/declared-coverage.md
  line: 31
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its Outcome still files every ESS gap in the `ess` repository and records the filed item in the design note, and no sentence of the parent asks for that: the revision added only the coverage half to the parent (\"The stage completes the suite's declared coverage as well (story:declared-coverage)\", .engineering/planning/epic/service-in-memory.md:34-36), so either the parent's Context names the filing or lines 31-32 leave the story"
```
