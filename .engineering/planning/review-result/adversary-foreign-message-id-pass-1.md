---
format: aep.planning-md/3
id: review-result:adversary-foreign-message-id-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:foreign-message-id-answers-as-unknown (d9b724d): 4 findings (2 warnings, 2 notes), 1 there before the unit'
relations:
- reviews: story:foreign-message-id-answers-as-unknown
revision: 1
---
## What was reviewed

story:foreign-message-id-answers-as-unknown at `d9b724d` on base `e353a41` (branch
`impl/foreign-message-id`), by one adversary on Opus, pass 1 of at most 2, on 2026-11-21.
No second pass ran: the corrections changed no guard, no source line of a crate and no scenario.

## Verdict

4 findings: 2 warnings and 2 notes. 3 were introduced by the unit, 1 was there before it. On the
running service the 5 commands answered a third principal with the same bytes for a message
between others as for an id nobody used, in every state tried.

## Findings

- F1 (warning, there before the unit; `crates/inbox-core/src/obligations.rs`, the `duplicate`
  check of `post_message`): `post` tells a third principal whether a message id is in use. A
  post as itself to an address no inbox has is answered `duplicate` under the id of a message
  between two others and `unknown-inbox` under an id nobody used, and stores nothing. The case
  `adv_w6_a_third_principal_cannot_tell_a_message_between_others_from_no_message_by_post` in
  `crates/inbox-server/tests/adversary_w6_foreign.rs` fails the same way on `e353a41`.
- F2 (warning, introduced; `docs/design/service.md`, `docs/design/inbox.md` decision 5 and the 5
  `summary:` lines of `foreign-message` in `ess/domains/messaging.yaml`): the unit stated without
  restriction that a third principal cannot tell a message between others from no message. F1
  shows that it holds for the 5 commands only.
- F3 (note, introduced; `ess/domains/messaging.yaml`, the 5 guards): no case held who is a party
  of a relayed message. With `on_behalf_of != caller.principal` in place of
  `from != caller.principal` in the 5 guards, regenerated, every lane stayed green: drift,
  coverage, the interpreted target, the core lane 97 of 97, the serve lane 22 passed and 75
  unsupported, `cargo test --workspace` and the 3 acceptance checks. Under that change a session
  that relays a request for another principal is answered `unknown-message` by `read` and can
  never collect the answer.
- F4 (note, introduced; `docs/design/service.md`): the sentence that the outcome moves nothing a
  wait reads is untrue. Every answer of a request that is not a GET wakes the held-open waits.
  It tells nothing: an `unknown-message` answer wakes them the same way.

## Changed copies (4 built, 3 judged by reading)

| Change | `cargo test` | conformance lanes | acceptance |
|---|---|---|---|
| `as_unknown` also on GET answers | passes | pass | not run; no GET answer carries the outcome |
| `on_behalf_of` compared in the 5 guards | passes | pass | passes |
| the status kept at 403, the name rewritten | fails (`adversary.rs`) | pass | passes |
| the name kept, the status rewritten | fails | not run | not run |

By reading, not built: `any:` in place of `all:` refuses the recipient its own commands; the
outcome behind `not-recipient` or removed gives a third principal 403, which
`crates/inbox-server/tests/foreign_message.rs` checks per route; a generated guard edited by
hand fails `task drift`.

## What held

- The whole response on the wire for an existing and a missing id, as a third principal, on the
  5 routes: status line, headers and their order, body bytes, the connection close. In the
  states sent, delivered, read, answered, refused, collected and gone, expired and gone, decided
  on silence, and with the inbox closed. The store is unchanged after each.
- `answer` as a third principal with a wrong option letter, a reply too long for its shape, an
  expired reply, no authority, a delegation the verifier refuses, no signature, a `from` of
  another principal, and a body that fails decoding: the same answer for an existing and a
  missing id in each.
- The mapping: a post body and a refusal reason that contain the text of a `foreign-message`
  answer do not get a `posted`, `refused` or `collected` answer rewritten; only the top-level
  `outcome` member is matched; an answer that is not JSON passes through; the rewritten body is
  the generated `unknown-message` body on every route; the function compares no caller.
- The views and the wait: a third principal reads no row of the message, and a wait wakes the
  same for every command answer.
- The parties: the sender gets `not-recipient`, the recipient `not-sender` from `Collect`, and a
  relaying `from` is a party.
- The claims: `a-third-principal-finds-no-message` expects `foreign-message` by name; the moved
  steps of `only-the-owner-handles-its-inbox` are refused for the reason their ids name; the
  line of `unsupported.tsv` gives the run's reason.

## Cases

1 case in `crates/inbox-server/tests/adversary_w6_foreign.rs`, red on `d9b724d` and on
`e353a41`. 1 probe outside the tree for F3, green on `d9b724d` and red on the changed guard.
