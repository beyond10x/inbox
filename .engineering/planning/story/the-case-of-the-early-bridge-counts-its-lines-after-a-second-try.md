---
format: aep.planning-md/3
id: story:the-case-of-the-early-bridge-counts-its-lines-after-a-second-try
kind: story
status: draft
title: The case of the early bridge counts its lines after a second try
summary: bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying counts the start lines before the second try, so a line on every try passes it.
revision: 1
---
## Outcome

The case `bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying`
fails when the bridge prints its start line on every try.

Today the case counts the lines of the bridge's standard error about 27 ms after the first
line. The first pause of the bridge is 100 ms, so the count is taken before the second try,
and "one line for the run of failures" passes when the line is printed on every try (a
mutant of `say_once` in `crates/inbox-cli/src/wake.rs`: `if *failures == 0` replaced by `if
true`). The case
`adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`
is red under that mutant: it waits 1600 ms before it counts.

## Acceptance

- The case counts its lines after at least 2 tries of the bridge (state transition: with the
  start line printed on every try the case moves from green to red).
- The case stays under 5 s; `task check` is green.

## Context

Note 2 of review-result:adversary-closed-port-pass-1 (origin undecided: the story of that
pass changed only where the case gets its address). There before that story. It waits until
the planned waves are delivered.
