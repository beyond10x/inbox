---
format: aep.planning-md/3
id: story:post-from-is-the-caller
kind: story
status: implemented
title: A post or an answer is refused when its from is not the caller
summary: A recipient sends a sender's signed envelope again under its own request signature and it is posted once the first message is gone; the model guards PostMessage and Answer on the caller.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-http-service-pass-1
- blocks: epic:two-hosts-and-wake
- blocks: epic:jetstream-backend
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
  path: crates/inbox-core/src/obligations.rs
- confidence: inferred
  path: crates/inbox-core/tests
- confidence: cited
  path: crates/inbox-server/tests/adversary.rs
- confidence: cited
  path: docs/design/inbox.md
- confidence: inferred
  path: docs/design/service.md
- confidence: cited
  path: ess/domains/messaging.yaml
- confidence: cited
  path: ess/ess-inputs.yaml
- confidence: cited
  path: ess/scenarios
- confidence: cited
  path: generated
- confidence: cited
  path: tools/scenarios.py
- confidence: cited
  path: website/docs/first-use.md
- confidence: inferred
  path: website/docs/how-it-is-built.md
- confidence: cited
  path: website/docs/limits.md
- confidence: inferred
  path: website/docs/status.md
revision: 25
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T15:32:33Z", actor: "agent:inbox", revision: 22}
- {from: "proposed", to: "active", at: "2026-11-21T15:32:33Z", actor: "agent:inbox", revision: 23}
- {from: "active", to: "implemented", at: "2026-11-21T17:17:28Z", actor: "agent:inbox", revision: 25, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"verification":1}}}
---
## Outcome

A post or an answer is refused when the envelope's `from` is not the principal of the caller that
sent the request. Today `PostMessage` reads no attribute of the caller: a principal that holds a
signed envelope of another principal (every recipient holds the ones it received, `Pending` hands
them out) sends the same bytes again under its own request signature, and once the first message
is handled and removed the service answers `posted` a second time. The message reappears in the
sender's outbox, and a decision with `default_on_silence` gives the sender a second decided
answer. `already-expired` compares the envelope's own `expires` and `created`, so the clock does
not end this.

The rule belongs in `ess/domains/messaging.yaml` as an input-guarded refusal on the caller
(`from != caller.principal`), as `OpenInbox` already guards its input on the caller
(`address != caller.inbox`), with a
scenario that holds it; the core and the listener then take it from the generated crate.

## Acceptance

A request signed by one principal that posts an envelope whose `from` is another principal is
refused by the running service and stores nothing, where before this story it is answered
`posted` once the first message under that id is gone.

## Context

Found by the adversary of story:http-service in stage 2, wave 3
(review-result:adversary-http-service-pass-1, finding F6; the case is
`adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` in
`crates/inbox-server/tests/adversary.rs`, ignored until this story). The specification has no
such guard at the wave base and the listener passes the command on, so the finding was there
before the unit.

With a delegation the caller is still `from` (the agent that signs); `on_behalf_of` names the
principal it speaks for, so the rule does not touch delegation.

## Decisions

Accepted on 2026-11-21 (approval-record:wave-6-security-drafts-accepted): the host operator's
session accepted this draft together with story:foreign-message-id-answers-as-unknown and
story:dedup-key-scope, to run as wave 6 after the 5 waves of the stage 2 plan, with the gate of
those waves and no release.

Settled by the coordinator on 2026-11-21:

- The outcome is named `not-sender` and carries the error `inbox.messaging.NotSender`, as
  `Collect` names the same comparison on a stored message (`from != caller.principal`). No
  error is added.
- It is the first outcome of `PostMessage` and of `Answer`: a caller that is not `from` gets
  this answer whatever else the envelope holds, and learns nothing about a stored message.
- Probed on a copy of the model: ESS 0.96.0 validates the guard on both commands, and the
  generated contract answers it with the status 403.
- This story's unit runs first of the 3: the other 2 change the same model file and the same
  generated crate.

## Scope

Derived 2026-11-21 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `ess/domains/messaging.yaml`: a new input-guarded outcome `from !=
  caller.principal` on `PostMessage` (outcomes `:383-433`) and on `Answer` (`:538-572`), built like
  `OpenInbox` `not-owner` (`:317-319`). Cited.
