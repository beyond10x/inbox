# Design: the wake

How a post reaches an idle session. The model names a wake adapter per inbox (`Inbox.adapter`,
`docs/design/inbox.md`) and the state `Delivered`: "An adapter handed it to a live wake surface"
(`ess/domains/messaging.yaml`). The shared part below is the client command every adapter runs,
`inbox wake` (`crates/inbox-cli/src/wake.rs`); each adapter has a section of its own after it.
The service adds one query parameter of its wait for it, `undelivered=1` (`docs/design/service.md`,
"The wait").

## `inbox wake`

`inbox wake` holds the `Pending` view of the caller's own inbox open until it has a message not
yet delivered that is not due (a row in `Sent` whose `expires` the service's clock has not
reached), then prints those rows, marks them delivered and exits 0. A harness that resumes a
session when a background task exits resumes it on a post, and not again on a message it was
handed before.

It reads `Pending` in rounds: a read without a wait (`wait=0`), then a read the service holds
open for at most 60 s (`wait=60`, cut to the service's `--max-wait`). Both ask with
`undelivered=1`, so the service answers only the caller's rows in `Sent` that are not due at its
clock, and only such a row ends the held read: a row in `Delivered` or past its `expires` does
not (the service has the clock, so the service decides). The client keeps a filter on `Sent` of
its own, against a service that does not know the parameter and answers its whole view.

After every answer the reconnect of every client command runs (`commands::reconnect`,
`docs/design/service.md`, "The epoch"), with its rules unchanged: at a new start it opens the
caller's inbox again with the adapter of its last `inbox open` on this host and posts the kept
envelopes again; its line goes to standard error as it runs, and with `--json` the document of
the last reconnect that ran is the member `reconnected` of the wake's document.

A read with `undelivered=1` is never held for a caller whose inbox is not open at the service:
the service answers it at once with `409 {"refused":"inbox not open"}` (also when a close of the
inbox ends a held read), and the answer carries the epoch of its start. On that answer the wake
runs the reconnect, and:

- when it opened the inbox again (a restart: this host recorded an `inbox open` of it at another
  start), the wake reads again at once;
- when this host's record of an `inbox open` is of the start that answered (an `inbox open` of
  the owner after the read began, or an answer that names no epoch), there is nothing to open
  again: the wake reads again at once;
- when the open of the reconnect gets no answer, or an answer the wake tries again (`408`, a
  status from `500` on without a refusal), the wake takes the pause of a failure (below) and
  reads again: the next "not open" runs the reconnect again;
- otherwise the wake ends with exit 1 and one line that names `inbox open` (with `--json` the
  document `{"error":"inbox not open","detail":…}`), in 3 cases:
  - this host has no record of an `inbox open` of it (never opened here, or `inbox close` took
    the record): `inbox wake: <address> is not open at the service, and this host has no record
    of an `inbox open` of it (never opened here, or closed by `inbox close`): run `inbox open`
    first`;
  - the service refused the open: `inbox wake: <address> is not open at the service, and its
    open again with the adapter of this host's `inbox open` was refused: <name> (<status>): run
    `inbox open` to see why`;
  - "not open" again right after an open that succeeded (the reconnect's, or the record of the
    start that answered), also when that answer names yet another start: `inbox wake: <address>
    is still not open at the service right after an open of it succeeded: run `inbox open` to
    open it again`. So a service that is never open after an open does not make the wake open
    and read forever; 2 restarts in a row end the wake the same way.

  So `inbox close` ends the wake of that inbox, and the wake never opens an inbox its owner
  closed. An `inbox close` followed by an `inbox open` while the wake holds its read races with
  the wake: when the open wrote its record before the wake's reconnect looks for it, the wake
  reads again and keeps waiting; when it had not, the wake ends with exit 1.

Another failure of the reconnect's open ends the wake as the same failure of a read does (a
certificate that is not the pinned one, an answer that is neither a success nor a refusal: 3; a
local file: 4).

So a wake that meets a restart opens the inbox again before it waits on the new start, whichever
of its 2 reads the new start gets first, and a post to it is not refused `UnknownInbox` for longer
than that. The read without a wait is kept: it names the epoch of the start before the wake waits
there, and it costs one request per round.

A held read answered without a row starts the next round at once when it held for 1 s or more
(the wait ended at its bound). When it came back sooner (a service with `--max-wait 0`, which
stays allowed, or one that does not know `undelivered=1` and answers its rows in `Delivered` at
once), the wake pauses first as after a failure, 100 ms doubled after each such answer in a row
up to 30 s (`wake::pause`), so it does not spin; a held read that held resets it.

The wake exits on the first answer with a row. It prints what `inbox pending` prints of that
answer: the rows in text, each control character escaped (`src/output.rs`), or with `--json` the
one document of the answer. A row in `Delivered` is neither printed nor marked; `inbox pending`
lists it.

## The delivery

