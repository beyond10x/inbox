---
format: aep.planning-md/3
id: approval-record:wave-6-security-drafts-accepted
kind: approval-record
status: draft
title: 'Wave 6 accepted: the 3 security drafts of story:http-service''s first adversary pass'
relations:
- decides: story:post-from-is-the-caller
- decides: story:foreign-message-id-answers-as-unknown
- decides: story:dedup-key-scope
- decides: story:post-does-not-tell-a-message-id-exists
revision: 1
---
## Decision

The 3 drafts filed from the first adversary pass on story:http-service are accepted and run as
one wave, wave 6, after the 5 waves of the stage 2 plan: story:post-from-is-the-caller,
story:foreign-message-id-answers-as-unknown and story:dedup-key-scope.

## Who decided

The session `host-operator`, which acts for the operator on this host. Its message of 2026-11-21
to the coordinating session `inbox`, in answer to the closing report of wave 5, as received:

> host-operator: A. The 3 drafts story:post-from-is-the-caller,
> story:foreign-message-id-answers-as-unknown and story:dedup-key-scope are accepted (record
> this decision and its date in each); run them as one wave now, same gate, report after the
> wave. No release with it. epic:service-in-memory stays "active" until that wave is delivered;
> ask again then. Declining the delete the implementor asked for after its `rm` was refused was
> right; the file stays. Standing: a draft that an adversary or security pass filed against an
> open limit of the limits page is accepted by you without asking; name it in the wave report.

Option A of the closing report read: accept the 3 and run them as one wave now: scope them,
check they can run at once, implementor and adversary per story, same gate, report after the
wave, no release with it.

## What it grants

- The moves of the 3 stories from `draft` to `active`, run by the coordinator.
- For wave 6: the opening commit, one commit per unit and per correction round, the merges into
  the integration branch, the closing store commit, the fast-forward of `main` and the bot push
  of `main` through `b10x-gates`.
- Standing, from this date: a draft that an adversary pass or a security pass files against an
  open limit of `website/docs/limits.md` is accepted by the coordinator without asking, and is
  named in the wave report.
- Nothing else: no release, no tag, no deployment.

## Conditions stated with it

- The gate of the earlier waves (`task check` on the integration branch), and a report after the
  wave.
- epic:service-in-memory stays `active` until wave 6 is delivered; its move is asked again then.
- `tests/acceptance/tests/adversary.rs` stays as it is.

## Taken by the coordinator, not by this decision

The two points the drafts left open "for whoever accepts this" were not part of the answer. The
coordinator settled them on 2026-11-21, and each story's text says so:

- story:dedup-key-scope: the scope of a dedup key is the sender (`from`).
- story:foreign-message-id-answers-as-unknown: the model names the case, a caller that is
  neither the recipient nor the sender, as an outcome of its own, and the listener gives that
  one outcome the answer of `unknown-message`. The model alone cannot: probed on a copy of the
  model, ESS 0.96.0 refuses a second outcome named `unknown-message` (`ESS-COMMAND-006`) and
  gives a caller-guarded outcome with the error `UnknownMessage` the status 403 and its own
  name in the body.

The 3 stories change the same model file and the same generated crate, so their units run one
after another on one integration branch, in the order story:post-from-is-the-caller,
story:dedup-key-scope, story:foreign-message-id-answers-as-unknown.
