---
format: aep.planning-md/3
id: story:a-bridge-case-ends-its-bridge-after-the-record-owes-no-line
kind: story
status: implemented
title: A bridge case ends its bridge after the record owes no line
summary: A case of the bridge waits on the stand-in's count of lines and ends its bridge before the bridge dropped the owed line; the next bridge writes it again (red gate run on the build node, 2026-11-24).
scope:
- confidence: cited
  path: crates/inbox-cli/tests/bridge_answer.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T15:10:39Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T15:10:39Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T15:40:33Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

A case of the bridge ends or restarts its bridge only after the bridge has finished the line
it wrote, so that the next bridge of the case owes no line.

The bridge records a line it owes before it writes it and drops the line from its record
only after the write has returned; a bridge started again writes every line still owed,
under the same key and with the same text (`docs/design/bridge.md`). The stand-in for
`connectors` of the cases counts a line when it is invoked, which is before the bridge has
the answer, and it counts a repeated key as a new line. A case that waits on that count and
then ends its bridge can end it between the two. On 2026-11-24 a gate run on the build node
ended red that way in 1 case of 1145:
`bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing` found the line of
its note twice, both under one key, after its second bridge. With a wait of 1.5 s between the
write and the record the case is red 10 times of 10; without help it was red 0 times of 40
alone and 0 times of 728 beside the other cases of the bridge.

## Acceptance

