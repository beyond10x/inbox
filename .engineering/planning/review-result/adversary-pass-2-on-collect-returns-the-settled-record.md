---
format: aep.planning-md/3
id: review-result:adversary-pass-2-on-collect-returns-the-settled-record
kind: review-result
status: active
title: Adversary pass 2 on story:collect-returns-the-settled-record
relations:
- reviews: story:collect-returns-the-settled-record
revision: 1
---
unit: story:collect-returns-the-settled-record, the branch `fix/collect-record` after round 2 of the implementor (base 7c4bfe1)
verdict: CONFIRMED
cases: executed 381→386 (the packages `inbox-core`, `inbox-cli`, `inbox-conformance`, `inbox-server`), red 3
origin: introduced 2 / pre-existing 3 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with each
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added 3 case files named
`adversary_collect_record_2.rs`, under `crates/inbox-core/tests/`, `crates/inbox-cli/tests/` and
`crates/inbox-conformance/tests/`, and one `mod` line for each in the package's `tests/main.rs`.
Every mutation ran in a copy outside the tree.

**2. Cases added (5).**

- `every_mix_of_caller_attributes_in_every_state_gets_the_outcome_of_the_base` (core): green,
  0.2 s. 3 × 3 mixes of the caller's inbox and principal (absent, matching, other) in each of
  `Sent`, `Delivered`, `Read`, `Answered` and `Refused`, held to a table of the outcomes of the
  base's generated `Collect`. Only `collected` carries a record and removes the row.
- `the_record_is_the_outbox_row_for_every_settlement` (core): green, 0.1 s. 5 settlements: a
  request refused with a reason of 2 lines; a decision with a default taken on silence; a
  decision with a default refused; a decision without a default answered with an option; a
  delegated `free` request answered with a body that holds a carriage return and a line feed.
- `an_answer_outside_the_declared_record_neither_succeeds_nor_drops_the_line` (binary): red,
  5.8 s alone (finding F2).
- `control_sequences_of_the_recipient_do_not_reach_the_senders_terminal` (binary): red, 4.1 s
  alone (finding F1).
- `a_response_that_states_a_member_twice_fails_the_step` (runner): red, 0.09 s (finding F3).

**3. Suite run.** `cargo nextest run -p inbox-core -p inbox-cli -p inbox-conformance -p
inbox-server --locked --no-fail-fast`: `386 tests run: 383 passed (1 slow), 3 failed, 1 skipped`,
the 3 failures being the adversary's red cases. With them deselected: `381 tests run: 381
passed`.

Mutants, each against the tests of its package:

- The guard `not-sender` of the hand-written `Collect` answering false for an unknown caller
  attribute, and the same for the guard of the foreign message: both survive the unit's cases,
  the cases of pass 1 and the conformance run against the core; only the matrix case above kills
  them (finding F4).
- `invariant::all` to `any`; `default_on_silence` left out of the record; `not-settled` decided
  before `not-sender`; the client accepting any text as `message_id`; the client without its
  check of `state`; the union check without its refusal of an undeclared member: each killed.
- The line "decided on silence" of the printed record removed: survives (finding F5).
- The runner's `ExpectResponse` no longer requiring that the answer has no error: survives.
- The count of occurrences skipping the payload of a union: survives; equivalent for this
  specification, in which no union payload holds a constrained newtype.
- A target that drops `response`: survives the tests of the package for the HTTP target (the
  conformance run of the gate over HTTP judges it) and is caught by the conformance run against
  the core (3 scenarios fail).

**4. Findings.**

F1. `crates/inbox-cli/src/output.rs`. Control characters in the body of a reply or in the reason
of a refusal are written as they are to the sender's terminal by `inbox collect`: ESC, an OSC
sequence ended by BEL, a carriage return, a C1 control sequence introducer. Any recipient
reaches it through `inbox answer` or `inbox refuse`. The same printer serves `inbox outbox`,
`inbox pending` and `inbox thread`. It existed before this unit: the case is red against the
base commit.

F2. `crates/inbox-cli/src/commands.rs`. The client accepts any 2xx answer whose `response` holds
the collected `message_id` and some text as `state`. It exits 0 and drops the outbox line for
the outcome `not-settled` without an error, for no outcome, for the state `Sent` or a text that
is no state, for a `reply` that is a text or whose body is no text, and for `message_id` or
`reply` stated twice. No service of this repository answers so; a service that does not conform
on `INBOX_URL` does. The base commit dropped the line on every 2xx.

F3. `crates/inbox-conformance/src/invariants.rs`, `src/runner.rs`. The step that expects a
direct response passes a response that states the tag of the union, the value of the union,
`state` or `message_id` twice; the check reads the first. No target writes a member twice.
Introduced with the step.

F4. `crates/inbox-core/src/obligations.rs`. The 2 mutants of the hand-written guards named
above survive everything but the adversary's matrix case. Over HTTP every caller carries both
attributes. Introduced.

F5. `crates/inbox-cli/src/output.rs`. The line "decided on silence" of a printed record has no
case at any level. It existed before this unit.

**5. Attacked and not broken.** The guards of the core in all 45 mixes of attributes and states
against the base; the record against the row of `Outbox` member for member in 5 more
settlements, with no member more or fewer and no `null` for an absent optional member; a third
principal and the recipient get no record; `MessageHandled` carries only `message_id`; the
service logs no request and no answer; the client's transport error names only the id; the
read rule of the listener leaves the answer `collected` as it is; the generated OpenAPI schema
of the result against the bytes on the wire; the union checks for an unknown variant, a missing
`value`, a payload of the wrong type and an extra member.

**What the coordinator did.**

- F1: its own story on this branch, published with this unit:
  story:the-client-prints-no-control-sequence-of-another-principal
  (approval-record:d22-the-client-prints-no-control-sequence-of-another-principal). Its red case
  left this unit's files and is the first case of that story.
- F2: back to the implementor as part of this unit. The client drops the outbox line only for
  the outcome `collected` with a record that decodes as the declared response, names the
  collected message, is `Answered` or `Refused` and states no member twice; otherwise it exits 3
  and keeps the line.
- F3: back to the implementor. The runner fails a direct response that states a member twice.
- F4: the matrix case stays in the unit.
- F5: back to the implementor: one case of the binary pins the line.

```findings
- file: crates/inbox-cli/src/output.rs
  line: 147
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a recipient's reply body or refusal reason reaches the sender's terminal with raw control sequences (ESC, OSC/BEL, CR, C1 CSI) through inbox collect, and the same printer serves outbox, pending and thread
- file: crates/inbox-cli/src/commands.rs
  line: 203
  category: acceptance
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: inbox collect exits 0 and drops the outbox line on a 2xx whose outcome is not collected or whose record breaks the declared response (unsettled or unknown state, malformed reply, a member stated twice); no shipped service answers so
- file: crates/inbox-conformance/src/invariants.rs
  line: 357
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: expect_direct_response passes a response that states the union tag, the union value or a top-level member twice, reading only the first; no target writes duplicate members
- file: crates/inbox-core/src/obligations.rs
  line: 373
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: turning either Unknown guard of the hand-written Collect into false survives the unit's cases, pass 1's cases and the conformance core run, and only the adversary's attribute matrix kills it
- file: crates/inbox-cli/src/output.rs
  line: 138
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the decided-on-silence line of a printed record has no case at any level, so removing it fails nothing
```
