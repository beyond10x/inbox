---
format: aep.planning-md/3
id: story:inbox-wake-returns-on-a-post
kind: story
status: draft
title: inbox wake holds the wait open until a message arrives and marks it delivered
summary: 'A client command every wake adapter runs: it waits again past the end of a wait and past a restart, and exits on a message after MarkDelivered.'
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:the-client-reconnects-after-a-restart
- informed_by: story:no-test-holds-a-wall-clock-time
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/failure.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/wake.rs
- confidence: inferred
  path: crates/inbox-server/src/service.rs
- confidence: cited
  path: docs/checks.md
- confidence: inferred
  path: docs/design/wake.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 7
---
## Outcome

A new client command, `inbox wake`, is what every wake adapter runs. It holds the `Pending` view
of the caller own inbox open, starts the wait again when the service ends it without a row,
reconnects after a restart of the service (story:the-client-reconnects-after-a-restart), and
exits 0 when the inbox has a row, after it has printed the rows and sent `MarkDelivered` for
each. A harness that resumes a session when a background task exits resumes it on a post.

`inbox wake` also holds the one-wake-per-inbox rule for every adapter (Open 2): a second
`inbox wake` of the same inbox on the same host exits at once with a stated status and one line
on standard error, through a lock beside the outbox file. So no adapter writes a guard of its
own; story:the-claude-code-adapter-wakes-an-idle-session and
story:the-terminal-adapter-shows-a-post rely on this one.

## Acceptance

An `inbox wake` started on an empty inbox exits 0 with the posted message in its output after
another key posts to that inbox, and the message is in `Delivered`, where the wake kept running
across the end of the service wait and across a restart of the service before the post (state
transition: `inbox wait` returns an empty list when the service wait ends and no client command
moves a message from `Sent` to `Delivered`, to a wait that ends only on a message and delivers it).

## Context

`docs/design/inbox.md` "What the specification cannot say": "No blocking read. `inbox wait` is
the `Pending` view held open until it has a row" (line 298). `docs/design/service.md` "The wait"
(lines 215-228): the service ends a wait at `max_wait` (default 60 s,
`crates/inbox-server/src/config.rs:51`) with an empty `rows`, and wakes a held wait whenever a
command is answered on another connection, not by a clock. Today `inbox wait --seconds` makes one
such request and returns what it got (`crates/inbox-cli/src/main.rs:77-87`,
`crates/inbox-cli/src/commands.rs:239-258`).

`MarkDelivered` is declared: "An adapter reports that a live wake surface took the message. It is
sent only after the surface took it" (`ess/domains/messaging.yaml:545`), and the state
`Delivered` is "An adapter handed it to a live wake surface" (line 240). No client command sends
it (the commands of `crates/inbox-cli/src/main.rs`).

The design names no wake mechanism beyond the `Inbox` entity ("its wake adapter",
`docs/design/inbox.md:39`) and the risk "Waking an idle session relies on a harness resuming a
session when a background task exits" (line 395). This story writes that part of the design
(`docs/design/wake.md`, new), and the adapter stories
(story:the-claude-code-adapter-wakes-an-idle-session,
story:the-metaharness-adapter-wakes-an-idle-session,
story:the-terminal-adapter-shows-a-post) build on it.

Tests hold the folder rule "Tests stay fast": no test over 60 s, the test step within 120 s; no
case waits for the 60 s of `max_wait` (Open 3).

## Domain relations

- Message to Inbox: many messages reference one inbox through `to` (`ess/domains/messaging.yaml`,
  entity `inbox.messaging.Message`, relation `inbox`, `kind: references`, `cardinality: one`,
  `via: to`, lines 230-235). Inferable from the ess/1 document.
- Who moves a message to `Delivered`: only the owner of the inbox in `to`; the sender gets
  `not-recipient` and a third principal `unknown-message` (same file, command
  `inbox.messaging.MarkDelivered`, outcomes `foreign-message`, `not-recipient`, `delivered`,
  `already-delivered`, lines 542-571; transition `deliver` from `Sent`). Inferable from the ess/1
  document.
- Wait to Inbox: how many waits one inbox may have at once is declared nowhere; the service holds
  each on a connection of its own. Inferable only as the code does it, inferred from
  `docs/design/service.md` "The wait" (lines 225-228). The guard of one wake per inbox on one host
  is a rule of the client, Open 2; the service stays as it is.

## Open

