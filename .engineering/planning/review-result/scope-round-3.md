---
format: aep.planning-md/3
id: review-result:scope-round-3
kind: review-result
status: active
title: scope critic, round 3 read, stage 2 stories
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

**What I read and ran:** I read 1 parent, 9 stories and 4 sibling epics in full, plus approval-record:stage-2-waves-1-to-5, review-result:scope-round-1 and review-result:scope-round-2. I also read docs/design/inbox.md (Shape, The model, What the specification cannot say, Stage 1 result, Stages) and all of docs/plan/stage-2.md.
- Commands: `aep plan artifact show` on the parent and both earlier reports, `aep plan artifact graph`, `aep plan artifact validate` (valid, 23 artifacts), numbered reads of every file under .engineering/planning/{epic,story}/, `git status`, `git log`, `git show --stat HEAD`, and a grep of the stories, epics and wave plan for filing language ("ess repository", "ESS gaps", "closed in ESS", "issue").
- I took 15 promises from the parent before reading any story, and traced all 15 to a story:
  1. one Rust `inbox serve`
  2. every rule of messaging.yaml
  3. an in-memory backend behind one contract
  4. `inbox` as the only client
  5. rules from the synthesized crate, 85 of 87, with hand-written obligations, ports, listener and client
  6. removal on handling or at `expires`
  7. nothing survives a restart
  8. the sender keeps each message and reposts it
  9. the HTTP suite with 0 failed and 0 errored
  10. the signed exchange between two local sessions
  11. the repost after a restart arriving once
  12. the declared coverage
  13. model types from synthesis
  14. clap derive
  15. clients talk to the service only
  - The signature verifier comes from the parent's summary at .engineering/planning/epic/service-in-memory.md:7.
- Nothing outside the set claims part of the parent. Besides `decomposes` and `depends_on`, the only edges in the graph are `reviews` and `decides`.
- The capability count "85 of 87" matches in all three places: .engineering/planning/epic/service-in-memory.md:17, .engineering/planning/story/rust-workspace-and-model.md:39 and docs/design/inbox.md:142.

**Round 2 finding (story:declared-coverage files ESS gaps): resolved.**
- The Outcome now says "The story changes scenarios only" (.engineering/planning/story/declared-coverage.md:31-32).
- The summary is "`task coverage` exits 0: every scenario synthesis refuses is covered by an authored one" (line 7).
- Scope (lines 57-60) names no `ess` repository.
- The grep finds no filing language in any story, epic or the wave plan.
- docs/design/inbox.md:99-100 says "None is filed in the `ess` repository yet."
- Every line of the Outcome now traces to the parent: .engineering/planning/epic/service-in-memory.md:37-39.

**Round 1 findings, rechecked:** still fixed.
- The merged observations: .engineering/planning/epic/service-in-memory.md:33-34.
- The outbox: .engineering/planning/epic/two-hosts-and-wake.md:27-28 and .engineering/planning/story/cli-client.md:53-55.

**Round 2 "could not establish" notes, one by one:**
- **story:backend-test-kit has no parent sentence:** the story did not change (revision 8). It still checks "one backend contract" (.engineering/planning/epic/service-in-memory.md:16), and .engineering/planning/epic/jetstream-backend.md:15 expects "the same backend test kit". Still not reach.
- **story:http-service limits and `503`:** the story did not change (.engineering/planning/story/http-service.md:45-46). I still read them as how the service is built, not a new outcome. Still not reach.
- **Repost after a restart in two places:** the parent checks it locally (line 34-35). .engineering/planning/epic/two-hosts-and-wake.md:28-29 checks it across hosts. docs/design/inbox.md:133-134 lists it under stage 3. The duplicate sits between the parent and a sibling epic, not inside this set, and it predates the set. It stays not a finding.
- **Design, outside my lane:**
  - The wrong citation of a heading "ESS gaps" is resolved: the body no longer names it.
  - The summary's "or the gap is closed in ESS" is resolved: it was removed.
  - A residual is still open. "The story changes scenarios only" (.engineering/planning/story/declared-coverage.md:31-32) sits next to its own "`task check` runs `task coverage`" (line 32), and Scope lists Taskfile.yml and docs/design/inbox.md (lines 59-60). That is an internal-consistency point for the design lane.
- **Acceptance, outside my lane:**
  - The expiry check against `default_on_silence` is resolved. It is no conflict: .engineering/planning/story/service-acceptance.md:42-43 checks an unanswered `request`, and only a `decision` waits until `keep_until` (docs/design/inbox.md:58-61).
  - The `unsupported.tsv` point still stands. The parent says "0 failed and 0 errored" (.engineering/planning/epic/service-in-memory.md:26), and docs/design/inbox.md:129 says "green". It is in .engineering/planning/story/conformance-gate.md:34-36 and .engineering/planning/story/service-acceptance.md:30-35.
- **Parallel-safety, outside my lane:** several stories touch Taskfile.yml, AGENTS.md and crates/inbox-conformance. docs/plan/stage-2.md:8 says every collision falls between stories of different waves. That is for the parallel-safety lane to confirm.

**What I could not establish:**
- **`inbox keyfile add-delegation`** (.engineering/planning/story/cli-client.md:39-40): no parent sentence names delegation tooling, and .engineering/planning/epic/authority-in-daily-use.md:27-29 puts the handling of delegations in stage 6. That epic claims no tool that creates a delegation entry, so this is no duplicate. I read it as part of a usable verifier (parent summary line 7) and did not raise it.
- **Removal at `expires`:** the parent promises a message "is removed ... when `expires` passes" (line 19). docs/design/inbox.md:60-61 keeps a silence-answered decision until `keep_until`. That conflicts between the parent and its source, not within the set, and judging the parent is outside my rubric.

```findings
[]
```
