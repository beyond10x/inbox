---
format: aep.planning-md/3
id: story:a-wake-whose-reader-is-gone-marks-nothing
kind: story
status: draft
title: A wake whose reader is gone marks nothing
summary: A harness that died without a signal, or a hook ended by SIGKILL, leaves a wake that marks the next post delivered though no session reads it
revision: 1
---
## Outcome

A wake whose reader went away marks no message delivered: a post that no session read stays in
`Sent`, and the next wake of the inbox returns on it.

## Why

`inbox wake` marks a message delivered once it has written the rows to its standard output. The
`claude_code` adapter hands it a file, reads the file after the wake has ended and writes the
rows to the harness. Adversary pass 1 on story:the-claude-code-adapter-wakes-an-idle-session
showed 2 ways in which the rows reach no one and the message is marked all the same
(review-result:adversary-the-claude-code-adapter-wakes-an-idle-session-pass-1, findings 3 and
4):

- The harness is gone without a signal to the hook (it crashed, or was ended by SIGKILL). The
  hook and its wake live on. The next post is written to the file and marked; the hook's write
  to the harness fails.
- The hook is ended by SIGKILL alone. Its wake lives on, writes the next post to a file no one
  reads and marks it. Whether the harness's time limit ends a hook that way is not verified.

In both the message stays in `Delivered`: `inbox pending` lists it and no wake returns on it.
The wake that was left also holds the wake's lock, so the hooks of later turns end at once
(exit status 5) until it ends. Neither can be closed in the adapter alone: only the wake marks,
and with a file as its standard output it cannot see that its reader is gone.

## Acceptance

- A wake whose standard output is a pipe or a FIFO ends with exit status 4 and marks nothing
  when the reading end is closed while it waits; it notices within a bound the story states,
  without a post having to arrive first, and frees its lock.
- The `claude_code` hook hands the wake a pipe or FIFO that the hook's own shell reads, so a
  hook ended by SIGKILL ends its wake by the rule above and no message is marked.
- A harness that is gone without a signal: the hook notices it within a bound the story states
  (decide how: the parent process, or a write to the harness before the rows are taken) and
  ends its wake before a message is marked; or the story says why that cannot be held and
  what stays open.
- The 2 cases of `crates/inbox-xtask/tests/tools/adversary_claude_code_adapter.rs` that hold
  today's behaviour as an open limit are turned back to what the adversary wrote: no message is
  marked delivered in either.
- A wake on a terminal or a regular file behaves as before.

## Scope

`crates/inbox-cli/src/wake.rs` and `crates/inbox-cli/src/output.rs` (the watch of the reader);
`adapters/claude-code/scripts/wake.sh`; their cases; `docs/design/wake.md`;
`website/docs/limits.md` (the limit leaves the page). No change of the listener.

## Decisions

Open: how the wake watches its reader (a poll of descriptor 1 for an error while it waits; an
acknowledgement the reader writes to the wake's standard input before a mark is sent; both).
The second also closes the window in which a reader takes the rows and dies before it shows
them, and changes what a wake adapter must do.
