---
format: aep.planning-md/3
id: review-result:adversary-http-service-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:http-service (973b5a7): 2 findings, both notes, 0 red cases'
relations:
- reviews: story:http-service
revision: 1
---
2 findings. Both are gaps in the unit's own suite that only the new cases close. No defect is red on the tree.

```
unit: story:http-service, impl/http-service at 973b5a7 (correction df9d07b..973b5a7, base c582901), plus one uncommitted test file
verdict: CONFIRMED
cases: executed 31→38, red 0
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the brief's scratch directory, adversary2/)
needs-coordinator: whether two mutant gaps closed by green cases (note) hold the unit
```

## 1. Diff stat

`git --no-pager diff --stat` is empty: nothing tracked changed. `git status --short` shows only `?? crates/inbox-server/tests/adversary2.rs`, a test file. No implementation path was touched.

## 2. Cases added (`crates/inbox-server/tests/adversary2.rs`)

Each was run alone first (`cargo test -p inbox-server --locked --test adversary2 -- --exact <name>`). All 7 are green on the tree. Each one turns red on at least one mutant (part 3).

- `:83 adv2_stop_returns_while_a_refused_connection_keeps_sending`: the request is refused at once (`BAD\r\n`), the caller keeps sending a byte every 300 ms, and `request_timeout` is 3600 s. `stop` must return within 3 s. Green. Red on m11 and m02.
- `:112 adv2_a_refused_connection_that_keeps_sending_gives_its_slot_back_at_the_request_timeout`: `max_connections` 1 and `request_timeout` 1.5 s. The same refused, dripping connection must not keep a signed GET from being served within 4 s. Green. Red on m11, m01, m12 and m13.
- `:143 adv2_the_first_connection_after_an_idle_time_is_timed_out`: the service sits idle for 1.5 s, then a half-sent request gets 408 within 950 ms. Green. Red on m01, m09, m12 and m13.
- `:160 adv2_the_408_comes_at_the_request_timeout_and_not_before`: with `request_timeout` 700 ms the answer is 408, and it does not come before 650 ms. Green. Red on m17.
- `:177 adv2_a_request_cut_while_its_body_is_read_is_one_408`: the head arrives in full, then the body drips past the bound. The caller gets exactly one answer, `408 Request Timeout` with `{"refused":"request timeout"}`. Green. Red on m01, m09, m12 and m13.
- `:225 adv2_a_signed_request_that_arrives_slowly_inside_the_request_timeout_is_served`: a signed request sent in 5 pieces 400 ms apart, inside a 3 s bound, is answered 200. Green.
- `:242 adv2_slow_clients_that_reconnect_hold_a_signed_request_at_most_about_one_request_timeout`: `max_connections` 2 and `request_timeout` 1.5 s. Two clients send a byte every 900 ms and reconnect as soon as they are cut off. A signed GET is still served within 3 s. Green. Red on m01, m12 and m13.

Red output on the mutants, verbatim:
- m11: `panicked at crates/inbox-server/tests/adversary2.rs:104:10: Running::stop returns within 3 s while a connection whose request was refused keeps sending a byte every 300 ms: Timeout`, and `panicked at crates/inbox-server/tests/adversary2.rs:132:32: a signed request is served within 5 s while the one slot is held by a refused connection that keeps sending a byte every 300 ms (request_timeout 1.5 s): Timeout`
- m17: `panicked at crates/inbox-server/tests/adversary2.rs:172:5: assertion left == right failed: after 1.004326518s: {"refused":"a header could not be read: Resource temporarily unavailable (os error 11)"} left: 400 right: 408`

## 3. Suite run, after all cases existed

`CARGO_INCREMENTAL=0 cargo test -p inbox-server --locked --no-fail-fast`, exit 0:
- unittests src/lib.rs: ok. 5 passed; 0 failed; 0 ignored
- tests/acceptance.rs: ok. 1 passed; 0 failed; 0 ignored
- tests/adversary.rs: ok. 6 passed; 0 failed; 3 ignored
- tests/adversary2.rs: ok. 7 passed; 0 failed; 0 ignored
- tests/cases.rs: ok. 19 passed; 0 failed; 0 ignored
- doctests: ok. 0 passed

Other checks:
- **20 runs in a loop:** all 20 exited 0 with the same counts in every run (5, 1, 6+3 ignored, 6, 19). No run differed. The loop ran before case :160 existed.
- **Format and lint:** `cargo fmt --check` exit 0; `cargo clippy -p inbox-server --all-targets --locked -- -D warnings` exit 0.
- **Gate:** `CARGO_INCREMENTAL=0 slot task check` exit 0.

**Mutants, one at a time, in a scratch copy, running the whole package suite:**
- Caught by the unit's own cases:
  - m01: no cut at the deadline.
  - m02: no cut at stop.
  - m04: a watch is not removed on drop.
  - m05, m06, m07: `checked_add` replaced by `+` in `watch`, `sleep` and `waited`.
  - m08: no clamp in `format_instant`.
  - m09: no 408 mapping.
  - m10: no 503 mapping.
  - m12: `watch` wakes nobody.
  - m13: the reaper sleeps past the next deadline.
- Caught only by the new adversary2 cases:
  - m11: the linger after a refusal runs unwatched (`drop(watch)` moved before `linger`).
  - m17: the deadline is read as twice `request_timeout`.
- Caught by no case:
  - m03: a connection accepted while stopping is put in the map instead of cut.
  - m14: the `stopping()` break after `accept` is removed.
  - m15: a request read in full stays watched while it is answered.
  - m16: the first cut reason is overwritten by a later one.
  - m18: `deadline <= now` changed to `<`.

  m15 and m18 change nothing a client can see. m03, m14 and m16 depend on a race between `stop` and one connection thread that no outside case can force; they are listed under part 5, not as findings.

