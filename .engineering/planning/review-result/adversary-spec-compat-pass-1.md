---
format: aep.planning-md/3
id: review-result:adversary-spec-compat-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the compatibility check of the gate: 8 findings, 5 red cases'
relations:
- reviews: story:the-gate-fails-on-a-breaking-change-of-the-specification
revision: 1
---
ADVERSARY PASS 1

unit: story:the-gate-fails-on-a-breaking-change-of-the-specification of beyond10x/inbox, commit cbe89ba (round 1, base 0345c87), in the adversary tree
verdict: NEEDS-CHANGE (one warning); the rest CONFIRMED or INFEASIBLE notes; no breaking change passes on a state anybody reaches
cases: executed 134→141, red 5
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the scratch directory: logs, one backup file, the directory probe)
needs-coordinator: none

1. Diff of the adversary tree (test paths only)
 crates/inbox-xtask/tests/main.rs | 1 +   (the line `mod adversary_spec_compat;`, after `mod adversary2;`)
 untracked: crates/inbox-xtask/tests/adversary_spec_compat.rs (7 cases, uncommitted, formatted with rustfmt, clippy -D warnings exit 0)
 git-ignored, in the tree: .engineering/drafts/adv-mutants.sh (the mutant script)
 `git status --short -- ess generated .config` is empty. The probes that changed ess/domains/messaging.yaml in the tree were reverted by copying back a backup.

2. The cases (all run on build-1d; time from the suite run)
- a_breaking_edit_git_is_told_not_to_see_fails: RED, 0.19 s. A breaking edit of ess/domains/messaging.yaml, then `git update-index --assume-unchanged` (and in a second repository `--skip-worktree`) on that file. Red line: "--assume-unchanged: a breaking edit of ess/ passed: exit 0 ... spec-compat: ess/ equals the one of c299bc72c9cd; nothing compared". I reproduced the same thing on the adversary tree through `slot task spec-compat`: with skip-worktree set, it printed "spec-compat: ess/ equals the one of 0345c877fa8d; nothing compared", exit 0. Without the flag, the same edit failed with exit 4.
- origin_main_wins_over_a_stale_main: GREEN, 0.08 s. main is behind origin/main; HEAD is at origin/main. The test asserts the line "equals the one of <origin/main>". It kills the mutant that swaps the order in MAIN; no case of the story kills that mutant.
- an_acknowledgement_of_an_absent_change_is_not_called_another_comparison: RED, 0.18 s. The acknowledgements file has the 2 digests of this comparison and one change id with a typo. Red output: "refused: .config/ess-diff-acknowledgements.json: [undeclared_reference] acknowledgements.acknowledged[0]: `command/inbox.messaging.CloseInbox/input-added/reason-typo` is not a change of this delta" and then "spec-compat: .config/ess-diff-acknowledgements.json is not for this comparison: write it for this comparison or remove it". The file is for this comparison. A probe with ess on files in the scratch directory: an empty file ("EOF while parsing") and a file that is not JSON ("expected ident") both get the same `refused:` line, so the step tells them the same wrong cause.
- the_refusal_line_is_added_only_on_a_refusal: GREEN, 0.10 s. Uses a stand-in ess. Three runs: a `refused:` line without the file, exit 1; the file present, exit 1 without a `refused:` line; the file present, a `refused:` line, exit 3. Each must end with "ess verify diff exited N; ... was not compared". It kills 3 mutants that the story's cases let live.
- an_exit_0_with_a_breaking_count_is_no_pass: RED, 0.06 s. A stand-in ess prints "compatibility: 1 breaking, 0 unknown, 0 compatible" and "passed (0 acknowledged)", then exits 0. Red line: "1 breaking, 0 acknowledged passed: exit 0 ... spec-compat: ess/ against c299bc72c9cd: compatibility: 1 breaking, 0 unknown, 0 compatible".
- a_root_that_is_no_git_repository_is_named: RED, 0.02 s. The root holds ess/ and Cargo.toml and is no git repository (GIT_CEILING_DIRECTORIES is set). Red line: "spec-compat: no base to compare with: neither origin/main nor main names a commit, and INBOX_SPEC_BASE is not set". Cause: `commit()` reads every failure of `git rev-parse` as "names no commit".
- a_cause_line_holds_no_path_of_the_host: RED, 0.05 s. .out/spec-compat/base is a regular file. Red line: "spec-compat: remove <absolute path of the case's root>/.out/spec-compat/base: Not a directory (os error 20)". The create and write errors of write_base have the same form.
- Sum of the 7 cases: 0.58 s; none is over 5 s.
- Order slip: my first run of these cases (red.log) started while my probe edit was still in the tree's ess/. The acknowledgement case went red then for that reason. I restored the file and ran again (red2.log): the same 5 red and 2 green, for the reasons above. The slip led to finding 6.