The order is fixed: the rows are written to standard output and flushed, then `MarkDelivered`
is sent for each printed row, one after another, then the wake exits. A mark is sent only after
the write and the flush of the rows succeeded (decision A of the story's first open item). What
holds is that much: a reader that exits without reading what the pipe already took is not seen,
and its rows are marked.

- Only rows in `Sent` that are not due are printed and marked. The model's `deliver` moves a
  message from `Sent` only, and a message in `Delivered` was handed to a surface before:
  printing it again would wake the session again for it.
- `already-delivered` is no failure: a wake on another host may mark a row between the read
  and the mark. A message is delivered once.
- A mark that fails (a refusal, such as `Expired` for a row that turned due between the answer
  and its mark; or a transport failure) loses none of the output and is named in one line on
  standard error; the other rows are still marked. The wake then exits with the status of the
  first mark that failed: 1 for a refusal, 3 for a transport failure. Why not 0: the output is
  complete either way, so nothing is lost, but the service may still hold that row; the status
  lets an adapter tell a clean delivery from one that was not, and a harness that resumes on any
  exit resumes the session the same.
- A standard output that cannot be written ends the wake without a mark, exit 4, one line on
  standard error: the harness is gone (the pipe has no reader), or descriptor 1 is open only for
  reading (`inbox wake 1<file`). The rows are written through a file made from a duplicate of
  descriptor 1 (`output::reader`), not through the standard library's standard output, which
  counts a write the kernel refuses with `EBADF` as written; the bytes are the same. The rows
  stay in `Sent` for the next wake.
- A standard output that no one reads, `/dev/null` or closed when the wake started (`inbox wake
  >&-`: the standard library's start of the program puts `/dev/null` in place of a closed
  descriptor 1, where a write succeeds and reaches no one), or a pipe or a FIFO whose reading
  end is closed already, ends the wake with exit 4 and one line before anything else: it takes
  no lock and sends no request, so it neither blocks a working wake of the inbox (exit 5) nor
  holds a read for rows no one would get. The same check (`output::reaches_a_reader`) runs
  again, once, after the rows of a round are written and before their first mark: rows written
  into a pipe whose reader closed before that check are not marked (exit 4, the line below).

### A reader that is gone

A wake whose standard output is a pipe or a FIFO watches its reader while it waits, with or
without `--follow` (story:a-wake-whose-reader-is-gone-marks-nothing). A thread of its own
(`inbox-reader`) looks every 500 ms (`WATCH_PERIOD` of `src/wake.rs`) whether the other end
still has a reader. When the reading end is closed, the wake ends with exit 4 and one line,
`inbox wake: standard output: it is a pipe or a FIFO whose reader is gone: no message is marked
delivered` (with `--json` also the document of that failure, to a pipe no one reads): it marks
nothing, it does not need a post to arrive first, and the end of the process frees its lock, so
the next wake of the inbox takes it. A post that arrives after it stays in `Sent`, and the next
wake returns on it. On an idle host the wake notices within 500 ms and the time the kernel takes
to run the thread; on a loaded host it may take longer, and no case holds a time.

- The watch does not cut a round: from the write of a round's rows to its last mark it waits,
  and it ends the process only between rounds. A wake that ended in another way first (its marks
  sent without `--follow`, a failure) is not ended by it a second time, so it writes one line at
  most. The check after the write (above) runs once per round: a reader that closes during the
  write makes the write fail, and one that closes before the check is found gone (exit 4, no
  mark either way); a reader that closes after the check, with the rows still unread in the
  pipe, gets every mark of that round.
- How it asks. `poll(2)` would tell it (an error or a hang-up for a write end without a reader),
  but needs either unsafe code, which the workspace forbids (`unsafe_code = "forbid"`), or a
  crate `Cargo.lock` does not hold (rustix, nix, mio). The watch asks the kernel through the
  standard library instead, without writing a byte: it copies from an empty pipe of its own that
  has no writer into a duplicate of descriptor 1 (`std::io::copy`, `output::reader_gone`). On
  Linux the standard library makes that copy with `splice(2)`, which answers `EPIPE` when the
  pipe has no reader and otherwise finds its source empty and without a writer, and moves
  nothing. Where the copy is no `splice` (another kernel), it reads the empty source and answers
  0: the reader is never seen gone, and only the write of the rows tells (exit 4, no mark). The
  cases of `crates/inbox-cli/tests/reader_gone.rs` fail when the check no longer sees a reader
  go.
- Only a pipe or a FIFO is watched (`S_IFIFO`, `output::pipe`). On a terminal, a regular file,
  `/dev/null` or a socket no thread runs and the wake is as before: a regular file has no reader
  to lose, a terminal that is closed sends SIGHUP, and a socket that is closed makes the write
  of the rows fail.
- What stays open: a reader that took the rows from the pipe and died before it showed them, and
  a reader that closed after the check of its round with the rows unread. The write succeeded and
  the check found the reader, so the rows are marked, and no later wake returns on them; `inbox
  pending` lists them. In the `claude_code` hook a post that arrives between the end of the
  harness and the next look of the hook's watch (up to 2 s and one `ps`, longer on a loaded
  host) is marked and handed to no one (`claude_code` below), and so are the rows of a round the
  wake had written and checked when the hook's write of them to its file failed. The reader
  acknowledges nothing: a
  wake that waited for a word of its reader before each mark would close these windows, and
  would change what every adapter does.

