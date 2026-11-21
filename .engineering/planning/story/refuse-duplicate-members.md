---
format: aep.planning-md/3
id: story:refuse-duplicate-members
kind: story
status: draft
title: The listener refuses a request whose JSON carries a member name twice
summary: A post whose text carries a member twice decodes to the first value and verifies; the service refuses such a text before decoding.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-signature-verifier-pass-1
revision: 1
---
## Outcome

The service refuses a request whose JSON text carries a member name twice in one object, at any
depth, before it decodes the body. On ESS 0.96.0 the generated reader keeps the first of two
members (`Value::member` in `generated/rust/inbox/src/server/json.rs`) and the generated decoders
do not refuse the second, so a post whose text carries `body` twice decodes to the first value
and its signature verifies: one signature covers two texts, and a reader that keeps the last
member reads another message than the one the service stored.

The listener of story:http-service calls the check `crates/inbox-verify` offers for a parsed
text and answers the refusal the wire API declares for a body that is not valid.

## Acceptance

A post whose text carries `body` twice is refused by the running service and stores nothing,
where before this story the service decodes it to the first `body` and stores it.

## Context

Found by the adversary of story:signature-verifier in stage 2, wave 2
(review-result:adversary-signature-verifier-pass-1, finding 4). The cause is in the generated
crate, which is identical at the wave base, so the finding was there before the unit.
story:signature-verifier added the check as a public function and uses it for the key file; no
decode path of an envelope exists before story:http-service.

RFC 8259 leaves the reading of a duplicate member to the implementation, so this is filed here
and not as a defect of ESS.

## Scope

- the listener crate of story:http-service
- `crates/inbox-verify/` (the check, if its shape has to change)
- `docs/design/signing.md`
