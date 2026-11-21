---
format: aep.planning-md/3
id: review-result:adversary-backend-test-kit-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:backend-test-kit (efe92d7): 5 findings, 4 red cases'
relations:
- reviews: story:backend-test-kit
revision: 1
---
5 findings

```
unit: story:backend-test-kit, efe92d7 (unit 8ed00d9 + correction, base c582901), worktree inbox-s2-w3-kit plus the uncommitted crates/inbox-backend-testkit/tests/adversary2.rs
verdict: NEEDS-CHANGE
cases: executed 25→30, red 4
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the assigned <scratch> = adversary2 directory from the brief)
needs-coordinator: none
```

## 1. Diff

`git --no-pager diff --stat` is empty. `git status --short` shows only `?? crates/inbox-backend-testkit/tests/adversary2.rs`, which is a test file. No other path was touched.

## 2. Cases added (crates/inbox-backend-testkit/tests/adversary2.rs)

Each case was run alone before the suite (`<scratch>/alone.log`, `<scratch>/alone2.log`).

- `clones_that_answer_their_own_earlier_write_fail` (:196). Red. The backend is a per-handle write-through cache: a handle answers its own last put or delete before it asks the store, and a clone copies that cache. The case first proves the backend is broken: A puts a row, its clone B replaces it, and A still answers the old row. Output: `the kit passed a backend whose handles answer their own earlier write after a clone replaced it`
- `clones_made_before_a_panic_still_share_the_section` (:390). Red. On finding its lock poisoned, a handle puts a new lock in its own slot. The case proves that after a panic, a clone made before the panic runs while the original holds the section. Output: `the kit passed a section that clones made before a panic no longer share`
- `a_factory_whose_backend_holds_a_message_is_refused_by_the_inbox_check` (:440). Red. Output: `the kit passed a factory whose backend holds a message`
- `a_section_not_free_on_the_second_factory_call_fails_on_the_documented_rule` (:471). Red. Output: `expected a line starting "backend contract broken: Exclusive: a second command waits while one runs", got "backend contract broken: Exclusive: the section is free again after a command that panics (exclusive did not return within 2s from a command that panicked inside the section)"`
- `clones_whose_list_alone_is_stale_fail` (:314). Green on efe92d7. In this backend `get` is fresh and `list` is a per-handle cache. With the `broken(` at lib.rs:425 removed (in the tree copy), it is red: `the kit passed a backend whose clones list from a stale cache` (`<scratch>/mutant-425-adv2.log`).
- A fifth red candidate, a two-place semaphore shared by clones (`<scratch>/two-places-held.rs`), came out green on its first run and was removed from the file; it is listed under "what held".

## 3. Suite run, after all cases existed

Command: `CARGO_INCREMENTAL=0 cargo test -p inbox-backend-testkit --locked --no-fail-fast`

- adversary: 9 passed, 0 failed
- adversary2: 1 passed, 4 failed (the 4 above)
- memory: 3 passed, 0 failed
- planted: 13 passed, 0 failed

Exit 101 (`<scratch>/suite2.log`). An earlier suite run (`<scratch>/suite.log`) was made after the first 3 red cases existed and before :314 and :471 were added; those two cases were then each run alone before `suite2.log`.

## 4. Findings (all on efe92d7, all introduced: the crate does not exist at c582901)

