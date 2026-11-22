---
format: aep.planning-md/3
id: story:a-due-message-takes-no-mark
kind: story
status: draft
title: A message past its expires takes no read mark and no delivery mark
summary: MarkRead and MarkDelivered act on a due message, so the event the sender of a due note gets depends on when the expiry task ran.
revision: 2
---
## Outcome

A message whose `expires` has passed takes no `MarkRead` and no `MarkDelivered`, whether or not
the expiry task has reached it. The sender of a due note gets the same event whenever the task
runs.

Today both commands act on a due message. A due note that its recipient reads before the expiry
task runs is removed with `MessageHandled`; the same note read after the task ran was removed
with `MessageExpired`. The stored outcome is the same (the note is gone); the event the sender
gets depends on when the task ran.

## Acceptance

- `MarkRead` and `MarkDelivered` on a message whose stored `expires` is at or before the
  service's clock reading change no row and publish no event (state transition: a due message
  in `Sent`, `Delivered` or `Read` stays as it is until the expiry task decides it).
- One case runs the two orders (mark then sweep, sweep then mark) on a due note and finds the
  same events for the sender.
- A caller that is no party of the message still gets `unknown-message`.
- The rule is declared in `ess/`; the core and the HTTP service give the same outcome.

## Context

Found by the adversary pass on story:a-due-message-takes-no-answer
(review-result:adversary-pass-1-on-a-due-message-takes-no-answer, finding 5). It existed before
that story. A draft: accepting it is the operator's act.
