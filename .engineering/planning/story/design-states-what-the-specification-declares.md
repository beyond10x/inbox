---
format: aep.planning-md/3
id: story:design-states-what-the-specification-declares
kind: story
status: draft
title: The design states what the specification declares
revision: 2
---
## Outcome

`docs/design/inbox.md` states each rule the specification declares, and says one thing about
the refusal order of `Answer`.

The design review of 2026-11-22 (hardening, technique 8) read the design against `ess/` and
found 10 places where the specification declares a rule the design does not state, states
otherwise, or states twice:

1. For `Answer` the design puts `not-sender` "before every other refusal of the command" and,
   3 lines on, `foreign-message` "before `not-recipient`, `not-sender`". The specification
   follows the first sentence (finding 8).
2. "`Sent → Delivered → Read → Answered | Refused`" reads as a strict sequence. The lifecycle
   lets a message go from `Sent` to `Read`, `Answered` or `Refused` (finding 10).
3. "`decision` (lettered options, a recommended one)": `recommended_option` is `Optional`
   (finding 11).
4. The summaries of `Reply` and of `Answer` in the specification give a reply a `kind` and a
   `to`. Neither type has them, and the design says a reply has none (finding 13).
5. A closed inbox opens again (`reopen`, outcome `reopened`): no design statement (finding 14).
6. A repeated command succeeds and changes nothing (`already-open`, `already-closed`,
   `already-delivered`, `already-read`): no design statement (finding 15).
7. A reply is checked against `reply_shape` and the options of the decision
   (`shape-mismatch`): no design statement (finding 16).
8. The 10 events and what each carries: no design statement (finding 17).
9. `consistency: read_your_writes` on all 9 views: no design statement (finding 18).
10. The 4 adapters, the address format, the 3 reply shapes, `request_ref` and the client's
    default of 24 hours for `expires`: no design statement (finding 19).

## Acceptance

Each of the 10 has one sentence in `docs/design/inbox.md` that a reader can hold against the
named declaration, the 2 summaries of the specification no longer name `kind` or `to` of a
reply, and a second run of the design review brief returns none of the 10 (state transition:
10 findings of the review change from open to closed).

## Decisions

- The design follows the specification in all 10. A place where the designer wants the other
  reading is taken out of this story and gets its own.
- Not in this story: whether a refusal is signed and whether a `reply_id` is a message id
  (`story:design-decides-refusal-signature-and-reply-id`), and 3 findings held for the operator's decision.

## Scope

- `docs/design/inbox.md`
- `ess/domains/messaging.yaml` (2 summaries), `generated/`
