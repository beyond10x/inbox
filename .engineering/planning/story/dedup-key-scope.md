---
format: aep.planning-md/3
id: story:dedup-key-scope
kind: story
status: draft
title: A dedup key of one sender does not refuse another sender's post
summary: duplicate-key compares the key alone over every message, so a third principal learns of a key or takes it first.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-http-service-pass-1
- blocks: epic:two-hosts-and-wake
- blocks: epic:jetstream-backend
revision: 1
---
## Outcome

A dedup key one sender used does not refuse a post of another sender. Today `duplicate-key`
compares the key alone over every stored message (`ess/domains/messaging.yaml`, `PostMessage`,
`where: dedup_key == input.dedup_key`): a principal that posts into its own inbox under a key
learns from `duplicate-key` whether any message carries that key, and by posting first makes
another sender's post under the same key fail.

## Acceptance

After a third principal has posted into its own inbox under a dedup key, a post of another sender
to another inbox under the same key is answered `posted`, where before this story it is refused
`duplicate-key`.

## Context

Found by the adversary of story:http-service in stage 2, wave 3
(review-result:adversary-http-service-pass-1, finding F8; the case is
`adv_a_third_principal_cannot_take_another_senders_dedup_key` in
`crates/inbox-server/tests/adversary.rs`, ignored until this story). The comparison is the
specification's, so the finding was there before the unit.

Open, for whoever accepts this: the scope of a key (per sender, or per sender and inbox).
`docs/design/inbox.md` is where the decision is written down.

## Scope

- `ess/domains/messaging.yaml` (`PostMessage`), `ess/scenarios/`
- `generated/` (regenerated)
- `crates/inbox-core/` (the obligation of `PostMessage`)
- `docs/design/inbox.md`
- `crates/inbox-server/tests/adversary.rs` (the ignored case)
