---
format: aep.planning-md/3
id: story:suite-answers-and-refuses-from-every-state
kind: story
status: draft
title: The conformance suite answers and refuses a message from every state
revision: 1
---
## Outcome

The conformance suite answers and refuses a message from every state the lifecycle allows.

The mutation audit of 2026-11-22 (`ess verify conform mutate --class from-drop`, ESS 0.97.1,
specification of `8e9c3ab`) left 3 of 6 mutants unwitnessed: with `Delivered` dropped from
`from:` of the transition `answer`, with `Delivered` dropped from `refuse`, and with `Read`
dropped from `refuse`, no scenario of the suite changes its result. Synthesis cannot reach
those states (`ESS-SYNTH-004`), and no authored scenario answers a message that was delivered
and not read, refuses one that was delivered, or refuses one that was read.

A core that refuses `Answer` on a delivered message, or `Refuse` on a delivered or a read
message, passes the suite today.

## Acceptance

`ess verify conform mutate --class from-drop` on the interpreted target exits 0 with 6 of 6
mutants killed, where it exits 3 with 3 unwitnessed before (state transition: the 3 mutants
change from unwitnessed to killed).

## Decisions

- The 3 cases are steps of authored scenarios that already bring a message to the state
  (`post-read-answer`, `refused-message-waits-for-the-sender`), or one new scenario.

## Scope

- `ess/scenarios/`, the scenario source
- `crates/inbox-conformance/`, `docs/checks.md`
