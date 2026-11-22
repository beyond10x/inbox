---
format: aep.planning-md/3
id: story:the-claude-code-adapter-wakes-an-idle-session
kind: story
status: active
title: The claude_code adapter wakes an idle Claude Code session on a post
summary: The adapter keeps inbox wake running as a background task of the session; its exit on a post resumes the session, which handles the message and starts the wake again.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:inbox-wake-returns-on-a-post
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: adapters/claude-code
- confidence: inferred
  path: crates/inbox-xtask/tests/tools/claude_code_adapter.rs
- confidence: cited
  path: crates/inbox-xtask/tests/tools/mod.rs
- confidence: inferred
  path: docs/design/wake.md
- confidence: cited
  path: tools/gate-paths.sh
- confidence: cited
  path: website/docs/first-use.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:15Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

A Claude Code session whose inbox is opened with `--adapter claude_code` keeps `inbox wake`
(story:inbox-wake-returns-on-a-post) running as a background task. When a message arrives the
task exits, Claude Code resumes the idle session, the session reads and handles the message, and
the wake is started again.

## Acceptance

An idle Claude Code session whose inbox was opened with `--adapter claude_code` takes its next
turn after a post to that inbox and reads the message, so the message leaves the owner view
`Pending` (which holds `Sent` and `Delivered` only, `ess/domains/messaging.yaml:950`): a request
or a decision is then `Read`, and `Answered` in the sender view `Outbox` when the session answers
it, and a note is removed by its read (state transition: `Delivered` to `Read` without a person
or another process prompting the session; checked by the operator on one session with
`inbox pending` and `inbox outbox`, recorded as evidence of the story; the gate holds the adapter
files against a stand-in harness and makes no model call).

## Context

The epic Outcome: "the harness adapter holds `inbox wait` open as a background task and the
session resumes when it returns", and its Risk, the same as `docs/design/inbox.md` "Risks"
(line 395): waking relies on the harness resuming a session when a background task exits, which
belongs to the harness and can change. `claude_code` is a member of `inbox.messaging.Adapter`,
"The wake surface the owner of an inbox reads it through" (`ess/domains/messaging.yaml:69-73`).
The service stores the value and acts on none of it (no file of `crates/inbox-server/src` names
the adapter). A note read with `inbox read` is removed, so it leaves `Pending`; a request or a
decision read waits in `Read` for its answer (`docs/design/inbox.md`, "The model", the table of
removals, lines 66-74; `ess/domains/messaging.yaml`, transition `read` from `Sent` and
`Delivered`).

The gate cannot hold the resume itself: the test step makes no model call and no request to a
real API (the folder rule "Tests stay fast"). It holds the part of the adapter: the files it
installs start `inbox wake` for the inbox of the session with its key, outbox and URL, and start
it again after the session handled what woke it. That they start no second wake while one runs
is the guard inside `inbox wake` (story:inbox-wake-returns-on-a-post, Open 2 A: a second wake of
the same inbox on the same host exits 5): the adapter writes no guard of its own, and its case
holds that a hook run while a wake runs leaves one wake. Whatever the adapter installs that runs
is POSIX sh or the `inbox` binary (the folder rule "No Python for permanent tools").

## Domain relations

- Inbox to Adapter: the inbox records `claude_code` when opened with it
  (`ess/domains/messaging.yaml`, entity `inbox.messaging.Inbox`, field `adapter`, line 196;
  command `OpenInbox`, `sets: {adapter: input.adapter}`). Inferable from the ess/1 document. What
  an adapter does is declared nowhere in `ess/`.

## Open

1. Where the adapter lives and how it is installed. (A) a Claude Code plugin in this repository
   (`adapters/claude-code/`: a hook that starts `inbox wake` in the background when a turn ends,
   and the instruction the session follows when it wakes), installed by the operator; (B) a skill
   in the operator own skills repository that calls `inbox wake`, and this repository holds only
   the first-use page; (C) a page of the site that the operator copies into each project.
   Default: A.
2. What starts the wake again after each turn. (A) a hook at the end of each turn that starts
   `inbox wake`; a start while a wake runs exits 5 by the guard of `inbox wake`
   (story:inbox-wake-returns-on-a-post, Open 2 A), so the hook checks nothing itself; (B) the
   session itself, told by the instruction to start it before it ends its turn. Default: A.
3. How a change of the harness behaviour is noticed (the Risk). (A) the operator checks this
   acceptance once per release, recorded as evidence; (B) no check beyond this story. Default: A.

## Scope

Order (parallel safety): this story edits no code file of `crates/inbox-cli`; it depends on
story:inbox-wake-returns-on-a-post, which creates the sections of the shared documents it edits.

Code:

- `adapters/claude-code/` (new: the plugin manifest, the hook, the instruction): inferred
- `crates/inbox-xtask/tests/tools/claude_code_adapter.rs` (new, the one test location: runs the
  hook under a stand-in harness and a stand-in `inbox`, as the cases of `tools/gate-paths.sh` and
  `tools/test-step.sh` run their scripts) and one `mod` line in
  `crates/inbox-xtask/tests/tools/mod.rs`: inferred (the directory and `mod.rs` are cited)
- `tools/gate-paths.sh` and its cases (only if the adapter carries Markdown that the gate must not
  skip): cited

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/wake.md`: only under the heading `## claude_code`: inferred
- `website/docs/first-use.md`: only under the heading `### claude_code` of "Wake a session": cited
- `website/docs/status.md`: not edited (the row of the wake is story:inbox-wake-returns-on-a-post):
  cited
- `AGENTS.md`: one new Layout row for `adapters/claude-code/`, inserted after the row of
  `crates/inbox-backend-testkit/`: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
