---
format: aep.planning-md/3
id: story:the-gate-fails-on-a-breaking-change-of-the-specification
kind: story
status: implemented
title: The gate fails on a breaking change of the specification
summary: 'task check compares no two revisions of ess/: a breaking change of the specification passes the gate and CI.'
scope:
- confidence: cited
  path: .github/workflows/check.yml
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: Taskfile.yml
- confidence: inferred
  path: crates/inbox-xtask/src
- confidence: inferred
  path: crates/inbox-xtask/tests
- confidence: inferred
  path: docs/checks.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T08:52:48Z", actor: "agent:inbox", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-24T08:52:48Z", actor: "agent:inbox", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-24T10:04:38Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":17}}}
---
## Outcome

`task check` fails when a change of `ess/` is breaking or of unknown compatibility against the
specification of `main`, and CI fails on the same change, so a narrowing is not merged as if
it were additive.

Today no step of `task check` compares two revisions of the specification. For
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal, which changed 3 summary
sentences of `ess/domains/messaging.yaml`, the comparison was run by hand: `ess verify diff
--from <the specification before> --to ess --compatibility --fail-on breaking-or-unknown`
answered `compatibility: 0 breaking, 0 unknown, 3 compatible` (2026-11-24, ESS 0.98.0).

## Acceptance

- A step of `task check` runs `ess verify diff --compatibility --fail-on breaking-or-unknown`
  from the specification of the merge base with `main` to the one of the tree and fails on a
  breaking or unknown item with the line of `ess`; a tree whose `ess/` equals the one of
  `main` passes without a comparison (state transition: a breaking change of the
  specification moves the gate from exit 0 to a non-zero exit).
- A planted breaking change (a removed command outcome) makes the step fail, and the same
  tree without it passes; both runs are named in the story's Measured.
- An acknowledged breaking change has one place in the repository where it is written, and
  the step reads it.
- The step is timed before and after; the gate's test step stays within 120 s; CI runs the
  same step (the local gate equals CI).

## Scope

No other story is ahead of this one in its files. The stories of the chat bridge that are
built beside it change the rows of `crates/inbox-cli/` and of the test step in `AGENTS.md`
and one bullet of `CHANGELOG.md`; whichever comes to `main` second resolves those rows in its
own tree.

Code:

- `crates/inbox-xtask/src` (the step as a command of `inbox-xtask`): inferred
- `crates/inbox-xtask/tests` (its cases): inferred
- `Taskfile.yml` (a task of its own, called by `task check` after `task spec`): cited
- `.github/workflows/check.yml` (the job names the revision it compares against): cited

Documents:

- `AGENTS.md` (the rows of `task check`, of the new task, of the CI job and of
  `crates/inbox-xtask/`; the place of an acknowledged breaking change): cited
- `docs/checks.md` (the step among the checks of the gate): inferred
- `CHANGELOG.md` (only if the file names changes of the gate): inferred

Nothing under `ess/`, `generated/`, `crates/inbox-core`, `crates/inbox-server` or
`crates/inbox-cli` changes.

## Context

A gap that was there before the story named above, found in its round 1 and filed as a draft.
The operator ordered it into the next wave, ahead of the run on the real channel (2026-11-24):
a breaking change of the specification must fail the gate, and the local gate equals CI. The
technique is the spec diff in the gate of the hardening catalogue of ESS.

## Decisions

- The story was taken into the wave ahead of the story of the real run on the order of the
  operator's session (2026-11-24): "a breaking change of the specification must fail the
  gate". No panel of plan critics read it first. The options were: (A) no panel, the
  adversary pass reviews the build; (B) a panel of 4 before the build. A is taken: the story
  was ordered into the wave, and its acceptance is one command with one exit status.
- The form of the step. The options were: (A) a command of `inbox-xtask` in Rust, with its
  cases in the test step; (B) a POSIX sh script under `tools/`, as `gate-paths.sh`. A is
  taken: the shell of the CI runner is `dash`, this host has none, and a script could not be
  run under the runner's shell before a push.
