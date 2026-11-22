#!/usr/bin/env python3
"""Decide whether a change needs `task check`.

Prints `skip` when every changed path is a planning or prose file, else `run`.
`.github/workflows/check.yml` calls it with the changed files of a pull request or a push, and
`task gate-paths` (in `task check`) holds its answers to the cases below.

A path is planning or prose when no step of `task check` reads it:

- everything under `.engineering/` (the planning store);
- a `*.md` file outside `website/` and `generated/`, except `README.md`.

`website/` is read by `task docs-words` and `task animation-drift`, `generated/` by
`task projections-drift` and `task drift`, and `README.md` by the acceptance case of
`crates/inbox-cli/tests/acceptance.rs` (its "First use" section). A list with no path, or a path
this script cannot read as a plain path, answers `run`.

usage: gate_paths.py <file with one changed path per line>
       gate_paths.py --test
"""
import sys

PLANNING = (".engineering/",)
READ_BY_THE_GATE = ("website/", "generated/")
READ_BY_A_TEST = ("README.md",)

CASES = [
    ("only planning and prose", [".engineering/planning/story/x.md", "docs/plan/stage-2.md"], "skip"),
    (
        "planning, prose and one source file",
        [".engineering/planning/story/x.md", "docs/plan/stage-2.md", "crates/inbox-core/src/lib.rs"],
        "run",
    ),
    ("the documents at the root that no step reads", ["AGENTS.md", "CHANGELOG.md", "docs/checks.md"], "skip"),
    ("an evidence record of the store, which is not Markdown", [".engineering/evidence/story/x/1.json"], "skip"),
    ("README.md, which an acceptance case reads", ["README.md"], "run"),
    ("a page of the site, which task docs-words reads", ["website/docs/status.md"], "run"),
    ("a generated document", ["generated/docs/docs/index.md"], "run"),
    ("the workflow itself", [".github/workflows/check.yml"], "run"),
    ("this script", ["tools/gate_paths.py"], "run"),
    ("no path", [], "run"),
    ("a path git quoted", ['"docs/pl\\303\\244ne.md"'], "run"),
    ("a path that leaves the tree", ["docs/../crates/inbox-core/src/lib.md"], "run"),
]


def planning_or_prose(path: str) -> bool:
    if path.startswith(('"', "/")) or ".." in path.split("/"):
        return False
    if path.startswith(PLANNING):
        return True
    if not path.endswith(".md"):
        return False
    return not path.startswith(READ_BY_THE_GATE) and path not in READ_BY_A_TEST


def decide(paths: list[str]) -> str:
    paths = [path for path in paths if path]
    return "skip" if paths and all(planning_or_prose(path) for path in paths) else "run"


def test() -> int:
    failed = 0
    for name, paths, expected in CASES:
        answer = decide(paths)
        if answer != expected:
            failed += 1
            print(f"gate_paths: {name}: {paths} answers {answer}, expected {expected}", file=sys.stderr)
    if not failed:
        print(f"gate_paths: {len(CASES)} case(s), each answers as stated")
    return 1 if failed else 0


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().split("usage: ")[1], file=sys.stderr)
        return 2
    if sys.argv[1] == "--test":
        return test()
    try:
        with open(sys.argv[1], encoding="utf-8") as listing:
            paths = listing.read().split("\n")
    except (OSError, UnicodeDecodeError):
        print("run")
        return 0
    print(decide(paths))
    return 0


if __name__ == "__main__":
    sys.exit(main())
