---
format: aep.planning-md/3
id: story:planning-only-changes-skip-the-gate
kind: story
status: draft
title: A pull request that changes only planning or prose files passes without build or test
summary: Every pull request runs the whole gate, also when it changes only the planning store or documents; one path list in the workflow, with a test.
tags:
- budget
revision: 1
---
## Outcome

A pull request whose changed files are all planning or prose files passes its required check
without building or testing. The job `inbox / task check` of `.github/workflows/check.yml`
still runs and reports, so the required check has a result; it finds only such paths and exits
0 before it installs a tool.

One path list, in the workflow: `.engineering/`, and `*.md` outside `website/` and outside
`generated/`. A change under `website/` still runs the job `Build documentation` of
`.github/workflows/pages.yml` and skips `task check` when nothing else changed. A change to
any other path runs everything, as today.

## Acceptance

A test in the repository runs the path decision on 2 lists of changed files and fails when
either answer changes: a list with only `.engineering/planning/story/x.md` and `docs/plan/
stage-2.md` answers "skip"; the same list plus `crates/inbox-core/src/lib.rs` answers "run".
The workflow calls the same decision the test calls.

## Context

Rule of the folder this repository is developed in, 2026-11-21 ("Planning-only pull requests
skip the tests"): built as a story ahead of the next feature. Rulesets and branch protection
are the owner's and are not changed by this story.

Today every push to `main` and every pull request starts 4 runs, among them `task check`
(15 minutes bound, ESS and `task` downloaded and verified). The commits that open and close a
wave change `.engineering/` and documents and `AGENTS.md`, `README.md`, `CHANGELOG.md`.

To settle in the unit: `AGENTS.md`, `README.md` and `CHANGELOG.md` are prose, but `task check`
reads none of them; `docs/releasing.md` and `CHANGELOG.md` are read by `release.yml` only for
a tag. Whether the decision applies to a push to `main` as well as to a pull request (the rule
names pull requests).

## Scope

- `.github/workflows/check.yml`
- a script or task that holds the path decision, and its test (place decided in the unit)
- `AGENTS.md` (the rows of the workflows)
