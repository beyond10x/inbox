---
format: aep.planning-md/3
id: review-result:adversary-silence-leaves-the-other-fields-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:silence-leaves-the-other-fields-scenario (b0a4ac5): 4 findings (2 warnings, 2 notes), 1 introduced, 3 there before the unit'
relations:
- reviews: story:silence-leaves-the-other-fields-scenario
revision: 1
---
## What was reviewed

story:silence-leaves-the-other-fields-scenario at `b0a4ac5` (branch `impl/w8-silence-fields`),
by one adversary on Sonnet, pass 1 of at most 2, on 2026-11-21. No second pass ran: the
correction adds the adversary's own 3 cases; no source line of a crate and no scenario changed.

## Verdict

The claim of the unit holds for the members the scenario names: each of the 5 changed cores of
the brief fails the new scenario `silence-leaves-the-decision-as-it-was` (98 of 99). 4 findings:
1 introduced by the unit, 3 there before it. 19 other changed cores passed `task conform`.

## Findings

- F1 (warning, there before the unit; `tools/scenarios.py`): no scenario claims the members
  `on_behalf_of`, `to`, `kind`, `reply_shape`, `dedup_key`, `signature`, `authority` and
  `refusal` of a decision decided on silence. 9 cores whose `Expire` changes one of them, and 1
  that clears `body` of every other row of the thread, passed `task conform` (99 of 99).
- F2 (warning, introduced; `tools/scenarios.py`): the new scenario decides from `Sent` only. 6
  cores that change a member only when the decision was Delivered or Read passed
  `task conform`; the scenarios for those 2 states claim the 5 members they claimed before.
- F3 (note, there before the unit; `tools/scenarios.py`): in every scenario the recommended
  option, the silence option and the last option are the same letter. 3 cores that take
  `decided_option` from the recommended option, from the last option or from a literal passed
  `task conform`.
- F4 (note, there before the unit; `crates/inbox-conformance/tests/adversary3.rs`): the total
  the unit changed to 99 cannot fail the suite; the case passed with 98.

## Changed cores (each built in a copy of the tree)

| Change in `Expire`, on silence | `task conform` |
|---|---|
| the 5 of the brief | fails, the new scenario |
| options reversed, recommended option cleared or changed, `request_ref` set | fails |
| `default_on_silence`, `body`, `created`, `from`, `thread`, `expires` changed | fails |
| `to` or `thread` of the event wrong | fails |
| `on_behalf_of`, `to`, `kind`, `reply_shape`, `dedup_key` changed | passes |
| `signature.value`, `signature.key_id`, `authority`, `refusal` changed | passes |
| `body` of the other rows of the thread cleared | passes |
| a claimed member changed only from Delivered or Read (6 cores) | passes |
| `decided_option` from the recommended option, the last option, a literal | passes |

## What held

- The scenario compares the 10 members it lists: a wrong `created`, a `thread` set to the
  message id and a `body` emptied or altered each fail it.
- `crates/inbox-conformance/unsupported.tsv`: with the new line removed the gate fails (the
  scenario is unsupported and not listed); with the scenario removed and the line kept it fails
  (listed and not unsupported in the run).

## Cases

3 cases in `crates/inbox-core/tests/adversary_w8_fields.rs`, green on `b0a4ac5`: a decision with
every optional member set and a silence option that is not the recommended one is decided on
silence from Sent, Delivered and Read, and the stored message must equal the one posted apart
from `expires`, `answered_by` and `decided_option`. They fail 12 of the cores that passed
`task conform`; the Delivered and Read cases fail the 6 cores of F2. The core that clears other
rows of the thread passes them. A scenario that would hold F1 to F3 in the suite is given as
text in the adversary's report and is not written.
