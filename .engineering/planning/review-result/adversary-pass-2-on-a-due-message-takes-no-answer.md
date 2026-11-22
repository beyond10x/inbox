---
format: aep.planning-md/3
id: review-result:adversary-pass-2-on-a-due-message-takes-no-answer
kind: review-result
status: active
title: Adversary pass 2 on story:a-due-message-takes-no-answer
relations:
- reviews: story:a-due-message-takes-no-answer
revision: 1
---
unit: story:a-due-message-takes-no-answer, the branch `fix/answer-deadline` after round 2 of the implementor
verdict: NEEDS-CHANGE
cases: executed 379→390 (the packages `inbox-core`, `inbox-server`, `inbox-cli`, `inbox-conformance`), red 1
origin: introduced 4 / pre-existing 0 / undecided 0

The report of the adversary (one agent, 2026-11-22), shortened by the coordinator, with the path
of the unit's tree and of the scratch directory left out. What the coordinator did with each
finding is at the end.

**1. Diff.** The adversary edited no implementation file. It added
`crates/inbox-core/tests/adversary_deadline_2.rs` (6 cases) and
`crates/inbox-server/tests/adversary_deadline_2.rs` (5 cases), and one `mod` line in each
`tests/main.rs`. Every mutation ran in a copy outside the tree.

**2. Cases added (11).** In `inbox-core`: a reply refused at its own `expires` and taken a
nanosecond before, over 17 forms of the instant (fractions of 1, 2 and 9 digits, 2 offsets, lower
case, a reading written with an offset); `Answer` takes a reply where the sweep at that reading
keeps it, over the same 17 forms; the order of the expired reply among the other refusals (after
the party checks, before the shape); a delegate's reply held to its own `expires`; an expired
reply to a decision leaves it to silence at the deadline; a repeated answer to an answered
message after the reply's own `expires` (red, F1). In `inbox-server`, over HTTP on the manual
clock at a deadline with a fraction and 1 ns before it: a post, a reply, a refusal, the sweep,
and an expired reply to a decision. No case runs over 5 s.

**3. Suite run.** `cargo nextest run -p inbox-core -p inbox-server -p inbox-cli -p
inbox-conformance --no-fail-fast`: 390 run, 389 passed, 1 failed (the red case), 1 skipped.
Without the 11 cases: 379 run, 379 passed.

Mutants, each against `inbox-core` and `inbox-server` (224 cases): `<` in place of `<=` in the
comparison of the reply (killed by 4 new cases only); the branch always false (killed by a case
of pass 1 and 7 new ones); the branch asked before the party checks (killed); the branch asked
before `deadline-passed` (killed by 1 new case only); the reading of a command cut to the second
(killed); the reading of the sweep cut to the second (killed by 2 new cases only); `keep_until`
cut to the second (survived every case); the fix of F1 as a probe (all 224 green).

**4. Findings.**

F1. `crates/inbox-core/src/context.rs`, the branch of `Answer` for an expired reply. A repeated
or second `Answer` to a message already `Answered` or `Refused` whose reply's own `expires` has
passed is answered `expired-on-arrival` where the design document and the limits page say
`already-settled`. Reached by a client that sends its answer again after the reply's `expires`
and before the next sweep. The interpreted target of ESS 0.97.3 has the same order, so the suite
cannot tell the two apart. Introduced by round 2.

F2. `ess/domains/messaging.yaml`, the summary of `AlreadyExpired`: it names only a post's
`expires`, and the generated documents copy it. Introduced by round 2.

F3. Two mutants of the comparison and of the order survive the suite of the unit without the
cases of this pass.

F4. `crates/inbox-server/src/expiry.rs`: no case checks the fraction of `keep_until`.

F5. `crates/inbox-server/src/clock.rs`: the started clock goes back by a backward step of the
system clock that lands after its start; its doc comment and the name of its unit case claim
more. Not testable without stepping the host clock; reached only through the hidden flag of the
conformance run.

**5. Attacked and not broken.** The order for a caller that is no party; `Answer` against the
sweep over the 17 forms; the core and the service agree (one comparator); a delegate's reply; a
decision with a default on silence whose reply is refused and then swept; the reading written
back at nanosecond precision; a leap second and a fraction over 9 digits are refused before the
new branch; the hidden flag `--clock-start` is in no generated document and no page.

**What the coordinator did.**

- F1: back to the implementor (round 3): the branch holds only for a message in `Sent`,
  `Delivered` or `Read`, in `ess/` as a `when_subject` and in the core, with the authored scenario
  `a-second-expired-reply-to-an-answered-request-is-already-settled`. The red case is green.
- F2: fixed in round 3, the documents regenerated.
- F3: the 11 cases of the pass stay in the suite.
- F4: one case in round 3 pins the fraction of `keep_until`; the mutant that cuts it is killed.
- F5: the doc comment states the behaviour and the unit case is renamed; no change of behaviour.

```findings
- file: crates/inbox-core/src/context.rs
  line: 285
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a repeated or second Answer to a message already Answered or Refused whose reply's own expires has passed is answered expired-on-arrival instead of the documented already-settled, telling the client its stored answer was not taken
- file: ess/domains/messaging.yaml
  line: 295
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the AlreadyExpired error summary names only a post's expires against the clock, while Answer now raises it for a reply's own expires, and the generated schema, docs and openapi copy that text
- file: crates/inbox-core/src/context.rs
  line: 286
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the strict-less mutant of Answer's reply comparison and the mutant asking it before deadline-passed survive the unit's suite including pass 1, killed only by the pass-2 cases
- file: crates/inbox-server/src/expiry.rs
  line: 41
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: cutting the sweep's now to the second survives the unit's suite, and cutting keep_until to the second survives every case
- file: crates/inbox-server/src/clock.rs
  line: 87
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: StartedClock on SystemTime goes back by any backward step of the host clock that lands after its start, while its doc and its unit test name claim it holds or does not go back; not testable without stepping the host clock
```
