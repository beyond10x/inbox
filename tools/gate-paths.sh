#!/bin/sh
# Decide whether a change needs `task check`.
#
# Prints `skip` when every changed path is a planning or prose file, else `run`.
# `.github/workflows/check.yml` calls it with the changed files of a pull request or a push, before
# any toolchain is installed, and `task gate-paths` (in `task check`) holds its answers to the
# cases below. POSIX shell only: the runner's `sh` is dash.
#
# A path is planning or prose when no step of `task check` reads it:
#
# - everything under `.engineering/` (the planning store);
# - a `*.md` file outside `website/` and `generated/`, except `README.md`.
#
# `website/` is read by `task docs-words` and `task animation-drift`, `generated/` by
# `task projections-drift` and `task drift`, and `README.md` by the acceptance case of
# `crates/inbox-cli/tests/acceptance.rs` (its "First use" section). A list with no path, a file
# that cannot be read or is not UTF-8, or a path this script cannot read as a plain path (quoted by
# git, absolute, or with a `..` component) answers `run`. Lines end at a line feed, a carriage
# return or both; an empty line names no path.
#
# usage: gate-paths.sh <file with one changed path per line>
#        gate-paths.sh --test

set -u

usage() {
    echo 'usage: gate-paths.sh <file with one changed path per line>' >&2
    echo '       gate-paths.sh --test' >&2
    exit 2
}

planning_or_prose() {
    case $1 in
        '"'* | /*) return 1 ;;
    esac
    case /$1/ in
        */../*) return 1 ;;
    esac
    case $1 in
        .engineering/*) return 0 ;;
        *.md) ;;
        *) return 1 ;;
    esac
    case $1 in
        website/* | generated/* | README.md) return 1 ;;
    esac
    return 0
}

# Reads one path per line on standard input and prints the answer.
decide() {
    seen=no
    while IFS= read -r path || [ -n "$path" ]; do
        [ -n "$path" ] || continue
        seen=yes
        if ! planning_or_prose "$path"; then
            echo run
            return 0
        fi
    done
    if [ "$seen" = yes ]; then echo skip; else echo run; fi
}

failed=0
cases=0

# case <name> <expected> <path>...
case_() {
    name=$1
    expected=$2
    shift 2
    cases=$((cases + 1))
    if [ "$#" -eq 0 ]; then
        answer=$(printf '' | decide)
    else
        answer=$(printf '%s\n' "$@" | decide)
    fi
    if [ "$answer" != "$expected" ]; then
        failed=$((failed + 1))
        echo "gate-paths: $name: $* answers $answer, expected $expected" >&2
    fi
}

test_cases() {
    case_ 'only planning and prose' skip \
        .engineering/planning/story/x.md docs/plan/stage-2.md
    case_ 'planning, prose and one source file' run \
        .engineering/planning/story/x.md docs/plan/stage-2.md crates/inbox-core/src/lib.rs
    case_ 'the documents at the root that no step reads' skip \
        AGENTS.md CHANGELOG.md docs/checks.md
    case_ 'an evidence record of the store, which is not Markdown' skip \
        .engineering/evidence/story/x/1.json
    case_ 'README.md, which an acceptance case reads' run README.md
    case_ 'a page of the site, which task docs-words reads' run website/docs/status.md
    case_ 'a generated document' run generated/docs/docs/index.md
    case_ 'the workflow itself' run .github/workflows/check.yml
    case_ 'this script' run tools/gate-paths.sh
    case_ 'no path' run
    case_ 'a path git quoted' run '"docs/pl\303\244ne.md"'
    case_ 'a path that leaves the tree' run docs/../crates/inbox-core/src/lib.md
    if [ "$failed" -ne 0 ]; then
        exit 1
    fi
    echo "gate-paths: $cases case(s), each answers as stated"
}

[ "$#" -eq 1 ] || usage
if [ "$1" = --test ]; then
    test_cases
    exit 0
fi
if [ ! -f "$1" ] || [ ! -r "$1" ] || ! iconv -f UTF-8 -t UTF-8 < "$1" > /dev/null 2>&1; then
    echo run
    exit 0
fi
tr '\r' '\n' < "$1" | decide
