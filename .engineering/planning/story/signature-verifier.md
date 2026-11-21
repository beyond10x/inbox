---
format: aep.planning-md/3
id: story:signature-verifier
kind: story
status: active
title: 'The version 1 signature verifier: Ed25519 keys and delegations from an operator-signed key file'
summary: A standalone library that signs and verifies envelopes, delegations and requests against a key file whose entries the operator key signed.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:rust-workspace-and-model
scope:
- confidence: inferred
  path: crates/inbox-verify
- confidence: inferred
  path: docs/design/signing.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`crates/inbox-verify` is a library that depends on the generated crate and on no other crate of
this workspace. It reads a key file (the types `inbox.messaging.KeyFile`, `KeyEntry` and
`DelegationEntry` of `ess/domains/messaging.yaml`, through the codecs of the generated crate's `server` feature), keeps only the
entries the pinned operator public key signed, and offers:

- sign an envelope, and verify one at a given instant: the key id is known and not lapsed, and
  the Ed25519 signature verifies over the canonical bytes of the envelope;
- check a delegation at a given instant: the id exists, names the pair `from` and
  `on_behalf_of`, and is not lapsed. The chain length is one;
- resolve a key id to its principal;
- sign and verify a request: method, path with query, SHA-256 of the body and a timestamp.
  story:http-service authenticates a caller with it;
- create a key pair, and sign a key entry or a delegation entry with an operator key, for the
  CLI.

## Acceptance

`cargo test -p inbox-verify` rejects an envelope whose `body` was changed after signing and
accepts the unchanged envelope against a key file the test's operator key signed, where before
this story no code in the workspace verifies a signature.

## Context

The canonical bytes a signature covers are written into `docs/design/signing.md` before code.
Proposal: the JSON Canonicalization Scheme (RFC 8785) of the envelope without `signature`, of an
entry without `operator_signature`, and of the request fields, each prefixed with a fixed context
string so that a signature of one kind never verifies as another.

Left out of version 1: revocation lists (a short `valid_until` and a file edit replace them),
delegation chains, a delegation scope narrower than the pair, hardware-backed keys, payload
encryption. General agent identity belongs to Mandate.

## Scope

- `crates/inbox-verify/` (new)
- `docs/design/signing.md` (new): canonical bytes, context strings
