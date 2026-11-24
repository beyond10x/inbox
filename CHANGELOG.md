# Changelog

What changed in each release of the inbox. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`. How a release is cut:
[`docs/releasing.md`](docs/releasing.md).

## [Unreleased]

### Added

- A delegation now also lets its agent act for the inbox of the principal it names:
  `inbox <command> --for <address>` sends the request header `Inbox-On-Behalf-Of` with that
  address, signed as a sixth member of the request, and while the key file holds a live
  delegation of the pair the service answers the agent as the owner of that inbox: it reads,
  marks, answers and refuses the person's messages (`inbox answer --for` answers on behalf of the
  person; `--authority` names the delegation). Without a live delegation such a request is
  refused as one with a bad signature (`401`). Under the header a reply on behalf of anybody but
  the person is `authority-invalid`, and a `close-inbox` closes only the caller's inbox of that
  request. A request without `--for` is the one of before, byte for byte.
- Every client command takes `--for` now: `inbox open`, `inbox close` and `inbox wake` act for
  the person's inbox, and `inbox bridge --for <address> --authority <delegation id>` runs the
  chat bridge with a key of its own under a delegation of the person, so the bridge no longer
  holds the person's key. What they keep on this host is the person's: the line of the opened
  inbox, the lock of the wake (a second bridge or wake for the person exits 5) and the record of
  the chat posts, which a bridge with the person's key wrote before and a bridge with its own key
  reads on. After a restart of the service the next command under `--for` opens the person's
  inbox again. A bridge whose key entry or delegation the service refuses (`401`) ends with exit
  status 1 and one line that names the key, the delegation and the two `inbox keyfile` commands
  that add them, and sends nothing more. One of `--for` and `--authority` without the other is
  exit status 2. A bridge without either runs as before.
- `inbox bridge`, the chat bridge of a person's inbox: it posts each message of the inbox into
  the chat channel of its binding (`--chat-channel`, `--allowed-chat-user`, `--chat-connector`)
  through `connectors`, one post per message with the body in a block of literal text, records
  the post beside the outbox file, then marks the message delivered; a post that fails is tried
  again after a pause. It opens the inbox with the adapter `slack` and takes the lock of
  `inbox wake`. It passes no approval to `connectors`: it needs `connectors` 0.77.0 or later,
  which refuses its posts (`approval_required`) until it holds the operator's standing grant for
  the channel. No call of `connectors` was run yet, by the tests or by hand.
- `inbox bridge` marks a message read when the allowed chat user reacts to its post: it reads the
  recent posts of the channel through `connectors` every `--chat-poll-interval` seconds (10 by
  default) while one of its posts is open, and sends `MarkRead` at least once per post that user
  reacted to (again at the next read when an answer was lost), so a request or a decision moves to
  `Read` and a note is handled. A reaction of any other chat user does nothing; a reaction on a
  message past its deadline is named in at most one line. A post whose message left `Pending` another way
  (`inbox read`, an answer, its deadline) is no longer open, so the channel is read only back to
  the oldest post still open, that post included. Only the posts of the channel the bridge serves
  are asked about. A read of the channel and a post never run at once, and a read that fails
  (also on a record of the chat posts that cannot be read) marks nothing and is tried again after
  a pause.
- `inbox bridge` posts a message once per message id: a message its sender's outbox delivers
  again after a restart of the service is marked delivered, and read when it was read before,
  without a second post, also by a new bridge and when the post went into another channel. At the
  end of a message nobody answered it writes one reply in the thread of the post, "taken on
  silence" with the option and its text, or "expired, not answered", under the key
  `inbox-<message id>-end`; it learns the end from the `Thread` view and never answers in the
  person's place. An answered or refused message gets no reply. A post leaves the record of the
  chat posts once its message is gone and its deadline plus `--chat-post-retention` seconds
  (86400 by default) has passed (a post an older bridge recorded without a deadline: that long
  after the bridge found its message gone), so the record no longer grows without end. A record
  the bridge cannot read, also one with a line that is not a whole entry, ends it (exit 4) before
  any post, and a `read` lost after a repost is sent again by the next poll. A reply that fails
  is sent again at a later poll under the same key and with the same text, and stops no post and
  no read.

## [0.6.1] - 2026-11-23

### Changed

- A message's text no longer reaches a terminal with characters it does not show. `pending`,
  `wait`, `wake`, `outbox`, `collect`, `thread`, `replies` and the lines on standard error that
  repeat text of the service show the characters that take no width on a terminal (the soft
  hyphen U+00AD, U+180E, the zero width space U+200B, U+2060 to U+2064, U+206A to U+206F,
  U+FEFF, U+FFF9 to U+FFFB and the tag characters U+E0000 to U+E007F) as their escapes, such as
  `\u{200b}`; before, they were printed as sent, so 2 texts that differ looked the same. The
  joiners U+200C and U+200D, U+034F and the variation selectors are printed as sent, and the
  flag of a part of a country is shown with the escapes of its tags. No flag, exit status or
  `--json` output changes.
- The repository builds with ESS 0.98.0 (`ess/ess-inputs.yaml` and `ESS_VERSION` of the check
  workflow); a contributor needs `ess` 0.98.0. The specification validates on it unchanged, and
  0.98.0 writes `generated/` and the conformance suite byte for byte as 0.97.4 did: the same 119
  scenarios and the same 98 refusals of synthesis. Nothing of the service or the client changes.

### Fixed

- A command that has to change the epochs file (`<outbox>.epochs`) or the file of the opened
  inbox (`<outbox>.opened`) and cannot read it (no permission to read, not UTF-8, over its
  bound, no regular file) now fails with exit status 4, names the file and the reason in one
  line on standard error, and leaves the file and the outbox file as they are (but that
  `inbox repost` still drops the envelopes that expired or were refused for good). Before, it
  read the file as empty and replaced it with its own entry, losing every other epoch, mark
  and record. `inbox post` and `inbox close` fail before they send anything; `inbox open` and
  `inbox repost` fail after the service answered (the inbox is open, the envelopes are
  posted), `inbox repost` also when a post is answered `408`, `5xx` or `UnknownInbox`. After
  an `inbox open` that failed so, `inbox close` fails with 4 and sends nothing until the file
  is repaired or removed. A missing file is read as empty, and a command that only reads these
  files is unchanged.
- A change of the outbox file or of `<outbox>.epochs`, and of a key file (`inbox keyfile
  add-key` and `inbox keyfile add-delegation`), no longer follows a symbolic link or opens a
  FIFO left at the name of its staged file (`<file>.new`, for a key file `<file>.<pid>.new`),
  and a change of `<outbox>.opened` no longer opens a FIFO there: the entry is removed and the
  staged file is created anew with its mode (0600 for the outbox files, the mode of the key file
  it replaces). Before, a link there was followed: the file became a link to the link's target,
  whose bytes were overwritten, and a link to the file itself lost every entry of the file; a
  FIFO there held the command.
- `inbox wake` whose standard output is a pipe or a FIFO now ends with exit status 4 and marks
  nothing when its reader is gone: while it waits (within about a second, on Linux), and once
  more after the rows are written and before the first mark. Before, it noticed only at its
  next write, so a wake left behind kept the lock and marked the next message delivered though
  nobody read it. The Claude Code plugin reads the wake through a FIFO in the hook's own shell
  and watches its parent process: a hook killed with SIGKILL, or Claude Code gone without
  ending the hook, no longer leaves a wake that marks the next message; it keeps the rows in a
  file as it reads them, so its time grows with their size, not with its square; when that
  file takes no byte it starts no wait and says so, when a row cannot be kept there it stops
  the wait, says so and shows nothing, and a line a signal cut is left out whole. Still open, on
  the limits page: a reader that took the messages and ended before it showed them, or closed
  the pipe with them unread right after the wake's check, leaves them marked; a post in the up
  to 2 s before the plugin notices that Claude Code is gone is marked and shown to no session;
  the messages of a wait that ended because a row could not be kept are marked and shown to
  no session; and it is not checked that Claude Code is the hook's parent process.

## [0.6.0] - 2026-11-23

### Added

- `inbox wake --follow`, the adapter `terminal`: in a terminal of its own it shows each message
  as it arrives, in the form `inbox pending` prints, marks it delivered and waits for the next
  one, until Ctrl-C or SIGTERM. On a terminal it rings the bell once per arrival; a pipe or a
  file gets the rows only. With `--json` it writes one document per line. A mark that fails is
  named on standard error and the mode goes on. It takes the lock of `inbox wake`, so a second
  `inbox wake` of the inbox exits 5. It is for an inbox opened with `inbox open --adapter
  terminal`: when this host opened the inbox with another adapter it ends with 1 and a line that
  names that command, at its start or with the next arrival, and that message is not shown and
  stays for the next `inbox wake`. `inbox wake` without the flag is unchanged.

### Security

- A message's text can no longer reorder what a terminal shows. `pending`, `wait`, `wake`,
  `outbox`, `collect`, `thread`, `replies` and the lines on standard error that repeat text of
  the service show the characters that change the direction of text (U+061C, U+200E, U+200F,
  U+202A to U+202E and U+2066 to U+2069) as their escapes, such as `\u{202e}`. Before, they were
  printed as sent. `--json` is unchanged.

## [0.5.0] - 2026-11-23

### Added

- `inbox wake` waits until the caller's own inbox has a message not yet delivered and not past its
  `expires`, across the end of a wait and a restart of the service, prints those messages as `inbox
  pending` does (`--json`: one document), then marks each delivered with `MarkDelivered` and exits
  0. It is the command a wake adapter runs. The messages are written
  and flushed before the first mark; a mark that fails (such as `Expired` for a message that turned
  due in between) is named on standard error and sets the exit status (1 or 3). A message already
  delivered and not yet read neither ends the wait nor is printed; `inbox pending` lists it. A wake
  of an inbox that is not open at the service ends with 1 and a line that names `inbox open` when
  this host has no record of an `inbox open` of it, when the service refuses to open it again, or
  when it is still not open right after an open that succeeded; so does an `inbox close` while the
  wake waits, and the wake never opens an inbox its owner closed. When the open after a restart
  gets no answer, or an answer that is tried again, the wake pauses and reads again. No answer, an
  answer cut before its end, `408` and a status from `500` on without a refusal of the service are
  tried again after a pause that grows from 100 ms to 30 s; a redirect, a `4xx` without a refusal,
  an answer that is not the view and a TLS certificate the pin does not name end it with 3 (a whole
  answer fills its `Content-Length`, or has none and is not chunked). A held read answered within 1
  s without a message is followed by the same pause, so a service that ends every wait at once does
  not make it spin. A standard output that is closed or is `/dev/null` ends it with 4 before the
  lock and any request; one that cannot be written (the reader gone, open only for reading) ends it
  with 4 and nothing marked. A second wake of the same inbox with the same outbox file on the same
  host exits with the new exit status 5 and sends nothing (a lock file beside the outbox file,
  opened without following a link). SIGINT and SIGTERM end it at once, also when they were ignored
  at its start (story:inbox-wake-returns-on-a-post).
- The wait on `Pending` (`GET /messaging/views/pending`) takes `undelivered=1`, alone or beside
  `wait`: the answer holds only the caller's rows in `Sent` that are not due at the service's
  clock, and a held wait ends only on such a row; a row in `Delivered` does not end it. While
  the caller's inbox is not open the request is answered at once with
  `409 {"refused": "inbox not open"}`, and a close ends a held one with it. Any other value is
  `400 {"refused": "undelivered is not 1"}`. Without the parameter every answer is as before;
  `inbox wait` and `inbox pending` do not send it (`docs/design/service.md`, "The wait").
- A Claude Code plugin, `adapters/claude-code/`, wakes an idle session when a message arrives in
  its inbox (`inbox open --adapter claude_code`, `claude --plugin-dir`): a `Stop` hook with
  `asyncRewake` runs `inbox wake` when a turn ends and hands the messages to the session between a
  begin line and an end line that carry a mark made for each run, with their count, so a sender
  cannot write a line of the plugin. The begin line says that the messages are data of other
  principals and names `inbox read`, `inbox answer` and `inbox refuse`. More than 6,000 bytes of
  messages are handed on as their first 20 head lines. A session without `INBOX_URL` and
  `INBOX_SECRET_KEY_FILE`, or whose inbox was opened for another adapter, is left alone, and a wait
  that fails without a message does not wake the session. It was run on one real headless session
  of Claude Code 2.1.330 (`claude -p`): idle for 59 s, the session took a turn 2.0 s after a note
  was posted and read it with `inbox read`. It was not yet run on an interactive session. Open:
  when Claude Code ends without ending the hook, or the
  hook alone is killed, its `inbox wake` keeps running and marks the next message delivered though
  no session reads it (`website/docs/limits.md`;
  story:the-claude-code-adapter-wakes-an-idle-session).

### Changed

- An answer of the service that is not JSON is named with its status in the `transport:` line of
  every command (`the answer with the status 404 is not JSON: …`). The exit status stays 3.

### Security

- The text output of `pending`, `wait`, `wake`, `outbox`, `collect`, `thread` and `replies` keeps
  each message to one line that is not indented, whatever its sender wrote. Before, a line end in
  the text of a decision option or in a refusal reason started a line at the left edge, so a sender
  could make one message read as 2, or as a message of another principal. Now such a line is
  indented under its text, a line end in an address or another value of a line is shown as `\u{a}`,
  and the separators U+2028 and U+2029 are shown as their escapes. `--json` is unchanged.

## [0.4.0] - 2026-11-23

### Added

- TLS on the loopback address. `inbox tls new --out <file>` makes an Ed25519 key and a
  self-signed certificate, writes both to one file with mode 0600 (it never overwrites a file)
  and prints the pin: the SHA-256 of the certificate's SubjectPublicKeyInfo in canonical base64,
  44 characters. `inbox serve --tls-file <file>` answers over TLS 1.3 only and prints an
  `https://` URL. A client given `INBOX_URL=https://…` and `INBOX_TLS_PIN` (`--tls-pin`)
  completes the handshake and accepts only the certificate the pin names before it sends a byte
  of its request; another certificate ends the command with exit status 3, and a post keeps its
  envelope in the outbox. An `https://` URL without a pin, and a pin beside an `http://` URL, are
  wrong usage (exit status 2). Plain `http://` on loopback stays the default. The request
  timeout, the limits of a request, `max_connections` and the stop hold over TLS as over plain
  HTTP. The pin does not protect against a reader of the TLS file, there is no revocation, and
  no command renews a certificate: each `inbox tls new` makes a new key and so a new pin for
  every client (`docs/design/service.md`, TLS). The listener still binds loopback only
  (story:the-service-speaks-tls-with-a-pinned-certificate).
