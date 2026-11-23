---
format: aep.planning-md/3
id: review-result:adversary-tls-terminate-test-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the TLS terminate cases: no red case, 2 notes'
relations:
- reviews: story:the-tls-terminate-test-holds-on-a-loaded-host
revision: 1
---
unit: story:the-tls-terminate-test-holds-on-a-loaded-host, branch backlog/tls-terminate-test at 43d27cd (diff against 043ea56). Attacked in a scratch copy made with git archive HEAD.
verdict: nothing found that a case can make red. The listener seam is the same listener as before, and both cases hold at load 90 to 110. There are 2 notes: a story Scope line no longer matches the diff, and a bounded-join mutant survives because of the 300 ms window. That window is a property the case had before this change.
cases: executed 127→127, red 0 (I added no case; the 127 comes from the implementor's r2-suite.log)
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 7 paths (listed in part 6)
needs-coordinator: the story's Scope still says "No change of the listener", but the diff adds the seam you allowed. The story text needs amending, or the decision needs recording.

1. git --no-pager diff --stat of the story's tree: empty (0 lines from git status --short). I wrote nothing in <tree>. I made every mutant and the probe in the copy and restored the copy after each one (each step ends with cmp against tls.rs.orig: equal).

2. Cases added: none. No attack produced a red case (see parts 4 and 5), so there is no adversary-cases.rs.

3. Suite run, made after the mutant runs on the restored copy: `cargo nextest run -p inbox-server --locked` returned exit 0 with "Summary [9.418s] 127 tests run: 127 passed, 0 skipped". Load was 100.79 at the start and 102.34 at the end. The slowest test was 2.197s (adversary::adv_a_slow_unauthenticated_connection_does_not_hold_a_signed_request). The two story cases each took under 1 s.
- 20 runs of the 2 cases together, at load 109.99 at the start and 103.13 at the end: all exit 0. The slowest run took 0.959s (run 6: terminate_returns_only_when_the_thread_that_reads_the_caller_has_ended 0.958s). Every other run was between 0.33 and 0.49s.
- 20 runs of the tls::tests module (8 cases), at load 95.52 at the start and 100.79 at the end: all exit 0. The slowest run took 0.696s and its slowest case was 0.637s (the reads-the-caller case).

4. Findings:
- Story Scope against the diff.
  - Measured: .engineering/planning/story/the-tls-terminate-test-holds-on-a-loaded-host.md:40 says "the test module only. No change of the listener." The diff changes the listener at crates/inbox-server/src/tls.rs:524-538 (terminate now calls a new private fn terminate_on).
  - What reaches it: your round-2 permission. Neither the story nor a decision in it records that permission.
  - Fix: amend Scope, or add a Decisions entry.
  - Verdict CONFIRMED, origin introduced, severity note.
- A bounded join of the reading thread survives.
  - Measured: mutant MA replaces tls.rs:579 `let _ = inbound.join();` with a join that gives up after 1 s (the thread is then detached). Both cases passed in 20 of 20 runs (0.31 to 0.40s).
  - Why: the case keeps the reading thread alive for only about 300 ms after the answer ends, then drops `plain`. Any join timeout longer than that window cannot be seen. The base case had the same 300 ms window, so this is pre-existing. A wider window only moves the limit, and no window-free form exists for "has not returned yet".
  - What reaches it: no product code has such a join. The story's Acceptance names only "returns before the reading thread has ended", and that mutant is red.
  - Verdict INFEASIBLE, origin pre-existing, severity note.

Mutants (all made in the copy):
- MD `drop(inbound)` at :579 (detach; the story's mutant): red in 5 of 5 runs, at about 0.04s, from terminate_returns_only_when_the_thread_that_reads_the_caller_has_ended. The red line, verbatim: "panicked at crates/inbox-server/src/tls.rs:850:9: terminate returned while the thread that reads the caller was still writing".
- MA, a join with a 1 s timeout at :579: survived 20 of 20 (finding 2 above).
- MB, the reading thread's join dropped only when outbound failed (outbound returns `failed`, and :579 becomes `if !failed { join }`): survived 20 of 20 on both cases. I judge it equivalent in effect. On the failed path, outbound shuts inner (SHUT_WR included), so the reading thread's blocked write gets EPIPE and the thread ends within microseconds. No caller can observe the early return.
- MC `Shutdown::Both` → `Shutdown::Write` at :667: survived 5 of 5 on the sibling case, and the whole crate passed with it (127 of 127). Decided: equivalent on Linux, and CI and release run only on ubuntu-latest.
  - Probe on this host: shut a pair end for reading, then write to it from the peer, non-blocking. Output: "PROBE sent=2625024 stop=Some(WouldBlock) again=Ok(1) read_after_shut_rd=Ok(113152)". The peer keeps writing, nothing reaches it, and the shut end still reads the queued bytes.
  - So SHUT_RD alone is invisible to the service. The only reader of inner is outbound itself, which returns right after. The service's write fails because of the SHUT_WR half plus the final close of inner with unread bytes (RST).
  - Not checked: other operating systems. Windows resets on data after SD_RECEIVE; inbox does not build for it.
- The implementor's `if failed → if false` and `Both → Read`: I did not re-run them. Their logs show red at 20.03s ("the service's write fails: Timeout").

5. Attacked and could not break:
- The seam. At the base the order is the same (ServerConnection::new, then pair(), then the same early return). When pair() fails, outer is dropped unclosed by TLS, serve is not called and no thread is started; that path is identical before and after. terminate_on is a private fn, and outside the test module only terminate calls it, with the fn item `pair`.
- The 2 facts.
  - A would-block is not a lasting "full": the probe got Ok(1) 200 ms after a would-block. The helpers cover this with the writers that go on, as their comments say.
  - The reading thread can end only on an empty read of outer, an error, or a failed write to inner. Outer stays fed by the caller's writer, and the probe shows Linux still queues and returns data after SHUT_RD. The pair is never read. So no order of the 2 fills lets the correct terminate return inside the window.
  - The peek can fail only after a 20 s stall.
- The writers. Both are joined in both cases. The caller's writer has a 20 s write timeout and then ends quietly, which turns nothing red. The filler ends on RST (case 1) or EPIPE (case 2). A panic before the joins leaks threads only inside nextest's per-test process.
- Bounded waits.
  - Every one is HANG = 20 s, and the service's write timeout is 40 s. The fills are bounded by a byte count of 1 GiB, not by time.
  - A red mutant of the sibling ends at about 20 s. The peek can add at most 20 s more, but only if no byte ever arrives, so the worst case is about 40 s, under the 60 s kill.
  - The 300 ms window of the writes-the-answer case (:714) is untouched, and this change does not make it worse. In the reads-the-caller case the window now starts after the answer's end rather than after serve returned, which narrows what a mutant can slip through.
- Sockets. Everything is on 127.0.0.1 (pair() falls back to ::1), on ephemeral ports. Every listener is dropped inside its helper.
- Buffer sizes. This host has tcp_wmem max 4 MiB and tcp_rmem max 32 MiB, and the cases run in under 1 s. With 16 MiB buffers a fill is tens of MiB in 64 KiB writes, which takes milliseconds and kernel buffer memory only.
- Design claims. Every claim of docs/design/service.md:422-444 (the TLS section) is covered by the end-to-end cases (crates/inbox-server/tests/tls.rs, adversary_tls*.rs, security_tls.rs). The sentence the sibling lost ("the service reads the first byte before the caller goes") is pinned by the request-over-TLS cases there.

Not checked: other operating systems; clippy on the copy (I added no case).

6. Paths written outside the worktree, all under <scratch>/bl/tls/adversary/:
- copy/ (archive of HEAD; its target/ is deleted)
- run.sh
- probe.rs.txt
- build.out
- all.out
- all2.out
- logs/ (build.log, base-1..20.log, probe.log, md-1..5.log, ma-1..20.log, mb-1..20.log, mc-1..5.log, mc-crate.log, module-1..20.log, suite.log, tls.rs.orig)

```findings
[
  {"file": ".engineering/planning/story/the-tls-terminate-test-holds-on-a-loaded-host.md", "line": 40, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The story's Scope still says no change of the listener while the diff adds the terminate_on seam in the listener, and the permission for it is not recorded in the story."},
  {"file": "crates/inbox-server/src/tls.rs", "line": 579, "category": "mutant", "severity": "note", "verdict": "INFEASIBLE", "origin": "pre-existing", "message": "A join of the reading thread that gives up after 1 s survives 20 of 20 runs, because the case keeps that thread alive only about 300 ms past the answer and no window-free form of 'has not returned yet' exists."}
]
```
