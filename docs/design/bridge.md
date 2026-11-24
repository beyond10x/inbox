# Design: the chat bridge

`inbox bridge` serves the inbox of a person through a chat channel
(story:the-chat-bridge-posts-a-pending-message, the first story of epic:human-inbox). It is a
long-running client of the service: it holds the caller's own `Pending` open as
`inbox wake --follow` does, and for each message in `Sent` that is not due it makes one chat post
through `connectors`, records it, then sends `MarkDelivered`. The message moves from `Sent` to
`Delivered` with no typed command. At an interval it reads the recent posts of the channel
through `connectors`, and for each of its posts the allowed chat user reacted to it sends
`MarkRead` (story:the-chat-bridge-marks-a-message-read-on-a-reaction): a request or a decision
moves from `Delivered` to `Read`, a note is handled and removed. The bridge holds no token of the
chat provider and starts no listener: it reaches the provider only by starting `connectors`,
which holds the connection. The code is `crates/inbox-cli/src/bridge.rs`.

## The command

```
inbox bridge --chat-channel <id> --allowed-chat-user <id> --chat-connector <connector[/instance]>
             [--chat-poll-interval <seconds>]
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

Drafted here, outside `ess/`; no file under `ess/` is changed by the stories of the bridge so
far.

- `ChatChannelBinding`: the address of the inbox of the person, the chat channel the messages are
  posted into (an id of the provider), the allowed chat user (an id of the provider), and the
  target `connectors invoke` is called with (a connector, or a connector and its instance). It is
  given by the 3 flags of the command; the address is the caller's own inbox.
- `ChatPost`: one inbox message id, the chat provider's reference of the post that shows it (the
  `ts` of the answer), and the state of the bridge's handling of it.
- `ChatReaction`: the chat user id that reacted, the reaction (its name, such as `eyes`), and the
  chat post it is on (the `ts` of that post). The bridge keeps none: it reads them from the
  channel at each poll (below) and acts on them at once.

Relations:

- `ChatChannelBinding` to Inbox: one binding per bridge process, and one bridge per inbox and
  outbox file on a host (the lock of the wake). Whether one chat channel serves several inboxes is
  not restricted: two bridges of two inboxes may name the same channel. (Open 2 of the story,
  answered A.)
- `ChatChannelBinding` to the allowed chat user: exactly one chat user id per binding, given by
  the operator with the binding. It must not be empty and holds no white space and no control
  character. It is compared, as text and in whole, with the chat user ids the provider's answer
  lists for a reaction, never with a display name. (Open 3 of
  story:the-chat-bridge-posts-a-pending-message, answered A.)
- `ChatPost` to Message: UNMAPPED: whether a note or a request has at most one chat post, as a
  decision has by "asked once" (epic:human-inbox), no source says. Open 1 of
  story:the-chat-bridge-asks-a-decision-once answers it. Today the record holds one line per
  message id: a second post of a message replaces its line.
- `ChatReaction` to `ChatPost`: many reactions may be on one chat post (the provider lists them per
  post, each with the ids of the users who made it; not verified against the provider). Any
  reaction of the allowed chat user on the post counts, whatever its name (Open 1 of
  story:the-chat-bridge-marks-a-message-read-on-a-reaction, answered A).
- `ChatReaction` to the allowed chat user: a reaction counts when one of the user ids the
  provider lists for it is the allowed chat user's id. A reaction of every other chat user changes
  nothing: no request to the service, no chat post, no line (Open 2, answered A).

The states of a `ChatPost` and their transitions:

- `posted`: the post was made and recorded (story:the-chat-bridge-posts-a-pending-message). The
  only state the bridge asks the channel about.
- `posted` to `read`: a reaction of the allowed chat user on the post was read, and the service
  answered the `read` request with success (outcome `handled` or `read`), `already-read`, or
  `unknown-message` (the message was read meanwhile, or is gone).
- `posted` to `read`, settled: the person's `Pending`, read by a poll after it read the entry,
  does not list the message, so it left `Pending` another way (read with `inbox read`, answered
  or refused, removed or decided at its deadline, or lost by a new start of the service). No
  request and no line. So `read` means: the bridge has nothing left to mark of this post, the
  message was read through a reaction or it left `Pending` without one. One state and not a
  fourth: the later stories ask the service for the state of a message (a thread reply to a
  request in `Read` is an answer however the request was read), and none reads from the record
  how the message left `Pending`.
- `posted` to `due`: the service answered the `read` request `deadline-passed` (`Expired`): the
  message's `expires` has passed, and the expiry task decides it.
- An entry that is `read` or `due` is never asked about and never marked again. A message posted
  again (after a restart of the service that lost it, as a reposted envelope) gets its line
  replaced by a new `posted` line with the new reference.

The later stories add answered (story:the-chat-bridge-turns-a-thread-reply-into-an-answer) and
decided on silence (story:the-chat-bridge-asks-a-decision-once); the ESS document of the bridge
is written then (Open 6 of story:the-chat-bridge-posts-a-pending-message, answered A with iii:
the draft stays here).

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
(`invoke`), whose callers are `post` and the read of the channel (below, "The read mark"):

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
  story:the-chat-bridge-asks-a-decision-once; the posts do not read the record.

## The read mark

story:the-chat-bridge-marks-a-message-read-on-a-reaction. The person reads a message in the
channel by reacting to its post, as the person reads one with `inbox read` in a terminal.

- The poll. Once the inbox is open (the start of the rounds), a thread of the bridge
  (`inbox-bridge-poll`) polls: at once, then every `--chat-poll-interval` seconds (default 10,
  at least 1) while the rounds of `Pending` run (Open 3, answered A). It has a client of its own
  (the same service, key and outbox file); the rounds and the poll share no state but the files
  beside the outbox file and the turn (below).
- What one poll asks for (Open 4, answered B: one read of the recent posts of the channel per
  poll, so the invocations per poll do not grow with the open posts). The poll reads the record
  of the chat posts and keeps the entries `posted` of the bound channel (`--chat-channel`): an
  entry of another channel, written by a bridge of the same inbox started with another channel,
  is never asked about, settled or marked by this one; an entry without a channel (a line of a
  build before the channel was recorded) counts as of the bound channel. With no such entry it
  starts no child and asks nothing. A record it cannot read (no regular file, over its bound, not
  readable) is a failed poll whose line names the record (below), and is tried again after the
  pause of a failure; a post that has to change it still ends the bridge with 4 and its own line.
  Otherwise it reads the person's `Pending`
  whole and at once (`commands::pending_now`: no `wait` and no `undelivered`, so every row of a
  message in `Sent` or `Delivered`, due ones included; the view has no page and no bound of
  rows), and settles each `posted` entry it read whose message id no row holds: the entry
  becomes `read` through the `replace` of the files beside the outbox file under the outbox lock
  (`Outbox::mark_chat_post`, only while the line is still the one the poll read), with no
  request and no line. The record is read before `Pending`: an entry the poll read was written
  after its post, while its message was in `Sent`, so a message not listed by the later read of
  `Pending` has left it; an entry a round writes after the poll read the record waits for the
  next poll. A read of `Pending` that fails, or whose answer is no list `rows` with a text
  `message_id` in each row, is a failed poll: it settles nothing and reads no channel. The
  rounds of `Pending` do not settle: they read it with `undelivered=1`, which lists the rows in
  `Sent` only, and hand a round over only when it has a row, so their rows never list a message
  in `Delivered`. With no entry left `posted` after the settle the poll starts no child. Else it
  reads the channel from the oldest `posted` entry on, that entry's post included: `oldest` is the
  least reference of them in the order of the provider's `ts` (whole seconds, then the fraction),
  sent with `inclusive` `true`, since the provider lists only the posts after `oldest` without it
  and the oldest open post would never be seen; a reference that is not `<digits>.<digits>` is
  never sent as `oldest`, and with none of that form the read has no `oldest` and no
  `inclusive`.
- The read, through the same `invoke` as a post (the turn, the slot `wake::child`, the group of
  its own, the bounds on the streams, so SIGINT and SIGTERM end a running read as they end a
  running post; the thread whose child the signal path took writes no line and waits for the end
  of the process, so a bridge ended during a read names no failed read):

  ```
  connectors --output json invoke <target> conversations.history --input-stdin
  ```

  with the input `{"channel":"<channel id>","oldest":"<ts>","inclusive":true,"limit":100}` on
  standard input: flat, the provider's parameters as top-level members, `oldest` a string (a JSON
  number would lose its trailing zeros). No idempotency key and no approval: a read changes
  nothing.
- Paging. The next page is the same input with `cursor`, the `response_metadata.next_cursor` of
  the page before; an empty text or no `response_metadata` (or no `next_cursor` in it) ends the
  pages, `has_more` is not read. A poll reads at most 10 pages (`MOST_PAGES`, 1000 posts): a
  channel that has a further page then is a failed poll with its own line, and nothing is marked.
  Since an entry whose message left `Pending` is settled, the bound is met only while a message
  posted more than 1000 posts back in the channel is still in `Pending`; the polls fail until it
  leaves `Pending`.
- The turn: one `connectors` child at a time, a poll and a post never together. `invoke` takes a
  lock in the memory of the process (`TURN`, a `Mutex`) before it starts the child and holds it
  until the child has ended and its streams are read: a post waits for a poll that runs, a poll for
  a post that runs. A read of several pages takes the turn once per page, so a post may run
  between 2 pages. No file: the turn is of one process, and 2 bridges of one inbox are already
  kept apart by the lock of the wake. The cases see a thread waiting for it as a thread whose
  system call is `futex` (Linux `/proc/<pid>/task/<tid>/syscall`).
- The end by a signal. The signal path takes the slot's guard, takes the child from the slot and
  holds the guard from then until the process ends: no thread starts a child after it, and the
  thread whose child it took blocks on the guard, when it next looks whether its child has
  exited, until the process ends, so it writes no line. No thread finds the slot empty: only
  `invoke` fills and empties it, one at a time under the turn, and the signal path never gives
  the guard back. `invoke` still answers a slot found empty with a failed post or read and its
  line, a defensive check that no path reaches. That the guard is held until the end has no
  case: the window between the end of the child's group and the exit of the process is too short
  to be hit by a case without a race on the scheduler.
- The answer is untrusted input, judged as a post's is: standard output on exit 0, standard error
  on exit 1, each bounded to 256 KiB (an answer over it is a failed poll whose line says so). A
  page is read only when the process exited 0 with one JSON document whose
  `result.result.status` is 200, `result.result.body.ok` `true`, and `result.result.body.messages`
  a list; each post of it an object with a `ts` text that is not empty, and its `reactions`
  absent or a list. A member missing or of another type fails the whole poll: nothing of it is
  marked, also a post before the member that fails it. A reaction is one of the allowed chat user
  when it has a list `users` that holds the allowed chat user's id as a text; a reaction that is
  no object, has no such list, or lists the id as no text counts as no reaction of the allowed
  chat user.
- A refused read (exit 1): standard output empty, one document on standard error
  `{"ok":false,"error":{"code":…,"data":{…}}}`, `error.data.code` `service_failure`,
  `error.data.stage` `dispatch`, the class in `error.data.service_code` (`forbidden`,
  `not_found`, `unauthorized`, `rate_limited`, `invalid_input`, `internal`,
  `upstream_protocol`). This differs from a refused post, where the code of the operator's grant
  is `error.data.code` itself. The line names the code and the class and repeats no reason of the
  provider. After `rate_limited` the next poll is not before `error.data.retry_after_seconds`
  when it is a whole number from 1 to 3600, as after a post. An input `connectors` refuses before
  any request exits 2, a usage error.
- The match: each `posted` entry whose reference is the `ts` of a post the allowed chat user
  reacted to gets one `read` request, the one `inbox read` sends (`commands::read`): `handled`,
  `read` and `already-read` and the refusal `unknown-message` make the entry `read`, with no
  line; the refusal `deadline-passed` (`Expired`) makes it `due` and writes one line that names
  the message id, the channel and `Expired`, only after the record says `due`. That line is
  written at most once: a bridge ended between the record change and the line never writes it,
  and the entry is `due` and not asked about again. Each entry changes
  through the `replace` of the files beside the outbox file, under the outbox lock, right after
  its answer and before the next poll; the line of the entry changes only while it is still the
  `posted` line the poll read (a post of the same message meanwhile keeps its new line). Every
  other failure of the request (no answer, another refusal) leaves the entry `posted` and fails
  the poll; the entries marked before it stay marked.
- The read is sent at least once per reacted post, not exactly once: a `read` whose answer is lost,
  or whose answer came but whose change of the record failed, leaves the entry `posted`, and the
  next poll sends the same `read` again. The service answers a second `read` of a message that is
  already read with `already-read` (or the message has left `Pending` and the entry is settled),
  so a repeat changes nothing at the service.
- A failed poll (a record of the chat posts that cannot be read, a read of `Pending` that failed
  or answered no view, a child that did not answer a page, a refusal, an answer that is not a
  page, the bound of pages, a change of the record that failed, a `read` request that failed)
  marks
  nothing more and is tried again after the pause of a failure (`wake::pause`: 100 ms, doubled
  per failed poll in a row, up to 30 s), not after the interval, and after a named
  `retry_after_seconds` first. One line on standard error names the first failed poll of a run of
  them (`inbox bridge: the read of the reactions in the chat channel <id> failed: …`); the next
  line comes after a poll succeeded and another failed. A failed poll does not stop the rounds of
  `Pending`: a message posted meanwhile is posted and marked delivered.
- The poll never ends the bridge: only a signal or a failure of the rounds does.

Not verified against the provider or a released `connectors` (from the contract and the tests of
`connectors` 0.76.1 as its session states them; no read was run): the name and the input of the
read operation; that `connectors` hands the provider's `reactions` member through (inferred by
the session of `connectors`: its selection has no field list, and no test of it has the member);
the shape of a page and of the refusal; the classes; that posts come newest first; the provider's
rate limits and the cost of one `connectors invoke`.

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
`{"message_id":…,"channel":…,"reference":…,"state":…}` with the chat channel the post was made in
(a line without `channel`, of a build before it was recorded, is read as of the bound channel) and
the state `posted`, `read` or `due` (a post writes it `posted` with `Outbox::record_chat_post`; the poll changes its state with
`Outbox::mark_chat_post`, and reads the record with `Outbox::chat_posts`, which skips a line of
another form), mode 0600, changed under the lock of the
outbox file and replaced whole by a file written beside it and renamed over it, as the other files
beside the outbox file (`Outbox::record_chat_post` in `crates/inbox-cli/src/outbox.rs`, through
`replace`): whatever is at the name of the staged file `<record>.new` is removed and the file is
created anew with mode 0600, so a link planted there is not followed. A line of the same message
id is replaced; every other line is kept as it is. A file that is no regular file, holds more than
64 MiB or cannot be read fails the change (exit 4) instead of being read as holding nothing: its
lines are never given up for a new one.

No line is removed. The read mark needs no pruning to hold its rule that an entry `read` or
`due` is never asked about again: the poll starts from the `posted` entries, and a `ts` the
provider lists that no `posted` entry holds is not looked at, removed or not. It removes none
all the same: story:the-chat-bridge-turns-a-thread-reply-into-an-answer finds the post of a
request in `Read` in the record to read its thread, and when an entry may leave the record is
Open 1 of story:the-chat-bridge-asks-a-decision-once. At the bound of 64 MiB (about 700 thousand posts) the record cannot be read before a
change: every post then ends the bridge with 4 after the post was made, and no mark is sent. A
restart posts the same message again, which `connectors` answers as a replay by its key, and ends
again, until the operator removes or shortens the record.

## Refusals at the start

The bridge refuses to start with one line on standard error and no request:

- no binding is configured: one of the 3 flags is missing (exit 2, `no binding is configured for
  <address>`), or a value is empty or holds white space or a control character (exit 2);
- `connectors` is not on `PATH` (exit 4);
- the URL of the service is not a loopback URL (exit 2, the client's own refusal);
- `--chat-poll-interval` is not a whole number of at least 1 (exit 2, a usage error of the
  command line).

It starts no listener (Open 8, answered A): it asks the provider through `connectors` for the
reactions, and a later story asks for the replies the same way.

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
- A `connectors` that never ends holds the bridge: no post and no read has a time bound yet, and
  while one holds the turn the other waits.
- A message that stays in `Pending` (`Delivered`, not read) while more than 10 pages of 100 posts
  are made in the channel after its post keeps every poll failing at the bound of pages, and no
  reaction marks a message read, until it leaves `Pending` (read with `inbox read`, answered, or
  removed or decided at its deadline).
- A message lost by a new start of the service is settled `read`; its repost is posted again
  with a new line.
- A reaction removed after the poll read it does not make the message unread; the provider's
  answer names no time of a reaction, so a reaction made before the post was recorded counts.
- No reply or decision is read from the channel (the later stories of epic:human-inbox).

## Cases

`crates/inbox-cli/tests/bridge.rs`, each against a real `inbox serve` on a loopback address and a
stand-in `connectors`, a POSIX shell script the case writes and puts first on `PATH`: it records
the arguments, the environment and the input document of each invocation and answers from files
the case gives it, holding an answer until the case releases it. No case reaches a chat provider,
a network beyond loopback or a model, and none holds an upper bound of time. The adversary's cases
are in `crates/inbox-cli/tests/adversary_bridge.rs`, in the same form. The cases of the read
mark are in `crates/inbox-cli/tests/bridge_read.rs`: a stand-in that answers each read with the
posts and reactions the case scripts for it and each post with a reference of its own, a forwarder
in front of the service that keeps what each connection carried (the `read` requests are counted
there, a read of the whole `Pending` is failed or held there, and a `read` request is dropped
without an answer there, as a case says), and the interval of the poll set to 1 s. The stand-in
lists the posts of a read as the provider does: those after `oldest`, and the one at `oldest` only
with `inclusive` `true`. The cases wait for the stand-in's record of an invocation or for a fact,
a thread waiting for the turn through Linux `/proc/<pid>/task/<tid>/syscall`. The cases of
`bridge.rs` and
`adversary_bridge.rs` poll every 3600 s, so their stand-ins, which know the post only, see no
read. The unit cases of the text are in `crates/inbox-cli/src/output.rs`, those of the answers in
`crates/inbox-cli/src/bridge.rs`.