- New dependencies of `inbox-server`: `rustls` 0.23 without its default features, with the
  `ring` provider, `ring` 0.17, and `zeroize` 1.8, which `inbox-cli` already used.
- The client reconnects by itself after a restart of the service. A client command whose answer
  carries an `Inbox-Epoch` other than the one the client recorded runs its own request first. It
  then opens the caller's own inbox again with the adapter of its last `inbox open`, kept in the
  new file `<outbox>.opened` beside the outbox file (`inbox close` removes it), and posts the kept
  envelopes again as `inbox repost` does. An envelope is marked in the epochs file (`<id>`, a tab,
  `none`) from the moment `inbox post` keeps it until an answer says it arrived or refuses it for
  good. A post that did not reach the service, was stopped on its way, was answered `408` or a
  `5xx` status such as `503`, or was refused `UnknownInbox` keeps its mark and is posted again by
  the sender's next command in the same way, with or without a restart. A post whose envelope
  another command of the sender posted meanwhile is answered `Duplicate` for its own id; `inbox
  post` takes that as arrived and exits 0. One line on standard error names a reconnect, and a
  `--json` document carries the member `reconnected` (`epoch`, `open`, `repost`). Without `--json`
  the command's own lines are written before the reconnect starts. A step that fails changes
  neither the command's output nor its exit status, and the next command tries again. The answers
  of the reconnect's own requests start no other reconnect. `inbox close` removes the record before
  it sends its request and puts it back when the close did not reach the service or was refused
  with anything but `UnknownInbox`; a reconnect whose open a `close` overtook closes the inbox
  again, and opens it once more when a typed `inbox open` wrote a record meanwhile. A reconnect
  replaces the record only while it is the one it started from. A typed `inbox repost` answered
  `408` or `5xx` now leaves its envelope marked, so the next command posts it again. A file beside
  the outbox that is no regular file, or is over its bound, is read as empty with one warning.
  After an upgrade run `inbox open` once (an inbox opened with an older version has no record) and
  `inbox repost` once (an envelope an older version kept has no mark, and an older version that
  writes the epochs file drops the marks). `inbox open` and `inbox repost` stay as typed commands
  (story:the-client-reconnects-after-a-restart).

