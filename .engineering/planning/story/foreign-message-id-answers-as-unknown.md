---
format: aep.planning-md/3
id: story:foreign-message-id-answers-as-unknown
kind: story
status: implemented
title: A third principal gets one answer for a message between others and for no message
summary: not-recipient (403) against unknown-message (404) tells any authenticated client whether a message id exists.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-http-service-pass-1
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: inferred
  path: crates/inbox-conformance/unsupported.tsv
- confidence: cited
  path: crates/inbox-server/src
- confidence: cited
  path: crates/inbox-server/tests/adversary.rs
- confidence: inferred
  path: docs/design/inbox.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: ess/domains/messaging.yaml
- confidence: cited
  path: ess/scenarios
- confidence: cited
  path: generated
- confidence: cited
  path: tools/scenarios.py
- confidence: cited
  path: website/docs/limits.md
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T15:32:33Z", actor: "agent:inbox", revision: 16}
- {from: "proposed", to: "active", at: "2026-11-21T15:32:33Z", actor: "agent:inbox", revision: 17}
- {from: "active", to: "implemented", at: "2026-11-21T17:17:28Z", actor: "agent:inbox", revision: 19, decided_on: {"recorded":{"test_result":1,"review_outcome":4,"verification":1}}}
---
## Outcome

A principal that is neither the recipient nor the sender of a message gets the same answer for
the id of a message between two others as for an id no message carries. Today `read`, `collect`,
`refuse`, `answer` and `mark-delivered` answer `unknown-message` (404) for an id nobody used and
`not-recipient` (403) for a message addressed to another inbox, so any authenticated client that
knows or guesses a message id learns whether that message exists.

## Acceptance

A third principal that sends `read` for the id of a message between two others gets the status
and the outcome it gets for an id no message carries, where before this story the first is 403
`not-recipient` and the second 404 `unknown-message`.

## Context

Found by the adversary of story:http-service in stage 2, wave 3
(review-result:adversary-http-service-pass-1, finding F7; the case is
`adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message` in
`crates/inbox-server/tests/adversary.rs`, ignored until this story). The two outcomes and their
order are the specification's (`ess/domains/messaging.yaml`, the commands with `unknown_instance`
then `to != caller.inbox`), so the finding was there before the unit.

## Decisions

Accepted on 2026-11-21 (approval-record:wave-6-security-drafts-accepted): the host operator's
session accepted this draft together with story:post-from-is-the-caller and story:dedup-key-scope,
to run as wave 6 after the 5 waves of the stage 2 plan, with the gate of those waves and no release.

A message id is a ULID, which carries its creation time and is not a secret.

Settled by the coordinator on 2026-11-21:

- The model names the case and the listener gives it the answer. `MarkDelivered`, `MarkRead`,
  `Answer`, `Refuse` and `Collect` gain an outcome `foreign-message` for a caller that is
  neither the recipient (`to != caller.inbox`) nor the sender (`from != caller.principal`) of
  the stored message, with the error `inbox.messaging.UnknownMessage`, ahead of `not-recipient`
  and `not-sender`. The listener answers that one outcome with the status and the body of
  `unknown-message`. Who is a party is decided in the model only; the listener holds no
  comparison of its own.
- The model alone cannot give the answer. Probed on a copy of the model with ESS 0.96.0: a
  second outcome named `unknown-message` is refused (`ESS-COMMAND-006`,
  `duplicate_declaration`), and a caller-guarded outcome with the error `UnknownMessage` is
  answered with the status 403 and its own name in the body. The one construct ESS documents
  for two outcomes with one answer (`alike:`) needs an `ess-http/1` binding, which this
  repository does not have, and a body without the outcome name, which the client and the
  conformance run read.
- The sender still gets `not-recipient` from the 4 commands of the recipient, and the recipient
  still gets `not-sender` from `Collect`: each of them knows the message exists.
- This story's unit runs last of the 3.

## Scope

Derived 2026-11-21 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `ess/domains/messaging.yaml`, the party guards of MarkDelivered :472-476,
  MarkRead :495-499, Answer :556-560, Refuse :605-609, Collect :633-637 — cited
- **Files:** `tools/scenarios.py:557-561`: the 5 steps where `OTHER` (neither party) expects
  `not-recipient`/`not-sender`. `tools/scenarios.py:129-131`: the COVERS claims that rest on those
  steps — cited
- **Files:** `ess/scenarios/only-the-owner-handles-its-inbox.yaml:3-8,68,81,104,118,131`, rewritten
  by `task scenarios` (`Taskfile.yml:102-105`) — cited
- **Files:** `crates/inbox-server/tests/adversary.rs:298-327`: the ignore at :299 is removed. The
  case compares `(status, outcome)` at :321-327 — cited
- **Regenerated, `task generate`:** `generated/rust/inbox/` (Cargo.toml, PLAN.md, plan.json, every
  `src/` file, since each carries the source digest; `Cargo.lock` kept by the task; its
  `.ess-output/` is not checked in, AGENTS.md:20). Both earlier model changes touched 25 files here:
  `git show --stat d35d455 29575e0` — cited
- **Regenerated, `task projections`:** `generated/openapi/`, `generated/asyncapi/`,
  `generated/schema/`, `generated/docs/`, each with its `.ess-output/state.json`
  (`Taskfile.yml:49-53`; 13 files in each earlier commit, 38 files per commit in total) — cited
- **Also likely:** more content than the 2 earlier commits changed. A new outcome adds enum variants
  in `generated/rust/inbox/src/messaging.rs`, `behaviour.rs`, `server/wire.rs`,
  `server/inbox_service.rs`, `server/inbox-service.openapi.json`, and new schemas and responses in
  `generated/openapi/openapi/inbox-service.yaml` — inferred
