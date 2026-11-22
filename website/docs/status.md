---
title: What works
description: What is on main today, with the numbers the gate prints and the path that proves each.
---

# What works

Stage 2 builds the service with the in-memory backend and the command-line client, in 5 waves
([`docs/plan/stage-2.md`](https://github.com/beyond10x/inbox/blob/main/docs/plan/stage-2.md)).
All 5 waves are on `main`, and 3 more: 2 waves that close security findings (waves 6 and 7)
and wave 8, which closes gaps in the checks.
Release 0.1.0 holds waves 1 to 3, release 0.2.0 all 8.

In short: the rules of the inbox run in process and behind an HTTP listener, and the
conformance suite passes against them. One binary starts the service on a loopback address
and is its client ([first use](first-use.md)). No release attaches it, and two security
findings are open ([limits](limits.md)).

## On `main`

Numbers from 2026-11-21, ESS 0.97.1, Rust 1.99.1. Case counts are the passed cases of
`cargo test --workspace` in one run of `task check`.

| Part | What it does | Number | Proof |
|---|---|---|---|
| The specification | The domain `inbox.messaging`: inboxes, messages, their lifecycle, every refusal | 2 entities, 9 commands, 9 views, 10 events, 14 errors, 2 actors; 38 authored scenarios | [`ess/`](https://github.com/beyond10x/inbox/tree/main/ess), `task spec` |
| The generated crate | Model types, command behaviours, view queries, storage and context ports, route table and JSON codecs, written by ESS from `ess/` | 86 of 88 capabilities generated; 2 obligations left to hand-written code (`PostMessage`, `Expire`) | [`generated/rust/inbox/`](https://github.com/beyond10x/inbox/tree/main/generated/rust/inbox), `task drift` |
| The core | The 2 obligations over a backend; one command at a time per store | 128 cases | [`crates/inbox-core/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-core) |
| The in-memory backend | Both storage ports in memory; part of the core crate (`MemoryBackend`) | in the 128 cases above, and in the test kit's | [`crates/inbox-core/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-core) |
| The verifier | Ed25519 signatures over RFC 8785 canonical JSON for envelopes, key file entries and requests; the key ring | 63 cases | [`crates/inbox-verify/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-verify), [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md) |
| The backend test kit | The checks every backend passes: the 2 storage ports and the command section | 29 cases | [`crates/inbox-backend-testkit/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-backend-testkit) |
| The HTTP listener | A library: request signatures, the read rule, the wait on `Pending`, the expiry task, the limits; loopback only | 60 cases, none ignored | [`crates/inbox-server/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-server), [`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md) |
| The binary `inbox` | `inbox serve` starts the listener over the in-memory backend on a loopback address; the other commands are the client: signed requests, the outbox file and its epochs file, exit statuses 0 to 4 | 83 cases, none ignored | [`crates/inbox-cli/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-cli), [first use](first-use.md) |
| Conformance, interpreted | The suite ESS synthesizes from `ess/`, run on ESS's interpreted target | 111 scenarios: 111 passed, 0 failed, 0 error, 0 unsupported | `task conform-interpreted` |
| The conformance gate | The same suite run against the core in process, the report written by ESS and held to the suite | 111 scenarios: 111 passed, 0 failed, 0 error, 0 unsupported; `conformance_status: passed` | [`crates/inbox-conformance/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-conformance), `task conform` |
| The service over HTTP | The same suite against a running `inbox serve`, a fresh one per scenario; every request and envelope signed, each answer held to the HTTP status of the generated contract | 111 scenarios: 31 passed, 0 failed, 0 error, 80 unsupported, each listed with its reasons; `conformance_status: failed` (see below) | [`unsupported.tsv`](https://github.com/beyond10x/inbox/blob/main/crates/inbox-conformance/unsupported.tsv), `task conform` |
| Acceptance | 3 checks against the built binary: a signed request and its reply between 2 keys; a read note and an expired request in no view; a repost after a restart, and a second repost that posts nothing | 3 passed | [`tests/acceptance/`](https://github.com/beyond10x/inbox/tree/main/tests/acceptance), `task acceptance` |
| The conformance runner | Reads the suite, runs it against both targets, holds each report | 78 cases, 1 ignored | [`crates/inbox-conformance/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-conformance) |
| Declared coverage | Every refusal of the specification is claimed by an authored scenario; `task check` fails on one that is not | 93 refused generated scenarios, 93 covered; the suite reports `conformance_status: passed` on the interpreted target | `task coverage` |
| Repository tasks | The drift check of the generated crate | 35 cases | [`crates/inbox-xtask/`](https://github.com/beyond10x/inbox/tree/main/crates/inbox-xtask) |

## What wave 8 added

Wave 8 of stage 2 closes gaps in the checks that the adversary passes of wave 3 found, and
narrows one limit of the client.

- The service sends the header `Inbox-Epoch` on every answer, and `inbox repost` leaves alone
  a kept envelope that the epochs file records at the epoch the service names now. The limit
  that stays is on the [limits](limits.md) page.
- 2 authored scenarios: a decision its recipient read is decided on silence at `expires`, and
  the row a decision decided on silence leaves keeps its other fields. Both are unsupported
  over HTTP, as steps of the Sweeper.
- 6 authored scenarios hold the order in which `PostMessage` refuses when 2 refusals apply:
  the signature, the option, the delegation and the reply shape are refused before any row is
  read, and the 5 row refusals answer in one order. All 6 run over HTTP. The suite has 38
  authored scenarios and 111 in all.
- Cases of the core: one occurrence of each event `Expire` hands back, the order of the 3
  external refusals of `PostMessage`, a read decision decided on silence, and the fields
  `Expire` leaves unchanged.

## Why 31 of 111 over HTTP

Against the core in process all 111 scenarios pass, and ESS reports `passed`. Against the
running service a scenario runs only where every step can be sent and witnessed over HTTP.
80 cannot: a caller whose principal and inbox differ (68 scenarios; over HTTP both
are the principal of the caller's key), a step as the Sweeper (38; only the service's own
expiry task is the Sweeper), a step as no actor (9; a request without a signature is
answered `401`), a claim on the stored `signature` (7; the run signs each envelope with
its own key), one delegation id for two pairs of sender and principal (1; an id names one
entry of the key file). A scenario can have several reasons. ESS reports a run with an
unsupported scenario as `failed`; `task conform` fails unless the unsupported scenarios and their
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

- No release attaches a binary: build it from source ([first use](first-use.md)).
- No persistent backend, no second host, no wake-up, no TLS.
- No deployment.
