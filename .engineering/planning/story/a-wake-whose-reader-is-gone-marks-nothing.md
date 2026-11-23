---
format: aep.planning-md/3
id: story:a-wake-whose-reader-is-gone-marks-nothing
kind: story
status: implemented
title: A wake whose reader is gone marks nothing
summary: A harness that died without a signal, or a hook ended by SIGKILL, leaves a wake that marks the next post delivered though no session reads it
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:23:28Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:23:29Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T13:55:17Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":9}}}
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

- How the wake watches its reader (the Open item of the draft; options: (A) a poll of
  descriptor 1 for an error while it waits, and one check after the rows are written; (B) an
  acknowledgement the reader writes to the wake's standard input before a mark is sent; (C)
  both). Taken by the coordinator when the story was accepted: A. B changes what every wake
  adapter must do and is a story of its own; the window it would close (a reader that took the
  rows and ended before it showed them) stays on the limits page.
- How the Claude Code plugin notices a harness that is gone without a signal (options: (A) a
  watch that looks every 2 s whether the hook's parent process is still the one it started
  with; (B) nothing, the limit stays). Taken: A.
- A post in the up to 2 s between the end of the harness and the watch's next look (adversary
  pass 1, finding 1; options: (A) the window stays open and is named in
  `docs/design/wake.md` and on the limits page, and the adversary's red case is turned into
  `a_post_right_after_the_harness_is_gone_is_marked_an_open_limit`; (B) it is closed with an
  acknowledgement of the reader, a new story). Taken: A.
- A write of a row to the plugin's file `text` that fails (adversary pass 2, finding 1;
  options: (A) the hook writes one byte to the file before the wake starts and checks every
  write of a row; on a failure it closes the FIFO, ends the wake and exits 1 with one line and
  no frame; (B) the rows are held in a shell variable, without a file). Taken: A. B is the
  read loop of round 1, whose time grows with the square of the size of the rows (53.8 s for
  2 MB). What stays open and is named: a round the wake had written and checked when the write
  failed is marked and reaches no session.
- A line of the rows that a signal cut (adversary pass 2, finding 2; options: (A) the cut line
  is dropped whole in every shell; (B) the bytes a shell leaves in the variable are kept, which
  differs between bash and dash). Taken: A. The frame holds whole lines only, and its count is
  of the head lines it holds whole.
- Whether the hook reads on after a signal (round 3; options: (A) it reads the whole lines the
  wake wrote before it ended and stops at the first `read` the signal cut; (B) it reads nothing
  more after a signal, as the coordinator's brief said). Taken: A, on the implementor's
  measurement: under B the case
  `adversary_claude_code_adapter::rows_printed_before_a_signal_are_handed_on_without_a_note`
  failed in 1 of 10 runs, because whole rows the wake had printed, and may have marked, were
  dropped.
- 2 adversary passes are recorded: review-result:adversary-wake-reader-gone-pass-1 (7
  findings, 1 red case) and review-result:adversary-wake-reader-gone-pass-2 (2 findings, 2 red
  cases); every finding has its outcome in the evidence of this story.

## Measured

- After round 3, on the branch before `plan/stage-3` was brought in: `cargo nextest run -p
  inbox-cli --locked` 274 passed (266 before the story), `-p inbox-xtask` 120 passed (113
  before the story).
- Red first, round 3, against the hook of round 2: 4 of the 4 cases of the round failed
  (`a_tmpdir_that_takes_no_byte_starts_no_wake`,
  `a_full_tmpdir_does_not_drop_a_row_the_wake_marked`,
  `a_signal_inside_a_line_drops_the_cut_line_whole`,
  `a_row_that_cannot_be_kept_ends_the_wake_and_hands_on_no_frame`).
- Mutants of round 3, each red against a named case: the probe write removed; the check of a
  row's write removed; the FIFO not closed after a failed write; the cut line kept; the
  reading going on after a `read` a signal cut. Of round 2: the watch comparing the parent
  with 1; a last line without a line end dropped; the rows file left behind; the round cut
  before its marks.
- The read loop of the hook under `bash` as `sh`, rows of 1 kB bodies, load average 49 to 120:
  2 MB in 1.4 to 2.0 s, 8 MB in 5.1 to 9.5 s. The loop of round 1, which kept the rows in a
  variable: 2 MB in 53.8 s, 8 MB over 300 s.
- Times of the new cases: none over 5 s; the slowest is
  `a_signal_inside_a_line_drops_the_cut_line_whole` (4 runs of the hook) at 2.0 to 4.3 s.
- The reader cases of both crates (`test(/reader_gone/) | test(/claude_code_adapter/)`) in a
  row on the final hook: 3 runs of `inbox-cli` and 2 of `inbox-xtask`, all green. The loop was
  cut there: each run waited 13 to 16 minutes for a build slot.
- `dash` and `shellcheck` are not on the build host. The forge runs the hook's cases under
  `dash`.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T13:42:24Z to 2026-11-23T13:55:04Z, load average 68 to 75
  on 20 CPUs: exit 0, `928 tests run: 928 passed, 8 skipped`, the test step 68 s of
  its bound of 120 s.
