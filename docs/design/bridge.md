# Design: the chat bridge

`inbox bridge` serves the inbox of a person through a chat channel
(story:the-chat-bridge-posts-a-pending-message, the first story of epic:human-inbox). It is a
long-running client of the service: it holds the `Pending` of the inbox it acts for open as
`inbox wake --follow` does, and for each message in `Sent` that is not due it makes one chat post
through `connectors`, records it, then sends `MarkDelivered`. The message moves from `Sent` to
`Delivered` with no typed command. At an interval it reads the recent posts of the channel
through `connectors`, and for each of its posts the allowed chat user reacted to it sends
`MarkRead` (story:the-chat-bridge-marks-a-message-read-on-a-reaction): a request or a decision
moves from `Delivered` to `Read`, a note is handled and removed. A message is posted once per
message id (story:the-chat-bridge-asks-a-decision-once): a message that comes back in `Sent`
after a restart of the service, or that a new bridge finds in `Sent`, is marked and not posted
again. When nobody answers, the service answers a decision by silence at `expires`, and the
bridge answers nothing in the person's place; it writes one line into the thread of the chat post
that says how the message ended. The bridge holds no token of the chat provider and starts no
listener: it reaches the provider only by starting `connectors`, which holds the connection. The
code is `crates/inbox-cli/src/bridge.rs`.

## The command

```
inbox bridge --chat-channel <id> --allowed-chat-user <id> --chat-connector <connector[/instance]>
             [--chat-poll-interval <seconds>] [--chat-post-retention <seconds>]
             [--for <address> --authority <delegation id>]
```

with the client's `INBOX_URL`, `INBOX_SECRET_KEY_FILE` and outbox file, as every client command.
It signs with the key file it is given: a key of the person, whose entry names the address of the
person as its principal (`human:<name>`), or a key of its own under a delegation of the person
("The key of the bridge", below).

## The key of the bridge

story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, under the operator's answer B
(approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner). The bridge runs with a key of
its own (principal `agent:<host>/chat-bridge`, say) and the delegation the person gave it, with the
2 flags (Open 1 A, Open 3 A):