- **Also likely:** `crates/inbox-conformance/unsupported.tsv`. A generated scenario for a new
  outcome would likely get the reason "a caller's principal and inbox differ", as the 5
  `unknown-message` lines do (:1,11,18,19,55) — inferred
- **Documents:** `README.md:76-77`, `website/docs/limits.md:40-50`, `docs/plan/stage-2.md:569-571`
  state the 403/404 limit. `CHANGELOG.md:107-108` states it in the released 0.1.0 Known limits, so a
  new entry goes under `[Unreleased]` (:9) — cited
- **Documents:** `AGENTS.md:28` (3 ignored cases), :46 (343 passed, 9 ignored), :27/:53 (73
  unsupported, 64 of them principal≠inbox), :52/:55 (93 scenarios, 89 refused), :135-140 (the
  backlog rule) — cited lines, change inferred
- **Documents:** `docs/design/inbox.md`, the modelling decisions; both earlier model commits edited
  it — inferred
- **Not touched:** `crates/inbox-server/src` (the service maps nothing) and
  `docs/design/service.md`, which states no status for `not-recipient` (only the generic row 6 at
  :27) — cited
- **Not touched:** `crates/inbox-cli`, `tests/acceptance`, `crates/*/tests` other than adversary.rs:
  none of them expects `NotRecipient`/`NotSender` (`git grep`). The CLI exits 1 for every declared
  refusal (AGENTS.md:59, `crates/inbox-cli/src/client.rs:216`) — cited
- **Confidence:** high for the surfaces, because the story, the model and 2 earlier model commits
  name them. Medium that the decided route is expressible: the generated wire writes the outcome
  name and a caller-guard status (findings 1-4)
- **Would collide with:** any unit that edits `ess/domains/messaging.yaml`, `tools/scenarios.py` or
  `ess/scenarios/`, or regenerates `generated/`. Every model change rewrites every generated file's
  digest, so two model units collide on all of `generated/`. Also
  `crates/inbox-server/tests/adversary.rs`, `website/docs/limits.md`, `README.md`, `CHANGELOG.md`,
  `AGENTS.md` — inferred
- **Safety fact:** the listener passes command answers through unchanged and filters only view
  answers that are `200`, and every message view carries `to` and `from`. So the only leak is the
  command refusal, and nothing outside `ess/` has to change. Evidence:
  `docs/design/service.md:27-28`, `crates/inbox-server/src/service.rs:182-188,221-235` — step 2,
  unproven

Corrected by the coordinator on 2026-11-21, after the section above was returned:

- `crates/inbox-server/src` and `docs/design/service.md` are in scope: the listener answers
  the outcome `foreign-message` as `unknown-message` (Decisions). The section above was
  written for a model that answers alone.
- `docs/plan/stage-2.md`, `README.md`, `CHANGELOG.md`, `website/` and the count lines of
  `AGENTS.md` are written by the coordinator at the close of the wave, from the gate's
  numbers; the unit reports the lines its change makes untrue.

## As built

Wave 6 of stage 2, 2026-11-21: commits `d9b724d` and `bda962d`, merged as `0b45685`. Forked from
the merge of story:post-from-is-the-caller, without story:dedup-key-scope, which is held
(dependency-blocker:ess-805-second-run-witness). Checked against the Scope above by the
implementor; what differs:

- The model: the outcome `foreign-message` on `MarkDelivered`, `MarkRead`, `Answer`, `Refuse`
  and `Collect` (`when_subject`: `to != caller.inbox` and `from != caller.principal`, error
  `UnknownMessage`), behind `unknown-message` and ahead of `not-recipient` or `not-sender`. The 5
  commands stay generated.
- "Nothing outside `ess/` has to change" was untrue. The generated surface answers the outcome
  403 under its own name, so the listener gives it the status and the body of `unknown-message`
  (`as_unknown` in `crates/inbox-server/src/service.rs`, on answers of requests that are not a
  GET). The model alone decides who is a party; the function compares no caller.
- The 5 steps of a third principal are a new authored scenario,
  `a-third-principal-finds-no-message`, which claims the 5 `foreign-message` ids. It is
  unsupported over HTTP (a step as the Sweeper), and the run over HTTP could not witness the
  outcome anyway: it reads `unknown-message` there. The witness over HTTP is
  `crates/inbox-server/tests/foreign_message.rs` and the acceptance case.
- Not in the Scope: cases in `crates/inbox-server/tests/` and `crates/inbox-core/tests/`, and
  `crates/inbox-conformance/tests/adversary3.rs`.
- The claim is limited to the 5 commands. `post` still tells a third principal whether a message
  id is in use (`duplicate` for a stored id): story:post-does-not-tell-a-message-id-exists.
- A relaying sender (`from`) is a party; the principal it speaks for (`on_behalf_of`) is a third
  principal. 2 cases of `crates/inbox-core/tests/foreign_message.rs` hold that.

Adversary: 1 pass on `d9b724d`, 4 findings (2 warnings, 2 notes), 3 introduced and corrected in
`bda962d`, 1 there before the unit and filed as the story named above
(review-result:adversary-foreign-message-id-pass-1). No second pass: the corrections changed no
guard, no source line of a crate and no scenario.

Numbers after the unit (the coordinator's gate, `task check` exit 0): 97 scenarios, 27 authored,
95 refused and covered; the core lane 97 of 97; the serve lane 22 passed and 75 unsupported;
`cargo test --workspace` 366 passed, 0 failed, 8 ignored.
