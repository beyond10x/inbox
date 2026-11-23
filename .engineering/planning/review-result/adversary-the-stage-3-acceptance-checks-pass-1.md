---
format: aep.planning-md/3
id: review-result:adversary-the-stage-3-acceptance-checks-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the acceptance checks of stage 3: 2 mutants survive, 5 notes'
relations:
- reviews: story:the-stage-3-acceptance-checks
revision: 1
---
unit: story:the-stage-3-acceptance-checks, commit 6dea02d of inbox-s3-acc. The tree has since moved to cf0ee35, which changes no file of tests/acceptance or Taskfile.yml.
verdict: NEEDS-CHANGE
cases: executed 6→6 (`task acceptance`; my module is outside its `^checks::` filter), package run with all ignored cases 10, red 1
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s3/acc/adversary/ (listed in part 6)
needs-coordinator: whether to move `adversary_checks.rs` into the tree (the port case in it is red on purpose); the `task test` row of AGENTS.md (finding 7)

**1. Diff stat.** The tree is untouched (`git status --short` is empty). The copy differs from 6dea02d in 2 files, both test files:
- tests/acceptance/tests/adversary_checks.rs: +163, new
- tests/acceptance/tests/main.rs: +1 (`mod adversary_checks;`)

All product and check mutants were reverted. `cmp` against `git show HEAD:` was equal for checks.rs, wake.rs, tls.rs and lib.rs.

**2. Cases added**, in <scratch>/s3/acc/adversary/copy/tests/acceptance/tests/adversary_checks.rs. All 3 need `#[ignore]` because they need `INBOX_BIN`. All are under 1 s.

- `a_client_given_the_pin_of_another_certificate_posts_nothing`: alice posts with the pin of a second certificate made by `inbox tls new`. It asserts exit 3 and that bob's `Pending` is empty.
  - Green on the product, alone: 0.42 s.
  - Red on mutant M1, run with the others: `a post to a service whose certificate is not the pinned one ends with 3; stdout: {"message_id":…,"answer":{"outcome":"posted",…  left: Some(0)  right: Some(3)`
- `a_wake_answers_its_post_on_the_read_it_held`: check 2's setup and fact, plus one assertion after the wake ends: it made 3 connections (the read without a wait, the held read, the MarkDelivered).
  - Green on the product, alone: 0.59 s.
  - Red on mutant M2: `the wake read without a wait, held one read until the post and marked it: [Carried {.. service_ended: true }, Carried {..}, Carried {..}, Carried {..}]  left: 4  right: 3`
- `a_restart_survives_a_socket_that_takes_the_port_in_between`: a thread binds the service's port as soon as it is free, during `hosts.restart()`.
  - Red on the product, alone, 0.45 s: `inbox serve printed no first line; its standard error: inbox: --listen 127.0.0.1:42721: Address already in use (os error 98)`, then `the restart failed while another socket held its port`.
  - It is red by construction and stays red until the fixture changes. Do not put it under the `^checks::` filter.

Order note: the first 2 cases were run alone on the product first (green). Their reds came from the mutant runs, where they ran together with the 3 checks, not alone.

**3. Suite runs**, after the cases existed.
- `slot --timeout 20m task acceptance` in the copy: exit 0 after 5 s. `Summary [1.551s] 6 tests run: 6 passed, 4 skipped`, then `task acceptance: 6 checks passed`.
- The package with all ignored cases (`--run-ignored only -E 'all()'`): exit 100. `10 tests run: 9 passed, 1 failed`; the failure is the port case.
- Repeats on the product: the 3 checks together 20 times, 0 red. Each alone 5 times, 0 red. Slowest single check 3.2 s.
- clippy `-p inbox-acceptance --all-targets -D warnings` exit 0; `rustfmt --check` exit 0.

**4. Findings** (all against 6dea02d)

1. tests/acceptance/src/lib.rs:412. Note, CONFIRMED, introduced.
   - Measured: the doc says the stage-3 checks prove "the TLS certificate is checked by each against the pin it was given", but every client gets the right pin. M1 (tls.rs:355, accept any certificate) leaves all 3 checks green.
   - What reaches it: any change to the pin verifier. `task test` still catches it in crates/inbox-cli/tests/tls.rs:67 (`a_client_with_the_pin_posts_over_tls_and_one_with_another_pin_exits_3`), going by that test's name; I did not run it.
   - Fix: add the wrong-pin case to check 1, or drop the sentence.
2. tests/acceptance/tests/checks.rs:373. Warning, NEEDS-CHANGE, introduced.
   - Measured: the fact also holds for a wake that does not wait. It is true for a moment on any connection, between the client's handshake bytes and the service's answer. M2 (wake.rs:206, read with wait=0 every 300 ms) kept check 2 green in 6 of 6 runs. AGENTS.md says the wake is "seen holding its read", which M2 makes false.
   - Fix in check 2: after the wake ends, assert that it made 3 connections (my case), or that connection 2 was ended after the post.
3. tests/acceptance/tests/checks.rs:377. Note, CONFIRMED, introduced.
   - Measured, by reading: `from_client_after_service > 0` already holds once the client's TLS Finished is carried, before its request. So the case where alice's post reaches the service before the held read's request is only unlikely, not excluded. The product still exits with the post in that case.
   - Fix: also require `since[1].from_client_after_service >= since[0].from_client_after_service`. Both connections carry a request of about the same size: 455 bytes each on the first 3 connections in the M2 run.