### Changed

- Test builds compile every dependency at `opt-level = 2` (`[profile.dev.package."*"]` in
  `.cargo/config.toml`); the crates of the workspace stay unoptimized. The test step of the
  gate ran 108 s of its bound of 120 s for 633 cases at a load average of 134 to 146 on 20
  CPUs, and failed at the bound twice that day; with the setting it took 39 s at 105 to 139. No
  shipped code changes: `cargo build --release` and `cargo install` do not read the dev
  profile.
- The repository uses ESS 0.97.4 (`ess/ess-inputs.yaml` and `ESS_VERSION` of the check workflow).
  Before the reorder below, 0.97.4 writes `generated/` and the conformance suite byte for byte as
  the committed tree did. `PostMessage` now declares its outcome `unknown-inbox` above `duplicate`
  and `duplicate-key`: the `SYNTHESIS` note of `ess/domains/messaging.yaml` records that ESS
  answers it before them. The next ESS release refuses a declaration order that disagrees with
  that precedence; this is not measured for `PostMessage` on 0.97.4. The reorder changes 35 files
  of `generated/` (the `UnknownInbox` arm of `generated/rust/inbox/src/server/wire.rs` moves above
  `Duplicate`; the contract digest changes) and two digests of the suite
  (`provenance.contract_digest` and `provenance.spec_digest`); no scenario of the suite changes.
