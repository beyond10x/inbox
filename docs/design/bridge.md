# Design: the chat bridge

`inbox bridge` serves the inbox of a person through a chat channel
(story:the-chat-bridge-posts-a-pending-message, the first story of epic:human-inbox). It is a
long-running client of the service: it holds the caller's own `Pending` open as
`inbox wake --follow` does, and for each message in `Sent` that is not due it makes one chat post
through `connectors`, records it, then sends `MarkDelivered`. The message moves from `Sent` to
`Delivered` with no typed command. The bridge holds no token of the chat provider and starts no
listener: it reaches the provider only by starting `connectors`, which holds the connection. The
code is `crates/inbox-cli/src/bridge.rs`.

## The command

```
inbox bridge --chat-channel <id> --allowed-chat-user <id> --chat-connector <connector[/instance]>
```

with the client's `INBOX_URL`, `INBOX_SECRET_KEY_FILE` and outbox file, as every client command.
It signs with the key file it is given; the key's entry names the address of the person as its
principal (`human:<name>`), which today is the only key that may read that `Pending`
(`docs/design/service.md`, "The caller"). A key of its own under a delegation is
story:the-chat-bridge-runs-with-its-own-key-under-a-delegation.

It ends only by SIGINT, SIGTERM or a failure, with the exit statuses of `inbox wake`: 1 a refusal
(another adapter, the inbox not open and not opened again), 2 wrong usage (a URL outside loopback,
no binding), 3 an answer of the service that is neither a success nor a refusal, 4 a local file
(the lock, the record of the chat posts) or no `connectors` on `PATH`, 5 another wake or bridge of
the inbox and outbox file on this host. Its lines on standard error start with `inbox bridge: `;
it writes nothing to standard output but the document of a failure with `--json`.

## The nouns of the bridge

Drafted here, outside `ess/`; no file under `ess/` is changed by this story.

- `ChatChannelBinding`: the address of the inbox of the person, the chat channel the messages are
  posted into (an id of the provider), the allowed chat user (an id of the provider), and the
  target `connectors invoke` is called with (a connector, or a connector and its instance). It is
  given by the 3 flags of the command; the address is the caller's own inbox.
- `ChatPost`: one inbox message id, the chat provider's reference of the post that shows it (the
  `ts` of the answer), and the state of the bridge's handling of it.

Relations:

- `ChatChannelBinding` to Inbox: one binding per bridge process, and one bridge per inbox and
  outbox file on a host (the lock of the wake). Whether one chat channel serves several inboxes is
  not restricted: two bridges of two inboxes may name the same channel. (Open 2 of the story,
  answered A.)
- `ChatChannelBinding` to the allowed chat user: exactly one chat user id per binding, given by
  the operator with the binding. It must not be empty and holds no white space and no control
  character; this story stores and checks it and compares it with nothing.
  story:the-chat-bridge-marks-a-message-read-on-a-reaction is the first to compare it, with the
  user id the provider's answer names, never with a display name. (Open 3, answered A.)
- `ChatPost` to Message: UNMAPPED: whether a note or a request has at most one chat post, as a
  decision has by "asked once" (epic:human-inbox), no source says. Open 1 of
  story:the-chat-bridge-asks-a-decision-once answers it. Today the record holds one line per
  message id: a second post of a message replaces its line.

The states of a `ChatPost`: this story gives it one, `posted`, and no transition. The later
stories add read (story:the-chat-bridge-marks-a-message-read-on-a-reaction), answered
(story:the-chat-bridge-turns-a-thread-reply-into-an-answer), and decided on silence or expired
(story:the-chat-bridge-asks-a-decision-once), and with them the transitions; the ESS document of
the bridge is written then, not in this story (Open 6, answered A with iii: the draft stays here).
No entry is ever removed yet.

## The rounds, the lock and the adapter

