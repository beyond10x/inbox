---
format: aep.planning-md/3
id: story:pinned-scenario-total-can-fail
kind: story
status: draft
title: The scenario total pinned in adversary3.rs is checked or removed
summary: A case of the conformance crate carries the suite's total and passes with any number; every unit that adds a scenario edits it for nothing.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-read-decision-expires-on-silence-pass-1
revision: 1
---
## Outcome

The scenario total written in the case
`the_gate_refuses_a_listed_reason_that_is_not_the_reason_of_the_run` of
`crates/inbox-conformance/tests/adversary3.rs` is either checked by the case or not written.
Today the case passes with any total: every unit that adds a scenario edits the number, and the
edit changes nothing the gate checks.

## Acceptance

With the total in that case changed to a number that is not the suite's, `cargo test -p
inbox-conformance --test adversary3` fails and names the 2 numbers; or the case holds no total
and the unit shows that no other case of the file reads one that cannot fail.

## Context

Found by the adversary of story:read-decision-expires-on-silence-scenario in stage 2, wave 8
(review-result:adversary-read-decision-expires-on-silence-pass-1, finding 4): the case passed
with 97 and with 50 while the suite held 98 scenarios. The finding was there before the unit.

Units of waves 7 and 8 each changed that number by hand when they added scenarios.

## Scope

- `crates/inbox-conformance/tests/adversary3.rs`