- No case of `crates/inbox-cli/tests/` or of `tests/acceptance/` holds a wall-clock time where
  a slow host fails it. What is left there is of three kinds: a wait with a lower bound only;
  a bound of 10 s or more that only ends a hang; and a short wait whose running out cannot fail
  the case (the 2 s for request lines after the client ended, which were all sent before it
  ended; a 1 s wait for a request line in a case that expects it to run out; 200 ms after a
  SIGTERM). The runner's bound of 60 s per case is the only bound of the case that prints
  65 536 escape characters. The cases of the other crates still hold such
  times (`inbox-server`, `inbox-backend-testkit`, `inbox-xtask`). A debug build of `inbox
  serve` takes the hidden flag `--clock-file <path>`: the service's clock reads the RFC 3339
  instant the file holds each time it asks for the time, and a file that is missing,
  unreadable, empty, no regular file (it is opened without blocking, so a FIFO refuses too),
  holds no instant or goes back in time ends the service with exit 4 (`docs/design/service.md`,
  the expiry task). A release build has no such flag. The acceptance check of an expired
  request and the case of a due note move that clock past `expires` instead of waiting for it,
  each `expires` 30 s after the system clock just before its post; the case of an epochs file
  of 20 001 lines counts the steps of the read instead of holding it to 2 s (each line split,
  each comparison of two message ids as the read gets them, each read of an id's text; a read
  that compares the texts themselves is not counted); the cases of a full pipe wait for a
  thread blocked in `write(2)` instead of 300 ms without a byte. The acceptance check and the
  epochs case had each failed on a host at a load of 80 to 100.

### Fixed

- An argument of `inbox` that a command reads as text and that is not UTF-8 (a positional
  argument, an option value, an argument before the subcommand) ends the client with the exit
  status 2 and one line on standard error, `inbox: usage: argument <n> is not UTF-8:
  "<argument>"`, in printable ASCII (each byte that is not printable ASCII written `\xE9`, a
  control character `\x1B`, a line feed, carriage return and tab `\n`, `\r` and `\t`, a
  quote and a backslash behind a backslash), and sends no request; with `--json` the usual
  usage document is printed as well. In every release up to and including 0.3.1 clap read
  the arguments as `OsString`: the option that takes a
  file name (`--out`, `--secret-key-file`, `--outbox`, `--key-file`, `--file`,
  `--operator-secret-key-file`, `--body-file`) took a name that is not UTF-8, and still does.
  An argument read as text made clap fail, and the client then called `std::env::args()` on
  that error path (to look for `--json`), which panicked: the exit status 101, outside the
  documented statuses 0 to 4, and a panic message in place of the usage line
  (story:an-argument-that-is-not-utf-8-is-a-usage-error).

- A message whose `expires` the service's clock has reached takes no read mark and no
  delivery mark: `read` and `mark-delivered` of a message in `Sent`, `Delivered` or `Read`
  answer `deadline-passed` (502, error `Expired`; `inbox read` exits 1), change nothing and
  publish nothing, and the message stays until the expiry task removes it or decides it on
  silence. A third principal still gets `unknown-message`. In every release up to and
  including 0.3.1 a due note its recipient read before the sweep was removed with
  `MessageHandled`, and the same note read after the sweep with `MessageExpired`: the event the
  sender got depended on when the task ran; now it is `MessageExpired` in both orders.
  `inbox pending` and `inbox wait` mark nothing and still list a due message until the sweep
  (story:a-due-message-takes-no-mark).
- A wait on `Pending` (`GET /messaging/views/pending?wait=N`, `inbox wait`) is not ended by a
  row whose `expires` is at or before the service's clock, which its owner can no longer read
  or mark: it ends with a row that is not due, at its bound, or at the stop, and then answers
  the whole view, due rows included. Without this, a wake loop of `wait` and `read` would get
  the due row back at once, again and again, until the sweep. `pending` without `wait` is
  unchanged. A wait that ends at its bound or at the stop reads the view once more and answers
  that read, so it lists no row the expiry task removed while it slept; a sweep that expired a
  message wakes every wait, which reads its view again.

## [0.3.1] - 2026-11-22

### Security

- A message whose `expires` the service's clock has reached takes no answer and no refusal:
  `answer` and `refuse` answer the new outcome `deadline-passed` (502, error `Expired`, CLI
  exit status 1), change nothing and publish nothing, and the expiry task decides the
  message as before, by silence with its `default_on_silence` or by removal. The clock is
  read inside the command's section at its full precision, the precision the expiry task's
  sweep reads it at, and is not taken from the request. In 0.3.0 a late answer that arrived
  before the sweep replaced the decision silence was due to take.