The bridge runs the rounds of the wake (`wake::rounds` in `crates/inbox-cli/src/wake.rs`, with
the chat channel as its `Surface`): the read without a wait, then the read held up to 60 s, both
with `undelivered=1`; the reconnect after every answer; the tries and pauses of
`docs/design/wake.md`, "Failures and tries"; SIGINT and SIGTERM by their default action. Nothing of
that is a copy. The watch of the reader of standard output (`docs/design/wake.md`, "A reader that
is gone") belongs to the surface that writes rows there; the bridge writes none and has no watch.

- The start. Once it holds the lock the bridge opens the inbox (below). That open runs through the
  tries and pauses of a read: no answer, or an answer that may pass (`408`, from `500` on, no
  refusal of the model), gives the line `inbox bridge: <why>; trying again until the service
  answers` once and is tried again after the pause of a failure, so a bridge started before the
  service waits for it. A refusal of the open ends the bridge with its exit status.

- The lock. Before its first request it takes the lock of the wake
  (`<outbox>.wake.<16 hex digits>`, `docs/design/wake.md`, "One wake per inbox on a host"). A
  second bridge, an `inbox wake` or an `inbox wake --follow` of the same inbox with the same
  outbox file on the host finds it held, sends nothing and exits 5 (the bridge's line:
  `inbox bridge: another wake of <address> runs on this host`).
- The adapter is `slack` (Open 4, answered A). The bridge reads this host's record of an
  `inbox open` of the caller (`<outbox>.opened`) at the start, before the lock and any request,
  and in every round before its posts; when the record names another adapter it ends with 1 and
  one line that names `inbox open --adapter slack`. Once it holds the lock it opens the inbox with
  the adapter `slack` (`OpenInbox`, which records `slack` in `<outbox>.opened`), so after a restart
  of the service the reconnect opens it again with `slack`. `inbox wake --follow` ends with 1 on
  an inbox the bridge opened, as it does on any other adapter's: a terminal and the bridge never
  both serve one inbox with one outbox file. An inbox that another host opened with another
  adapter is answered `already-open` and keeps its adapter at the service.

## A post

One `connectors` child process per post (Open 5, answered A), started from one function
(`invoke`), whose one caller is `post`:

```
connectors --output json invoke <target> chat.postMessage --input-stdin --idempotency-key inbox-<message id>
```

- `connectors` is the first executable regular file of that name on `PATH`, looked up at the
  start, before anything is asked of the service.
- The input document is written to the child's standard input and is
  `{"body":{"channel":"<channel id>","text":"<text>"}}`: exactly these 2 members under `body`, and
  nothing beside `body` (the standing approval of the operator admits no other member). No text of
  a message is on a command line or in the environment. The target is refused at the start when
  it starts with `-`, which `connectors` would read as a flag.
- The idempotency key is `inbox-` followed by the message id: derived from the message id only,
  so every try of the same message repeats it. `connectors` says that a write whose answer is lost
  is classified `unknown` and never sent again, and that repeating the same key observes the
  original attempt (its page "A guarded write"; not verified here against a real provider).
- The child's standard output and standard error are captured; nothing of them reaches the
  bridge's own output as it is. A thread per stream reads it and keeps at most 256 KiB
  (`MOST_ANSWER_BYTES`); the bytes over the bound are read and dropped, so a full pipe does not
  stop the child, and a stream over the bound is a failed post. A failure line repeats at most the
  first line of 200 characters of them, through the escaping of every line on standard error, as
  text of another party.
- The exit of the child ends the post. The bridge waits for the exit, not for the end of the
  streams: it looks every 10 ms (`LOOK`) whether the child has exited, at once when a stream ends.
  After the exit it reads on for at most 1 s (`DRAIN`) until both streams end; the child's own
  bytes are in the pipes when it exits, so this ends at once unless another process (a helper the
  child started) holds a pipe open, and then the post is judged by what was read. Such a process
  holds no post and no round; its reading thread ends when it closes the pipe.
- The child is the leader of a process group of its own (`process_group(0)`), and the processes
  it starts are in that group unless they leave it.
- A SIGINT or SIGTERM to the bridge during a post ends that group first: while it runs the child
  is in a slot the signal path of the rounds reads (`wake::child`), and that path sends SIGKILL to
  the group (`rustix::process::kill_process_group`, no `unsafe`), then to the child alone, and
  waits for the child before the bridge ends by the signal. The child is not yet waited for while
  it is in the slot, so its id is still its own and names its group and no other. No process of
  the group of a running post outlives a bridge ended by one of these signals. A process the child
  started that left the group (a session or a group of its own) is not reached and outlives the
  bridge.
- In a terminal the interrupt key sends SIGINT to the foreground job, the bridge's group: it
  reaches the bridge and no longer the child, whose group is another one, and the bridge's signal
  path ends the child's group. The child reads and writes no terminal (its 3 streams are pipes),
  so a group outside the foreground job does not stop it. The suspend key stops the bridge and
  not a running child, which finishes its post meanwhile; the bridge judges it when it goes on.
  A hang-up of the terminal (SIGHUP) ends the bridge by its default action, outside the signal
  path, and leaves a running child and its group as a SIGKILL does.
- A post that ends by itself is not followed by a signal to its group: once the child has exited
  the post is judged, and a helper a finished `connectors` leaves behind is `connectors`' own (it
  may be meant to outlive the post); the bridge ends only what holds a post that is not finished.
  The drain after the exit stays as it is (above).
