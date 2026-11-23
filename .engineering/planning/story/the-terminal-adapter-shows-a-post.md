---
format: aep.planning-md/3
id: story:the-terminal-adapter-shows-a-post
kind: story
status: implemented
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:17Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-23T07:49:38Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":12}}}
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

## Decisions

The 2 open items were taken at their defaults (approval-record:stage-3-six-stories-on-default):
1 B (a person or an agent at a shell runs a mode of `inbox wake` in a terminal of its own) and
2 A (the mode keeps running and shows each post).

Taken while building:

- The mode is the flag `inbox wake --follow`, not a new command: it is the wake's round (read,
  write and flush the rows, mark each delivered) run again and again, with the wake's lock, its
  exit statuses and its retry rules. `inbox wake` without the flag is byte for byte as before.
- The bell is one BEL (0x07) after the rows of a round, written by the client and not through
  the printer, only when standard output is a terminal; `--json --follow` writes one document
  per line and no bell. A BEL in a sender's text stays shown as its escape.
- The mode is for an inbox opened with `--adapter terminal`. It reads this host's record
  (`<outbox>.opened`) at its start, before the lock and any request, and again in every round
  before it writes the rows; when the record names another adapter it writes no row, marks
  nothing and ends with 1 and a line that names `inbox open --adapter terminal`. Not taken: the
  check at the start only (round 1; adversary pass 1 finding 1: after `inbox open --adapter
  claude_code` the mode kept the lock, showed and marked the posts, and the plugin's wake ended
  with 5 without a word), and narrowing the design's sentence instead of the code. Still open,
  and written in `docs/design/wake.md`: an `inbox open` does not end a read the service holds,
  so the mode sees the new record only when a row answers its read; that row stays `Sent`.
- A mark that fails is named on standard error and the mode goes on after the wake's pause. A
  round that holds a row the round before marked with success takes the same pause (adversary
  pass 1 finding 3: a service that answers a mark 2xx and keeps the row in `Sent` made the mode
  write the row without end and without a pause). Only the ids of the round before are kept.
- Scope, corrected: the story's Scope named `commands.rs` and `main.rs`; the mode is in
  `crates/inbox-cli/src/wake.rs`, with one variant in `failure.rs` (`OtherAdapter`) and the
  bell in `output.rs`.
- Scope, widened: `output::visible` shows the characters that change the direction of text
  (U+061C, U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069) as their escapes, in every
  command that prints text another principal wrote. They were printed as sent before this
  story too (adversary pass 1 finding 2, pre-existing); the mode puts such text on a terminal
  for as long as it runs, so the fix ships with it. `--json` is unchanged. Not taken: every
  other character without width (U+200B, U+2060 to U+2064, U+FEFF, U+00AD, the tag characters),
  which is story:a-character-a-reader-cannot-see-is-shown-as-its-escape, a draft.
- The case of the bell runs the binary under `script` of util-linux, which makes the pseudo
  terminal; no new dependency. On a host without it the cases on a terminal fail with a line
  that names it, and are not skipped. They read Linux `/proc`. `docs/checks.md` says so.
- The reset of the pause after a clean round is held by no case: only an upper bound on the
  wall clock would see a pause that grew, and no case holds one.
- Written by the coordinator on this branch, outside the Scope below: `CHANGELOG.md` (one entry
  under Added, one under Security), one sentence each in `README.md`, `docs/checks.md`,
  `website/docs/first-use.md` ("What the commands print") and the row of `crates/inbox-cli/` in
  `AGENTS.md`, and the message of the adversary's helper that starts a process.


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

## Measured

- Failing first, round 1: the 7 cases of `crates/inbox-cli/tests/terminal_adapter.rs` red
  against the wake without the flag (`error: unexpected argument '--follow' found`, exit 2
  against 1; `the note is shown: not within 10 s`; `the binary runs under script: not within
  10 s`), `7 tests run: 0 passed, 7 failed`. The acceptance case was red for a wrong reason in
  its first run (a request without `--reply-shape` refused with 422); its helper was fixed and
  it ran red again on `the request is shown: not within 10 s`.
- Round 1 green: `cargo nextest run -p inbox-cli --locked` 3 runs in a row, `253 tests run: 253
  passed`. 9 mutants of the change, each red against a named case (the mark dropped, the
  wait-again dropped, the bell to a pipe, no bell on a terminal, the adapter check dropped, the
  adapter check without the flag, the flag changing the path without it, the rows after the
  mark, a failed mark ending the mode).
- Adversary pass 1, on round 1: 7 findings, 9 cases, 3 of them red. After `inbox open --adapter
  claude_code` the mode kept the lock and a plain `inbox wake` ended with 5 (`left: 5 right:
  5`). U+202E reached the terminal as sent (`U+202E reached the terminal raw`). A stand-in
  service that answered every mark 200 and kept the row in `Sent` got the row written 6 times
  with gaps under 12 ms. 6 mutants passed the 7 cases (the bell per row, the bell with
  `--json`, the bell in the plain wake, no pause after a failed mark, the adapter check before
  the check of standard output, the pause not reset); 6 green cases of the pass kill 5 of them.
  3 documents were wrong: the list "Missing" of the status page, "What it leaves out" of the
  design, and the first-use section, whose 2 commands ended with 2 as written.
- Round 2 closed the 7, each fix red first: `5 failed` of 13 selected cases before (the
  adversary's 3, a new case for the record in a later round, a unit case for the direction
  characters), then `cargo nextest run -p inbox-cli --locked` 3 runs in a row, `266 tests run:
  266 passed`. 3 mutants, each red: the check of the round dropped (the adapter `did not end
  within 10 s`), U+202E taken out of the range, the pause for a row that comes back dropped
  (`the row shown 9 times`). The first-use section run as written against `inbox serve` on
  loopback: `inbox open --adapter terminal` exit 0, the first row shown, one bell.
- Adversary pass 2, on round 2: 4 findings, no red case against the product, 9 mutants: 8 red,
  1 survived (ids of failed marks kept: only an upper bound on the wall clock would see it).
  The pass 1 case for the lock could go red with a correct product: with the adapter stopped
  by SIGSTOP from the post until the plain wake ended, the plain wake met the lock (`left: 5
  right: 5`), and under 2 mutants the case hung to the 60 s limit instead of failing. The
  coordinator changed that case as the pass says: the adapter ends first, by itself, with 1,
  then the plain wake runs with a bound and exits 0 with the post. With the check of the round
  taken out again the case fails at its bound (`the process did not end within 10 s`, 10.4 s). The sentence of the first-use page
  that text without control characters is printed as sent was false and is corrected; the page
  and the design say that a sender's literal `\u{202e}` and the escape of the character read
  the same in text output.
- No adversary pass ran on the coordinator's change of that case and of the documents.
- Times: no case of the 2 files over 5 s; the slowest in 20 runs of the 7 cases together was
  4.0 s (`a_message_delivered_before_the_start_is_not_shown`), 0 red of 20. The cases of the
  terminal files, 3 runs in a row after the last change: `21 tests run: 21 passed` each, the lock case 0.53 to 0.58 s.
- Not held by a case: the reset of the pause after a clean round; SIGHUP of a closed terminal;
  a reader of a pipe that goes away in a later round; the plugin's hook started for real
  against a running `--follow`.
- Not run: the mode in a terminal a person watches (the cases use the pseudo terminal of
  `script`); another host than Linux.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T07:46:22Z to 2026-11-23T07:48:50Z, load average 90 to 92
  on 20 CPUs: exit 0, `881 tests run: 881 passed, 8 skipped`, the test step 56 s of
  its bound of 120 s.
