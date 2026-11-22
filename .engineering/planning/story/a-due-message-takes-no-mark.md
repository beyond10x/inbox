---
format: aep.planning-md/3
id: story:a-due-message-takes-no-mark
kind: story
status: implemented
title: A message past its expires takes no read mark and no delivery mark
summary: MarkRead and MarkDelivered act on a due message, so the event the sender of a due note gets depends on when the expiry task ran.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T18:27:24Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T18:27:25Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T20:31:19Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

A message whose `expires` has passed takes no `MarkRead` and no `MarkDelivered`, whether or not
the expiry task has reached it. The sender of a due note gets the same event whenever the task
runs.

Today both commands act on a due message. A due note that its recipient reads before the expiry
task runs is removed with `MessageHandled`; the same note read after the task ran was removed
with `MessageExpired`. The stored outcome is the same (the note is gone); the event the sender
gets depends on when the task ran.

## Acceptance

- `MarkRead` and `MarkDelivered` on a message whose stored `expires` is at or before the
  service's clock reading change no row and publish no event (state transition: a due message
  in `Sent`, `Delivered` or `Read` stays as it is until the expiry task decides it).
- One case runs the two orders (mark then sweep, sweep then mark) on a due note and finds the
  same events for the sender.
- A caller that is no party of the message still gets `unknown-message`.
- The rule is declared in `ess/`; the core and the HTTP service give the same outcome.

## Context

Found by the adversary pass on story:a-due-message-takes-no-answer
(review-result:adversary-pass-1-on-a-due-message-takes-no-answer, finding 5). It existed before
that story. A draft: accepting it is the operator's act.

Accepted and implemented on the order of 2026-11-22 for the 4 gate-stability drafts
(approval-record:d30-a-due-message-takes-no-mark).

## Decisions

- Both commands get the outcome `deadline-passed` (502, error `Expired`), the outcome a due
  answer already has.
- The wait on `Pending` changes with it: a row its owner can no longer read or mark does not
  end a wait. Without that, a loop of `wait` and `read` gets the due row back at once until the
  sweep (adversary pass 1).
- A wait that ends at its bound or at the stop reads the view once more, and a sweep that
  expired a message wakes every wait (adversary pass 2).
- `inbox_core::due` is public: the marks and the wait share one comparison.
- `inbox read` on a due message now exits 1 with `deadline-passed`: a changed answer for a
  client, named as such in CHANGELOG.md.

## Scope

- `ess/domains/messaging.yaml`, 2 authored scenarios under `ess/scenarios/`, `generated/`.
- `crates/inbox-core/src/context.rs`, `crates/inbox-core/src/lib.rs`.
- `crates/inbox-server/src/service.rs` (`waited`), `listener.rs` (`sweep_now`), `clock.rs`.
- `crates/inbox-conformance/src/`, `unsupported.tsv`.
- Cases in `crates/inbox-core/tests/`, `crates/inbox-server/tests/`, `crates/inbox-cli/tests/`.
- `docs/design/inbox.md`, `docs/design/service.md`, `docs/checks.md`, `website/docs/`.

## Measured

- 23 new cases: 8 in `inbox-core` (194 cases), 14 in `inbox-server` (92 cases), 1 in `inbox-cli`
  (144 cases); 2 authored scenarios, 119 scenarios in all, 87 unsupported over HTTP.
- Adversary pass 1: 4 findings, closed. Pass 2: 4 findings, closed.
- `task check` on the branch merged with `main`, 2026-11-22T20:29:34Z to 2026-11-22T20:30:34Z, load average 29 to 42
  on 20 CPUs: exit 0, `691 tests run: 691 passed, 5 skipped`, the test step 23 s of
  its bound of 120 s.
