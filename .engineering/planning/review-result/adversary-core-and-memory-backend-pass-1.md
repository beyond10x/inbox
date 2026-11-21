---
format: aep.planning-md/3
id: review-result:adversary-core-and-memory-backend-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:core-and-memory-backend at ff76d83
relations:
- reviews: story:core-and-memory-backend
revision: 1
---
3 findings (1 red now, 2 found by mutants: the cases are green on the unit and red on the mutated copies).

unit: story:core-and-memory-backend, commit ff76d83 (on f13624f) plus the uncommitted crates/inbox-core/tests/adversary.rs
verdict: NEEDS-CHANGE
cases: executed 33→40, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 15 paths, all in the assigned scratch directory (listed in part 6)
needs-coordinator: whether finding 1 is a blocker here or goes to story:http-service.

## 1. Diff
`git diff --stat` is empty because the file is untracked. `git status --short`:
```
?? crates/inbox-core/tests/adversary.rs
```
This is the only path touched in the worktree, and it is a test file.

## 2. Cases added (crates/inbox-core/tests/adversary.rs, its own test target)
- `concurrent_posts_with_one_dedup_key_store_one_message`: **red.** Two systems over clones of one `MemoryBackend` post two ids with one dedup key at the same time. The backend wrapper delegates every call to the shared `MemoryBackend`; a barrier inside `list` only fixes one schedule the OS is free to pick. Expected: 1 row, answers `duplicate-key` and `posted`. Red output from the first run of this target alone:
  ```
  panicked at crates/inbox-core/tests/adversary.rs:180:5:
  assertion `left == right` failed: rows with the key: 2, answers: ["posted", "posted"]
    left: (2, ["posted", "posted"])
   right: (1, ["duplicate-key", "posted"])
  ```
- `concurrent_posts_with_one_dedup_key_store_one_message_under_the_scheduler`: **red.** The same race with no barrier in the backend: 8 threads per round, each with a plain `MemoryBackend` clone. First run:
  ```
  panicked at crates/inbox-core/tests/adversary.rs:214:9:
  round 1: 2 posts answered posted, 2 rows carry the one dedup key
  ```
- `posted_stores_every_envelope_field_as_sent`: green. Reads the stored row back and checks every field `sets:` names.
- `unknown_inbox_answers_before_duplicate_key`, `duplicate_answers_before_unknown_inbox`, `duplicate_key_answers_before_inbox_closed`, `duplicate_answers_before_inbox_closed`: green. Each pins one interpreter-measured pair (part 5).

## 3. Suite, run after the cases existed
`CARGO_INCREMENTAL=0 cargo test -p inbox-core --locked --no-fail-fast`, exit 101:
```
Running tests/adversary.rs
test concurrent_posts_with_one_dedup_key_store_one_message ... FAILED
test concurrent_posts_with_one_dedup_key_store_one_message_under_the_scheduler ... FAILED
test result: FAILED. 5 passed; 2 failed
Running tests/core/main.rs
test result: ok. 33 passed; 0 failed
```
(The under-the-scheduler case failed in round 2 on this run, with 4 rows.)

## 4. Findings

**F1: the dedup refusals do not hold when two requests run at once.**
- Verdict NEEDS-CHANGE, origin introduced. Locations: `crates/inbox-core/src/obligations.rs:182` (the read) and `:222` (the write).
- **What breaks:** `PostMessage` reads `get`, `get` and `list`, then calls `put`, and each call takes the `MemoryBackend` lock on its own. Two concurrent posts with one dedup key both answer `posted` and both rows are stored. Two posts with the same `message_id` race the same way: both answer `posted` and the second overwrites the first.
- **What reaches it:** the embedding the unit documents. `src/lib.rs:41` says to build one system per request over a clone of the backend, and the story says the listener of story:http-service moves a clone to another thread. A sender's retry while its first post is still in flight is the repeat dedup exists for. The generated behaviours (`Answer`, `OpenInbox`) have the same read-then-write shape, so the same hazard reaches them through this backend too.
- **Fix to name:** a critical section per command. Either the embedding holds one lock around `handle`, or the backend offers an exclusive section that `Core` uses. Also correct the `system` doc.

