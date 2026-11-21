---
format: aep.planning-md/3
id: story:core-and-memory-backend
kind: story
status: implemented
title: 'The core: the 2 obligations, the context port and the in-memory backend'
summary: 'What the synthesized crate leaves open, supplied once: `PostMessage`, `Expire`, the context port and a non-persistent backend behind the generated storage ports.'
relations:
- decomposes: epic:service-in-memory
- depends_on: story:rust-workspace-and-model
scope:
- confidence: cited
  path: crates/inbox-core
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-21T09:36:08Z", actor: "agent:inbox", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":8,"verification":1}}}
---
## Outcome

`crates/inbox-core` completes the generated crate into a working system and writes none of its
rules a second time. It supplies four things:

- The 2 obligations synthesis leaves: the behaviours of `PostMessage` and `Expire`, against their
  generated contracts (`PostMessageBehavior`, `ExpireBehavior`). The generated crate asks its
  context port nothing about `PostMessage`, so this obligation decides the 3 `external:`
  branches of `PostMessage` itself (`unknown-option`, `signature-invalid`,
  `authority-invalid`): it asks a trait this crate declares, `PostExternals`, one question per
  branch.
- The generated context port (`TryContext`): the caller's `principal` and `inbox`, handed in by
  the embedding code, and the answer to the 3 `external:` branches of `Answer`
  (`signature-invalid`, `authority-invalid`, `shape-mismatch`), the only command the generated
  crate asks it about.
- One implementation behind both ports, which answers those 6 branches. `unknown-option` is
  computed from the envelope and `shape-mismatch` from the reply and the stored message.
  The 2 `signature-invalid` and the 2 `authority-invalid` branches are asked of a
  `SignatureVerifier` trait this crate declares. It is built from a caller, a verifier and the
  backend it reads the answered message from, and takes nothing else: no outside code sets
  an answer on it. The tests of this crate run each of the 6 branches twice: once taken, once
  not taken. A harness that must force a branch (story:conformance-gate) supplies both ports
  itself: the generated context port and `PostExternals`.
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

The generated `InboxService` owns its behaviours and gives no access to them after
construction. An embedding with another caller per request either builds the generated system
per request over a clone of `MemoryBackend` or keeps a shared handle to the caller its context
reads; story:http-service chooses.

The tests use a verifier double that accepts or refuses by key id. The real verifier is
story:signature-verifier, a crate that does not depend on this one; story:http-service adapts it
to the trait declared here.

Added while the wave ran (stage 2, wave 2), each after a finding; the crate and the wave order
are as planned:

- One command at a time per store. Every command reads, decides and then writes through
  separate storage calls, so two requests at once over clones of one backend could both be
  accepted. The crate declares a third trait a backend implements, `Exclusive` (run a command
  while no other command runs over this store), and the entry `handle` / `handle_with_events`:
  it takes the section, builds the generated system over a clone of the backend and the
  request's context, and runs the generated `handle`. An embedding with another caller per
  request calls this entry; `system` alone serializes nothing. A context, verifier or
  `PostExternals` must not call the entry over the same store; `MemoryBackend` answers such a
  call with a panic that names the rule. For a later backend the contract is the 2 storage ports
  and `Exclusive`.
- The order in which `PostMessage` refuses is the one the ESS interpreted target answers, measured
  with scenarios in a scratch copy of `ess/`: the input-guarded refusals in declaration order
  (`no-authority`, `already-expired`, `request-without-shape`, `decision-without-options`), then
  the 3 external branches (`unknown-option`, `signature-invalid`, `authority-invalid`), then the
  row refusals (`duplicate`, `unknown-inbox`, `duplicate-key`, `inbox-closed`), then `posted`. So
  the obligation reads no row before the signature is checked. The contract text of `PostMessage`
  in `generated/rust/inbox/PLAN.md` states another order (row refusals first, external branches
  last); the suite of 92 scenarios holds one pair of this order, and a backlog draft asks for
  the rest.
- Choices the specification leaves open and this crate takes: `authority-invalid` is asked only
  when the envelope names a delegation; `unknown-option` is taken for a letter carried twice and
  for a `recommended_option` or `default_on_silence` that names no option; a `lines(n)` reply
  shape fits a body of at most `n` lines; a `schema` reply shape is only checked for a body that
  parses as JSON.

## Scope

- `crates/inbox-core/` (new): `src/{lib,backend,context,obligations}.rs`, `tests/core/`,
  `tests/adversary.rs`, `tests/adversary2.rs`
- `Cargo.lock` (the crate and its dependencies)
