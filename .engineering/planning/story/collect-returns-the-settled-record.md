---
format: aep.planning-md/3
id: story:collect-returns-the-settled-record
kind: story
status: implemented
title: Collect answers with the settled record it removes
summary: Collect deletes the settled row and answers with the id only, so the client shows a row it read earlier and a reply can be deleted unseen.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T13:00:48Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T13:00:48Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T16:45:32Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`Collect` answers with the settled record it removes: the state, who answered, the decided
option, the reply and the refusal reason, read in the same transition that deletes the row. The
client shows and keeps what that answer carries, not a row it read earlier.

Before this story `inbox collect` read the sender's `Outbox`, kept a copy of the row, and then
sent `Collect` as a second request. The service deleted the settled row and answered with the
message id only. When the recipient answered or refused between the two requests, the collect
succeeded, the client showed the older pending row, and it removed its local outbox entry: the
reply was deleted without the sender ever receiving it. The memory backend holds the only copy
of a message.

## Acceptance

- The answer of a successful `Collect` carries the settled record: `state`, `answered_by`,
  `decided_option`, `reply`, `refusal`, as the row held them at the moment it was deleted.
- `inbox collect` prints the record of that answer and removes its outbox line only after it
  has that answer.
- An answer that arrives between a read of `Outbox` and the `Collect` is in the answer of the
  `Collect`; a `Collect` on a message that is not settled yet removes nothing (`not-settled`)
  and the message stays to collect.
- The shape is declared in `ess/` (a typed response of the command); the generated contract of
  the HTTP service carries it.
- A third principal still gets `unknown-message` and no record.

## Not in this story

A `Collect` whose answer is lost on the way (a broken connection after the service deleted the
row) still loses the record: a second `Collect` answers `unknown-message`. This is stated as a
limit. Keeping a receipt of a collected message until its `expires` changes when a message is
removed and is a decision of its own.

## Context

Reported by a source review of the release 0.2.0 (finding 3 of 11). The operator decided on
2026-11-22 that the fix is built first and published together with the finding
(approval-record:d13-collect-returns-the-settled-record).

## Scope

- `ess/domains/messaging.yaml`: `Collect` declares its response, the row of `Outbox` of the
  moment before the removal (20 members; `reply` is a `CollectedReply`). `generated/` and the
  projections follow; `website/src/generated/animation.json`.
- `crates/inbox-core/src/obligations.rs`, `src/lib.rs`: the hand-written `Collect` answers the
  record in the transition that removes the message, and decides its guards three-valued as
  the generated code of the commit before did.
- `crates/inbox-cli/src/collected.rs` (new), `src/commands.rs`, `src/output.rs`, `src/main.rs`:
  `inbox collect` prints the record and drops the outbox line only for the outcome `collected`
  with a record that decodes as the declared response, names the collected message, is
  `Answered` or `Refused` and states no member twice; otherwise it exits 3 and keeps the line.
- `crates/inbox-conformance/src/suite.rs`, `src/runner.rs`, `src/invariants.rs`, `src/lib.rs`,
  `src/core_target.rs`, `src/wire_target.rs`: the step `expect_direct_response`, unions in a
  response, and the refusal of a response that states a member twice.
- Cases: `crates/inbox-core/tests/collect_record.rs`, `tests/adversary_collect_record.rs`,
  `tests/adversary_collect_record_2.rs`, `tests/core/obligations.rs`;
  `crates/inbox-cli/tests/collect_record.rs`, `tests/adversary_collect_record.rs`,
  `tests/adversary_collect_record_2.rs`, `tests/support/mod.rs`;
  `crates/inbox-conformance/tests/runner.rs`, `tests/adversary.rs`,
  `tests/adversary_collect_record_2.rs`; one `mod` line each in the 3 `tests/main.rs`.
- `docs/design/inbox.md`, `docs/design/service.md`, `docs/checks.md`, `AGENTS.md`,
  `CHANGELOG.md`, `website/docs/first-use.md`, `website/docs/limits.md`.

## Measured

- 89 capabilities (86 generated, 3 obligations), 112 scenarios, 112 passed on the interpreted
  target and against the core; over HTTP 31 passed and 81 unsupported, as before.
- `task check`: exit 0, `574 tests run: 574 passed, 5 skipped`, the test step 18 s of
  its bound of 120 s, run on the branch with
  story:the-client-prints-no-control-sequence-of-another-principal on top.
