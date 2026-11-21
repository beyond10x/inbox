---
format: aep.planning-md/3
id: review-result:adversary-read-decision-expires-on-silence-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:read-decision-expires-on-silence-scenario (469d7ee): 4 findings (1 warning, 3 notes), each there before the unit'
relations:
- reviews: story:read-decision-expires-on-silence-scenario
revision: 1
---
## What was reviewed

story:read-decision-expires-on-silence-scenario at `469d7ee` on base `d52ba0f` (branch
`impl/w8-read-silence`), by one adversary on Sonnet, pass 1 of at most 2, on 2026-11-21. No
second pass ran: the correction adds the adversary's own 8 cases; no source line of a crate and
no scenario changed.

## Verdict

The claim of the unit holds: a core whose `Expire` leaves `Read` out of the states it decides
on silence fails step 18 of the new scenario (97 of 98). 4 findings, none introduced by the
unit: 1 warning and 3 notes, each there before it. 4 other changed cores passed `task check`
with the unit.

## Findings

- F1 (warning, there before the unit; `tools/scenarios.py`): no scenario expires a read
  decision that names no silence option. A core that decides such a decision on silence, with
  no decided option, passed `task check`.
- F2 (note, there before the unit; `tools/scenarios.py`): every instant of the scenarios is a
  whole minute. A core that decides a read decision 1 second before `expires` passed
  `task check`; 60 seconds early fails.
- F3 (note, there before the unit; `crates/inbox-core/src/obligations.rs`, `Expire`): no
  conformance scenario expires a request that is delivered or read. A core that decides a
  delivered request on silence passed `task check`; for a read request it passed `task conform`
  and failed only `task acceptance`.
- F4 (note, there before the unit; `crates/inbox-conformance/tests/adversary3.rs`): the pinned
  total of the case `the_gate_refuses_a_listed_reason_that_is_not_the_reason_of_the_run` cannot
  fail. The case passes with 97 and with 50. The unit's edit of 97 to 98 changes nothing the
  gate checks.

## Changed cores (15 built in copies of the tree)

| Change in `Expire` | `task conform` | `task check` |
|---|---|---|
| `Read` left out | fails, step 18 of the new scenario | not run |
| `Delivered` left out | fails, `decision-taken-on-silence` | not run |
| `Sent` left out | passes | fails, a case of the core |
| a read decision decided 60 s early | fails, step 13 | not run |
| a read decision decided 1 s early | passes | passes |
| a read decision without a silence option decided | passes | passes |
| a read decision decided with option A | fails, step 22 | not run |
| `not-due` skipped for a read message | fails, step 13 | not run |
| `to` of the event is the recipient, for `Read` | fails, step 20 | not run |
| a read decision answered in the store, outcome `expired` | fails, step 18 | not run |
| a read request decided on silence | passes | fails, `task acceptance` |
| a delivered request decided on silence | passes | passes |
| a sent note decided on silence | fails, 2 scenarios | not run |
| `keep_until` ignored for `Read` | fails, step 22 | not run |
| `answered_by` is the principal, for `Read` | fails, step 22 | not run |

## What held

- The scenario reaches the state `Read` before `Expire` runs: the core without `Delivered`
  leaves it green, the core without `Read` fails it.
- The steps that expect `not-due` and no event can fail.
- `crates/inbox-conformance/unsupported.tsv`: the reason of the new line is the reason the
  serve run gives, byte for byte. With the line removed the gate fails (the scenario is
  unsupported and not listed); with the scenario removed and the line kept it fails (listed and
  not unsupported in the run).

## Cases

8 cases in `crates/inbox-core/tests/adversary_w8_read.rs`, green on `469d7ee`. 6 of them fail
at least 1 changed core; with them the 3 cores of F1 to F3 that passed `task check` fail
`cargo test -p inbox-core`. The scenarios that would hold F1 and F3 in the suite are given as
text in the adversary's report and are not written.
