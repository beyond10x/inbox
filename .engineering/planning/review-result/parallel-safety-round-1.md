---
format: aep.planning-md/3
id: review-result:parallel-safety-round-1
kind: review-result
status: active
title: parallel-safety critic, round 1, stage 2 stories
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
story:signature-verifier — this story implements the `SignatureVerifier` port, but story:core-and-memory-backend defines that port in `crates/inbox-core`, and no depends_on edge orders the two, so wave 2 holds both and the verifier is written against a trait that does not exist yet; the body needs a depends_on edge to story:core-and-memory-backend, or the port has to move to a crate this story does not wait on (cited, both bodies) — .engineering/planning/story/signature-verifier.md:20, .engineering/planning/story/core-and-memory-backend.md:21
story:signature-verifier — the body requires a typed home for the key file ("an ESS domain or a JSON Schema under `generated/`"), and that write is missing from its scope; if the home is an ESS domain, it lands on `ess/ess-inputs.yaml` and on the hard-coded specification list in `tools/scenarios.py`, which story:declared-coverage claims in the same wave, and it regenerates `generated/rust`, which story:core-and-memory-backend builds on; the body needs to pick the home and scope it, then either add an ordering edge or move the surface so the stories no longer share it (inferred: which of the two homes is chosen) — .engineering/planning/story/signature-verifier.md:41, tools/scenarios.py:607, .engineering/planning/story/declared-coverage.md:49
story:core-and-memory-backend — its acceptance runs the timeline of `ess/scenarios/post-read-answer.yaml`, a file inside the `ess/scenarios/` that story:declared-coverage claims and that `tools/scenarios.py` rewrites in full; no depends_on edge orders the two, so wave 2 holds both (inferred: whether declared-coverage changes this scenario) — .engineering/planning/story/core-and-memory-backend.md:27, .engineering/planning/story/declared-coverage.md:49
story:conformance-gate — its acceptance (0 failed, 0 errored, unsupported exactly as `unsupported.tsv` lists) reads the suite synthesized from `ess/scenarios`, and story:declared-coverage adds scenarios to that suite; no depends_on edge orders the two, and today's wave order comes only from their `Taskfile.yml` collision (inferred: whether the added scenarios pass against the core) — .engineering/planning/story/conformance-gate.md:31, .engineering/planning/story/declared-coverage.md:26

What I read: the 9 stories plus epic:service-in-memory, in full. Commands: `aep plan artifact graph`, `aep plan artifact waves` (5 waves, 13 inferred collisions on `AGENTS.md`/`Taskfile.yml`/`crates/inbox-conformance`, every one already split across waves), `aep plan artifact relations`, `aep plan artifact validate` (valid). I also read `Taskfile.yml`, `ess/ess-inputs.yaml`, `tools/scenarios.py`, `docs/design/inbox.md`, `ess/components.yaml` and `ess/scenarios/post-read-answer.yaml`, and git-grepped for `SignatureVerifier`. Surfaces: 9 cited (every body has a `## Scope` section naming paths), 0 inferred only, 0 unplaced. Acceptance reads traced to a producer in the set: 13, of which 10 are recorded by a depends_on edge (direct or transitive) and 3 are not (findings 1, 3 and 4).

What I could not establish:
- Whether `ess generate synthesize --target rust --layout crate` emits the `SignatureVerifier` trait itself. `generated/rust` does not exist yet. If it does, finding 1 shrinks to the trait shape and the test double.
- `Cargo.lock`: every crate story writes it and none of them scopes it. story:rust-workspace-and-model:56-57 admits this and names the merge remedy, so I report it as a named collision for a person to accept, not as a finding.
- Out of my lane (acceptance/scope): story:service-acceptance's acceptance needs "the service built with the expiry task switched off" (service-acceptance.md:38). story:http-service creates no such switch, so the work would land in `crates/inbox-server` without scoping it. That story runs alone in wave 5, so nothing collides.
- Out of my lane (design): story:declared-coverage may file follow-on model stories (declared-coverage.md:27-29). Those sit outside this set.

```findings
- file: .engineering/planning/story/signature-verifier.md
  line: 20
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this story implements the `SignatureVerifier` port, but story:core-and-memory-backend defines that port in `crates/inbox-core`, and no depends_on edge orders the two, so wave 2 holds both and the verifier is written against a trait that does not exist yet; the body needs a depends_on edge to story:core-and-memory-backend, or the port has to move to a crate this story does not wait on (cited, both bodies)"
- file: .engineering/planning/story/signature-verifier.md
  line: 41
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the body requires a typed home for the key file (an ESS domain or a JSON Schema under `generated/`), and that write is missing from its scope; if the home is an ESS domain, it lands on `ess/ess-inputs.yaml` and on the hard-coded specification list in `tools/scenarios.py`, which story:declared-coverage claims in the same wave, and it regenerates `generated/rust`, which story:core-and-memory-backend builds on; the body needs to pick the home and scope it, then either add an ordering edge or move the surface so the stories no longer share it (inferred: which of the two homes is chosen)"
- file: .engineering/planning/story/core-and-memory-backend.md
  line: 27
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs the timeline of `ess/scenarios/post-read-answer.yaml`, a file inside the `ess/scenarios/` that story:declared-coverage claims and that `tools/scenarios.py` rewrites in full; no depends_on edge orders the two, so wave 2 holds both (inferred: whether declared-coverage changes this scenario)"
- file: .engineering/planning/story/conformance-gate.md
  line: 31
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its acceptance (0 failed, 0 errored, unsupported exactly as `unsupported.tsv` lists) reads the suite synthesized from `ess/scenarios`, and story:declared-coverage adds scenarios to that suite; no depends_on edge orders the two, and today's wave order comes only from their `Taskfile.yml` collision (inferred: whether the added scenarios pass against the core)"
```
