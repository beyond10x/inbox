---
format: aep.planning-md/3
id: story:case-stand-ins-bound-their-wait
kind: story
status: implemented
title: Every stand-in a case file writes bounds its wait for a connection
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T18:27:26Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T18:27:26Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-22T20:13:09Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
---
## Outcome

Every stand-in service a case file of the workspace writes for itself bounds its wait for a
connection, as the shared stand-in of `crates/inbox-cli/tests/support/mod.rs` does since
story:test-step-enforces-its-budget: a case that joins its stand-in and whose client sends
nothing fails within seconds with a message that names the stand-in and what did not arrive.

## Acceptance

- At each of the 5 places in `crates/inbox-cli/tests/` below, a stand-in whose client never
  connects ends within 15 s with a message naming the stand-in and the missing connection
  (state transition: a blocking `accept()` before, which only the runner's kill at 60 s
  ended; a bounded one after). A case per place shows it, and the case goes red or is killed
  at 60 s when the accept of that stand-in blocks again.
- At the 2 places in `crates/inbox-conformance/tests/` the thread of the stand-in ends within
  15 s. These threads are detached: a case there ends on its own assertion, before and after.
- The wait every plain stand-in uses is one constant per crate, and a value over 15 s does
  not compile.

## Context

Found by the implementor of story:test-step-enforces-its-budget on 2026-11-22, existing before
that story. These stand-ins called `accept()` or iterated `incoming()` with no bound:

- `crates/inbox-cli/tests/adversary.rs:70`
- `crates/inbox-cli/tests/adversary2.rs:190`
- `crates/inbox-cli/tests/adversary_w8_repost.rs:86`
- `crates/inbox-cli/tests/adversary_w8_repost2.rs:99`
- `crates/inbox-cli/tests/adversary_control_sequences.rs:319` (`raw_stand_in`, found while the
  story was built)
- `crates/inbox-conformance/tests/wire.rs:207`
- `crates/inbox-conformance/tests/adversary3.rs:54`

The first wording said that at each place only the runner's kill at 60 s ended a case whose
client never connects. Adversary pass 1
(review-result:adversary-case-stand-ins-bound-their-wait-pass-1, finding 1) showed that this
was false for the 2 conformance places, where no case waits on the thread. The coordinator
corrected the acceptance on 2026-11-22.

## Scope

- the 7 files above
- `crates/inbox-cli/tests/support/mod.rs` (`accept_within`, `accept_or_panic`, `STAND_IN_WAIT`)
- `crates/inbox-conformance/tests/support.rs` (new)

## Measured

- 8 new cases, 5 in `inbox-cli` (117 cases) and 3 in `inbox-conformance` (89 cases, 1
  ignored); each takes 1.0 to 1.3 s.
- Adversary pass 1: 9 mutants, 4 caught, 5 survived. The 5 run again after round 2: a blocking
  accept in `adversary_w8_repost.rs` and in `adversary_w8_repost2.rs` fails the case after
  3.0 s; in `raw_stand_in_within` the case is killed at 60 s; `STAND_IN_WAIT` of 3600 s does
  not compile in either crate.
- `task check` on the branch merged with `main` (release 0.3.1 and the build setting of the
  test step), 2026-11-22T20:10:01Z to 20:12:40Z, load average 47 to 59 on 20 CPUs: exit 0,
  `642 tests run: 642 passed, 5 skipped`, the test step 34 s of its bound of 120 s.