- `--for <address>`, the client flag of every client command (`docs/design/service.md`, "The
  request signature"): every request of the bridge carries `Inbox-On-Behalf-Of` with the address,
  signed: both reads of each round, `MarkDelivered`, the `read` of a reaction, the reads of
  `Pending` and `Thread` of the poll, and the `OpenInbox` of its start and of the reconnect.
- `--authority <delegation id>`: the id of the delegation in the service's key file. The client
  reads no key file of the service and does not check the id; the bridge names it in the line of
  a refused key (below).

The two go together: `--for` without `--authority`, and `--authority` without `--for`, are wrong
usage (exit 2, one line, nothing sent). Without both the bridge is the one of before this story,
byte for byte.

The inbox the bridge acts for is the address of `--for`, else the caller's own address: one notion
in the client (`Client::own_inbox` in `crates/inbox-cli/src/commands.rs`). The 3 files the bridge
keeps beside the outbox file are keyed by it, not by the key's principal: the lock of the wake
(`<outbox>.wake.<16 hex digits>`, `docs/design/wake.md`, "One wake per inbox on a host"), the line
of the inbox in `<outbox>.opened` (written by the bridge's open, read by the check of the adapter
and by the reconnect), and the record of the chat posts (`<outbox>.chat.<16 hex digits>`). So a
bridge that ran with a key of the person and is started again with its own key for the same
person, with the same outbox file, holds the same lock, reads the same line and the same record,
and posts no message the record holds. The kept envelopes of the outbox file stay keyed by their
sender, the key's principal; the bridge posts none.

The inbox is opened under the header: the model sets its `owner` to the caller's principal, the
bridge's (`OpenInbox`, outcome `opened`); that is Open 4 A of
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal. An inbox the person opened
before is answered `already-open` and keeps its owner.

A refused key. The service reads its key file at its start and answers every request of a key
entry that is missing or lapsed, and every request under a delegation that is missing, lapsed or
of another pair, with the same `401 {"refused":"unauthenticated"}`; the bridge's secret key file
holds no `valid_until`. So the bridge cannot tell the causes apart, and a bridge started with
`--for` that gets a `401` to any request ends at once (Open 4 A): it sends no further request,
ends a running `connectors` child with its group as a signal does, and exits 1 with one line, the
same for every cause:

```
inbox bridge: the service refused the key <key id> acting for <address> under the delegation <delegation id> (401): its key file holds no live key entry <key id> or no live delegation <delegation id> of <principal> for <address>; add the missing one with `inbox keyfile add-key` or `inbox keyfile add-delegation` and start the service again: it reads the key file at its start only
```

The ids in it are not checked on the client: the delegation id comes as `--authority` was typed,
the key id and the principal as the secret key file holds them. Each is written with its line
ends as `\u{a}`, and the line, as every line the client writes, with each other control character,
each character that changes the direction of text and each one a reader cannot see as its escape
(`crates/inbox-cli/src/output.rs`), so it stays one line, shown in the order it was written,
whatever the ids hold.

It posts nothing after it. The client of the rounds and the client of the poll both end the
process on it, and a thread that starts a request after it waits for the end and sends nothing;
a request another thread of the bridge started at the same instant is not held back. A bridge without `--for` does on a `401` what it did before: a `401` to its open or
to a read of `Pending` ends it with 1 and the line `inbox bridge: refused: unauthenticated (401)`;
a `401` to a `MarkDelivered` is named in one line and the round is tried again after the pause of a
failure; a `401` in the poll fails the poll, named once per run of failures.

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
  given by the 3 flags of the command; the address is the inbox the bridge acts for (`--for`,
  else the caller's own).
- `ChatPost`: one inbox message id, the chat provider's reference of the post that shows it (the
  `ts` of the answer), the channel it was posted in, the thread and the `expires` of the message
  as `Pending` listed them when it was posted, and the state of the bridge's handling of it.
- `ChatReaction`: the chat user id that reacted, the reaction (its name, such as `eyes`), and the
  chat post it is on (the `ts` of that post). The bridge keeps none: it reads them from the
  channel at each poll (below) and acts on them at once.
- `ChatReply` (story:the-chat-bridge-turns-a-thread-reply-into-an-answer): the chat user id that
  wrote it, its text, its reference (its `ts`) and the chat post it replies to (the `ts` of the
  thread it is in). The bridge reads them from the thread of a post at each poll ("The answer",
  below) and keeps of them only the reference of the last one it handled and that of the last
  message of the thread it read, whoever wrote it; of its own lines in the thread it keeps the
  references the post operation answered until a read has passed them.

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
- `ChatPost` to Message: at most one chat post per message id, for every kind, while the record
  holds the id (Open 1 of story:the-chat-bridge-asks-a-decision-once, answered A). A message in
  `Sent` whose id the record holds is not posted again; the entry keeps its reference and its
  channel. An entry of another channel than the bound one counts as held too: once per message
  id, not once per channel. An entry leaves the record when its message is gone from the service
  and the clock of the bridge has passed its `expires` plus the retention
  (`--chat-post-retention`, default 86400 s, the default answer retention of the service); a
  message whose id came back after that is posted again.
- `ChatReaction` to `ChatPost`: many reactions may be on one chat post (the provider lists them per
  post, each with the ids of the users who made it; not verified against the provider). Any
  reaction of the allowed chat user on the post counts, whatever its name (Open 1 of
  story:the-chat-bridge-marks-a-message-read-on-a-reaction, answered A).
- `ChatReaction` to the allowed chat user: a reaction counts when one of the user ids the
  provider lists for it is the allowed chat user's id. A reaction of every other chat user changes
  nothing: no request to the service, no chat post, no line (Open 2, answered A).
- `ChatReply` to `ChatPost`: UNMAPPED: no ESS document declares either, and a post may have many
  thread replies in the provider (not verified). Which of them becomes the answer is Open 4 of
  story:the-chat-bridge-turns-a-thread-reply-into-an-answer, answered A: the first reply of the
  allowed chat user that the bridge sends as `Answer` and the service takes; every later reply of
  the allowed chat user gets the line of `already-settled` and sends nothing.
- `ChatReply` to the allowed chat user: a reply counts when its `user` is the allowed chat user's
  id, compared as text and in whole, as a reaction's. A reply of every other chat user changes
  nothing: no request, no line (check 6, as Open 2 of the reaction); the one mark it leaves is in
  the bridge's own record: the next read starts after it ("The answer"). A message of the thread
  whose reference is one the post operation answered for a line of the bridge is no reply,
  whatever user the provider names on it.
- `ChatReply` to Message: at most one `Answer` per message from the bridge; the reply envelope is
  the model's (`inbox.messaging.Answer`), one per message (`reply` of `Message`).

The states of a `ChatPost` and their transitions. An entry has two parts: its read mark (the
member `state`: `posted`, `read`, `due` or `answered`), what the bridge knows of the person's
reading and answering; and its
end (the member `end`: none, `answered`, `decided`, `expired` or `gone`), how its message ended as
the service shows it. An entry whose end is set is at its end; the read mark stays as it was, so a
repost of a message the person read is marked read again.

- `posted`: the post was made and recorded (story:the-chat-bridge-posts-a-pending-message). While
  its message is in the person's `Pending` the bridge asks the channel about it (its reactions)
  and asks the service only through the read of `Pending`.
- `posted` to `read`: a reaction of the allowed chat user on the post was read, and the service
  answered the `read` request with success (outcome `handled` or `read`), `already-read`, or
  `unknown-message` (the message was read meanwhile, or is gone).
- `posted` (or `due`) to `read` through the service: the row of the message in `Thread` is in
  `Read` (read with `inbox read` in a terminal).
- `posted` to `due`: the service answered the `read` request `deadline-passed` (`Expired`): the
  message's `expires` has passed, and the expiry task decides it.
- An entry that is `read` or `due` is never asked about in the channel and never marked again. A
  message that left `Pending` no longer moves the read mark by itself: from this story on an entry
  is `read` only when the bridge knows it (story:the-chat-bridge-marks-a-message-read-on-a-reaction
  made every entry whose message left `Pending` `read`, one state for every way out; a message lost
  by a new start of the service was never read by the person, so that no longer holds).
- Not at its end (no `end`), whatever its read mark, while its message is not in `Pending`: at
  every poll the bridge asks the service once for the row of the message (`Thread`), and moves the
  entry by what it shows:
  - a row in `Sent` or `Delivered` (posted again meanwhile): nothing;
  - a row in `Read`: the read mark becomes `read`, not at its end (a request or a decision in
    `Read` may still be answered, decided on silence or expire);
  - a row in `Answered` with `answered_by` `principal`, or in `Refused`: end `answered` (the person
    answered or refused it, in a terminal or by a later story's reply). Nothing is written.
  - a row in `Answered` with `answered_by` `silence`: end `decided`, after the bridge wrote one
    line into the thread of the post that names the option, its text, and that it was taken on
    silence (below, "The end of a message");
  - no row, and the clock of the bridge at or past the entry's `expires`: end `expired`, after the
    bridge wrote the line "expired, not answered" into the thread of the post;
  - no row, and the clock of the bridge before the entry's `expires`: end `gone` (read and
    handled, collected, or lost by a new start of the service). Nothing is written, and nothing is
    written later for it either;
  - no row, and the clock of the bridge at or past the entry's `expires` plus the retention: the
    entry is dropped with no line (a bridge that was not running for longer than the retention);
  - no row, and the entry names no `expires` (a line of a build before it was recorded): no end
    (the bridge cannot tell an expired message from one that is gone); the first such poll
    records the reading of the bridge's clock as `gone_at`, and the first poll at or past
    `gone_at` plus the retention that still finds no row drops the entry. It never gets the line
    of an expired message. Until it is dropped it costs one read of `Thread` per poll.
- Read and not at its end, while `Pending` lists its message in `Delivered` (a repost whose
  `read` was lost): at every poll the bridge sends the `read` of `inbox read` again, at least
  once, until the service answers it.
- At its end (`answered`, `decided`, `expired`, `gone`): never asked about in the channel, never
  marked, and no line is written for it again. Its retention starts at its `expires`, or, for an
  entry that names none, at its `gone_at`. The service is asked about it only once the clock of
  the bridge is at or past that start plus the retention: no row, and the entry is dropped; a row
  (an answer taken on silence the sender has not collected yet), and it stays until a later poll
  finds none. An entry at its end that names neither (a line of a build before, answered or
  decided on silence) is asked about at every poll until the service holds no row of it, and then
  records `gone_at`.
- A repost (a row in `Sent` whose id the record holds, for any read mark and end, of any
  channel): the bridge sends `MarkDelivered`, takes an end `gone` or `answered` and a `gone_at`
  off the entry, since its message is live again, and then, when the read mark is `read`,
  sends `MarkRead`; it makes no post, and the entry keeps its reference and its channel. The end
  is taken off whether or not the `MarkRead` succeeds. `decided` and `expired` stay, so their
  line is never written twice (a message decided or expired cannot come back: its `expires` has
  passed, and a post past its `expires` is refused).
- A message the person refused (`inbox refuse`, a row in `Refused`) has the end `answered`, as an
  answered one: nothing is written for it.
- `posted`, `read` or `due` to `answered` (the state answered, story:the-chat-bridge-turns-a-thread-reply-into-an-answer):
  the service took the `Answer` the bridge sent for a thread reply (outcome `answered`). Not at its
  end: while the service still holds the message (its row in `Thread` is in `Answered`), the poll
  reads the replies of the post and gives every later reply of the allowed chat user the line of
  `already-settled`. Its row gone (the sender collected the answer), the end `answered` and no
  line, also past its `expires`. A repost of it (the service lost the answer in a new start)
  makes it `read` again, so a later reply is sent as an answer again.

The ESS document of the bridge is written with the later stories (Open 6 of
story:the-chat-bridge-posts-a-pending-message, answered A with iii: the draft stays here).

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
  `inbox open` of the inbox it acts for (`<outbox>.opened`) at the start, before the lock and
  any request, and in every round before its posts; when the record names another adapter it ends with 1 and
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
- The shape of the answers, as the contract of `connectors` 0.77.0 states it (released
  2026-11-24): the shape of a write that ran, and the failure codes of the operator's grant
  (`approval_required`, `rate_limited`, `idempotency_conflict`, `unavailable`). Not verified: no
  call of `connectors` was run, by the bridge's cases or by hand.
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

## A repost

story:the-chat-bridge-asks-a-decision-once (Open 1, answered A). Each round reads the record of
the chat posts once before its posts. A row in `Sent` whose message id the record holds, in any
channel and with any read mark or end, is not posted: the bridge sends `MarkDelivered` and, when
the entry's read mark is `read`, the `read` of `inbox read` after it (a request or a decision goes
back to `Read`, a note is handled and removed). The entry keeps its reference and its channel; an
end `gone` or `answered` (and a `gone_at`) is taken off it after the `MarkDelivered` and before
the `read`, since its message is live again. Such a row comes after
a restart of the service, when the sender's next command posts the message again under its id
(`docs/design/service.md`, "The epoch"), and after a restart of the bridge, for a message the
bridge before it posted and did not mark. An entry of another channel than the bound one counts
as held too: a message is posted once per message id, not once per channel, and its marks are
sent by whichever bridge of the inbox runs.

