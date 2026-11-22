---
format: aep.planning-md/3
id: review-result:adversary-pass-1-on-the-core-answers-a-foreign-message-as-unknown
kind: review-result
status: active
title: Adversary pass 1 on story:the-core-answers-a-foreign-message-as-unknown
relations:
- reviews: story:the-core-answers-a-foreign-message-as-unknown
revision: 1
---
unit: story:the-core-answers-a-foreign-message-as-unknown, the branch `fix/core-read-rule` after round 1 of the implementor
verdict: INFEASIBLE
cases: executed 293→296 (the packages `inbox-core`, `inbox-server`, `inbox-conformance`), red 1
origin: introduced 0 / pre-existing 1 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with the
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added
`crates/inbox-core/tests/adversary_foreign_unknown.rs` and one `mod` line in
`crates/inbox-core/tests/main.rs`. Every mutation ran in a copy outside the tree.

**2. Cases added (3).**

- `adv_a_client_with_one_attribute_absent_cannot_tell_a_message_between_others_from_none`: red,
  0.5 s (finding F1). 3 forms of caller (a principal without an inbox, an inbox without a
  principal, no attribute), the 5 commands, 9 messages, through `handle` and
  `handle_with_events`, each against an id no message carries.
- `adv_attributes_that_name_two_parties_crosswise_are_answered_as_no_message`: green, 0.25 s. A
  caller with the recipient's principal and the sender's inbox, and a caller whose inbox is not
  its principal's. Red at the commit before the unit.
- `adv_after_removal_the_former_parties_and_a_third_get_the_answer_of_no_message`: green, 0.23 s.

**3. Suite run.** `cargo nextest run --no-fail-fast -p inbox-core -p inbox-server -p
inbox-conformance`: 296 run, 295 passed, 1 failed (the red case), 1 skipped. Without the 3
cases: 293 run, 293 passed.

Mutants, each against the 3 packages: `handle_with_events` without the rename; `handle` calling
`handle_as_specified`; the rename left out for `Collect` only; `as_unknown` adding a member to a
renamed answer; `as_unknown` renaming every outcome; the listener's status 404 changed to 403;
the listener without `as_unknown`; the listener writing the body before the rename with the
status 404: each killed. The conformance target calling `handle_with_events` survives the tests
of the packages and is refused by the gate's conformance run against the core (the scenario
`a-third-principal-finds-no-message` fails at its first step that expects `foreign-message`).

**4. Finding.**

F1. `crates/inbox-core/src/lib.rs`, `handle_with_events`. A `Client` whose context answers the
inbox or the principal attribute as none gets `Err(Unmet)` for a stored message between others:
the generated guard reads the attribute, finds it unknown and refuses. For an id no message
carries the answer is `unknown-message`, decided before any attribute is read. 234 of 270
requests tell the two apart, in every state, through both entries. The HTTP service builds both
attributes for every caller, so no request over HTTP reaches it; an embedding of `inbox-core`
that builds such a caller does. The acceptance of the story and 2 design documents claim the
same answer for a third principal without that condition. The case is red at the commit before
the unit too.

**5. Attacked and not broken.** The bytes of an HTTP answer and its status 404; `as_unknown` on
other outcomes, outcomes in another case, outcomes that are no text, `error` members, nested
rows, arrays and view answers; no event for a foreign and for a missing id in all 9 states;
the principal a delegate speaks for; crosswise attributes; the answers after removal; no public
entry of the core but `handle_as_specified` answers `foreign-message`, and only the conformance
target calls it; `ess/` and `generated/` are unchanged by the unit; the one deterministic
difference in work is a second read of the message on the foreign path.

**What the coordinator did.**

- F1: fixed in round 2 of the implementor: a client whose context answers an attribute
  as none gets one answer for every id from both entries, and the documents state the rule.

```findings
- file: crates/inbox-core/src/lib.rs
  line: 126
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: a Client whose context answers the inbox or principal attribute as none gets Err(Unmet) from handle and handle_with_events for a message between others and unknown-message for a missing id (234 of 270 requests differ), so an embedding answers a third principal alike only when both attributes are present; no caller in the repository builds such a Client
```