- The step is `task spec-compat` (`inbox-xtask spec-compat`), called by `task check` right
  after `task spec`, so it reads a specification that is valid.
- The base of the comparison is the commit `INBOX_SPEC_BASE` names when it is set and not
  empty, else the merge base of `HEAD` with `origin/main`, else with `main`. The local gate
  and the gate on the build node take the merge base. The CI job names the base: the revision
  its first step compares the changed files against (`HEAD^1`, the tip of `main`, on a pull
  request; the commit before the push on a push). A push whose commit before is all zeros
  (only when `main` is created) hands that value as the base, and the step fails with "names
  no commit".
- What differs between CI and the gate on the build node is written in one place, a bullet
  of the rules of `AGENTS.md`: the base (above); what is compared (the merge result in CI,
  the working tree with its uncommitted and untracked files on the node); and the
  acknowledgements file, which is bound to the digest of its base, so a file written against
  the fork is refused in CI once `main` changed `ess/` after the fork: merge `main` in and
  write the file anew.
- The step fails closed: a base it cannot find, a `git` or an `ess` that cannot be run or
  fails, and an `ess` that exits 0 without its verdict line each end the step with exit 1 and
  one line that names the cause. Only git's answer "this revision does not exist" (exit 1
  without a word of `rev-parse --verify --quiet`) is read as no commit, and only exit 1
  without a word of `merge-base` as no merge base; any other failure of git, such as a root
  that is no git repository, is named with the first line git wrote (adversary pass 1,
  finding 4).
- Whether `ess/` equals the base's is decided on the bytes of the files: the files git lists
  under `ess/` (tracked, and untracked that it does not ignore) are the paths of the base's
  tree, each a regular file whose bytes on disk, unfiltered (`git hash-object
  --no-filters`), have the blob id of the base's entry. Then the step writes one line and
  calls no `ess`. Round 1 decided it on git's difference, and adversary pass 2 (finding 1)
  showed a breaking edit that a clean filter of the repository's own configuration hides
  from that difference. The options were: (A) decide on bytes and keep the shortcut; (B)
  drop the shortcut and always run `ess`; (C) leave it as a note, since only local git
  configuration reaches it and CI reads committed content. A is taken: the acceptance says an
  equal `ess/` passes without a comparison, and the step fails closed. The executable bit is
  not compared: `ess` reads bytes.
- An entry under `ess/` that is marked assume-unchanged or skip-worktree in the index
  (adversary pass 1, finding 2: git then shows no difference for an edit of it, and the step
  said "equals"). The options were: (A) the step fails: it reads `git ls-files -v` for
  `ess/` before it decides, and exits 1 with one line that names the file, the mark and the
  command that clears it; (B) a note in the documents, since nothing in the repository sets
  such a mark. A is taken: the step fails closed, and a gate that passes on a hidden edit is
  the failure this story is for. A marked tree is compared first, so that `ess` shows what
  the mark hides; the step then exits 1 on the mark whatever `ess` says.
- An `ess` that exits 0 while its line counts a breaking change (adversary pass 1, finding
  6). The options were: (A) the step holds the counts: the line `compatibility: N breaking, N
  unknown, N compatible`, exactly one line `gate --fail-on breaking-or-unknown: passed (N
  acknowledged)` on standard error, and breaking plus unknown not more than the number
  acknowledged; a line it cannot read fails the step; (B) trust the exit status. A is taken:
  it is 2 lines of `ess` the step already relays, and a later `ess` that changes them fails
  the step instead of passing it. `no semantic change` is not held to counts.
- The comparison is `ess verify diff --compatibility --fail-on breaking-or-unknown` in all 3
  dimensions. `--strict-requires` is not passed: a base with an older pin of `ess` is then
  refused, measured on the move of the pin from 0.97.4 to 0.98.0, and every move of the pin
  would fail the step. `task spec` holds the tree's own `ess/` to the pin.