- `post` refuses an envelope whose `expires` is at or before the service's clock: the new
  outcome `expired-on-arrival` (502, error `AlreadyExpired`), and nothing is stored. 0.3.0
  compared `expires` with the envelope's own `created` only.
- `answer` refuses a reply to an open message whose own `expires` is at or before the
  service's clock with the same outcome and error, and the message stays as it was. An answer
  to a message already answered or refused stays `already-settled`, whatever its `expires`. Stored, such an answer would be
  removed by the next sweep before its sender could collect it.
- `collect` answers with the settled record it removes. The answer of `collected` is `200`
  (was `202`) and carries `response`: every member of the message's `Outbox` row in its order,
  `expires` (which the reply's signature covers) and the reply among them, read from the row in
  the request that deletes it. Its `body` is plain text, and its `reply` has the new type
  `CollectedReply`: the fields of `Reply`, its body as plain text; the JSON is the row's.
  `inbox collect` sends only the `Collect`, prints that record and then drops the outbox line.
  The text is the form `outbox` prints a row in, and `--json` puts the record under `message`
  with the member names and order 0.3.0 printed there; the values are those the service held
  when it removed the message. In 0.3.0 it read `Outbox` first and printed that row: a reply or
  a refusal that arrived between the two requests was deleted without the sender seeing it, and
  the outbox line was dropped. The behaviour of `Collect` is now an obligation of the
  generated crate, implemented in `inbox-core` (87 of 90 capabilities generated, 3
  obligations). A success answer that is not `collected` with the settled record of the message
  collected (no `response`, as a 0.3.0 service answers; another outcome or none; a record that
  does not decode as the declared response of `Collect`, is of another message or is not
  `Answered` or `Refused`; a member stated twice) makes `inbox collect` exit 3 with one line on
  standard error, and the outbox line stays. The conformance runner interprets the suite step
  `expect_direct_response`: it checks the response's members, their types (a tagged union
  among them), the alphabet and length of each newtype in it, and that no member is stated
  twice.
- The client writes no control character that another principal or the service chose to the
  terminal. In text, each control character of the output but the line end (C0 with the tab
  and a carriage return before a line end, DEL, C1) is shown as its escape `\u{1b}`: a reply
  body, a refusal reason, a message body, an option or an address printed by `collect`,
  `pending`, `wait`, `outbox`, `thread` or `replies`, and a line on standard error that repeats
  text of the service. In 0.3.0 they were printed as written, so a recipient could clear the
  sender's screen, set its window title or write over a line with its reply, and a sender
  could do the same to whoever lists an inbox. With `--json`, DEL and C1 are written as `\u`
  escapes as well (C0 was escaped already); a JSON reader gets the same values. Text without
  control characters is printed byte for byte as before. The stored message, its signature and
  the service are unchanged.
- The core applies the read rule. `inbox_core::handle` and `handle_with_events` answer a
  client only the rows of `Pending`, `Unanswered`, `Outbox`, `Settled`, `Thread` and
  `Replies` whose `to` is its inbox or whose `from` is its principal, whatever parameter it
  sends; the new `inbox_core::readable` is the rule, and the HTTP service calls it in place of
  its own copy. In 0.3.0 only `inbox serve` filtered the rows: an embedding of `inbox-core`
  that handed the core's answer on gave a third principal the bodies, replies and signatures
  of messages between others. `Due` and `Messages` stay whole for the expiry task, and a
  client is still refused both (story:the-core-applies-the-read-rule).
- The core answers a message between others as unknown. `inbox_core::handle` and
  `handle_with_events` answer a client that is neither the recipient nor the sender of a
  message, from `MarkDelivered`, `MarkRead`, `Answer`, `Refuse` and `Collect`, with the answer
  of an id no message carries (`unknown-message`, error `UnknownMessage`, nothing published),
  member for member. The new `inbox_core::as_unknown` is the rename, and the HTTP service
  calls it in place of its own copy; the bytes of every HTTP answer are unchanged. In 0.3.0
  only `inbox serve` renamed the model's `foreign-message`: an embedding of `inbox-core` that
  handed the core's answer on told a third principal that a message id exists. The new entry
  `inbox_core::handle_as_specified` answers the model's `foreign-message`; the conformance run
  against the core uses it, and an embedding does not. A client an embedding hands in without
  its inbox or its principal (or with a context that cannot read one) got an unmet obligation
  for a message between others and `unknown-message` for no message; `handle` and
  `handle_with_events` now answer such a refusal of the 5 commands as `unknown-message`, with
  no event. `inbox serve` hands in both attributes and is not changed by it
  (story:the-core-answers-a-foreign-message-as-unknown).

### Known limits

- A `collect` whose answer is lost after the service removed the message loses the record: a
  second `collect` answers `unknown-message`, and the envelope stays in the outbox file.

## [0.3.0] - 2026-11-22

### Added

- The landing page of the documentation site has an animated figure of one message going
  through the inbox, and 3 slides below it. Every command, view, error, state and actor they
  show is read from the compiled `ess/`: `inbox-xtask animation` writes
  `website/src/generated/animation.json` (`task animation`), and `task animation-drift`, in
  `task check`, fails when the file differs from what the command writes. 16 page tests
  (`website/tests/landing.spec.ts`, Playwright) hold the page, among them the count of open
  security findings to the limits page.
- `docs/checks.md`: the numbers the gate prints, what the checks do not cover, the closed
  security findings and the story of each open limit, moved out of the documentation site.
