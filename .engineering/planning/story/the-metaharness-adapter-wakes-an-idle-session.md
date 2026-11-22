---
format: aep.planning-md/3
id: story:the-metaharness-adapter-wakes-an-idle-session
kind: story
status: draft
title: The metaharness adapter wakes an idle metaharness session on a post
summary: A metaharness session opened with the metaharness adapter takes its next turn after a post, through inbox wake; how metaharness resumes a session is open.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:inbox-wake-returns-on-a-post
scope:
- confidence: inferred
  path: docs/design/wake.md
- confidence: cited
  path: website/docs/first-use.md
revision: 4
---
## Outcome

A metaharness session whose inbox is opened with `--adapter metaharness` is woken by a post to
that inbox through `inbox wake` (story:inbox-wake-returns-on-a-post), and takes its next turn.

## Acceptance

An idle metaharness session whose inbox was opened with `--adapter metaharness` takes its next
turn after a post to that inbox, where before it took none until prompted (checked by the
operator on one session and recorded as evidence of the story; the gate holds the part of the
adapter against a stand-in and makes no model call).

## Context

`metaharness` is a member of `inbox.messaging.Adapter` (`ess/domains/messaging.yaml:71`), and the
epic names it among the wake adapters of stage 3. Nothing else in the tree says what metaharness
is or how it resumes a session: besides the enum and the epic, the only mentions are the
projections `generated/` writes from the enum. The same Risk as for every adapter holds
(`docs/design/inbox.md`, "Risks", line 395). Whatever runs is POSIX sh, Rust or the language
metaharness builds (the folder rule "No Python for permanent tools").

## Domain relations

- Inbox to Adapter: the inbox records `metaharness` when opened with it
  (`ess/domains/messaging.yaml`, entity `inbox.messaging.Inbox`, field `adapter`, line 196;
  command `OpenInbox`, `sets: {adapter: input.adapter}`). Inferable from the ess/1 document.

## Open

1. How metaharness resumes an idle session. (A) on the exit of a background task, as Claude Code
   does, so the adapter is the claude_code one with another install path; (B) through a command
   or an API that adds a turn, which the adapter calls when `inbox wake` exits; (C) it offers
   neither, and this adapter leaves the epic while the enum keeps the member. Default: B, to be
   read in the metaharness documentation before the story starts.
2. Where the adapter lives. (A) in the metaharness repository, as a client of `inbox wake`; (B)
   in this repository under `adapters/metaharness/`. Default: A.

## Scope

Order (parallel safety): this story edits no code file of this repository under the default of
Open 2; it depends on story:inbox-wake-returns-on-a-post, which creates the sections of the
shared documents it edits.

Under the default of Open 2 (A) the code of the adapter is in the metaharness repository, as a
client of `inbox wake`. Its surface there is unassessed: nothing in this tree names a path of that
repository, and this story names none. Under Open 2 B the code would be a new directory of this
repository, and its scope is assessed again then.

This repository, the whole surface under the default:

- `docs/design/wake.md`: only under the heading `## metaharness`: inferred
- `website/docs/first-use.md`: only under the heading `### metaharness` of "Wake a session": cited
- `website/docs/status.md`, `website/docs/limits.md`, `AGENTS.md`, `docs/checks.md`: not edited:
  cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
