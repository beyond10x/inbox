<!--
  generated from inbox v1
  contract digest e34e82a0a81f7c01ba3d91fa287c7a723f89e6d1fbdd928b462676dc857fd14a
  do not edit: regenerate with `ess synthesize --layout crate`
-->
# Synthesis plan — inbox v1

Scope: `component-skeletons`, laid out as `crate`, planned by `ess-synth`. Regenerate with `ess synthesize --layout crate`.

90 capabilities: **87 generated**, **3 obligations**, **0 refused**. An obligation is yours to implement against its contract; a refusal is a fact about this synthesis scope, not about the specification.

## Generated

| capability | source |
| --- | --- |
| domain type | `inbox.messaging.Adapter` |
| domain type | `inbox.messaging.Address` |
| domain type | `inbox.messaging.AnsweredBy` |
| domain type | `inbox.messaging.Body` |
| domain type | `inbox.messaging.CollectedReply` |
| domain type | `inbox.messaging.DecisionAsk` |
| domain type | `inbox.messaging.DecisionOption` |
| domain type | `inbox.messaging.DedupKey` |
| domain type | `inbox.messaging.DelegationEntry` |
| domain type | `inbox.messaging.DelegationId` |
| domain type | `inbox.messaging.FreeShape` |
| domain type | `inbox.messaging.Inbox.State` |
| domain type | `inbox.messaging.KeyEntry` |
| domain type | `inbox.messaging.KeyFile` |
| domain type | `inbox.messaging.Kind` |
| domain type | `inbox.messaging.Message.State` |
| domain type | `inbox.messaging.MessageId` |
| domain type | `inbox.messaging.OptionLetter` |
| domain type | `inbox.messaging.Principal` |
| domain type | `inbox.messaging.Reply` |
| domain type | `inbox.messaging.ReplyShape` |
| domain type | `inbox.messaging.Signature` |
| domain type | `inbox.messaging.ThreadId` |
| entity lifecycle | `inbox.messaging.Inbox` |
| entity lifecycle | `inbox.messaging.Message` |
| command contract | `inbox.messaging.Answer` |
| command behaviour | `inbox.messaging.Answer` |
| command contract | `inbox.messaging.CloseInbox` |
| command behaviour | `inbox.messaging.CloseInbox` |
| command contract | `inbox.messaging.Collect` |
| command contract | `inbox.messaging.Expire` |
| command contract | `inbox.messaging.MarkDelivered` |
| command behaviour | `inbox.messaging.MarkDelivered` |
| command contract | `inbox.messaging.MarkRead` |
| command behaviour | `inbox.messaging.MarkRead` |
| command contract | `inbox.messaging.OpenInbox` |
| command behaviour | `inbox.messaging.OpenInbox` |
| command contract | `inbox.messaging.PostMessage` |
| command contract | `inbox.messaging.Refuse` |
| command behaviour | `inbox.messaging.Refuse` |
| event type | `inbox.messaging.DecisionTakenOnSilence` |
| event type | `inbox.messaging.InboxClosed` |
| event type | `inbox.messaging.InboxOpened` |
| event type | `inbox.messaging.MessageAnswered` |
| event type | `inbox.messaging.MessageDelivered` |
| event type | `inbox.messaging.MessageExpired` |
| event type | `inbox.messaging.MessageHandled` |
| event type | `inbox.messaging.MessagePosted` |
| event type | `inbox.messaging.MessageRead` |
| event type | `inbox.messaging.MessageRefused` |
| error type | `inbox.messaging.AlreadyExpired` |
| error type | `inbox.messaging.AlreadySettled` |
| error type | `inbox.messaging.Duplicate` |
| error type | `inbox.messaging.Expired` |
| error type | `inbox.messaging.MessageIdTaken` |
| error type | `inbox.messaging.NoAuthority` |
| error type | `inbox.messaging.NotAnswerable` |
| error type | `inbox.messaging.NotDue` |
| error type | `inbox.messaging.NotRecipient` |
| error type | `inbox.messaging.NotSender` |
| error type | `inbox.messaging.NotSettled` |
| error type | `inbox.messaging.ShapeMismatch` |
| error type | `inbox.messaging.SignatureInvalid` |
| error type | `inbox.messaging.UnknownInbox` |
| error type | `inbox.messaging.UnknownMessage` |
| view type | `inbox.messaging.Due` |
| view query | `inbox.messaging.Due` |
| view type | `inbox.messaging.Inboxes` |
| view query | `inbox.messaging.Inboxes` |
| view type | `inbox.messaging.Messages` |
| view query | `inbox.messaging.Messages` |
| view type | `inbox.messaging.Outbox` |
| view query | `inbox.messaging.Outbox` |
| view type | `inbox.messaging.Pending` |
| view query | `inbox.messaging.Pending` |
| view type | `inbox.messaging.Replies` |
| view query | `inbox.messaging.Replies` |
| view type | `inbox.messaging.Settled` |
| view query | `inbox.messaging.Settled` |
| view type | `inbox.messaging.Thread` |
| view query | `inbox.messaging.Thread` |
| view type | `inbox.messaging.Unanswered` |
| view query | `inbox.messaging.Unanswered` |
| actor grants | `inbox.messaging.Client` |
| actor grants | `inbox.messaging.Sweeper` |
| component port | `inbox-service` |
| component transport | `inbox-service` |

## Ports — yours to provide

What the specification fully determines is generated; what it cannot determine is an obligation. A generated command behaviour or view query reads and writes through the ports below, and they are yours to provide: synthesis generates each port's contract and never an implementation of one, so where instances live stays your decision.