- An acknowledged change has one place, `.config/ess-diff-acknowledgements.json`, a document
  `ess-diff-acknowledgements/1`. `ess` accepts exactly its 4 members (`format`, `before`,
  `after`, `acknowledged`), so the reason of an acknowledged change is written in the
  changelog entry, not in the file. A file that `ess` refuses fails the step; the line the
  step adds says that `ess` refused the file and points at the line `refused:` of `ess`
  above it, and claims nothing about why (adversary pass 1, finding 3). The file is absent
  after this story.
- Every path in a line of the step is relative to the root of the repository, also where the
  line quotes git or the system (adversary pass 1, finding 5). The lines of `ess` are relayed
  as `ess` wrote them; `ess` is given relative paths.
- The cases of the step copy a specification of their own,
  `crates/inbox-xtask/tests/fixtures/spec-compat/ess/` (committed files; domain `shelf.box`),
  and never the `ess/` of the tree they run in (adversary pass 1, finding 1).
- The planted change of the story's check, a removed command outcome, is rated by `ess`
  0.98.0 as of unknown compatibility for callers and readers, not as breaking. The step fails
  on both ratings, and the cases hold one change of each.
- The step compares the model only: a change of a scenario file alone is "no semantic
  change". It compares a branch with the base of its pull request, not a release with the
  release before.
- A name of `main` whose reference exists and names no commit, such as a reference whose
  object is missing, fails the step with one line; the step does not go on to the next name
  (adversary pass 2, finding 2: `rev-parse` answers a missing object as it answers a missing
  reference, and the step fell back to a stale `main`). The step asks `git show-ref
  --verify` after a silent exit 1 of `rev-parse`, not before: 2 cases hold that the first
  failing command of a root git refuses is `rev-parse`.
- On exit 0 `ess` must write exactly one verdict line on standard output, `compatibility:`
  or `no semantic change` counted together, and exactly one line of the pass on standard
  error, also beside `no semantic change` (adversary pass 2, finding 3).
- When git finds no merge base and the repository is shallow, the line says that the history
  may end before the fork and names the 2 ways on: fetch more, or name the base with
  `INBOX_SPEC_BASE` (adversary pass 2, finding 9).
- Round 3 was not read by a third adversary pass: it is 160 lines of the step on 9 notes,
  each note with a case, and 8 mutants red.

## Measured

- Red first: the 14 cases of round 1 before the command existed: `14 tests run: 0 passed, 14
  failed`, each with `unrecognized subcommand 'spec-compat'`; after it the cases of
  `inbox-xtask`, 2 runs in a row: `134 tests run: 134 passed, 0 skipped` each.
- The planted change of the acceptance, on the tree of the story (round 1, base 0345c87):
  with the outcome `no-authority` of `PostMessage` removed in `ess/domains/messaging.yaml`,
  `task spec-compat` printed `fails --fail-on breaking-or-unknown:
  command/inbox.messaging.PostMessage/outcome-removed/no-authority` and `spec-compat: ess/
  against 0345c877fa8d: a change is breaking or of unknown compatibility (ess verify diff
  exited 4)`, and the step exited 1. With the edit reverted: `spec-compat: ess/ equals the
  one of 0345c877fa8d; nothing compared`, exit 0. With one summary sentence changed:
  `spec-compat: ess/ against 0345c877fa8d: compatibility: 0 breaking, 0 unknown, 1
  compatible`, exit 0.
- The time of the step: before the story it did not exist. `task spec-compat` through the
  pool, wall time with the sync and `cargo run`: 2.40 s on a clean tree, 2.07 s with the
  planted change, 2.07 s after the revert, 2.18 s with one summary sentence changed. The
  binary alone on the build node: 9 ms when `ess/` equals the base's, 44 ms when it differs
  by one summary sentence; one `ess verify diff` on this specification 0.027 s. The test step
  of `task check`: 15 s in round 1, 15 s in round 2, 16 s in the run after round 2; the run of
  `main` before the story 16 s. No commit made the step slower.
