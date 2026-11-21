---
format: aep.planning-md/3
id: review-result:design-round-3
kind: review-result
status: active
title: design critic, round 3 read, stage 2 stories
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
story:conformance-gate — its target forces each `external:` branch through its own `TryContext`, but the generated `TryContext` is asked only for the three branches of `Answer`. The three of `PostMessage` (`unknown-option`, `signature-invalid`, `authority-invalid`) are decided inside core's `PostMessage` obligation, which asks core's `SignatureVerifier`, so the gate's per-step answers never reach them. A key-id double cannot refuse the step of `bad-signature-refused.yaml`, because it signs with `key-1` and the same placeholder value as the accepted posts. Fix: the gate states that it hands core's `PostMessage` a `SignatureVerifier` double that answers as the step expects, or it lists those steps in `unsupported.tsv`. It also drops the sentence that core's `unknown-option` computation is off its path, because `PostMessage` is on that path — .engineering/planning/story/conformance-gate.md:27-31, .engineering/planning/story/core-and-memory-backend.md:26-32, ess/scenarios/bad-signature-refused.yaml:43-46, ess/scenarios/post-read-answer.yaml:49; the synthesis copy src/behaviour.rs:62-65, src/behaviour.rs:97-104, src/bin/inbox-service-server.rs:21
story:core-and-memory-backend — its outcome puts `unknown-option` and the signature and authority answers of `PostMessage` in the generated context port, but the generated crate asks that port only about `Answer`: `ExternalCommand` has the single variant `InboxMessagingAnswer`, and `PostMessage` is an obligation whose contract holds its own external branches. Fix: the outcome states that the `PostMessage` obligation decides its three external branches itself, and names how an embedding other than core's own context (story:conformance-gate) supplies the verifier those branches ask — .engineering/planning/story/core-and-memory-backend.md:24-32; the synthesis copy src/behaviour.rs:62-65, src/messaging.rs:1999-2006, src/bin/inbox-service-server.rs:21

Round 2 findings:
- (1) conformance-gate external outcomes: **partly fixed.** It holds for `Answer`: the gate supplies its own `TryContext` (conformance-gate.md:27-29), and core says no outside code sets an answer on its context (core-and-memory-backend.md:30-32). It does not hold for `PostMessage`: its branches never go through `TryContext`, which is the two findings above.
- (2) backend-test-kit clone sharing: **fixed.** Core now states that its clones share one store (core-and-memory-backend.md:33-34), which is what the kit checks (backend-test-kit.md:25-26).

Round 2 "could not establish" notes:
- KeyFile/KeyEntry/DelegationEntry codecs: **resolved.** rust-workspace-and-model.md:38 lists them, and signature-verifier.md:25 reads them through the `server` feature. They exist and are public in the synthesis copy: src/server/wire.rs:195-216, 300-321 and 352-383, and src/server.rs (`pub mod wire`, `pub mod json`).
- Why core has its own `MemoryBackend`: **resolved.** core-and-memory-backend.md:33-36 states that it is `Send` and gives the generated store's `Rc<RefCell>` as the reason. The synthesis copy confirms the `Rc<RefCell>` at src/server/memory.rs:9 and 25.
- Wire target signing (out of lane): **resolved.** service-acceptance.md:31-35.
- 13 inferred collisions (out of lane, parallel safety): still 13.

What I read: the 9 stories, epic:service-in-memory, approval-record:stage-2-waves-1-to-5 and review-result:design-round-1 and design-round-2, all whole bodies. I ran `aep plan artifact relations`, `graph`, `waves` and `validate` (result: valid). I read the external branches in ess/domains/messaging.yaml and grepped ess/scenarios for every external outcome and key id. I synthesized the crate with ESS 0.96.0 into session scratch ("87 capabilities: 85 generated, 2 obligation(s), 0 refused") and read in that copy src/behaviour.rs, src/server/memory.rs, src/system.rs, src/ports/inbox_service.rs, src/bin/inbox-service-server.rs, src/server.rs, src/lib.rs, Cargo.toml features, the codec functions in src/server/wire.rs, and the signature of `handle` in src/server/inbox_service.rs.

Graph: 103 edges (12 depends_on, 9 decomposes, 72 reviews, 10 decides), walked including the approval record, the review results and the 4 other epics. No cycle. No serialising chain: the waves are 1, 2, 3, 2, 1 wide. No horizontal slice and no split abstraction other than the `PostMessage` externals seam above.

Acceptance reads: 14 traced to a producer in the set. Edges record all 14, directly or transitively.

What I could not establish:
- How a per-request caller reaches the context inside `InboxService`: the `behaviors` field is private (the synthesis copy, src/ports/inbox_service.rs:48). Building a System per request over a clone of `MemoryBackend` would work, because clones share the store, but no body says which way story:http-service does it. This is not a defect of the shape.
- Out of lane (acceptance): no acceptance line checks core's own computation of `unknown-option` or `shape-mismatch`. Core's acceptance runs post-read-answer only (core-and-memory-backend.md:46-49), and the gate's sentence "its own tests check it" (conformance-gate.md:30) names a check that core does not state.
- Out of lane (parallel safety): `aep plan artifact waves` reports 13 inferred collisions on AGENTS.md, Taskfile.yml and crates/inbox-conformance.

```findings
- file: .engineering/planning/story/conformance-gate.md
  line: 27
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its target forces each `external:` branch through its own `TryContext`, but the generated `TryContext` is asked only for the three branches of `Answer`; the three of `PostMessage` (`unknown-option`, `signature-invalid`, `authority-invalid`) are decided inside core's `PostMessage` obligation, which asks core's `SignatureVerifier`, so the gate's per-step answers never reach them, and a key-id double cannot refuse the step of `bad-signature-refused.yaml`, which signs with `key-1` and the same placeholder value as the accepted posts. Fix: the gate states that it hands core's `PostMessage` a `SignatureVerifier` double that answers as the step expects, or it lists those steps in `unsupported.tsv`, and it drops the sentence that core's `unknown-option` computation is off its path"
- file: .engineering/planning/story/core-and-memory-backend.md
  line: 26
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome puts `unknown-option` and the signature and authority answers of `PostMessage` in the generated context port, but the generated crate asks that port only about `Answer` (`ExternalCommand` has the single variant `InboxMessagingAnswer`; `PostMessage` is an obligation whose contract holds its own external branches). Fix: the outcome states that the `PostMessage` obligation decides its three external branches itself, and names how an embedding other than core's own context (story:conformance-gate) supplies the verifier those branches ask"
```