| port | what it answers |
| --- | --- |
| storage | one per entity a generated behaviour or query reads or writes: the instance stored under an identity; storing, replacing and removing one; and every stored instance, in the order the store keeps them |
| context | where a generated behaviour asks it: the caller's attributes, every identity and value the specification says the implementation assigns, and whether each `external:` branch is taken |

## Obligations — yours to implement

| capability | source | why not generated | contract |
| --- | --- | --- | --- |
| command behaviour | `inbox.messaging.Collect` | kept an obligation by a typed response (`response:`) | given `inbox.messaging.Collect` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unknown-message` for an identity no record carries, error `inbox.messaging.UnknownMessage`; `foreign-message` when the existing subject's stored fields satisfy `(to != caller.inbox and from != caller.principal)`, error `inbox.messaging.UnknownMessage`; `not-sender` when the existing subject's stored fields satisfy `from != caller.principal`, error `inbox.messaging.NotSender`; `collected` when the existing subject is in Answered or Refused, deletes `inbox.messaging.Message`, emits `inbox.messaging.MessageHandled`; `not-settled` otherwise, error `inbox.messaging.NotSettled` |
| command behaviour | `inbox.messaging.Expire` | kept an obligation by a narrowed read of the subject's Optional `default_on_silence`, which its branch proves present, for `decided_option`, in `decided-on-silence` | given `inbox.messaging.Expire` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unknown-message` for an identity no record carries, error `inbox.messaging.UnknownMessage`; `not-due` when the existing subject's stored fields satisfy `expires > input.now as timestamp`, error `inbox.messaging.NotDue`; `decided-on-silence` when the existing subject's stored fields satisfy `(kind == decision and state in [Sent, Delivered, Read] and defined(default_on_silence))`, takes `decide_on_silence` of `inbox.messaging.Message`, emits `inbox.messaging.DecisionTakenOnSilence`; `expired` otherwise, removes every `inbox.messaging.Message` where `message_id == input.message_id`, emits `inbox.messaging.MessageExpired` |
| command behaviour | `inbox.messaging.PostMessage` | kept an obligation by `distinct o in decision.options by o.letter as string`, distinct list members no generated behaviour compares, in `unknown-option` | given `inbox.messaging.PostMessage` input, decide and enact exactly one outcome. Selection precedence: on commands with `when_related:`, check `existing_instance` then `exists: false` before input-guarded refusals; choose the first declared input refusal whose guard holds; then check addressed-row existence (`unknown_instance`, and `existing_instance` on commands without `when_related:`); then the held state (`when_subject_state` and `when_subject`), with `wrong_state` only if the selected branch moves from a state the row does not hold; then choose the present `when_related:` predicate refusal whose predicate and optional input guard hold; then accepting and external branches in declaration order. An accepting branch that moves nothing answers in every state. Related-presence predicates do not precede input-guarded refusals. Declared outcomes (declaration order, not selection precedence): `not-sender` when `from != caller.principal`, error `inbox.messaging.NotSender`; `no-authority` when `(on_behalf_of != {fact: from} and not (defined(authority)))`, error `inbox.messaging.NoAuthority`; `already-expired` when `expires <= {fact: created} as timestamp`, error `inbox.messaging.AlreadyExpired`; `request-without-shape` when `(kind == request and not (defined(reply_shape)))`, error `inbox.messaging.ShapeMismatch`; `decision-without-options` when `(kind == decision and (not (defined(decision)) or decision.options.count == 0))`, error `inbox.messaging.ShapeMismatch`; `unknown-option` when `((defined(default_on_silence) and not (defined(decision))) or (defined(decision) and (not (distinct o in decision.options by o.letter as string) or (defined(decision.recommended_option) and not (exists o in decision.options: (o.letter == decision.recommended_option))) or (defined(default_on_silence) and not (exists o in decision.options: (o.letter == {fact: default_on_silence}))))))`, error `inbox.messaging.ShapeMismatch`; `expired-on-arrival` externally decided (the service's Clock port reads at or after the post's `expires`), error `inbox.messaging.AlreadyExpired`; `signature-invalid` externally decided (the SignatureVerifier port does not know the key, finds it lapsed, or finds that the signature does not verify), error `inbox.messaging.SignatureInvalid`; `authority-invalid` externally decided (the SignatureVerifier port finds no live delegation with this id that lets `from` speak for `on_behalf_of`), error `inbox.messaging.NoAuthority`; `unknown-inbox` when no `inbox.messaging.Inbox` carries the identity `input.to` names, error `inbox.messaging.UnknownInbox`; `duplicate` when, of the `inbox.messaging.Message` rows satisfying `(message_id == input.message_id and from == input.from)`, at least one row is selected, error `inbox.messaging.Duplicate`; `duplicate-key` when, of the `inbox.messaging.Message` rows satisfying `(dedup_key == input.dedup_key and from == input.from)`, at least one row is selected, error `inbox.messaging.Duplicate`; `inbox-closed` when the `inbox.messaging.Inbox` that `input.to` names satisfies `state == Closed`, error `inbox.messaging.UnknownInbox`; `id-taken` when, of the `inbox.messaging.Message` rows satisfying `message_id == input.message_id`, at least one row is selected, error `inbox.messaging.MessageIdTaken`; `posted` otherwise, creates `inbox.messaging.Message`, emits `inbox.messaging.MessagePosted` |

## Refused — not represented by this synthesis

| capability | source | stage | why |
| --- | --- | --- | --- |
