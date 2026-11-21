---
format: aep.planning-md/3
id: review-result:parallel-safety-round-2
kind: review-result
status: active
title: parallel-safety critic, round 2, stage 2 stories
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

What I read: all 9 stories in full, plus round 1's parallel-safety review (review-result:parallel-safety-round-1). Commands, each run from the worktree: `aep plan artifact show` for each story, `aep plan artifact waves` (5 waves, 13 inferred collisions, 0 unassessed), `aep plan artifact graph` (12 depends_on edges), `aep plan artifact relations`, and `aep plan artifact validate` (valid). I also read `AGENTS.md` (Commands table), the task list in `Taskfile.yml` and the `ess/scenarios` listing, and git-grepped `ess/` for `KeyFile`, `KeyEntry`, `DelegationEntry` and `SignatureVerifier`.

Surfaces: 9 cited (every body has a `## Scope` section that names paths and matches its typed scope), 0 inferred only, 0 unplaced. Acceptance reads traced to a producer in the set: 11. All 11 are recorded by a depends_on edge, 10 direct and 1 transitive (service-acceptance reads `task check` from rust-workspace-and-model, through cli-client, http-service and core).

Round 1 findings, how each was resolved:
- signature-verifier and the `SignatureVerifier` trait: fixed. `inbox-verify` now depends on no other crate in the workspace, and story:http-service owns the adapter and has depends_on edges to both stories (signature-verifier.md Outcome, http-service.md Outcome "The verifier port").
- The typed home of the key file: fixed. `KeyFile`, `KeyEntry` and `DelegationEntry` already exist at `ess/domains/messaging.yaml:119-147`, so the story writes nothing under `ess/` and nothing under `generated/`.
- core vs declared-coverage on `ess/scenarios/post-read-answer.yaml`: fixed. declared-coverage depends on conformance-gate, which depends on core, so the scenario edits come after core.
- conformance-gate reading the suite declared-coverage grows: fixed by the edge declared-coverage depends_on conformance-gate.

Collisions: all 13 that the waves tool reports fall between stories in different waves. 12 of them are also ordered by a depends_on path. The only pair with no path between them is story:cli-client (wave 4) and story:conformance-gate (wave 3), on the `AGENTS.md` Commands table (cited, both Scope sections). They are separated only because cli-client sits deeper in the chain: rust-workspace, then core, then http-service, then cli-client. Under the derived schedule they never run at once.

What I could not establish:
- story:cli-client and story:conformance-gate share the `AGENTS.md` Commands table with no edge between them. Neither body names the other, and only chain depth keeps them in separate waves. If a revision shortens cli-client's chain, they could land in one wave. I report it as a collision for you to accept or not, not as a finding.
- `Cargo.lock`: every crate story writes it, and only story:rust-workspace-and-model scopes it. That story's Scope section admits the sharing and names the remedy: regenerate the file with `cargo metadata` at merge. A named collision for you to accept, not a finding.
- Out of my lane (acceptance/scope): story:backend-test-kit checks that "two clones of one backend handle read one store" against `MemoryBackend`, and story:core-and-memory-backend's outcome does not say that clones share one store. If they don't, the work would land unscoped in `crates/inbox-core` during wave 3. No other wave-3 story writes that crate, so nothing collides.
- Out of my lane (design): story:declared-coverage may need new scenarios to pass against the core. That could mean edits to `crates/inbox-core` or `unsupported.tsv`, neither of which is in its scope. Its wave-4 neighbour, story:cli-client, touches neither path, so nothing collides.

```findings
[]
```
