---
format: aep.planning-md/3
id: review-result:scope-round-1
kind: review-result
status: active
title: scope critic, round 1, stage 2 stories
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
story:service-acceptance — the parent says the stage's two observations are "each carried by its own story", but this one story carries both (the signed request and reply between two clients, and the repost after a restart arriving once), and no body records the decision to merge them — .engineering/planning/epic/service-in-memory.md:28
story:cli-client — its outbox file and `inbox repost` are the "client's outbox that reposts after a reconnect" that epic:two-hosts-and-wake claims for stage 3, and its Context hands only the harness plugin to stage 3, so two artifacts will both be marked done for one outbox — .engineering/planning/epic/two-hosts-and-wake.md:27
story:declared-coverage — no sentence of the parent asks for this story's outcome (covering the 50 `ESS-SYNTH-013` refusals, filing ESS gaps in the `ess` repository); only docs/design/inbox.md:95-96 gives it to stage 2, so either the parent's Context names it or the story leaves the set — .engineering/planning/story/declared-coverage.md:26

What I read: 1 parent and 9 stories, whole bodies, plus the 4 sibling epics and docs/design/inbox.md (Shape, ESS gaps, Stages). Commands: `aep plan artifact show epic:service-in-memory`, `aep plan artifact kinds`, `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact validate` (result: valid, 14 artifacts), and numbered reads of every .engineering/planning/{epic,story}/*.md. I took 14 promises from the parent and traced all 14 to at least one story:
- one Rust `inbox serve`
- every rule of messaging.yaml
- an in-memory backend behind one contract
- the `inbox` CLI as the only client
- removal on handling or at `expires`
- nothing survives a restart
- the sender keeps each message and reposts it
- the acceptance over HTTP with 0 failed and 0 errored
- the signed-exchange observation
- the restart-repost observation
- the model comes from synthesis
- clap derive
- clients talk to the service only
- the signature verifier (from the summary)

The only one not honoured as written is the "own story each" division (finding 1).

What I could not establish:
- Whether "two local sessions" (parent line 28, design note line 125) is met by two `inbox` clients in an automated test (service-acceptance line 30). I did not raise it as a narrowing.
- Who owns repost-after-restart. Parent line 29 owes it to stage 2, design note line 128-129 puts it in stage 3, and epic:two-hosts-and-wake:28 checks it across hosts. That is a conflict between the parent and its source and between the epics, so it predates this set and judging the parent's promises is not my job.
- The CLI exposes no `MarkDelivered`, `Inboxes`, `Unanswered` or `Settled`. The parent does not promise the CLI covers every command, and `MarkDelivered` is for the adapters (messaging.yaml:438), so it is not a finding.
- Outside my lane:
  - acceptance: service-acceptance lets scenarios be `unsupported` over the wire. That is within the parent's literal "0 failed and 0 errored", while the design note says "green".
  - parallel-safety: several stories touch Taskfile.yml, AGENTS.md and crates/inbox-conformance.

```findings
- file: .engineering/planning/epic/service-in-memory.md
  line: 28
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the parent says the stage's two observations are \"each carried by its own story\", but this one story carries both (the signed request and reply between two clients, and the repost after a restart arriving once), and no body records the decision to merge them"
- file: .engineering/planning/epic/two-hosts-and-wake.md
  line: 27
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outbox file and `inbox repost` are the \"client's outbox that reposts after a reconnect\" that epic:two-hosts-and-wake claims for stage 3, and its Context hands only the harness plugin to stage 3, so two artifacts will both be marked done for one outbox"
- file: .engineering/planning/story/declared-coverage.md
  line: 26
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "no sentence of the parent asks for this story's outcome (covering the 50 `ESS-SYNTH-013` refusals, filing ESS gaps in the `ess` repository); only docs/design/inbox.md:95-96 gives it to stage 2, so either the parent's Context names it or the story leaves the set"
```
