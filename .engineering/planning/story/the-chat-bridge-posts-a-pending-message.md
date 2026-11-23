---
format: aep.planning-md/3
id: story:the-chat-bridge-posts-a-pending-message
kind: story
status: proposed
title: The chat bridge posts each message of a person's inbox into the chat channel and marks it delivered
summary: 'First story of stage 4: a long-running client command posts each row of the person''s Pending through connectors and sends MarkDelivered; tested against a stand-in of connectors.'
relations:
- decomposes: epic:human-inbox
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: crates/inbox-cli/src/bridge.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/failure.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/output.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/src/wake.rs
- confidence: inferred
  path: crates/inbox-cli/tests/bridge.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/support
- confidence: inferred
  path: docs/design/bridge.md
- confidence: cited
  path: docs/design/wake.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:44Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

A person whose inbox is served by the chat bridge sees each message posted to that inbox as one
post in the chat channel, without typing `inbox pending`, and the message moves to `Delivered`.
The bridge is a long-running client of the service: it holds the `Pending` view of the person
open as `inbox wake --follow` does, and for each row in `Sent` that is not due it makes one chat
post through `connectors`, then sends `MarkDelivered`. It holds no provider token of its own.

This is the first and smallest story of epic:human-inbox: every later story of the epic (the
read mark, the decision asked once, the answer) stands on the bridge, its record of the chat
posts and its stand-in of `connectors`, which this story makes.

## Acceptance

A note posted to the inbox of the person while the bridge runs makes exactly one invocation of
the chat provider's post operation through `connectors`, naming the chat channel of the binding
and carrying the sender, the kind and the body of the message, and the message is in
`Delivered`, where before this story a message to a person's address is shown only by
`inbox pending`, `inbox read` or `inbox wake --follow` in a terminal (state transition: `Sent` to
`Delivered` with no typed command, through a chat post).

Checks, each of which can fail:

1. The nouns are drafted first, outside `ess/`: `docs/design/bridge.md` gets a section "The
   nouns of the bridge" with `ChatChannelBinding` (the address of the inbox of the person, the
   chat channel, the allowed chat user) and `ChatPost` (one inbox message id, the chat
   provider's reference of the post that shows it, and the state of the bridge's handling of
   it), and each relation the sources do not settle is written there as `UNMAPPED:` with the
   Open item that answers it (see Domain relations). A relation enters `ess/` only once the
   operator has answered its Open item, written out with no marker; a rule ESS cannot express is
   an `external:` outcome listed in `docs/design/inbox.md` under "What the specification cannot
   say". After the story `grep -rn UNMAPPED ess` prints nothing and `task spec` exits 0. When a
   document of the bridge enters `ess/`, the time of `task coverage` and of the synthesis is
   measured before and after, and both times are in the story's report (folder rule "Tests stay
   fast": a new domain of a generator is timed before and after).
2. A note, a request and a decision posted to the inbox each give one invocation of the post
   operation, in the order of `Pending`, and `MarkDelivered` is sent for a message only after the
   stand-in answered its invocation with success; the message is then in `Delivered`.
3. A post operation that fails (the stand-in exits non-zero, or answers a refusal) sends no
   `MarkDelivered`: the message stays in `Sent`, and the bridge tries it again after the pause of
   a failure (`wake::pause`), with one line on standard error per run of failures. A post that
   succeeded is written into the record of the bridge (a `ChatPost` with the reference the
   stand-in answered, Open 6) before `MarkDelivered` is sent. That a message whose id the record
   holds is not posted again after a restart is story:the-chat-bridge-asks-a-decision-once.
4. Under Open 7 A: a body that holds the channel-wide mention of the chat provider and the
   delimiter of its block of literal text, both as `docs/design/bridge.md` records them from the
   connector's description of the post operation (`connectors invoke --describe true`, which
   invokes nothing), reaches the stand-in inside one block of literal text, with the delimiter
   escaped as recorded; the input document the stand-in received holds that mention only inside
   the block; and an ESC (0x1B) and a U+202E of the body reach it as `\u{1b}` and `\u{202e}`, as
   `rows_text` shows them (`crates/inbox-cli/src/output.rs`). The case fails when the delimiter
   arrives unescaped, the mention outside the block, or either control character raw.
