---
format: aep.planning-md/3
id: story:expiry-scenarios-hold-the-remaining-cases
kind: story
status: draft
title: The conformance suite holds 5 behaviours of Expire that only cases of the core hold
summary: '5 changed cores of Expire pass task conform: a read decision without a silence option, 1 second early, a delivered request, thread and to of the event, a refused decision.'
tags:
- from-finding
relations:
- informed_by: review-result:adversary-expire-publishes-once-pass-1
- informed_by: review-result:adversary-read-decision-expires-on-silence-pass-1
revision: 1
---
## Outcome

The conformance suite holds 5 behaviours of `Expire` that today only cases of the core, or
nothing, hold. A core that gets one of them wrong fails `task conform`:

1. a read decision that names no `default_on_silence` is removed at `expires` (outcome
   `expired`), not decided on silence;
2. a decision is decided on silence at `expires` and not 1 second before it;
3. a request that is delivered or read is removed at `expires`, not decided on silence;
4. `thread` and `to` of `DecisionTakenOnSilence` are the thread and the principal the message
   was sent for, in a scenario where the thread is not the message id and `on_behalf_of` is
   not `from`;
5. a refused decision that names a `default_on_silence` is removed at `expires` with outcome
   `expired`.

## Acceptance

Each of the 5 changed cores below, built in a copy of the tree, fails `task conform` on the
interpreted lane or the core lane, where before this story each passes it:

- a read decision with no silence option decided on silence;
- a read decision decided 1 second before `expires`;
- a delivered request decided on silence;
- `thread` of the event written as the message id;
- `Refused` counted as an open state by `Expire`.

## Context

Found by 2 adversary passes of stage 2, wave 8, each on a finding that was there before its
unit: review-result:adversary-expire-publishes-once-pass-1 (findings 2 and 3) and
review-result:adversary-read-decision-expires-on-silence-pass-1 (findings 1, 2 and 3).

Since wave 8 the cases of `crates/inbox-core/tests/adversary_w8_expire.rs` and
`crates/inbox-core/tests/adversary_w8_read.rs` fail these cores in `cargo test -p inbox-core`.
A second backend is held by the conformance suite and not by the cases of the core, so the
behaviours belong in scenarios.

Every instant of the authored scenarios is a whole minute; behaviour 2 needs a step at
`expires` minus 1 second. The adversary of the read unit gave 2 scenarios as text
(`read-decision-without-silence-expires`, `request-expires-unanswered`); they were not run.

Each new scenario adds a line to `crates/inbox-conformance/unsupported.tsv` when the serve
lane cannot run it, and moves the totals of `task coverage`.

## Scope

- `tools/scenarios.py`
- `ess/scenarios/` (the generated files)
- `crates/inbox-conformance/unsupported.tsv`
- `AGENTS.md`, `website/docs/status.md` (the counts)
