---
format: aep.planning-md/3
id: story:signature-verifier
kind: story
status: implemented
title: 'The version 1 signature verifier: Ed25519 keys and delegations from an operator-signed key file'
summary: A standalone library that signs and verifies envelopes, delegations and requests against a key file whose entries the operator key signed.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:rust-workspace-and-model
scope:
- confidence: cited
  path: crates/inbox-verify
- confidence: cited
  path: docs/design/signing.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-21T09:36:08Z", actor: "agent:inbox", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":11,"verification":1}}}
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

Settled while the wave ran (stage 2, wave 2); `docs/design/signing.md` is the full text:

- As proposed: RFC 8785 canonical JSON, a context string per kind (`inbox.v1.envelope`,
  `inbox.v1.entry`, `inbox.v1.request`, each followed by a line feed), and a request signature
  over method, path with query, body hash and timestamp with a window of 60 s, both bounds
  included. `valid_until` is exclusive. Verification is strict (`verify_strict`), and a signature
  or key that is not canonical padded base64 is refused.
- A key file in which an object carries a member name twice, at any depth, is refused whole.
  The generated reader keeps both members and its decoders take the first, so the crate offers
  the check as a public function, `duplicate_member`, over a parsed text. The same hazard holds
  for a post or a reply; the listener's call is a backlog draft.
- An id carried by more than one entry the operator signed is ignored in every one of them,
  whatever else is wrong with either. An id is never reused: a renewal takes a new id. Whoever
  can write the key file can remove an id, by deleting its entry or by putting back an earlier
  signed entry for it; nobody without the operator key adds an entry or changes what an id
  means.
- The verifier reads no clock: every check takes the instant from its caller. It keeps no record
  of requests it has seen, so a signed request can be sent again inside its window.
- Dependencies: `ed25519-dalek` 2.2, `sha2` 0.10, `getrandom` 0.4, `zeroize` 1.8.

## Scope

- `crates/inbox-verify/` (new): `src/`, `tests/{acceptance,cases,jcs,adversary,adversary2}.rs`,
  `tests/common/`
- `docs/design/signing.md` (new): canonical bytes, context strings, what a verifier refuses
- `Cargo.lock` (the crate and its dependencies)
