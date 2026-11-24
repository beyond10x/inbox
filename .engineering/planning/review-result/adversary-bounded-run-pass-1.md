---
format: aep.planning-md/3
id: review-result:adversary-bounded-run-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 of the bounded run story: 2 warnings, 5 notes'
relations:
- reviews: story:no-case-of-the-bounded-run-waits-on-a-real-tool
revision: 1
---
ADVERSARY PASS 1

story:no-case-of-the-bounded-run-waits-on-a-real-tool, tree at 5004cbc against ab34af6. Every job ran on build-1d (each log starts `slot: synced to build-1d`). No job ran into its time limit and none was refused. I did not see a stale binary.

I found 2 warnings and 5 notes, and no blocker. No pre-existing security finding.

Cases I added (uncommitted, in the adversary tree):
- `crates/inbox-xtask/tests/adversary_bounded_run.rs`, with `mod adversary_bounded_run;` added to `crates/inbox-xtask/tests/main.rs`:
  - `adversary_bounded_run::every_setup_case_of_task_test_is_a_case_of_the_binary`: green, 0.006 to 0.025 s.
- `crates/inbox-cli/tests/adversary_outbox_expiry.rs`, with `mod adversary_outbox_expiry;` added to `crates/inbox-cli/tests/main.rs`:
  - `adversary_outbox_expiry::a_post_that_starts_after_its_expires_is_kept_and_dropped_by_the_next_command`: green, 0.014 to 0.047 s, and 5 of 5 under `--stress-count 5`.
- Both files pass `rustfmt --edition 2024 --check` and `cargo clippy -p inbox-cli -p inbox-xtask --all-targets --locked -- -D warnings` (exit 0). `git diff --stat` shows only the 2 `main.rs` lines and the 2 new files.

Findings

1. Warning: case 3 does not prove that `repost` drops the expired entry without sending it.
   - What I did: in `repost_each` (`commands.rs`), I made the outbox read use the clock minus 60 s (mutant M2).
   - Result: `adversary::an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires` stays green (1.035 s). The repost sends the expired envelope to no service and exits 3. The line still leaves the file, because `mark_unsent(…, now())`, written after the transport failure, filters out expired lines.
   - The case reads `reposted.status` but only prints it; it never asserts it.
   - My cli case asserts exit 0 and an empty `reposted`. It is red under M2: `adversary_outbox_expiry.rs:48`, `left == right failed: repost sent an expired envelope … status: 3`.
   - The fix lands in case 3 (`adversary.rs` at line 307): assert the repost's exit 0 and its empty `reposted` list.

2. Warning: the setup step does not check that all 3 setup cases ran.
   - What I did: ran the setup command with one name mistyped (`…stubborn_run_is_bilt`).
   - Result: cargo-nextest ran the other 2 cases and exited 0 (`2 tests run: 2 passed`). It fails only when nothing matches (exit 4, `no tests to run`).
   - So a setup case that is renamed or mistyped moves the build of its product into the bound, and nothing says so.
   - My xtask case reads the setup line of `Taskfile.yml` and checks that each named case is listed by the test binary. With the typo it is red: `adversary_bounded_run.rs:50`, `task test names the setup case tools::test_step::the_fixture_of_the_stubborn_run_is_bilt, which is no case of inbox-xtask`.
   - The fix: keep my case, or have `Taskfile.yml` (line 36) check the count.

3. Note: the fixture key misses some inputs of the build.
   - `stubborn_key` (`test_step.rs:250`) hashes the package files, the commands in `LISTS`, `rust-toolchain.toml` and the cargo-nextest version.
   - The entry is built under `target/tmp/` of the tree, so cargo also reads the repository's `.cargo/config.toml`. That file is not in the key, and neither is the environment that reaches the build (`RUSTFLAGS`, the wrapper).
   - A change of any of these keeps the old entry in use. I found this by reading the code; I did not run it.

4. Note: a damaged fixture entry is never built again.
   - `built_stubborn` returns any directory that exists (`test_step.rs:291`).
   - With the test binaries deleted from the entry:
     - the setup case fails at `test_step.rs:333` with "no test binary of the fixture was built";
     - case 2 fails at `test_step.rs:412` with nextest's `failed to exec … No such file or directory`.
   - Neither line names the cache entry, and every later run fails until someone removes it by hand. The synthesis entries behave the same way.
   - A `<key>.partial` left with junk in it is removed, and the entry is built anew and goes green.

