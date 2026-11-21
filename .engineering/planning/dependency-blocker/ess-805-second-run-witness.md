---
format: aep.planning-md/3
id: dependency-blocker:ess-805-second-run-witness
kind: dependency-blocker
status: cleared
title: story:dedup-key-scope waits for an ESS release that fixes beyond10x/ess#805
relations:
- blocks: story:dedup-key-scope
- blocks: story:post-does-not-tell-a-message-id-exists
withholds: test_result
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-11-21T20:08:05Z", actor: "agent:inbox", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## What is blocked

story:dedup-key-scope, accepted for stage 2, wave 6. Its unit was started on 2026-11-21 and
stopped before any commit.

## What blocks it

A defect of ESS synthesis, reported as beyond10x/ess#805. With the rule the story settled
(`duplicate-key` compares the key among the stored messages whose `from` is the `from` of the
post: `where: {all: [dedup_key == input.dedup_key, from == input.from]}`), ESS 0.96.0 writes the
second run of the generated scenario `PostMessage/outcome/decision-without-options` with a fresh
`from` under a caller it does not name. The guard `not-sender` answers first, and ESS's own
interpreted target fails the scenario: 96 scenarios, 95 passed, 1 failed. `task check` exits at
`task conform-interpreted`.

Measured on the model of this repository: 7 other spellings of the same rule fail the same way
(members reordered, operands flipped, a nested `all`, an `any`, a `not`, a doubled `any`, the
`from` comparison alone). The selector cannot read the caller (`ESS-COMMAND-003`). The 2
functions the report names read the same in the ESS source 2 commits after the tag 0.97.0.

## What was decided

The host operator's session, 2026-11-21, to the coordinating session, as received:

> host-operator: A. Hold story:dedup-key-scope until an ESS release fixes the synthesis defect;
> wave 6 closes with units 1 and 3. Reason: the service is on loopback and no real session uses
> it, so the open key probe has no one to reach today, and B would put a second meaning of the
> key scope on the wire that we then change again. The gate stays closed on this story: no
> listener outside loopback, no NATS wave, no deployment, no real session before it lands. Write
> that on the limits page. File the ESS defect today with your probe (the 8 spellings, the two
> source sites) and send the issue number to me and to `ess`; I order the fix there. When the
> ESS release with the fix is out, take the story up without asking.

Option B, not taken, was a scope by `on_behalf_of`, which passes the interpreted target and
leaves a delegate sharing the keys of the principal it speaks for.

## What clears it

An ESS release in which the model of beyond10x/ess#805 passes the interpreted target, and this
repository pinned to it. The coordinating session then takes story:dedup-key-scope up without
asking. Until then: no listener outside loopback, no NATS backend wave, no deployment and no
use by a real session.

## What the stopped unit left

Nothing is committed. The unit had removed the ignore attribute of the acceptance case
(`adv_a_third_principal_cannot_take_another_senders_dedup_key`, red on the base with
`duplicate-key` where `posted` is expected), written the rule into the model and regenerated.
The core's comparison, the core cases, the authored scenario of 2 senders under one key and
decision 6 of `docs/design/inbox.md` were not started.

## Also blocked

story:post-does-not-tell-a-message-id-exists, by the host operator's decision of 2026-11-21: it
waits for the same ESS release and runs in one wave with story:dedup-key-scope. Whether its rule
meets the defect is not measured; a rule that compares the message id among the sender's own
messages would read `input.from` in a row guard, the construct the defect is about.
