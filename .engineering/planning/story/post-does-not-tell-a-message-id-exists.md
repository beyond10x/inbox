---
format: aep.planning-md/3
id: story:post-does-not-tell-a-message-id-exists
kind: story
status: active
title: A post does not tell a third principal whether a message id exists
summary: post answers duplicate for a stored message id, whoever sent it, so a third principal learns from a post whether an id is in use.
tags:
- from-finding
relations:
- informed_by: review-result:adversary-foreign-message-id-pass-1
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
- confidence: cited
  path: crates/inbox-core/tests/adversary.rs
- confidence: inferred
  path: crates/inbox-core/tests/core/obligations.rs
- confidence: cited
  path: crates/inbox-server/tests/adversary_w6_foreign.rs
- confidence: cited
  path: docs/design/inbox.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: ess/domains/messaging.yaml
- confidence: inferred
  path: ess/ess-inputs.yaml
- confidence: inferred
  path: ess/scenarios
- confidence: inferred
  path: generated
- confidence: cited
  path: tools/scenarios.py
- confidence: cited
  path: website/docs/limits.md
- confidence: inferred
  path: website/docs/status.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T17:17:28Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T17:17:28Z", actor: "agent:inbox", revision: 3}
---
## Outcome

A post to an address no inbox has, or to a closed inbox, answers the same for a `message_id`
that a stored message of another sender carries and for an id no message carries. Before this
story `PostMessage` answers `duplicate` for a `message_id` that a stored message carries,
whoever sent it (`ess/domains/messaging.yaml`, `PostMessage`, `existing_instance`), and it
answers that before `unknown-inbox`. A principal that posts as itself to an address no inbox
has, under the id of a message between two others, is answered `duplicate`, and `unknown-inbox`
under an id nobody used. Neither post stores anything.

With this story `duplicate` is the answer for the sender's own stored message only, and it is
still given before `unknown-inbox` and `inbox-closed`, so a sender's repost of an arrived
message answers `duplicate` whatever happened to the inbox since. An id that a message of
another sender carries is refused `id-taken`, after `unknown-inbox`, `duplicate-key` and
`inbox-closed`.

Narrowed on 2026-11-21 from "a post does not tell a principal that is neither the recipient nor
the sender of a message whether that message's id is in use" (see Decisions). What stays, and
is stated on `website/docs/limits.md`: a principal that holds a message id learns from a post
into an open inbox whether the id is live (`id-taken` against `posted`, which stores a
message), and it can take a free id first, after which the sender's post is refused
`id-taken`. That is accepted for principals of one operator. story:message-id-per-sender
removes it and is required before principals of different operators share one service.

## Acceptance

A third principal that posts, as itself, to an address no inbox has, once under the
`message_id` of a message between two others and once under an id no message carries, gets the
same status and the same body for both, where before this story the first is `duplicate` and
the second `unknown-inbox`. The same holds for a post into a closed inbox: both answer
`inbox-closed`.

A sender that posts a message again under its own `message_id` is still answered `duplicate`,
also after the recipient closed the inbox. A post into an open inbox under a `message_id` that
a stored message of another sender carries is refused `id-taken` and stores nothing.

## Context

Found by the adversary of story:foreign-message-id-answers-as-unknown in stage 2, wave 6
(review-result:adversary-foreign-message-id-pass-1, finding 1; the case is
`adv_w6_a_third_principal_cannot_tell_a_message_between_others_from_no_message_by_post` in
`crates/inbox-server/tests/adversary_w6_foreign.rs`, ignored until this story). The order of the
outcomes and the id shared by every sender are the specification's, so the finding was there
before the unit: the case fails the same way on the unit's base.

It is the fact the limit "A refusal tells a third principal whether a message exists" of
`website/docs/limits.md` names, reached through another command. Since
story:foreign-message-id-answers-as-unknown the 5 commands that name a stored message answer a
third principal as for an id nobody used; `post` is the command that still tells.

A post into the caller's own inbox shows the same (`duplicate` against `posted`) and stores a
message when the id is free.

## Decisions

Accepted on 2026-11-21 by the coordinating session under the standing rule of
approval-record:wave-6-security-drafts-accepted: a draft that an adversary pass files against an
open limit of `website/docs/limits.md` is accepted by the coordinator without asking and named
in the wave report. It is not part of wave 6.

Decided by the host operator's session on 2026-11-21, on 3 options the coordinating session put
to it (A: wait for the ESS fix, outside the gate; B: wait and join the gate; C: run now as its
own wave), as received:

> host-operator: B. story:post-does-not-tell-a-message-id-exists waits for the ESS fix of issue
> 805, runs in one wave with story:dedup-key-scope, and joins the gate (blocks edges to
> epic:two-hosts-and-wake and epic:jetstream-backend).
> - Reason: it is a finding of the same kind as the 3 in the gate; a gate that leaves one of
>   them out says less than the limits page.

It also ordered: when the ESS release is out, the wave starts without asking and is reported at
its close; no release, no listener outside loopback and no deployment before the gate stories
are implemented.