3. Suite runs, after the cases existed
- `cargo nextest run -p inbox-xtask --locked --no-fail-fast` (build-1d): "Summary [5.067s] 141 tests run: 136 passed, 5 failed, 0 skipped", exit 100. The 5 failures are the 5 red cases above. The 134 before is the count in the implementor's green run.
- `--stress-count 5`, without my module: "5/5 stress run iterations: 5 passed", 28.4 s. Each of the story's 14 spec_compat cases passed 5 times (70 PASS lines).
- `cargo clippy -p inbox-xtask --all-targets --locked -- -D warnings`: exit 0.

4. Findings
F1 (warning, NEEDS-CHANGE, introduced): crates/inbox-xtask/tests/spec_compat.rs:57. The story's cases copy the tree's live ess/ and replace literal lines of it (OPEN_SUMMARY, CLOSE_OUTCOMES, NO_AUTHORITY, lines 14-24). Measured: one word changed in the summary of OpenInbox in the tree's ess/ makes 8 of the 14 cases go red with "summary: Open the inbox at an address, or open it again. not in ...". What reaches it: any later story that edits that summary, the outcomes of CloseInbox or the outcome no-authority of PostMessage. The working edit of a developer reaches it too, and the test step goes red for a reason unrelated to the change. Fix lands in the cases: a fixed copy of ess/ as the fixture (committed, or read from a fixed commit), not the tree's working ess/.
F2 (note, INFEASIBLE, introduced): crates/inbox-xtask/src/spec_compat.rs:197, `equal()`. An index entry marked assume-unchanged or skip-worktree hides an edit from `git diff`. The step then says "nothing compared" while every other step of the gate reads the edited file. What reaches it: nothing in the repository's tooling sets these flags. Such an edit is also never committed, so CI does not see it. Fix: refuse when `git ls-files -v -- ess` shows a lowercase tag or an S tag, or compare file contents instead of asking git.
F3 (note, CONFIRMED, introduced): spec_compat.rs:108. The step adds "is not for this comparison: write it for this comparison or remove it" after every `refused:` line of ess. That includes a file with the right digests and a wrong id, an empty file, and a file that is not JSON. Fix: name ess's refusal ("ess refused <file>; see the line above") without claiming which comparison the file is for.
F4 (note, CONFIRMED, introduced): spec_compat.rs:176, `commit()`. Any non-zero exit of `git rev-parse` reads as "names no commit". A root that is no git repository, or a git that refuses the directory, therefore gets the line "neither origin/main nor main names a commit", or '"HEAD^1" names no commit' when INBOX_SPEC_BASE is set. It still fails closed. What reaches it: a tree exported without .git, or a git that refuses the directory as owned by another user (safe.directory).
F5 (note, INFEASIBLE, introduced): spec_compat.rs:232 (also 266 and 268). The cause lines of write_base hold the absolute path of the root. What reaches it: only a leftover file at .out/spec-compat/base, or an .out that cannot be written. Nothing found that makes either. Fix: show the path relative to the root.
F6 (note, INFEASIBLE, introduced): spec_compat.rs:130. On exit 0 the verdict line is trusted. "compatibility: 1 breaking ..." with 0 acknowledged passes. What reaches it: only a defect of ess; ess 0.98.0 exits 4 there, as measured. Fix: compare breaking plus unknown with the number in "passed (N acknowledged)".
F7 (note, CONFIRMED, introduced): spec_compat.rs:28. Swapping the order of MAIN (main before origin/main) survives the story's 14 cases. It is killed by origin_main_wins_over_a_stale_main.
F8 (note, CONFIRMED, introduced): spec_compat.rs:106. Three mutants of the refusal arm survive the story's 14 cases: the `refused:` check dropped, `Some(1)` widened to any exit, and `acknowledged &&` dropped. All 3 are killed by the_refusal_line_is_added_only_on_a_refusal.
0 pre-existing findings; no private file was written.

