---
format: aep.planning-md/3
id: review-result:adversary-spec-compat-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the compatibility check of the gate: 9 notes, 5 cases added, 3 of them red'
relations:
- reviews: story:the-gate-fails-on-a-breaking-change-of-the-specification
revision: 1
---
ADVERSARY PASS 2

unit: story:the-gate-fails-on-a-breaking-change-of-the-specification of beyond10x/inbox, commit cabb9ec (round 2), in the adversary tree
verdict: no breaking change passes in CI. Locally, two states that nobody reaches by normal use let a breaking ess/ pass: a clean filter in the repository's own configuration, and an origin/main ref whose object is missing. The rest are notes.
cases: 7 → 12 in the adversary module, 3 red; crate suite 150 run, 147 passed, 3 failed
origin: introduced 9 / pre-existing 0 / undecided 0
0 pre-existing findings; no private file was written.
wrote-outside-worktree: the scratch directory only (probe scripts, probe directories, logs)
needs-coordinator: none

1. Diff of the adversary tree
 crates/inbox-xtask/tests/adversary_spec_compat.rs | 127 +  (5 cases appended after a comment line "Pass 2."; nothing that was there is changed; uncommitted; formatted with rustfmt; `cargo clippy -p inbox-xtask --all-targets --locked -- -D warnings` exit 0)
 git-ignored, in the tree: .engineering/drafts/adv2-mutants.sh
 `git status --short -- ess generated .config` is empty. I did not edit ess/ of the tree. All probes ran on copies in the scratch directory.

2. The cases (build-1d; time from the run of the adversary module)
- a_breaking_edit_a_clean_filter_hides_fails: RED, 0.038 s. The repository config sets `filter.hide.clean` to a grep that drops the added input line, and `.git/info/attributes` has `ess/** filter=hide`. Then the breaking edit. Red output: "spec-compat: ess/ equals the one of 865919fbf9d9; nothing compared", exit 0. A git-only probe gives the same: `git diff --quiet --no-ext-diff --no-textconv main -- ess` exits 0, and `ls-files -v` shows only `H` tags.
- an_origin_main_whose_object_is_missing_is_no_fallback_to_main: RED, 0.114 s. History: stale main without the outcome not-yours, then origin/main with it, then the branch work, which removes it. The control run fails, exit 1. Then the loose ref origin/main is overwritten with the id of an object that does not exist. Red output: "spec-compat: ess/ equals the one of 5b58c3c781cd; nothing compared" (the stale main), exit 0. Cause: `git rev-parse --verify --quiet <ref>^{commit}` on a missing object exits 1 and writes nothing, so `commit()` reads it as "absent" and goes on to `main`. A ref that names a tree is different: git writes "expected commit type", and the step fails with that line, as it should.
- a_second_compatibility_line_is_no_pass: RED, 0.024 s. A stand-in ess prints "compatibility: 0 breaking, 0 unknown, 1 compatible", then "compatibility: 1 breaking, 0 unknown, 0 compatible", then "passed (0 acknowledged)", and exits 0. The step takes the first line and passes.
- two_lines_of_the_pass_are_no_pass: GREEN, 0.081 s. Kills mutant M2, which survives the story's cases.
- a_root_through_a_symbolic_link_is_named_relative: GREEN, 0.016 s. CARGO_MANIFEST_DIR is set under a symbolic link to the root, and `.git` is a gitfile that points nowhere. git names the real path of the root's `.git`, and the line must show neither the real path nor the link. Kills mutant M1, which survives the story's cases. My first version also required the word "nowhere" in the line. The git on the node does not print it ("gitfile does not point to a valid repository: .git"), so I dropped that condition.
- Sum of the 5 new cases: 0.27 s. None is over 5 s. Crate suite: "Summary [4.158s] 150 tests run: 147 passed, 3 failed"; the 3 failures are the red cases above.

