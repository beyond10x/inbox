---
format: aep.planning-md/3
id: story:foreign-message-id-answers-as-unknown
kind: story
status: draft
title: A third principal gets one answer for a message between others and for no message
summary: not-recipient (403) against unknown-message (404) tells any authenticated client whether a message id exists.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-http-service-pass-1
- blocks: epic:two-hosts-and-wake
- blocks: epic:jetstream-backend
revision: 1
---
## Outcome

A principal that is neither the recipient nor the sender of a message gets the same answer for
the id of a message between two others as for an id no message carries. Today `read`, `collect`,
`refuse`, `answer` and `mark-delivered` answer `unknown-message` (404) for an id nobody used and
`not-recipient` (403) for a message addressed to another inbox, so any authenticated client that
knows or guesses a message id learns whether that message exists.

## Acceptance

A third principal that sends `read` for the id of a message between two others gets the status
and the outcome it gets for an id no message carries, where before this story the first is 403
`not-recipient` and the second 404 `unknown-message`.

## Context

Found by the adversary of story:http-service in stage 2, wave 3
(review-result:adversary-http-service-pass-1, finding F7; the case is
`adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message` in
`crates/inbox-server/tests/adversary.rs`, ignored until this story). The two outcomes and their
order are the specification's (`ess/domains/messaging.yaml`, the commands with `unknown_instance`
then `to != caller.inbox`), so the finding was there before the unit.

Open, for whoever accepts this: whether the model answers `unknown-message` for both, or the
service maps the two refusals to one answer. A message id is a ULID, which carries its creation
time and is not a secret.

## Scope

- `ess/domains/messaging.yaml`, `ess/scenarios/`
- `generated/` (regenerated)
- `crates/inbox-server/` and `docs/design/service.md` (if the service maps the answer)
- `crates/inbox-server/tests/adversary.rs` (the ignored case)
