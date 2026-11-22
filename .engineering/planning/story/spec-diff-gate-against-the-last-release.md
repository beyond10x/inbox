---
format: aep.planning-md/3
id: story:spec-diff-gate-against-the-last-release
kind: story
status: draft
title: The gate rates the specification against the last release
revision: 2
---
## Outcome

`task check` compares `ess/` with the specification of the last release and fails on a change
that breaks a caller, a reader or the history, or that ESS cannot rate, until the change is
acknowledged in a committed file.

Today no step of the gate reads the released specification. Measured on 2026-11-22 with
ESS 0.97.1 (`ess verify diff --fail-on breaking-or-unknown`):

- a specification against itself: 0 changes, exit 0;
- release 0.2.0 against `8e9c3ab`: 0 changes, exit 0;
- a copy with one planted change: exit 4, the change named;
- release 0.2.0 against the specification after the stories
  `no-unmapped-rule-in-the-specification` and
  `the-specification-follows-the-design-on-notes-and-options`: 12 changes, 8 rated `unknown`
  (3 on `Expire/expired`, 2 outcome conditions of `PostMessage`, the added outcome
  `Refuse/note`, the 2 added fields of `MessageExpired` counted as 2), exit 4.

Those 7 went to `main` with no step naming them.

## Acceptance

With the acknowledgement file of the current comparison committed, `task spec-diff` exits 0.
With one change id taken out of that file it exits 4 and prints the id (state transition: an
unacknowledged change of the specification turns the gate from green to red).

## Decisions

- The released specification is `ess/` at the newest tag that is a version (`0.2.0` today),
  written with `git archive` into `target/`. The forge's checkout fetches tags; the local gate
  and the forge read the same tag.
- The acknowledgement file is `ess-diff-acknowledgements/1`, bound to both digests by ESS: it
  is written again, with its reason per change, in the change that moves `ess/`.
- `docs/releasing.md` gains the step: a release starts a new comparison, so the file is
  emptied in the commit after the tag.
- The command is a command of `crates/inbox-xtask` or POSIX shell. No Python.

## Scope

- `Taskfile.yml`, `crates/inbox-xtask/`
- `.github/workflows/check.yml` (tags in the checkout)
- `ess-diff-acknowledgements.json`, `docs/checks.md`, `docs/releasing.md`, `AGENTS.md`