4. tests/acceptance/tests/checks.rs:381. Note, NEEDS-CHANGE, introduced.
   - Measured, by reading: `assert_eq!` of 2 environment maps prints both on failure. The right side holds the run's pin and paths. The left side is the wake's whole `/proc` environment, so if `env_clear` is ever dropped (the implementor's plant C), it is the test process's environment.
   - What reaches it: CI runs `task check`. That step's env holds no secret and the job has `contents: read`, but the log is public, so runner variables would be printed. Locally the same lands in `.out/acceptance/test.log`. No value is quoted here.
   - Fix: compare the maps, and print only the names that differ.
5. tests/acceptance/src/lib.rs:579. Note, INFEASIBLE, introduced.
   - Measured: the restart rebinds the port the first start chose. Another socket that takes it in between gives a red that names the cause in under 0.5 s. No hang, no green on another service.
   - What reaches it: I built that state; 0 of 25 runs of check 3 hit it.
   - Fix: give the clients an address the fixture owns (a forwarder kept across the restart, pointed at the new service's ephemeral port), or retry the start a bounded number of times.
6. tests/acceptance/tests/checks.rs:485. Note, CONFIRMED, introduced.
   - Measured: "no typed open or repost" is checked against the fixture's own log of the commands the check typed. No product behaviour can change that log, so it can never fail. The real proof is the `reconnected` members and `Pending`.
7. AGENTS.md:72 (the `task test` row). Note, CONFIRMED, introduced.
   - Measured: the row still says "5 skipped (the 4 ignored cases of tests/acceptance …)". The package now has 7 ignored cases, and the implementor's gate run shows 8 skipped. The row is outside the story's Scope; it is the coordinator's.

Mutants:
- M1, tls.rs:355 accepts any certificate: survived checks 1, 2 and 3; caught by `a_client_given_the_pin_of_another_certificate_posts_nothing`.
- M2, wake.rs:206 polls every 300 ms with wait=0: survived check 2 (6 of 6) and checks 1 and 3; caught by `a_wake_answers_its_post_on_the_read_it_held`.
- M3, checks.rs:388 compares the wake's cwd to alice's directory (used only to make check 2 fail while its wake runs): caught by check 2. After that red no inbox process of the copy was left (`ps` listed none before or after).

**5. Attacked and not broken**
- `assert_apart`: `Path::starts_with` compares whole components, so `alice` against `alice2` and a trailing slash are handled. `..` and symlinks are not resolved, but the fixture never builds such a path.
- Reads of the other client's files are not observed. The product reads no global path (grep: only a `HOME` fallback when `XDG_STATE_HOME` is unset), so no reader was found.
- Check 2: a wake that prints but does not mark, prints another id, or marks and prints nothing is red. `Delivered` is read from the service through bob's `pending`. The `/proc` pid is the wake itself, not a wrapper; if it has ended, the `expect` panics.
- Check 3: the lost request is asserted before the repost. A repost under a new id is red. 2 copies under one id cannot appear because the backend keys by id (as tests/adversary.rs says). A second reconnect is red.
- Count line: `set: [pipefail]` makes a failing check fail the task. Removing an `#[ignore]` or moving a check out of `checks::` drops the count below 6, which fails. 6 PASS lines of other tests pass the awk alone, but the filter keeps them out of the log.
- No sleep stands for a fact. HANG is 20 s and only ends a hang.

Not checked: the 30 s answer timeout of a client command that hangs (it is bounded only by nextest's 60 s kill); the README and docs/checks.md:280 edits against the Scope list; a TLS-resumption mutant (resumption is disabled in client_config).

**6. Paths written outside the worktree**, all under <scratch>/s3/acc/adversary/:
- copy/ (its target/ is deleted; copy/.out/ remains)
- scripts: build.sh, run.sh, mutant.sh, repeat.sh, leftover.sh, count.sh, clippy.sh
- original copies of mutated files: tls.rs.orig, wake.rs.orig, checks.rs.orig
- logs: build.log, alone-*.log (3), mutant-pin-ignored.{build.,}log, mutant-poll-300ms.{build.,}log, poll-check2-1..5.log, leftover.{build.,}log, repeat/ (35 logs), repeat.out, task-acceptance.log, package-all.log, clippy.log, count/ (3 logs)

```findings
[
{"file":"tests/acceptance/src/lib.rs","line":412,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The fixture says the stage-3 checks prove each client checks the certificate against its pin, yet a client that accepts any certificate passes all 3 checks; only crates/inbox-cli/tests/tls.rs holds that."},
{"file":"tests/acceptance/tests/checks.rs","line":373,"category":"mutant","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The waiting fact of check 2 also holds for a wake that polls with wait=0 every 300 ms, which passed check 2 in 6 of 6 runs; asserting the wake made 3 connections catches it."},
{"file":"tests/acceptance/tests/checks.rs","line":377,"category":"concurrency","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"from_client_after_service > 0 holds once the client's TLS Finished is carried, before its request, so a post that reaches the service before the held read's request is unlikely but not excluded."},
{"file":"tests/acceptance/tests/checks.rs","line":381,"category":"judgement","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The assert_eq! of the wake's /proc environment prints the pin and, if env_clear is ever dropped, the test process's whole environment into the public CI log."},
{"file":"tests/acceptance/src/lib.rs","line":579,"category":"concurrency","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"The restart rebinds the first start's port with no retry, so a socket that takes it in between makes check 3 red (it names the cause within 0.5 s); the state was built by hand and seen in 0 of 25 runs."},
{"file":"tests/acceptance/tests/checks.rs","line":485,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The no-typed-open-or-repost assertion reads the fixture's own log of what the check typed, so no product defect can make it fail."},
{"file":"AGENTS.md","line":72,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The task test row still says 5 skipped with 4 ignored cases in tests/acceptance; the package now has 7 ignored cases and the gate reports 8 skipped."}
]
```
