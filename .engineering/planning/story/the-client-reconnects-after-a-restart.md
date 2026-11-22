---
format: aep.planning-md/3
id: story:the-client-reconnects-after-a-restart
kind: story
status: active
title: A client that meets a new start of the service opens its inbox again and reposts by itself
summary: On the first answer from a new start the client sends OpenInbox with its last adapter and reposts its kept envelopes, with no typed open or repost.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:the-service-speaks-tls-with-a-pinned-certificate
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/reconnect.rs
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:14Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:14Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

After a restart of the service a client opens its own inbox again and posts its kept envelopes
again by itself, on the first command that meets the new start, so nobody types `inbox open` or
`inbox repost`.

## Acceptance

After a restart of the service, the first command of the owner opens its inbox again with the
adapter it last opened it with, and the next command of the sender puts its kept unanswered
request into the owner `Pending` once, with neither `inbox open` nor `inbox repost` run (state
transition: a restarted service that holds no inbox and no message until both are typed, to one
that holds them after the next command of each client).

## Context

`docs/design/inbox.md` "Shape" (lines 26-28): "The sender keeps each message until it is handled
and posts it again after a reconnect; the owner opens its inbox again." `OpenInbox` says the
same: "A client sends this after every reconnect, because a restarted service holds no inbox"
(`ess/domains/messaging.yaml:336`). The epic: "the client reposting its outbox by itself after a
reconnect". Stage 2 left both to a typed command (story:cli-client): `inbox open`
(`crates/inbox-cli/src/main.rs:54-60`) and `inbox repost` (`crates/inbox-cli/src/commands.rs:634`).

A client tells a new start by the header `Inbox-Epoch` every answer carries
(`docs/design/service.md`, "The epoch", lines 77-98; `crates/inbox-cli/src/client.rs:153-250`),
and `repost` already posts only the kept envelopes the epochs file records at another epoch or at
none (`crates/inbox-cli/src/outbox.rs`). Until the owner opened its inbox again a repost is
refused `unknown-inbox`, which keeps the envelope and records no epoch (`AGENTS.md`, the row of
`crates/inbox-cli/`), so it arrives on a later command. The restart rule of stage 3 (`inbox.md`,
"Stages", lines 368-369: "after a restart of the service the sender repost arrives once") is
checked across client hosts by story:the-stage-3-acceptance-checks.

The limit of `website/docs/limits.md` lines 54-66 stays: the first repost after a restart
delivers once more a note its recipient read before it. This story makes that repost happen
without a typed command; it changes nothing of the limit.

The adapter the client last opened its inbox with is kept on the client host, in a file beside
the outbox file as the epochs file is. It is no entity of `ess/`: the service holds the value in
`Inbox.adapter`, and the file is what the client needs to send `OpenInbox` again.

## Domain relations

- Client to Inbox: a client opens one inbox, its own. `OpenInbox` is refused `not-owner` when
  `address != caller.inbox` (`ess/domains/messaging.yaml`, command `inbox.messaging.OpenInbox`,
  outcome `not-owner`, line 341), and the actor `inbox.messaging.Client` carries one `inbox`
  attribute (same file, `actors`, line 252). Inferable from the ess/1 document (an outcome guard
  and an actor attribute, not a `relations:` entry).
- Inbox to Adapter: each inbox has one adapter, set by `opened` and by `reopened` (same file,
  entity `inbox.messaging.Inbox`, field `adapter`, line 196; `OpenInbox`, `sets: {adapter:
  input.adapter}`). Inferable from the ess/1 document (a field: `Adapter` is an enum, so there is
  no `relations:` entry).
- Kept envelope to epoch: zero or one recorded epoch per message id, replaced by each answer that
  says the service holds the message. Inferable, inferred from `crates/inbox-cli/src/outbox.rs`
  (the epochs file, `record_epoch`, line 131) and `docs/design/service.md` "The epoch"; no ess/1
  document declares it.

## Open

1. Which commands reconnect. (A) every client command whose answer carries an `Inbox-Epoch` other
   than the last one the client recorded; (B) only `inbox wake` and `inbox post`: this ties the
   reconnect to `inbox wake`, the command story:inbox-wake-returns-on-a-post adds, so picking B
   reverses the `depends_on` edge between the two stories (this story then depends on the wake
   story, and the wake story loses its edge on this one) and cannot be picked without that change
   of the edges; (C) a long-running client process per host that reconnects for every session.
   Default: A.
2. In which order. (A) the command own request first, then the open and the repost, so a typed
   command answers as before and reports the reconnect after it; (B) the open and the repost
   first. Default: A.
3. How a reconnect shows. (A) one line on standard error through `src/output.rs` and a member
   `reconnected` in the `--json` document; (B) nothing. Default: A.

## Scope

Order (parallel safety): this story edits `crates/inbox-cli/src/client.rs`, `src/main.rs`,
`src/reference.rs`, `generated/cli/inbox.json` and the module list of
`crates/inbox-cli/tests/main.rs` after story:the-service-speaks-tls-with-a-pinned-certificate,
which edits the same files (edge `depends_on`). story:inbox-wake-returns-on-a-post edits them
after this one.

Code:

- `crates/inbox-cli/src/commands.rs` (`open`, `repost`, `command_in_epoch`, the commands that
  read an answer): cited
- `crates/inbox-cli/src/client.rs` (the epoch of an answer): cited
- `crates/inbox-cli/src/outbox.rs` (the epochs file; the file of the opened inbox beside it): cited
- `crates/inbox-cli/src/main.rs` (the help of `open` and `repost`): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json`: cited
- `crates/inbox-cli/tests/reconnect.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred (the directory and `main.rs` are cited)

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/service.md`: the last paragraph of "The epoch" (what `repost` reads it for): cited
- `website/docs/first-use.md`: the section "The outbox file": cited
- `website/docs/status.md`: in the table of "Messages", the row "Post again after a restart of the
  service, from the sender outbox file": cited
- `website/docs/limits.md`: the section "`inbox repost` can deliver a read note once more", and in
  "The service" the bullet "One backend, in memory": cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/` and the Commands row of the client commands:
  cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
