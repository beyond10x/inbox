---
format: aep.planning-md/3
id: story:the-terminal-adapter-shows-a-post
kind: story
status: active
title: The terminal adapter shows each post in its owner terminal and marks it delivered
summary: An inbox opened with the terminal adapter, the default of inbox open, shows each post in a terminal without a typed pending; whom it serves in stage 3 is open.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:inbox-wake-returns-on-a-post
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/output.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/terminal_adapter.rs
- confidence: inferred
  path: docs/design/wake.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/status.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:17Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

An owner whose inbox is opened with `--adapter terminal`, the default of `inbox open`, is shown
each post in the terminal it works in without typing `inbox pending`, and the message moves to
`Delivered`.

## Acceptance

A terminal that runs the `terminal` adapter for an inbox shows a line naming the sender and the
kind of a message posted to that inbox, and the message is in `Delivered`, where before nothing
showed until `inbox pending` was typed (state transition: `Sent` to `Delivered` on a post, with no
typed command; written for Open 1 B).

## Context

`terminal` is a member of `inbox.messaging.Adapter` (`ess/domains/messaging.yaml:71`) and the
default of `inbox open --adapter` (`crates/inbox-cli/src/main.rs:58-60`). The epic names it among
the wake adapters of stage 3. epic:human-inbox names `slack` and `terminal` as "the two adapters of
a person inbox" (stage 4). So whom the terminal adapter serves in stage 3, an agent session that
runs in a terminal or a person, is not settled (Open 1). It builds on `inbox wake`
(story:inbox-wake-returns-on-a-post). Every line it prints goes through `src/output.rs`, which
shows a control character of another principal as its escape (`AGENTS.md`, the row of
`crates/inbox-cli/`).

## Domain relations

- Inbox to Adapter: the inbox records `terminal` when opened with it (`ess/domains/messaging.yaml`,
  entity `inbox.messaging.Inbox`, field `adapter`, line 196). Inferable from the ess/1 document.
- Who moves a message to `Delivered`: only the owner of the inbox in `to` (same file, command
  `inbox.messaging.MarkDelivered`, lines 542-571). Inferable from the ess/1 document.

## Open

1. Whom the terminal adapter serves in stage 3. (A) an agent session in a terminal multiplexer:
   the adapter types a prompt into its pane, which wakes it as a person typing would; (B) a person
   or an agent at a shell: a mode of `inbox wake` in a terminal of its own prints each message,
   rings the bell and waits again, and stage 4 builds on it; (C) moved to stage 4
   (epic:human-inbox), and this epic covers `claude_code` and `metaharness` only. Default: B.
2. Whether the adapter exits on a post, as for `claude_code`, or keeps running. (A) keeps running
   and shows each post; (B) exits on the first. Default: A.

## Scope

Order (parallel safety): this story edits `crates/inbox-cli/src/main.rs`, `src/reference.rs`,
`generated/cli/inbox.json` and the module list of `crates/inbox-cli/tests/main.rs` after
story:inbox-wake-returns-on-a-post (edge `depends_on`), which edits them after
story:the-client-reconnects-after-a-restart and story:the-service-speaks-tls-with-a-pinned-certificate.
story:the-service-binds-a-reachable-address edits them after this one.

Code:

- `crates/inbox-cli/src/commands.rs`, `crates/inbox-cli/src/main.rs` (a mode of `inbox wake`):
  cited
- `crates/inbox-cli/src/output.rs` (the line shown): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json`: cited
- `crates/inbox-cli/tests/terminal_adapter.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred (the directory and `main.rs` are cited)

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/wake.md`: only under the heading `## terminal`: inferred
- `website/docs/first-use.md`: only under the heading `### terminal` of "Wake a session": cited
- `website/docs/status.md`: one row appended at the end of the table of "Messages": cited
- `AGENTS.md`: the Commands row of the client commands (the mode of `inbox wake`): cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
