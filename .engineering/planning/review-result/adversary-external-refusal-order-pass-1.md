---
format: aep.planning-md/3
id: review-result:adversary-external-refusal-order-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:external-refusal-order-cases (cf4cee0): 2 low findings, 1 there before the unit'
relations:
- reviews: story:external-refusal-order-cases
revision: 1
---
## What was reviewed

story:external-refusal-order-cases at `cf4cee0` on base `d52ba0f` (branch
`impl/w8-external-order`), by one adversary on Sonnet, pass 1 of at most 2, on 2026-11-21. No
second pass ran: the correction adds the adversary's own 9 cases; no source line of a crate
changed.

## Verdict

Broken. 2 findings, both low: 1 introduced by the unit, 1 there before it. The 5 other orders
of the 3 external refusals and 31 placements of them among the other refusals each fail
`cargo test -p inbox-core`.

## Findings

- F1 (low, there before the unit; `crates/inbox-core/src/obligations.rs`, `post_message`): a
  core that asks the verifier all 3 external questions before it answers, and then answers the
  first that holds in the right order, passes every case (44 of 44 of the core binary). The
  comment of `PostExternals` in `crates/inbox-core/src/context.rs` says one question per branch,
  in declaration order. No case saw what the verifier is asked.
- F2 (low, introduced; `crates/inbox-core/tests/core/precedence.rs`): the 4 cases post a note
  and a decision. A core that asks the 3 refusals in reverse for a request, and in the right
  order otherwise, passes them (44 of 44).

## Changed cores (40 built in a copy of the tree, plus the 2 of the findings)

- The 5 other orders: each fails at least 1 of the 4 cases of the unit. Authority before
  signature fails only `signature_invalid_answers_before_authority_invalid`.
- The 3 refusals together before `not-sender`, `no-authority`, `already-expired`,
  `request-without-shape` or `decision-without-options` (5), and after `duplicate`,
  `unknown-inbox`, `duplicate-key` or `inbox-closed` (4): each fails.
- Each of the 3 alone moved to 8 places (24): each fails. Which of these the older cases alone
  fail was not separated.
- A core that stores the message before it refuses: seen by the unit's cases on
  `unknown-option` and `signature-invalid`; on `authority-invalid` only by an older case, since
  no case of the unit ends in that refusal.

## What held

- Both faults of each case hold: each fault alone, on the unit's envelopes, answers its own
  refusal, and the envelope with no fault posts.
- The order holds for faults made in other ways: a recommended option no option has, 2 options
  with one letter, a silence option on a request, a key the verifier does not know, a
  delegation that names another principal.
- The assertion that nothing was stored can fail: the backend the case reads is the one the
  post writes.

## Cases

9 cases in `crates/inbox-core/tests/adversary_w8_order.rs`, green on `cf4cee0`. 2 of them fail
the core of F1, 1 fails the core of F2.
