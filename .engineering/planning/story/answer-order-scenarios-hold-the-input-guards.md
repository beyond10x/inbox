---
format: aep.planning-md/3
id: story:answer-order-scenarios-hold-the-input-guards
kind: story
status: draft
title: The conformance suite holds the order of the input-guarded refusals of Answer
revision: 1
---
## Outcome

The conformance suite holds the declared order of the input-guarded refusals of `Answer`.

The mutation audit of 2026-11-22 (`ess verify conform mutate --class precedence-swap`,
ESS 0.97.1, 13 mutants, 1,060 s) left 2 mutants of `Answer` alive: with `not-sender` and
`no-authority` in each other's place, and with `no-authority` and `already-expired` in each
other's place, every scenario of the suite passes.

The design gives the first pair a reason: `not-sender` answers "before every other refusal of
the command", so a caller that holds another principal's signed reply learns nothing from the
answer. A core that answers `no-authority` first passes the suite today.

The same audit left 2 mutants of `PostMessage` alive (`no-authority` / `already-expired`,
`already-expired` / `request-without-shape`); those are
`story:post-order-scenarios-hold-the-input-guards`.

## Acceptance

`ess verify conform mutate --class precedence-swap` reports the 2 mutants of `Answer` as
killed, where it reports them as survived before (state transition: 2 mutants change from
survived to killed).

## Decisions

- 2 steps in authored scenarios, each with both refusals holding and the declared one
  expected: a reply whose `from` is not the caller and that names another `on_behalf_of`
  without `authority`; a reply with `on_behalf_of` and no `authority` whose `expires` is not
  after `created`.
- The audit's run time is over the test budget (1,060 s): it is not a step of the gate. The
  acceptance is run once, off the pool.

## Scope

- `ess/scenarios/`
- `docs/checks.md`