- 3 reference pages on the documentation site, each rendered by `ApiReference` of
  `@beyond10x/docs-system` from a generated document and each with a link that downloads
  the document: the HTTP API (`generated/openapi/openapi/inbox-service.yaml`), the events
  (`generated/asyncapi/asyncapi/inbox-service.yaml`) and the command line
  (`generated/cli/inbox.json`). 10 page tests (`website/tests/reference.spec.ts`) hold
  them.
- `generated/cli/inbox.json`: every command of `inbox` with its arguments, defaults,
  environment variables and exit statuses, written from the clap definition by
  `task cli-reference`. The case `the_cli_reference_document_is_current` of `cargo test`
  fails when a flag changes without the document.
- `task docs-words`, in `task check`: fails when a page of `website/docs` or
  `website/product.json` names a wave, a stage, a story, an epic or a count of scenarios.
- The event `MessageExpired` names the sender (`to`) and the thread of the removed message, as
  `MessageAnswered`, `MessageRefused` and `DecisionTakenOnSilence` do. A reader of 0.2.0
  ignores the 2 members it does not know.
- `task spec`, in `task check`: fails with the file and line when `ess/` holds an `UNMAPPED`
  marker.

### Changed

- The test step of the gate enforces its budget. `task test`, in `task check`, runs the cases
  with cargo-nextest (0.9.143, pinned in the CI job with its SHA-256): a case that runs 60 s is
  killed and fails, a run over 120 s is ended and fails (`tools/test-step.sh`), and the step
  prints its 10 slowest cases. `task acceptance` runs its 3 checks with the same runner. Each
  package has one integration-test binary (`tests/main.rs`, the case files as modules) and test
  builds carry no debug information (`.cargo/config.toml`): 18 test binaries where there were
  63, `target/debug/deps` 337 MB where it was 882 MB. The stand-in service of the client's
  cases waits at most 10 s for a connection. The cases of `crates/inbox-xtask` that need a
  synthesis of the unchanged `ess/` read one cache entry under `target/tmp/synthesis/`, keyed
  by the SHA-256 of `ess/`, the `ess` release and the arguments, which `task test` makes once
  before its bounded run: 3 syntheses per run where there were 18 (the 2 others are made by
  the cases that change `ess/`).
- The CI job `inbox / task check` ends green without installing a tool when a pull request
  or a push changes only planning or prose files: paths under `.engineering/`, and `*.md`
  files outside `website/` and `generated/` other than `README.md`. `tools/gate-paths.sh`
  holds the decision and `task gate-paths`, in `task check`, its 12 cases.
- No file of the repository is Python any more. The authored scenarios are the files of
  `ess/scenarios/`, written by hand; `task scenarios` checks them and rewrites
  `ess/ess-inputs.yaml` from them, and `task scenarios-drift` runs the same checks (one
  claimant per covered id, each claim of a type invariant at a view field resting on a row
  value, the scenario named as its file, a summary of at most 200 characters, message ids of
  26 characters) and names each path that keeps the manifest from listing every file once
  (`edited:`, `missing:`, `not in the manifest:`). `task animation`, `task animation-drift`
  and `task docs-words` are commands of `crates/inbox-xtask`; the path decision of the CI job
  is the POSIX shell script `tools/gate-paths.sh`, with the same 12 cases; the release
  workflow reads the workspace version with `cargo metadata` and `jq`. The scenario files,
  the manifest and `website/src/generated/animation.json` are unchanged.
- The text of the landing page states the release and its 2 open security findings. It said
  that no program and no client exist.
- The documentation site is written for a user of the tool. "What works" is now "Features":
  what is available with its command, the defaults of `inbox serve` with their flags, what is
  missing, and what is planned. The limits page states each limit as the user meets it. No
  page names a wave, a stage, a story or a count of scenarios or test cases.
- `unknown-option` of `post` (a `recommended_option` or `default_on_silence` that names no
  option of the decision, a `default_on_silence` without a decision, or two options with one
  letter) is a guard of the specification, no longer an `external:` outcome. A post is refused
  as in 0.2.0 and in the same order.
- `post` refuses a decision whose `options` list is empty: `decision-without-options`
  (`ShapeMismatch`), and nothing is stored. 0.2.0 stored it, and no letter could answer it.
- `refuse` of a note is refused: the new outcome `note` (`NotAnswerable`, as `answer` of a
  note), and the note stays as it was. 0.2.0 refused the note, and its sender could then
  collect it.
- ESS 0.97.3 is the pinned release, in place of 0.97.1: `ess/ess-inputs.yaml`, the manifest
  head `inbox-xtask scenarios` writes, and the release archive the CI job installs with its
  SHA-256. The generated crate, the projections and the data of the landing page's figure are
  byte for byte as before. The suite `task coverage` writes keeps its format, its 112
  scenarios and its coverage; 6 read-grant scenarios of `Replies`, `Settled` and `Unanswered`
  no longer read other views, so over HTTP 1 scenario, not 7, is unsupported for a claim on
  the stored signature (`unsupported.tsv`), and 81 stay unsupported. The synthesis of
  `task coverage` takes 20 s of CPU time where it took 41 s.

## [0.2.0] - 2026-11-22

### Added

- The binary `inbox` (`crates/inbox-cli`), built from source with
  `cargo build --release -p inbox-cli`. `inbox serve` starts the HTTP service over the
  in-memory backend on a loopback address. The other commands are the client: `open`,
  `close`, `post`, `pending`, `wait`, `read`, `answer`, `refuse`, `outbox`, `collect`,
  `thread`, `replies`, `repost`, `key new`, `keyfile add-key` and `keyfile add-delegation`.
  Each request is signed with the caller's key from a secret key file. Each posted envelope
  is kept in an outbox file until it is collected, refused with an error of the
  specification other than `UnknownInbox`, or expired. Exit statuses: 0 done, 1 refused, 2
  wrong usage, 3 transport, 4 a local file.
