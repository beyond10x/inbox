---
format: aep.planning-md/3
id: story:signature-invalid-names-every-check-of-the-verifier
kind: story
status: draft
title: signature-invalid names every check of the verifier
revision: 1
---
## Outcome

The outcome `signature-invalid` of `PostMessage` and of `Answer` names every refusal of the
verifier that the design gives it.

`docs/design/signing.md` ("In this order, the first failure refuses") lists 6 checks. The
`external:` condition of `signature-invalid` in `ess/domains/messaging.yaml` names 3 of them:
the key is unknown, the key is lapsed, the signature does not verify. It omits:

- check 3, "The entry's `principal` is the envelope's `from`." This is the check that binds
  `key_id` to the sender: a signature under another principal's key verifies and is refused.
- check 4, `signature.value` is a canonical signature encoding;
- check 5, the envelope has a canonical form.

The verifier is `crates/inbox-verify` (check 5 in `src/canonical.rs`); this story does not
change it. The text a reader of the specification, of the OpenAPI document and of the
generated pages gets is short by 3.

Found by the design review of 2026-11-22 (hardening, technique 8), finding 2.

## Acceptance

The `external:` text of `signature-invalid` on both commands, and the summary of the error
`SignatureInvalid`, name the 6 checks of `signing.md`; `task check` exits 0 with the
projections and the generated crate written again (state transition: the condition text
changes, `ess verify diff` rates it a summary or condition-text change).

## Decisions

- Text only: no outcome is added and no order changes.

## Scope

- `ess/domains/messaging.yaml`
- `generated/`, `docs/design/signing.md` (a line that names the outcome)
