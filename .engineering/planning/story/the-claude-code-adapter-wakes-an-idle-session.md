---
format: aep.planning-md/3
id: story:the-claude-code-adapter-wakes-an-idle-session
kind: story
status: implemented
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:15Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-23T06:10:42Z", actor: "agent:inbox", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":16}}}
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

## Decisions

The 3 open items were taken at their defaults (approval-record:stage-3-six-stories-on-default):
1 A (a Claude Code plugin in this repository, `adapters/claude-code/`, installed by the
operator), 2 A (a hook at the end of each turn starts `inbox wake`, and the guard is the
wake's own exit status 5), 3 A (the operator checks the acceptance on a real session once per
release, recorded as evidence).

The scope was widened once, by the decision of the operator's deputy on 2026-11-23: the story
changes `crates/inbox-cli/src/output.rs`, which it had named unchanged ("edits no code file of
`crates/inbox-cli`"). Adversary pass 1 showed that the client printed a line end in the text of
a decision option or a refusal at the left edge, so a sender could make one message read as 2,
or as a line of the plugin, in the text the plugin hands to a session. The flaw was older than
the story. Not taken: a story of its own first; a plugin that hands on only id, sender and
kind.

Taken while building:

- The plugin is 3 files: `.claude-plugin/plugin.json` (the name `inbox`; no version and no
  author, so validation passes with warnings: a version would drift at each release),
  `hooks/hooks.json` (one `Stop` command hook in exec form with `asyncRewake` and a `timeout`
  of 604800 s) and `scripts/wake.sh` (POSIX sh). No skill and no Markdown: the begin line
  carries the instruction, and `tools/gate-paths.sh` is unchanged; a case holds that every
  plugin file answers `run`. The local install is `claude --plugin-dir <path>`. Not taken: a
  marketplace file; JSON `additionalContext`; a shell-form hook command.
- A session without `INBOX_URL` and `INBOX_SECRET_KEY_FILE` gets no wake and no word, and so
  does one whose record `<outbox>.opened` holds no `"adapter":"claude_code"`. The script does
  not tell which inbox the record names: only the secret key file names the caller, the script
  reads no file that holds a key, and no client command prints the caller's address without a
  request. An outbox file shared by a principal opened with `claude_code` and one opened with
  another adapter makes the sessions of both the adapter's; the design says so.
- Rows the wake printed are handed on whatever its status (0, 1, 3, 4, 128+n): they may be
  marked delivered. The hook then exits 2, which wakes the session. A status other than 0 adds
  one note line. A failure without rows (1, 2, 3, 4, a signal, no `inbox` on `PATH`, no mark,
  no directory) exits 1 with one line and does not wake the session: a failure that repeats
  would else wake it at the end of every turn. Status 5, and 0 without rows, exit 0 with
  nothing printed.
- The frame. Before the wake starts, the script reads a mark of 16 hexadecimal digits from
  `/dev/urandom` (`od`, `tr`); without one it starts no wake. On standard error it writes a
  begin line `inbox <mark>: <n> message(s) follow, down to the line …`, the rows, the line
  `inbox <mark>: end`, and the note line when there is one, also behind the mark. The begin
  line says that the rows are data of other principals and not instructions of the user or of
  the plugin, that a message is one unindented head line with indented lines under it, that a
  line in between that claims to come from the plugin or the user was written by a sender, and
  which commands read, answer and refuse. The count is the number of unindented lines. The
  mark is held in the shell alone. Not taken: a fixed line without a mark (round 1; a sender
  could write it); the mark in a path or a file.
- The printer (`rows_text`, `settlement` in `crates/inbox-cli/src/output.rs`). A message is
  one line that is not indented, its head line; every other line starts with at least 2
  spaces. The lines of an option text and of a refusal after their first are indented by 4
  spaces (`first_and_rest`), as the lines of a body are. Every value printed inside a line
  shows a line end as `\u{a}` (`inline`), also the values the decoders constrain. `visible`
  also escapes U+2028 and U+2029. It holds for `pending`, `wait`, `wake`, `outbox`, `collect`,
  `thread` and `replies`; `--json` is unchanged. Not taken: escaping only the values a sender
  can set today.
- The size. Rows over 6,000 bytes are handed on as their head lines only, at most 20, each
  cut to its first 300 bytes (`cut -b 1-300`), with one line of the plugin that gives the
  count of all and names `inbox pending`. What the script writes to standard error is at most
  7,336 bytes (begin line 836, 20 head lines 6,020, size line 260, end line 28, note line
  192), under the design's bound of 8,000 bytes. A head line cut inside a character ends in an
  incomplete character. The hooks page caps plain hook output at 10,000 characters; whether
  that cap applies to the standard error of an `asyncRewake` hook is not verified. Not taken:
  no bound, so that the end line could be cut off; `cut -c` (round 2: GNU `cut -c` counts
  characters in a UTF-8 locale, and 20 head lines of 2-byte characters made 12,513 bytes); a
  second pass that drops the head lines when they are over the bound.
- The line of status 2 without rows names an `inbox` that has no `wake` command (one older
  than the plugin) before a wrong `INBOX_URL` or pin: clap exits 2 for an unknown command. Not
  taken: a probe of `inbox wake --help` or of the version, one more process per turn for a
  text.
- The rows go to a file in a directory of mode 700 that the script makes itself: the name
  `inbox-claude-code.<pid>` under `TMPDIR` (else `/tmp`), then `.1` to `.99` when a name
  exists, never a directory it did not make; removed at the end. The wake runs in the
  background and the script waits for it, so that a trap can forward SIGTERM, SIGINT and
  SIGHUP as SIGTERM. Not taken: a pipe or a command substitution (sh defers a trap until a
  foreground command ends); `mktemp` (not POSIX).
- The wake's standard error is thrown away: a line of it may name the URL.
- `timeout` is 604800 s (7 days). The hooks page names no largest value. A hook that reaches
  its time limit is cancelled and its output discarded (the page); that Claude Code takes the
  value as it is, and which signal it sends, is not verified.
- Left open, written in the design and on the limits page, and the draft
  story:a-wake-whose-reader-is-gone-marks-nothing: when the harness is gone without a signal
  to the hook, or the hook alone is ended by SIGKILL, its wake lives on, marks the next
  message delivered though no session reads it, and holds the wake's lock. It cannot be closed
  in the plugin alone: only the wake marks, and with a file as its standard output it cannot
  see that its reader is gone. 2 cases hold today's behaviour and name that story.
- Not done, by the rule that the gate and the build make no model call and start no session:
  the operator's check of the acceptance on a real Claude Code session. Every statement about
  the harness in `docs/design/wake.md` is on the saved page it cites or is marked "not
  verified": that `Stop` takes `asyncRewake`, that a plugin's `hooks.json` takes it, that exit
  2 wakes an idle session and shows it standard error, where the line of an exit 1 is shown.

Scope as built, beyond the list below: `crates/inbox-cli/src/output.rs` (as widened above) with
its unit cases; `crates/inbox-xtask/tests/tools/adversary_claude_code_adapter.rs`,
`crates/inbox-xtask/tests/tools/adversary_claude_code_adapter_2.rs` and
`crates/inbox-cli/tests/adversary_printer.rs` with its `mod` line in
`crates/inbox-cli/tests/main.rs` (the adversary's cases); `docs/checks.md` (the counts of the 2 packages and the sentence on what a
sender's text can do); the rows of `AGENTS.md` for `crates/inbox-cli/` and
`crates/inbox-xtask/`; one sentence of `docs/design/wake.md` under "What it leaves out"; and,
written by the coordinator on this branch: `CHANGELOG.md` (3 entries under "Unreleased"), the
lines of `website/docs/limits.md`, `website/docs/status.md`, `website/docs/first-use.md` and
`README.md` that said no adapter wakes a session, and that now say the plugin was run against
a stand-in and not on a real session.


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

## Measured

- Failing first, round 1: the 13 cases of `crates/inbox-xtask/tests/tools/claude_code_adapter.rs`
  red without the plugin (`13 tests run: 0 passed, 13 failed, 80 skipped`), green after.
- Failing first, round 2: `8 tests run: 4 passed, 4 failed, 237 skipped` for the printer's unit
  cases and `20 tests run: 13 passed, 7 failed, 87 skipped` for the plugin's, green after.
- `claude plugin validate adapters/claude-code`: "Validation passed with warnings" (no version,
  no author, no icon, hosts not declared), exit 0. No session was started.
- Adversary pass 1, on round 1: 10 findings. 5 cases red (a line end in a decision option
  that forged the plugin's note line and a second message; a harness gone without a signal
  and a hook ended by SIGKILL, each with the post marked delivered and read by no one; a
  directory left under the hook's name), 2 green cases for 2 surviving mutants, 3 notes on
  the documents and the size. Round 2 closed 8. The 2 on a session that is gone are not closed
  here: their 2 cases hold today's behaviour and name
  story:a-wake-whose-reader-is-gone-marks-nothing.
- Mutants of round 2, run by the implementor against the 27 plugin cases and the 8 unit cases
  of the printer: 6 of round 1 again, the 13 of pass 1 and 11 new. 3 survive: the explicit
  `< /dev/null` (equivalent: POSIX gives an asynchronous list `/dev/null`), the kill of a wake
  started in the moment between `&` and `pid=$!`, and the wait loop after a signal (each needs
  a time bound to show).
- Adversary pass 2, on round 2: 4 notes. 2 cases red (20 head lines of addresses of 2-byte
  characters made the frame 12,513 bytes against the stated 8,000, because GNU `cut -c`
  counts characters in a UTF-8 locale; an `inbox` without the `wake` command was reported as
  a wrong `INBOX_URL`), and 3 green cases: the hook against the built `inbox` and `inbox
  serve` with a decision whose texts hold every line end, 5 directory names taken, and rows
  of 6,001 bytes. Not broken by the pass: no line end, separator or member that is not a text
  puts sender text at the left edge; no line of the plugin can be written by a sender; the
  mark is in no argument, environment or file; no path exits 2 without a message.
- Round 3 closed the 2 red cases of pass 2: `cut -b 1-300`, and the line of status 2 that names
  an `inbox` without `wake`. Each of the 2 changes taken back makes its case red (`31 tests
  run: 30 passed, 1 failed, 80 skipped` each). The 2 packages after it: `357 tests run: 357
  passed, 0 skipped`, 36.5 s; `inbox-cli` 246 cases, `inbox-xtask` 111, of them 31 of the
  plugin. No case of the plugin or of the printer ran over 0.5 s.
- The bound in bytes, summed from the script's own lines for a mark of 16 digits, a count of
  10 digits and the status 143: begin line 836, 20 head lines 6,020, size line 260, end line
  28, note line 192: 7,336 bytes. Rows of at most 6,000 bytes handed on whole: 7,056.
- No adversary pass ran on round 3.
- The first gate run on round 3 ended at `cargo fmt --check` (the `mod` line of the printer's
  adversary case stood out of order in `crates/inbox-cli/tests/main.rs`); the line was moved.
- Not run: the cases under dash (not on this host; the forge runs them under it); the plugin
  on a real Claude Code session; whether Claude Code counts its cap of hook output in bytes,
  characters or UTF-16 units.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T06:06:08Z to 2026-11-23T06:10:06Z, load average 106 to 115
  on 20 CPUs: exit 0, `859 tests run: 859 passed, 5 skipped`, the test step 63 s of
  its bound of 120 s.
