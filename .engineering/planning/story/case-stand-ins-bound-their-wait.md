---
format: aep.planning-md/3
id: story:case-stand-ins-bound-their-wait
kind: story
status: draft
title: Every stand-in a case file writes bounds its wait for a connection
revision: 1
---
## Outcome

Every stand-in service a case file of the workspace writes for itself bounds its wait for a
connection, as the shared stand-in of `crates/inbox-cli/tests/support/mod.rs` does since
story:test-step-enforces-its-budget: a case whose client sends nothing fails within seconds with
a message that names what did not arrive.

## Acceptance

For each of the 6 places below, a case whose client never connects fails within 15 s with a
message naming the missing connection, where today only the runner's kill at 60 s ends it.

## Context

Found by the implementor of story:test-step-enforces-its-budget on 2026-11-22, existing before
that story. These stand-ins call `accept()` or iterate `incoming()` with no bound:

- `crates/inbox-cli/tests/adversary.rs:70`
- `crates/inbox-cli/tests/adversary2.rs:190`
- `crates/inbox-cli/tests/adversary_w8_repost.rs:86`
- `crates/inbox-cli/tests/adversary_w8_repost2.rs:99`
- `crates/inbox-conformance/tests/wire.rs:207`
- `crates/inbox-conformance/tests/adversary3.rs:54`

Since that story cargo-nextest kills such a case at 60 s and names it, so none of them can hold
the gate longer. What is missing is the message: a `TIMEOUT` line says which case, not which
connection.

## Scope

- the 6 files above
- `crates/inbox-cli/tests/support/mod.rs` (`accept_within`, made usable by the case files)
- `crates/inbox-conformance/tests/` (a helper of the same kind, if 2 files need it)
