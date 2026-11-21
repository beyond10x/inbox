---
format: aep.planning-md/3
id: story:dedup-key-scope
kind: story
status: active
title: A dedup key of one sender does not refuse another sender's post
summary: duplicate-key compares the key alone over every message, so a third principal learns of a key or takes it first.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-http-service-pass-1
- blocks: epic:two-hosts-and-wake
- blocks: epic:jetstream-backend
- depends_on: story:post-from-is-the-caller
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/inbox-core/src/obligations.rs
- confidence: inferred
  path: crates/inbox-core/tests
- confidence: cited
  path: crates/inbox-server/tests/adversary.rs
- confidence: cited
  path: docs/design/inbox.md
- confidence: cited
  path: ess/domains/messaging.yaml
- confidence: inferred
  path: ess/ess-inputs.yaml
- confidence: inferred
  path: ess/scenarios
- confidence: cited
  path: generated
- confidence: inferred
  path: tools/scenarios.py
- confidence: cited
  path: website/docs/limits.md
- confidence: inferred
  path: website/docs/status.md
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T15:32:33Z", actor: "agent:inbox", revision: 17}
- {from: "proposed", to: "active", at: "2026-11-21T15:32:33Z", actor: "agent:inbox", revision: 18}
---
## Outcome

A dedup key one sender used does not refuse a post of another sender. Today `duplicate-key`
compares the key alone over every stored message (`ess/domains/messaging.yaml`, `PostMessage`,
`where: dedup_key == input.dedup_key`): a principal that posts into its own inbox under a key
learns from `duplicate-key` whether any message carries that key, and by posting first makes
another sender's post under the same key fail.

## Acceptance

After a third principal has posted into its own inbox under a dedup key, a post of another sender
to another inbox under the same key is answered `posted`, where before this story it is refused
`duplicate-key`.

## Context

Found by the adversary of story:http-service in stage 2, wave 3
(review-result:adversary-http-service-pass-1, finding F8; the case is
`adv_a_third_principal_cannot_take_another_senders_dedup_key` in
`crates/inbox-server/tests/adversary.rs`, ignored until this story). The comparison is the
specification's, so the finding was there before the unit.

## Decisions

Accepted on 2026-11-21 (approval-record:wave-6-security-drafts-accepted): the host operator's
session accepted this draft together with story:post-from-is-the-caller and
story:foreign-message-id-answers-as-unknown, to run as wave 6 after the 5 waves of the stage 2 plan,
with the gate of those waves and no release.

Settled by the coordinator on 2026-11-21:

- The scope of a dedup key is the sender. `duplicate-key` compares the key among the stored
  messages whose `from` is the `from` of the post: the `where` of the outcome holds
  `dedup_key == input.dedup_key` and `from == input.from`. Probed on a copy of the model: ESS
  0.96.0 validates it.
- A sender that uses one key for two inboxes is refused the second post, as today. The scope
  is not narrowed to the inbox: the key names one message of its sender.
- This story's unit runs after story:post-from-is-the-caller. With that guard `from` is the
  caller, so a post is compared only with the caller's own messages; without it a third
  principal could still post with `from` set to the sender whose key it wants to take.
- `docs/design/inbox.md`, decision 6, is where the scope is written down.

## Scope

