---
format: aep.planning-md/3
id: story:no-test-holds-a-wall-clock-time
kind: story
status: implemented
title: No test of the gate fails because the host was slow
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T18:27:27Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T18:27:28Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-22T21:53:15Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":5}}}
---
## Outcome

No test of the gate fails because the host was slow.

On 2026-11-22 the gate failed 2 times in a row on a host with a load of about 80 to 100, each
time in a test that holds a wall-clock time:

- `a_read_note_and_an_expired_unanswered_request_are_in_no_view`
  (`tests/acceptance/tests/checks.rs`): a request lived 2 s, and the fourth of 4 client
  processes read its view after that.
- `an_epochs_file_of_20000_lines_is_read_within_2_seconds`
  (`crates/inbox-cli/tests/adversary_w8_repost.rs`): one run took 2.2 s against 2 s.

Both were changed in the same change that found them: the first makes its claim only for views
read before `expires` and repeats with a longer life, the second holds the fastest of at most
5 runs to the bound. Each still reads the wall clock. 2 more bounds of 10 s are in
`crates/inbox-cli/tests/adversary.rs`.

## Acceptance

`cargo test --workspace` and `task acceptance` pass 5 times of 5 with the host's processors
held busy by another job (state transition: a gate that is red on a loaded host and green on
an idle one is green on both).

## Decisions

- A test of expiry gives the service its clock: `inbox serve` takes a clock file or a test
  flag only a test build has, so no test sleeps until an instant passes.
- A test of a read's cost counts work (lines parsed, comparisons) through a counter the code
  exposes to tests; it holds no time.
- Not part of this story: the runner's 60 s bound per test and the 120 s bound of the step
  (`story:test-step-enforces-its-budget`).

## Decision on the acceptance

The first wording covered `cargo test --workspace`. The implementor's inventory and adversary
pass 1 (review-result:adversary-no-test-holds-a-wall-clock-time-pass-1) showed cases of
`inbox-server`, `inbox-backend-testkit` and `inbox-xtask` that hold a time a slow host breaks;
the request timeout of the listener runs on a monotonic clock no test can move, and changing
that is shipped code of the listener. The coordinator narrowed the acceptance on 2026-11-22:

- No case under `crates/inbox-cli/` and under `tests/acceptance/` fails because the host was
  slow: none asserts an upper bound on a wall-clock time, none depends on a real instant, and a
  bounded wait only ends a hang (10 s or more, or the runner's 60 s).
- `task test` and `task acceptance` pass 5 times of 5 at the host's own load. No job is started
  to hold the processors busy: the folder forbids load tests on this machine.
- The cases of the other crates are story:no-test-of-the-other-crates-holds-a-wall-clock-time
  (a draft).

## Decisions taken while building

- The clock switch is `inbox serve --clock-file <path>`, a hidden flag that exists only in a
  debug build (`cfg(debug_assertions)`). A release build has no such flag. The clock reads
  the RFC 3339 instant the file holds at every reading; a file that is missing, unreadable,
  no regular file, holds no instant or goes back in time ends the service with exit 4.
- The read of an epochs file is held to a count of comparisons through a counter that exists
  only in the test build of `inbox-cli` (`cfg(test)`). A step is each line split, each
  comparison of two message ids as the read gets them (`LineId`) and each read of an id's
  text. A read that compares the texts themselves is not counted: adversary pass 2 showed it
  (review-result:adversary-no-test-holds-a-wall-clock-time-pass-2, finding 1), and the
  coordinator left it, because closing it moves the same hole into a submodule. The
  changelog entry says what is counted.
- A stand-in's thread reports the wait it kept, and its case asserts that text, so a stand-in
  that ignores its 1 s wait fails its case.
- A case that needs a full pipe waits until the writing thread is blocked in `write(2)`.

## Scope

- `crates/inbox-cli/src/clock_file.rs` (new), `main.rs`, `serve.rs`, `outbox.rs`;
  `crates/inbox-cli/Cargo.toml` (`libc`).
- `crates/inbox-cli/tests/`, `tests/acceptance/`.
- `docs/design/service.md`, `CHANGELOG.md`, `AGENTS.md`.

## Measured

- Round 2, on ESS 0.97.3, one run after another at the host's own load (load average 40 to 53
  on 20 CPUs): `task test` 5 times of 5 `701 passed, 5 skipped` (23 to 34 s); `task acceptance`
  5 times of 5, 3 passed.
- Round 3: `cargo nextest run -p inbox-cli --locked`, `155 tests run: 155 passed, 0 skipped`
  (21.8 s).
- Plants and mutants shown red before the fix and green after: a delay of 3 s before the post
  of the due-note case; the earlier quadratic read against the step case; a `step()` that
  counts nothing; `blocked_in_write` that answers true at once; `LineId::eq` without its step;
  a stand-in that widens its 1 s wait to 10 s.
- Adversary pass 1: 10 findings, 8 closed, 2 moved to
  story:no-test-of-the-other-crates-holds-a-wall-clock-time. Pass 2: 3 findings, 2 closed, 1
  left with the wording of the changelog (what the step count counts).
- No job was started to hold the processors busy; the folder forbids load tests on this host.
- 11 new cases: `task test` runs 702 cases where `main` runs 691.
- `task check` on the branch merged with `main`, 2026-11-22T21:45:02Z to 2026-11-22T21:52:59Z, load average 128 to 155
  on 20 CPUs: exit 0, `702 tests run: 702 passed, 5 skipped`, the test step 69 s of
  its bound of 120 s.
