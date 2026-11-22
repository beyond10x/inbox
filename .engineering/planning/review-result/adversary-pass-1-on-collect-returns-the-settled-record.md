---
format: aep.planning-md/3
id: review-result:adversary-pass-1-on-collect-returns-the-settled-record
kind: review-result
status: active
title: Adversary pass 1 on story:collect-returns-the-settled-record
relations:
- reviews: story:collect-returns-the-settled-record
revision: 1
---
unit: story:collect-returns-the-settled-record, the uncommitted working tree of `fix/collect-record` (base 42c7eb1)
verdict: NEEDS-CHANGE
cases: executed 371→378 (the packages `inbox-core`, `inbox-cli`, `inbox-conformance`, `inbox-server`), red 4
origin: introduced 4 / pre-existing 0 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with each
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added 2 case files,
`crates/inbox-core/tests/adversary_collect_record.rs` and
`crates/inbox-cli/tests/adversary_collect_record.rs`, and one `mod` line for each in the package's
`tests/main.rs`. Every mutation ran in a copy outside the tree.

**2. Cases added.**

- `a_sender_whose_caller_carries_no_inbox_gets_the_outcome_the_model_orders` (core): red, 0.16 s
  (finding F4).
- `the_record_is_the_outbox_row_of_the_moment_before_member_for_member` (core): green, 0.06 s. A
  decision answered with an option, spoken for another principal under a delegation; the body
  holds quotes, a backslash, line ends, U+2028, U+1F680 and U+10FFFF.
- `an_empty_refusal_reason_is_in_the_record` (core): green, 0.13 s.
- `collects_at_once_give_one_record_and_an_answer_racing_a_collect_is_never_lost` (core): green,
  2.7 s. 50 rounds of two collects at once: one record, the outcomes `collected` and
  `unknown-message`. 50 rounds of an answer racing a collect: a collected record carries the
  reply, `not-settled` leaves the row.
- `collect_without_the_record_of_its_message_neither_succeeds_nor_drops_the_line` (client): red,
  3.8 s (finding F1).
- `collect_json_keeps_the_members_0_2_0_printed_under_message` (client): red, 2.4 s (finding F3).
- `the_record_lets_the_sender_verify_the_reply_it_collected` (client): red, 1.9 s (finding F2).

**3. Suite run.** `cargo nextest run -p inbox-core -p inbox-cli -p inbox-conformance -p
inbox-server --locked --no-fail-fast`: `378 tests run: 374 passed, 4 failed, 1 skipped`, exit
status 100. The 4 red cases are the adversary's.

Mutants, applied in a copy, against the unit's 13 new cases and the conformance run against the
core (112 passed without a mutant).

- The answer is built and the delete skipped: 2 unit cases fail; conformance 3 failed.
- The row is deleted on `not-settled`: unit cases fail; conformance 2 failed.
- The recipient gets the record: unit cases fail; conformance 1 failed.
- `not-settled` is checked before `not-sender`: no unit case fails (survives); conformance 1
  failed.
- The record's `refusal` is always absent: 1 unit case fails; conformance 112 passed.
- The outbox line is removed before the collect is sent: 1 unit case fails.

**4. Findings.**

F1. `crates/inbox-cli/src/commands.rs`, `collect`. The client exits 0 and removes the outbox
line when the `collected` answer has no record, the record of another message, a record that is
not an object, or a record without `state`. A service of release 0.2.0 that still runs after the
client is upgraded answers so: the reply is deleted unseen. Introduced.

F2. `ess/domains/messaging.yaml`, the `response:` of `Collect`. The record has no `expires`. The
signature of a reply covers the reply's `expires`, so the sender cannot verify the reply it
collected. Release 0.2.0 printed it. Introduced.

F3. `crates/inbox-cli/src/commands.rs` and `CHANGELOG.md`. `collect --json` prints under
`message` the 8 members of the record where 0.2.0 printed the row of `Outbox`: `from`,
`on_behalf_of`, `kind`, `reply_shape`, `created`, `expires`, `dedup_key`, `body` and `signature`
are gone, and the changelog says "as before". Introduced.

F4. `crates/inbox-core/src/obligations.rs`, the hand-written `Collect`. A caller whose principal
is the sender and that has no inbox attribute gets an unmet obligation where the model and the
generated code of the base commit answer `collected` or `not-settled`. No caller of the shipped
service builds that context. Introduced.

F5. None of the unit's 13 cases fails when `not-settled` is checked before `not-sender`.
Introduced.

**5. Attacked and not broken.** The record member for member for a delegated reply with an
option, a body of special characters, an empty refusal reason and a decision answered by
silence; a collect against a collect and an answer against a collect through
`inbox_core::handle`; a third principal over HTTP (the bytes equal those of `unknown-message`,
also after a collect); a truncated body and a refusal keep the outbox line; the runner refuses a
missing response, a null, an undeclared member, a wrong type and a breach of an alphabet. The
conformance steps that expect a direct response check the shape of the record and no value.

**What the coordinator did.**

- F1: back to the implementor. A `collected` answer without the record of its message is a
  transport failure (exit 3), and the outbox line stays, so a repost can ask again.
- F2 and F3: back to the implementor. The record carries the members of the row of `Outbox`.
- F4: back to the implementor. The hand-written `Collect` decides its guards as the generated
  code of the base commit did.
- F5: back to the implementor: one case more.

```findings
- file: crates/inbox-cli/src/commands.rs
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: inbox collect exits 0 and drops the outbox line on a collected answer with no record, another message's record or a malformed one, so with a 0.2.0 service still running the reply is deleted unseen
- file: ess/domains/messaging.yaml
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the Collect response omits the reply's expires, which the signature of the reply covers, so the sender cannot verify the reply it collected
- file: CHANGELOG.md
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: collect --json prints under message only the 8 members of the record and drops 9 members 0.2.0 printed, while the changelog says "as before"
- file: crates/inbox-core/src/obligations.rs
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the hand-written Collect returns an unmet obligation for a sender whose caller has no inbox attribute where the model and the generated code of the base answer collected or not-settled; no shipped caller builds that context
- file: crates/inbox-core/tests/collect_record.rs
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: none of the unit's 13 new cases fails when not-settled is checked before not-sender; only the conformance run does
```
