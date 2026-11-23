---
format: aep.planning-md/3
id: story:the-tls-terminate-test-holds-on-a-loaded-host
kind: story
status: implemented
title: The TLS terminate test holds on a loaded host
summary: A unit case of the TLS listener decides a full buffer from a 500 ms stall and failed once in 4 runs under load
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:20:03Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:20:03Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T09:25:26Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

The unit case `tls::tests::terminate_returns_only_when_the_thread_that_reads_the_caller_has_ended`
of `crates/inbox-server/src/tls.rs` passes on a loaded host as on an idle one.

## Why

The case failed once in 4 runs of `cargo nextest run -p inbox-cli -p inbox-server --locked`
on 2026-11-23, on a host with a load average of about 70 to 90 on 20 CPUs, with "terminate
returned while the thread that reads the caller was still writing". It was found while
`story:inbox-wake-returns-on-a-post` was built and is older than that story: the file was not
changed by it. The case decides that the way in is full when writes stall for 500 ms. A loaded
host stalls a write for that long without a full way in, so the case then goes on from a state
it did not reach.

## Acceptance

- The case decides "the way in is full" from a fact and not from a pause of a fixed length
  (for example a write that would block on a socket set to non-blocking, or a counted number
  of bytes over the buffer sizes it set).
- It holds no wall-clock bound that a slow host breaks; a bounded wait only ends a hang.
- A mutant that makes `terminate` return before the reading thread has ended is still red
  against it.
- 20 runs of the case in a row are green while another build occupies the pool.

## Scope

`crates/inbox-server/src/tls.rs`: the test module, and one seam in the listener. `terminate`
calls a private `terminate_on(config, outer, pair, serve)` with its own `pair`; the test module
calls `terminate_on` with a pair it made itself. No behaviour of the listener changes (see
Decisions). The draft of this story said "the test module only. No change of the listener."

## Decisions

Taken by the coordinator on 2026-11-23 while the story was built.

- The seam in the listener. Round 1 of the implementor ended blocked: no fact inside the test
  module can prove that the way in is full, because the socket pair is private to `terminate`
  and the standard library sets no socket buffer size. (A) a private `terminate_on` that takes
  the function that makes the pair, 10 lines, the order of the listener's steps unchanged:
  taken. (B) the case reads the kernel's state of the sockets from `/proc`: not taken, it ties
  the case to one kernel. (C) a would-block on the caller's side alone, without the seam:
  tried first, it failed 1 of 20 runs.
- The 2 facts. The helper `the_way_in_full` makes the pair, fills it without blocking until a
  write would block, fills the caller the same way, and waits until a peek at a clone of
  `outer` returns a byte. A would-block says the buffer is full now, not that it stays full
  (a probe of the adversary wrote 1 byte 200 ms after a would-block), so both writers go on
  writing until the case ends. Then the reading thread can neither finish a write to the pair
  nor end on an empty read. A bounded wait of 20 s only ends a hang.
- The sibling case. `a_caller_gone_in_the_middle_of_an_answer_fails_the_services_next_write`
  took its state from the same stall of 500 ms. (A) it is fixed in this story with the same
  helper: taken. (B) a draft story of its own: not taken, the defect and the fix are the same.
  The case no longer says that the service has read the first byte of the request: with the
  pair filled by the case that byte proves nothing, and the cases of
  `crates/inbox-server/tests/tls.rs` hold a request read over TLS.
- Mutants that stay green, read as no gap. `Shutdown::Both` to `Shutdown::Write` in the thread
  that writes the answer: the same in effect on Linux, the only system the gate and the
  release run on (the adversary's probe: an end shut for reading still reads what is queued,
  and its peer goes on writing; the service's write fails by the half shut for writing and the
  close with unread bytes). A join of the reading thread that gives up after 1 s: the case
  keeps that thread alive for about 300 ms after the answer ended, as it did before this
  story, and "has not returned yet" has no form without a window; the listener has no such
  join.
- The window of 300 ms of `terminate_returns_only_when_the_thread_that_writes_the_answer_has_ended`
  stays as it is: it must run before the answer is released, so no fact can stand for it. It
  cannot turn the case red against the correct listener.

## Measured

- The flaw, round 1: with the pause of the old case set to 0 ms the case goes on from a way in
  that is not full; the first form without the seam (a would-block on the caller's side alone)
  failed 1 of 20 runs.
- 20 runs of the 2 cases together by the implementor, load average 53 to 69 on 20 CPUs: 20
  green, each run 0.31 to 0.54 s. 20 runs by the adversary at a load average of 103 to 110: 20
  green, the slowest 0.96 s. The sibling case runs 0.010 to 0.013 s; it ran 0.706 s before.
- `cargo nextest run -p inbox-server --locked`: 127 passed before and after (2 cases rewritten,
  none added).
- Mutants. `terminate` returns before the reading thread has ended (the join dropped): red in
  5 of 5 runs after about 0.04 s, "terminate returned while the thread that reads the caller
  was still writing". The thread that writes the answer no longer shuts the pair when the
  caller is gone (`if failed` to `if false`), and `Shutdown::Both` to `Shutdown::Read`: each
  red at the sibling after its bound of 20 s, "the service's write fails: Timeout". 2 mutants
  stay green and are read as no gap under Decisions.
- The adversary pass (review-result:adversary-tls-terminate-test-pass-1) wrote no red case: 2
  notes, the Scope of this story (amended above) and the join bounded to 1 s.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T09:14:45Z to 2026-11-23T09:22:37Z, load average 104 to 113
  on 20 CPUs: exit 0, `881 tests run: 881 passed, 8 skipped`, the test step 86 s of
  its bound of 120 s.