- **Error of the new outcomes:** not decided. `NotSender` already exists (`:279-281`, "The caller
  did not send the message."). A new error would add a file under `generated/schema/schema/errors/`
  and change "13 errors" (`AGENTS.md:16`, `website/docs/status.md:24`). Inferred.
- **Hand-written code:** `crates/inbox-core/src/obligations.rs:144-148`. `PostMessage` is an
  obligation (`generated/rust/inbox/PLAN.md:116`), so its guard is written here and not generated.
  Cited.
- **Hand-written code, detail:** the impl bound `C: PostExternals` (`obligations.rs:144`) needs
  `TryContext` as well to read `caller.principal`, because `Core` offers it only where `C:
  TryContext` (`:74-81`). The order comment `:121-140` names the guards in order and must change.
  Inferred.
- **Generated, not hand-written:** the `Answer` guard comes from the generated crate
  (`generated/rust/inbox/src/behaviour.rs:133`, `AnswerBehavior for Generated`). Cited.
- **Tests:** `crates/inbox-server/tests/adversary.rs:265`: remove the `#[ignore]` of
  `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal`. Cited.
- **Tests at risk:** `crates/inbox-core/tests/core/section.rs:117-127,205-216`. The `Panics` context
  answers no `caller_principal` and the case expects a panic in `unknown_option`. A guard that reads
  an absent principal the way generated code does returns an unmet obligation before the panic
  (`decided`/`equal`, `generated/rust/inbox/src/behaviour.rs:811-813,838-840`), so the case would
  fail. Inferred.
- **New core cases for the guard:** in `crates/inbox-core/tests/core/branches.rs` or
  `precedence.rs`. Inferred.
- **Scenario source:** `tools/scenarios.py`. `post()` takes no caller argument (`:225`; the caller
  is the sender at `:253`), and `answer()` sets `from` to the caller (`:283`). A replay scenario
  needs a caller argument. Cited.
- **Coverage claims:** the `COVERS` map (`tools/scenarios.py:110-158`) gets the ids of the new
  outcomes, as `OpenInbox` `not-owner` is claimed at `:132`. Inferred.
- **Written by `task scenarios`:** `ess/scenarios/<new>.yaml` and `ess/ess-inputs.yaml`
  (`Taskfile.yml:102-105`, `AGENTS.md:103`). Cited.
- **Regenerated by `task generate`:** `generated/rust/inbox/` (`Taskfile.yml:28-32`). Cited.
- **Regenerated by `task projections`:** `generated/{openapi,asyncapi,schema,docs}/`, each with its
  checked-in `.ess-output/state.json` (`Taskfile.yml:49-53`). `generated/rust/inbox/.ess-output/` is
  not checked in (`AGENTS.md:20`). Commits `d35d455` and `29575e0` each rewrote 34 files under
  `generated/`. Cited.
- **Conformance lists:** `crates/inbox-conformance/unsupported.tsv` gets one line per new generated
  scenario id that HTTP cannot run. Generated callers carry a principal different from the inbox
  (`principal-262144` / `inbox-262144`), which the wire target refuses
  (`crates/inbox-conformance/src/wire_target.rs:765`). `unsupported-core.tsv` stays empty. Inferred.
- **No change:**
  - `crates/inbox-server/src`: the caller is built from the key's principal (`service.rs:83-84`).
  - `CoreTarget`: hands the step caller's `principal` through
    (`crates/inbox-conformance/src/core_target.rs:181-217`).
  - `WireTarget`: signs the request with the step caller's key and the envelope with the key of its
    `from` (`wire_target.rs:110-118,871-890`), so a replay scenario runs over HTTP.
  - `crates/inbox-cli`: sets `from` from the caller (`src/commands.rs:424,514`).
  Cited.
- **Documents stating the old behaviour:**
  - `website/docs/limits.md:13-38`
  - `website/docs/first-use.md:13-14`
  - `README.md:69-75,123-124`
  - `AGENTS.md:28` (3 ignored cases), `:46` (343 passed, 9 ignored), `:135-140`
  - `CHANGELOG.md` `[Unreleased]`, and `:66-68` ("the binding of a key to `from`" is held by tests)
  - `docs/design/inbox.md:89-91` (decision 5, ownership)
  - `docs/plan/stage-2.md:418,541-544,562-568`
  Cited.
- **Documents for the rule's new text:** `docs/design/service.md:74-82` (the caller).
  `CHANGELOG.md:102-106` sits in the released `[0.1.0]` section and stays. Inferred.
- **Count lines that move if scenarios are added:**
  - `AGENTS.md:17,27,52,53,55`
  - `README.md:15,22,84`
  - `CHANGELOG.md:60`
  - `docs/design/inbox.md:117,128`
  - `website/docs/status.md:32`, `website/docs/how-it-is-built.md:11,31`,
    `website/docs/limits.md:92`
  The lines are cited; that they change is inferred.
- **Confidence:** medium. Every surface was read, but synthesis was not run, so which generated ids
  appear or are refused, and whether existing generated posts get `from = caller.principal`, is
  unknown.
- **Would collide with:** any unit that changes `ess/` (each regeneration rewrites about 34 files
  under `generated/`), `tools/scenarios.py` or `ess/scenarios/`,
  `crates/inbox-core/src/obligations.rs`, `crates/inbox-conformance/unsupported.tsv`,
  `crates/inbox-server/tests/adversary.rs`, or the count lines in `AGENTS.md`, `README.md`,
  `CHANGELOG.md` and `website/docs/`.
- **Safety fact:** no authored scenario, running test or CLI path posts or answers with a `from`
  other than the caller's principal (`tools/scenarios.py:253,283`;
  `crates/inbox-core/tests/core/support.rs:300,305`; `crates/inbox-cli/src/commands.rs:424,514`).
  Level 2, unproven.