A message stays in `Pending` until its owner reads it (`inbox read`, which moves it to `Read`),
answers or refuses it. A wake started again while a delivered message is unread does not return
on it: it waits for the next message in `Sent`, and prints that one alone. An adapter that starts
`inbox wake` again after each turn of a session is therefore woken once per message; a message
the session did not read stays in `Pending` and `inbox pending` lists it.

## One wake per inbox on a host

Before its first request the wake takes an exclusive `flock` of a file beside the outbox file,
`<outbox>.wake.<16 hex digits>`: the digits are the 64-bit FNV-1a hash of the address of the inbox
the wake acts for, so every address has a file of its own and a name of fixed length, and two
principals that share one outbox file do not block each other. That address is the caller's own
(the text of its principal), or, with `--for <address>`, the address of `--for`
(story:the-chat-bridge-runs-with-its-own-key-under-a-delegation): a wake or a bridge of an agent
under a delegation takes the lock of the person's inbox, so it and a wake or a bridge of the
person's own key with the same outbox file exclude each other, and a wake of the agent's own inbox
is not blocked by it. The kernel releases the lock when the process ends in
any way, a kill included, and a file left behind holds no lock and blocks nothing. The directory
is made as the outbox lock makes it (mode 0700); the file is created with mode 0600, opened
without blocking (a FIFO in its place refuses at once) and without following a symbolic link in
its place (`O_NOFOLLOW`: a link, dangling or not, is refused and nothing is created through it),
and must be a regular file: exit 4 otherwise.

A second `inbox wake` of the same inbox with the same outbox file on the same host finds the lock
held, sends nothing and exits 5 with one line on standard error:
`inbox wake: another wake of <address> runs on this host` (with `--json` also the document
`{"error":"another wake","detail":…}`). So no adapter writes a guard of its own. Two wakes of
one principal on two hosts, or with two outbox files, stay possible: the service holds each wait
on a connection of its own, and a message they both read is marked once and answered
`already-delivered` to the other. Two addresses whose hashes are equal would share a lock; with
64 bits that is left as it is.

## Failures and tries

| What the wake meets | What it does |
|---|---|
| No answer: the service cannot be reached, or the connection ends before the answer is whole (its head; a body that is not JSON short of the bytes its `Content-Length` names; a chunked body that is not JSON, which the client does not read) | tries again |
| `408`, or any status from `500` on, that is no refusal of the model (the service busy or stopping, a proxy on loopback that answers `502`) | tries again |
| `409 {"refused":"inbox not open"}` | runs the reconnect; reads again when it opened the inbox or has nothing to open, pauses and reads again when its open got no answer, exits 1 otherwise (see above) |
| A TLS certificate that is not the pinned one, or a peer that speaks no TLS (`https://`) | exits 3 at once: a wake never keeps trying against such a service |
| Any other answer that is neither a success nor a refusal: a `3xx`, a `4xx` without `refused`, a whole answer whose body is not JSON (it fills its `Content-Length`, or has neither a `Content-Length` nor a chunked `Transfer-Encoding` and ends with its connection, as a `404` page of another server may), a `2xx` whose body is not the view (no `rows`) | exits 3 with one line |
| Wrong usage (an `INBOX_URL` outside loopback, a pin beside `http://`, …) | exits 2 before the lock and before any request |
| Any other refusal of a read (`401`, `400`, a declared error) | exits 1 |
| A standard output no one reads (`/dev/null`, closed, a pipe or a FIFO without a reader) | exits 4 first of all: no lock, no request |
| A local file it cannot use (the lock file, the key file), a standard output it cannot write, or a pipe or a FIFO whose reader is gone while it waits | exits 4, no mark |

The pause before a try again starts at 100 ms and doubles after each failure in a row, up to
30 s (`wake::pause`); an answer resets it. One line on standard error names the first failure of
a run of them. The wake keeps trying until it is stopped (decision A of the story's fourth open
item): it does not end after a bound for its adapter to start it again.

SIGINT and SIGTERM end the wake at once by their default action, also when it was started with
them ignored (a thread takes either and ends the process as the default action does); nothing is
written to standard output, and the kernel releases the lock. An ignored SIGINT inherited from
the parent is overridden on purpose, against the convention that a background job keeps it: a
wake adapter starts the wake from a script, and a wake that cannot be interrupted is the worse
failure.

## The 60 s

A post ends a held wait without a timer: the service wakes a held wait whenever a command is
answered on another connection (`docs/design/service.md`, "The wait"), and the wake prints at
once. After the end of a wait at its bound the wake asks again at once; after a restart the
first read of the new start, whichever of the 2 it is, is answered "not open" at once and the
reconnect opens the inbox before the wake waits; while the service is down it tries again within
30 s of the service being back. No case waits for `max_wait` and none holds a time: the case of
the story starts the service with `--max-wait 1` for the end of a wait and the new start with a
`--max-wait` longer than the case, so only the post can end its wait
(`crates/inbox-cli/tests/wake.rs`).

