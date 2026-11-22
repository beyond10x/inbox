---
format: aep.planning-md/3
id: story:design-decides-refusal-signature-and-reply-id
kind: story
status: draft
title: The design decides whether a refusal is signed and whether a reply id is a message id
revision: 1
---
## Outcome

The design says whether an owner's refusal is signed, and whether a `reply_id` is a message
id; the specification declares what the design says.

The design review of 2026-11-22 (hardening, technique 8) found 2 questions the design leaves
open and the specification answers by omission:

- Finding 7. `docs/design/inbox.md`: "The service verifies the signature and the authority of
  every post and every reply before it stores anything." `Refuse` takes `message_id` and
  `reason` only: no signature, no `from`, no `on_behalf_of`. The sender collects the refusal
  as the answer to its request, and nothing in the stored message proves who refused. The
  request to the service is signed by the caller (`docs/design/service.md`); that signature
  is not stored.
- Finding 9. "A message id is one name space for every sender". A `reply_id` has the type
  `MessageId` and no uniqueness guard: it may equal the id of a stored message or of another
  reply.

## Acceptance

`docs/design/inbox.md` holds one sentence for each question, and for each the specification
either declares the rule (a signed `Refuse`; a guard on `reply_id`) with a scenario that is
refused without it, or the design says why the rule is not needed (state transition: 2
findings of the review change from open to closed).

## Decisions

Both are the designer's. The options, with the one this draft recommends first:

- Refusal. A: a refusal stays unsigned and the design says so: it is a state of the message,
  set by the owner the service authenticated, and the sender trusts the service for it as it
  does for `Read`. B: `Refuse` carries a signed envelope like `Answer`, at the cost of a
  second reply kind in `signing.md` and a breaking change of the command.
- Reply id. A: a `reply_id` is outside the message-id name space and the design says so; the
  pair (`in_reply_to`, `reply_id`) names a reply. B: `Answer` refuses a `reply_id` that is the
  id of a stored message, which needs a `when_related` guard and tells the owner that an id
  exists in another inbox.

## Scope

- `docs/design/inbox.md`, `docs/design/signing.md`
- `ess/domains/messaging.yaml` only under option B of either question
