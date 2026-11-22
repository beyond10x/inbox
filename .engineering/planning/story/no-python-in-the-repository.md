---
format: aep.planning-md/3
id: story:no-python-in-the-repository
kind: story
status: implemented
title: No file of the repository is Python
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:17:40Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T04:17:40Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-22T05:59:36Z", actor: "agent:inbox", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

No file of this repository is Python and no task, workflow or document calls `python3`.

The operator, 2026-11-22: "No Python for permanent stuff."

On `main` at `8e9c3ab` there are 4 Python files (1,623 lines) and one inline call:

| file | lines | called by |
|---|---|---|
| `tools/scenarios.py` | 1,041 | `task scenarios`, `task scenarios-drift` |
| `tools/animation.py` | 441 | `task animation`, `task animation-drift` |
| `tools/gate_paths.py` | 91 | `task gate-paths`, `.github/workflows/check.yml` |
| `tools/docs_words.py` | 50 | `task docs-words` |
| `python3 -c 'import tomllib …'` | 1 | `.github/workflows/release.yml` |

## Acceptance

`git ls-files | grep -c '\.py$'` prints 0 and `grep -rn python Taskfile.yml .github AGENTS.md`
prints nothing, while `task check` exits 0 and every check the 4 scripts ran still fails on
the defect it failed on before (state transition: the 4 scripts and the inline call are
replaced, and the gate is as strict as it was).

## Decisions

- The tools are Rust, commands of `crates/inbox-xtask`.
- The path decision of the CI job runs before any toolchain is installed, so it is a POSIX
  shell script, `tools/gate-paths.sh`, with the same 12 cases.
- `ess/scenarios/*.yaml` and `ess/ess-inputs.yaml` do not change by one byte, and
  `website/src/generated/animation.json` does not either.
- `AGENTS.md` gains the rule and loses every mention of a Python script.

## Scope

- `tools/` (4 files removed, 1 added)
- `crates/inbox-xtask/` (commands, tests, dependencies)
- `Taskfile.yml`, `.github/workflows/check.yml`, `.github/workflows/release.yml`
- `AGENTS.md`, `CHANGELOG.md`, `Cargo.lock`