## What it leaves out

- No adapter but `claude_code`, `terminal` and `slack` (`inbox bridge`, `docs/design/bridge.md`),
  which are built: each other adapter is a story of its own and a section below (`metaharness`).
- No wake for one message id or one kind; every row of `Pending` in `Sent` wakes it.
- `inbox wait` stays as it is: one held read, its answer printed, nothing marked.

## claude_code

A Claude Code plugin in this repository, `adapters/claude-code/`, that the operator installs
(story:the-claude-code-adapter-wakes-an-idle-session). It holds 3 files:

- `.claude-plugin/plugin.json`: the manifest, the plugin's name `inbox`;
- `hooks/hooks.json`: one `Stop` hook, a command hook with `"asyncRewake": true`,
  `"timeout": 604800` and the command `${CLAUDE_PLUGIN_ROOT}/scripts/wake.sh` in exec form
  (`"args": []`);
- `scripts/wake.sh`: the hook's script, POSIX shell (dash runs it), which runs the `inbox`
  binary found on `PATH` and no other program but `cat`, `cut`, `grep`, `mkdir`, `mkfifo`, `od`,
  `ps`, `rm`, `sed`, `sleep`, `tr` and `wc`.

The loop: a turn of the session ends, Claude Code starts the hook in the background, the script
runs `inbox wake` until it exits, and when the wake printed rows the script writes them to
standard error inside a frame (below) and exits 2. Claude Code then wakes the session, also an
idle one, and shows it that text. The turn that follows ends, and its hook starts the next wake.
A turn that ends while a wake runs starts a second one, which exits 5 by the lock above; its
script exits 0 and prints nothing, so one wake runs and the plugin holds no guard of its own.

The script reads nothing of the hook's input on standard input. It starts `inbox wake` with the
environment the session gave it (`INBOX_URL`, `INBOX_SECRET_KEY_FILE`, `INBOX_OUTBOX`,
`INBOX_TLS_PIN`), with no argument but `wake`, with standard input `/dev/null` and with its
standard error thrown away (a line of it may name the URL); the script prints no URL, key file
or pin. Before it starts the wake it checks 2 things and, when either fails, exits 0 at once and
prints nothing:

- the session has an inbox: `INBOX_URL` and `INBOX_SECRET_KEY_FILE` are set and not empty;
- the inbox is this adapter's: the record of `inbox open` beside the outbox file
  (`<outbox>.opened`, the outbox file found as `inbox` finds it: `INBOX_OUTBOX`, else
  `$XDG_STATE_HOME/inbox/outbox.jsonl`, else `$HOME/.local/state/inbox/outbox.jsonl`) holds a
  line with `"adapter":"claude_code"`. The script does not tell which inbox the line names: only
  the secret key file names the caller, no client command prints the caller's own address
  without a request to the service, and the script reads no file that holds a key. So an outbox
  file shared by a principal opened with `claude_code` and one opened with another adapter makes
  the sessions of both the adapter's. An address that holds the text `claude_code` does not
  match: `inbox` writes a `"` inside a value as `\"`.

Then it makes the mark of the run and a directory, and when either fails it exits 1 with one line
`inbox plugin: …` and starts no wake:

- the mark: 16 lower-case hexadecimal digits, the first 8 bytes `od` reads of `/dev/urandom`. It
  is held in the shell alone and written only to the script's standard error, never to a file;
