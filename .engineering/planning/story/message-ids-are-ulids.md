---
format: aep.planning-md/3
id: story:message-ids-are-ulids
kind: story
status: draft
title: A message id the specification admits is a ULID
revision: 1
---
## Outcome

A `MessageId` and a `ThreadId` that the specification admits is a ULID.

`docs/design/inbox.md`: "A message id is a ULID and not a secret." The types
`inbox.messaging.MessageId` and `inbox.messaging.ThreadId` declare the Crockford alphabet and
`value.count == 26`. A ULID is 128 bits, so its first character is `0` to `7`. The invariant
admits a first character of `8` to `Z`: a text of 130 bits that no ULID library decodes.

Found by the design review of 2026-11-22 (hardening, technique 8), finding 12. Severity: note.

## Acceptance

A `PostMessage` whose `message_id` is 26 Crockford characters starting with `8` is refused at
decoding (`broken_constraint`), where it is stored today; one starting with `7` is stored
(state transition: such an id changes from admitted to refused).

## Decisions

- The rule is an invariant on both newtypes over the first character. `ess verify diff` rates
  it against release 0.2.0; a narrowing is acknowledged in the same change.
- The client (`crates/inbox-cli/src/ids.rs`) writes ULIDs already; no client change.

## Scope

- `ess/domains/messaging.yaml`, `generated/`
- `crates/inbox-core/tests/`, `CHANGELOG.md`
