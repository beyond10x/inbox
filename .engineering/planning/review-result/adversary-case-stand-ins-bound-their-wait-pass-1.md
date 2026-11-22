---
format: aep.planning-md/3
id: review-result:adversary-case-stand-ins-bound-their-wait-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the bounded stand-ins: 9 mutants, 5 survived, 7 findings'
relations:
- reviews: story:case-stand-ins-bound-their-wait
revision: 1
---
unit: story:case-stand-ins-bound-their-wait, the uncommitted working tree on branch test/stand-ins-bound-their-wait at `<tree>` (base 7cd7416)
verdict: CONFIRMED (notes and warnings only, no red case)
cases: executed 79→79, red 0 (touched modules only; I added no case)
origin: introduced 5 / pre-existing 2 / undecided 0
wrote-outside-worktree: 1 directory (the assigned scratch directory, see part 6)
needs-coordinator: whether the conformance half and the 7th place need cases that go through a real case's stand-in (findings 1 and 3)

I wrote no failing case. Every stand-in is a private fn in its own case module, so a new test file cannot call it, and the rules forbid editing those files. I ran the mutants on a copy of the tree in scratch instead of the tree itself. That follows the adversary procedure, which is stricter than the brief's mutate-and-restore. The four findings that hold are all about tests that cannot fail or claims nothing checks. None is a broken bound.

1. git diff --stat of `<tree>`: byte-identical before and after my pass (11 files, +240/-28, all the implementor's and the coordinator's: the 6 case files, adversary_control_sequences.rs, support/mod.rs, conformance main.rs and wire.rs, docs/checks.md, the story). `git status --short` shows the same 13 entries. I wrote no tracked or untracked file in the tree.

2. Cases I added: none. Reasons above.

3. Runs
- Case lists, in the tree: `ESS_TOOLCHAIN=0.97.3 cargo nextest list -p inbox-cli -p inbox-conformance --locked`. inbox-cli: 26 in bin/inbox and 90 in main, 116 in all. inbox-conformance: 6 + 83 = 89 listed; `--run-ignored all` gives 90 and `--run-ignored only` gives 1 (adversary::ess_contains_null_at_the_top_level_needs_the_member_published). docs/checks.md says 116 and "89 cases, 1 ignored". Both are correct.
- Touched modules, in the tree: `cargo nextest run -p inbox-cli -p inbox-conformance --locked --no-fail-fast -E 'test(/^(adversary|adversary2|adversary_w8_repost|adversary_w8_repost2|adversary_control_sequences|wire|adversary3|support)::/)'`. Result: "Summary [17.408s] 79 tests run: 79 passed, 127 skipped", exit 0, 21 s wall, load average about 125 on 20 CPUs.
- Times of the 7 new cases (no-client wait is 1 s):
  - adversary::a_stand_in_whose_client_never_connects_ends_and_names_the_request: 1.029 s
  - adversary2::a_relay_whose_client_never_connects_ends_and_names_the_request: 1.292 s
  - adversary_w8_repost::a_stand_in_whose_client_never_connects_fails_the_case_and_names_the_request: 1.037 s
  - adversary_w8_repost2::a_proxy_whose_client_never_connects_fails_the_case_and_names_the_request: 1.076 s
  - adversary3::a_stand_in_whose_client_never_connects_ends_and_names_the_request: 1.039 s
  - wire::a_stand_in_whose_client_never_connects_ends_and_names_the_request: 1.068 s
  - support::a_wait_for_a_connection_that_never_comes_ends_and_names_it: inside the same run, under 2 s.
- Load margin (attack 5): the slowest case that uses one of the bounded stand-ins took 4.215 s in total (adversary_w8_repost::a_transport_failure_of_the_epoch_question...). post_keeps_the_envelope_before_the_request_reaches_the_service took 0.233 s, a_400_for_a_request_whose_bytes_stalled 2.911 s, the repost2 cases 2.2 to 3.0 s. The time to the first connection is always below the case's total, so 10 s leaves at least twice the margin at this load. The only case over 5 s is adversary::add_key_processes_at_once_lose_no_entry at 10.938 s. It is pre-existing and uses no stand-in.
- Mutants, on a copy of the tree in scratch, each run alone through nextest with its filter, then restored in the copy:
  - M1 adversary.rs:79, bounded accept made blocking: caught. The new case hit TIMEOUT at 60.041 s.
  - M2 adversary2.rs:200, same change: caught, TIMEOUT at 60.164 s.
  - M3 adversary_w8_repost.rs:96, accept_within replaced by a blocking accept: survived. The new case passed in 2.029 s.
  - M4 adversary_w8_repost2.rs:108, same change: survived. The new case passed in 1.077 s.
  - M5 wire.rs:228, blocking accept: caught, TIMEOUT at 60.011 s.
  - M6 adversary3.rs:69, blocking accept: caught, TIMEOUT at 60.016 s.
  - M7 adversary_control_sequences.rs:320, blocking accept: survived. 13 cases ran, 13 passed; no case covers it.
  - M8 inbox-cli support STAND_IN_WAIT 10 s changed to 3600 s: survived (6 of 6 passed).
  - M9 conformance support STAND_IN_WAIT 10 s changed to 3600 s: survived (6 of 6 passed).
  - My first versions of M3 and M4 did not compile (a let-else with a closing brace before else). I reran them in a form that compiles; the results above are from that rerun.