- A `MarkDelivered` that fails is named in one line and the next round starts after the pause of
  a failure; it comes again with the row. A `read` that fails after its `MarkDelivered` succeeded
  is named in one line and ends nothing; the poll sends it again (at least once, until the service
  answers it) while the entry is `read`, not at its end, and `Pending` lists its message in
  `Delivered` ("The read mark"), so the `read` may reach the service more than once. A `read`
  refused by the model (the message read, gone or due meanwhile) is no failure.
- No post without a readable record (fail closed, as the record's own module says): a round that
  cannot read the record (no permission, not UTF-8, over its bound of 64 MiB, no regular file, a
  line that is no entry: "The record of the chat posts") posts and marks nothing and ends the
  bridge with 4 and one line that names the first message of the round and the record, as a post
  whose record cannot be written did in the story before
  (story:the-chat-bridge-posts-a-pending-message: a directory in the place of the record ends the
  bridge with 4, the message in `Sent`; its case now holds that no post is made). A missing
  record holds no entry. The poll stays as it was: a record it cannot read fails the poll, which
  is tried again.
- An entry of a build before this story (no `thread`, no `expires`) is held as every other: its
  repost is not posted. Found gone, it records `gone_at` and is kept for the retention counted
  from there, then dropped; a repost after that is posted again.
- Not exactly once: a poll that reads the row of a message in `Thread` just before its repost
  arrives and changes its entry to `gone` just after the round read the record leaves the entry
  `gone` while its message is live; no line is written at its end then.

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
  of the chat posts and keeps the entries of the bound channel (`--chat-channel`) that are not at
  their end, those at their end whose retention has passed and those at their end without a start
  of their retention (below, "The end of a message"): an entry of another channel, written by a bridge of the same inbox started
  with another channel, is never asked about, settled, marked or dropped by this one; an entry
  without a channel (a line of a build before the channel was recorded) counts as of the bound
  channel. With no such entry it starts no child and asks nothing. A record it cannot read (no regular file, over its bound, not
  readable, a line that is no entry) is a failed poll whose line names the record (below), and is
  tried again after the pause of a failure; a post that has to change it still ends the bridge
  with 4 and its own line. Otherwise it reads the person's `Pending`
  whole and at once (`commands::pending_now`: no `wait` and no `undelivered`, so every row of a
  message in `Sent` or `Delivered`, due ones included; the view has no page and no bound of
  rows). First, for each entry `read` not at its end whose message a row holds in `Delivered` (a
  repost whose `read` was lost, "A repost"), it sends the `read` of `inbox read` again; a `read`
  that fails fails the poll as below, and the next poll sends it again. It asks the channel only
  about the entries `posted` whose message a row holds; each
  entry not at its end whose message id no row holds has left `Pending`, and the poll asks the
  service how it ended (below, "The end of a message") after the reactions, instead of settling
  it `read` as story:the-chat-bridge-marks-a-message-read-on-a-reaction did. The record is read
  before `Pending`: an entry the poll read was written
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
  line comes after a poll succeeded and another failed. A poll that failed on an `Answer` without
  an answer the bridge can act on is counted the same for the pause, but its line is the
  `Answer`'s own ("The answer"), and this line names the first other failure of the run. A failed poll does not stop the rounds of
  `Pending`: a message posted meanwhile is posted and marked delivered.
- The poll never ends the bridge: only a signal or a failure of the rounds does.

Not verified against the provider or `connectors` (from the contract and the tests of
`connectors` 0.76.1 as its session states them; no read was run, by the bridge's cases or by
hand): the name and the input of the
read operation; that `connectors` hands the provider's `reactions` member through (inferred by
the session of `connectors`: its selection has no field list, and no test of it has the member);
the shape of a page and of the refusal; the classes; that posts come newest first; the provider's
rate limits and the cost of one `connectors invoke`.

## The end of a message

story:the-chat-bridge-asks-a-decision-once (Open 2 and Open 3, answered A). The service answers a
decision that names `default_on_silence` by silence at `expires` (`Expire`, outcome
`decided-on-silence`) and removes every other due message; the bridge sends no `answer`, `read`
or `refuse` in the person's place at `expires`, or at any other time but the `read` of a reaction
and of a repost of a message the person read.

