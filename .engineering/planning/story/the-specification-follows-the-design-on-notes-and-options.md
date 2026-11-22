---
format: aep.planning-md/3
id: story:the-specification-follows-the-design-on-notes-and-options
kind: story
status: active
title: A note cannot be refused and a decision has an option
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:58:01Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T04:58:01Z", actor: "agent:inbox", revision: 3}
---
## Outcome

The specification follows the design in 2 places where a review of the design against the
specification found them apart (hardening, 2026-11-22).

- **A note cannot be refused.** The design removes a note by `MarkRead` or `Expire` only
  (`docs/design/inbox.md`, "When a message is removed"). `Refuse` has no guard on the kind
  (`ess/domains/messaging.yaml:670` on `8e9c3ab`), so the owner can refuse a note, and the
  sender then removes it with `Collect`. `Answer` already refuses a note with `note`
  (`NotAnswerable`). `Refuse` gains the same outcome.
- **A decision has at least one option.** `decision-without-options` refuses only an absent
  `decision` (`:410`); a decision whose `options` list is empty is stored and can never be
  answered with a letter. `DecisionAsk.options` gains the invariant of at least 1 element, or
  the guard of `decision-without-options` also refuses the empty list, whichever synthesis
  keeps its scenarios with.

## Acceptance

`Refuse` of a note answers `note` (`NotAnswerable`, HTTP 422 class of the generated
contract) on the interpreted target, on the core and over HTTP, and the note stays as it was;
a post of a decision with an empty `options` list is refused `ShapeMismatch` and nothing is
stored (state transition: both inputs were accepted before and are refused after).

## Decisions

- Both are narrowings for a caller of 0.2.0 (`refusal-added`); the changelog says so under
  `### Changed`.
- At least 1 option, not 2: the design says "lettered options" and gives no number.

## Scope

- `ess/domains/messaging.yaml`, `ess/scenarios/`, the scenario source
- `generated/`, `crates/inbox-core/`, `crates/inbox-conformance/`
- `docs/design/inbox.md`, `CHANGELOG.md`, `docs/checks.md`
