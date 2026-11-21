---
format: aep.planning-md/3
id: review-result:design-round-2
kind: review-result
status: active
title: design critic, round 2, stage 2 stories
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
story:conformance-gate — its outcome configures "the caller of each step and the suite's external outcomes" into the context port, but story:core-and-memory-backend's context port computes `unknown-option` and `shape-mismatch` itself and asks a `SignatureVerifier` for the other two, and offers no way to set an external answer from outside; the fix is either for core's outcome to name that injection point, or for the gate to say it drives the externals only through a verifier double and the step's own envelope — .engineering/planning/story/conformance-gate.md:22-24, .engineering/planning/story/core-and-memory-backend.md:23-27
story:backend-test-kit — its contract still includes "two clones of one backend handle read one store", which the contract in story:core-and-memory-backend (get/put/delete/list only) does not state, so the kit's acceptance against `MemoryBackend` checks something its producer never promises; the fix is for core's outcome to state clone sharing, or for the kit to drop that check — .engineering/planning/story/backend-test-kit.md:18-23, .engineering/planning/story/core-and-memory-backend.md:30-32

Round 1 status: four of the five round-1 findings are fixed. Core now implements only the 2 obligations over the generated ports (core-and-memory-backend.md:18-34). The verifier no longer implements core's trait; http-service adapts it instead (signature-verifier.md:20, http-service.md:34-35). Request signing and key-to-principal lookup moved into the verifier (signature-verifier.md:29-31). http-service builds on the generated `server` feature (http-service.md:21-26). The fifth, the backend-test-kit finding, is fixed in part: the dedup, insertion-order and due-listing checks are gone, and only the clone-sharing claim is left, stated above.

What I read: 9 stories plus epic:service-in-memory and review-result:design-round-1, whole bodies (`cat -n .engineering/planning/story/*.md`). I ran `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact waves` and `aep plan artifact validate` (result: valid). I also read the Taskfile.yml tasks, `ess/domains/messaging.yaml` (key-file types, `external:` branches) and `ess/scenarios/bad-signature-refused.yaml`. In the synthesis copy I read PLAN.md, src/server.rs, src/server/entry.rs, src/server/memory.rs, src/system.rs, src/bin/inbox-service-server.rs, and the trait list of src/behaviour.rs.

Graph walk: 57 edges (12 depends_on, 9 decomposes, 36 reviews), including the review-results and the 4 other epics, which carry no edges. No cycle. No serialising chain: waves are 1, 2, 3, 2, 1 wide, and the longest path is rust-workspace → core → conformance-gate → declared-coverage → service-acceptance (rust-workspace → verifier → http-service → cli-client → service-acceptance is as long). No horizontal-slice defect: each item has its own demonstrable acceptance.

Acceptance reads: I traced 11 to a producer in the set. Edges record all 11, directly or transitively. Two of those producers do not state the contract the reader depends on (the two findings above).

What I could not establish:
- Whether synthesis writes JSON codecs for `KeyFile`/`KeyEntry`/`DelegationEntry`. No command or view references those types, and the synthesis copy predates them. story:signature-verifier reads them "through the generated codecs" (signature-verifier.md:21-22), which story:rust-workspace-and-model does not list. The codecs live in the `server` feature (src/server/wire.rs).
- Why core writes its own `MemoryBackend` when the generated crate already ships `MemoryInboxStorage`/`MemoryMessageStorage`, which are shared by clones but `Rc<RefCell>`, so not `Send` (src/server/memory.rs:7-30). Core's body names no reason. If the reason is concurrency for story:http-service, no body states that `MemoryBackend` must be `Send`.
- Out of lane (acceptance/scope): the scenarios carry a fixed placeholder signature (`value: c2lnbmF0dXJl`, ess/scenarios/bad-signature-refused.yaml). For story:service-acceptance to run the suite over HTTP against the real verifier, its wire target must sign every envelope and request with keys from a key file the service starts with. No body says so (service-acceptance.md:26-28).
- Out of lane (parallel safety): `aep plan artifact waves` reports 13 inferred collisions on AGENTS.md, Taskfile.yml and crates/inbox-conformance.

```findings
- file: .engineering/planning/story/conformance-gate.md
  line: 22
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome configures \"the caller of each step and the suite's external outcomes\" into the context port, but story:core-and-memory-backend's context port computes `unknown-option` and `shape-mismatch` itself and asks a `SignatureVerifier` for the other two, and offers no way to set an external answer from outside; the fix is either for core's outcome to name that injection point, or for the gate to say it drives the externals only through a verifier double and the step's own envelope"
- file: .engineering/planning/story/backend-test-kit.md
  line: 18
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its contract still includes \"two clones of one backend handle read one store\", which the contract in story:core-and-memory-backend (get/put/delete/list only) does not state, so the kit's acceptance against `MemoryBackend` checks something its producer never promises; the fix is for core's outcome to state clone sharing, or for the kit to drop that check"
```