- Each line of the acceptance and its case, in `spec_compat`: the comparison and the line of
  `ess` (`a_breaking_change_fails_with_the_line_of_ess`,
  `a_removed_command_outcome_fails_as_a_change_of_unknown_compatibility`); the merge base
  (`a_main_that_moved_on_after_the_fork_compares_from_the_fork`,
  `a_tree_that_merged_a_newer_main_compares_only_its_own_change`); an equal `ess/` without a
  call of `ess` (`ess_equal_to_the_base_passes_without_a_comparison`: a stand-in `ess` first
  on `PATH` writes a file, and the case holds that the file is absent); the place of an
  acknowledged change (`an_acknowledged_change_passes_and_the_file_kept_for_another_change_fails`).
- Mutants of round 1, 7, each red in a named case: the exit of `ess` ignored; `--fail-on
  breaking`; the comparison skipped when only an untracked file differs; skipped when the
  base is not found; `HEAD` in place of the merge base; the acknowledgements file not passed;
  the tip of `main` in place of the merge base.
- Adversary pass 1 (review-result:adversary-spec-compat-pass-1): 8 findings, 7 cases, 5 of
  them red: the cases copied the `ess/` of the tree they ran in (one changed word of a
  summary there turned 8 of 14 red); an edit of an entry marked assume-unchanged or
  skip-worktree passed as "equals"; the line added after any refusal of the acknowledgements
  file claimed another comparison; a root that is no git repository was read as "names no
  commit"; a cause line held an absolute path; an `ess` that exits 0 with a breaking count
  passed; 2 groups of mutants that every case of round 1 let live (the order of the 2 names
  of `main`; the arm that reads the refusal), each killed by a green case of the pass. It
  corrected round 1's report: a push whose commit before is all zeros hands that value as
  the base and the step fails with "names no commit". It did not check: 2 merge bases; clean
  filters, `ident` and line-end attributes; the count "113 cases" in `docs/checks.md`; the
  git environment of a hook.
- Round 2 on those findings (all 8 recorded as fixed): the cases copy a specification of
  their own, domain `shelf.box` with 2 commands. With one word of the summary of `OpenInbox`
  changed in the `ess/` of the tree, the 2 modules before the change: `21 tests run: 6
  passed, 15 failed`; after it: `24 tests run: 24 passed`. The 5 red cases of the pass before
  round 2: `21 tests run: 16 passed, 5 failed`; after it `25 tests run: 25 passed`. In the
  adversary's file only the lines that make the fixture and name its literals changed. 4
  cases added: `an_entry_git_is_told_not_to_look_at_fails_with_its_mark`,
  `an_exit_0_is_held_to_the_number_it_acknowledged`,
  `a_root_git_refuses_is_named_by_the_line_of_git_without_a_path_of_the_host`,
  `a_merge_base_git_cannot_compute_is_named_by_the_line_of_git`.
- What `ess` 0.98.0 rates on the fixture: a changed summary sentence of `OpenBox`
  compatible; a required input added to `CloseBox` breaking; an optional input added to
  `CloseBox` unknown; the outcome `not-yours` of `OpenBox` removed unknown.
- The lines of `ess` 0.98.0 the rule on exit 0 is made from, measured on the fixture. No
  acknowledged change (one summary sentence): standard output `compatibility: 0 breaking, 0
  unknown, 1 compatible`, standard error `gate --fail-on breaking-or-unknown: passed (0
  acknowledged)`. One acknowledged change (the removed outcome, with the file): standard
  output `compatibility: 0 breaking, 1 unknown, 0 compatible`, standard error `acknowledged:
  command/shelf.box.OpenBox/outcome-removed/not-yours` and `gate --fail-on
  breaking-or-unknown: passed (1 acknowledged)`. No change: standard output `no semantic
  change: these two revisions mean the same system`, standard error the line of the pass with
  0.
- Mutants of round 2, 15, each red in a named case: the 7 of round 1 again, against the
  cases on their own specification; the index marks not read; the line of the mark only when
  `ess` passed; the old line after a refusal; any failure of `rev-parse` read as "no commit";
  any failure of `merge-base` read as "no merge base"; the paths not made relative; the
  counts not held; `>=` in place of `>` in the rule of the counts. 2 of them survived the
  first run and got an assertion (the mark with a hidden breaking edit) and a case (the merge
  base git cannot compute).