4. Findings
- F1 (warning, CONFIRMED, introduced). The acceptance is not met as written at the two conformance places. `crates/inbox-conformance/tests/wire.rs:203` and `adversary3.rs:52`: `stand_in` drops the JoinHandle of `serve`. A real case whose client never connects therefore never sees the message: it is only eprintln'd from a detached thread 10 s later, after the case has ended on its own assertion about the result. Before the change no case waited on these threads either, so the story's claim "today only the runner's kill at 60 s ends it" was false for these two places, and the change alters no case's outcome. The new cases call `serve` directly and join it, so they prove the thread, not a case. What reaches it: every caller of these stand_in fns, e.g. wire.rs:311, 322 and 372, and adversary3.rs:129.
- F2 (note, CONFIRMED, introduced). M3 and M4 survived. The new cases at `adversary_w8_repost.rs:370` and `adversary_w8_repost2.rs:330` pass even when the stand-in's or proxy's accept blocks for ever. The message comes from `requests`/`requests_within` (`recv_timeout`), not from the thread's bound. The doc comments on both cases say "its thread ends after its wait", and nothing checks it. The thread is detached, so its bound carries no weight. The doc comments could drop that claim, or the bound could be removed as dead code.
- F3 (note, CONFIRMED, introduced). M7 survived. The 7th place, `adversary_control_sequences.rs:320` (`raw_stand_in`), has no case whose client never connects. Its bound is unproven, and blocking it leaves every case green.
- F4 (note, CONFIRMED, introduced). M8 and M9 survived. Every new case passes 1 s to a `*_within` form, so the 10 s that the real stand-ins use (inbox-cli `support/mod.rs:429`, conformance `support.rs:9`) is exercised by no case. The acceptance's "within 15 s" is proven only for the 1 s variants; a wrapper that waited an hour stays green.
- F5 (note, CONFIRMED, introduced). The message at `adversary.rs:79` and `adversary2.rs:200` is the same text, "the stand-in received no connection for the request of the case". It does not name the relay as a relay and does not say which request. Only the panic location tells the two apart. The repost files name "request N of M", and the shared stand-in names "answer N of M".
- F6 (note, CONFIRMED, pre-existing). `adversary.rs:228`, a_redirect_answer_is_not_reported_as_posted, never joins `_seen`. The stand-in's panic, now with the bounded message, is swallowed, and the case stays green when no request reaches it. This is the same at the base.
- F7 (note, CONFIRMED, pre-existing). `adversary.rs:495`, add_key_processes_at_once_lose_no_entry, took 10.938 s. That is over the 5 s naming bound. It is outside the unit.

5. Attacked and could not break
- Unbounded waits in stand-ins: every TcpListener in crates/*/tests now goes through a bounded accept, and every stream read in the unit's stand-ins has a read timeout (5, 10 or 20 s). The other recv() and join() calls (backend-testkit adversary2.rs:371, server adversary_deadline.rs:110) end when the sender drops; they are not stand-ins.
- Non-blocking state leaking into a stream: both accept_within helpers set the accepted stream back to blocking.
- A case that relied on a wait over 10 s: none. The slowest stand-in case took 4.2 s at load 125. repost2's per-line wait went from 15 s to 10 s, and its forwards finish quickly because client and server both send Connection: close.
- docs/checks.md counts: 116 and 89 (1 ignored) are correct.
- M1, M2, M5 and M6 were caught: those four cases do test the real stand-in body.

6. Paths written outside the worktree, all under the assigned scratch directory `<scratch>/d30/standins/adversary/`:
- `copy/`: a copy of the tree with its own `copy/target` build directory, the largest item.
- `orig/`: the unmutated files of the copy.
- Scripts: `mutants.sh`, `mutants2.sh`, `suite.sh`.
- Logs: `mutants.log`, `mutants2.log`, `mutants.stdout`, `mutants2.stdout`, `mutant-*.out`, `suite.out`, `list.txt`, `list.err`, `diffstat-before.txt`, `diffstat-after.txt`.

```findings
[
  {"file": "crates/inbox-conformance/tests/wire.rs", "line": 203, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The conformance stand_in (here and adversary3.rs:52) drops the JoinHandle, so no real case whose client never connects ever fails with the stand-in's message, and the bound changes no case's outcome because nothing ever waited on these detached threads."},
  {"file": "crates/inbox-cli/tests/adversary_w8_repost.rs", "line": 370, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "This case and adversary_w8_repost2.rs:330 stay green when the stand-in or proxy accept blocks for ever (mutants M3 and M4 survived), so their claim that the thread ends after its wait is unchecked."},
  {"file": "crates/inbox-cli/tests/adversary_control_sequences.rs", "line": 320, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "raw_stand_in's bounded accept has no case whose client never connects, and making it blocking leaves all cases green (mutant M7 survived)."},
  {"file": "crates/inbox-cli/tests/support/mod.rs", "line": 429, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "STAND_IN_WAIT here and in inbox-conformance tests/support.rs:9 can be 3600 s with every case green (mutants M8 and M9 survived), so the bound the real stand-ins use is proven by no case."},
  {"file": "crates/inbox-cli/tests/adversary2.rs", "line": 200, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The relay and the adversary.rs:79 stand-in panic with the same text, 'the stand-in received no connection for the request of the case', which names neither the relay nor which request."},
  {"file": "crates/inbox-cli/tests/adversary.rs", "line": 228, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "a_redirect_answer_is_not_reported_as_posted never joins its stand-in, so the stand-in's panic is swallowed and the case stays green when no request reaches it."},
  {"file": "crates/inbox-cli/tests/adversary.rs", "line": 495, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "add_key_processes_at_once_lose_no_entry took 10.938 s, over the 5 s naming bound."}
]
```
