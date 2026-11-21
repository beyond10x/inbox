---
format: aep.planning-md/3
id: review-result:design-round-1
kind: review-result
status: active
title: design critic, round 1, stage 2 stories
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
story:core-and-memory-backend — its outcome has inbox-core hand-implement "the 9 commands and 9 views" over its own `Backend` trait and `MemoryBackend`, but the crate that story:rust-workspace-and-model synthesizes already generates 7 command behaviours and all 9 view queries (`Generated<P>`) over generated per-entity storage ports and an ephemeral memory store, and leaves only `Expire` and `PostMessage` as obligations. The rules and the storage contract therefore exist twice. The fix is for the outcome to name the generated bundle and storage/context ports as its seam and to implement only the 2 obligations — .engineering/planning/story/core-and-memory-backend.md:18-23; `ess generate synthesize --path ess --target rust --layout crate` (ess 0.96.0) PLAN.md:10 "84 capabilities: 82 generated, 2 obligations", src/behaviour.rs:7-8, src/server/memory.rs
story:signature-verifier — its outcome implements the `SignatureVerifier` port that story:core-and-memory-backend defines in inbox-core (the generated crate has no such trait, only `Context::external(…, "signature-invalid")`), and no depends_on edge runs to story:core-and-memory-backend. Fix: add that edge, which puts the verifier in wave 3 and http-service in wave 4, or have this story own the port trait in a crate core then depends on — .engineering/planning/story/signature-verifier.md:20, .engineering/planning/story/core-and-memory-backend.md:22; aep plan artifact graph
story:backend-test-kit — its checks specify a `Backend` contract (duplicate reported on store by `message_id`/`dedup_key`, inbox listing in storing order, due listing at or before the reading) that story:core-and-memory-backend's trait description does not state, and that the generated storage port contradicts (`put` replaces, `list` in store order). The trait is declared in one item and specified in another, and the kit's acceptance against `MemoryBackend` depends on behaviour its producer never promises. Fix: core's outcome states the contract, or the kit checks only what core declares — .engineering/planning/story/backend-test-kit.md:18-22, .engineering/planning/story/core-and-memory-backend.md:20-22
story:signature-verifier — its outcome offers only envelope verification and signing. story:http-service, whose scope excludes crates/inbox-verify, resolves a request signature's key to a principal "through the SignatureVerifier port's key file", so request authentication is split across the two items. Fix: this outcome names the key-to-principal lookup or request-signature check that http-service consumes — .engineering/planning/story/signature-verifier.md:20-26, .engineering/planning/story/http-service.md:26-28
story:http-service — its outcome does not say whether inbox-server builds on or replaces the `server` feature of the generated crate (generated routes, JSON codecs, HTTP/1.1 surface, ephemeral stores, binary `inbox-service-server`) that story:rust-workspace-and-model checks in. As written, the workspace holds two HTTP surfaces for one API — .engineering/planning/story/http-service.md:21-23; `ess generate synthesize --path ess --target rust --layout crate` src/server/http.rs, Cargo.toml `[features] server`

What I read: 9 stories plus epic:service-in-memory, whole bodies (`cat -n .engineering/planning/{epic,story}/*.md`), `aep plan artifact relations`, `aep plan artifact graph`, `aep plan artifact waves` and `aep plan artifact validate` (valid), Taskfile.yml, and docs/design/inbox.md "Decisions", "ESS gaps" and "Stages". I also ran a scratch synthesis of the generated crate with ess 0.96.0 (the version ess-inputs.yaml requires) into a scratch directory outside the tree. I walked 19 edges (10 depends_on, 9 decomposes), including the 4 other epics in the store, which carry no edges. I found no cycle and no serialising chain: the longest path is rust-workspace → core → http-service → cli-client → service-acceptance, and waves 2 and 3 are 3 wide. I found no horizontal-slice defect, because each item has its own demonstrable acceptance. I traced 12 acceptance reads to producers in the set; edges record 11 of them, directly or transitively. The one not recorded is signature-verifier → core-and-memory-backend.

What I could not establish:
- Whether story:rust-workspace-and-model builds the generated crate with the `server` feature enabled; its body does not say (.engineering/planning/story/rust-workspace-and-model.md:31-36).
- Out of lane (acceptance/scope): story:service-acceptance's acceptance reads "the service built with the expiry task switched off", a switch no outcome produces (.engineering/planning/story/service-acceptance.md:37-38, http-service.md:33-34).
- Out of lane (acceptance): in story:http-service's acceptance, one changed body byte breaks the request signature, which is refused "before any command runs". So whether the answer is `SignatureInvalid` is undetermined (.engineering/planning/story/http-service.md:28,41-43).
- Out of lane (scope): story:cli-client and story:service-acceptance both claim the restart-and-repost observation. Also, story:http-service says limits come from flags, but story:cli-client, which owns the `inbox serve` binary, names no flags.
- Out of lane (parallel safety): `aep plan artifact waves` reports 13 inferred collisions on AGENTS.md, Taskfile.yml and crates/inbox-conformance.

```findings
- file: .engineering/planning/story/core-and-memory-backend.md
  line: 18
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome has inbox-core hand-implement \"the 9 commands and 9 views\" over its own `Backend` trait and `MemoryBackend`, but the crate that story:rust-workspace-and-model synthesizes already generates 7 command behaviours and all 9 view queries (`Generated<P>`) over generated per-entity storage ports and an ephemeral memory store, and leaves only `Expire` and `PostMessage` as obligations. The rules and the storage contract therefore exist twice. The fix is for the outcome to name the generated bundle and storage/context ports as its seam and to implement only the 2 obligations"
- file: .engineering/planning/story/signature-verifier.md
  line: 20
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its outcome implements the `SignatureVerifier` port that story:core-and-memory-backend defines in inbox-core (the generated crate has no such trait, only `Context::external(…, \"signature-invalid\")`), and no depends_on edge runs to story:core-and-memory-backend. Fix: add that edge, which puts the verifier in wave 3 and http-service in wave 4, or have this story own the port trait in a crate core then depends on"
- file: .engineering/planning/story/backend-test-kit.md
  line: 18
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its checks specify a `Backend` contract (duplicate reported on store by `message_id`/`dedup_key`, inbox listing in storing order, due listing at or before the reading) that story:core-and-memory-backend's trait description does not state, and that the generated storage port contradicts (`put` replaces, `list` in store order). The trait is declared in one item and specified in another, and the kit's acceptance against `MemoryBackend` depends on behaviour its producer never promises. Fix: core's outcome states the contract, or the kit checks only what core declares"
- file: .engineering/planning/story/signature-verifier.md
  line: 20
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome offers only envelope verification and signing. story:http-service, whose scope excludes crates/inbox-verify, resolves a request signature's key to a principal \"through the SignatureVerifier port's key file\", so request authentication is split across the two items. Fix: this outcome names the key-to-principal lookup or request-signature check that http-service consumes"
- file: .engineering/planning/story/http-service.md
  line: 21
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its outcome does not say whether inbox-server builds on or replaces the `server` feature of the generated crate (generated routes, JSON codecs, HTTP/1.1 surface, ephemeral stores, binary `inbox-service-server`) that story:rust-workspace-and-model checks in. As written, the workspace holds two HTTP surfaces for one API"
```