- Adversary pass 2 (review-result:adversary-spec-compat-pass-2), on round 2: 9 notes, 5
  cases added, 3 of them red: a breaking edit that a clean filter of the repository's own
  configuration hides from git passed as equal
  (`a_breaking_edit_a_clean_filter_hides_fails`); an `origin/main` whose object is missing
  was read as absent and the step fell back to a stale `main`
  (`an_origin_main_whose_object_is_missing_is_no_fallback_to_main`); a second line
  `compatibility:` with a breaking count passed (`a_second_compatibility_line_is_no_pass`).
  3 mutants of round 2 that the story's cases let live: the canonical root dropped from the
  rewriting of paths and the first of several lines of the pass taken (each killed by a
  green case of the pass), and the bare root kept (no case). 2 sentences of
  `docs/checks.md`: the count, and the base named as the merge base with `main`. A shallow
  history named as "no merge base". Its answers: in CI no breaking change passes, since a
  commit holds the filtered content; a clone of depth 2, as the CI checkout, resolves
  `HEAD^1`; the real `ess` 0.98.0 with 1 breaking, 1 unknown and 1 compatible change and 2
  acknowledged prints `compatibility: 1 breaking, 1 unknown, 1 compatible` and `passed (2
  acknowledged)`, and the rule holds. It did not run: a second merge base, a submodule entry
  under `ess/`, the untracked cache, `ess` itself as a symbolic link, the step on a clone of
  depth 2.
- Round 3 on those notes (all 9 recorded as fixed): the 3 red cases before the round: `22/30
  tests run: 19 passed, 3 failed`; after it the 2 modules `32 tests run: 32 passed`, and the
  cases of `inbox-xtask`, 2 runs in a row: `152 tests run: 152 passed, 0 skipped` each. 2
  cases added: `a_shallow_history_that_ends_before_the_fork_is_named` (a clone of depth 1
  made by the case) and `a_line_of_git_that_ends_on_the_root_names_it_as_dot` (git's refusal
  of a repository owned by another user, a line that ends on the root). 8 mutants, each red
  in a named case: the bytes not compared; the sets of paths not compared; the reference
  never asked; the first verdict line taken; the shallow history not asked; and the 3 of the
  adversary. The binary on the tree with an equal `ess/`, on the build node, 5 runs each:
  before the round 63 (the first run), 13, 15, 12 and 10 ms; after it 13, 13, 13, 13 and 12
  ms. `ident`, `core.fsmonitor` and a restored modification time are covered by reading the
  code (the bytes are hashed unfiltered, and no stat data is read), without a case. No
  adversary pass read round 3.
- The cases: 32 in 2 modules of `inbox-xtask`, 20 of the story (14 of round 1, 4 of round 2,
  2 of round 3) and 12 of the 2 adversary passes (7 and 5). No case over 5 s: the slowest
  took 0.17 s; the 25 cases after round 2 took 1.5 s of case time together, and each of the
  7 added since 0.13 s or less.
- Not held by a case: a line-end conversion on checkout or a smudge filter makes an
  unchanged `ess/` compared instead of passed over, read from the code. The count "113
  cases" of the row "Repository tasks" in `docs/checks.md` counts all cases of `inbox-xtask`
  at an earlier commit and was 120 before this story; this story counts its cases in a row
  of their own and leaves that number.
- The gate runs of the rounds on the build node (`task check`): round 1 1066 passed, 8
  skipped, the test step 15 s, exit 0; round 2, with `main` at 13f0ee6 merged in, 1096
  passed, 8 skipped, 15 s and 16 s, exit 0; round 3 1103 passed, 8 skipped, 16 s, exit 0.
  Each printed `spec-compat: ess/ equals the one of 0345c877fa8d; nothing compared`.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T10:00:35Z to 2026-11-24T10:01:07Z, on the build
  node of the pool: exit 0, `1103 tests run: 1103 passed, 8 skipped`, the test step 16 s of
  its bound of 120 s.
