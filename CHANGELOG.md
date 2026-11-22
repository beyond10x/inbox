# Changelog

What changed in each release of the inbox. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`. How a release is cut:
[`docs/releasing.md`](docs/releasing.md).

## [Unreleased]

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