- Declared coverage: every refusal of the specification is claimed by an authored scenario.
  `task coverage` exits 0 (93 refused generated scenarios, 93 covered) and is part of
  `task check`; the suite it writes reports `conformance_status: passed` on ESS's
  interpreted target. 15 authored scenarios are new (38 authored, 111 in the suite).
- `task scenarios-drift`, in `task check`: fails when `ess/scenarios/` or
  `ess/ess-inputs.yaml` differs from what `tools/scenarios.py` writes.
- `task conform` also runs the conformance suite against a running `inbox serve` over
  HTTP (`crates/inbox-conformance`, `run --target serve`): a fresh service per scenario,
  every request and envelope signed with keys made for the run, each answer held to the
  HTTP status of the generated contract. 31 of the 111 scenarios pass there; 80 cannot be
  witnessed over HTTP and are listed with their reasons in
  `crates/inbox-conformance/unsupported.tsv`.
- `task acceptance`, in `task check`: 3 checks against the built binary
  (`tests/acceptance`): a signed request and its reply between 2 keys; a read note and an
  expired request in no view; after a restart, 2 reposts, the first answered `posted` and
  the second posting nothing.
- The response header `Inbox-Epoch` on every answer of `inbox serve`, whatever its status: 32
  lower-case hex digits made once from the OS random source when the listener is bound. The
  answers of one start carry one value and a new start makes a new one. It carries no time
  and no secret.
- The file `<outbox>.epochs` beside the outbox file, one line `<message id>\t<epoch>` per
  kept envelope whose post the service said it holds: an accepted post, or the outcome
  `duplicate`; `duplicate-key` records none, since the service stored nothing of that
  envelope. `inbox repost` asks the service for its epoch when a kept envelope has one, and
  leaves alone the envelopes recorded at that epoch: it does not post or list them. It posts
  every other kept envelope: one recorded at another epoch, one with none, and every one when
  the service sends no epoch. A missing epochs file, and a line of it that is malformed,
  doubled or unreadable, mean no epoch. When the epochs file cannot be written, `post` still
  prints the message id and exits 4, and `repost` posts every envelope, prints its report and
  exits 4. The outbox file keeps the form it had
  (`story:repost-does-not-redeliver-a-handled-note`).