- A SIGKILL of the bridge cannot be caught: then the child and its group run on until they end
  by themselves, and a post the child makes is neither recorded nor marked. In each of these
  cases the message stays in `Sent`, and the next bridge posts it again with the same idempotency
  key: if the first child had made the post, `connectors` answers that try with the answer of the
  first (a replay), and no second post reaches the channel.
- The shape of the answers, as the contract of `connectors` states it (0.77.0, not released): the
  shape of a write that ran is read from its code of today; the failure codes of the operator's
  grant (`approval_required`, `rate_limited`, `idempotency_conflict`, `unavailable`) are the
  design of `connectors` and not released.
  - A write that ran exits 0 with one JSON document on standard output:
    `result.mutation.classification`, `result.result.status`, `result.result.body.ok`,
    `result.result.body.ts`, and maybe `result.mutation.standing.grant_id`.
  - Every failure exits 1 with standard output empty and one JSON document on standard error:
    `ok` `false`, `error.code` the fixed string `failure`, the real code in `error.data.code`,
    the classification in `error.data.mutation.classification` (`not_attempted`, `refused` or
    `unknown`), and for `rate_limited` `error.data.retry_after_seconds`, a whole number from 1 to
    3600 (in the failure's `code`, not in a `service_code`).
  - A refusal under the standing grant is classified `not_attempted`. A refusal at the first check
    carries no `standing` member; a refusal at the recheck before dispatch carries
    `mutation.attempt` and `mutation.standing.grant_id`. The bridge reads neither: both are the
    refusal they are, by their exit, their code and their classification.
  - The same key with the same input replays the answer of the first write; the same key with
    another input is `idempotency_conflict`.
  - A usage error (a bad flag) exits 2 with no JSON promised.
- The bridge reads standard output on exit 0 and standard error on every other exit. The post was
  made only when the child exits 0, its standard output is one JSON document, and that document
  has `result.mutation.classification` `applied`, `result.result.status` 200,
  `result.result.body.ok` `true` and `result.result.body.ts` a text that is not empty: that text is
  the reference of the post. Members the bridge does not know (such as
  `result.mutation.standing.grant_id`) are left alone. Every other run is a failed post, and its
  line says which: an exit 0 whose document is not that (with what it lacks) or that is no JSON;
  an exit 1 with the failure document (its classification, its code, and its message from
  `error.data.reason` or `error.message`), or whose standard error is no JSON or not a failure
  document; an exit 2, a usage error; another exit; a child ended by a signal.
- The classification `unknown` is an outcome nobody knows: `connectors` does not send the same key
  again, so the bridge's next try with the same key observes the first attempt. Its line says so.
- The codes of the operator's grant, each a failed post with its own line:
  - `approval_required` (classification `not_attempted`): no live grant covers the post (none,
    ended, another channel, or a member it does not name). The line also says that the operator's
    grant of `connectors` is missing, ended or does not cover this post.
  - `rate_limited`: the cap of writes per hour of the grant is reached. The next try of the
    message is not before `retry_after_seconds` when it is a whole number from 1 to 3600 (one
    hour, the span of the cap); a missing value, or one outside 1 to 3600 or not a whole number,
    gives no wait of its own, and the pause of a failure applies. The pause of a failure follows
    the wait.
  - `idempotency_conflict` (classification `not_attempted`): the idempotency key was used with
    another input document. Tried again at the pause of a failure, never in a tight loop (see
    "What stays open").
  - `unavailable` (classification `not_attempted`): the approval clock of `connectors` cannot be
    read, or its grant file is unsafe. A failed post whose line says so, tried again at the pause
    of a failure.
  - Every code the bridge does not know: a failed post whose line names the code, tried again at
    the pause of a failure.
    A `result` or `error.data` that carries `mutation.standing.grant_id` says nothing about
    whether the post was made: only exit 0 with `applied` is a post made.

The order is fixed: the post, then the record, then `MarkDelivered`.

- A post that succeeded is written into the record of the chat posts before `MarkDelivered` is
  sent. A record that cannot be written ends the bridge with 4 and one line, and no mark is sent:
  the message stays in `Sent`, and the next bridge posts it again with the same key.
- A post that failed sends no `MarkDelivered`: the message stays in `Sent`. The round ends at it,
  so the posts keep the order of `Pending`, and the next round starts after the pause of a
  failure (`wake::pause`: 100 ms, doubled per round in a row, up to 30 s) and tries it again. One
  line on standard error names the first failure of a run of them; the next line comes after a
  post succeeded and another failed.
- A mark that fails is named in one line, as `inbox wake --follow` names it, and the next round
  starts after the pause of a failure. The row may come again, and is then posted again with the
  same key.
- That a message whose id the record holds is not posted again after a restart of the bridge is
  story:the-chat-bridge-asks-a-decision-once; this story does not read the record.

## The text of a post

One chat post per message (Open 7, answered A): a head line, then the body in one block of
literal text.

```
<kind> from <sender>[ on behalf of <principal>] expires <expires> message <message id>
```
````
```
<body>
```
````

The text is built in `crates/inbox-cli/src/output.rs` (`chat_post`). The following are the
coordinator's decisions for this story, not verified against the connector's description of the
post operation (`connectors invoke --describe true` was not run) nor against the provider:

- Every character of the message that `rows_text` shows as its escape (`visible`: the control
  characters but the line end, U+2028, U+2029, the bidirectional controls, the characters a
  terminal shows with no width) is its escape here too, such as `\u{1b}` and `\u{202e}`. In the
  head line a line end is `\u{a}` as well.
- Then the provider's 3 escapes on all text the bridge takes from a message: `&` as `&amp;`, `<`
  as `&lt;`, `>` as `&gt;`. So the channel-wide mention (`<!channel>`, `<!here>`, `<!everyone>`)
  and a mention of a user arrive as text inside the block, never as a mention.
- The delimiter of the block is 3 backticks. In the body each backtick of a run of 3 or more is
  written `\u{60}`, so no sender ends the block or opens another; a run of 1 or 2 stays. The head
  line holds no backtick of a sender: there each backtick is `\u{60}`.
- The bound: a body whose text in the block would be over 3000 bytes is cut before the first
  character whose text does not fit, so no escape is cut in half, and a line after the block says
  `cut at 3000 bytes: inbox pending shows the whole message <message id>`. `inbox pending` shows
  the whole body; the message is marked delivered all the same.

## The record of the chat posts

`<outbox>.chat.<16 hex digits>` beside the outbox file (Open 6, answered A with iii): the digits
are the FNV-1a hash of the address, as in the lock of the wake. One JSON line per `ChatPost`,
`{"message_id":…,"reference":…,"state":"posted"}`, mode 0600, changed under the lock of the
outbox file and replaced whole by a file written beside it and renamed over it, as the other files
beside the outbox file (`Outbox::record_chat_post` in `crates/inbox-cli/src/outbox.rs`, through
`replace`): whatever is at the name of the staged file `<record>.new` is removed and the file is
created anew with mode 0600, so a link planted there is not followed. A line of the same message
id is replaced; every other line is kept as it is. A file that is no regular file, holds more than
64 MiB or cannot be read fails the change (exit 4) instead of being read as holding nothing: its
lines are never given up for a new one.

No line is removed in this story; how long a line is kept is settled by the stories that read the
record. At the bound of 64 MiB (about 700 thousand posts) the record cannot be read before a
change: every post then ends the bridge with 4 after the post was made, and no mark is sent. A
restart posts the same message again, which `connectors` answers as a replay by its key, and ends
again, until the operator removes or shortens the record.

## Refusals at the start

The bridge refuses to start with one line on standard error and no request:

- no binding is configured: one of the 3 flags is missing (exit 2, `no binding is configured for
  <address>`), or a value is empty or holds white space or a control character (exit 2);
- `connectors` is not on `PATH` (exit 4);
- the URL of the service is not a loopback URL (exit 2, the client's own refusal).

It starts no listener (Open 8, answered A): it asks the provider through `connectors`, and later
stories ask for reactions and replies the same way.

## What stays open

- Approval. A released `connectors` refuses every write that has neither an approval proof nor a
  standing approval. By the operator's decision (D89, answer B) the bridge issues and passes no
  proof, ever: it runs no `connectors approvals` command and passes no `--approval-file`. The
  operator gives `connectors` a standing approval, a policy entry that admits the post operation
  into one fixed channel without a proof per write. That entry is built in the repository of
  `connectors` and not released yet; until it is, a released `connectors` refuses the bridge's
  posts, and the run on a real channel waits for that release. A post without a live grant is
  answered `not_attempted` with the code `approval_required`.
- Not verified: the operations of the provider's connector and its input beyond what `connectors`
  says (above), the failure codes of the grant against a released `connectors` (they are its
  design), the provider's escapes and the delimiter, and whether a repeated idempotency key makes
  a repeated post arrive once.
- A `connectors` that writes more than 256 KiB on a stream is read to its end all the same; only
  the bytes kept are bounded.
- A SIGKILL of the bridge, and a hang-up of its terminal, leave a running `connectors` child and
  its process group running; a process the child started that left the group outlives the bridge
  on every signal (above).
- The idempotency conflict. The text of a post is built from the message's own fields only, so
  two tries of one message give the same input document and the same key. A later version that
  changes the text of a post (its head line, its escapes, its bound) would meet
  `idempotency_conflict` for a message tried before the change and not yet made: such a message
  stays in `Sent`, tried at the pause of a failure, until the operator handles it.
- The grant ends and has a cap per hour: when it ended, posts wait in `Sent`
  (`approval_required`) until the operator renews it; over the cap they wait until the hour's
  cap frees (`rate_limited`).
- A `connectors` that never ends holds the bridge: no post has a time bound yet.
- The allowed chat user is compared with nothing yet; no reaction, reply or decision is read from
  the channel (the later stories of epic:human-inbox).

## Cases

`crates/inbox-cli/tests/bridge.rs`, each against a real `inbox serve` on a loopback address and a
stand-in `connectors`, a POSIX shell script the case writes and puts first on `PATH`: it records
the arguments, the environment and the input document of each invocation and answers from files
the case gives it, holding an answer until the case releases it. No case reaches a chat provider,
a network beyond loopback or a model, and none holds an upper bound of time. The adversary's cases
are in `crates/inbox-cli/tests/adversary_bridge.rs`, in the same form. The unit cases of the
text are in `crates/inbox-cli/src/output.rs`, those of the answer in
`crates/inbox-cli/src/bridge.rs`.
