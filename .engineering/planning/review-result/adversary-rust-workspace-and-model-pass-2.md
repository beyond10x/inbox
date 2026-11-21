---
format: aep.planning-md/3
id: review-result:adversary-rust-workspace-and-model-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:rust-workspace-and-model at 64b72d5
relations:
- reviews: story:rust-workspace-and-model
revision: 1
---
3 findings, all INFEASIBLE notes; one of them (finding 1) is a regression introduced by the corrections. None blocks the merge.

unit: story:rust-workspace-and-model, commit 64b72d5 (on d469eab, base 8abe907), worktree inbox-s2-w1-rust plus one untracked test file
verdict: INFEASIBLE (3 notes)
cases: executed 27→30, red 3. The 27 is the implementor's count, verified with `adversary2.rs` deselected: lib 3 + adversary 5 + drift 19, all passed. In the gate run, cargo stopped after the red `adversary2` binary, so `drift.rs` was not reached there.
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 new paths this pass (part 6)
needs-coordinator: none

## 1. git diff --stat
Empty: no tracked file changed. `git status --short` shows only `?? crates/inbox-xtask/tests/adversary2.rs`. `adversary.rs` was not touched, and no implementation file was touched.

## 2. Cases added (`crates/inbox-xtask/tests/adversary2.rs`), run alone: `cargo test --locked -p inbox-xtask --test adversary2`, exit 101, all red
- `generate_creates_the_crate_where_generated_rust_does_not_exist_yet`: a copy with `ess/` and `generated/openapi`, but no `generated/rust`, runs the binary's `generate`.
  ```
  generate exited exit status: 1; stderr: <scratch>/bootstrap (two levels above CARGO_MANIFEST_DIR) is no repository root: it holds no `generated/rust/`
  ```
- `generate_after_a_failed_restore_keeps_the_held_lock`: the held crate exists only in `target/inbox-xtask/previous`, with `Cargo.lock` = "held lock", and `generated/rust/inbox` is absent.
  ```
  generate returned Ok(()); the held lock is neither in the crate (None) nor left in <scratch>/target/inbox-xtask/previous
  ```
- `a_relative_manifest_directory_still_reports_the_drift`: `CARGO_MANIFEST_DIR=crates/inbox-xtask`, current directory = a drifted copy.
  ```
  exit exit status: 1; stderr: run ess: No such file or directory (os error 2)
  ```

## 3. Suite: `CARGO_INCREMENTAL=0 slot task check`, exit 201
- spec, projections-drift, drift, then `92 scenarios: 92 passed, 0 failed, 0 error, 0 unsupported`.
- fmt and clippy passed.
- lib: `3 passed`. `tests/adversary.rs`: `5 passed`. `tests/adversary2.rs`: `FAILED. 0 passed; 3 failed`, then `task: Failed to run task "check": exit status 101`.

With `adversary2.rs` deselected (`cargo test --locked -p inbox-xtask --lib --bins --test adversary --test drift`): 3 + 0 + 5 + 19 passed, exit 0.

## 4. Findings
1. **The root check refuses a tree that has no `generated/rust/` yet, so `generate` cannot create the crate.** `crates/inbox-xtask/src/lib.rs:72`, contract-drift, INFEASIBLE, origin introduced (by 64b72d5; d469eab would have created it).
   - The root check runs before `replace`, which would create the parent directory.
   - What reaches it: `rm -rf generated/rust && task generate`, or a second synthesized crate set up the same way. Not shown in this repository's workflow.
   - The same tree under `task drift` also fails with "no repository root" instead of `missing:` lines. That still fails closed.
   - Fix: require only `ess/` (and for example `Cargo.toml`) as the root markers.
2. **The next `generate` deletes a held crate that a failed restore left in `previous`, and loses its lock.** `crates/inbox-xtask/src/lib.rs:154`, concurrency, INFEASIBLE, origin introduced.
   - When moving the held crate back fails, the error says the crate is in `target/inbox-xtask/previous`.
   - The next `generate` then removes `previous` before reading anything from it. `generated/rust/inbox/Cargo.lock` is absent at `:133`, so no lock is kept, and `cargo metadata` resolves a new one. The drift check does not compare that lock.
   - What reaches it: both renames failing, or a run killed between the two moves. Not shown; I could not make the restore fail from outside, because all three renames need write access to the same two parent directories.
3. **A relative `CARGO_MANIFEST_DIR` reports a missing `ess` instead of the drift.** `crates/inbox-xtask/src/lib.rs:66`, boundary, INFEASIBLE, origin introduced (d469eab behaves the same).
   - `crates/inbox-xtask` resolves to the empty path. That passes the marker check relative to the current directory, then `Command::current_dir("")` fails, which reads as if `ess` were missing.
   - It fails closed, with a misleading message.
   - What reaches it: nothing found; cargo sets the variable to an absolute path.

## 5. What held
- Pass-1 cases do not pass for the wrong reason: the built binary, run from a subdirectory of a drifted copy with no `CARGO_MANIFEST_DIR`, printed `edited: src/lib.rs` and exited 1.
- The root search picks the nearest checkout upwards. `CARGO_MANIFEST_DIR` pointing at a tree without the markers is refused, not run on. Tests that spawn the binary remove or set that variable, so they do not inherit cargo's value.
- `replace` removes a leftover `previous` first. Across filesystems it now fails at the first move and leaves the crate unchanged. The held lock comes back with the restored crate.
- Links are never followed. A crate directory that is itself a link is refused, and a fresh side that is a dangling link is refused the same way. Only the top-level `target`, `Cargo.lock` and `.ess-output` are skipped; a nested `target` is compared.
- Checked by reading, not run: of the 27 cases, I found none that passes without exercising the behaviour it names.
- `.gitignore` covers `/generated/rust/*/target`.

## 6. Paths written outside the worktree
All in the scratch directory `.../w1/rust-workspace-and-model/adversary/`, new this pass: `p2-adversary2-alone.log`, `p2-suite.log`, `p2-deselected.log`, `p2-outside-cargo.sh`. The tree copy `copy/` is deleted. The pass-1 files are still there: `adversary-alone.log`, `mutant.log`, `mutant-drift.log`, `mutant.sh`, `suite.log`. The test scratch copies sit inside the worktree's git-ignored `target/tmp/adversary2/`.

```findings
- file: crates/inbox-xtask/src/lib.rs
  line: 72
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the root check demands generated/rust/ before generate runs, so generate cannot create the crate in a tree where generated/rust does not exist yet, which d469eab could
- file: crates/inbox-xtask/src/lib.rs
  line: 154
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: after a failed restore leaves the held crate only in target/inbox-xtask/previous, the next generate deletes that directory unread and the held Cargo.lock is lost to a fresh resolution
- file: crates/inbox-xtask/src/lib.rs
  line: 66
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'a relative two-component CARGO_MANIFEST_DIR resolves to the empty path, passes the root check and fails as "run ess: No such file or directory" instead of reporting the drift'
```
