---
title: What works
description: What is on main today, with the numbers the gate prints and the path that proves each.
---

# What works

Stage 2 builds the service with the in-memory backend and the command-line client, in 5 waves
([`docs/plan/stage-2.md`](https://github.com/beyond10x/inbox/blob/main/docs/plan/stage-2.md)).
Waves 1 to 3 are on `main`. Wave 4 (the command-line client, declared coverage) and wave 5 (the
suite over HTTP against a running service) are not.

In short: the rules of the inbox run in process and behind an HTTP listener, and the
conformance suite passes against them. Nothing starts the service as a program, there is no
client, and four security findings are open ([limits](limits.md)).

## On `main`

Numbers from 2026-11-21, ESS 0.96.0, Rust 1.99.1. Case counts are the passed cases of
`cargo test --workspace` in one run of `task check`.

| Part | What it does | Number | Proof |
|---|---|---|---|
| The specification | The domain `inbox.messaging`: inboxes, messages, their lifecycle, every refusal | 2 entities, 9 commands, 9 views, 10 events, 13 errors, 2 actors; 23 authored scenarios | [`ess/`](https://github.com/beyond10x/inbox/tree/main/ess), `task spec` |
| The generated crate | Model types, command behaviours, view queries, storage and context ports, route table and JSON codecs, written by ESS from `ess/` | 85 of 87 capabilities generated; 2 obligations left to hand-written code (`PostMessage`, `Expire`) | [`generated/rust/inbox/`](https://github.com/beyond10x/inbox/tree/main/generated/rust/inbox), `task drift` |
| The core | The 2 obligations over a backend; one command at a time per store | 48 cases | [`crates/inbox-core/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-core) |
| The in-memory backend | Both storage ports in memory; part of the core crate (`MemoryBackend`) | in the 48 cases above, and in the test kit's | [`crates/inbox-core/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-core) |
| The verifier | Ed25519 signatures over RFC 8785 canonical JSON for envelopes, key file entries and requests; the key ring | 63 cases | [`crates/inbox-verify/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-verify), [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md) |
| The backend test kit | The checks every backend passes: the 2 storage ports and the command section | 29 cases | [`crates/inbox-backend-testkit/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-backend-testkit) |
| The HTTP listener | A library: request signatures, the read rule, the wait on `Pending`, the expiry task, the limits; loopback only | 38 cases, 3 ignored (the open findings 1 to 3 of [limits](limits.md)) | [`crates/inbox-server/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-server), [`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md) |
| Conformance, interpreted | The suite ESS synthesizes from `ess/`, run on ESS's interpreted target | 92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported | `task conform-interpreted` |
| The conformance gate | The same suite run against the core in process, the report written by ESS and held to the suite | 92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported; `conformance_status: inconclusive` | [`crates/inbox-conformance/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-conformance), `task conform` |
| Repository tasks | The drift check of the generated crate | 35 cases | [`crates/inbox-xtask/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-xtask) |

## Why "inconclusive"

Every scenario passes, and the report still says `conformance_status: inconclusive`. That is
ESS's word for a suite that does not claim every refusal of the specification: 50 refusals are
claimed by no scenario. `task coverage` lists them; it exits 1 while one is left and is not part
of `task check`. `story:declared-coverage`, wave 4, owns them. Stage 2 is not done while the
status is `inconclusive`.

## The listener's defaults

From `docs/design/service.md`; each is set by the embedding.

| Limit | Default |
|---|---|
| connections at once | 64 |
| a request arrives in full within | 10 s |
| request body | 1 MiB |
| messages in the store | 10 000 |
| longest wait on `Pending` | 60 s |
| expiry sweep every | 10 s |
| an answered message waits for its sender | 24 h |
| request signature window | 60 s either side of the clock |

## Not on `main`

- No binary: `inbox serve` and the client are `story:cli-client` (wave 4).
- No run of the suite over HTTP against a running service (`story:service-acceptance`, wave 5).
- No persistent backend, no second host, no wake-up, no TLS.
- No deployment.
