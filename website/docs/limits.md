---
title: Limits
description: What the inbox does not do, or does wrong, in release 0.6.1, the security limits first.
---

# Limits

What the inbox does not do, or does wrong, in release 0.6.1. The security limits come first.

**Use it for the principals of one operator only**: that operator's own sessions and hosts.
The service refuses, before binding, any address that is not a loopback address, and the
client refuses an `INBOX_URL` outside loopback.

## Security findings, open

### 1. A principal that holds a message id learns whether it is live

A post into an open inbox answers `id-taken` (409, error `MessageIdTaken`) for a `message_id`
that a stored message of another sender carries, and `posted`, which stores a message, for a free
one. So a principal that holds a message id learns from a post whether the id is live, and it
can take a free id first, after which the sender's post is refused `id-taken`. A message id is a
ULID the sender's client makes, and it carries its creation time; the client has no flag to
choose one.

The same post to an address no inbox has, or into a closed inbox, answers the same for a held
and a free id: `unknown-inbox` or `inbox-closed`, or `duplicate-key` when a live message of the
caller carries the post's dedup key. `duplicate` answers the sender's own stored message only,
also after the recipient closed the inbox.

- Accepted for principals of one operator.
- A message id unique per sender removes it. That change is planned
  ([features](status.md#planned)) and required before principals of different operators share
  one service.

### 2. A signed request is accepted again inside its 60 s window

The service keeps no record of the requests it has seen. A request signature is valid from
60 s before to 60 s after the service's clock reading, both bounds inclusive. A signed request
sent again inside that window is answered again. Posts are idempotent by `message_id` and
`dedup_key`; a repeated `Answer` is answered `already-settled`. Version 1 leaves a replay
record out.

## The service

- **Loopback only.**
- **No second host.**
- **The Claude Code plugin was run on one real session, a headless one.** On Claude Code
  2.1.330 (`claude -p` with its input and output as `stream-json`), a session that stood idle
  for 59 s took a turn 2.0 s after a note was posted to its inbox, read the note with
  `inbox read`, and the plugin waited again when that turn ended. Not seen on a real session:
  an interactive session in a terminal, a wait longer than 59 s, a request answered with
  `inbox answer`, more than one message, and TLS. The tests run the plugin against a stand-in
  for Claude Code. No other adapter wakes a session.
- **A reader that ends right after it took the messages leaves them marked.** `inbox wake`
  whose output is a pipe ends without marking anything when the reader closes the pipe, also
  while it waits, but messages a reader took and did not show before it ended are marked
  delivered, and so are messages a reader left unread in the pipe when it closed it right after
  the wake checked it: `inbox pending` still lists them, and no later wait returns on them. The
  Claude Code plugin ends its wait within seconds when Claude Code ends without ending the
  plugin's hook, such as in a crash; a post that arrives in that time, up to 2 s and longer on a
  busy host, is marked delivered and shown to no session. This holds as long as Claude Code
  started the hook itself; that it does is not checked, and if it starts the hook through a
  shell of its own, such a wait keeps running and marks the next message though no session
  reads it. End that process ([first use](first-use.md#claude_code)). When the plugin's
  temporary directory fills up while it reads, it says so and stops the wait, and the messages
  that wait had already taken are marked delivered and shown to no session. The plugin reads the
  messages of one wake byte by byte: 8 MB of them took 6.4 s on a busy host, so a large
  backlog wakes the session seconds late.
- **One backend, in memory.** A restart of the service loses every message and every open
  inbox. The next command of the owner opens its inbox again, and the next command of the
  sender posts its kept envelopes again, by themselves; dedup by `message_id` and `dedup_key`
  makes the repost arrive once. The inbox is opened again only when `inbox open` ran on this
  host with this version or later; after an upgrade, run `inbox open` once. An envelope kept by
  a version before this one is not posted again by itself; a typed `inbox repost` posts it.
  More than one typed `inbox open` or `inbox close` of the owner while a reconnect runs can
  leave the record and the service apart until the next typed `inbox open` or `inbox close`.
- **Nothing is deployed**, and no release attaches a binary ([releases](releases.md)).
- One request per connection: no keep-alive and no pipelining.
- The key file is not reloaded while the service runs: a new or a removed delegation takes effect
  at the next start.
- **Acting for another inbox (`--for`) has limits.** An agent with a live delegation for a
  person reads, marks, answers and refuses the messages of the person's inbox with `--for`.
  - `inbox open`, `inbox close`, `inbox wake` and `inbox bridge` do not take `--for`: each keeps
    state on this host under the caller's own address. So no command wakes an agent for the
    person's inbox yet, and none opens it for the person.
  - An inbox an agent opens for the person through the service (the request with
    `Inbox-On-Behalf-Of`) is the agent's: the person's own key does not close it, and the agent
    closes it only with that header while its delegation is live. After the delegation lapses
    nobody closes it until the service starts again.
  - An agent with a live delegation answers any message of the person's inbox in the person's
    place, one it sent itself included, and the record says the person answered. A sender that
    must not answer its own question holds no delegation of the person it asks.
  - A wait admitted under a delegation reads on after the delegation lapses, until the wait ends.

## The chat bridge

`inbox bridge` posts each message of a person's inbox into a chat channel through `connectors`,
marks it delivered, and marks it read when the person reacts to its post. What it does not do:

- **It needs `connectors` 0.77.0 or later and the operator's standing grant.** `connectors`
  refuses each post (`approval_required`) until the operator gave it one standing grant of posts
  into the channel (`connectors approvals grant-standing`); the post waits as undelivered until
  then. The bridge passes no approval of its own and gives no grant.
- **Not run on a real channel yet.** No call of `connectors` was run, by the bridge's tests or by
  hand: what the bridge expects of the answers of `connectors` and of the chat provider is not
  checked against them.
- The operator's approval of the posts has an end time: once it has ended, posts wait as
  undelivered until the operator renews it.
- The approval allows a number of posts per hour: over it, posts wait as undelivered until the
  hour's cap frees.
- It reads no reply and no decision from the channel. The person answers with `inbox answer`.
- Only reactions of the allowed chat user count, any reaction of theirs: a reaction of anyone
  else does nothing and is not reported.
- A reaction is seen at the next read of the channel, every 10 s by default
  (`--chat-poll-interval`). Taking a reaction back does not make the message unread.
- A reaction on a message past its deadline does not mark it read: the bridge writes one line,
  and the message is removed or decided at its deadline as any other. A bridge stopped just
  before that line never writes it.
- The bridge reads at most 1000 posts of the channel at a time. While a message the person has
  not read yet was posted more than 1000 posts back, every read of the channel fails and no
  reaction marks a message read, until that message is read with `inbox read`, answered, or
  removed at its deadline.
- A bridge reads only the channel it serves: started again with another `--chat-channel`, it
  does not see reactions on the posts it made in the channel before.
- Whether `connectors` hands the reactions of a post through to the bridge is not checked
  against the chat provider.
- A posted message is marked delivered whether or not anyone reads the channel.
- A message posted and not marked delivered, such as when the bridge was stopped between the
  two, is posted again by the next bridge, with the same idempotency key; `connectors` answers
  that as a replay of the first post, not as a second post.
- Stopping the bridge with SIGINT or SIGTERM ends a post that is still running: its `connectors`
  and the processes that `connectors` started, unless one of them left its process group. A
  SIGKILL of the bridge or a hang-up of its terminal does not: the running `connectors` goes on
  alone, and what it posts is not recorded.
- A process that a finished `connectors` left behind is not ended by the bridge.
- A message is posted once per message id while the record of the posts holds that id: after a
  restart of the service the sender's outbox delivers it again, and the bridge marks it
  delivered, and read when it was read before, without a second post. A message the bridge
  posted into another channel before counts as posted.
- A message nobody answered gets one reply in its thread at its end: "taken on silence" with the
  option, or "expired, not answered". The bridge never answers in the person's place. A decision
  taken on silence that its sender collected before the bridge looked gets "expired, not
  answered". A reply that fails is tried again at a later read, under the same key and with the
  same text; the bridge reads the end of a message only every `--chat-poll-interval` seconds.
- A post leaves the record when its message is gone and its deadline plus
  `--chat-post-retention` (one day by default) has passed; a post recorded by an older bridge,
  which kept no deadline, leaves it that long after the bridge first found its message gone, and
  never gets the "expired, not answered" reply. A message reposted after its post left the record
  is posted again.
- When the record of the posts cannot be read (no permission, over 64 MiB, not a regular file, a
  line that is not a whole entry) as a message comes in, the bridge posts and marks nothing and
  stops with exit status 4, naming the record; the message stays undelivered until the record is
  fixed and the bridge started again.
- A "read" sent again after a restart of the service that is lost on the way is sent again at the
  next read of the channel, so the service may get it more than once.
- A bridge bound to another channel writes no reply for a post in a channel it does not serve.
- A body over 3000 bytes of text is cut in the post, with a line that names the message;
  `inbox pending` shows it whole.
- How the post escapes a mention and the block of literal text is not checked against the chat
  provider.
- A `connectors` that never ends holds the bridge; there is no time bound on a post or a read of
  the channel.
- One bridge serves one inbox and one channel; `inbox wake` and the bridge do not serve one
  inbox with one outbox file on a host at the same time.

## `inbox repost` can deliver a read note once more

A note stays in its sender's outbox file until it expires, 24 hours by default. `repost`
leaves alone an envelope the current start of the service already took
([first use](first-use.md#the-outbox-file)). After a restart the repost happens by itself, on
the sender's next command, and what follows holds for it as for a typed `inbox repost`. A note
is delivered a second time in 3 cases:

- The first `repost` after a restart of the service posts every kept envelope. The service
  keeps no record of a handled note, so a note its recipient read before the restart is posted
  and delivered again.
- A service that sends no `Inbox-Epoch` header (an older one) gives the client nothing to
  compare: every `repost` posts every kept envelope.
- An epochs file (`<outbox>.epochs`) that is lost or unreadable, or a line of it that is
  malformed or doubled, makes `repost` post the envelopes concerned. A command that has to
  change an epochs file it cannot read, or the file of the opened inbox (`<outbox>.opened`),
  exits 4, names the file in one line and leaves it as it is: `inbox post` and `inbox close`
  before they send anything, `inbox repost` and `inbox open` after the service answered. After
  an `inbox open` that ended so, the inbox is open, and `inbox close` ends with 4 and sends
  nothing until the file is repaired or removed.

A message the service still holds is answered `duplicate` and changes nothing.

## A collect whose answer is lost loses the answer

`inbox collect` takes the answer or the refusal of a message off the service: the service
removes the message and, in the same request, answers with what it held: the message as
`inbox outbox` shows it, with its state, who answered, the decided option, the reply and the
refusal reason. `collect` prints that and only then drops the envelope from the outbox file. When the answer does not reach the client, for
example because the connection breaks after the service removed the message, the reply or the
refusal is gone:

- a second `inbox collect` of that message answers `unknown-message`;
- the envelope stays in the outbox file until it expires. `inbox repost` leaves it alone until
  the service is restarted and then posts it again, so its recipient gets it once more.

The service keeps no receipt of a collected message.

## The generated API documents

The input `from` of `PostMessage` carries the example value `principal-262144` in the OpenAPI
documents and in the JSON Schema of `PostMessage`. It is a placeholder, not an address anybody
holds ([beyond10x/ess#804](https://github.com/beyond10x/ess/issues/804)).

## Signing, by design of version 1

From [`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md):
no revocation list (a short `valid_until` and an edit of the key file replace it), one operator
key with no rotation, no hardware-backed keys, no payload encryption. A compromised host can
sign as its own agents until their key entries lapse.