1. When the surface "took" a message, the moment `MarkDelivered` may be sent. (A) when
   `inbox wake` has written the rows to its standard output, before it exits: the harness holds
   the output; (B) the session sends it when it reads the output (a command `inbox deliver`), and
   `inbox wake` sends nothing; (C) no `MarkDelivered`: the session reads with `inbox read`, which
   moves `Sent` to `Read`. Default: A.
2. One wake per inbox. The need: an adapter that starts `inbox wake` again after each turn (the
   claude_code adapter, Open 2 A of its story) must not start a second one while one runs, or one
   post wakes the session twice. This story owns the guard. (A, default) a guard inside
   `inbox wake`: a lock file beside the outbox file (`<outbox>.wake`), held while the wake runs; a
   second `inbox wake` of the same inbox on the same host exits with a new exit status 5 and one
   line on standard error, `inbox wake: another wake of <address> runs on this host`, and sends
   nothing; (B) no guard: two waits on one inbox are allowed, each wakes, the first
   `MarkDelivered` is `delivered` and the other `already-delivered`, and each adapter keeps its
   own count. Two waits of one principal on two hosts stay possible under both, as the service
   allows them. Default: A.
3. How the 60 s of the epic is held without a test that waits 60 s. (A) the case starts the
   service with a `--max-wait` longer than the case, so only the post can end the wait, and
   asserts that the wake exits with the message after the post, holding no time: the bound then
   follows from the design, which puts no timer between a post and the end of a held wait; the
   start again after the end of a wait is held with `--max-wait 1`; (B) the case holds a bound of a
   few seconds on the wall clock, which story:no-test-holds-a-wall-clock-time takes out of the
   gate. Default: A.
4. How long `inbox wake` keeps trying while the service is down. (A) until it is stopped, with a
   pause between tries that grows to at most 30 s, so a restart is met within the 60 s of the
   epic; (B) it exits non-zero after a bound, and the adapter starts it again. Default: A.
5. A new command or a flag. (A) `inbox wake`; (B) `inbox wait --until-message`. Default: A.

## Scope

Order (parallel safety): this story edits `crates/inbox-cli/src/client.rs`, `src/main.rs`,
`src/reference.rs`, `generated/cli/inbox.json` and the module list of
`crates/inbox-cli/tests/main.rs` after story:the-client-reconnects-after-a-restart (edge
`depends_on`), which edits them after story:the-service-speaks-tls-with-a-pinned-certificate.
story:the-terminal-adapter-shows-a-post edits them after this one.

Code:

- `crates/inbox-cli/src/commands.rs` (`pending`, the new command, `MarkDelivered`): cited
- `crates/inbox-cli/src/main.rs` (the command, its help, exit status 5): cited
- `crates/inbox-cli/src/client.rs` (a request that outlives the connect timeout; a retry): cited
- `crates/inbox-cli/src/outbox.rs` (the path of the wake lock beside the outbox file): cited
- `crates/inbox-cli/src/failure.rs` (exit status 5): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json`: cited
- `crates/inbox-cli/tests/wake.rs` (new) and one `mod` line in `crates/inbox-cli/tests/main.rs`:
  inferred (the directory and `main.rs` are cited)
- `crates/inbox-server/src/service.rs` (the wait; expected unchanged): inferred

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/wake.md` (new): this story creates it with the shared part (`inbox wake`, the
  delivery, the one-wake guard, the 60 s) and one headed section per adapter, `## claude_code`,
  `## metaharness` and `## terminal`, each holding one line that the adapter is not built yet;
  each adapter story edits only under its own heading: inferred
- `website/docs/first-use.md`: a new section `## Wake a session` after "The outbox file", with one
  `###` heading per adapter (`claude_code`, `metaharness`, `terminal`), each holding one line that
  the adapter is not available yet, which each adapter story replaces under its own heading; and
  the row of exit status 5 in "Exit statuses": cited
- `website/docs/status.md`: in the table of "Messages", the row "Wait until a message arrives";
  in "Missing", the bullet "Waking an idle agent session when a message arrives.": cited
- `website/docs/limits.md`: in "The service", the words "and no wake-up" of the bullet "No second
  host and no wake-up": cited
- `docs/checks.md`: in "Open limits and their stories", the words "no wake-up" of the bullet on
  line 86: cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`, the Commands row of the client commands,
  and a new Layout row for `docs/design/wake.md` after the row of `docs/design/inbox.md`: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