5. The bridge opens the inbox with the adapter of Open 4, and a second bridge of the same inbox
   with the same outbox file on the same host exits 5 and posts nothing (the lock of the wake,
   `docs/design/wake.md`, "One wake per inbox on a host").
6. The bridge refuses to start, with one line and no request, when no binding is configured for
   the inbox of the caller, when `connectors` is not on `PATH`, or when the URL of the service is
   not a loopback URL; it starts no listener of its own (Open 8).
7. Every case runs against a real `inbox serve` on a loopback address and a stand-in
   `connectors` executable on `PATH` that the case writes (POSIX sh, or a fixture of the test
   binary), which records the arguments and the input document of each invocation and answers
   from a script the case gives it. No case reaches a chat provider, a network beyond loopback or
   a model. No case runs over 60 s, the test step stays within 120 s, no case holds a wall-clock
   time (story:no-test-holds-a-wall-clock-time), and the new cases are a module of
   `crates/inbox-cli/tests/main.rs`. No new Python: the bridge is Rust, the stand-in Rust or POSIX
   sh. A build or a test runs through `slot` (the `cargo` shim); a pool job ends after 10 minutes
   unless it names its time with `slot --timeout` (30 minutes at most).

## Context

epic:human-inbox, Outcome: "A long-running bridge is a client of the service: it reads the
person's `Pending` view, posts each message into a chat channel, marks it delivered". Its
Context: "The bridge calls the chat provider through `connectors`; it holds no provider token of
its own. It replaces a per-session poller of the chat channel."

What the client has: `inbox wake --follow` (`crates/inbox-cli/src/wake.rs`) holds the own
`Pending` of the caller open in rounds with `undelivered=1`, runs the reconnect after every
answer, writes the rows of each round and then sends `MarkDelivered` for each, holds one lock
per inbox and outbox file on a host, and refuses an inbox this host opened with another adapter
(`terminal_only`, `wake.rs:186`; `docs/design/wake.md`, `## terminal`). `docs/design/wake.md`
ends the terminal section with "a person inbox of its own (epic:human-inbox builds on this
mode)". The bridge is that mode with another surface: the chat channel instead of a terminal.

