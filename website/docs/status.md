---
title: What works
description: What is on main today, with the numbers the gate prints and the path that proves each.
---

# What works

Stage 2 builds the service with the in-memory backend and the command-line client, in 5 waves
([`docs/plan/stage-2.md`](https://github.com/beyond10x/inbox/blob/main/docs/plan/stage-2.md)).
All 5 waves are on `main`; release 0.1.0 holds waves 1 to 3.

In short: the rules of the inbox run in process and behind an HTTP listener, and the
conformance suite passes against them. One binary starts the service on a loopback address
and is its client ([first use](first-use.md)). No release holds it, and four security
findings are open ([limits](limits.md)).

## On `main`

Numbers from 2026-11-21, ESS 0.96.0, Rust 1.99.1. Case counts are the passed cases of
`cargo test --workspace` in one run of `task check`.

| Part | What it does | Number | Proof |
|---|---|---|---|
| The specification | The domain `inbox.messaging`: inboxes, messages, their lifecycle, every refusal | 2 entities, 9 commands, 9 views, 10 events, 13 errors, 2 actors; 24 authored scenarios | [`ess/`](https://github.com/beyond10x/inbox/tree/main/ess), `task spec` |
| The generated crate | Model types, command behaviours, view queries, storage and context ports, route table and JSON codecs, written by ESS from `ess/` | 85 of 87 capabilities generated; 2 obligations left to hand-written code (`PostMessage`, `Expire`) | [`generated/rust/inbox/`](https://github.com/beyond10x/inbox/tree/main/generated/rust/inbox), `task drift` |
| The core | The 2 obligations over a backend; one command at a time per store | 48 cases | [`crates/inbox-core/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-core) |
| The in-memory backend | Both storage ports in memory; part of the core crate (`MemoryBackend`) | in the 48 cases above, and in the test kit's | [`crates/inbox-core/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-core) |
| The verifier | Ed25519 signatures over RFC 8785 canonical JSON for envelopes, key file entries and requests; the key ring | 63 cases | [`crates/inbox-verify/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-verify), [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md) |
| The backend test kit | The checks every backend passes: the 2 storage ports and the command section | 29 cases | [`crates/inbox-backend-testkit/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-backend-testkit) |
| The HTTP listener | A library: request signatures, the read rule, the wait on `Pending`, the expiry task, the limits; loopback only | 38 cases, 3 ignored (the open findings 1 to 3 of [limits](limits.md)) | [`crates/inbox-server/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-server), [`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md) |
| The binary `inbox` | `inbox serve` starts the listener over the in-memory backend on a loopback address; the other commands are the client: signed requests, the outbox file, exit statuses 0 to 4 | 52 cases, 1 ignored (`story:repost-does-not-redeliver-a-handled-note`) | [`crates/inbox-cli/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-cli), [first use](first-use.md) |
| Conformance, interpreted | The suite ESS synthesizes from `ess/`, run on ESS's interpreted target | 93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported | `task conform-interpreted` |
| The conformance gate | The same suite run against the core in process, the report written by ESS and held to the suite | 93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported; `conformance_status: passed` | [`crates/inbox-conformance/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-conformance), `task conform` |
| The service over HTTP | The same suite against a running `inbox serve`, a fresh one per scenario; every request and envelope signed, each answer held to the HTTP status of the generated contract | 93 scenarios: 20 passed, 0 failed, 0 error, 73 unsupported, each listed with its reasons; `conformance_status: failed` (see below) | [`unsupported.tsv`](https://github.com/beyond10x/inbox/blob/main/crates/inbox-conformance/unsupported.tsv), `task conform` |
| Acceptance | 3 checks against the built binary: a signed request and its reply between 2 keys; a read note and an expired request in no view; a repost after a restart | 3 passed | [`tests/acceptance/`](https://github.com/beyond10x/inbox/tree/main/tests/acceptance), `task acceptance` |
| The conformance runner | Reads the suite, runs it against both targets, holds each report | 78 cases, 1 ignored | [`crates/inbox-conformance/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-conformance) |
| Declared coverage | Every refusal of the specification is claimed by an authored scenario; `task check` fails on one that is not | 89 refused generated scenarios, 89 covered; the suite reports `conformance_status: passed` on the interpreted target | `task coverage` |
| Repository tasks | The drift check of the generated crate | 35 cases | [`crates/inbox-xtask/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-xtask) |

## Why 20 of 93 over HTTP

Against the core in process all 93 scenarios pass, and ESS reports `passed`. Against the
running service a scenario runs only where every step can be sent and witnessed over HTTP.
73 cannot: a caller whose principal and inbox differ (64 scenarios; over HTTP both
are the principal of the caller's key), a step as the Sweeper (35; only the service's own
expiry task is the Sweeper), a step as no actor (9; a request without a signature is
answered `401`), a claim on the stored `signature` (7; the run signs each envelope with
its own key). A scenario can have several reasons. ESS reports a run with an unsupported
scenario as `failed`; `task conform` fails unless the unsupported scenarios and their
reasons are those of the list, and no scenario failed or errored.

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

- No release holds the binary: build it from source ([first use](first-use.md)).
- No persistent backend, no second host, no wake-up, no TLS.
- No deployment.
