---
format: aep.planning-md/3
id: review-result:adversary-rust-workspace-and-model-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:rust-workspace-and-model at d469eab
relations:
- reviews: story:rust-workspace-and-model
revision: 1
---
4 findings: 1 warning (a mutant the suite misses), 3 notes (INFEASIBLE). None blocks the merge.

unit: story:rust-workspace-and-model, commit d469eab on base 8abe907, worktree inbox-s2-w1-rust plus one untracked test file
verdict: CONFIRMED (one warning; the other three are INFEASIBLE notes)
cases: executed 10→5, red 4. The 10 is from the implementing state. In the gate run, cargo stopped after the red `adversary` test binary, so `drift.rs` was not reached. Its 10 cases were run separately on the mutant copy: 10 passed.
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (part 6)
needs-coordinator: none

## 1. git diff --stat
Empty: no tracked file changed. `git status --short` shows only `?? crates/inbox-xtask/tests/adversary.rs`. After the first run I reformatted one call in that file (line breaks only, so `cargo fmt --check` stays green). No implementation file was touched.

## 2. Cases added (`crates/inbox-xtask/tests/adversary.rs`), run alone: `cargo test --locked -p inbox-xtask --test adversary`, exit 101
- `the_drift_binary_exits_nonzero_on_drift`: runs `CARGO_BIN_EXE_inbox-xtask drift` on a copy whose `src/lib.rs` was edited, and asserts a non-zero exit plus the drift line. **Green now.** It is the case the suite lacks (finding 1).
- `the_binary_run_outside_cargo_checks_the_tree_it_is_run_from`: **red.**
  ```
  drift run from a drifted tree exited exit status: 0; stdout: 87 capabilities: 85 generated, 2 obligation(s), 0 refused
  23 artifact(s), written to <worktree>/target/inbox-xtask/drift
  generated/rust/inbox: no drift from a fresh synthesis of ess/
  ```
- `a_changed_mode_is_drift`: **red.** `assertion left != right failed  left: []  right: []`
- `a_symbolic_link_in_place_of_a_generated_file_is_drift`: **red.** `assertion left != right failed  left: []  right: []`
- `a_build_directory_inside_the_held_crate_is_not_drift`: **red.** `left: [NotGenerated("target/debug/.fingerprint/x")]  right: []`

Mutant probe, run on a copy in scratch (copy since deleted). `src/main.rs:49` was changed from `ExitCode::FAILURE` to `ExitCode::SUCCESS`:
- `cargo test --locked -p inbox-xtask --test drift`: `test result: ok. 10 passed; 0 failed`, exit 0.
- `the_drift_binary_exits_nonzero_on_drift`: red, `drift exited exit status: 0 on a drifted crate; stderr: generated/rust/inbox drifted from a fresh synthesis of ess/ (run \`task generate\`):\n  edited: src/lib.rs`.

## 3. Suite: `CARGO_INCREMENTAL=0 slot task check`, exit 201
- spec, projections-drift, drift, then `92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported`.
- `cargo fmt --check` and `cargo clippy ... -D warnings` passed.
- `cargo test --workspace --locked`, `tests/adversary.rs`: `test result: FAILED. 1 passed; 4 failed`, then `task: Failed to run task "check": exit status 101`.

## 4. Findings
1. **Warning: the exit status the acceptance rests on is never tested.** `crates/inbox-xtask/src/main.rs:49`, mutant, CONFIRMED, origin introduced.
   - Measured: `task check` fails on drift only because `inbox-xtask drift` exits non-zero. No case runs the binary. With FAILURE turned into SUCCESS, all 10 unit cases stay green, and the gate would print the drift line and still pass.
   - What reaches it: the Taskfile `check` → `drift` → `cargo run -p inbox-xtask -- drift`. Removing `- task: drift` from `check` in `Taskfile.yml` is not caught by any case either.
   - Fix: add a binary-level case like `the_drift_binary_exits_nonzero_on_drift`.