- Where the bridge looks. At each poll, after the read of `Pending` and the reactions, for each
  entry of the bound channel that is not at its end and whose message `Pending` does not list,
  one read of the `Thread` view of its thread (`commands::thread_now`, the view `inbox thread`
  reads; the read rule gives the person the rows whose `to` is the person's inbox), and the row of
  its message id. An entry of a build before the thread was recorded is read under its message
  id, the thread of a message sent to one inbox. What the row shows moves the entry as "The nouns
  of the bridge" says. A poll makes at most: 1 read of `Pending`; 1 read of `Thread` per entry not
  at its end whose message left `Pending`, per entry at its end whose retention has passed (until
  it is dropped), and per entry at its end without a start of its retention (an entry of a build
  before this story, answered or decided on silence, until its message is gone); 10 pages of the
  channel; 1 `read` per entry whose message `Pending` lists (a reacted post, or an entry `read`
  whose message is `Delivered`); 1 reply per message that reached its end. An entry at its end
  with a start of its retention costs nothing until its retention has passed, so the requests do
  not grow with those entries; with many entries at their end past their retention at once (a
  bridge that was stopped longer than the retention), one poll reads `Thread` once for each.
- A read of `Thread` that fails, or whose answer is no list `rows`, fails the poll as a read of
  `Pending` does (one line per run of failed polls, the pause of a failure); the entries changed
  before it stay changed.
- The line. One reply in the thread of the chat post:

  ```
  connectors --output json invoke <target> chat.postMessage --input-stdin --idempotency-key inbox-<message id>-end
  ```

  with the input `{"body":{"channel":"<channel id>","text":"<text>","thread_ts":"<reference of
  the chat post>"}}`: the input of a post and one member, `thread_ts` a string, exactly these 3
  members under `body`. From the session of `connectors`, its contract and its tests at the branch
  of 0.77.0, no call run: the body is closed, an empty `thread_ts` is refused, a standing grant
  admits `/body/text` and `/body/thread_ts` in the granted channel (one grant covers posts and
  replies), the answer has the shape of a post's and the refusals are the same classes. It runs
  through `invoke` like every `connectors` child (the turn, the slot of the child, the group of its
  own, the bounds on the streams) and is judged as a post is.
- The text. For a decision taken on silence: `taken on silence: option <letter>, <text>`, the
  option `decided_option` names and its text from `decision.options` of the row of `Thread` (the
  letter alone when the row names no such option). For a message that expired without an answer:
  `expired, not answered`. The letter and the option's text go through the escapes of the head
  line of a post (`output::chat_silence`). The text is built from the row and the record only, so
  every try of one message has the same input under the same key; the same key with another input
  is `idempotency_conflict`.
- The key `inbox-<message id>-end` differs from the key of the post (`inbox-<message id>`) and is
  the same at every try.
- The order: the reply, then the record change that says it was written (the end `decided` or
  `expired`). A bridge ended between the two, or a record change that fails (a failed poll), sends
  the reply again with the same key at the next poll: at least once at the bridge, and once at
  `connectors`, which answers a repeated key with the same input as a replay.
- A reply that fails (any failed run of `connectors`, a refusal of any class, a parent post that
  is gone included) is named in one line per run of failed replies and tried again at a later
  poll; it fails no poll, so it stops neither posts nor reads. The wait of a `rate_limited` reply
  (`retry_after_seconds`) is not kept: the next try is at the next poll. The entry is at its end
  only after a reply that went through; every try has the key and the input of the first.
- A control character in an option text reaches the reply as its escape (`\u{b}`), a line end as
  `\u{a}`, and `&`, `<` and `>` as the provider's escapes, so a mention or a link form in an
  option text is no mention or link in the reply (held by the case of checks 5 and 6).
- An entry of another channel than the bound one gets no reply from this bridge: the poll never
  looks at it. A bridge bound to that channel again writes it at its next poll. Not held by a
  case.
- A row that is gone gives the line of an expired message only when the clock of the bridge has
  passed the entry's `expires`; before, the entry is `gone` and gets no line, then or later. So a
  decision taken on silence whose answer the sender collected before a poll saw it gets the line
  of an expired message; the bridge polls every `--chat-poll-interval` seconds, and an answer
  taken on silence waits for its sender up to the answer retention of the service.
- Not verified: that the provider shows the reply in the thread of the post; the class of a
  failure only a reply has (a parent post that is gone, which the bridge treats as any failed
  post); a reply under a standing grant (no test of `connectors` sends one).

## The answer

story:the-chat-bridge-turns-a-thread-reply-into-an-answer. The person answers a request or a
decision by a reply in the thread of its chat post, as the person answers one with `inbox answer`
in a terminal.

- Which posts. At each poll, last (after the reactions, the ends and the retention), for each
  entry of the bound channel that is not at its end and whose message the service holds live: a
  row of `Pending` in `Delivered` (the poll's read of `Pending`), a row of `Thread` in `Read` (the
  poll's read of `Thread` of an entry whose message left `Pending`, "The end of a message"), or,
  for an entry in the state answered, a row of `Thread` in `Answered`. A row in `Sent` waits for
  its `MarkDelivered`. The kind, the options and the state are the row's. Notes are read too, so a
  reply to a note gets its line. The record is read again before this step, so an entry the
  reactions or the ends changed in this poll is taken as it now is.
- The read of the replies: one `connectors` child per such post and poll, through `invoke` (the
  turn, the slot, the group, the bounds on the streams), with no idempotency key:

  ```
  connectors --output json invoke <target> conversations.replies --input-stdin
  ```

  with the input
  `{"channel":"<channel id>","ts":"<reference of the post>","oldest":"<reference>","limit":100}`:
  `oldest` is the last message of the thread the bridge read (`read_to`), or the post itself
  before the first read; the provider then lists the messages after it, that one not included.
  One page: no `cursor`. The existing read of the channel (`conversations.history`) does not hand
  the replies through: the provider lists the replies of a thread only in this read (not
  verified). So a poll makes at most 1 read of the replies per live post, besides the bound of
  "The end of a message".
- The answer is judged as a page of the channel is (exit 0, one JSON document,
  `result.result.status` 200, `result.result.body.ok` `true`, `result.result.body.messages` a
  list; exit 1, the refusal of a read). Each message of it is an object with a `ts` text of the
  form `<digits>.<digits>`; its `user` and `text`, when there, are texts. A member of another type
  fails the read whole. Every message at or before `oldest` is left out; the message whose `ts` is
  the post's own (the provider lists the parent first, not verified), a message whose `ts` is one
  of the bridge's own lines (below), and a message whose `user` is not the allowed chat user are
  left out of the replies; the rest are taken in the order of their `ts`, oldest first. Once every
  reply of the page is handled, the record takes the greatest `ts` of the page as `read_to`,
  whoever wrote that message (Finding 2 of the adversary, the coordinator's decision): a page full
  of replies of other chat users moves the next read past them, and the person's reply after them
  is read at the next poll. A reply of another chat user gets no line and sends no request; its
  one mark is this member of the bridge's own record. (Following `has_more` and the cursor through
  every page at each poll would write nothing, and would cost one read more per 100 replies of
  the thread at every poll.)
- A read of the replies that fails is that post's (Finding 1 of the adversary): the child
  exits other than 0, its answer says `ok` `false` (a post removed from the channel:
  `thread_not_found`, not verified), or the page is refused as above. The bridge writes one line
  on standard error for the first failure of a run of them of that post, in the form of a failed
  poll (`inbox bridge: the read of the replies to the post of <message id>: <why>; <after>, and it
  is read again at the next poll (no further line until a read of it succeeds)`). A read that
  failed without a wait it named goes on with the posts after it in the same poll (`<after>`: `the
  posts after it are read all the same`). A read refused `rate_limited` with a
  `retry_after_seconds` the bridge takes (1 to 3600) ends the reads of the replies of this poll
  (Finding 2 of adversary pass 2): the posts after it are read at the next poll, which comes after
  that wait (`<after>`: `the posts after it are read at the next poll, after that wait`), since the
  provider's limit counts the calls of the method, not of one thread (not verified). A post whose
  reads keep being refused so keeps the posts after it waiting as long. The poll does not count
  as failed: the read marks, the ends and the replies of the other posts do not wait for the
  pause of a failure; the next poll comes after the interval, or after the wait such a refusal
  named when that is longer. An id leaves the set of failing reads when its entry ends, so a
  repost starts a new run. The `read` before an `Answer`, an `Answer` without an answer the bridge
  can act on, and a change of the record that fails still fail the poll: they are failures of the
  service or of the host, the same for every post.
- The bridge's own lines (the coordinator's item 6). When the connection of `connectors` posts as
  the allowed chat user, the read of the replies lists each line the bridge wrote into the thread
  as a reply of that user. The post operation answers the reference of the line it made (the `ts`
  of "A post"); the bridge keeps it in the entry (`own_lines`) when the line went through, and a
  message of the thread with such a reference is never a reply: no request, no line. A reference
  leaves `own_lines` once `read_to` is at or past it, since no later read lists it again. Not
  verified against the provider: that the read of the replies lists a line of the bridge with the
  `ts` the post operation answered for it.
- The match (Open 2, answered A and A). For a decision: the text of the reply, trimmed of white
  space, is the letter of one option in either case, or starts with that letter followed by a
  space, a colon or a full stop; the rest, trimmed, is the body, and with no rest the body is the
  text of the option. Any other text names no option. For a request: the whole text is the body,
  as it is. A letter anywhere else in the text is no match.
- What a reply of the allowed chat user becomes, in this order:
  1. the entry is in the state answered: no request; the line of `already-settled` (Open 4 A);
  2. the text is longer than 65536 bytes of UTF-8 (`Body`): no request; the line "too long";
  3. the message is a note: no request; the line "a note takes no answer";
  4. a decision whose reply names no option: no request; the line that names the letters;
  5. a request whose reply's text is empty or only white space (a file or an image shared in the
     thread: the provider lists its `text` empty, not verified): no request; the line "it holds no
     text" (Finding 3 of the adversary);
  6. otherwise the `Answer`. When the row is in `Delivered`, the `read` of `inbox read` goes first
     (Open 3 A), so the message moves `Read` to `Answered`; its success makes the entry `read`; a
     refusal of the model (the message due, read or gone meanwhile) does not stop the `Answer`,
     which gets its own answer.
