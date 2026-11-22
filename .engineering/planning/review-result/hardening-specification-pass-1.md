---
format: aep.planning-md/3
id: review-result:hardening-specification-pass-1
kind: review-result
status: active
title: 'Hardening pass 1 on the specification ess/ (1870cd6): 24 findings (8 warnings, 16 notes), 2 introduced, 22 there before the wave'
relations:
- reviews: story:the-specification-follows-the-design-on-notes-and-options
- reviews: story:no-unmapped-rule-in-the-specification
revision: 1
---
## What was hardened

The specification `ess/` of beyond10x/inbox (domain `inbox.messaging`, `format: ess/37`), with
ESS 0.97.1 and the `ess:hardening` skill 0.26.0, on 2026-11-22, by the coordinating session.
The suite was green before the techniques ran: `task check` exit 0 on `3d11f9f`, 112 scenarios,
interpreted 112 of 112, core 112 of 112.

Two revisions were read. `8e9c3ab` is `main` before the specification wave. `1870cd6` is `main`
after it; its specification is the one of `88c92d2` plus one scenario step added in `3d11f9f`.
Line numbers in the findings are those of `1870cd6`.

Order run: design review (8), specification diff (7), mutation audit (1), guard analysis (6).
Not run: random command sequences (2), caller replay (3), determinism (4), metamorphic
relations (5). The reasons are in the last section.

## 8. Design review

- Read: `docs/design/inbox.md`, `docs/design/signing.md`, `docs/design/service.md` and all of
  `ess/` at `8e9c3ab`, by one agent on the brief of the skill.
- Planted defect: the invariant `value.utf8_bytes <= 65536` of `inbox.messaging.Body` deleted in
  a copy. `ess specify validate` accepted the copy (exit 0). The review returned `missing` with
  the design line and the specification line, so the review caught what validation did not.
- Result on `8e9c3ab`: 19 findings. 2 are closed by `88c92d2`: `Refuse` of a note now answers
  `note`, and a decision with an empty `options` list now answers `decision-without-options`.
  14 stand and are in the block below. 3 are held outside this record until the operator decides
  how they are published.
- The 14: 1 rule of the verifier that the condition of `signature-invalid` omits, 1 invariant
  wider than the design's type, 1 summary that contradicts the types, 5 statements the design
  leaves open, 6 declarations of the specification without a design statement.
- Drafts filed for them: `story:signature-invalid-names-every-check-of-the-verifier`,
  `story:message-ids-are-ulids`, `story:design-decides-refusal-signature-and-reply-id`,
  `story:design-states-what-the-specification-declares`.

## 7. Specification diff

- Command: `ess verify diff --from <a>/ess --to <b>/ess --fail-on breaking-or-unknown --format json`.
- Planted defect: one declaration changed in a copy of `8e9c3ab`. The diff reported 1 change
  rated `unknown` and exited 4. The planted text was not kept.
- Green: `8e9c3ab` against itself, 0 changes, exit 0. Release 0.2.0 against `8e9c3ab`, 0 changes,
  exit 0.
- Release 0.2.0 against `88c92d2`: exit 4, 12 changes, 8 rated `unknown` and 4 `compatible`.
  The 8: `Expire/expired` payload, set effect and subject; the conditions of
  `PostMessage/decision-without-options` and `PostMessage/unknown-option`; the added outcome
  `Refuse/note`; the fields `to` and `thread` added to `MessageExpired`.
- No step of `task check` runs this command, and no acknowledgement file exists, so the next
  release would carry the 8 changes without a recorded decision. Draft:
  `story:spec-diff-gate-against-the-last-release`.

## 1. Mutation audit

- Command, one class per run, outside the build pool, at most 1,800 s per class:
  `ess verify conform mutate --path ess --target interpreted --scenarios ess --class <class> --report-out <file>`.
- Each killed mutant is a planted defect the suite failed on. The counts are the tool's own lines.
- On `8e9c3ab`:
  - `from-drop`: 6 mutants, 3 killed, 3 unwitnessed.
  - `transition-to`: 2 killed, 2 of 7 sites scored.
  - `guard-boundary`: 19 killed, 19 of 19 sites scored.
  - `sets-retarget`: 7 killed, 7 of 19 sites scored.
  - `emit-drop`: 3 killed, 9 stillborn.
  - `grant-drop`: 1 killed, 8 stillborn.
  - `order-flip`, `grant-add`: no site.
