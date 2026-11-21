---
format: aep.planning-md/3
id: review-result:adversary-dedup-key-scope-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:dedup-key-scope (58258a8): 2 warnings, both gaps of the gate, none against the code'
relations:
- reviews: story:dedup-key-scope
revision: 1
---
## What was reviewed

The unit of story:dedup-key-scope in stage 2, wave 7: the commits `da272a7` (ESS 0.97.1 pinned,
no rule change) and `58258a8` (the rule) on the branch `impl/dedup-key-scope`, by an adversary
pass on 2026-11-21. The pass was started by one agent and finished by a second from the first
one's files, because the coordinator's process restarted in between.

## Result

2 findings, both warnings, both introduced by the unit, both gaps of the gate. No finding
against the code: the rule in the model and in `fn post_message` is the decided one.

1. A model whose `duplicate-key` compares `to == input.to` in place of `from == input.from`,
   regenerated, passes all of `task check`. The authored scenario `dedup-key-is-the-senders`
   refuses its cross-inbox post in an inbox where another sender already holds the key, so an
   inbox scope gives the same answers.
2. A rule that compares `on_behalf_of` in place of `from` passes the unit's own cases, every
   scenario, `task conform` and `task acceptance`, in the core and in the model: nothing
   posted under a key with an `on_behalf_of` that differs from `from`.

## What was tried and held

- 15 cases the adversary wrote, all green on the unit: a delegate's key and its principal's key
  refuse nothing of each other; two delegates of one principal do not share a key; a delegate is
  refused its own key whoever it speaks for; the whole answer for a held key equals the answer
  for a free key, in the core and byte for byte over HTTP into 3 inboxes; the empty key is the
  sender's; a refused message holds its key until collected; a decision taken on silence holds
  its key until the sweep; a read note frees its key; the order for the sender's own key is
  `unknown-inbox`, `duplicate-key`, `inbox-closed`; keys are compared as decoded text; an absent
  or `null` key is refused by decoding; a third principal reads no other sender's key from any
  of 6 views.
- 8 changed copies of the service. The gate noticed 5: the core predicate with the key alone,
  with `to`, with `||`, `duplicate-key` before `duplicate`, and the model with the key alone
  (the interpreted target only). It noticed none of: the core with `on_behalf_of` (only the
  adversary's cases), the model with `to`, the model with `on_behalf_of`. Not run: the model
  with `any:`, and `from == caller.principal`.
- `generated/` regenerated in a copy: no difference. Step 1 changes versions, comments and one
  reason of the unsupported list only. The unsupported list is true of the run. Decision 6 of
  `docs/design/inbox.md` and the doc comment of `fn post_message` are true of the code. No
  name, address, home path or secret in the unit's commits.

```findings
[
{"file":"tools/scenarios.py","line":795,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A model whose duplicate-key compares to instead of from passes all of task check, because scenario 28 refuses the cross-inbox post in an inbox where another sender already holds the key."},
{"file":"crates/inbox-core/tests/dedup_key_scope.rs","line":134,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Scoping the key by on_behalf_of in the core or in the model passes the unit's own tests, scenarios, conform and acceptance, because no case posts with on_behalf_of different from from."}
]
```
