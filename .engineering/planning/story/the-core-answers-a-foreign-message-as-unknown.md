---
format: aep.planning-md/3
id: story:the-core-answers-a-foreign-message-as-unknown
kind: story
status: implemented
title: The core answers a message between others as unknown
summary: Only the HTTP listener renames foreign-message, so an embedding of inbox-core tells a third principal that a message id exists.
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T14:22:37Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T14:22:37Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T16:50:00Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A client that is no party of a message gets from `inbox_core::handle` and
`handle_with_events` the answer of `unknown-message` for every command that addresses that
message: the same outcome, error and members as for an id no message carries. The rename
exists once, in `inbox-core`; the HTTP service calls it in place of its own `as_unknown`.

Before this story only the HTTP listener renamed the outcome. Through `inbox_core::handle` a
third principal got `foreign-message` from `MarkDelivered`, `MarkRead`, `Answer`, `Refuse` and
`Collect` for a message between others and `unknown-message` for no message, so an embedding
of `inbox-core` that handed the answer on told a third principal that a message id exists.
The summary of `foreign-message` in `ess/domains/messaging.yaml` says it cannot tell the two
apart.

## Acceptance

- For each of the 5 commands, a third principal gets byte for byte the same answer from
  `inbox_core::handle` for a message between others, in every state, and for an id no message
  carries. No event is handed back for either.
- The parties of a message get the answers they got before.
- The bytes of an HTTP answer are unchanged, and the listener holds no second copy of the
  rename.
- The model keeps `foreign-message` (`ess/` and `generated/` are unchanged), and the conformance
  run against the core still judges the model's order of refusals: 112 of 112.

## Context

Found by the adversary pass on story:the-core-applies-the-read-rule
(review-result:adversary-pass-1-on-the-core-read-rule, finding F1, pre-existing). Built on the
same branch and published with it (approval-record:d17-the-core-answers-a-foreign-message-as-unknown).

## Scope

- `crates/inbox-core/src/lib.rs`: `as_unknown`, the one copy of the rename, with the constants
  `FOREIGN_MESSAGE` and `UNKNOWN_MESSAGE`; `handle_as_specified`, the entry that gives the
  model's answer; `handle_with_events` is that entry and the rename, and `handle` goes
  through it.
- `crates/inbox-server/src/service.rs`: `as_unknown` calls `inbox_core::as_unknown` and gives a
  renamed answer the status 404; its own copy of the rename is removed.
- `crates/inbox-conformance/src/core_target.rs`: a command step calls `handle_as_specified`,
  so the run against the core still judges `foreign-message` and its place in the order of
  refusals.
- `crates/inbox-core/tests/foreign_unknown.rs` (5 cases), `crates/inbox-core/tests/main.rs`,
  `crates/inbox-core/tests/foreign_message.rs` (its driver calls `handle_as_specified`),
  `crates/inbox-core/tests/adversary_read_rule.rs` (its fixture is shared).
- `docs/design/inbox.md`, `docs/design/service.md`, `docs/checks.md`, `AGENTS.md`,
  `CHANGELOG.md`.
- Round 2, after the adversary pass: `handle_with_events` answers one of the 5 commands that
  address a message as `unknown-message`, with no event, when the context answers the inbox
  or the principal attribute of the caller as none or cannot read it and the model answered
  unmet. `crates/inbox-core/src/context.rs` states the rule in the doc of `CallerAttributes`.
  2 more cases in `foreign_unknown.rs`, and the 3 cases of the adversary in
  `crates/inbox-core/tests/adversary_foreign_unknown.rs`.
- `ess/` and `generated/` are unchanged.

## Stays

- A party known by one attribute only gets `unknown-message` for its own message where a
  branch needs the other attribute. `handle_as_specified` still gives such a client the
  unmet refusal of the model.

## Measured

- `task check` on the branch with main at release 0.3.0, 2026-11-22T16:49:41Z: exit 0,
  `555 tests run: 555 passed, 5 skipped`, the test step 17 s of its bound of 120 s; 112
  scenarios, 112 passed on the interpreted target and against the core, over HTTP 31 passed
  and 81 unsupported.
- review-result:adversary-pass-1-on-the-core-answers-a-foreign-message-as-unknown: 10 mutants
  killed, 1 finding, fixed in round 2 (3 mutants of the fix killed).
