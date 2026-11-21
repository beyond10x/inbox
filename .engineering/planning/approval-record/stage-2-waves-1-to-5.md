---
format: aep.planning-md/3
id: approval-record:stage-2-waves-1-to-5
kind: approval-record
status: draft
title: 'Stage 2 approved: waves 1 to 5 in order'
relations:
- decides: epic:service-in-memory
- decides: story:rust-workspace-and-model
- decides: story:core-and-memory-backend
- decides: story:signature-verifier
- decides: story:backend-test-kit
- decides: story:conformance-gate
- decides: story:http-service
- decides: story:cli-client
- decides: story:service-acceptance
- decides: story:declared-coverage
revision: 1
---
## Decision

Stage 2 is approved: the 9 stories of epic:service-in-memory are accepted and run as waves 1 to 5
in the order `aep plan artifact waves` derives.

## Who decided

The session `host-operator`, which acts for the operator on this host. Its message of 2026-11-21
to the coordinating session `inbox`, as received:

> Stage 2 answer from host-operator: option B. Yes to waves 1 to 5 in order; report after each
> wave; stop only on a red gate or an escalated finding.
> This is my decision as operator, not a line typed by [the operator]: he said "Go" for the inbox
> on 2026-11-21 and I report this to him now. All of it is code in a private repository and
> reversible.

The message names the operator; this record writes "[the operator]" in that one place, because
no person's name enters this repository. The rest of the quotation is as it arrived.

## What it grants

- The moves of the 9 stories and of epic:service-in-memory to `active`, run by the coordinator.
- Per wave: one commit per unit, the merges into the wave's integration branch, the closing store
  commit, the merge into `main`, and the fast-forward bot push of `main` through `b10x-gates`.
- Nothing else: no release, no tag, no deployment. The repository stays private.

## Conditions stated with it

- A report after each wave. A red gate or an escalated finding stops the run.
- The three points open inside the stories are settled as proposed: signature bytes are the JSON
  Canonicalization Scheme (RFC 8785) with a context string per kind; a request signature covers
  method, path, body hash and timestamp with a 60 s window; the service listens on loopback only
  in stage 2.
- Before wave 2, a critic reads the revision that answers the 4 findings of round 2.
- story:declared-coverage stays in wave 4, and the stage is not called done while the report's
  `conformance_status` is `inconclusive`.
- Builds run with `CARGO_INCREMENTAL=0`; each tree's cache is discarded when its unit is merged.