- On `88c92d2`:
  - `guard-negate`: 27 mutants, 26 killed, 1 stillborn.
  - `guard-connective`: 18 mutants, 16 killed, 1 stillborn, 1 unwitnessed.
  - `sets-drop`: 3 killed, 3 of 29 sites scored.
  - `precedence-swap`: 13 mutants, 6 killed, 5 survived, 2 equivalent.
  - `emit-swap`: 10 killed, 2 sites unavailable (no other event has the same fields).
  - `error-swap`: no result within 1,800 s.
- The 3 unwitnessed `from-drop` mutants are gaps of the suite: `Delivered` removed from the
  `from` list of `answer`, and `Delivered` or `Read` removed from that of `refuse`. With each
  applied by hand, `ess specify validate` refuses none of the 38 authored scenarios, so the
  verdict is not the dropped-scenario case described under `guard-connective` below. No scenario
  answers a delivered message or refuses a delivered or a read one. Draft:
  `story:suite-answers-and-refuses-from-every-state`.
- The 5 surviving `precedence-swap` mutants: `Answer` `not-sender`/`no-authority` and
  `no-authority`/`already-expired` (draft `story:answer-order-scenarios-hold-the-input-guards`);
  `PostMessage` `no-authority`/`already-expired` and `already-expired`/`request-without-shape`
  (draft `story:post-order-scenarios-hold-the-input-guards`); `Refuse` `not-recipient`/`note`.
- The `Refuse` order was introduced by `88c92d2`. `3d11f9f` added a step to
  `note-takes-no-answer`: the sender of a note refuses it and is answered `not-recipient`.
  Re-measured on `1870cd6` (939 s): 13 mutants, 7 killed, 4 survived, 2 equivalent; the `Refuse`
  mutant is killed by `note-takes-no-answer`. The 4 others stand.
- The 2 equivalent `precedence-swap` mutants are of `PostMessage` (ESS-MUTATE-005):
  `request-without-shape`/`decision-without-options`, which no input satisfies together, and
  `decision-without-options`/`unknown-option`, which both refuse with `ShapeMismatch` and no
  payload.
- The unwitnessed `guard-connective` mutant (`decision-without-options`, `or` becomes `and`) is a
  false positive of the technique: the authored scenario `envelope-must-fit-its-kind` refuses the
  mutant at validation (ESS-AUTHOR-041), the tool drops that scenario, and the remaining suite
  asks what the baseline's asks (ESS-MUTATE-004). The behaviour is documented in ESS.

## 6. Guard analysis

- Tool: a one-off script over the compiled IR (`ess specify compile --path ess --format json`),
  not kept in the repository. It enumerates every assignment of the atoms of each command's
  guards and reports dead outcomes, unanswered assignments, pairs whose order decides the answer,
  and the states each kind of message reaches.
- Planted defect: in a copy, the outcome `MarkRead/read` moved ahead of `handled`. The script
  reported `dead=['handled']`.
- `8e9c3ab`: 0 dead, 0 unanswered, 2 forbidden states reached: a note reaches `Refused`. This is
  the design finding that `88c92d2` closed.
- `88c92d2`: 60 outcomes, 8,863,176 assignments, 0 dead, 0 unanswered, 0 forbidden states
  reached. A note reaches `Sent` and `Delivered` only.
- No finding stands from this technique.

## Not run

- Random command sequences (2) and the two-run determinism check (4): the explorer ships in the
  generated Go and TypeScript packages, and this repository builds a Rust core. A source scan
  found no clock read and no random source in `crates/inbox-core`; a scan has no planted defect
  behind it and is not counted as the technique.
- Caller replay (3): no recorded traffic of a real caller exists.
- Metamorphic relations (5): needs a reference model driven by own traces; none is built.
- `error-swap` of the mutation audit: no result, as above.