2. **Note: run directly, the binary checks the tree it was compiled in.** `crates/inbox-xtask/src/main.rs:26`, judgement, INFEASIBLE, origin introduced.
   - Measured: with no `CARGO_MANIFEST_DIR` set, the binary uses the path fixed at compile time. Run from a drifted copy, it checked the original worktree and exited 0.
   - Run the same way, `generate` would rewrite the original tree's crate.
   - What reaches it: nothing found. Every task goes through `cargo run`, which sets the variable.
3. **Note: the drift check compares bytes only.** `crates/inbox-xtask/src/lib.rs:152`, boundary, INFEASIBLE, origin introduced.
   - Measured: `differences` follows symbolic links (`path.is_dir()` here, `fs::read` at `:133`) and ignores file mode. A held file set to mode 0755, or replaced by a symbolic link to identical bytes, reports nothing, though git records both.
   - What reaches it: nothing found.
4. **Note: a build directory inside the generated crate counts as drift.** `crates/inbox-xtask/src/lib.rs:149`, boundary, INFEASIBLE, origin introduced.
   - Measured: a `target/` inside `generated/rust/inbox` is reported as `not generated: target/...`. The generated crate declares its own `[workspace]`, so a `cargo build` or an editor's `cargo check` there writes that `target/`. `.gitignore` ignores only `/target`, so git also shows it as untracked.
   - What reaches it: not the gate (`build-generated` passes `--target-dir`). A developer or editor building inside the crate would; not shown.

## 5. What held
- The workflow step: all 5 archive URLs and sha256 values match the local Rust 1.99.1 channel manifest (dist date 2026-10-15). The checksum is checked with `sha256sum --check --strict`, and clippy and rustfmt are installed.
- The generated crate's lock: if it is missing, `cargo ... --locked` fails ("cannot create the lock file ... because --locked was passed"), so it cannot go stale without the gate failing.
- `Cargo.lock` and `.ess-output` are ignored only at the top level; a nested `Cargo.lock` is reported (the unit's own case).
- `ess` missing or failing makes the drift check fail, not pass.
- An empty directory is not reported, but git does not track one.
- No home path, e-mail or person's name in any tracked file of the unit. `unsafe_code` is forbidden in the workspace and in the generated crate. No comment in the authored manifests.
- Could not establish:
  - `generate` removes the held crate (`src/lib.rs:82`) before the rename (`:87`). A failing rename (for example `target/` on another filesystem, which gives EXDEV) would leave no crate. On this host `target/` and the tree are on one filesystem, so I could not make the rename fail.
  - Whether the GitHub runner's rustup proxies in `~/.cargo/bin` come ahead of the installed prefix on PATH: untestable offline.

## 6. Paths written outside the worktree
All in the scratch directory `.../w1/rust-workspace-and-model/adversary/`: `adversary-alone.log`, `mutant.log`, `mutant-drift.log`, `suite.log`, `mutant.sh`. The tree copies `mut/` (which held its own `target/`) and `lockprobe/` are deleted. Running the gate and the outside-cargo case also wrote into the worktree's own `target/inbox-xtask/drift`, the same place `task drift` writes.

```findings
- file: crates/inbox-xtask/src/main.rs
  line: 49
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: no case runs the inbox-xtask binary, so turning its drift exit status into success leaves all 10 unit cases green while task check would pass on drift
- file: crates/inbox-xtask/src/main.rs
  line: 26
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: run outside cargo run, the binary falls back to the compile-time manifest directory and checks (or with generate rewrites) the tree it was built in, not the one it is run from
- file: crates/inbox-xtask/src/lib.rs
  line: 152
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: differences compares bytes only, following symbolic links and ignoring file mode, so a symlink or executable-bit change in the held crate that git records is not drift
- file: crates/inbox-xtask/src/lib.rs
  line: 149
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a target/ directory inside generated/rust/inbox, written by any cargo build there because the crate declares its own workspace, is reported as not-generated drift and is not git-ignored
```