Corrected by the coordinator on 2026-11-21, after the section above was returned:

- The error is decided (Decisions): `NotSender`, so the count of 13 errors stays.
- `docs/plan/stage-2.md`, `README.md`, `CHANGELOG.md`, `website/` and the count lines of
  `AGENTS.md` are written by the coordinator at the close of the wave, from the gate's
  numbers; the unit reports the lines its change makes untrue.

## As built

Wave 6 of stage 2, 2026-11-21: commits `3cb3eb9` and `1d2e791`, merged as `e353a41`. Checked
against the Scope above by the implementor; what differs:

- `not-sender` (`when: from != caller.principal`, error `NotSender`) is the first outcome of
  `PostMessage` and of `Answer`. `Answer` stays generated. The obligation `post_message` checks it
  first; a caller without a principal is an unmet obligation, as in the generated guard.
- The authored scenario `answer-from-another-principal-refused` claims
  `Answer/outcome/not-sender`. `post-from-another-principal-refused` claims nothing: ESS refuses
  a claim on `PostMessage/outcome/not-sender`, which synthesis writes itself. That generated
  scenario is unsupported over HTTP (its caller's principal and inbox differ); the authored one
  witnesses the replay there.
- Not in the Scope: `crates/inbox-conformance/tests/adversary3.rs` (the pinned totals) and a
  `# SYNTHESIS:` comment in the model.
- The interim the Scope did not foresee: with the guard, ESS 0.96.0 synthesis writes no stored
  message and 49 generated scenarios leave the suite (beyond10x/ess#804). The input `from` of
  `PostMessage` carries `example: principal-262144`, the value synthesis gives its first caller.
  No scenario id leaves with it; 19 generated scenarios lost the second run under a second
  caller. The example shows in 3 generated files (2 OpenAPI documents, 1 JSON Schema) and
  changes no decoder. It goes when an ESS release fixes the issue.
- `generated/`: 36 files changed, 12 of them beyond the digest line.

Adversary: 1 pass on `3cb3eb9`, 2 notes, both introduced and both answered with cases in
`1d2e791` (review-result:adversary-post-from-is-the-caller-pass-1). 8 changed cores were built;
the 2 that stayed green are the 2 notes.

Numbers after the unit (the coordinator's gate, `task check` exit 0): 96 scenarios, 26 authored,
90 refused and covered; the core lane 96 of 96; the serve lane 22 passed and 74 unsupported;
`cargo test --workspace` 355 passed, 0 failed, 8 ignored.