```findings
- file: ess/domains/messaging.yaml
  line: 441
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The signature-invalid condition of PostMessage and Answer (also line 611) omits two checks of docs/design/signing.md, that the key entry's principal is the envelope's from and that the envelope has a canonical form.
- file: docs/design/inbox.md
  line: 31
  category: spec-gap
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The design does not say whether an owner's refusal counts as a reply that must be signed; the specification's Refuse carries no signature and no authority.
- file: docs/design/inbox.md
  line: 113
  category: spec-gap
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: For Answer the design puts not-sender before every other refusal (line 107) and also foreign-message before not-sender (line 113); the specification follows line 107.
- file: docs/design/inbox.md
  line: 125
  category: spec-gap
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The design does not say whether a reply_id belongs to the one message-id name space; the specification does not check it.
- file: docs/design/inbox.md
  line: 40
  category: spec-gap
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The arrow chain Sent, Delivered, Read, Answered or Refused may mean a strict sequence, while the specification lets a message skip Delivered and Read.
- file: docs/design/inbox.md
  line: 43
  category: spec-gap
  severity: note
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: The phrase "lettered options, a recommended one" may make recommended_option mandatory, while the specification declares it Optional.
- file: ess/domains/messaging.yaml
  line: 30
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The MessageId and ThreadId invariants (also line 39) accept 26-character Crockford strings whose first character is above 7, which are not ULIDs as the design states.
- file: ess/domains/messaging.yaml
  line: 158
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The summaries of Reply and Answer give a reply a kind and a to that neither type carries and that the design says a reply does not have.
- file: ess/domains/messaging.yaml
  line: 185
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The reopen transition and the reopened outcome of OpenInbox have no design statement.
- file: ess/domains/messaging.yaml
  line: 339
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The repeat-without-effect outcomes already-open, already-closed, already-delivered and already-read have no design statement.
- file: ess/domains/messaging.yaml
  line: 646
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The shape-mismatch check of a reply against reply_shape and the decision's options has no design statement.
- file: ess/domains/messaging.yaml
  line: 788
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The ten events and their payloads have no design statement, apart from what MessageExpired carries.
- file: ess/domains/messaging.yaml
  line: 836
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: read_your_writes consistency on all nine views has no design statement.
- file: ess/domains/messaging.yaml
  line: 71
  category: spec-gap
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The Adapter variants, the address format, the ReplyShape variants, request_ref and the 24-hour client default for expires have no design statement.
- file: Taskfile.yml
  category: spec-gap
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: No gate step runs ess verify diff against the last release, so the 8 changes rated unknown between release 0.2.0 and main are acknowledged nowhere.
- file: ess/domains/messaging.yaml
  line: 230
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The from-drop mutant that removes Delivered from the answer transition is unwitnessed, so no scenario answers a delivered message.
- file: ess/domains/messaging.yaml
  line: 232
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The from-drop mutants that remove Delivered or Read from the refuse transition are unwitnessed, so no scenario refuses a delivered or a read message.
- file: ess/domains/messaging.yaml
  line: 599
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The precedence-swap mutant of Answer that swaps not-sender and no-authority survives, so no scenario holds that order.
- file: ess/domains/messaging.yaml
  line: 602
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The precedence-swap mutant of Answer that swaps no-authority and already-expired survives, so no scenario holds that order.
- file: ess/domains/messaging.yaml
  line: 398
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The precedence-swap mutant of PostMessage that swaps no-authority and already-expired survives, so no scenario holds that order.
- file: ess/domains/messaging.yaml
  line: 405
  category: missing-scenario
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: The precedence-swap mutant of PostMessage that swaps already-expired and request-without-shape survives, so no scenario holds that order.
- file: ess/domains/messaging.yaml
  line: 408
  category: equivalent
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: The precedence-swap mutant of PostMessage that swaps request-without-shape and decision-without-options is equivalent, because no input satisfies both guards.
- file: ess/domains/messaging.yaml
  line: 414
  category: equivalent
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The precedence-swap mutant of PostMessage that swaps decision-without-options and unknown-option is equivalent, because both outcomes refuse with ShapeMismatch and no payload.
- file: ess/domains/messaging.yaml
  line: 414
  category: false-positive
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: The guard-connective mutant of decision-without-options is reported unwitnessed although the authored scenario envelope-must-fit-its-kind refuses it at validation.
```
