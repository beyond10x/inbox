---
format: aep.planning-md/3
id: review-result:adversary-core-and-memory-backend-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:core-and-memory-backend: 2 findings'
relations:
- reviews: story:core-and-memory-backend
revision: 1
---
2 findings (both notes; 1 red case now, 1 shown by a mutant). Neither is a regression of something that held at ff76d83.

unit: story:core-and-memory-backend at 32b954f (on ff76d83, base f13624f), plus the uncommitted crates/inbox-core/tests/adversary2.rs
verdict: INFEASIBLE (strongest returned: one CONFIRMED note, one INFEASIBLE note)
cases: executed 44→48, red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 13 paths, all in the assigned scratch directory adversary2/ (listed in part 6)
needs-coordinator: none

## 1. Diff

`git status --short`:

```
?? crates/inbox-core/tests/adversary2.rs
```

That is the only path touched in the worktree, and it is a test file (a test target of its own). Nothing is committed.

## 2. Cases added (crates/inbox-core/tests/adversary2.rs)

- `a_context_that_calls_the_entry_again_returns`: **red.** A `PostMessage` whose `PostExternals::unknown_option` reads the `Inboxes` view through `inbox_core::handle` over the same store. It runs on a thread and is given 5 s. Output from the first run of this target alone:
  ```
  thread 'a_context_that_calls_the_entry_again_returns' panicked at crates/inbox-core/tests/adversary2.rs:327:5:
  a post whose context reads a view through the entry did not return within 5s: Err(Timeout)
  test result: FAILED. 3 passed; 1 failed; ... finished in 5.00s
  ```
- `a_clone_waits_while_another_clone_holds_the_section`: green. One clone holds the section, and a request through another clone is still waiting after 300 ms. After the release it answers `opened`. This catches the per-clone mutant (finding 2).
- `a_store_of_its_own_does_not_wait_for_another_stores_section`: green. Two `MemoryBackend::new()` do not share a section.
- `an_answer_that_panics_leaves_the_message_open_and_the_next_answer_settles_it`: green. The `shape-mismatch` answer panics inside the section. Afterwards the row is still `Sent` with no reply, and the next `Answer` through the entry answers `answered` within 5 s.

## 3. Suite, run after the cases existed

`CARGO_INCREMENTAL=0 cargo test -p inbox-core --locked --no-fail-fast`, exit 101:

```
Running tests/adversary.rs    test result: ok. 7 passed; 0 failed; finished in 2.00s
Running tests/adversary2.rs   test a_context_that_calls_the_entry_again_returns ... FAILED
                              test result: FAILED. 3 passed; 1 failed; finished in 5.00s
Running tests/core/main.rs    test result: ok. 37 passed; 0 failed; finished in 0.21s
```

Wall time per target: tests/core 0.21 s, tests/adversary.rs 2.00 s, tests/adversary2.rs 5.0 s (the 5 s bound of the red case). The adversary.rs time is the 2 s `PATIENCE` the first thread always waits inside the section. It is a fixed cost, not flakiness.

## 4. Findings

**F1: a command whose context calls the entry again deadlocks.**

- Verdict INFEASIBLE, origin introduced (by 32b954f), note.
- Locations: `crates/inbox-core/src/lib.rs:100` and `crates/inbox-core/src/backend.rs:47`.
- **What was measured:** `handle_with_events` takes the section, a non-reentrant `std::sync::Mutex`. A context or verifier that calls `inbox_core::handle` on the same store from inside a command locks it a second time on the same thread. The std docs leave the result unspecified ("might panic or deadlock"); here it deadlocked. Case: adversary2.rs:328.
  - Neither the `Exclusive` doc nor the `handle` doc says a context or verifier must not call the entry. The `Exclusive` doc only covers the command's own storage calls.
- **What reaches it:** nothing found. The real verifier (story:signature-verifier) cannot depend on inbox-core, and `ServiceContext` only calls `MessageStorage::get`.
- **Regression?** No. At ff76d83 there was no entry; `system` plus the generated `handle` still re-enters fine.
- **Fix to name:** one sentence in the `handle` / `Exclusive` docs: the context, the verifier and `PostExternals` must not call the entry over the same store.