- The `Answer` is the one of `inbox answer` (`commands::answer`): a new reply id, `created` the
  system clock, `expires` `created` plus 24 hours, the option, the body, signed with the bridge's
  key. With `--for` and `--authority`: `from` the bridge's principal, `on_behalf_of` the address of
  `--for` (the person), `authority` the delegation id, sent under the header; the service asks
  that `on_behalf_of` is the person of the inbox and that the delegation is live. Without them:
  `from` and `on_behalf_of` the person, no `authority`, as `inbox answer` without flags.
- Its answer:
  - `answered`: the entry moves to the state answered, with the reply as the last one handled. No
    line: the message is answered as the person meant.
  - a refusal of the model (`already-settled`, `deadline-passed`, `expired-on-arrival`,
    `shape-mismatch`, `no-authority`, `authority-invalid`, `signature-invalid`, and every other
    declared one), or of the service with a status below 500 but `401` and `408`: the line that
    names it; it is not sent again. A reply after `expires` is answered `deadline-passed` this way,
    and the decision is left to silence ("The end of a message").
  - no answer, `401`, `408`, from `500` on, or an answer that is neither: the poll fails (the
    pause of a failure), the reply is not handled, and the next poll sends a new `Answer` for it
    (unless the service took the first: "The record", below); no line in the thread. On standard
    error the `Answer` gets a line of its own (Finding 1 of adversary pass 2), not the line of a
    failed poll: `inbox bridge: the answer to <message id> from the thread of its post got no
    answer of the service that the bridge can act on (<failure>), so whether the service took it
    is not known; the next poll sends it again unless the message has left `Pending` by then (no
    further line until the service answers an answer to it)`, once per run of such failures of
    that message; the run ends when the service answers an `Answer` of it or its entry ends. The
    line of a failed poll then names only the first other failure of a run of failed polls. A
    `401` with `--for` ends the bridge ("The key of the bridge").
- The lines. One reply in the thread of the post, through the write of "The end of a message"
  (the input `{"body":{"channel","text","thread_ts"}}`), under the key
  `inbox-<message id>-reply-<reference of the reply>`: derived from the message and the reply
  only, the same at every try. The texts, none of which quotes the reply:
  - `the reply was not taken: the service refused it: <outcome>` (`already-settled` also for a
    reply after the one taken);
  - `the reply was not taken: it is longer than 65536 bytes`;
  - `the reply was not taken: a note takes no answer`;
  - `the reply was not taken: it names no option; reply with one of the letters <A, B>`;
  - `the reply was not taken: it holds no text`.
  The outcome and the letters go through the escapes of the head line of a post.
- The record (decided 4 of the coordinator). Each entry keeps the reference of the last reply of
  the allowed chat user it handled (`replied`, the reply of the key of its line), the text of a
  line it owes for it (`owed`), the last message of the thread it read (`read_to`, moved to the
  reply with `replied` and past the whole page once every reply of it is handled), and the
  references of its own lines no read has passed yet (`own_lines`). For a line: first the record
  takes `replied`, `read_to` and `owed`, then the line is written, then `owed` is dropped and the
  line's reference joins `own_lines`. A line that fails is named in one line per run of
  failures, fails no poll, and is written again at the next poll with the same key and text,
  before any later reply of that post is handled. For an `Answer` taken: the record takes the
  state answered, `replied` and `read_to` after the answer. So a repeated poll and a new start of
  the bridge send no second `Answer` and write no second line for a reply they handled. An
  `Answer` the service took whose answer was lost (or a bridge ended, or a change of the record
  that failed, between the answer `answered` and the record change): the poll fails; the next poll
  finds the message out of `Pending`, its row in `Thread` in `Answered`, and gives the entry the
  end `answered` before any read of its thread ("The end of a message"): 1 `Answer`, no line. A
  later reply of the person to that message gets no line either: the entry is at its end and its
  thread is no longer read (measured by the adversary, held by
  `an_answer_taken_whose_answer_was_lost_ends_the_entry_with_no_line`).
- A reply's text is data: it reaches the service only inside the signed envelope, never a command
  line, a file name, a line of the bridge on standard error or a line in the thread.
- Not verified against `connectors` and the provider (no call was run): the name and the input of
  the read of the replies; that the provider lists the replies oldest first, the parent post
  first, those after `oldest` only and at most `limit` of them; the members `user` and `text` of a
  reply, and an empty `text` for a file or an image; the answer `thread_not_found` for a removed
  post; that a line of the bridge is listed with the `ts` its post operation answered; the escapes
  of the provider in a reply's text (`&amp;`, `&lt;`, `&gt;` reach the body as they arrive).

## The clock of the bridge

`inbox bridge --clock-file <path>` is a flag of a debug build only, hidden from `--help` and from
the command-line reference, read through `FileClock` of `crates/inbox-cli/src/clock_file.rs` as
`inbox serve --clock-file` reads its file (`docs/design/service.md`, "The expiry task"): the
instant the file holds, each time the bridge asks for the time. It fails closed as the service's
does: a file that does not hold one instant refuses the start (exit 4, `--clock-file <path>:
<cause>`), and while it runs a reading that fails or goes back ends the process at once with exit
4 and one line `inbox bridge: the clock file <path>: <cause>: stopping`. Without it, the system
clock. The clock decides when the `expires` of an entry and its retention have passed; the bridge
still signs its requests with the system clock, as every client command does, so a case gives the
bridge a clock file of its own and keeps the service's within 60 s of the system clock.

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
- A decision (story:the-chat-bridge-turns-a-thread-reply-into-an-answer, Open 1 A): after the
  block, before the line of a cut, one line per option `<letter>: <text>` in the order of the
  row, the recommended one followed by ` (recommended)`; then `silence takes option <letter> at
  <expires>`, or `silence takes no option: it expires at <expires>` when the row names no
  `default_on_silence`; then `reply in this thread with the letter of an option`. The letters, the
  texts and `expires` go through the escapes of the head line. A note and a request are as
  before.

