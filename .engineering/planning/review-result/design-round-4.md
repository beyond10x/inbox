---
format: aep.planning-md/3
id: review-result:design-round-4
kind: review-result
status: active
title: design critic, round 4 read of the round 3 revision, stage 2 stories
relations:
- reviews: story:core-and-memory-backend
- reviews: story:conformance-gate
- reviews: story:declared-coverage
revision: 1
---
approve

Both round 3 design findings are resolved, and the revision adds no new design defect. I checked it against the synthesis copy.

- **story:conformance-gate (finding 1): resolved.** The target now supplies both ports: `TryContext` for the 3 branches of `Answer`, and the `PostExternals` trait of `inbox-core` for the 3 branches of `PostMessage`. It answers each of the 6 as the step expects (.engineering/planning/story/conformance-gate.md:27-30). The wrong sentence that put core's `unknown-option` off the path is gone: line 31 now says every other rule of `PostMessage` on that path is core's own code.
- **story:core-and-memory-backend (finding 2): resolved.** The body now says what the generated crate does (.engineering/planning/story/core-and-memory-backend.md:24-41). The context port is asked only about the 3 branches of `Answer`, which matches `ExternalCommand`, whose only variant is `InboxMessagingAnswer` (the synthesis copy, src/behaviour.rs:62-65). It also matches the unmet startup list, which names only `Answer`'s branches (the synthesis copy, src/bin/inbox-service-server.rs:21). The `PostMessage` obligation asks `PostExternals`, which core declares. A harness supplies both ports itself, so core states the seam the gate consumes, and the existing `depends_on` edge from the gate to core records the order.
- **No new design defect.**
  - **Ports:** each one is declared by exactly one story. Core declares `PostExternals` and `SignatureVerifier`; the gate implements `PostExternals`, and story:http-service adapts `SignatureVerifier` (http-service.md:37-38).
  - **Caller per request:** the new paragraph at core-and-memory-backend.md:66-69 matches the private `behaviors` field (the synthesis copy, src/ports/inbox_service.rs:48). It hands the choice to story:http-service, which already owns the listener, so no seam is left without an owner.
  - **`shape-mismatch`:** computing it "from the reply and the stored message" fits the generated `Answer`. That behaviour hands the context only the input (src/behaviour.rs:173), which is why the implementation is built over the backend it reads from (core-and-memory-backend.md:37-38).
- **No other story contradicts the two changed bodies.**
  - story:http-service: its adapter and its statement that `SignatureInvalid` comes from `PostMessage` and `Answer` still fit (http-service.md:34-38).
  - story:service-acceptance: its wire target marking a step `unsupported` when it expects `unknown-option` or `shape-mismatch` from an envelope without that mismatch still fits (service-acceptance.md:34-35).
  - story:signature-verifier: "verify an envelope" covers the reply, which the specification calls the reply envelope (ess/domains/messaging.yaml:158, 526).
  - story:backend-test-kit: it touches neither port.
- **Other changed bodies:** the one-sentence edits to story:declared-coverage and epic:service-in-memory touch nothing in my lane.

What I read and ran:
- `git diff -- .engineering/planning`, all four changed bodies, and the port names grepped across every story.
- `aep plan artifact validate` (valid, 26 artifacts), `aep plan artifact graph` (130 edges, the same 12 `depends_on`) and `aep plan artifact waves` (5 waves, 13 inferred collisions).
- In the synthesis copy from round 3: src/behaviour.rs, src/messaging.rs:1999-2006, src/ports/inbox_service.rs and src/bin/inbox-service-server.rs.

What I could not establish: none in my lane. Out of lane (parallel safety): the 13 inferred collisions on AGENTS.md, Taskfile.yml and crates/inbox-conformance are unchanged.

```findings
[]
```