1. lib.rs:384 `one_store`: CONFIRMED / NEEDS-CHANGE, warning. The clone check reads each row only through the handle that did not write it. A handle that answers its own earlier write after a clone replaced or deleted the row passes on both ports. Case: `clones_that_answer_their_own_earlier_write_fail`. What reaches it: inbox-core today makes a fresh clone per command from a long-lived handle that does not write, so nothing found in the current code. A later backend with a write-through cache whose clones live across commands would break the story's rule "two clones of one backend read one store". Fix: in `one_store`, let the reader put or read row `n` itself before the writer replaces it and deletes it.
2. lib.rs:173 (and :154): CONFIRMED / NEEDS-CHANGE, warning. Every clone in the after-panic stage is made after the panic. A section whose handles stop sharing it with clones made before the panic passes. Case: `clones_made_before_a_panic_still_share_the_section`. What reaches it: the documented use; MemoryBackend's doc says a listener moves a clone to another thread, and those clones exist before any command panics. Fix: take `panicking.clone()` before the panicking command and add it as a waiter of the after-panic `excludes`.
3. lib.rs:425: mutant, note. Removing the `list` half of `reads` leaves all 25 cases green. It is the only check of a clone whose `get` is fresh and whose `list` is stale. Guard case written: `clones_whose_list_alone_is_stale_fail`.
4. lib.rs:323 (docs at :17-18, :65-66, :80-81): CONFIRMED, note. The docs say the factory is refused when its backend "holds a snapshot already". The guard reads only the port under check, so the inbox check accepts a backend that holds a message. Case: `a_factory_whose_backend_holds_a_message_is_refused_by_the_inbox_check`. Harmless to the check itself. The fix is one doc word (a snapshot of that port).
5. lib.rs:100-102 against :144: INFEASIBLE, note. The correction's stated replacement for the guard, "a section that does not let the first command in within 2 s fails `a second command waits while one runs`", is true for the factory's first call only. On the second call the kit prints the freed rule and says that a command panicked, which never ran. Case: `a_section_not_free_on_the_second_factory_call_fails_on_the_documented_rule`. The state was constructed (a factory whose second backend's section is held); nothing found reaches it.

## Pass 1 findings

1. Not closed for every stale handle. Read caches are closed: the pass 1 case is green, and the list-cache variant is caught by :425. By reading the code, a get-cache filled by `list`, and a cache that goes stale only on replacement, are caught at the after-replacement read; a copy-on-write clone is caught at the first put. A write-through cache still passes (finding 1).
2. Closed. `HOLD` is 500 ms, the 300 ms lease case is green, and the limit is documented.
3. Closed for the same handle and for clones made after the panic. Not closed for clones made before it (finding 2).
4. Closed. The hang case answers. A section that returns from the panic after 1.9 s passes (2.9 s in total); after 2.1 s it fails the freed rule, at 2.5 s (`<scratch>/regress.log`). The hang case does not assert the outcome it sends (`is_err`); the :144 mutant survives for that reason, but :163 then fails the same backend with the same rule.
5. Closed for the storage checks. The guard runs on each call. `check_port` calls `fresh` twice, so a factory that misbehaves on the third call only is never called a third time. The doc sentence for `check_exclusive` is imprecise (finding 5).
6. Closed. The :465 mutant is caught by `a_list_that_keeps_the_replaced_snapshot_fails`.
7. Closed. The :442 mutant is caught by `a_get_that_answers_none_fails`.
8. Closed. The :413 mutant is caught by `clones_whose_writes_do_not_reach_the_original_fail`. The tombstone case is also caught by :425.
9. Closed. The :268 mutant is caught by `a_section_held_after_a_command_that_returns_fails`.
10. Closed. `planted::clones_whose_writes_do_not_reach_the_original_fail` fails on the :413 mutant.

## What held

- Shared hold. A section with two places shared by clones lets both waiters in one after the other, and the kit prints `a second command waits while one runs`. Exactly one waiter gets in only for a per-handle section, and then the kit prints the right rule.
- Correct backends pass all 3 checks, run in the tree copy (`<scratch>/regress.rs`, `<scratch>/regress.log`): a channel to one owner thread (exclusive check 1.0 s); an insertion-order list (1.0 s); a list sorted by identity, descending (1.0 s); 50 ms per storage call and per section entry (storage checks 1.95 s each, exclusive check 1.25 s).
- Which way each bound fails. `HOLD` fails toward a false pass: a non-excluding section with more than 500 ms of entry latency passes. `BOUND` fails toward a false failure: a correct section that takes 2.1 s to unwind fails.
- Flakiness. 20 loop runs of memory + planted + adversary: all exit 0, 9 + 3 + 13 passed every time, 0 "panicked at" lines.
- Spawned threads. A passing memory run with `--nocapture` prints no panic line. A thread left behind by a failed check holds only its own `fresh()` backend.
- Mutants of the 16 `broken(` sites. Caught: 163, 245, 268, 354, 365, 413, 437, 442, 457, 465, 487. Survivors other than 425: 139 and 227 are unreachable; for 144, :163 fails the same backend with the same rule; for 205, :268 fails the same backend with the same rule.
- Not built. A section that stops excluding only for the 2nd command after the panic is unseen, because that command runs alone. A section that stops excluding only for post-panic clones is caught (by reading the code).
- `tests/adversary.rs` is byte-identical to the coordinator's pass 1 copy.
- Repository rules hold. No name, e-mail or home path in the diff. No comment in the manifest. `forbid(unsafe_code)`. Nothing under generated/, ess/ or docs/.
- adversary2.rs passes clippy `--tests -D warnings` and rustfmt.

## Paths written outside the worktree

- `<scratch>/` holds scripts (`loop.sh`, `mutants.sh`, `mutate.py`, `mutant425.sh`, `regress.rs`, `two-places-held.rs`), logs (`alone.log`, `alone2.log`, `suite.log`, `suite2.log`, `regress.log`, `clippy.log`, `memory-nocapture.log`, `loop.txt`, `loop-1..20.log`, `mutants.txt`, `mutant-<line>.log`, `mutant-425-adv2.log`) and `lib.rs.orig`.
- The tree copy with its target directory has been deleted.
- No permission check denied any call.

```findings
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 384
  category: acceptance
  severity: warning
  origin: introduced
  message: 'the clone check reads each row only through the handle that did not write it, so a per-handle write-through cache that answers its own earlier write after a clone replaced it passes (adversary2.rs clones_that_answer_their_own_earlier_write_fail)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 173
  category: concurrency
  severity: warning
  origin: introduced
  message: 'every clone of the after-panic stage is made after the panic, so a section whose clones made before a panic stop sharing it passes (adversary2.rs clones_made_before_a_panic_still_share_the_section)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 425
  category: mutant
  severity: note
  origin: introduced
  message: 'removing the list half of the clone check leaves all 25 cases green; guard case adversary2.rs clones_whose_list_alone_is_stale_fail is red on that mutant'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 323
  category: contract-drift
  severity: note
  origin: introduced
  message: 'the docs refuse a factory whose backend holds a snapshot, but the guard reads only the checked port, so the inbox check takes a backend holding a message (adversary2.rs a_factory_whose_backend_holds_a_message_is_refused_by_the_inbox_check)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 100
  category: contract-drift
  severity: note
  origin: introduced
  message: 'the check_exclusive doc says a section not free within 2 s fails the waits rule, but on the second factory call it fails the freed rule claiming a command panicked (adversary2.rs a_section_not_free_on_the_second_factory_call_fails_on_the_documented_rule)'
```