- `bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing` waits until the
  record of its two messages owes no line before it ends its first bridge (state transition:
  the line of a reply moves from "owed in the record" to "written and dropped from the
  record" before the bridge is ended).
- Every other case of `crates/inbox-cli/tests` that waits on the stand-in's count of lines
  and then ends or restarts a bridge is read: each one either waits the same way, or the
  story's record says why it cannot be red by this order of events.
- With the wait of 1.5 s between the write and the record put into the bridge for the
  proof (not committed), every changed case is green; without the change of the case, the
  named case is red.
- No line of the product changes. `CHANGELOG.md` does not change.
- `task check` is green; no case over 5 s is added and no changed case gets slower than 5 s.

## Context

- Not in this story: a stand-in that answers a repeated key as a replay of the first write.
  Whether the real tool does that for this write is read from its contract documents only
  and is not verified before the run on the real channel.
- Not in this story: what a bridge does when it is ended while the tool's process is still
  running after its post. The diagnosis names it as not tested.

## Scope

Written after the build, from the diff against `main` at c6639e7: every path below is read
from it (cited).

One case file (no file under a `src/` directory changes):

- `crates/inbox-cli/tests/bridge_answer.rs`: the case
  `a_reply_that_is_not_taken_gets_one_line_and_sends_nothing` waits until the record of its
  two messages owes no line before it ends its first bridge (6 lines added).

Read and not changed, each with the reason in the story's record: the other cases of
`crates/inbox-cli/tests` that wait on a count of the stand-in and then end or restart a
bridge (`bridge_answer.rs`, `adversary_bridge_answer.rs`, `bridge_once.rs`,
`adversary_bridge_once.rs`, `bridge_own_key.rs`, `bridge_read.rs`, `adversary_bridge_read.rs`,
`bridge.rs`).

Not touched: every product crate, `ess/`, `generated/`, `CHANGELOG.md`, `AGENTS.md` beyond
the row of the test step that every story record writes.

## Decisions

- The story was filed and accepted by the coordinator on 2026-11-24, after a gate run on the
  build node ended red in one case that the change under test does not touch. No panel of
  plan critics read the story.
- Where the fix goes. The options were: (A) the case waits on the bridge's own record until
  it owes no line, and then ends its bridge; (B) the stand-in for `connectors` answers a
  repeated key as a replay of the first write and does not count it again. A is taken: the
  red run came from the order of events in the case, and the bridge did what its design
  says. B changes what every case of the bridge counts and rests on a behaviour of the real
  tool that is read from its contract documents only.
- No line of the product changes. That a bridge started again writes an owed line under the
  same key is the design (`docs/design/bridge.md`).
- Only the named case changes. Every other case that waits on a count of the stand-in and
  then ends or restarts a bridge was read; each one waits on the bridge's record or on the
  state of the message before it ends its bridge, or starts no second bridge.
- No helper in `tests/support`: one place needs the wait.
- Not decided here: whether the real tool answers a repeated key of this write as a replay.
  It is not verified before the run on the real channel.
- Not decided here: what a bridge does when it is ended while the tool's process is still
  running after its post. No case holds it.
- `CHANGELOG.md` does not change: no shipped code and nothing a user of the release sees.

## Measured

- The red run: `task check` on the build node, 2026-11-24, on a tree of another story that
  does not touch the bridge, 1 of 1145 cases:
  `bridge_answer::a_reply_that_is_not_taken_gets_one_line_and_sends_nothing`, the assertion
  "no line again" (`tests/bridge_answer.rs:416`): the line of the note was there twice,
  both under one key. The next run of the same tree was green, 1145 passed.
- The diagnosis, on c6639e7 on the build node. Without help the case was red 0 times of 40
  alone and 0 times of 728 case runs beside the 91 cases of the bridge. With a wait of
  1500 ms in the bridge after the write of an owed line and before the record drops it
  (not committed), red 10 of 10, with the failure of the gate run. With that wait and the
  change of the case, 0 of 5; with the change alone, 0 of 40.
- Round 1: `git diff --stat` names `crates/inbox-cli/tests/bridge_answer.rs`, 6 lines added.
  With the wait of 1500 ms and without the change the named case is red 3 of 3; with the
  change green 5 of 5 (8.3 to 8.4 s with the wait). The changed case 20 of 20 green in a
  row, 2.28 to 2.43 s. No case added, no case over 5 s.
- The other places, read and measured in round 1. With 3 waits of 1500 ms in the bridge
  (after a post before its record, after the write of an owed line, before the change of a
  line) every case of `bridge_once`, `bridge_answer`, `adversary_bridge_answer`,
  `bridge_own_key` and `adversary_bridge_once` was green: 48 passed. The places and the
  reason each one cannot be red by this order of events: `bridge_answer.rs:512`, `:786`
  and `adversary_bridge_answer.rs:456` end no bridge before their assertions;
  `bridge_once.rs:972`, `:1145` and `:1326` wait on the end the bridge records after its
  write; the cases that end or restart a bridge after a post wait on the record of the post
  or on `Delivered` (`bridge_own_key.rs:205`, `:281`, `:351`, `bridge_read.rs:1256`,
  `adversary_bridge_read.rs:485`, `:638`, `:656`, `bridge.rs:660`, `:765`, `:812`, `:1230`,
  `bridge_answer.rs:674`, `:954`); in `adversary_bridge_once.rs:705` and
  `bridge_own_key.rs:763` the bridge ends by itself.
- Adversary pass 1 (review-result:adversary-bridge-owed-pass-1): no finding, 1 note with the
  verdict INFEASIBLE, no case added. It put 4 waits of 1500 ms into the bridge, each between
  an effect and its record (an answer, the mark of a reaction, a first read, an owed line):
  every case of the bridge green, 102 passed. With the wait of the case made always true and
  the 4 waits in, the named case is red at "no line again". It read 8 more places that no
  list held; none can be red by this order of events. The changed case 20 of 20 green, 2.29
  to 2.32 s; the cases of the bridge twice, 102 passed both times. The note: the helper
  `recorded()` gives an empty text also for an entry that is missing, so the wait would pass
  for a missing entry; both entries exist through the case. It did not reach: a bridge ended
  while the tool's process still runs after its post; a line changed by a second post while
  it is owed; the full `task check`.
- The gate of round 1 on the build node (`task check`): 1129 passed, 8 skipped, the test
  step 20 s, exit 0.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T15:36:46Z to 2026-11-24T15:37:31Z, on the build
  node of the pool: exit 0, `1159 tests run: 1159 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
