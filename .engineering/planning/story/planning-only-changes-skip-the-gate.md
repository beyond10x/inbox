---
format: aep.planning-md/3
id: story:planning-only-changes-skip-the-gate
kind: story
status: implemented
title: A pull request that changes only planning or prose files passes without build or test
summary: Every pull request runs the whole gate, also when it changes only the planning store or documents; one path list in the workflow, with a test.
tags:
- budget
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T02:04:08Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T02:04:09Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-22T02:46:47Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A pull request whose changed files are all planning or prose files passes its required check
without building or testing. The job `inbox / task check` of `.github/workflows/check.yml`
still runs and reports, so the required check has a result; it finds only such paths and exits
0 before it installs a tool.

One path list, in `tools/gate_paths.py`, which the workflow calls: `.engineering/`, and `*.md`
outside `website/` and outside `generated/`, except `README.md`. A change to any other path
runs everything, as today. The job `Build documentation` of `.github/workflows/pages.yml` is
not changed.

## Acceptance

`task gate-paths`, a step of `task check`, runs the path decision on 12 lists of changed files
and fails when an answer changes. Among them: a list with only `.engineering/planning/story/x.md` and `docs/plan/
stage-2.md` answers "skip"; the same list plus `crates/inbox-core/src/lib.rs` answers "run".
The workflow calls the same decision the test calls.

## Context

Rule of the folder this repository is developed in, 2026-11-21 ("Planning-only pull requests
skip the tests"): built as a story ahead of the next feature. Rulesets and branch protection
are the owner's and are not changed by this story.

Today every push to `main` and every pull request starts 4 runs, among them `task check`
(15 minutes bound, ESS and `task` downloaded and verified). The commits that open and close a
wave change `.engineering/` and documents and `AGENTS.md`, `README.md`, `CHANGELOG.md`.

Settled in the unit:

- `README.md` answers "run": the case
  `every_command_line_of_the_readme_first_use_is_a_step_of_this_test_in_its_order` of
  `crates/inbox-cli/tests/acceptance.rs` reads its "First use" section. `AGENTS.md`,
  `CHANGELOG.md` and the files under `docs/` are read by no step of `task check`;
  `docs/releasing.md` and `CHANGELOG.md` are read by `release.yml` only for a tag.
- A path under `website/` answers "run": `task docs-words` reads `website/docs` and
  `website/product.json`, and `task animation-drift` reads
  `website/src/generated/animation.json`. Both steps were added after this story was drafted.
- The decision applies to a pull request and to a push to `main`. A push that the script
  cannot list (no `before` commit, or one the runner cannot fetch) answers "run".
- A list with no path, a path git quoted and a path with `..` answer "run".

## Scope

- `.github/workflows/check.yml`
- `tools/gate_paths.py` (the decision and its cases), `Taskfile.yml` (`task gate-paths`)
- `AGENTS.md` (the rows of the workflows)