What `connectors` offers, read from its help only (`connectors --help`, `connectors invoke
--help`, `connectors help approvals issue`, `connectors help approvals policy-set`,
`connectors server --help`; no connection started, nothing invoked, no configuration printed):
`connectors invoke [CONNECTOR[/INSTANCE]] <OPERATION>` with `--input-json`, `--input-file` or
`--input-stdin`, `--output json`, `--approval-file` ("The approval proof a write is invoked
with") and `--idempotency-key` ("A write's idempotency key"); `connectors approvals issue`
approves "the exact reconstructed subject" of one operation and one input; `approvals
policy-set` publishes "the exact permitted write-operation set"; `connectors server` serves
"admitted read-only MCP tools over stdio". Not verified: which operations the connector of the
chat provider has (post a message, read reactions, read a thread); whether a write runs without a
proof per write; whether an idempotency key makes a repeated post arrive once.

The bridge runs on the host of the service: the service binds a loopback address only and the
client refuses an `INBOX_URL` outside loopback (`docs/design/service.md`, "Binding and
stopping"; `website/docs/limits.md`, "Loopback only"). It reaches the chat provider only by
starting `connectors`, so it needs no listener: it learns of reactions and replies by asking
(later stories), not by being called.

The bridge signs its requests with a key of its own under a delegation (the operator's answer B,
approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner), built in
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation. This story does not wait for that
story: it signs with the secret key file it is given (`INBOX_SECRET_KEY_FILE`), and its
cases give it a key whose entry names the address of the person as its principal
(`human:person`), which today is the only key that may read that `Pending` (decision 5 of
`docs/design/inbox.md`; `docs/design/service.md`, "The caller").

## Domain relations

- Message to Inbox: many messages reference one inbox through `to` (`ess/domains/messaging.yaml`,
  entity `inbox.messaging.Message`, relation `inbox`, `kind: references`, `cardinality: one`,
  `via: to`, lines 230-235). Inferable from the ess/1 document.
- Inbox to Adapter: one adapter per inbox, set by `OpenInbox` (`opened` and `reopened` set
  `adapter: input.adapter`) (same file, entity `inbox.messaging.Inbox`, field `adapter`, line
  196; command `inbox.messaging.OpenInbox`, lines 333-362; enum `inbox.messaging.Adapter`, members
  `claude_code`, `metaharness`, `slack`, `terminal`, lines 69-73). Inferable from the ess/1
  document.
- Who moves a message to `Delivered`: only the owner of the inbox in `to`, and only from `Sent`
  (same file, command `inbox.messaging.MarkDelivered`, lines 542-579; transition `deliver`, line
  246). Inferable from the ess/1 document.
- `ChatChannelBinding` to Inbox: no ESS document declares the noun. Whether one inbox has one
  binding or several, and whether one chat channel binds one inbox or several, no source says:
  `UNMAPPED:` in `docs/design/bridge.md` (check 1), answered by Open 2.
- `ChatChannelBinding` to the allowed chat user: no source says whether a binding names one chat
  user or several: `UNMAPPED:` in `docs/design/bridge.md`, answered by Open 3.
- `ChatPost` to Message: the epic says a decision is "asked once" (epic:human-inbox, Context),
  which bounds a decision to at most one chat post; for a note and a request no source says:
  `UNMAPPED:` in `docs/design/bridge.md`, answered by Open 1 of
  story:the-chat-bridge-asks-a-decision-once.
- The states of a `ChatPost`, as far as the sources go: posted (this story), read
  (story:the-chat-bridge-marks-a-message-read-on-a-reaction), answered
  (story:the-chat-bridge-turns-a-thread-reply-into-an-answer), decided on silence or expired
  (story:the-chat-bridge-asks-a-decision-once). Whether a `ChatPost` is ever removed, and when,
  is Open 1 of story:the-chat-bridge-asks-a-decision-once (options A to C; default A: dropped
  when its message is gone from the service and its `expires` plus a retention has passed).
  This story removes no entry.
## Open

1. Where the bridge lives. (A) a command of the `inbox` binary, `inbox bridge`, in
   `crates/inbox-cli/src/bridge.rs`: it reuses the client, the reconnect, the outbox file, the
   lock and the rounds of `wake.rs` as they are; the crate has no library target, so another
   crate could not reuse them without moving them; (B) a crate of this workspace with its own
   binary, `crates/inbox-bridge`: first a story that moves the client of `crates/inbox-cli`
   into a library both binaries use, on every file of its `src/`; (C) `adapters/chat/`, a POSIX
   sh script around `inbox` and `connectors invoke`, as `adapters/claude-code/` is: no new Rust,
   but a long-running loop that parses the JSON of both in sh, and `inbox wake --follow`
   refuses an inbox opened with another adapter. Default: A.
2. Where the binding is configured, and how many one bridge serves. (A) one binding per bridge
   process, given by flags or a file a flag names (`--chat-channel`, `--allowed-chat-user`): one
   bridge per inbox of a person; (B) a file beside the outbox file that lists bindings, one
   bridge for several inboxes, so several keys; (C) the configuration of `connectors` (not
   verified that it can hold such a value). Default: A.
3. Which chat user is the allowed chat user. (A) exactly one chat user id per binding, given by
   the operator with the binding (Open 2) and compared with the user id the answer of the
   provider names, never with a display name; (B) several ids; (C) any member of the chat
   channel. Default: A. This story records it; story:the-chat-bridge-marks-a-message-read-on-a-reaction
   is the first to compare it.
4. Which adapter the bridge opens the inbox with, and how it shares the inbox with
   `inbox wake --follow`. (A) `slack`, the member of `inbox.messaging.Adapter` the epic names;
   like `--follow`, the bridge reads this host's record of `inbox open` (`<outbox>.opened`) and
   ends with 1 and one line when it names another adapter, and `inbox wake --follow` refuses an
   inbox the bridge opened (`wake.rs:186`), so a terminal and the bridge never both serve one
   inbox with one outbox file; (B) `slack`, and both may run: each shows what it was handed, and
   the messages of the other stay in `Delivered` for it; (C) a new member `chat` of `Adapter` (a
   change of `ess/domains/messaging.yaml` and of `generated/`), when the provider is not the one
   the member names. Default: A.
5. How the bridge invokes `connectors`, and what a write needs. (A) one `connectors invoke`
   child process per operation, `--output json`, the input document on standard input
   (`--input-stdin`), so no message text is on a command line; (B) `connectors server` over
   stdio: its help says it serves read-only tools, so it could not post (not verified that no
   write tool is admitted); (C) a library of `connectors` linked into the bridge (not verified
   that one is published). For a write: (i) a write policy of `connectors` (`approvals
   policy-set`) that admits the post operation for the channel of the binding without a proof
   per write, set up by the operator, not by the bridge; (ii) a proof per post (`approvals
   issue`, run by the operator for each message: then the bridge cannot run unattended). Not
   verified which of the two `connectors` allows. Default: A with (i).
6. Where the record of the chat posts is kept, and where its ESS document sits once its
   relations are answered. (A) a file beside the outbox file, `<outbox>.chat.<16 hex digits>`
   (the hash of the address, as the lock of the wake), one JSON line per `ChatPost` (message id,
   reference of the chat post, state), changed under the outbox lock and read as the other files
   beside the outbox file are read, so later stories find each chat post again; (B) no record:
   the bridge writes the message id into the text of the chat post and finds it again by reading
   the channel; (C) kept by `connectors` (not verified). For the document, once the operator has
   answered the relations of check 1: (i) a domain of its own with a component `chat-bridge` in
   `ess/components.yaml`, synthesized as the service's is (its cost measured, check 1); (ii) a
   document under `ess/` that `ess specify validate` reads and synthesis does not (not verified
   that `ess-inputs.yaml` can leave a document out); (iii) the draft stays in
   `docs/design/bridge.md` only. Default: A with (iii) until the relations are answered, then
   (ii).
7. How a message is shown in the chat channel. (A) one chat post per message: a head line (kind,
   sender, `on_behalf_of` when it differs from the sender, `expires`, message id) and the body in
   the block of literal text of the provider, so no markup of a sender is read as markup; the
   escapes of the provider applied as `connectors` documents them (not verified); (B) the body as
   plain text with the escapes of the provider only. Default: A. How the options of a decision
   are shown is Open 1 of story:the-chat-bridge-turns-a-thread-reply-into-an-answer.
8. Whether the bridge needs a listener outside loopback. (A) no: it asks the chat provider
   through `connectors` for reactions and replies at an interval (later stories), starts no
   listener and needs no inbound connection; (B) the provider calls the bridge (a webhook or an
   event socket): a listener outside loopback, which needs the operator's own order and is not
   assumed. Default: A.

## Scope

For the defaults of Open 1 (A) and Open 6 (A, iii). This story is the first of the bridge
stories; story:the-chat-bridge-marks-a-message-read-on-a-reaction,
story:the-chat-bridge-asks-a-decision-once, story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
and story:the-chat-bridge-turns-a-thread-reply-into-an-answer edit `bridge.rs` after it, in
that order (edges `depends_on`). It edits no file under `ess/` at the defaults; `ess/` enters
its scope only when the operator answered a relation of check 1 before it is built.

Code:

- `crates/inbox-cli/src/bridge.rs` (new: the command, its rounds, the record of the chat posts,
  the invocation of `connectors`): inferred
- `crates/inbox-cli/src/main.rs` (the command `bridge`, its flags and help): cited
- `crates/inbox-cli/src/wake.rs` (the rounds and the lock, callable with another surface than
  standard output): cited
- `crates/inbox-cli/src/commands.rs` (`undelivered`, `mark_delivered`, `open`): cited
- `crates/inbox-cli/src/outbox.rs` (the path of the record beside the outbox file, under the
  outbox lock): cited
- `crates/inbox-cli/src/output.rs` (the text of a chat post with each control character escaped;
  the one file that writes standard output and standard error): cited
- `crates/inbox-cli/src/failure.rs` (the failures of the bridge and their exit statuses): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json` (the command-line reference):
  cited
- `crates/inbox-cli/tests/bridge.rs` (new) and one `mod` line in `crates/inbox-cli/tests/main.rs`:
  inferred (the directory and `main.rs` are cited)
- `crates/inbox-cli/tests/support/` (the stand-in `connectors`): inferred

Documents:

- `docs/design/bridge.md` (new: the bridge, its record, its use of `connectors`, what it leaves
  out): inferred
- `docs/design/wake.md`: the last paragraph of `## terminal`: cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`, a row for `docs/design/bridge.md`, the
  Commands row of the client commands: cited
- `website/docs/status.md`: a row in "Available", and the planned item "An inbox for a person":
  cited
- `website/docs/first-use.md`: a section for the bridge: cited
- `website/docs/limits.md`: what the bridge does not do: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