- the directory of the rows: `$TMPDIR/inbox-claude-code.<process id>` (`/tmp` without `TMPDIR`),
  made with `mkdir -m 700` under `umask 077`; when that name exists already (a directory, a file
  or a symbolic link: another user's, or one a killed hook left), the next one,
  `….<process id>.1` up to `.99`, and never a name the script did not make itself. In it the
  script makes the FIFO `rows` with `mkfifo -m 600` and the file `text`, into which it writes one
  byte and empties it again, as an empty file is made also on a full disk or past a quota (no
  FIFO, or a file that takes no byte: exit 1, one line, no wake). The script removes the
  directory when it exits, unless it is killed.

The wake writes its rows into that FIFO, and the script's own shell reads them, line by line with
its `read`, as the wake writes them; no other process holds the reading end (descriptor 4 of the
shell, closed for every child). The shell appends each line to the file `text` through a
descriptor it opens once (6), whole lines and a last line without a line end as it came, and the
frame below is made from that file: the time grows with the size of the rows, not with its
square. Each write to the file is checked: when one fails, the shell reads no more, closes its
end of the FIFO at once (the wake, its reader gone, marks no later round), ends the wake, hands
on no frame and exits 1 with one line that names the file. `read` takes
one byte from a pipe per system call; under bash on a host with a load of about 110, 2 MB of
rows took 1.7 s and 8 MB 6.4 s. The shell opens the FIFO for reading and writing first
(descriptor 5, which waits for no other end), then for reading (descriptor 4, at once, as a
writer exists), and starts the wake with the FIFO as its standard output and descriptors 4 and 5
closed; the wake holds descriptor 5 as a writer until it has opened its own end, and the shell
closes descriptor 5 once the wake started. So the shell reads to the end of the wake's output and
no further, and a wake that dies before it opened the FIFO ends the read too. Then it waits for
the wake's exit status. A hook ended in any way, SIGKILL included, closes the reading end with
its process: its wake has no reader, and ends by the rule of "A reader that is gone" above
(exit 4 within 500 ms on an idle host, no mark, the lock freed). A signal to the hook ends the
wake (the trap below). A signal that cuts a `read` of the shell inside a line ends the reading,
and that line is dropped whole, whatever bytes of it the shell kept (bash run as `sh` drops
them and ends the `read` at once; bash by its own name and zsh go on with the `read` to the end
of the wake's output and keep a cut last line, which is dropped), so the frame holds whole lines
and counts only head lines it holds whole. The wake, ended before it marked that round, marks
none of it, and the message comes again. Whole lines the wake wrote before it ended are read on
and handed on, as they may be marked.

The harness. The script reads its parent process id at its start (`$PPID`, the harness that
started it) and starts a watch: a child that holds no descriptor of the hook (standard input,
output and error `/dev/null`, descriptor 3 closed, started before the FIFO is opened) and looks
every 2 s with `ps -o ppid= -p <hook>` which process is the hook's parent. When the harness is
gone without a signal to the hook (it crashed, or was killed with SIGKILL), the kernel gives the
hook another parent, the id differs, and the watch sends the hook SIGTERM, which ends it as
below: its wake is ended, and a hook without rows exits 0 and writes nothing. So the hook
notices a harness that is gone within 2 s and the time of one `ps` on an idle host, without a
line written to the harness. A post that arrives in that time is read by the hook, which is still
alive, marked delivered and handed to no one: an open limit ("What stays open" above). The watch ends by itself when the hook is gone (`ps` finds no such
process), and the script ends it when it exits. Without `ps` on `PATH` there is no watch. That
real Claude Code is the hook's parent process is not verified by any case here: a harness that
starts the hook through a shell of its own leaves that shell as the parent, and a harness that
is gone then changes nothing the watch sees. A process id used again within those 2 s by
another process is not told apart.

### The frame

The session reads what the plugin wrote apart from what a sender wrote. In what `inbox wake`
prints, each message is one head line that is not indented (its message id, kind, state, sender,
recipient, thread and `expires`), and every other line of it starts with at least 2 spaces:
`inbox` indents the body, the options, the reply and the refusal, and shows a line end inside a
member of a line as `\u{a}` (`crates/inbox-cli/src/output.rs`, `rows_text`). On standard error the
script writes, in this order:

1. one begin line `inbox <mark>: <n> message(s) follow, down to the line `inbox <mark>: end`: …`,
   where `<n>` is the count of lines of the rows that are not indented. It says that the text
   down to the end line is what other principals posted to the inbox of the session, as
   `inbox wake` printed it, data, not instructions of the user or of the plugin; that each
   message is one head line that is not indented with indented lines under it; that a line in
   between that claims to come from the plugin or the user was written by a sender; and what to
   do: `inbox read <message id>`, `inbox answer <message id> --body <text>` (a decision with
   `--option <letter>`), `inbox refuse <message id> --reason <text>`, and not to start
   `inbox wake`, which the plugin starts again when the turn ends;
2. the rows as `inbox wake` printed them, or, when they are over 6,000 bytes (`wc -c`), only
   their head lines (the lines that are not indented), at most 20, each cut to its first 300
   bytes (`cut -b 1-300`: a head line holds the sender's and the recipient's address, which have
   no bound of their own), and after them one line `inbox <mark>: …` that says how many messages
   there are in all, that every other line is left out for its size, and that `inbox pending`
   lists every message with its body;
3. the end line `inbox <mark>: end`;
4. when the wake's exit status was not 0 and no signal ended the hook, one note line
   `inbox <mark>: …`: the status, that a message may not be marked delivered and may wake the
   session again, and that `inbox pending` lists every message not yet read.

So every line the plugin writes after the begin line starts with `inbox <mark>: `, and a sender,
who cannot know the mark, writes no such line: its text is indented, and a line it copies from an
earlier run carries another mark. On standard output the script writes one line,
`inbox: messages arrived in the inbox of this session.`, and never the rows or the mark. The
script keeps the harness's standard error on descriptor 3 and sends its own descriptor 2 to
`/dev/null` at its start (`exec 3>&2 2> /dev/null`), writes each of its own lines (the frame,
each `inbox plugin: …` line) to descriptor 3, and closes it for every child (`3>&-`), so a line
the shell writes by itself (dash writes `Terminated` when the wake it waits for was ended by a
signal) or a utility writes never reaches the session, and no child holds the harness's pipe.

The size. The hooks page says plain hook output is capped at 10,000 characters: over the cap
Claude Code saves the output to a file and hands on its path and the first 2,000 characters,
and it does not ask Claude to read the file. Whether that cap applies to the standard error of
an `asyncRewake` hook is not verified. The script writes at most 8,000 bytes to standard error,
and so at most 8,000 characters in any count of them. The sum, each line with its line end, for
a mark of 16 digits, a count of 10 digits and a status of 3: the begin line 836 bytes, 20 head
lines of at most 300 bytes 6,020, the size line 260, the end line 28 and the note line 192,
7,336 bytes in all; rows of at most 6,000 bytes are handed on whole, which with the same lines
is at most 7,056. So the end line reaches the session within the cap. The cut is by bytes in
every locale (`cut -b`; GNU `cut -c` counts characters in a UTF-8 locale, and 20 head lines of
300 characters of 2 bytes each made the frame 12,513 bytes), so a head line cut inside a
character of several bytes ends in an incomplete character. Cases hold the 8,000 bytes for 25
rows of over 1,000 ASCII bytes each and for 25 rows of 400 characters of 2 bytes each in a UTF-8
locale. A body may hold up to 65,536 bytes (`docs/design/inbox.md`); the session reads it with
`inbox pending`.

### Exit statuses

What the script does after the wake, by the wake's exit status (rows: the wake wrote to its
standard output):

| `inbox wake` | The script |
|---|---|
| 0 with rows | exits 2: the frame on standard error, its one line on standard output |
| 1 or 3 with rows (a mark failed) | exits 2: the frame and the note line |
| 4 with rows (the reader of its output was gone after it wrote rows: does not happen while the hook reads it) | exits 2: the frame with what was read, and the note line |
| 128 + n with rows (the wake ended by a signal that was not the hook's) | exits 2: the frame and the note line |
| 2 or 5 with rows | does not happen (both exit before a request); it would be the frame and the note line |
| 0 without rows | does not happen (the wake exits 0 only after it printed rows); exits 0 and prints nothing |
| 5 without rows (another wake of the inbox runs) | exits 0 and prints nothing |
| 1, 2, 3 or 4 without rows | exits 1 with one line on standard error: the status, what it means, and to run `inbox wake` in a terminal with the same variables. The session is not woken. For 2 the line names wrong usage: this `inbox` may have no `wake` command (it is older than the plugin, as a release before `wake` was added; clap refuses the command with 2), or `INBOX_URL` is not a loopback `http://` or `https://` URL, or the pin does not go with it. The script does not ask `inbox` whether it has `wake`: that is one more process each turn for the text of a line |
| 128 + n without rows | exits 1 with one line: the wake ended by a signal or by a failure of its own. The session is not woken |
| `inbox` not on `PATH`, no mark, no directory, no FIFO, or a file of the rows that takes no byte | exits 1 with one line that says so; no wake is started |
| a write of a row to the file `text` failed (a full disk, a quota) | reads no more, closes its end of the FIFO, sends SIGTERM to the wake and waits for its end; no frame; exits 1 with one line that names the file. The wake marks no later round; the rows of a round it had written and checked are marked and not handed on (open) |
| the hook ended by SIGTERM, SIGINT or SIGHUP, or its harness gone (its watch sends it SIGTERM) | sends SIGTERM to the wake, reads on the whole lines the wake wrote before it ended and waits for its end; they are handed on in the frame without the note line (exit 2), a line the signal cut inside a `read` ends the reading and is dropped whole, and so is a last line without a line end; else it exits 0 with nothing printed |
| the hook ended by SIGKILL | nothing: the wake finds the FIFO without a reader and ends by itself with 4, no mark |

Rows the wake printed are handed on whatever its exit status, because it may have marked them
delivered, and a wake does not return on a message it delivered before. A failure without rows
does not wake the session, because it would repeat: every turn that ends starts the wake again,
and a session woken for a failure would end its turn, start the wake, fail again and be woken
again, a turn each time, with no message to handle. The line of exit 1 is for the operator;
whether Claude Code shows it, and where, is not verified (its hooks page says the completion of
an `async` hook is shown in verbose mode). A post wakes the session again once a turn of it ends
after the cause is gone. The line on standard output of exit 2 is there because the hooks page
says that a `Stop` hook that exits 2 with nothing on standard output and `No such file or
directory` on standard error counts as a failure that blocks nothing, and the body of a message
may hold those words; that this applies to an `asyncRewake` hook is not verified.

### The timeout

The hooks page says Claude Code enforces `timeout` on an `asyncRewake` hook, that the default of
a command hook is 600 s, and that a hook canceled at its timeout has its output discarded; it
names no largest value. 600 s would end the wait of an idle session after ten minutes, so the
hook sets 604800 s (7 days). When the timeout ends a wake, the operator sees nothing in the
session: the hook is canceled, a post that arrives after it does not wake the session, and the
session is woken again only after its next turn ends (a turn the operator starts) and the hook
starts a new wake; `inbox pending` lists the posts in between, and the next wake prints those it
has not delivered. That Claude Code takes the value 604800 as it is, and how it ends the hook at
its timeout or at the end of the session (which signal, to the script alone or to its process
group), is not verified. When a headless session ended because its standard input was closed,
no `inbox wake` was left (seen once, below). SIGTERM, SIGINT or SIGHUP to the script ends the
wake; SIGKILL to the script leaves a wake that ends by itself without a mark; a harness that is
gone ends the hook through its watch. The cases hold all 3:
`a_hook_killed_with_sigkill_leaves_no_wake_that_marks_a_post` and
`a_harness_that_is_gone_gets_no_post_marked_delivered`
(`crates/inbox-xtask/tests/tools/adversary_claude_code_adapter.rs`) against a stand-in `inbox`,
and `reader_gone::a_hook_ended_by_sigkill_leaves_a_wake_that_ends_without_marking` of
`inbox-cli` against the built one.

### Seen on a real session, not verified, and the operator's check

Seen once, on 2026-11-23, on one real headless session of Claude Code 2.1.330 (`claude -p` with
`--input-format stream-json` and `--output-format stream-json`, its standard input held open,
the model `claude-haiku-5-5`, the plugin loaded with `--plugin-dir`, no user settings, the
session allowed `inbox read` and no other command; `inbox serve` over plain HTTP on a loopback
address with keys made for the run):

- Claude Code ran the `Stop` hook of the plugin's `hooks/hooks.json` with `asyncRewake` when the
  first turn ended (its event `hook_started`), and the turn's result came without waiting for
  the hook;
- the session then stood idle for 59 s with no event;
- a note posted by a second principal ended the hook with exit status 2. Its event
  `hook_response` held the one line of standard output and, as standard error, the frame: the
  begin line, the head line of the note, its body indented, and the end line with the same mark;
- the session took a turn with nobody typing: the first event of the model came 2.0 s after the
  post. It ran `inbox read <message id>`, named the id, the kind and the sender of the note, and
  its turn ended 4.9 s after the post. `inbox pending` of the session's principal was empty
  after it;
- the `Stop` hook started again when that turn ended;
- when the session's standard input was closed the session ended, and no `inbox wake` process
  was left on the host.

Not seen in that run, and still not verified: an interactive session (the terminal interface
of Claude Code); a wait longer than 59 s; a request answered with `inbox answer`; more than one
message, and messages over 6,000 bytes; TLS; where the line of an exit 1 is shown; the cap of
hook output; how Claude Code ends the hook (which signal, and to which processes). The
operator's check below covers the first 3.

What the hooks page says and the run did not contradict: it lists `asyncRewake` among the
fields of a command hook and names no event that refuses it, and it describes `Stop` as "Runs
when the main Claude Code agent has finished responding", and not after an interrupt by the
user. `claude plugin validate adapters/claude-code` (Claude Code 2.1.330) reports
`Validation passed with warnings`: no version, no author.

The gate holds the plugin against a stand-in harness and a stand-in `inbox`
(`crates/inbox-xtask/tests/tools/claude_code_adapter.rs`,
`crates/inbox-xtask/tests/tools/adversary_claude_code_adapter.rs`); it starts no Claude Code and
makes no model call. The operator checks the acceptance on one real session once per release and
records it as evidence of the story:

1. Start `inbox serve` and make the keys of 2 principals as in `website/docs/first-use.md`: the
   session's key `bob.key` (`agent:demo/bob`) and a second key `alice.key`.
2. In the terminal of the session, export `INBOX_URL`, `INBOX_SECRET_KEY_FILE` (the absolute path
   of `bob.key`) and, over TLS, `INBOX_TLS_PIN`, then run `inbox open --adapter claude_code`.
3. Start Claude Code there with the plugin:
   `claude --plugin-dir <repository>/adapters/claude-code`. Give it one prompt, let the turn end,
   and leave the session idle.
4. In a second terminal with the same `INBOX_URL`: `inbox post --secret-key-file alice.key --to
   agent:demo/bob --kind request --body "Which build is green?"`; it prints the message id.
5. Without typing in the session, see it take a turn, run `inbox read <message id>` and answer
   with `inbox answer`.
6. `inbox pending --secret-key-file bob.key` no longer lists the message, and
   `inbox outbox --secret-key-file alice.key` shows it `Answered`. A note posted the same way is
   in neither once the session has read it.

## metaharness

Not built yet: story:the-metaharness-adapter-wakes-an-idle-session.

## terminal

`inbox wake --follow` is the adapter `terminal` (story:the-terminal-adapter-shows-a-post,
decisions B and A of its 2 open items): a person or an agent at a shell runs it in a terminal of
its own, and it shows each message posted to the inbox there, with no typed `inbox pending`, and
keeps running. The code is `crates/inbox-cli/src/wake.rs`; without the flag `inbox wake` is as
above, byte for byte.

- Each round is the wake's round: the reads with `undelivered=1`, the reconnect after every
  answer, the tries and the pauses, all as above. When an answer has rows in `Sent` that are not
  due, it writes them and flushes them to standard output through the printer of `inbox wake`
  (`rows_text`, one head line that is not indented per message whatever a sender wrote, each
  control character of a sender escaped), then sends `MarkDelivered` for each, one after
  another, then starts the next round with a read without a wait. A message is marked delivered
  only after its rows were written, as above: the terminal is the live wake surface it was
  handed to. A message in `Delivered` when it starts (a wake before it printed it) is not shown;
  `inbox pending` lists it.
- With `--json` each round writes the one document of its answer on a line of its own (with the
  member `reconnected` when a reconnect ran in that round): a stream of JSON lines, and no bell.
- The bell. After the text of a round, in the same write, one BEL (0x07) when standard output
  is a terminal (`isatty` of the duplicate of descriptor 1 the rows are written through,
  `output::try_print_round`), and none when it is not: a pipe or a file gets the rows only. The
  BEL is the client's own byte, added after the text went through `visible`, so a round rings
  once however many messages it holds, and a BEL in a sender's text is shown as `\u{7}` and
  rings nothing. `visible` also shows each bidirectional control of a sender as its escape
  (U+061C, U+200E, U+200F, U+202A to U+202E, U+2066 to U+2069, such as `\u{202e}`), in every
  command that prints text, so a sender's text cannot reorder what the terminal shows; `--json`
  keeps them as values. A backslash is printed as it was sent, so a sender who writes the 8
  characters `\u{202e}`, or `\u{7}`, gets the line of a sender whose text holds that character:
  the text output does not tell the 2 apart, and `--json` does. Escaping the backslash would
  change text that holds none of these characters, and is not done.
- It never exits 0 by itself. SIGINT and SIGTERM end it at once by their default action, as they
  end `inbox wake`: its status is that of the signal (128 + n in a shell), and the kernel
  releases the lock. A failure that ends `inbox wake` ends it with the same status (1, 3, 4) in
  any round. A terminal that is closed sends SIGHUP, which ends it by its default action unless
  it was started with SIGHUP ignored (`nohup`); then a write to the closed terminal fails and
  ends it with 4 and no mark. No case holds the SIGHUP.
- A mark that fails is named in one line on standard error, as `inbox wake` names it, and the
  mode goes on: the failure does not change how it ends later (a signal gives the status of the
  signal). The row may still be in `Sent`, so the next round may get it and show it again; it
  starts after the pause of a failure (`wake::pause`: 100 ms, doubled for each such round in a
  row, up to 30 s), so a service that keeps answering a row it refuses to mark does not make it
  spin. A round that gets a row the round before marked delivered with success counts the same
  (a service that answers the mark and does not keep it): it shows the row again and pauses
  after it. Only the message ids of the round before are kept, so nothing grows over a long
  run. A round whose marks all succeed and that got no such row resets the pause.
- The lock is the wake's lock: one `inbox wake`, with or without the flag, runs per inbox and
  outbox file on a host, and a second exits 5 and sends nothing.
- The adapter of the record. It reads this host's record of an `inbox open` of the caller
  (`<outbox>.opened`, the adapter of the last `inbox open`) twice over: at the start, after the
  check of standard output (exit 4 first of all) and before the lock and any request; and in
  every round, after the answer with rows and before the rows of that round are written. When
  the record names another adapter (`claude_code`, `metaharness`, `slack`) it ends with 1 and
  one line:
  `inbox wake: <address> was opened on this host with the adapter <adapter>, and `inbox wake
  --follow` is the adapter terminal: run `inbox open --adapter terminal` to show its messages in
  this terminal` (with `--json` also `{"error":"another adapter","detail":…}`). In a round it
  writes no row of that round and marks nothing, and the end of the process frees the lock. So a
  terminal does not keep the lock of a Claude Code session's plugin, whose `inbox wake` would
  then exit 5 and never wake the session. What is still open: an `inbox open --adapter
  claude_code` while the mode holds a read does not end that read, so the mode sees the new
  record only when its read is answered with a row. The message that ends the read stays in
  `Sent`, neither shown nor marked, and the next `inbox wake` of the plugin returns on it; a
  plugin wake started between the open and the end of the mode (at the latest when the post
  arrives) still finds the lock held, exits 5, and the session is woken for that message only
  after its next turn ends, while `inbox pending` lists it. With no record of the caller it goes
  on as `inbox wake` does
  without one: an inbox not open at the service ends it with 1 and the line that names
  `inbox open`, one opened on another host is followed. `inbox wake` without the flag does not
  read the adapter.

What it leaves out: whether anyone looks at the terminal (a message shown in a terminal no one
watches is `Delivered` all the same, and `inbox pending` lists it until it is read). The inbox of
a person is served through a chat channel by `inbox bridge` (`docs/design/bridge.md`,
epic:human-inbox), which runs the rounds of this mode with the chat channel as its surface and
takes the same lock: a bridge and this mode of one inbox and outbox file on a host exclude each
other (exit 5), and this mode ends with 1 on an inbox the bridge opened (the adapter `slack`).

The cases are `crates/inbox-cli/tests/terminal_adapter.rs` and
`crates/inbox-cli/tests/adversary_terminal.rs`, against a real `inbox serve` and stand-ins for
the order of the rows and the marks and for a row that stays in `Sent`. The bell on a terminal
runs through a pseudo terminal that `script` of util-linux makes: the crate forbids the unsafe
code that would open one inside the case. Those cases need `script` of util-linux and Linux
`/proc`. That the pause is reset is held by no case: only an upper bound of time would see a
pause that grew, and no case holds one.