Decided by the host operator's session on 2026-11-21, on the options the scoper measured on ESS
0.97.1 (A, A2, B1, B2, B3 and C, listed under Scope), as received, with a person's name
replaced by "the operator":

> host-operator: B3 now, C filed as a backlog draft. Dispatch it when unit 1 is merged.
> - Reason: B3 meets the acceptance, keeps a sender's repost answering `duplicate`, and turns a
>   taken id into a refusal the sender sees; A2 loses a message silently in the repost path
>   (your inferred reading of the code is enough to rule it out); C is a wire change of 5
>   commands and a story of its own.
> - The story's Outcome is narrowed to what holds: an unknown or closed inbox answers the same
>   for a held and a free id. The limits page names the rest in your words: a principal that
>   holds a message id learns from a post into an open inbox whether it is live, and can take a
>   free id first.
> - Gate: it opens when both gate stories are implemented with B3, for one case only:
>   principals of one operator (the operator's own sessions and hosts). C is required before
>   principals of different operators share one service; write that on the limits page and as a
>   `blocks` edge from C to whatever epic first serves a second operator.
> - A release, a listener outside loopback and a deployment still each need their own order.

It asked that the operator be shown this decision; the operator's word replaces it.

C is story:message-id-per-sender, a draft. On 2026-11-21 the store holds 5 epics and none of
them serves principals of a second operator (`docs/design/signing.md` names "more than one
operator key" as outside version 1), so the `blocks` edge has no target yet. That draft's
Context says which epic gets the edge when it is written.

## Open for whoever scopes this

- The adversary's reading: reordering the outcomes does not close it. While a message id is
  shared by every sender, any answer that differs for an id in use tells. The ways it named:
  an id that is unique per sender, or a document that says an id can be probed.
- A message id is a ULID: 48 bits of time and 80 random bits. Whether "knows the id" needs a
  party that passed it on, and what that is worth, belongs in the scope.
- story:dedup-key-scope is the same question for the dedup key and is held on beyond10x/ess#805.
  A rule that compares the id among the sender's own messages reads `input.from` in a row
  guard, which is the construct that defect is about.

## Scope

Derived 2026-11-21 by `story-scoper` against `main` at `a5b681f`, on top of the rule of
story:dedup-key-scope, measured with ESS 0.97.1. Every line is **cited** (read from the story,
the tree or an ESS run) or **inferred** (a reading that could be wrong).

- **Measured first:** on ESS 0.97.1 the interpreted target answers an `existing_instance`
  outcome before every row refusal, whatever the declaration order. A `when_related` with
  `{via, exists: false}` answers before every `when_related` with `where`. Moving `duplicate`
  below `inbox-closed` leaves the third principal's post to an unknown address at `duplicate`.
  So the fix replaces `existing_instance` — cited (probe runs A, A2, B3)
- **Primary surface:** `ess/domains/messaging.yaml`, `PostMessage` outcomes (`:387-440`).
  `{name: duplicate, existing_instance: true}` (`:428`) becomes a `when_related` over
  `inbox.messaging.Message` with `where: {all: [message_id == input.message_id,
  from == input.from]}`, declared before `unknown-inbox`. A new outcome `id-taken` (the same
  `when_related`, `message_id == input.message_id` alone) goes after `inbox-closed`
  (`:438-440`). It needs an error: a new `MessageIdTaken` in `errors:` (`:266-307`, probed), or
  `NotSender` reused (unprobed) — cited (the lines; probe B3)
- **Files:** `crates/inbox-core/src/obligations.rs:186-206`, the row refusals of
  `fn post_message` (`:149`), become: own `duplicate` (`get` by id and `from == input.from`),
  `unknown-inbox`, `duplicate-key`, `inbox-closed`, `id-taken`. The doc comment `:123-145`
  states the 0.96.0 order and changes with it — cited (the lines; the order is the measured
  answer of the interpreted target, pair by pair)
- **Files:** `crates/inbox-server/tests/adversary_w6_foreign.rs:31`: remove
  `#[ignore = "pre-existing, story:post-does-not-tell-a-message-id-exists"]` — cited
- **Files:** `crates/inbox-core/tests/adversary.rs:288-298`:
  `duplicate_answers_before_unknown_inbox` now answers `unknown-inbox` (measured). The other 3
  pairs at `:276-287` and `:300-324` stay true — cited (probe scenarios
  `pr-own-id-against-nobody`, `pr-repost-after-close`, `pr-key-against-closed`,
  `pr-key-against-nobody`)
- **Files:** `tools/scenarios.py`. The `COVERS` claims on `PostMessage` `signature-invalid`
  (`:125`), `authority-invalid` (`:127`) and `unknown-option` (`:146`) go. Without
  `existing_instance`, synthesis writes those 3 generated scenarios, and synthesis with declared
  coverage stops at `covers.unknown-scenario`. A new authored scenario claims
  `PostMessage/outcome/id-taken`, which synthesis refuses (`ESS-SYNTH-001`) — cited (probe logs)
- **Also likely:** a second authored scenario: a third principal posts to an unknown address,
  under a held id and under a free one. Then `task scenarios` rewrites `ess/scenarios/` and
  `ess/ess-inputs.yaml` — inferred
- **Also likely:** `crates/inbox-conformance/unsupported.tsv`: the 3 new generated `PostMessage`
  scenarios run as caller `principal-262144`, inbox `inbox-262144`. That is the
  "principal and inbox differ" reason of `:49-55`, so 3 more lines — inferred
- **Also likely:** `crates/inbox-core/tests/core/obligations.rs`: a case for `id-taken` beside
  `:113` (`duplicate` before `duplicate-key`, which still holds: measured) — inferred
- **Regenerated:** `generated/`, the 34 files a model change rewrites (see
  story:dedup-key-scope), now also carrying outcome `id-taken` and its error (enum variant,
  codecs, status, OpenAPI, docs page) — inferred (`task generate` and `task projections` not
  run)
- **Not changed in code:** `crates/inbox-cli`. `final_refusal` (`src/commands.rs:380-382`)
  drops the outbox line on every model error but `UnknownInbox`. `repost` counts only
  `Duplicate` as arrived (`:575`). So `id-taken` is final: `post` and `repost` drop the line and
  exit 1 — cited (code); not run
- **Behaviour kept:** a repost of an arrived message whose recipient has since closed the inbox
  still answers `duplicate`. `repost` reports it arrived, exits 0 and keeps the line until it is
  collected or expires — cited (`pr-repost-after-close`; `commands.rs:575`)
- **Not touched:** the backend contract and its kit. The core reads `MessageStorage::get` by id
  and compares `from` (`generated/rust/inbox/src/behaviour.rs:45-55`,
  `crates/inbox-core/src/backend.rs:132-142`) — cited
- **Documents:** these state the old behaviour: `website/docs/limits.md:20-43` (finding 1) and
  `:13` (gate), `README.md:70-74`, `CHANGELOG.md:74-76`, `docs/design/inbox.md:100-107`
  (decision 5 tail, decision 6), `docs/design/service.md:94-98`, and `AGENTS.md:28`,
  `:153-157` — cited. The scenario and test counts (`AGENTS.md:46,52-55`,
  `website/docs/status.md:32-36`; 97 scenarios becomes 100 plus the new authored ones) are the
  coordinator's at the close — inferred
- **Still true:** "idempotent by `message_id`" (`website/docs/limits.md:66`,
  `docs/design/service.md:73`) still holds for a sender's own repost — inferred
- **Residual for limits.md:** a party that holds a message id still learns from a post into an
  open inbox whether the id is live (`id-taken` against `posted`, which stores). It can take a
  free id first, and the sender's post is then refused `id-taken` — cited (measured), wording
  inferred
- **Confidence:** high for the model site, the core site, the ignored case and the ESS answers
  (measured on 0.97.1). Medium for `unsupported.tsv`, the authored scenario set and the
  regenerated content (coverage, generate and conform not run)
- **Would collide with:** story:dedup-key-scope: the same outcome list
  (`messaging.yaml:428-440`), the rows of `fn post_message` (`obligations.rs:186-206`) and its
  doc comment, `generated/`, `tools/scenarios.py` and `ess/scenarios/`,
  `crates/inbox-core/tests/adversary.rs:276-324`, and the finding numbers in `limits.md`,
  `README.md`, `AGENTS.md`, `status.md`, `CHANGELOG.md`. Run this unit after that one, on its
  tree
- **Safety fact:** with `existing_instance` replaced by `when_related` over `Message` by id,
  `--strict-requires` accepts the model, and the 0.97.1 interpreted target passes 107 of 107
  (100 generated and authored, plus 7 probes) — step 4 against ESS's interpreter, unproven
  against the core and over HTTP

The options the scoper measured on ESS 0.97.1, each on top of the rule of story:dedup-key-scope:

- A, the order alone (`duplicate` and `duplicate-key` below `inbox-closed`): changes no answer.
  The third principal's post to an unknown address is still answered `duplicate`.
- A2, `duplicate` as a `when_related` by id after `inbox-closed`: meets the acceptance. A
  sender's repost to an inbox closed since answers `inbox-closed`; an id another principal took
  first answers the sender `duplicate`, which `inbox repost` counts as arrived.
- B1, own `duplicate` by `when_related` and `id-taken` by `existing_instance`: the interpreted
  target fails `repost-after-restart-arrives-once` (`observed: outcome = id-taken`).
- B2, `existing_instance` beside `when_subject`: refused, `ESS-COMMAND-004`.
- B3, own `duplicate` by `when_related` with `from == input.from`, declared first, and
  `id-taken` by `when_related` after `inbox-closed`: meets the acceptance; a sender's repost
  still answers `duplicate`; the interpreted target passes 107 of 107.
- C, a message id that is unique per sender: not probed. It changes the wire of the 5 commands
  that name a stored message, the backend contract, its kit and the CLI.

No answer for an id another sender holds can equal the answer for a free id in an open inbox: a
free id answers `posted` and stores a message.
