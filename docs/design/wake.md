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
  descriptor 1, where a write succeeds and reaches no one), ends the wake with exit 4 and one
  line before anything else: it takes no lock and sends no request, so it neither blocks a
  working wake of the inbox (exit 5) nor holds a read for rows no one would get. The same check
  (`output::reaches_a_reader`) runs again before the rows are written: the reader may be gone by
  then.

A message stays in `Pending` until its owner reads it (`inbox read`, which moves it to `Read`),
answers or refuses it. A wake started again while a delivered message is unread does not return
on it: it waits for the next message in `Sent`, and prints that one alone. An adapter that starts
`inbox wake` again after each turn of a session is therefore woken once per message; a message
the session did not read stays in `Pending` and `inbox pending` lists it.

## One wake per inbox on a host

Before its first request the wake takes an exclusive `flock` of a file beside the outbox file,
`<outbox>.wake.<16 hex digits>`: the digits are the 64-bit FNV-1a hash of the caller's address,
so every address has a file of its own and a name of fixed length, and two principals that share
one outbox file do not block each other. The kernel releases the lock when the process ends in
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
| A standard output no one reads (`/dev/null`, closed) | exits 4 first of all: no lock, no request |
| A local file it cannot use (the lock file, the key file), or a standard output it cannot write | exits 4 |

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

- No adapter: each is a story of its own and a section below.
- No wake for one message id or one kind; every row of `Pending` in `Sent` wakes it.
- `inbox wait` stays as it is: one held read, its answer printed, nothing marked.

## claude_code

Not built yet: story:the-claude-code-adapter-wakes-an-idle-session.

## metaharness

Not built yet: story:the-metaharness-adapter-wakes-an-idle-session.

## terminal

Not built yet: story:the-terminal-adapter-shows-a-post.
