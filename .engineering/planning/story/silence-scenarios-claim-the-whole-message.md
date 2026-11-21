---
format: aep.planning-md/3
id: story:silence-scenarios-claim-the-whole-message
kind: story
status: draft
title: The conformance suite holds that deciding on silence changes nothing else of a decision
summary: '19 changed cores of Expire pass task conform: a member no scenario claims, a claimed member changed only from Delivered or Read, decided_option from another source, other rows of the thread.'
tags:
- from-finding
relations:
- informed_by: review-result:adversary-silence-leaves-the-other-fields-pass-1
revision: 1
---
## Outcome

The conformance suite holds that deciding on silence changes nothing of the stored message
except `expires`, `answered_by`, `decided_option` and the state, for a decision that was Sent,
Delivered or Read. A core whose `Expire` changes another member, changes one only from
Delivered or Read, takes `decided_option` from another source than `default_on_silence`, or
changes another message of the thread fails `task conform`.

## Acceptance

Each of the changed cores below, built in a copy of the tree, fails `task conform` on the
interpreted lane or the core lane, where before this story each passes it:

- `Expire` changes `on_behalf_of`, `to`, `kind`, `reply_shape`, `dedup_key`, `authority` or
  `refusal` when it decides on silence (one core per member);
- `Expire` changes `body` only when the decision was Delivered, and only when it was Read;
- `decided_option` is the recommended option; it is the letter of the last option;
- `Expire` clears `body` of the other messages of the thread.

A claim on `signature` is made where the lanes can read it; where a lane cannot, the story says
so and the cases of the core keep holding it.

## Context

Found by the adversary of story:silence-leaves-the-other-fields-scenario in stage 2, wave 8
(review-result:adversary-silence-leaves-the-other-fields-pass-1, findings 1, 2 and 3).

Since wave 8 the 3 cases of `crates/inbox-core/tests/adversary_w8_fields.rs` fail all of these
cores except the one that changes other messages of the thread, in `cargo test -p inbox-core`.
A second backend is held by the conformance suite and not by the cases of the core, so the
behaviour belongs in scenarios.

The scenario needs a decision with 3 options whose recommended option, silence option and last
option are 3 different letters, a sender that is not the recipient, every optional member set,
and a second message in the thread. The adversary gave a loop over the 3 start states as text;
it was not run.

Each new scenario adds a line to `crates/inbox-conformance/unsupported.tsv` when the serve lane
cannot run it, and moves the totals of `task coverage`.

## Scope

- `tools/scenarios.py`
- `ess/scenarios/` (the generated files)
- `crates/inbox-conformance/unsupported.tsv`
- `AGENTS.md`, `website/docs/status.md` (the counts)