Mutants: one slot job on build-1d's copy of the adversary tree, `slot --timeout 30m sh .engineering/drafts/adv-mutants.sh`, filter spec_compat. Each mutant was reverted. "Killed by" leaves out my 5 cases that are red without any mutant.
- I1, acknowledgements file not passed (the implementor's): killed by spec_compat::an_acknowledged_change_passes_and_the_file_kept_for_another_change_fails
- I2, tip of main in place of the merge base (the implementor's): killed by spec_compat::a_main_that_moved_on_after_the_fork_compares_from_the_fork
- I3, untracked check dropped while the tracked one stays (the implementor's): killed by spec_compat::an_untracked_file_under_ess_is_compared
- A1, main before origin/main: the story's cases let it live; killed by adversary_spec_compat::origin_main_wins_over_a_stale_main
- A2, `refused:` line not checked: the story's cases let it live; killed by adversary_spec_compat::the_refusal_line_is_added_only_on_a_refusal
- A3, `refused:` matched on any exit: the story's cases let it live; killed by the same case
- A4, refusal arm without `acknowledged &&`: the story's cases let it live; killed by the same case
- A5, `--to` at the copy of the base: killed by 8 of the story's cases, among them a_breaking_change_fails_with_the_line_of_ess
- A6, copy of the base not emptied first: killed by spec_compat::the_base_is_written_under_out_anew_on_each_run
- A7, exit 1 of ess read as a pass: killed by spec_compat::an_acknowledged_change_passes_and_the_file_kept_for_another_change_fails and adversary_spec_compat::the_refusal_line_is_added_only_on_a_refusal

Answers to the brief, without a finding
- Item 1: ess 0.98.0 refuses an input path with `..` and an input that is a symbolic link (probe in the scratch directory). A file that git ignores matters only if ess-inputs.yaml lists it, and that listing is a tracked change that is compared. The variable cannot carry text into git: `--end-of-options` and the suffix `^{commit}` stop that. An INBOX_SPEC_BASE of HEAD compares only uncommitted changes; that is decided. The copy under .out is removed first, and remove_dir_all removes a link without following it. Paths are taken from ls-tree output with -z and must be plain.
- Item 2: For each case, what the step compares and whether a breaking change passes.
  - Behind main: fork to tree; it cannot pass.
  - Merged main: main's tip to tree (the story's case); it cannot pass.
  - 2 merge bases: git picks one of them; I made no case.
  - Stale local main beside a newer origin/main: origin/main wins (my green case).
  - Stale origin/main beside a newer local main: the base is older. More is compared, plus whatever only the local main holds. A removal of something that only the unpushed main added passes locally, and passes in CI as well, because the push would carry both commits.
  - Shallow clone without the fork: the merge base fails, and the step fails closed.
  - Linked worktree and detached HEAD: as on a branch. The node run in the adversary tree, a linked worktree, found 0345c87.
  - Tag named main, or a branch named origin/main: not used, because fully qualified refs are read.
- Item 3, CI:
  - Pull request: base=HEAD^1, the tip of main, against the merge result. Depth 2 holds it.
  - Push of one commit or of several: base=before, fetched at depth 1. Its tree and blobs are enough for diff, ls-tree and cat-file.
  - Forced update: base=the old tip, if the fetch gets it. Otherwise the step fails closed with "names no commit".
  - Re-run: the same values.
  - All-zero before: `grep -Ex '[0-9a-f]{40}'` accepts 40 zeros. So the base is that value, not empty, and the step fails with "names no commit". The implementor's report says it falls back to origin/main and compares nothing; that is wrong. This happens only when main is created.
  - The value is either HEAD^1 or 40 hex digits and goes in through `env:`, so it cannot carry text into a shell.
  - A pull request that changes ess/ answers `run` unless it changes only *.md files under ess/, which ess does not read. A pull request that changes only the acknowledgements file answers `run`.
- Item 3, differences between CI and the gate on the node for the same change:
  - CI's base is the tip of main, or before on a push; the node's is the merge base with origin/main.
  - CI compares a merge result; the node compares the working tree, with uncommitted and untracked files under ess/.
  - An acknowledgements file is bound to the digest of its base. A file written against the fork is refused by CI once main has changed ess/ since the fork. AGENTS.md implies this ("one comparison only") but does not say it.
- Item 5: git missing gives "git cannot be run"; ess missing gives "ess cannot be run"; a signal gives "ended by"; a base without ess/ gives "holds no ess/". All exit 1. The wrong cause for a tree without git is F4, and the host path is F5.
- Item 6: through `slot task spec-compat`, a failure exits 201 and a pass exits 0. On a failure the lines of ess come before the step's last line. A pass with a comparison ends with the counts.
- Item 7: the cases set GIT_CONFIG_GLOBAL=/dev/null and GIT_CONFIG_NOSYSTEM, a made-up identity, `init -b`, and `core.hooksPath=/dev/null` on their own git calls. The stand-in scripts are POSIX sh. The directories are unique per case, under target/tmp. Two runs of the suite at the same time on one tree would collide, because scratch() removes the directory by name. The story's cases take about 1.4 s in sum. Their dependence on the tree's ess/ is F1.

Not checked
- 2 merge bases (no case written).
- A clean filter, an ident attribute and CRLF attributes: I only reasoned about them; no case.
- `task check` as a whole (not run, by rule).
- The count "113 cases" in docs/checks.md (the implementor names it): I could not tell which cases it counts, so I make no claim.
- The git environment of a hook (GIT_DIR set by a pre-push hook): not run.

Paths written outside the adversary tree, all in the scratch directory:
- logs: task-spec-compat.log, p1.log, p2.log, red.log, red2.log, coupling.log, mutants.log, suite.log, stress.log, clippy.log
- messaging.yaml.orig (the backup used to revert the probes)
- the directory probe (ess copies and acknowledgements files of the ess probes)

Build commands run without the `slot` prefix: none. cargo nextest and clippy went through the shim to build-1d. `slot task spec-compat` and the mutant script ran with the prefix. ess, git and rustfmt were run directly.

```findings
[
{"file": "crates/inbox-xtask/tests/spec_compat.rs", "line": 57, "category": "judgement", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The story's cases copy the tree's live ess/ and replace literal lines of it, so one changed word in the summary of OpenInbox turns 8 of the 14 cases red."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 197, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A breaking edit of ess/ that is marked assume-unchanged or skip-worktree in the index passes as equal to the base without a comparison."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 108, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Every refusal of the acknowledgements file is called a file for another comparison, also a file with this comparison's digests and a wrong id, an empty file and a file that is not JSON."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 176, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A root that is no git repository fails with the line that neither origin/main nor main names a commit, because every failure of git rev-parse is read as no commit."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 232, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The cause lines of write_base for remove, create and write hold the absolute path of the root."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 130, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "An ess that exits 0 with the line 1 breaking and 0 acknowledged is reported as a pass, because the counts are not checked against the acknowledged number."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 28, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Swapping main ahead of origin/main in MAIN survives the story's cases and is killed by origin_main_wins_over_a_stale_main."},
{"file": "crates/inbox-xtask/src/spec_compat.rs", "line": 106, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Three mutants of the refusal arm (refused line not checked, any exit, file not required) survive the story's cases and are killed by the_refusal_line_is_added_only_on_a_refusal."}
]
```
