---
format: aep.planning-md/3
id: story:message-id-per-sender
kind: story
status: draft
title: A message id is unique per sender
summary: A stored message is named by sender and message id together, so no answer tells whether another sender used an id and nobody can take an id first.
tags:
- from-finding
relations:
- informed_by: story:post-does-not-tell-a-message-id-exists
revision: 1
---
## Outcome

A message id is unique per sender. A stored message is named by its sender and its
`message_id` together, so no principal can learn from any answer of the service whether another
sender used an id, and no principal can take an id before the sender that chose it posts.

Since story:post-does-not-tell-a-message-id-exists a post under a `message_id` that a stored
message of another sender carries is refused `id-taken` in an open inbox. A principal that
holds a message id learns from that answer whether the id is live, and a principal that posts
under a free id first makes the sender's own post fail with `id-taken`. This story removes
both: `id-taken` goes, and the second sender's post is `posted`.

## Acceptance

Two principals each post a message under the same `message_id` into open inboxes; both are
answered `posted`, and both messages are stored and collected. A repost by either is answered
`duplicate`. No command answers a principal differently for an id another sender used and an id
no message carries.

## Context

Option C of the scope of story:post-does-not-tell-a-message-id-exists (stage 2, wave 7), which
took option B3. Not probed on ESS. The scoper's reading: it changes the wire of the 5 commands
that name a stored message, the backend contract and its kit, and the CLI.

Decided by the host operator's session on 2026-11-21: B3 opens the gate for principals of one
operator only, and this story is required before principals of different operators share one
service. It asked for a `blocks` edge from this story to the epic that first serves a second
operator. On 2026-11-21 the store has no such epic: the 5 epics of `docs/design/inbox.md`
serve one operator, and `docs/design/signing.md` names "more than one operator key" as outside
version 1. Whoever drafts that epic adds the edge
(`aep plan artifact relate story:message-id-per-sender blocks <epic id>`).

A draft. Accepting it is the operator's act.