5. Note: nothing removes old cache entries.
   - One synthesis entry is 1.3 MB and one fixture entry 3.4 MB.
   - Each change of `ess/` leaves 2 synthesis entries behind (the unchanged and the changed one). Each change of a fixture key member leaves one fixture entry.

6. Note: the changelog bullet does not match the code or the measured times.
   - `CHANGELOG.md:102` and the newest sentence of the `task test` row in `AGENTS.md` both say no case of the test step "waits on a real tool or a fixed sleep".
   - Case 2 still runs cargo-nextest and sleeps a fixed 1.5 s (`test_step.rs:395`). That was decided as accepted, but the sentence says the opposite.
   - The bullet gives 0.2 and 0.05 s for case 1, where the `AGENTS.md` row gives 0.15 to 0.24 and 0.04 to 0.10 s. The 0.2 is a time from the build node; the story measured 5.8 s alone and 8 to 17 s in a run on the workstation.
   - The rest checks out:
     - the `.config/nextest.toml` row and the `crates/inbox-xtask/` row match the code;
     - the `Taskfile.yml` desc matches;
     - "7 cases" in `test_step.rs` is right (9 `#[test]`, 2 of them inside `STUBBORN`);
     - 122 = 154 − (20 + 12);
     - `### Changed` has a precedent for a change only contributors see (the ESS 0.98.0 bullet of 0.6.1).

7. Note: case 3's name no longer says what it tests.
   - `adversary.rs:257` still says an envelope over the body limit leaves the outbox when it expires. That envelope is never kept; what expires is a second, small envelope.
   - The implementor is right that at ab34af6 everything after the first 2 assertions proved nothing. `admitted()` refuses the body before `keep`, and the case asserted the empty outbox before its 4 s sleep.

Attack results
- Case 1:
  - At ab34af6 it ran the real `ess` on the changed `ess/`. Now it reads a cache entry whose key covers every file of `ess/` (including the `ess-inputs.yaml` pin and the scenarios), the `ess` version, path, length and modification time, the synthesis arguments and the script text.
  - One changed summary sentence in `messaging.yaml` made both entries anew (2 new keys), and so did one added line in a scenario file (2 more keys).
  - The code of `inbox_xtask::synthesize_with` is not in the key; see mutant M-args below.
- Case 2:
  - Over 3 runs no file in the entry changed, was added or got newer. The JUnit report and the beat files go to the copy.
  - Cold bounded run of `inbox-xtask` alone with all entries removed: 155 passed. Times: case 1 0.206 s, the synthesis setup cases 0.011 and 0.064 s, the fixture setup case 0.42 s, case 2 3.94 s. Each product was built once (1 entry per key).
  - Setup case, case 2, case 1 and the 2 synthesis setup cases started together on empty caches, 5 times: all green, 1 entry per key.
  - If a setup case fails, go-task stops before the bounded run.
- Case 3 under load:
  - `Outbox::keep` appends after it filters, so an envelope that has already expired is still kept.
  - My cli case shows it: the client started 2 s after its `expires`, the post exits 3, and the outbox holds 1 line.
  - Under mutant M3 (keep filters after it appends) case 3 stays green and my case is red at `adversary_outbox_expiry.rs:35/36` (`left: 0, right: 1`). On a loaded node, M3 would make case 3 fail at its line count.
- Waits:
  - Case 3 polls every 20 ms with a 10 s bound; the case took 1.027 to 1.105 s, so the margin is about 9 s.
  - The cache locks wait with no bound of their own; nextest's 60 s kill is the only bound. A cold build took 0.3 to 0.42 s on the node.
  - The fixed waits of case 2 are decided.
- Times:
  - `slot --timeout 30m task test`, 3 runs: exit 0 each, 1131 passed, 8 skipped.

    | Run | Load at start (64 CPUs) | Wall time | Bounded run | Case 1 | Case 2 | Case 3 | Setup cases |
    |---|---|---|---|---|---|---|---|
    | 1 | 30.9 | 25.9 s | 22 s | 0.570 s | 3.675 s | 1.027 s | 0.007 to 0.018 s |
    | 2 | 59.6 | 21.6 s | 21 s | 0.180 s | 3.544 s | 1.105 s | 0.017 to 0.032 s |
    | 3 | 77.8 | 19.8 s | 19 s | 0.064 s | 3.528 s | 1.050 s | 0.011 to 0.026 s |

  - `--stress-count 10`, all green:
    - case 1: 0.039 to 0.055 s;
    - case 2: 3.522 to 3.534 s;
    - case 3: 1.027 to 1.081 s.
  - None of the 3 cases went over 5 s.

