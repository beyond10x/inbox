---
format: aep.planning-md/3
id: story:post-does-not-tell-a-message-id-exists
kind: story
status: active
title: A post does not tell a third principal whether a message id exists
summary: post answers duplicate for a stored message id, whoever sent it, so a third principal learns from a post whether an id is in use.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-foreign-message-id-pass-1
- blocks: epic:two-hosts-and-wake
- blocks: epic:jetstream-backend
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T17:17:28Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T17:17:28Z", actor: "agent:inbox", revision: 3}
---
## Outcome

A post does not tell a principal that is neither the recipient nor the sender of a message
whether that message's id is in use. Today `PostMessage` answers `duplicate` for a `message_id`
that a stored message carries, whoever sent it (`ess/domains/messaging.yaml`, `PostMessage`,
`existing_instance`), and it answers that before `unknown-inbox`. A principal that posts as
itself to an address no inbox has, under the id of a message between two others, is answered
`duplicate`, and `unknown-inbox` under an id nobody used. Neither post stores anything.

## Acceptance

A third principal that posts, as itself, to an address no inbox has, once under the
`message_id` of a message between two others and once under an id no message carries, gets the
same status and the same body for both, where before this story the first is `duplicate` and
the second `unknown-inbox`.

## Context

Found by the adversary of story:foreign-message-id-answers-as-unknown in stage 2, wave 6
(review-result:adversary-foreign-message-id-pass-1, finding 1; the case is
`adv_w6_a_third_principal_cannot_tell_a_message_between_others_from_no_message_by_post` in
`crates/inbox-server/tests/adversary_w6_foreign.rs`, ignored until this story). The order of the
outcomes and the id shared by every sender are the specification's, so the finding was there
before the unit: the case fails the same way on the unit's base.

It is the fact the limit "A refusal tells a third principal whether a message exists" of
`website/docs/limits.md` names, reached through another command. Since
story:foreign-message-id-answers-as-unknown the 5 commands that name a stored message answer a
third principal as for an id nobody used; `post` is the command that still tells.

A post into the caller's own inbox shows the same (`duplicate` against `posted`) and stores a
message when the id is free.

## Decisions

Accepted on 2026-11-21 by the coordinating session under the standing rule of
approval-record:wave-6-security-drafts-accepted: a draft that an adversary pass files against an
open limit of `website/docs/limits.md` is accepted by the coordinator without asking and named
in the wave report. It is not part of wave 6.

Decided by the host operator's session on 2026-11-21, on 3 options the coordinating session put
to it (A: wait for the ESS fix, outside the gate; B: wait and join the gate; C: run now as its
own wave), as received:

> host-operator: B. story:post-does-not-tell-a-message-id-exists waits for the ESS fix of issue
> 805, runs in one wave with story:dedup-key-scope, and joins the gate (blocks edges to
> epic:two-hosts-and-wake and epic:jetstream-backend).
> - Reason: it is a finding of the same kind as the 3 in the gate; a gate that leaves one of
>   them out says less than the limits page.

It also ordered: when the ESS release is out, the wave starts without asking and is reported at
its close; no release, no listener outside loopback and no deployment before the gate stories
are implemented.

## Open for whoever scopes this

- The adversary's reading: reordering the outcomes does not close it. While a message id is
  shared by every sender, any answer that differs for an id in use tells. The ways it named:
  an id that is unique per sender, or a document that says an id can be probed.
- A message id is a ULID: 48 bits of time and 80 random bits. Whether "knows the id" needs a
  party that passed it on, and what that is worth, belongs in the scope.
- story:dedup-key-scope is the same question for the dedup key and is held on beyond10x/ess#805.
  A rule that compares the id among the sender's own messages reads `input.from` in a row
  guard, which is the construct that defect is about.
