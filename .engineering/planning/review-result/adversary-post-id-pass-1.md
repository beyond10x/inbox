---
format: aep.planning-md/3
id: review-result:adversary-post-id-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:post-does-not-tell-a-message-id-exists (93e05e3): 2 findings (1 warning, 1 note), both introduced'
relations:
- reviews: story:post-does-not-tell-a-message-id-exists
revision: 1
---
## What was reviewed

The unit of story:post-does-not-tell-a-message-id-exists in stage 2, wave 7: the commit
`93e05e3` on the branch `impl/post-id`, by one adversary on Opus, pass 1 of at most 2, on
2026-11-21. No second pass ran: the correction adds the adversary's 2 cases and changes one
sentence of a design document; no source line of a crate, no rule of the model and no scenario
changed.

## Verdict

The claim of the unit holds: for a post to an address no inbox has and into a closed inbox, the
whole HTTP answer is the same for a held and a free message id, in 36 combinations of the core
and byte for byte over HTTP. 2 findings, both introduced by the unit: 1 warning (a gap of the
gate) and 1 note (a sentence of a document).

## Findings

- F1 (warning, introduced; `crates/inbox-core/src/obligations.rs`, `post_message`): a core
  whose `duplicate` compares `on_behalf_of` in place of `from` passes every case of the
  workspace, the core lane (103 of 103), the serve lane (25 passed, 78 unsupported) and
  `task acceptance`. No case and no scenario posts a delegated message twice, or under its id
  as the principal spoken for. The same change in the model was not run.
- F2 (note, introduced; `docs/design/service.md`, the paragraph on a third principal): the
  sentence said a post into a closed inbox answers with the body of `unknown-inbox` or of
  `inbox-closed`. A caller whose own live message carries the post's dedup key is answered
  `duplicate-key`, alike for a held and a free id, so nothing is told; the sentence was wrong.

## Changed copies (8)

- `duplicate` without `from`; `id-taken` before `inbox-closed`; `id-taken` before
  `unknown-inbox`; `id-taken` removed; `duplicate` before `unknown-inbox` (the core): each
  fails `cargo test` and the core lane.
- The model's `id-taken` declared before `inbox-closed`: fails `task conform-interpreted` only
  (scenario `a-third-principal-cannot-tell-a-message-id-by-post`).
- The model's `duplicate` without `from`: refused by `task spec` (`ESS-COMMAND-012`, `id-taken`
  is never taken).
- The core's `duplicate` by `on_behalf_of`: F1.

## What held

- None of the 8 refusals that come before the row refusals reads a stored message for
  `PostMessage`.
- Status and body of `id-taken` agree across the generated server, the OpenAPI document and the
  stand-in of the client's cases. No 5xx path for a held id was found.
- The client drops the outbox line on `MessageIdTaken`; `repost` counts only `Duplicate` as
  arrived; a third party that reposts another sender's envelope sends nothing.
- `generated/` regenerated in a copy equals the tree, apart from the counter in
  `.ess-output/state.json`. The unsupported list is true of the run.

## Cases

2 cases: `adv_w7_a_delegated_message_id_is_its_senders_not_the_spoken_for_principals`
(`crates/inbox-core/tests/adversary_w7_id.rs`), green on `93e05e3` and red on the core of F1;
and a case over HTTP in `crates/inbox-server/tests/adversary_w7_id.rs` that failed on `93e05e3`
against the sentence of F2 and holds the 3 bodies since the correction.