Mutants
- 7a, `synthesize_changed` answers from the unchanged entry: caught by case 1 (`drift.rs:326`).
- 7b, fixture key without the package files, with the fixture's tests then changed: caught by setup case `the_fixture_of_the_stubborn_run_is_built` (`test_step.rs:326`). Case 2 stayed green on the stale entry.
- 7c, without `--target-dir-remap`: caught by case 2 (`test_step.rs:410`, failed to exec the binary under `<key>.partial`).
- 7c2, without `--workspace-remap`: caught by case 2 (`test_step.rs:410`, "workspace root manifest … does not exist").
- 7d, the poll of case 3 with `<` in place of `<=`: survives, but behaves the same. The instants have sub-millisecond precision and `repost` reads the clock later.
- M1, `expires > now || true` in `unexpired`: caught by case 3 (`adversary.rs:308`) and by my cli case.
- M2, the outbox read of `repost_each` 60 s behind the clock: survives the story's cases; caught by my cli case (finding 1).
- M3, `keep` filters after it appends: survives the story's cases; caught by my cli case.
- M-rename, a setup name mistyped in `Taskfile.yml`: survives the story's cases; caught by my xtask case (finding 2).
- M-args, `synthesize_with` drops its last argument:
  - with warm entries case 1 stays green;
  - in the crate run it is caught by `adversary2::generate_creates_the_crate_where_generated_rust_does_not_exist_yet`, `adversary2::a_relative_manifest_directory_still_reports_the_drift` and `adversary::the_drift_binary_exits_nonzero_on_drift`;
  - with cold entries it is caught by both synthesis setup cases (`synthesis.rs:235`).

Not checked
- Whether an entry made in another or a moved tree is read correctly, beyond the 2 remap mutants.
- The host triple as a key member.
- Running a build with a changed `RUSTFLAGS` or `.cargo/config.toml` (finding 3 is from reading the code).
- Case 3 on the workstation's load. Every run was on the node, at a load of 26 to 78 on 64 CPUs, not 100 on 20 CPUs.

Build commands I ran without `slot`: none. Locally I ran only `rustfmt` (formatting) and `git` (reads). Every edit for a mutant was made in the node's copy inside its own job and undone in the same job; the adversary tree itself got no mutant. Scripts and logs are in the adversary tree's git-ignored `.engineering/drafts/` and in the scratch directory.

```findings
[
{"file":"crates/inbox-cli/tests/adversary.rs","line":307,"category":"test-strength","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Case 3 never asserts the exit or the empty `reposted` list of its repost, so a repost that sends the expired envelope (mutant: the outbox read of `repost_each` 60 s behind the clock) stays green, because the mark written after the transport failure drops the line."},
{"file":"Taskfile.yml","line":36,"category":"setup-step","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"The setup command selects its 3 cases by name and cargo-nextest runs the ones it finds with exit 0 when a name is mistyped or renamed, so that product is then built inside the bound with no line saying so, and nothing checks that 3 ran."},
{"file":"crates/inbox-xtask/tests/tools/test_step.rs","line":250,"category":"cache-key","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The fixture key leaves out the repository's `.cargo/config.toml`, which cargo reads for the package under `target/tmp/`, and the environment that reaches the build such as `RUSTFLAGS` and the wrapper, so a change of either keeps the old entry in use."},
{"file":"crates/inbox-xtask/tests/tools/test_step.rs","line":291,"category":"cache-integrity","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"An entry directory that lost a file is used as it is and never built anew; the setup case fails with `no test binary of the fixture was built` and case 2 with nextest's `failed to exec … No such file`, neither naming the entry, so every later run fails until it is removed by hand."},
{"file":"crates/inbox-xtask/tests/tools/test_step.rs","line":287,"category":"cache-growth","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Nothing removes old cache entries: each change of `ess/` leaves 2 synthesis entries of 1.3 MB and each change of a fixture key member a fixture entry of 3.4 MB."},
{"file":"CHANGELOG.md","line":102,"category":"docs","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The bullet and the newest sentence of the `task test` row of AGENTS.md say no case of the test step waits on a real tool or a fixed sleep while case 2 still runs cargo-nextest and sleeps a fixed 1.5 s, and the bullet's 0.2 and 0.05 s for case 1 are not the row's 0.15 to 0.24 and 0.04 to 0.10 s."},
{"file":"crates/inbox-cli/tests/adversary.rs","line":257,"category":"naming","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Case 3's name still says an envelope over the body limit leaves the outbox when it expires, but that envelope is never kept and what expires is a second small envelope."}
]
```