**F2: the suite never reads 5 of the stored envelope fields.**
- Verdict CONFIRMED, origin introduced, warning. Location: `crates/inbox-core/src/obligations.rs:201`.
- **What breaks:** a scratch copy mutated so the `posted` row stores `on_behalf_of` taken from `to`, `authority` as None, `created` taken from `expires`, an empty `body` and an empty `signature`. All 33 existing cases still passed. `posted_stores_every_envelope_field_as_sent` failed on that copy (adversary.rs:290, left `agent:box/worker`).
- **What reaches it:** every client reads these fields back through the views.

**F3: the order among the 4 row refusals is correct but no existing case pins it.**
- Verdict CONFIRMED, origin introduced, note. Location: `crates/inbox-core/src/obligations.rs:182`.
- **What breaks:** three reorderings each left all 33 existing cases green:
  - duplicate-key before unknown-inbox (the order the contract text in `PLAN.md` gives)
  - inbox-closed before duplicate-key
  - unknown-inbox before duplicate
- Each was caught by one of the adversary's order cases: adversary.rs:232, :251 and :241.

## 5. What held
- **Row-refusal order on the ESS interpreted target**, measured pair by pair: duplicate before unknown-inbox, unknown-inbox before duplicate-key, duplicate-key before inbox-closed, duplicate before inbox-closed. The unit matches all four.
- **Input refusals before row refusals:** already-expired before unknown-inbox and before duplicate, no-authority before duplicate, decision-without-options before duplicate-key. Synthesis refused the reverse expectation of each with ESS-AUTHOR-041.
- **Dedup scope:** a dedup key held in another inbox, or posted by another sender, is `duplicate-key` on the interpreter, as in the unit.
- **Expire:** `now == expires` is expired, and a `+02:00` offset is compared as an instant (not-due); both match the interpreter. With a leap second (`23:59:60Z`) in `now` or `expires`, the interpreter reports unsupported and the unit returns an unmet obligation, which is the same reading the generated crate gives.
- **Other mutants**, all caught by the existing suite: `Le` changed to `Lt` in already-expired, the `open` check dropped in Expire, `keep_until`, `answered_by` or `decided_option` dropped, `<=` changed to `<` for `lines(n)`, the decision-without-option check, and authority-invalid.
- **Repository rules:** no name, e-mail or home path in the diff; no comment in a manifest; `forbid(unsafe_code)` is set; nothing edited under `generated/`.
- **Not testable:** the "poisoned lock is recovered" claim. No outside code can panic while holding the guard, and no path was found that does.

## 6. Paths written outside the worktree
All under `<scratch>/w2/core-and-memory-backend/adversary/`:
- scripts: `make_case.py`, `run_cases.sh`, `run_cases2.sh`, `mutate.py`, `run_mutants.sh`
- logs: `cases.log`, `cases2.log`, `adv1.log`, `adv2.log`, `suite.log`, `mutants.log`, `mutant-fields.log`, `mutant-dedup-before-inbox.log`, `mutant-closed-before-dedup.log`, `mutant-inbox-before-duplicate.log`

The tree copies (`cases/` and `mut/`) are deleted. No permission check denied a tool call.

## 7. Findings block
```findings
- file: crates/inbox-core/src/obligations.rs
  line: 182
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'PostMessage reads then writes through separate MemoryBackend locks, so two concurrent posts over clones of one backend (the embedding lib.rs documents) both answer posted and store two rows with one dedup key or overwrite one id'
- file: crates/inbox-core/src/obligations.rs
  line: 201
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the posted row can store a wrong on_behalf_of, authority, created, body and signature with all 33 existing cases green; only adversary.rs posted_stores_every_envelope_field_as_sent catches it'
- file: crates/inbox-core/src/obligations.rs
  line: 182
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the order among duplicate, unknown-inbox, duplicate-key and inbox-closed matches the interpreted target but no existing case pins it; three reorderings survive the suite'
```
