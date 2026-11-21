---
format: aep.planning-md/3
id: story:core-and-memory-backend
kind: story
status: active
title: 'The core: the 2 obligations, the context port and the in-memory backend'
summary: 'What the synthesized crate leaves open, supplied once: `PostMessage`, `Expire`, the context port and a non-persistent backend behind the generated storage ports.'
relations:
- decomposes: epic:service-in-memory
- depends_on: story:rust-workspace-and-model
scope:
- confidence: inferred
  path: crates/inbox-core
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`crates/inbox-core` completes the generated crate into a working system and writes none of its
rules a second time. It supplies three things:

- The 2 obligations synthesis leaves: the behaviours of `PostMessage` and `Expire`, against their
  generated contracts (`PostMessageBehavior`, `ExpireBehavior`).
- The generated context port (`TryContext`): the caller's `principal` and `inbox`, handed in by
  the embedding code, and the answer to each `external:` branch. `unknown-option` and
  `shape-mismatch` are computed here from the envelope and the stored message.
  `signature-invalid` and `authority-invalid` are asked of a `SignatureVerifier` trait this crate
  declares. This context is built from a caller and a verifier and takes nothing else: no
  outside code sets an external answer on it. A harness that must force one
  (story:conformance-gate) supplies the generated context port itself.
- `MemoryBackend`, the first backend. It implements both storage ports, its clones share one
  store, and it is `Send`, so the listener of story:http-service moves it to another thread.
  The generated `MemoryInboxStorage` and `MemoryMessageStorage` keep their rows in an
  `Rc<RefCell>` and stay on the thread that made them.

The backend contract is the generated storage ports, `InboxStorage` and `MessageStorage`: `get`
answers the snapshot stored under an identity, `put` stores or replaces it, `delete` removes it,
`list` answers every stored snapshot in the order the store keeps them. The generated behaviours
and view queries read and write through these ports only. A backend decides no rule: dedup by
`dedup_key` is the `PostMessage` obligation reading `list`, and expiry is the `Expire` obligation.

## Acceptance

`cargo test -p inbox-core` runs the timeline of `ess/scenarios/post-read-answer.yaml` through the
generated `handle` over `MemoryBackend` and reads the answered message in the `Settled` view,
where before this story the behaviour of `PostMessage` is an unmet obligation and no
`PostMessage` stores a message.

## Context

A message leaves the backend in the three places the specification deletes it: `MarkRead` of a
note and `Collect` (generated behaviours) and `Expire` (obligation). `Expire` takes the clock
reading as input, as the specification models it; the task that sends it is story:http-service.

The tests use a verifier double that accepts or refuses by key id. The real verifier is
story:signature-verifier, a crate that does not depend on this one; story:http-service adapts it
to the trait declared here.

## Scope

- `crates/inbox-core/` (new)
