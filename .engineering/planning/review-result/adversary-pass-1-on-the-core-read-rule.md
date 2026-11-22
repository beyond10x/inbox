---
format: aep.planning-md/3
id: review-result:adversary-pass-1-on-the-core-read-rule
kind: review-result
status: active
title: adversary, pass 1, the core's read rule
relations:
- reviews: story:the-core-applies-the-read-rule
revision: 1
---
unit: story:the-core-applies-the-read-rule, the uncommitted working tree of `fix/core-read-rule` (base 42c7eb1)
verdict: CONFIRMED
cases: executed 280→289 (the packages `inbox-core`, `inbox-server`, `inbox-conformance`), red 1
origin: introduced 1 / pre-existing 2 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with each
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added 2 case files,
`crates/inbox-core/tests/adversary_read_rule.rs` and
`crates/inbox-server/tests/adversary_read_rule.rs`, and one `mod` line for each in the package's
`tests/main.rs`. Every mutation ran in a copy outside the tree.

**2. Cases added.** The fixture of the core cases is a world with one message in each state
(Sent, Delivered, Read, Answered, Refused, answered by silence), a delegated message and a
fan-out of one thread to two inboxes.

- `a_third_principal_reads_no_row_in_any_state_through_handle_with_events`: green, 0.06 s. All 6
  views with every parameter (5 inboxes, 5 senders, 9 threads): the third principal gets 0 rows
  and 0 events, the parties get rows.
- `the_principal_spoken_for_reads_no_row_by_the_rule_as_written`: green, 0.01 s.
- `each_recipient_of_a_fan_out_reads_only_its_own_row_of_the_thread`: green, 0.02 s.
- `a_third_principals_command_hands_back_no_event_and_no_content_of_the_message`: green, 0.03 s.
- `a_third_principal_cannot_tell_a_message_between_others_from_none_through_handle`: red, 0.01 s
  (finding F1).
- `readable_drops_a_row_that_names_another_party_in_one_member_only`: green, 0.02 s.
- `readable_compares_text_exactly_and_never_a_value_that_is_not_text`: green, 0.04 s.
- `readable_binds_every_caller_but_the_sweeper_and_keeps_the_answer_in_order`: green, 0.01 s.
- `a_partys_view_answer_is_the_generated_answer_byte_for_byte` (the listener): green, 1.3 to
  3.3 s.

**3. Suite run.** `cargo nextest run --locked --no-fail-fast -p inbox-core -p inbox-server -p
inbox-conformance`: `289 tests run: 288 passed, 1 failed, 1 skipped`, exit status 100. On an
export of the base commit the row cases, the fan-out case and the spoken-for case fail: the fix
of the unit is real. The red case fails on the base commit with the same values.

Mutants of `readable` and `party`, applied in a copy. "unit" is the unit's 8 cases.

- The `Sweeper` exception applies to every actor: unit 4 failed.
- No `Sweeper` exception: unit 2 failed.
- `||` becomes `&&` in `party`: unit 4 failed.
- Rows whose `to` is missing are kept: unit 8 passed (survives); the adversary's
  `readable_drops_a_row_that_names_another_party_in_one_member_only` fails.
- `from` is ignored: unit 3 failed.
- No filter on `Replies` in `handle_with_events`: unit 3 failed.
- No filter on `Settled`: unit 3 failed.
- Inbox and principal swapped in `party`: unit 1 failed.
- The listener skips `readable` on GET: unit 2 failed.

**4. Findings.**

F1. `inbox_core::handle`. A third principal that calls `MarkDelivered`, `MarkRead` or `Collect`
through `handle` on a message between others gets the outcome `foreign-message`; for an id no
message carries it gets `unknown-message`. The summary of `foreign-message` in
`ess/domains/messaging.yaml` says a third principal cannot tell the two apart; only the
listener's `as_unknown` makes that true. An embedding that hands the core's answer on tells a
third principal that a message id exists. It existed before this unit.

F2. `party` in `crates/inbox-core/src/lib.rs`. The condition "a row with neither `to` nor `from`
stays" is not held by the unit's 8 cases. No view reaches it: every row of a message view
carries both members. Introduced by the unit.

F3. `readable` in `crates/inbox-server/src/service.rs`. When `json::parse_written` refuses a
view body, the listener hands the body on unfiltered, where the core refuses. No body the
generated writer writes was found that the parser refuses. It existed before this unit.

**5. Attacked and not broken.** All 6 views in all states with a delegation and a fan-out; no
view declares paging, and an answer has no count and no cursor; a wait on `Pending` held open by
a third principal ends at its bound; a caller that is none is refused the views; an embedding
that passes the actor `Sweeper`, or a context with another principal's attributes, reads as
that: the embedding chooses the actor and authenticates the attributes. `OpenInbox` requires
`address == caller.inbox`. No other path of `crates/` reads message rows for a caller. The bytes
of an HTTP answer for a party are unchanged. ESS 0.97.1 refuses a view filter over the caller
(`ESS-VIEW-003`).

**What the coordinator did.**

- F1: taken out of this unit. The red case left the unit's tree; it is the first case of
  story:the-core-answers-a-foreign-message-as-unknown, which is built on the same branch.
- F2: closed by the adversary's case, which stays in the unit.
- F3: no change. Recorded here.

```findings
- file: crates/inbox-core/src/lib.rs
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: inbox_core::handle answers a third principal foreign-message for a message between others and unknown-message for none, so an embedding that hands the answer on tells that a message id exists
- file: crates/inbox-core/src/lib.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's 8 cases survive the mutant that keeps every row without `to`; an adversary case now kills it
- file: crates/inbox-server/src/service.rs
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: the listener hands a view body on unfiltered when its parser refuses it, where the core refuses; no reachable body was found
```