3. Findings
F1 (note, CONFIRMED, introduced): spec_compat.rs:359, `equal()`.
- What happens: the "nothing compared" shortcut trusts git's view of the working tree. A clean filter makes a breaking edit invisible to git, and the step passes without running ess. Every other step of the gate reads the edited file.
- Same cause, reasoned but not run: the `ident` attribute, `core.fsmonitor` with a hook that reports nothing, and a same-size edit with the mtime restored under `core.trustctime=false`.
- What reaches it: only local configuration. A commit carries the cleaned content, so CI never sees the edit. It is the same class as F2 of pass 1.
- Fix lands in `equal()`: decide equality by content (the base's blob ids against `git hash-object --no-filters` of the listed files), or drop the shortcut and always run ess.
F2 (note, CONFIRMED, introduced): spec_compat.rs:297, `commit()`.
- What happens: exit 1 without a word is read as "this ref does not exist". A missing object answers the same way, so a broken origin/main silently falls back to main, and a change that is breaking against origin/main passes.
- What reaches it: a corrupt repository only.
- Fix lands in `base()`: ask separately whether the ref exists (`git show-ref --verify`), and fail when it exists but names no commit.
F3 (note, INFEASIBLE, introduced): spec_compat.rs:197, `judge()`. Only the first `compatibility:` line is held to the counts; a second line with a breaking count passes. What reaches it: only a defect of ess. Fix: require exactly one such line, as is already done for the pass line.
F4 (note, CONFIRMED, introduced): spec_compat.rs:84, `relative()`. Mutant M1 (the canonical root is never added) survives the story's 18 cases. It is killed by a_root_through_a_symbolic_link_is_named_relative.
F5 (note, CONFIRMED, introduced): spec_compat.rs:220, `judge()`. Mutant M2 (`let [line, ..]`, the first of several pass lines taken) survives the story's cases. It is killed by two_lines_of_the_pass_are_no_pass.
F6 (note, CONFIRMED, introduced): spec_compat.rs:107, `relative()`. Mutant M4 (the bare root is never replaced) survives every case, mine included. I found no git or system line that ends on the bare root, so I wrote no case.
F7 (note, CONFIRMED, introduced): docs/checks.md:16. The row says "24 cases (17 of the story, 7 of its adversary)". The story's module has 18 cases (`grep -c '#[test]'`), so 25 in all: round 2 added the merge-base case after it wrote the count. If the coordinator moves my 5 cases, the count is 30 (23 adversary).
F8 (note, CONFIRMED, introduced): docs/checks.md:16. The row names the base as "the merge base of `HEAD` with `main`". The code, AGENTS.md and the `desc:` in Taskfile.yml take origin/main first. This is round 1's sentence; round 2 left it as it was.
F9 (note, CONFIRMED, introduced): spec_compat.rs:275, `base()`.
- What happens: in a shallow clone whose fork is beyond the depth, `git merge-base HEAD origin/main` exits 1 without a word (git probe, depth-2 clone). The step then says "HEAD and origin/main have no merge base". It fails closed, but it does not name the shallow history.
- What reaches it: a local or node run in a shallow clone. CI is not reached, because it names the base.

Mutants
One slot job on build-1d's copy of the tree: `slot --timeout 30m sh .engineering/drafts/adv2-mutants.sh`, filtered to both modules. Each mutant was reverted inside the script, and the local source is unchanged.
- M1, `&& real != root` changed to `&& real == root` (no canonical root): the story's cases let it live; killed by adversary_spec_compat::a_root_through_a_symbolic_link_is_named_relative
- M2, `let [line] = passed[..]` changed to `let [line, ..]`: the story's cases let it live; killed by adversary_spec_compat::two_lines_of_the_pass_are_no_pass
- M3, `skip = tag == "S"` only (lowercase s not read as skip-worktree): killed by spec_compat::an_entry_git_is_told_not_to_look_at_fails_with_its_mark
- M4, `if ends && false` (bare root kept): survived every case (F6)
"Killed by" leaves out my 3 cases that are red without any mutant.

Answers to the brief, without a finding
- Item 1:
  - An intent-to-add entry: git diff shows it (exit 1), so it is compared.
  - Unmerged entries: `ls-files -v` tags them `M` three times; the diff shows them, so they are compared, and ess then reads the conflict markers.
  - A path with a space or a line break: `-z` and the split at the first space handle it. A non-UTF-8 path is only shown lossily in a line.
  - The fixture's `ess-inputs.yaml` lists its files explicitly, so a file that git ignores is not read.
  - CRLF conversion only hides a line-end change, which is no semantic change.
- Item 2:
  - The real ess 0.98.0, with 1 breaking, 1 unknown and 1 compatible change and the 2 failing ids acknowledged: exit 0, "compatibility: 1 breaking, 1 unknown, 1 compatible", 2 `acknowledged:` lines, "passed (2 acknowledged)". The rule holds: 2 ≤ 2.
  - Number forms: `+1` parses (u64 parse takes a plus sign), which is harmless. A trailing space is refused. A CR before the line end is stripped by `lines()`. A pass line on standard output counts as 0 lines and fails.
  - A sum that overflows panics in the debug build that `task spec-compat` runs: exit 101, no pass. Reasoned, not run.
- Item 3: the root is always absolute (the current directory, or CARGO_MANIFEST_DIR joined to it), so a root of `.` is not reached. A root of `/` is skipped by the empty check. A root reached with `..` is covered by the canonical branch.
- Item 4:
  - Depth-2 clone (git probe): `HEAD^1` resolves (exit 0) and `HEAD~2` exits 1 without a word.
  - In CI the step therefore compares a pull request against `HEAD^1`, and a push against `before`, which the first step fetches at depth 1.
  - An all-zero `before`: git's rev-parse exits 1 without a word, so the step says "names no commit", as AGENTS.md says.
- Item 6: the AGENTS.md bullet, its rows, CHANGELOG.md and the `desc:` agree with the code, apart from F7 and F8.

Not checked
- Not run: a second merge base, a submodule entry under ess/ (with `ignore = all`), the untracked cache, `ident`, `core.fsmonitor`, a scenario-only change with the real ess, and ess itself as a symbolic link.
- The step itself was not run on a depth-2 clone: only git was probed.
- Item 5: no mutant of the fixture. The planted breaking and unknown changes are asserted by ess's own `fails` lines in the cases that use them, so a different rating would turn those cases red.
- `task check` was not run, by rule.

Build commands run without the `slot` prefix: none. nextest and clippy went through the shim to build-1d (every log starts with "slot: synced to build-1d"). The mutant script ran with `slot --timeout 30m`. ess, git and rustfmt ran directly.

Paths written outside the adversary tree, all in the scratch directory: gitprobe.sh, gitmsg.sh, essprobe.sh, the probe directories gp, gm and ep, box.before, run1.log, run2.log, run3.log, mutants.log, clippy.log, suite.log.

```findings
[
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 359, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A breaking edit that a clean filter of the repository's own configuration hides from git diff passes as equal to the base without a comparison."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 297, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "An origin/main whose object is missing is read as absent, so the step falls back to a stale main and passes a change that is breaking against origin/main."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 197, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "Only the first compatibility line is held to the counts, so an ess that exits 0 with a second line counting a breaking change passes."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 84, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping the canonical root from relative() survives the story's cases and is killed by a_root_through_a_symbolic_link_is_named_relative."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 220, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Taking the first of several pass lines survives the story's cases and is killed by two_lines_of_the_pass_are_no_pass."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 107, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Keeping the bare root unreplaced in relative() survives every case of both modules."},
{"file": "docs/checks.md", "line": 16, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The row counts 24 cases (17 of the story) while the story's module holds 18 cases, 25 in all."},
{"file": "docs/checks.md", "line": 16, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The row names the base as the merge base of HEAD with main while the code takes origin/main first."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 275, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "In a shallow clone whose fork is beyond the depth, the silent exit 1 of git merge-base is named as no merge base rather than as missing history."}
]
```
