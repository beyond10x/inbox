#!/bin/sh
# The test step of `task check`: run the tests under a bound on wall time and print the ten
# slowest tests, pass or fail.
#
# Runs <command> with its arguments. When it runs longer than <seconds>, the whole process group
# gets SIGTERM (`timeout`), SIGKILL 10 s later, and the step fails with exit 1 and says so on
# standard error. Else the step ends with the command's exit status. Before the run the report
# file is removed, so a report of an earlier run is never read; after the run the ten slowest
# <testcase> elements of the JUnit report the command wrote are printed, slowest first, with their
# time, binary and name. A run that wrote no report fails with exit 1.
#
# `task check` calls it with 120, the report `.config/nextest.toml` names
# (`target/nextest/default/junit.xml`) and `cargo nextest run`, after the test binaries are built,
# so the bound counts the run and not the build. The 60 s bound of one test is nextest's own
# (`slow-timeout` in `.config/nextest.toml`). POSIX shell and `timeout` only: the CI runner's `sh`
# is dash. `crates/inbox-xtask/tests/tools/test_step.rs` holds it to its cases.
#
# usage: test-step.sh <seconds> <JUnit report> <command> [<argument>...]

set -u

usage() {
    echo 'usage: test-step.sh <seconds> <JUnit report> <command> [<argument>...]' >&2
    exit 2
}

[ $# -ge 3 ] || usage
seconds=$1
report=$2
shift 2
case $seconds in
    '' | *[!0-9]* | 0*) usage ;;
esac

slowest() {
    awk '
        function attribute(line, name,    start, rest) {
            start = index(line, " " name "=\"")
            if (start == 0) return ""
            rest = substr(line, start + length(name) + 3)
            return substr(rest, 1, index(rest, "\"") - 1)
        }
        function text(value) {
            gsub(/&lt;/, "<", value)
            gsub(/&gt;/, ">", value)
            gsub(/&quot;/, "\"", value)
            gsub(/&apos;/, "\047", value)
            gsub(/&amp;/, "\\&", value)
            return value
        }
        /<testcase / {
            printf "%s\t%s %s\n", attribute($0, "time"), text(attribute($0, "classname")), text(attribute($0, "name"))
        }
    ' "$1" | sort -t "$(printf '\t')" -k1,1 -n -r | head -n 10 | awk -F '\t' '{printf "%10.3f s  %s\n", $1, $2}'
}

rm -f "$report"
start=$(date +%s)
timeout --kill-after=10 "$seconds" "$@"
status=$?
took=$(($(date +%s) - start))
over=no
if { [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; } && [ "$took" -ge "$seconds" ]; then
    over=yes
fi

if [ -f "$report" ]; then
    echo 'test-step: the 10 slowest tests'
    slowest "$report"
else
    echo "test-step: the run wrote no JUnit report at $report" >&2
    [ "$status" -eq 0 ] && status=1
fi
echo "test-step: the run took $took s of its bound of $seconds s, exit $status"

if [ "$over" = yes ]; then
    echo "test-step: the run took longer than its bound of $seconds s and was ended" >&2
    exit 1
fi
exit "$status"
