"""Fail when an end-user page names a wave, a stage, a story, an epic or a count of scenarios.

The pages of the documentation site are written for a user of the tool. The plan and the numbers
of the gate belong in docs/checks.md and docs/plan/.

usage: docs_words.py <file or directory>...
"""
import pathlib
import re
import sys

WORDS = re.compile(
    r"\bwaves?\b|\bstages?\b|\bstory:|\bepic:|\bdraft stor(y|ies)\b|\badversary\b"
    r"|\b\d+ (authored |generated )?scenarios\b|\b\d+ of the \d+\b",
    re.IGNORECASE,
)


def files(arguments: list[str]) -> list[pathlib.Path]:
    found: list[pathlib.Path] = []
    for argument in arguments:
        path = pathlib.Path(argument)
        if path.is_dir():
            found.extend(sorted(p for p in path.rglob("*") if p.suffix in (".md", ".mdx")))
        else:
            found.append(path)
    return found


def main(arguments: list[str]) -> int:
    paths = files(arguments)
    if not paths:
        print("docs_words: no file to read", file=sys.stderr)
        return 2
    hits = 0
    for path in paths:
        for number, line in enumerate(path.read_text().split("\n"), start=1):
            match = WORDS.search(line)
            if match:
                hits += 1
                print(f"{path}:{number}: `{match.group(0)}`: {line.strip()[:120]}")
    if hits:
        print(f"docs_words: {hits} line(s) name the plan or a count of the checks; move them to docs/checks.md", file=sys.stderr)
        return 1
    print(f"docs_words: {len(paths)} file(s), none names the plan or a count of the checks")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