- Tests and scenarios: 2 authored scenarios, `read-decision-is-decided-on-silence` and
  `silence-leaves-the-decision-as-it-was`, listed as unsupported over HTTP (a step as the
  Sweeper); 6 authored scenarios that hold the order in which `PostMessage` refuses when 2
  refusals apply (the signature, the option, the delegation and the reply shape before any
  row is read; the 5 row refusals in one order; a message id a delegate posted is its
  sender's), all 6 run over HTTP; cases of `inbox-core` for the events `Expire` hands back
  (each list compared whole, so one occurrence of each), for the order of the 3 external
  refusals of `PostMessage`, for a read decision decided on silence and for the fields
  `Expire` leaves as they were when it decides on silence; the acceptance case
  `tests/acceptance/tests/adversary.rs` removes the epochs file before each repost.

### Changed

- The rows of the views `Pending` and `Unanswered` no longer carry `reply`,
  `decided_option`, `answered_by` and `refusal`, and the rows of the view `Replies` no
  longer carry `refusal`. Against 0.1.0 these members were always absent there: `Pending`
  and `Unanswered` hold messages in `Sent`, `Delivered` or `Read`, and only an answered or a
  refused message carries the 4 fields; `Replies` holds answered messages only, and only a
  refused message carries `refusal`. They are gone from `GET /messaging/views/pending`,
  `GET /messaging/views/unanswered` and `GET /messaging/views/replies` and from the row
  schemas `inbox.messaging.Pending.Row`, `inbox.messaging.Unanswered.Row` and
  `inbox.messaging.Replies.Row`. `Settled`, `Thread`, `Outbox`, `Due` and `Messages` still
  carry all 4; `Replies` still carries `reply`, `decided_option` and `answered_by`.

- `task conform` runs the suite `task coverage` writes, and its report against the core
  says `conformance_status: passed`, where 0.1.0 reports `inconclusive`. The gate refuses
  `inconclusive`.

- `post` has a new outcome, `id-taken` (409, error `MessageIdTaken`): a post into an open inbox
  under a `message_id` that a stored message of another sender carries. `duplicate` now answers
  the sender's own stored message only; in 0.1.0 it answered any stored id. The row refusals of
  `PostMessage` come in the order `unknown-inbox`, `duplicate`, `duplicate-key`, `inbox-closed`,
  `id-taken`, then `posted`. A sender's repost of an arrived message still answers `duplicate`,
  also after the recipient closed the inbox. For the client `id-taken` is final: `post` and
  `repost` exit 1 and drop the outbox line. The generated OpenAPI document and the JSON Schema
  list the new error.

- ESS 0.97.1 is the pinned release (0.1.0 was written with 0.96.0).

### Security

- A post or an answer whose envelope `from` is not the caller's principal is refused:
  `PostMessage` and `Answer` answer `not-sender` (403, error `NotSender`) before every
  other refusal. In 0.1.0 a principal that held another principal's signed envelope could
  post the same bytes again under its own request signature; once the first message was
  handled and removed, the service answered `posted` again.
- `read`, `collect`, `refuse`, `answer` and `mark-delivered` answer a caller that is
  neither the recipient nor the sender of a message with `unknown-message` (404), the
  answer of an id no message carries. In 0.1.0 they answered that caller 403, which told
  any authenticated client that the message exists. The specification names the case as
  the outcome `foreign-message` of `MarkDelivered`, `MarkRead`, `Answer`, `Refuse` and
  `Collect`; the listener answers it with the status and the body of `unknown-message`.
  The generated OpenAPI document lists `foreign-message` under 403 for the 5 commands;
  `inbox serve` never sends it.
- A dedup key belongs to its sender: `post` is refused `duplicate-key` only for a key the same
  sender used on a live message, to any inbox. In 0.1.0 a key of any sender refused the post, so
  a third principal learned from a post whether a key was in use and could take another
  sender's key first (`story:dedup-key-scope`).
- A post to an address no inbox has, or into a closed inbox, answers the same for the id of
  another sender's message and for a free id. In 0.1.0 `post` answered `duplicate` for any
  stored id, before `unknown-inbox` (`story:post-does-not-tell-a-message-id-exists`).

### Known limits

- A principal that holds a message id learns from a post into an open inbox whether the id is
  live (`id-taken` against `posted`, which stores a message), and it can take a free id first,
  after which the sender's post is refused `id-taken`. This is accepted for principals of one
  operator. `story:message-id-per-sender` (a draft: a message id unique per sender) removes it
  and is required before principals of different operators share one service. A message id is a
  ULID the sender's client makes; the client has no flag to choose one.
- The gate is open for principals of one operator. A release, a listener outside loopback and
  a deployment each still need the operator's own order.
- The input `from` of `PostMessage` carries the example value `principal-262144` in the
  OpenAPI documents and the JSON Schema of `PostMessage`. It is a value ESS synthesis
  needs until beyond10x/ess#804 is fixed, and not an address anybody holds.
- The first `inbox repost` after a restart of the service delivers once more a note its
  recipient read before the restart. So does a `repost` against a service that sends no
  `Inbox-Epoch`, and one whose epochs file is lost or unreadable
  (`story:repost-does-not-redeliver-a-handled-note`).
- Over HTTP 80 of the 111 scenarios are unsupported: a caller whose principal and
  inbox differ (68), a step as the Sweeper (38), a step as no actor (9), a claim on the
  stored `signature` (7), one delegation id for two pairs (1); a scenario can have
  several. ESS reports that run as `failed`, and the gate holds it to the list.
- The run over HTTP and the acceptance checks send valid signatures only and read a view
  as every key. The read rule, the request signature and its window, the lapse of a key
  or a delegation, and the binding of a key to `from` are held by the tests of
  `crates/inbox-server` and `crates/inbox-verify`, not by those runs.
- No release attaches a binary: `inbox` is built from source.

## [0.1.0] - 2026-11-21

The first release. It ships no binary and publishes no crate: there is no program to run until
the command-line client exists.

### Added

- The specification in ESS (`ess/`): the domain `inbox.messaging` with 2 entities, 9 commands,
  9 views, 10 events and 13 errors, and 23 authored scenarios.
- The generated crate `generated/rust/inbox`, written by ESS 0.96.0 from `ess/` and held to it
  by `task drift`: 85 of 87 capabilities, with the obligations `PostMessage` and `Expire` left to
  the core.
- The core (`crates/inbox-core`): the 2 obligations over a backend, one command at a time per
  store, and the entry an embedding calls per request.
- The in-memory backend (`MemoryBackend` in `crates/inbox-core`). A restart loses every message.
- The signature verifier (`crates/inbox-verify`): Ed25519 over RFC 8785 canonical JSON for
  envelopes, key file entries and requests; the key ring of entries the operator key signed.
  What each signature covers: [`docs/design/signing.md`](docs/design/signing.md).
- The backend test kit (`crates/inbox-backend-testkit`): the checks every backend passes, for
  the 2 storage ports and the command section.
- The HTTP listener (`crates/inbox-server`), a library: request signatures, the read rule, the
  wait on `Pending`, the expiry task and the limits, on a loopback address only. Design:
  [`docs/design/service.md`](docs/design/service.md).
- The conformance gate (`crates/inbox-conformance`, `task conform`): the 92 scenarios ESS
  synthesizes, run against the core in process, all passed; `conformance_status` is
  `inconclusive`.
- The documentation site, <https://beyond10x.github.io/inbox/>, with the specification pages
  ESS generates (`generated/docs`), held to `ess/` by `task check`.

### Known limits

Stated in full on <https://beyond10x.github.io/inbox/docs/limits>.

- Security, open: a principal that holds another principal's signed envelope can post it again
  under its own request signature once the first message is gone
  (`story:post-from-is-the-caller`). Until that is fixed the service must not listen outside
  loopback and must not be used by a real session.
- Security, open: `not-recipient` (403) against `unknown-message` (404) tells any authenticated
  client whether a message id exists (`story:foreign-message-id-answers-as-unknown`).
- Security, open: a dedup key is compared over every sender's messages, so a third principal
  learns of a key or takes it first (`story:dedup-key-scope`).
- Security, open: no record of requests; a signed request is accepted again inside its 60 s
  window.
- No `inbox serve` binary and no command-line client (`story:cli-client`).
- Loopback only, no TLS. One backend, in memory. No second host, no wake-up.
- The conformance status is `inconclusive`: 50 refusals are claimed by no scenario
  (`story:declared-coverage`).
- Gaps in the suite, each a draft story: the order of `PostMessage` refusals
  (`story:post-refusal-order-scenarios`, `story:external-refusal-order-cases`), a read decision
  decided on silence (`story:read-decision-expires-on-silence-scenario`), the fields `Expire`
  leaves unchanged on silence (`story:silence-leaves-the-other-fields-scenario`), one event per
  `Expire` (`story:expire-publishes-once`).
- Two ESS issues are worked around:
  [beyond10x/ess#798](https://github.com/beyond10x/ess/issues/798) and
  [beyond10x/ess#799](https://github.com/beyond10x/ess/issues/799).
- Nothing is deployed.