## The record of the chat posts

`<outbox>.chat.<16 hex digits>` beside the outbox file (Open 6, answered A with iii): the digits
are the FNV-1a hash of the address, as in the lock of the wake. One JSON line per `ChatPost`,
`{"message_id":…,"channel":…,"reference":…,"state":…,"thread":…,"expires":…,"end":…,"gone_at":…,"replied":…,"owed":…,"read_to":…,"own_lines":[…]}`
(`replied`, `owed`, `read_to` and `own_lines`, a list of texts left out when empty: "The
answer") with the
chat channel the post was made in (a line without `channel`, of a build before it was recorded,
is read as of the bound channel), the read mark `posted`, `read` or `due`, the thread and the
`expires` of the message as the row of `Pending` named them at the post, the end, when it has
one, and, for an entry without `expires`, the reading of the bridge's clock at which it first
found the message gone (`gone_at`, in the form of `format_reading`) (a post writes it `posted` with `Outbox::record_chat_post`; the poll changes its read mark
with `Outbox::mark_chat_post`, its end or drops it with `Outbox::change_chat_post`, and reads the
record with `Outbox::chat_posts`). The record is read only whole: a line that is no entry (no JSON
object with the text members `message_id`, `reference` and `state`) makes it a record that cannot
be read, and the line on standard error names the record and the number of the line, since the
entry such a line held would else be taken as missing and its message posted a second time. The
bridge writes the record whole, each line with its line end, and renames it over the old one, so it
never leaves a line cut short or an empty line: an empty line is a line that is no entry too, and a
last line without its line end is read as every other line (an entry when it is one). A line of a
build before
`thread`, `expires` and `end` were recorded is read without them: its thread is its message id,
it is held as every other entry (its repost is not posted), it never gets the line of an expired
message, and once its message is gone it records `gone_at` and is dropped when the retention
counted from there has passed. Mode 0600, changed under the lock of the
outbox file and replaced whole by a file written beside it and renamed over it, as the other files
beside the outbox file (`Outbox::record_chat_post` in `crates/inbox-cli/src/outbox.rs`, through
`replace`): whatever is at the name of the staged file `<record>.new` is removed and the file is
created anew with mode 0600, so a link planted there is not followed. A line of the same message
id is replaced; every other line is kept as it is. A file that is no regular file, holds more than
64 MiB, cannot be read or holds a line that is no entry fails the change (exit 4) instead of being
read as holding nothing: its lines are never given up for a new one. A round that cannot read it
posts and marks nothing and ends the bridge with 4 ("A repost"); a poll that cannot read it marks
and drops nothing and is tried again after the pause of a failure ("The read mark").

The record does not grow without a bound (Open 1, answered A): an entry is dropped, under the
outbox lock through `replace` and only while its line is the one the poll read, when its message
is gone from the service (its read of `Thread` answers no row) and the clock of the bridge has
passed its `expires` (for an entry without one, its `gone_at`) plus the retention
(`--chat-post-retention <seconds>`, default 86400, the
default answer retention of the service). So the record holds the entries of the messages of
about the last day and a half at the defaults. An entry of a channel no bridge of the inbox is
bound to any more is not dropped, and an entry whose message the service still holds (an answer
taken on silence its sender did not collect) stays until a later poll finds it gone. The bound of
64 MiB still holds for a record a user keeps from growing otherwise; a post that cannot change the
record ends the bridge with 4.

## Refusals at the start

The bridge refuses to start with one line on standard error and no request:

- no binding is configured: one of the 3 flags is missing (exit 2, `no binding is configured for
  <address>`), or a value is empty or holds white space or a control character (exit 2);
- `connectors` is not on `PATH` (exit 4);
- the URL of the service is not a loopback URL (exit 2, the client's own refusal);
- `--chat-poll-interval` is not a whole number of at least 1, or `--chat-post-retention` not a
  whole number (exit 2, a usage error of the command line);
- in a debug build, `--clock-file` names a file that does not hold one instant (exit 4).

It starts no listener (Open 8, answered A): it asks the provider through `connectors` for the
reactions and for the replies ("The answer").

## The operator's check

story:the-operator-runs-the-chat-bridge-on-the-real-channel. The gate holds the bridge against a
stand-in `connectors` ("Cases", below) and never starts the real one: no task, test, CI step or
gate reaches the chat provider. The operator runs this check by hand on one real chat channel
once for that story, and again once per release that changes the bridge (Open 1 A of the story),
and records what was seen as evidence of the story. Not run yet: no run is recorded.

Every value below that names the chat provider is a placeholder: `<channel id>` the id of the
chat channel, `<chat user id>` the id of the allowed chat user, `<connector>/<instance>` the
target of `connectors invoke`, `<person address>` the address of the person (`human:<name>`),
`<sender address>` that of the sender (`agent:<host>/<name>`), `<bridge address>` that of the
bridge (`agent:<host>/chat-bridge`) and `<delegation id>` the id of the delegation. None of
them is written into this repository.

### What the operator sets up before the run

Each item is the operator's act, never the bridge's or a task's:

- `connectors` 0.77.0 or later first on `PATH` in the terminal of the bridge (`connectors
  --version`), with a connection to the chat provider whose instance may run
  `chat.postMessage`, `conversations.history` and `conversations.replies`.
- One standing grant of that instance for the operation `chat.postMessage` into the one channel
  `<channel id>`, which admits the input members `/body/channel`, `/body/text` and
  `/body/thread_ts` and no other: the bridge sends no other ("A post" for a post, "The end of a
  message" for a line in a thread, which "The answer" writes the same way). It is made with
  `connectors approvals grant-standing --input-file <file>`, together with the member
  `standing_grants` of the instance in the configuration of `connectors`; the form of the file
  is on the page "running services" of `connectors`, section "standing write grants". Its cap
  of writes per hour leaves room for the 3 writes of the run (2 posts and 1 line), and one more
  per reply that names no option. The bridge runs no `connectors approvals` command and passes
  no proof.
- The keys of the run, made for the run in an empty directory, as `website/docs/first-use.md`
  makes them: the operator key of the key file, a key of the person, a key of the sender and a
  key of the bridge; the key file with the 3 key entries and the delegation from the person to
  the bridge ("The key of the bridge"). The entries are valid 30 days unless `--valid-until`
  names a shorter end. The bridge gets only its own key; the person's key lets the operator look
  at the person's inbox (`inbox pending --secret-key-file person.key`).

  ```sh
  OPERATOR_PUBLIC_KEY=$(inbox key new --out operator.key)
  PERSON_PUBLIC_KEY=$(inbox key new --out person.key --key-id person-1 --principal <person address>)
  SENDER_PUBLIC_KEY=$(inbox key new --out sender.key --key-id sender-1 --principal <sender address>)
  BRIDGE_PUBLIC_KEY=$(inbox key new --out bridge.key --key-id chat-bridge-1 --principal <bridge address>)
  inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id person-1 --principal <person address> --public-key "$PERSON_PUBLIC_KEY"
  inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id sender-1 --principal <sender address> --public-key "$SENDER_PUBLIC_KEY"
  inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id chat-bridge-1 --principal <bridge address> --public-key "$BRIDGE_PUBLIC_KEY"
  inbox keyfile add-delegation --file keys.json --operator-secret-key-file operator.key --delegation-id <delegation id> --agent <bridge address> --on-behalf-of <person address>
  ```

- A start of `inbox serve` on a loopback address after the key file is written, or a restart of
  one that ran before it: the service reads the key file at its start only.

  ```sh
  inbox serve --listen 127.0.0.1:0 --key-file keys.json --operator-public-key "$OPERATOR_PUBLIC_KEY"
  ```

  In each other terminal of the run, in the same directory: `export INBOX_URL=<the URL inbox
  serve printed>` and `export INBOX_OUTBOX="$PWD/outbox.jsonl"`, so the files the run leaves
  beside the outbox file are in that directory.

### The run

1. Start the bridge with its own key for the person:

   ```sh
   inbox bridge --secret-key-file bridge.key --for <person address> --authority <delegation id> --chat-channel <channel id> --allowed-chat-user <chat user id> --chat-connector <connector>/<instance> --chat-poll-interval 5
   ```

   `--chat-poll-interval` is at least 1 and 10 by default; 5 lets a reply be seen within about
   5 s, and a poll of the run reads the channel once and the replies of each of its 2 posts once
   at most, about 36 calls of `connectors` a minute; the provider's rate limits are not verified,
   and a read refused `rate_limited` waits as "The read mark" says. The bridge writes nothing
   while it runs well. It opens the person's inbox with the adapter `slack`, and
   `<outbox>.opened` then holds the line of `<person address>` with the adapter `slack`.
2. The sender posts decision 1 to the person's inbox, with the default `expires` (24 hours):

   ```sh
   inbox post --secret-key-file sender.key --to <person address> --kind decision --body "Check 1 of the chat bridge: which option?" --option "A=Keep" --option "B=Change" --recommended-option A
   ```

   It prints the message id. Seen: one post in the channel, its head line `decision from <sender
   address> expires <expires> message <message id>`, the body in a block, the lines `A: Keep
   (recommended)`, `B: Change`, `silence takes no option: it expires at <expires>` and `reply in
   this thread with the letter of an option` ("The text of a post"); `inbox outbox
   --secret-key-file sender.key` shows the message `Delivered`.
3. The person replies `B` in the thread of that post. Seen within about the poll interval:
   `inbox outbox --secret-key-file sender.key` shows the message `Answered`, and

   ```sh
   inbox collect --secret-key-file sender.key <message id> --json
   ```

   prints a document whose `message` has `answered_by` `principal` and `decided_option` `B`, and
   a `reply` with `option` `B`, `from` `<bridge address>`, `on_behalf_of` `<person address>` and
   `authority` `<delegation id>`. No second post in the channel and no line in the thread.
4. The sender posts decision 2, which silence decides (Open 2 A of the story), with an `expires`
   5 minutes ahead in RFC 3339 (such as `2026-11-24T12:05:00Z`; the command below asks GNU
   `date` for it):

   ```sh
   inbox post --secret-key-file sender.key --to <person address> --kind decision --body "Check 2 of the chat bridge: leave this without a reply" --option "A=Proceed" --option "B=Stop" --recommended-option B --default-on-silence A --expires "$(date -u -d '+5 minutes' +%Y-%m-%dT%H:%M:%SZ)"
   ```

   Seen: one post in the channel, with `silence takes option A at <expires>`. Nobody reacts and
   nobody replies. Seen after `expires`, within the sweep interval of the service (10 s by
   default) and one poll: one line in the thread of that post, `taken on silence: option A,
   Proceed` ("The end of a message", Open 2 A of story:the-chat-bridge-asks-a-decision-once).
   Only then `inbox collect --secret-key-file sender.key <message id> --json` prints a
   `message` with `answered_by` `silence` and `decided_option` `A`, the `default_on_silence` of
   the post. A collect before the bridge saw the answer makes the line `expired, not answered`
   instead ("What stays open").
5. End the bridge with SIGINT (Ctrl-C in its terminal), then `inbox serve` the same way. Left on
   the host, beside the outbox file: the record of the chat posts (`<outbox>.chat.<16 hex
   digits>`, one entry per decision until a later bridge drops it at its retention), the file of
   the lock of the wake (`<outbox>.wake.<16 hex digits>`, which holds no lock once the bridge
   has ended), the line of the person's inbox in `<outbox>.opened` (adapter `slack`), and the
   outbox file with its epochs file and its lock file; the keys and the key file of the run in
   its directory. The service kept everything in memory and loses it. The 2 posts and the line
   stay in the channel, and the standing grant stays in `connectors` until the operator revokes
   it (`connectors approvals revoke-standing`).

### What to record

One evidence record of the story (`aep plan artifact evidence`, a kind from the closed list the
command prints), written by the coordinator from what the operator hands over:

- the day of the run, and the versions `inbox --version` and `connectors --version` printed;
- each item of the run as seen or not seen: decision 1 appeared as one chat post with its
  options by letter; its collect printed `answered_by` `principal` and option `B`; no second
  post and no line in its thread; decision 2 was collected with `answered_by` `silence` and
  `decided_option` `A`; its thread got the line `taken on silence: option A, Proceed`;
- what else was seen or not seen, such as a reaction, a reply that names no option and its line,
  a restart of the service during the run, a line of the bridge on standard error (such as a
  refusal `approval_required` or `rate_limited`).

The record holds no name of a chat workspace, a chat channel or a chat user, no chat user id, no
token and no key, and no text of a message other than the messages made for the run.

### What the check does not show

The run reaches the post, the read of the replies and the line of silence. It does not reach
what this document marks not verified beyond them: the provider's rate limits and the cost of
one `connectors invoke`; a repeated idempotency key after a lost answer of `connectors`, and
whether such a post arrives once; `idempotency_conflict`; a thread with more than 100 replies,
and a channel with more than 10 pages of posts after an open post; a parent post that is gone
(`thread_not_found`); a reply that is a file or an image (an empty `text`) and the provider's
escapes in a reply's text; a grant that ended or reached its cap; a refused key (`401`); a
restart of the service or a SIGKILL of the bridge during the run; the line `expired, not
answered`; and the read mark on a reaction, which the run does not ask for.

## What stays open

- Approval. `connectors` refuses every write that has neither an approval proof nor a standing
  approval. By the operator's decision (D89, answer B) the bridge issues and passes no proof,
  ever: it runs no `connectors approvals` command and passes no `--approval-file`. The bridge
  needs `connectors` 0.77.0 or later (released 2026-11-24, with `connectors approvals
  grant-standing`, `list-standing` and `revoke-standing`) and one standing grant of the operator
  for the channel, a policy entry that admits the post operation into one fixed channel without a
  proof per write; giving it is the operator's act, never the bridge's. With no live grant each
  post is answered `not_attempted` with the code `approval_required` and waits in `Sent`. The
  first run on a real channel is the operator's check (above), not run yet.
- Not verified: every statement about the answers of `connectors` and of the provider, since no
  call was run, by the bridge's cases or by hand: the operations of the provider's connector and
  its input beyond what `connectors` says (above), the failure codes of the grant, the provider's
  escapes and the delimiter, and whether a repeated idempotency key makes a repeated post arrive
  once.
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
- Once per message id while the record holds the id: a message whose id comes back after its
  entry was dropped (its `expires` plus the retention passed) is posted again, and an entry of
  another channel holds the id too, so a bridge started on another channel marks such a message
  and does not post it into its own channel.
- A decision taken on silence whose answer the sender collected before a poll saw it gets the line
  of an expired message instead of the line of silence.
- A reply refused `idempotency_conflict` (a text built from a row that changed between two tries,
  such as a row collected in between) is tried at each poll until the entry is dropped at the end
  of its retention.
- A reaction removed after the poll read it does not make the message unread; the provider's
  answer names no time of a reaction, so a reaction made before the post was recorded counts.
- The replies of a thread are read one page of 100 at a time from the last message of the thread
  read, whoever wrote it: a reply of the allowed chat user after 100 or more replies of other
  chat users is read one poll later per full page before it.
- The connection of `connectors` may post as the allowed chat user: the bridge tells its own
  lines from the person's replies by the references the post operation answered for them, kept
  in the record ("The answer"). A line whose reference did not reach the record (the bridge
  ended, or the change of the record failed, after the line was made) would read back as a reply
  of the person; the line is written again with the same key at the next poll, and the reference
  that answer names is kept then (whether a repeated key answers the first line's reference is not
  verified).
- A reply to a message answered in a terminal, or by silence, gets no line: the entry is at its
  end and its thread is no longer read. A reply after the sender collected an answer taken from
  the thread gets none either.
- The text of the post of a decision changed with
  story:the-chat-bridge-turns-a-thread-reply-into-an-answer (its options): a decision tried before
  that change and not yet posted meets `idempotency_conflict` ("The idempotency conflict" above).

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

The cases of a message asked once are in `crates/inbox-cli/tests/bridge_once.rs`, against a real
`inbox serve` of the debug build with `--clock-file` and `--sweep-interval 1` behind one forwarder
that keeps what each connection sent (a restart of the service is a new `inbox serve` the
forwarder is put in front of), and a stand-in that numbers a post, a reply (a post whose input has
`thread_ts`) and a read of the channel apart. One case restarts the service while the bridge runs
(waiting until the bridge finds the messages gone before the sender's repost) and again while
none runs, starting the second bridge bound to another channel, and counts the posts, the
`MarkDelivered` and the `read` requests per message and the line of a decision reposted and then
decided on silence. One gives the bridge a clock file of its own, moves it and the service's past
`expires`, and checks every reply, its key and its input, with an option text that needs every
escape of the head line, and that a refused or answered message gets none. One holds the
retention: three entries, one of them rewritten as the build of the story before wrote it; the
two whose start of the retention plus 1 h has passed leave the record in one poll and the third
is kept, each wait on a fact of the record, none on a count of polls. One has the stand-in fail
every reply into the thread of one post until the case lets it through: meanwhile another message
is posted and another post is read on a reaction, one line names the failure, and the entry is
not at its end; then the reply goes through with the key and the input of the failed tries. One
cuts the line of a posted request short: the poll fails with one line naming the record and the
line, and the repost after a restart of the service ends the bridge with 4, no post, the request
in `Sent`. The unit cases of the record's lines are in `crates/inbox-cli/src/outbox.rs`, that of
the `read` sent again in `crates/inbox-cli/src/bridge.rs`. The adversary's cases of this story
are in `crates/inbox-cli/tests/adversary_bridge_once.rs`: a `read` of a repost lost once, a
record the rounds cannot read at a repost, an entry of the build before after a restart of the
service, a held line cut short at a repost, a record that cannot be written, and the bridge's
clock file removed while it runs.

The cases of the bridge's own key are in `crates/inbox-cli/tests/bridge_own_key.rs`, in the forms
above: the key file of the service holds the key entry of `agent:demo/chat-bridge` and its
delegation from the person (`support::Signer`). They run the acceptance cases of `bridge.rs`,
`bridge_read.rs` and `bridge_once.rs` again with that key, and check the header of every request
the bridge's key signed (by its `Inbox-Key-Id` at the forwarder), the lock against a second bridge,
the person's own wake and a bridge for another person, the line of `<outbox>.opened` and the
`Inboxes` view after a restart of the service, the record of a bridge with the person's key read
on by one with its own, the check of the adapter, and the end on a refused key (no delegation, a
lapsed delegation, a lapsed key entry: one request, one line, exit 1). The forwarder of
`bridge_once.rs` replaces the answer to a named request with that `401`, and its stand-in holds a
read of the channel on a file of the case: so a `401` reaches the poll's whole read of `Pending`
first (one line, no request after it), and one reaches the rounds while the poll's `connectors`
child runs (the child is gone when the bridge has ended). A delegation that lapses at the
service's clock while the rounds hold their read ends the bridge the same way. Further cases run
`inbox wake`, `inbox open` and `inbox close` under `--for` (with one outbox file for 2 persons, a
close for one keeps the line of the other), and hold that the agent's kept envelopes, the one
posted under `--for` and the one without, are posted again once each by its next command after a
new start, with the header when that command runs under `--for`. The usage of `--for` and
`--authority` alone is in `crates/inbox-cli/tests/acting_for.rs`; the adversary's case of a line
break in `--authority` is in `crates/inbox-cli/tests/adversary_bridge_own_key.rs`.

The cases of the answer are in `crates/inbox-cli/tests/bridge_answer.rs`, in the forms of
`bridge_once.rs`: its stand-in answers a read of the replies of a post with the replies the case
scripts for it (else none), the messages after `oldest` and at most `limit` of them, and its
forwarder, when the case asks for it, drops one request without an answer, answers one `408`
without carrying it, or carries one and throws its answer away. Every stand-in of the bridge's
cases answers a read of the replies with none. A case collects the answer with `inbox collect`
of the sender. Round 2 added the cases of a full page of other chat users, an answer lost after
the service took it, a `408`, an entry of another channel, a repost of an answered message and
the bridge's own line read back as the person's reply; the adversary's cases are in
`crates/inbox-cli/tests/adversary_bridge_answer.rs` (a read that fails for one post, a full page
of other chat users, a reply without text; pass 2: the line of an answer lost after the service
took it, a read refused `rate_limited` that fires no other read, a run of failed reads named
once, an own line a read has passed). Round 3 added the cases of the wait a refused read of the
replies named (a lower bound of time from the clock the stand-in reads), of an `Answer` without
an answer that fails its poll, and of an entry that names no channel; its script in front of the
stand-in is the adversary's (`scripted`). The unit cases of the match and of a page
of replies are in `crates/inbox-cli/src/bridge.rs`, those of the text of a decision's post in
`crates/inbox-cli/src/output.rs`.