Derived 2026-11-21 by `story-scoper` against `main` at `7b43de0`, before
  story:post-from-is-the-caller has merged. Every line is **cited** (read from the story or the
  tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `ess/domains/messaging.yaml`, `PostMessage` outcome `duplicate-key`
  (`ess/domains/messaging.yaml:422-427`, today `where: dedup_key == input.dedup_key`), which becomes
  `where: {all: [dedup_key == input.dedup_key, from == input.from]}` — cited (story Outcome; the
  decision)
- **Files:** `crates/inbox-core/src/obligations.rs:187-192`, the hand-written `duplicate-key` in
  `impl PostMessageBehavior for Core` / `fn post_message` (`:144-145`):
  `MessageStorage::list(..).iter().any(|row| row.data.dedup_key == input.dedup_key)` gains `&&
  row.data.from == input.from`; the doc comment at `:130-139` names `duplicate-key` (over `list`) —
  cited
- **Files:** `crates/inbox-server/tests/adversary.rs:331-332`, remove `#[ignore = "pre-existing,
  story:dedup-key-scope"]` from `adv_a_third_principal_cannot_take_another_senders_dedup_key` —
  cited
- **Files:** `docs/design/inbox.md:92-93`, decision 6 ("Dedup holds among live messages only, by
  `message_id` and by `dedup_key`"), where the scope by sender is written down; `:27-28` and `:159`
  name dedup and the obligation, and still hold — cited
- **Regenerated:** `generated/`, the 34 files `d35d455` and `29575e0` each rewrote. `task generate`
  writes 23 files under `generated/rust/inbox` (the selector text in `PLAN.md:116`,
  `plan.json:298-300`, `src/messaging.rs:1264,1981`, `src/server/inbox-service.docs.md:763`,
  `src/server/inbox-service.openapi.json:4037`; in every other file only the digest header changes;
  `Cargo.lock` stays as it is; `.ess-output/` is not checked in). `task projections` writes 11 files
  under `generated/{openapi,asyncapi,schema,docs}`: the text in
  `generated/openapi/openapi/inbox-service.yaml:2813` and
  `generated/docs/docs/domains/inbox-messaging.md:763`, and otherwise digests and the four
  `.ess-output/state.json` — cited (the file lists of `git show --stat d35d455 29575e0`, plus `git
  grep 'dedup_key == '` over `generated/`)
- **Not regenerated by a model-only change:** `ess/scenarios/*.yaml` and `ess/ess-inputs.yaml`. They
  carry no digest, so `task scenarios` rewrites them only when `tools/scenarios.py` changes.
  `d35d455` touched one scenario file, and only because the script changed — cited (`grep -l digest
  ess/scenarios` is empty)
- **Also likely:** `tools/scenarios.py`, plus `task scenarios` output in `ess/scenarios/` and
  `ess/ess-inputs.yaml`: a new authored scenario in which a third principal posts under the key and
  then another sender posts to another inbox under it and is answered `posted`. The only authored
  key scenario, `repeated-dedup-key` (`tools/scenarios.py:432-438`, claimant of
  `PostMessage/outcome/duplicate-key` at `:118`), uses one sender and one inbox, so it does not tell
  a global key from a per-sender one — inferred
- **Also likely:** `crates/inbox-core/tests/core/obligations.rs:75-110`: a unit case for a second
  sender beside `duplicate_key_refuses_a_second_message_with_a_live_dedup_key` — inferred
- **Also likely:** the counts that follow a new scenario and one un-ignored case: `AGENTS.md:28` ("3
  cases in `tests/adversary.rs` are ignored"), `AGENTS.md:46` ("343 passed, 0 failed, 9 ignored"),
  the scenario counts 93/24/89 in `AGENTS.md`, `docs/design/inbox.md:117-130`, and
  `website/docs/status.md:30` ("38 cases, 3 ignored (the open findings 1 to 3 …)") — inferred
  (whether synthesis numbers move is unrun)
- **Documents:** these state the old behaviour as an open finding: `website/docs/limits.md:51-59`
  (finding 3), `README.md:78-79` (limit 3) and `AGENTS.md:135-136` (the backlog rule naming this
  story) — cited. `docs/plan/stage-2.md:572-573` (Backlog) is cited too; that page is the
  coordinator's. `CHANGELOG.md` gets a new line under `## [Unreleased]` (`:9`) — inferred.
  `CHANGELOG.md:109-110` belongs to released 0.1.0 and stays — cited
- **Documents, still true per sender, no change needed:** `docs/design/service.md:72`,
  `docs/design/signing.md:202`, `website/docs/limits.md:66,83` ("idempotent by `message_id` and
  `dedup_key`") and `crates/inbox-cli/src/main.rs:296` ("The sender's key of this logical message")
  — cited
- **Not touched:** the backend contract and its kit (`crates/inbox-backend-testkit`,
  `crates/inbox-core/src/backend.rs`) and the CLI — cited (see Safety fact)
- **Runs on top of:** story:post-from-is-the-caller. It adds an input-guarded refusal `from !=
  caller.principal` to the same outcome list (`ess/domains/messaging.yaml:383-427`) and to `Answer`,
  and a branch to step 1 of the same `fn post_message` (`obligations.rs:146-163`). It regenerates
  the same 34 files, likely adds a scenario and a `COVERS` entry in `tools/scenarios.py`, removes
  the ignore at `crates/inbox-server/tests/adversary.rs:265`, and takes finding 1 out of
  `website/docs/limits.md`, `README.md` and the ignored counts. This story then rebases its line
  numbers and its finding numbering onto that tree — cited (story:post-from-is-the-caller Outcome
  and Scope); the exact lines it leaves are inferred
- **Confidence:** high for the primary surface, the core site and the regenerated set (the story and
  decision name them, and two earlier model changes show the file list). Medium for the scenario and
  count fallout (no synthesis was run)
- **Would collide with:** any unit on `PostMessage` in `ess/domains/messaging.yaml`, and any unit
  that regenerates `generated/` (every regeneration rewrites the digest line of all 34 files). Also
  any unit on `fn post_message` in `crates/inbox-core/src/obligations.rs`, on
  `crates/inbox-server/tests/adversary.rs`, on `tools/scenarios.py` / `ess/scenarios/`, and on the
  limits lists and ignored counts in `website/docs/limits.md`, `README.md`, `AGENTS.md` and
  `website/docs/status.md`
- **Safety fact:** no backend looks a message up by dedup key. The generated `MessageStorage` port
  is `get`/`put`/`delete`/`list` (`generated/rust/inbox/src/behaviour.rs:43-55`). `MemoryBackend`
  keys its `BTreeMap` by message id (`crates/inbox-core/src/backend.rs:90,131-148`), and the core
  scans `list` (`obligations.rs:187-189`). The kit leaves dedup to the core
  (`crates/inbox-backend-testkit/src/lib.rs:27-29`). So a per-sender key changes one predicate in
  the core and no backend or kit rule — step 2, unproven
- **Safety fact (ordering):** the sender scope stops a third principal only once `from` is bound to
  the caller. Before story:post-from-is-the-caller, `PostMessage` reads no attribute of the caller
  (`website/docs/limits.md:20-21`), so a third principal can post with `from` set to the victim and
  still take the victim's key — step 3, unproven

Corrected by the coordinator on 2026-11-21, after the section above was returned:

- `README.md`, `CHANGELOG.md`, `website/` and the count lines of `AGENTS.md` are written by
  the coordinator at the close of the wave, from the gate's numbers; the unit reports the
  lines its change makes untrue.
