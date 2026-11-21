---
format: aep.planning-md/3
id: epic:service-in-memory
kind: epic
status: draft
title: 'Stage 2: `inbox serve` with the in-memory backend, and the CLI client'
summary: The service of the specification, its own non-persistent backend, the signature verifier and the `inbox` CLI.
revision: 3
---
## Outcome

One Rust service, `inbox serve`, that holds every rule of `ess/domains/messaging.yaml` and stores
messages in its own in-memory backend behind one backend contract, and one CLI, `inbox`, that is
its only client. The rules are the crate `ess generate synthesize` writes (85 of 87 capabilities
on ESS 0.96.0); hand-written code supplies the 2 obligations it leaves, its ports, the listener
and the client. A message is removed when it is handled or when `expires` passes. Nothing
survives a restart of the service; the sender keeps each message until it is handled and posts it
again.

## Acceptance

`task check` runs the synthesized conformance suite against a running `inbox serve` over HTTP with
0 failed and 0 errored, where before this epic the suite runs on the ESS interpreted target only
and the repository holds no service.

## Context

Stage 2 of `docs/design/inbox.md`. Stage 1 delivered the specification and its suite: 92 scenarios
(23 authored), 92 passed on the interpreted target with ESS 0.96.0. The stage also owes two
observations a suite cannot make, both carried by story:service-acceptance because they share one
harness: two local sessions exchange a signed request and reply, and a repost after a restart of
the service arrives once.

The stage completes the suite's declared coverage as well (story:declared-coverage): stage 1
left 50 of the 97 generated scenarios synthesis refuses covered by nothing, and the report's
`conformance_status` is `inconclusive` until they are covered.

## Rule

Model types come from `ess generate synthesize`, never from hand transcription. Running code is
Rust with clap derive. Every client talks to the service only; no client talks to a backend.
