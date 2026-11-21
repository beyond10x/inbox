---
format: aep.planning-md/3
id: review-result:adversary-backend-test-kit-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:backend-test-kit (8ed00d9): 10 findings, 5 red cases'
relations:
- reviews: story:backend-test-kit
revision: 1
---
10 findings

```
unit: story:backend-test-kit, commit 8ed00d9 (base c582901), worktree inbox-s2-w3-kit plus the uncommitted crates/inbox-backend-testkit/tests/adversary.rs
verdict: NEEDS-CHANGE
cases: executed 15→24, red 5
origin: introduced 10 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (the assigned scratch directory)
needs-coordinator: severity of finding 1 (it could be a blocker for a kit that decides whether JetStream is accepted)
```

## 1. Diff

`git --no-pager diff --stat` is empty: the only change is an untracked file. `git status --short` shows one entry: `?? crates/inbox-backend-testkit/tests/adversary.rs`. It is a test file; no other path was touched.

## 2. Cases added (crates/inbox-backend-testkit/tests/adversary.rs)

Each case was run alone before the suite (`<scratch>/alone.log`, `<scratch>/guards2.log`). All share a helper that panics with `the kit passed <what>` when the kit accepts the backend it is given.

Red: broken backends or claims the kit misses

- `clones_that_read_a_stale_cache_fail` (:146), red. The backend caches what each handle reads. A clone starts with an empty cache, and each handle clears its cache only on its own writes. The case first proves the backend is broken: after one clone deletes a row, the other clone still answers it. The kit then passes it on both ports. Output: `the kit passed a backend whose clones read a stale per-handle cache`
- `a_section_whose_lease_expires_after_300_ms_fails` (:237), red. The section is a lease: a second command gets in once the first has held it 300 ms. The case proves the overlap happens, then the kit passes it. Output: `the kit passed a section whose lease expires after 300 ms`
- `a_section_that_stops_excluding_after_a_panic_fails` (:302), red. After one command panics, the section lets every command in. Output: `the kit passed a section that stops excluding after a panic`
- `the_kit_does_not_hang_on_a_section_that_blocks_after_a_panic` (:336), red. The section never returns from a panicking command. `check_exclusive` does not answer within 10 s. Output: `check_exclusive did not answer within 10 s: it waits without a bound on the command that panics`
- `a_factory_that_answers_one_store_on_every_call_is_refused` (:356), red. The factory answers clones of one store on every call. The doc says the kit panics on a non-empty store, but it checks only the first call. Output: `the kit passed a factory that answers one store on every call`

Green now, red on a mutant the unit's suite misses (each was run against a copy of the tree with that one `broken(` call removed)

- `a_list_that_keeps_the_replaced_snapshot_fails` (:412). `list` replays the replaced row too, as an append-only stream would. Red on the mutant at lib.rs:361.
- `a_get_that_answers_none_fails` (:449). Red on the mutant at lib.rs:338.
- `a_section_held_after_a_command_that_returns_fails` (:491). Red on the mutant at lib.rs:191, with output: `expected a line starting "backend contract broken: Exclusive: a second command waits while one runs", got "... the section is free again after a command that panics (a command over a clone did not run within 2s ...)"`
- `a_delete_that_hides_the_row_from_its_own_handle_only_fails` (:557). The delete is a per-handle tombstone. Red on the mutant at lib.rs:321: `the kit passed a delete that hides the row from its own handle only`.

## 3. Suite run, after the cases existed

Command: `CARGO_INCREMENTAL=0 cargo test -p inbox-backend-testkit --locked --no-fail-fast`

- adversary: 4 passed, 5 failed (the 5 above).
- memory: 3 passed.
- planted: 12 passed.
- Exit 101.

Without `--no-fail-fast`, cargo stopped after the adversary binary and never ran memory or planted. The 15 → 24 count is from the no-fail-fast run.

## 4. Findings

All findings cover 8ed00d9. All are introduced, because the crate does not exist at the base.

1. lib.rs:291, CONFIRMED / NEEDS-CHANGE, warning. "Two clones of one backend read one store" is checked so that no handle ever reads a row a second time after another handle wrote it. A per-handle read cache therefore passes. What reaches it: a JetStream backend with a local cache or mirror; `inbox_core` runs commands through clones, and Expire deletes through one clone what another may have read. Fix: read through the writer before the reader writes, then read through the writer again; also call `list` on each handle after a cross-handle put and delete.
2. lib.rs:182 (OVERLAP at :46), CONFIRMED, warning. The kit holds the first command only as long as it watches, 200 ms. Any section that admits a second command after a longer hold passes. Leases and locks with a TTL are a likely JetStream design. This cannot be closed for every lease length. It is fixable in part by holding the first command for longer than 200 ms, and should be documented as a limit of the kit.
3. lib.rs:123, CONFIRMED / NEEDS-CHANGE, warning. After the panic, the kit checks only that commands run again, not that they still exclude one another. Fix: run `second_waits` on the backend after the panic, on the same handle and on a clone.
4. lib.rs:109, CONFIRMED, note. The panicking command runs on the caller's thread with no time limit, so a section that blocks on its way out of a panic hangs the kit. This contradicts the doc's "fails after a bound of 2 s instead of hanging". Fix: run it on a spawned thread and wait at most BOUND.
5. lib.rs:242, CONFIRMED, note. The factory guard checks only the first `fresh()` call. The second call (:291) is never checked, so the doc's "panics when fresh answers a store that is not empty" is only true of the first call.
6. lib.rs:361, mutant, note. If the "not stored as that" branch is removed, no case in the unit's suite notices. That branch is the only check that catches a list that keeps replaced rows. Guard case written (:412).
7. lib.rs:338, mutant, note. If the `None` branch of `answered_equal` is removed, no case in the unit's suite notices. Guard case written (:449).
8. lib.rs:321, mutant, note. If the "original's get still answers a row its clone deleted" check is removed, no case in the unit's suite notices. Guard case written (:557).
9. lib.rs:191, mutant, note. If this check is removed, a section held after a command that returns fails on the wrong rule. No case in the unit's suite notices. Guard case written (:491).
10. lib.rs:313, mutant, note. If this check is removed, no case notices. It is reached only by a backend whose clone's writes do not reach the original while the original's writes do reach the clone. INFEASIBLE: nothing found that would produce it, so no case was written.

