---
format: aep.planning-md/3
id: story:post-from-is-the-caller
kind: story
status: draft
title: A post or an answer is refused when its from is not the caller
summary: A recipient sends a sender's signed envelope again under its own request signature and it is posted once the first message is gone; the model guards PostMessage and Answer on the caller.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-http-service-pass-1
revision: 2
---
## Outcome

A post or an answer is refused when the envelope's `from` is not the principal of the caller that
sent the request. Today `PostMessage` reads no attribute of the caller: a principal that holds a
signed envelope of another principal (every recipient holds the ones it received, `Pending` hands
them out) sends the same bytes again under its own request signature, and once the first message
is handled and removed the service answers `posted` a second time. The message reappears in the
sender's outbox, and a decision with `default_on_silence` gives the sender a second decided
answer. `already-expired` compares the envelope's own `expires` and `created`, so the clock does
not end this.

The rule belongs in `ess/domains/messaging.yaml` as an input-guarded refusal on the caller
(`from != caller.principal`), as `OpenInbox` already guards its input on the caller
(`address != caller.inbox`), with a
scenario that holds it; the core and the listener then take it from the generated crate.

## Acceptance

A request signed by one principal that posts an envelope whose `from` is another principal is
refused by the running service and stores nothing, where before this story it is answered
`posted` once the first message under that id is gone.

## Context

Found by the adversary of story:http-service in stage 2, wave 3
(review-result:adversary-http-service-pass-1, finding F6; the case is
`adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` in
`crates/inbox-server/tests/adversary.rs`, ignored until this story). The specification has no
such guard at the wave base and the listener passes the command on, so the finding was there
before the unit.

With a delegation the caller is still `from` (the agent that signs); `on_behalf_of` names the
principal it speaks for, so the rule does not touch delegation.

## Scope

- `ess/domains/messaging.yaml` (`PostMessage`, `Answer`), `ess/scenarios/`
- `generated/` (regenerated)
- `crates/inbox-core/` (the obligation of `PostMessage`)
- `crates/inbox-server/tests/adversary.rs` (the ignored case)