## 4. Findings

- **F1** (`crates/inbox-server/src/listener.rs:272`, CONFIRMED, introduced, note)
  - What breaks: the note says a refused request "stays watched while the service reads and drops what the caller still sends". No case of the unit pins that.
  - Evidence: with mutant m11 the generated `linger` (up to 64 reads of 0.5 s) runs unwatched. A refused connection that keeps sending then holds a slot about 19 s past `request_timeout`, and it holds `stop` as well. The unit's 31 cases pass on that mutant.
  - What reaches it: any local unauthenticated client. Cases :83 and :112 close the gap if they are kept.
- **F2** (`crates/inbox-server/src/signals.rs:170`, CONFIRMED, introduced, note)
  - What breaks: the value of `request_timeout` is pinned only loosely. A deadline of twice the bound (m17) passes all 31 cases, because `cases.rs` checks a 300 ms bound against `< 900 ms`.
  - Case :160 pins it from both sides: no earlier than the bound, and before the reader's 1 s gap.

Pass 1, one line each:
- **F1:** closed. `adversary.rs:142` is green, and m01, m12 and m13 turn it red. The adversary2 cases cover the idle reaper, a cut inside the body, the linger and slow clients that reconnect.
- **F2:** closed. `adversary.rs:167` and `cases.rs` `stop_answers_a_connection_still_sending_its_request_with_503` are green, and m02 turns both red. Case :83 covers a refused connection that lingers.
- **F3:** closed. `adversary.rs:191` and `every_duration_of_the_configuration_may_be_the_largest_there_is` are green, and m05, m06, m07 and m08 turn them red.
- **F4:** closed. `adversary.rs:226` and `:240` are kept and green.
- **F5:** closed. `adversary.rs:214` is kept and green.

## 5. Attacked and held

- **Reaper after an idle time:** `watch` goes through `update`, so it wakes the reaper (case :143; m12 is caught).
- **A request whose last byte arrives at the deadline:** Linux returns queued bytes before the end of the stream, so such a request is answered. Each branch writes exactly one answer (case :177).
- **Linger after 408 or 503:** the read side is already shut, so the linger ends at once. After any other refusal the watch is still in place, so the bound and the stop end the linger.
- **Pending wait:** it is ended by `stop` and by `max_wait`. A `max_wait` of `Duration::MAX` is answered.
- **Memory:** a watch is removed on drop, also after a panic. The duplicated socket handle is closed when its entry leaves the map.
- **Lock order:** the reaper never takes the section, and `command_answered` and the `Watch` drop run outside it. No inversion.
- **The 408 and 503 answers:** they carry only the fixed body plus `Content-Type`, `Content-Length` and `Connection: close`. Only the service's own timer or `stop` can cause one, so no client can make another client's connection get one.
- **Duration edges:** a duration of 0 for `max_wait`, `sweep_interval` (floored at 1 ms) or `answer_retention` is harmless. `Duration::MAX` means no deadline, as the note says.
- **Slow clients within the bound:** the note promises no wait bound for a signed request, only that `request_timeout` counts from the accept and that the kernel's backlog holds further connections. The code keeps that: the backlog is first in, first out, and case :242 is served within one bound. An attacker who queues connections ahead of a signed one in the backlog delays it by queued/`max_connections` × `request_timeout`. That is the accepted bound, and I measured no wait beyond what the note says.
- **Connection accepted after stop:** it is closed without an answer. That matches "accepts nothing more".
- **Task startup:** if the expiry thread cannot be started, the reaper is stopped and joined, and the error is returned.
- **Regressions:** the cases of pass 1's held list that touch the listener are green in all 20 runs.
- **Repository rules:** no person's name, e-mail or home path in the unit's diff; `forbid(unsafe_code)`; no comment in the manifest; `generated/` and `ess/` unchanged; dependencies only inside the workspace.
- **Not raised, no case could be built:**
  - A socket that cannot be cloned is not watched, so `stop` would wait on a slow client there. That needs the process to be out of file descriptors; the limit here is 1 000 000, so this is infeasible. The design note does not mention it.
  - A signed client that reads a very large answer slowly holds `stop`, because requests read in full are allowed to finish. This was already true at df9d07b and needs an authenticated client.
- **Process:** no tool call was denied. I did not take the worktree session lease that the role file describes; the brief does not ask for one. To stop the first mutant batch I killed two processes of my own (the mutant script and its `timeout` wrapper), so that case :242 could be changed to end its drip clients on stop instead of hanging under m01.

## 6. Paths written outside the worktree

All under the brief's scratch directory `adversary2/`:
- scripts: `alone.sh`, `mutants.sh`, `mutate.py`, `loop20.sh`
- logs: `alone.log`, `alone-1.log`, `alone-2.log`, `build.log`, `clippy.log`, `suite.log`, `suite-1.log`, `gate.log`, `loop20.log`, `loop/run-1..20.log`, `mutants.log`, `mutants-1.log`, `mutant/m01..m18*.log`
- original copies of the source files: `signals.rs.orig`, `listener.rs.orig`, `clock.rs.orig`, `service.rs.orig`

The tree copy `adversary2/copy` has been deleted.

## 7. Findings block

```findings
- file: crates/inbox-server/src/listener.rs
  line: 272
  category: mutant
  severity: note
  origin: introduced
  message: 'no case of the unit pins that a refused request stays watched while it lingers; with the watch dropped before linger a refused connection that keeps sending holds a slot and stop about 19 s past request_timeout, and all 31 unit cases pass'
- file: crates/inbox-server/src/signals.rs
  line: 170
  category: mutant
  severity: note
  origin: introduced
  message: 'the value of request_timeout is pinned only loosely: a deadline of twice the bound passes all 31 unit cases'
```
