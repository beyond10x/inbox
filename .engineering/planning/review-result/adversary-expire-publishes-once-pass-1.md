---
format: aep.planning-md/3
id: review-result:adversary-expire-publishes-once-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:expire-publishes-once (7b00bbb): 4 findings (1 blocker, 2 warnings, 1 note), 2 there before the unit'
relations:
- reviews: story:expire-publishes-once
revision: 1
---
## What was reviewed

story:expire-publishes-once at `7b00bbb` on base `d52ba0f` (branch `impl/w8-expire-once`), by
one adversary on Sonnet, pass 1 of at most 2, on 2026-11-21. No second pass ran: the correction
adds the adversary's own 6 cases and changes one comment; no source line of a crate changed.

## Verdict

Broken. 4 findings: 1 blocker and 1 note introduced by the unit, 2 warnings there before it.
5 changed cores that publish an event of `Expire` twice passed the unit's 9 cases,
`cargo test -p inbox-core` and the conformance run of the core.

## Findings

- F1 (blocker, introduced; `crates/inbox-core/tests/expire_once.rs`): the 9 cases build one
  message whose thread is its own id and whose sender speaks for itself, with `now` equal to
  `expires` and at most 2 `Expire` calls on a backend. A core that publishes the event twice
  only when `now` is after `expires`, only when the store holds more than 1 message, only for a
  message sent on behalf of another principal, only when the thread is not the message id, or
  from the sixth call of a process on, fails 0 of the 9 cases. The sweep of the service sends
  the clock reading as `now`, which is after `expires` for a message not swept in the same
  second.
- F2 (warning, there before the unit; the fixture of the cases and the scenarios of the suite):
  thread equals the message id and `on_behalf_of` equals `from` wherever `Expire` decides on
  silence, so a core that writes the message id as `thread` or the principal spoken for as `to`
  of `DecisionTakenOnSilence` passes `cargo test -p inbox-core` and the conformance run.
- F3 (warning, there before the unit): no case and no scenario sweeps a refused decision that
  names a `default_on_silence`. A core that counts `Refused` as open answers that sweep with an
  unmet obligation in place of `expired` and passes both.
- F4 (note, introduced; the header of `expire_once.rs` and the body of `7b00bbb`): the sentence
  that no step of the suite reads a field of an event `Expire` publishes is untrue.
  `expect_event` compares the whole payload; a core with a wrong `to` fails scenario
  `decision-taken-on-silence` at step 20. What holds: no step bounds the number of occurrences
  from above.

## Changed cores (21 built in a copy of the tree)

| Change | of the 9 cases red | older tests red | the 6 new cases red | conformance |
|---|---|---|---|---|
| the last event twice (the story's own) | 7 | 0 | 6 | passes |
| twice, only `MessageExpired` | 5 | 0 | 3 | passes |
| twice, only `DecisionTakenOnSilence` | 3 | 0 | 4 | passes |
| twice, only when thread is not the id | 0 | 0 | 1 | passes |
| twice, only in state `Read` | 1 | 0 | 0 | passes |
| twice, only for a request | 1 | 0 | 0 | passes |
| twice, only when `now` is after `expires` | 0 | 0 | 3 | passes |
| twice, only with more than 1 message stored | 0 | 0 | 1 | passes |
| twice, only when `from` is not `on_behalf_of` | 0 | 0 | 1 | passes |
| an extra `MessageRead` | 7 | 0 | 6 | not run |
| `MessageExpired` in place of the declared event | 3 | 0 | 4 | not run |
| no event | 7 | 0 | 6 | not run |
| twice from the sixth call on | 4 | 0 | 1 | passes |
| `to` is `on_behalf_of` | 0 | 0 | 1 | passes |
| `thread` is the message id | 0 | 0 | 1 | passes |
| `to` is the recipient address | 3 | 1 | 4 | fails, 1 scenario |
| `Refused` counted as open | 0 | 0 | 1 | passes |
| `Delivered` not counted as open | 1 | 0 | 0 | passes |
| the id of `MessageExpired` read from the row | 0 | 0 | 0 | not run; same behaviour |
| late is not due | 7 | 3 | 4 | fails, 4 scenarios |
| `Expire` deletes every other due message | 0 | 0 | 1 | fails, 1 scenario |

## What held

- The unit's 9 cases fail a core that doubles the event for every message, by event type, in
  state `Read` or for a request, one that adds an event, publishes the wrong one or none.
- Each case compares a whole event list with a literal, asserts the outcome of every command
  that builds its state, and cannot pass on a default value.
- A doubling in `handle` alone cannot be observed: `handle` returns what `handle_with_events`
  hands back, the sweep discards the events, and `Expire` is granted to the Sweeper only.

## Cases

6 cases in `crates/inbox-core/tests/adversary_w8_expire.rs`, green on `7b00bbb`, each red on at
least 1 of the changed cores.