## 5. What held

- 7 of the 17 `broken(` call sites are covered: removing any one of 133, 270, 281, 333, 353 or 383 turns the suite red. Removing 183, 297, 304 or 160 leaves it green, but a sibling check fails the same backend with the same rule.
- lib.rs:117 cannot be reached: `exclusive` cannot return an `R` after the command panicked. lib.rs:176 is reachable only if spawning a thread stalls for 2 s.
- A section that runs the command twice, not at all, or drops its value cannot exist: the command is `FnOnce` and the return type is a generic `R`. A list answering the other port's rows cannot exist either, because the snapshot types differ.
- A list that answers a deleted row only on the second call fails at `every_once` (by reading).
- All 11 planted cases assert the rule text with `starts_with`, and each fails at the check it names, not at an earlier one.
- The unit's 15 cases ran 20 times in a loop: 20 runs, all exit 0, 3 + 12 passed every time.
- Sorted or insertion-order `list`, and slow storage, do not affect the storage checks, which are order-independent and have no timeouts (by reading). A section that takes 50 ms to acquire fits within both bounds.
- Under load, the 200 ms OVERLAP can only cause a false pass and the 2 s BOUND only a false failure. No load test was run.
- A thread left behind by a failed section check holds only its own fresh backend. The hang case's leaked thread did not affect the other cases in the same binary.
- Repository rules hold: no name, e-mail or home path in the diff; no comment in Cargo.toml; `forbid(unsafe_code)`; nothing under generated/, ess/ or docs/; no rule of the specification restated.
- adversary.rs passes clippy `-D warnings` and `cargo fmt --check`.

## 6. Paths written outside the worktree

- `<scratch>/` holds scripts (`alone.sh`, `mutants.sh`, `guards2.sh`, `loop.sh`), logs (`alone.log`, `build.log`, `guards2.log`, `suite.log`, `suite2.log`, `suite3.log`, `loop.txt`, `loop-1..20.log`, `mutants.txt`, `mutants.out`, `mutant-<line>.log`, `mutant-<line>-adv.log`) and `lib.rs.orig`.
- The tree copy with its own target directory has been deleted.
- No permission check denied any call.

## 7. Findings block

```findings
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 291
  category: acceptance
  severity: warning
  origin: introduced
  message: 'the clone check never reads through a handle again after another handle wrote, so a backend with a per-handle read cache passes (tests/adversary.rs clones_that_read_a_stale_cache_fail)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 182
  category: concurrency
  severity: warning
  origin: introduced
  message: 'a section that admits a second command once the first has held it longer than the 200 ms OVERLAP, as a TTL lease does, passes (a_section_whose_lease_expires_after_300_ms_fails)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 123
  category: concurrency
  severity: warning
  origin: introduced
  message: 'after a panic the kit checks only that commands run, not that they still exclude, so a section that stops excluding after a panic passes (a_section_that_stops_excluding_after_a_panic_fails)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 109
  category: contract-drift
  severity: note
  origin: introduced
  message: 'the panicking command runs on the caller thread without a bound, so a section that blocks after a panic hangs the kit despite the documented no-hang claim (the_kit_does_not_hang_on_a_section_that_blocks_after_a_panic)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 242
  category: contract-drift
  severity: note
  origin: introduced
  message: 'the empty-store guard checks only the first fresh() call, so a factory answering one store every call passes against the documented panic (a_factory_that_answers_one_store_on_every_call_is_refused)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 361
  category: mutant
  severity: note
  origin: introduced
  message: 'removing the not-stored-as-that branch leaves the unit suite green; it alone catches a list that keeps replaced rows (guard a_list_that_keeps_the_replaced_snapshot_fails)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 338
  category: mutant
  severity: note
  origin: introduced
  message: 'removing the None branch of answered_equal leaves the unit suite green (guard a_get_that_answers_none_fails)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 321
  category: mutant
  severity: note
  origin: introduced
  message: 'removing the clone-delete check leaves the unit suite green; it alone catches a per-handle tombstone delete (guard a_delete_that_hides_the_row_from_its_own_handle_only_fails)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 191
  category: mutant
  severity: note
  origin: introduced
  message: 'removing the after-release check leaves the unit suite green and a section held after a returning command then fails on the wrong rule (guard a_section_held_after_a_command_that_returns_fails)'
- file: crates/inbox-backend-testkit/src/lib.rs
  line: 313
  category: mutant
  severity: note
  origin: introduced
  message: 'removing the original-sees-clone-put check leaves the suite green; only an asymmetric clone-only write buffer reaches it and nothing found builds one'
```