**F2: the section cases in tests/core/section.rs do not test that clones share the section.**

- Verdict CONFIRMED, origin introduced (by 32b954f), note.
- Location: `crates/inbox-core/tests/core/section.rs:24`.
- **What was measured:** in a scratch copy, `MemoryBackend` got a hand-written `Clone` that shares the store but makes a new section per clone. Result:
  - tests/core: 37 passed, 0 failed (section.rs included).
  - The mutant was caught only by tests/adversary.rs (both concurrency cases) and by adversary2's clone case.
  - Cause: `at_once` hands every thread the same `World` by reference, so all threads lock one `MemoryBackend` value. The cases test the mutex, not the documented claim "a lock its clones share".
- **What reaches it:** the embedding the story names, a listener that moves a clone to another thread. The full suite does catch the mutant today through tests/adversary.rs, so this is test quality, not a live defect.

## 5. What held

- **"Each command writes at most one row":** holds. In `generated/rust/inbox/src/behaviour.rs`, Answer, CloseInbox, Collect, MarkDelivered, MarkRead, OpenInbox and Refuse each reach exactly one `put` or `delete` per path. `PostMessage` and `Expire` (obligations.rs) do the same. The generated `run_*`, `pump` and `System` write no storage. A panic between two writes cannot happen.
- **Panic inside the section:** after an `Answer` panics mid-command, the store has no half-written row, the section is free and the next answer settles the message (adversary2 case, green).
- **Section scope:** clones share it; two `new()` stores do not (adversary2, green). `Default`/`Clone` give no way to share the store without the section.
- **Lock order:** section, then rows. The row lock is never held across outside code, so no other deadlock path was found.
- **Changed two-thread case in tests/adversary.rs:**
  - Section present: 20 of 20 runs green, 2.05–2.64 s each.
  - Section removed in a copy: 20 of 20 runs red with "rows with the key: 2".
  - So the 2 s wait did not turn it into a case that passes either way.
- **Mutants caught by section.rs** (and by adversary.rs):
  - no section at all
  - section only around views
  - section released before the command runs (system built inside the section, run outside it)
  - poisoned section not recovered (also caught by adversary2's panic case)
- **Regressions vs ff76d83:** none found. precedence.rs's `Counting` and support.rs now go through `inbox_core::handle`, which adds no read before the external branches. The acceptance (timeline through the generated `handle`) still runs, wrapped.
- **Repository rules:** no name, e-mail or home path in 32b954f; no manifest touched; `forbid(unsafe_code)` kept; nothing under `generated/` edited.
- **Not run:** the full gate `slot task check`. Only `cargo test -p inbox-core` and `rustfmt --check` on the new file were run.
- No permission check denied a tool call.

## 6. Paths written outside the worktree

All under `<scratch>/w2/core-and-memory-backend/adversary2/`:

- scripts: `mutate.py`, `run_mutants.sh`, `suite.sh`, `repeat_mutant.sh`
- logs: `case.log`, `suite.log`, `repeat.log`, `repeat-no-section.log`, `run_mutants.out`, `mutant-no-section.log`, `mutant-per-clone.log`, `mutant-views-only.log`, `mutant-released-before-run.log`, `mutant-poison-kept.log`
- markers: `mutants.done`, `suite.done`, `repeat-mutant.done`

The tree copy `mut/` (including its `target/`) is deleted.

## 7. Findings block

```findings
- file: crates/inbox-core/src/lib.rs
  line: 100
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'handle_with_events holds a non-reentrant section while the context runs, so a context, verifier or PostExternals that calls inbox_core::handle over the same store deadlocks (adversary2.rs:328), and neither the handle nor the Exclusive doc forbids it; nothing in the plan reaches it'
- file: crates/inbox-core/tests/core/section.rs
  line: 24
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the section cases share one MemoryBackend value by reference across threads, so a section made per clone leaves all 37 tests/core cases green; only tests/adversary.rs and adversary2.rs catch it'
```
