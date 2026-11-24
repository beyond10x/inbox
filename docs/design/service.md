# Design: the HTTP service, stage 2

Status: written before the code of `crates/inbox-server`, which implements it. Where this note and
`ess/domains/messaging.yaml` disagree, `ess/` holds; where it and `docs/design/signing.md`
disagree on a signature, `signing.md` holds.

`crates/inbox-server` is a library: a listener in front of the `server` feature of
`generated/rust/inbox`. The generated route table, JSON codecs and `dispatch` answer every command
and view; this crate writes no second route table and no second codec. It adds what the generated
surface leaves out by decision (concurrency, authentication) and what the specification leaves to
the service (the wait, the expiry task, the limits); the read rule, which the specification
cannot state either, is the core's, and the listener applies it. The binary `inbox serve` that
starts it is story:cli-client.

## One request, in order

Each step answers or passes the request on; the first refusal wins, and nothing after it runs.

| Step | Refusal | Status | Body |
|---|---|---|---|
| 1. Read, with the generated reader's limits | a malformed request, a chunked body, too many headers, a head over `MAX_HEAD`, a body over `MAX_BODY` | `400`, `411`, `431`, `413` | the generated reader's own |
| 1a. The request arrives in full within `request_timeout` of the accept | it has not | `408` | `{"refused": "request timeout"}` |
| 1b. The service is not stopping before the request has arrived in full | it is | `503` | `{"refused": "stopping"}` |
| 2. The body limit of the embedding | a body longer than `max_body` | `413` | `{"refused": "body too large", "limit": <max_body>}` |
| 3. Authenticate the request signature at the clock reading; for a request that names an inbox to act for (`Inbox-On-Behalf-Of`), then the delegation of the pair at the same reading | no valid request signature; an inbox named and no live delegation of the pair | `401` | `{"refused": "unauthenticated"}` |
| 4. A body that is JSON: `inbox_verify::duplicate_member` | an object, at any depth, carries a member name twice | `400` | `{"refused": "duplicate member", "member": "<dotted path>"}` |
| 5. A post: the message count, inside the backend's section | the store holds `max_messages` messages or more | `503` | `{"refused": "message limit", "limit": <max_messages>}` |
| 6. The generated `dispatch`, inside the backend's section | everything the generated surface answers: `404`, `405`, `400`, `403` (not granted), the declared outcomes, `501` | as generated | as generated |
| 6a. A command answer of the outcome `foreign-message`: the core's rename, `inbox_core::as_unknown` | the caller is neither party to the message (the model's guard) | `404` | the generated body with `outcome` `unknown-message` |
| 6b. `PostMessage`, `Answer`, `Refuse`, `MarkDelivered`, `MarkRead`: the deadline, at the clock reading inside the same section | a post whose `expires` is at or before the reading (`expired-on-arrival`, error `AlreadyExpired`); an answer, a refusal, a read mark or a delivery mark of a message in `Sent`, `Delivered` or `Read` whose stored `expires` is at or before the reading (`deadline-passed`, error `Expired`); a reply to such a message whose own `expires` is at or before the reading (`expired-on-arrival` of `Answer`, error `AlreadyExpired`) | `502`, the status the generated contract gives an external outcome | the generated body |
| 7. A view answer: the read rule | rows the caller may not see are removed | `200` | the generated body without them |

So an unauthenticated request never reaches the duplicate check's answer, the message limit or a
command: a request without a valid signature whose body carries a member twice is answered `401`.
A body that is not JSON passes step 4, and the generated reader refuses it in step 6 (`400`). A
body that carries a member twice is refused before decoding, because the generated decoders take
the first of two members and one signature would cover two texts (`signing.md`, the envelope).

`/openapi.json` and `/docs` are behind step 3 as well: every request is authenticated.

Step 3 reads the clock once per request, and that one reading serves the request signature, the
delegation of a named inbox, and the verifier of the envelopes in its body ("The verifier"); a
wait does not read it again ("The wait"). The delegation is looked up only after the request
signature verified, and its refusal is the `401` of a bad signature ("The request signature"), so
the steps after it run for a caller under a delegation exactly as for any other, with the caller
of "The caller".

Steps 1a and 1b bound the generated reader from outside it; the reader itself is neither copied
nor changed. Until a request has been read in full, its connection is watched: at
`request_timeout`, or when the service stops, a task of the service shuts the socket for reading.
A read blocked on it returns at once with the end of the stream, the reader refuses the request it
has not finished, and that refusal is answered `408` or `503` instead. The write side stays open
for the answer. This was chosen over a read timeout per read (it bounds each read, not the
request, and a blocked read does not see a change of it) and over a wrapper around the stream (the
reader takes the socket itself). A request read in full is in flight and no longer watched; a
request the reader refused stays watched while the service reads and drops what the caller still
sends, so the bound and the stop end that too.

## The request signature

The signed bytes are `signing.md`'s (the request object of five members, and of six for a
request that acts for an inbox). They travel in three headers, and a fourth for that request; a
header name is case-insensitive, as HTTP has it.

| Header | Carries |
|---|---|
| `Inbox-Key-Id` | the key id of the caller's key entry in the key file |
| `Inbox-Timestamp` | the RFC 3339 instant the caller signed at, as the text that was signed |
| `Inbox-Signature` | the Ed25519 signature over the request's signed bytes, 88 characters of canonical base64 |
| `Inbox-On-Behalf-Of` | only in a request that acts for an inbox: the address of that inbox, the text of the signed member `on_behalf_of` |

A request without `Inbox-On-Behalf-Of` is signed and answered as before the header existed.

`401 {"refused": "unauthenticated"}` answers each of: a header missing, a header sent twice, an
unknown or lapsed key, a timestamp that is not RFC 3339 or lies more than 60 s before or after the
clock reading (both bounds inclusive, `inbox_verify::KeyRing::verify_request`), a signature that is
not canonical or does not verify. The body says nothing more, so a caller learns nothing about the
key file from a refusal.

`Inbox-On-Behalf-Of` adds these, each answered with the same `401` (`signing.md`, "A request"):
the header sent twice (the rule of the other three); the header with an empty value, which is
never read as absent; a request signed without the member and sent with the header, or signed
with it and sent without; a header whose value is not the signed member; and an inbox no live
delegation of the pair covers at the reading of step 3, the signer's own address included. That
last refusal is no oracle on the key file: its status, its body and its headers are those of a bad
signature byte for byte (the head of an answer depends only on its status, its body and the
epoch), and the service writes no line for it, as it writes none for any request it answers,
refused or not. A caller that reaches the lookup already holds a live key of the key file; the
time the lookup takes is not made constant. `inbox_server::ON_BEHALF_OF_HEADER` names the header,
and `inbox_server::request_headers_for` signs a request with or without it.

The client sends the header on `--for <address>`: `inbox <command> --for <address>` puts the
address, as given, in `Inbox-On-Behalf-Of` of every request of the command and signs it as the
member `on_behalf_of`; without `--for` every request is the one of before the header, byte for
byte. An empty value, one with white space at either end (`char::is_whitespace`: the service's
reader trims every Unicode white space, not only a space or a tab) and one that holds a control
character (`char::is_control`, a line break among them) are wrong usage (exit 2) before any
request and before any file is written, since the service never honours one; nothing else of the
value is checked. Nor does the client make a request head with a line break in any of its parts,
whatever its caller passes. The key id of the secret key file is a header value of every request:
`inbox key new` refuses one with a control character (`char::is_control`) as wrong usage (exit 2)
before a key is made or a file exists, and a client command given a secret key file whose `key_id`
holds one ends as for every other defect of that file, exit 4 and one line that names the file,
before any connection (`keys::caller`). The host of `INBOX_URL` (or `--url`) is the value of
`Host`: one with a control character in the host or its port is wrong usage (exit 2), one line
that names `INBOX_URL`, before the host is handed to the resolver (`Service::from_url`). Each of
these lines shows the value escaped and at most its first 64 characters (`output::quoted_cut`).
A client command refuses in this order: a wrong `--for`
(exit 2), a wrong `INBOX_URL` (exit 2), then the secret key file (exit 4). The
address is the caller's inbox in the command: the default of `--inbox` of `pending` and `wait`,
and the default `on_behalf_of` of the reply of `answer` (an `--on-behalf-of` given stays as given,
and `--authority` is the caller's to give). A request the service refuses because no live
delegation of the pair covers the inbox is printed as any `401`, with the same exit status 1 and
line. Every client command takes it (story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
added `open`, `close`, `wake` and `bridge`; `bridge` takes it with `--authority`,
`docs/design/bridge.md`, "The key of the bridge"). The state a command keeps on this host for an
inbox is keyed by the inbox it acts for, the address of `--for`: the line of the inbox in the
record of the opened inbox, the wake lock, the record of the chat posts. The kept envelopes stay
keyed by their sender, the key's principal.

The signed `path` is the request target as the generated reader splits it: the path, then `?` and
the query when the query is not empty. The reader keeps no empty query, so a target that ends in a
bare `?` is signed without it. The signed body is the exact bytes after the head.

The service keeps **no record of requests it has seen**, and `inbox-verify` keeps none: a signed
request may be sent again inside its 60 s window, and is answered again. Posts are idempotent by
`message_id` and `dedup_key`; a repeated `Answer` is `already-settled`.

## The epoch

Every answer the service writes carries the response header `Inbox-Epoch`
(`inbox_server::EPOCH_HEADER`): 32 lower-case hex digits, made once when the listener is bound
(`Server::bind`) from 16 bytes of the OS random source. Every answer of one start carries the same
value, and a new start makes a new one, so a caller that compares the value with the one an
earlier answer carried tells whether the service was started again since. The memory backend
loses every message on a restart; that is what the client's `inbox repost` reads it for: it posts
again only the kept envelopes that the epochs file beside its outbox file (`<outbox>.epochs`, one
line `<message id>\t<epoch>` per envelope the service was said to hold) records at another epoch,
or at none. An envelope records the epoch of an answer that accepted its post or refused it as
`duplicate` (the message id is stored), right after that answer, so a `repost` ended between two
posts keeps the epochs of the posts that arrived; `duplicate-key` records none, since the service
stored nothing of that envelope. When the epochs file cannot be written, `repost` still posts
every envelope, prints its report and exits 4 with the local failure. Every client command
reads the epoch for a reconnect too: after its own request, when the answer names an epoch
other than one the client recorded, it opens the caller's own inbox again with the adapter of
its last `inbox open` (under `--for` the inbox acted for, from its own line of the file, with the
header; the caller's own inbox is opened by the next command without `--for`; the reposts of a
command under `--for` carry the header too; the file `<outbox>.opened` holds it with the epoch
of the answer that said the inbox is open; `inbox close` removes it before it sends the close and puts it back when
the close fails in transport or is refused other than `UnknownInbox`), and posts the kept
envelopes again as `repost` does, at that epoch and without asking for it, when one of them is
recorded at another epoch or marked unsent (the epochs file holds `<message id>\tnone` for
it): each envelope not recorded at that epoch once, but the one the command itself posted.
`inbox post` writes the mark with the envelope, in the same change under the lock, before its
request is sent; the epochs file is replaced before the outbox file, so a process ended between
the two leaves a mark of an envelope the outbox file does not keep, which nothing posts and the
next write drops. The mark goes only when the answer says that this request, sent again
unchanged, is refused again (`400`, `401`, `413`, a refusal of the model that keeps the
envelope), or when it records an epoch or drops the envelope. An answer of the listener's own
that says the request was not taken (`408`, any `5xx`, such as `503` while the service stops)
counts as a failure in transport, as does `UnknownInbox`: the mark stays, in `inbox post` as in
the reconnect's repost, and the sender's next command posts the envelope again. A second command of the sender that runs while a post is still on its way posts that
envelope too; the post on its way is then answered `Duplicate` with the outcome `duplicate`
for its own message id, which `inbox post` takes as arrived (exit 0). The reconnect's repost
leaves an envelope in the state a typed post would leave it for the same answer, but that
`duplicate-key` keeps the envelope without a mark and without an epoch, so no reconnect posts
it again and a typed `inbox repost` still does once the key is free. An envelope kept by a
version before this one has no line in the epochs file and starts no reconnect; a typed
`inbox repost` posts it. When the reconnect's open reaches the service after an `inbox close`
of the owner took the record, it sends `CloseInbox` again, writes no record, says on standard
error that the inbox was closed meanwhile, and its `open` member is
`{"outcome":"closed-meanwhile","opened":…,"closed":…}`; a failure of that close is one line on
standard error. After that close it reads the record again: when an `inbox open` of the owner
wrote one meanwhile, it sends `OpenInbox` once with that record's adapter, leaves the record as
it is, and adds `"reopened":…` to the member. When the reconnect's open finds another record
than the one it started from (an `inbox open` of the owner after a close, while the open was on
its way), it leaves that record as it is, sends nothing more, says on standard error that the
inbox was opened by another command meanwhile, and its `open` member is
`{"outcome":"opened-meanwhile","opened":…}`; neither is a failed step. More than one typed
`inbox open` or `inbox close` of the owner while a reconnect runs can leave the record and the
service apart until the next typed `inbox open` or `inbox close`. A typed `inbox open` and a
typed `inbox close` that run at once end in the order the service and the outbox lock take
them, which the owner chose by starting both: the record can stay while the service holds the
inbox closed, and the reconnect after a restart then opens it again. The answers of these requests start no other reconnect. A step that fails
leaves the command's output and exit status as they are and is named in one line on standard
error; it writes only what the same answer to a typed command writes (a repost refused
`UnknownInbox`, answered `408` or `5xx`, or failed in transport marks an envelope at no epoch
unsent, one refused for good loses the mark), so the next command tries again. Without `--json` the command's own
lines are written before the reconnect starts; with `--json` its one document is written once
the reconnect ended, with the member `reconnected`. A file beside the outbox file that is no
regular file or is over its bound is read as holding nothing by a command that only reads it,
named in one line on standard error, and blocks no command. Two commands that do it at once
post a message id twice, and the service answers the second `duplicate`.

A command that has to change the epochs file or `<outbox>.opened` and cannot read it (no
permission to read, not UTF-8, over its bound, no regular file) changes neither that file nor
the outbox file (but that `inbox repost` drops from the outbox file, in the same run, the
envelopes that expired and those the service refused for good), names the file and the reason
in one line on standard error and exits 4, as a change of an unreadable outbox file does: the
other entries of the file are not lost to a file written with the change's own entry only. That line is the only one of the run that names the
file: the warning "read as holding nothing" of a read of the same file in the same run is not
written (the read of the kept envelopes of `inbox repost` before the failure, held back until
its posts are done; the reconnect's read after it). A command that only reads the file warns as
before. `inbox post` and `inbox close` change it before their request and send nothing. `inbox
open` changes it only after the service answered: the service has opened the inbox, the outcome
is printed, and the record is not written; until the file is repaired or removed, `inbox close`
then ends with 4 and sends nothing, so the inbox stays open. `inbox repost` changes it only
after each answer to a post (an epoch recorded, a mark taken off, or a mark written for an
answer `408`, `5xx` or `UnknownInbox`): every kept envelope (the unreadable file gives none of
them an epoch) is posted and its answer reported, none is recorded, and the command exits 4
after the report; a failure in transport ends the run with 3 as before, and a local failure
before it is named in one line. The reconnect's open changes it after the service opened the
inbox, a failed step that leaves the command's own exit status; its repost does not start from
an unreadable epochs file, which marks no envelope unsent. A file that does not exist is read
as empty. A line of the file that is UTF-8 but is no entry (another form, or a message id or an
inbox named twice) gives no epoch or record and is left out when the file is written again, as
before.

The value is not made from the `Clock` of `Config` (a clock a test sets does not move between two
starts) and carries no time and no secret. It is written in the one function that writes the head
of an answer, so the answers of every status carry it: the refusals of step 1 to 5 included, `401`
among them, which a caller without a key gets. Such a caller learns that the service was started
again, and nothing else. A connection dropped without an answer (a thread that cannot be started)
gets none. The generated `http::Response` and the generated `http::write` are not changed.

## The caller

The key id resolves to the principal of its key entry (`KeyRing::verify_request_for`). The caller is the
actor `inbox.messaging.Client` with that principal as its `principal`. Its `inbox` is, for a
request without `Inbox-On-Behalf-Of`, the address with the same text as the principal, as before
the header existed; for a request with it that step 3 admitted, the address the header names,
while `principal` stays the signer (decision 5 of `docs/design/inbox.md`). The listener makes the
two attributes once per request, after step 3, and hands both to the core for every request.

Which attribute decides each command and view the grant of `inbox.messaging.Client` holds
(`ess/domains/messaging.yaml`, lines 259-274), and what an agent gets there in a request that
names the inbox of a person it holds a live delegation of. "Beyond" marks what the story asks
not for (it asks for `Pending`, `mark-delivered`, `read`, `answer`):

| Command or view | Decided by | Source | Under a live delegation, naming the person's inbox |
|---|---|---|---|
| `open-inbox` | `inbox`: `not-owner` when `address != caller.inbox`; `principal`: `opened` sets `owner: {caller: principal}` | lines 341-348 | Beyond. Opens the person's address: an address no inbox has is created with the agent as `owner`; a closed one is reopened with the agent's adapter and keeps its owner; an open one is `already-open`. Without the header the agent gets `not-owner` (`403`), as before |
| `close-inbox` | `principal`: `not-owner` when `owner != caller.principal`; `inbox`: `not-owner` when `address != caller.inbox`, the service's rule (below) | lines 372-375 | Beyond. The agent closes the person's inbox it opened only under the header, while its delegation is live. Without the header its `inbox` is its own address, so the person's inbox answers it `not-owner` (`403`); under the header so does its own inbox. An inbox the person opened answers it `not-owner` |
| `post` | `principal`: `not-sender` when `from != caller.principal` | lines 418-420 | Beyond. A post with the header is answered as without it: its `from` must be the agent, and its `to`, `on_behalf_of` and `authority` are checked as in any post. No post is made "as" the person's inbox |
| `mark-delivered` | `inbox`: `not-recipient` when `to != caller.inbox`; both: `foreign-message` when `to != caller.inbox` and `from != caller.principal` | lines 551-562 | Asked for. Marks a message of the person's inbox `delivered` |
| `read` | as `mark-delivered` | lines 590-601 | Asked for. Moves a request or a decision to `Read` (`read`), removes a note (`handled`) |
| `answer` | `principal`: `not-sender` when `from != caller.principal`; `inbox`: `not-recipient`; both: `foreign-message`. `authority-invalid` reads no attribute: the verifier checks the reply's own `authority` against its `from` and `on_behalf_of` | lines 649-681 | Asked for. A reply `from` the agent, `on_behalf_of` the person, `authority` a live delegation of that pair, to a message of the person's inbox, is `answered` with `answered_by: principal`. A reply `on_behalf_of` anybody else is `authority-invalid`, the service's rule (below). Version 1 has no scope narrower than the pair, so the agent answers any message of the person's inbox in the person's place, one it sent itself included, and the record says the person answered. A sender that must not answer its own question holds no delegation of the person it asks |
| `refuse` | `inbox`: `not-recipient`; both: `foreign-message` | lines 761-772 | Beyond. Refuses a message of the person's inbox with a reason |
| `collect` | `principal`: `not-sender` when `from != caller.principal`; both: `foreign-message` | lines 839-850 | Beyond, and nothing new: the agent collects what it sent, with the header or without it. A message the person sent to another inbox is `unknown-message` to it; one the person sent to its own inbox is `not-sender` |
| `expire` | none: not granted to a client | lines 276-282 | `403` `not granted`, as for every request |
| `Inboxes` | none: its rows carry no `to` and no `from`, the read rule keeps them all | lines 951-961 | As without the header |
| `Pending` | both, by the read rule over the rows its `inbox` parameter selects: a row stays when `to` is `caller.inbox` or `from` is `caller.principal` | lines 963-988; `inbox_core::readable` | Asked for. Every row of the person's inbox in `Sent` or `Delivered`; of another inbox, only the rows the agent sent |
| `Unanswered` | both, by the read rule | lines 990-1015 | Beyond. The requests and decisions of the person's inbox in `Read` |
| `Outbox` | both, by the read rule over the rows of its `sender` parameter | lines 1017-1046 | Beyond. The rows the agent sent, as without the header, and the rows of any sender to the person's inbox, in every state, with the reply or the refusal they carry. A row the person sent to another inbox stays hidden |
| `Settled` | both, by the read rule | lines 1048-1077 | Beyond. As `Outbox`, the answered and refused rows only |
| `Thread` | both, by the read rule | lines 1079-1108 | Beyond. The rows of the thread to the person's inbox or from the agent |
| `Replies` | both, by the read rule | lines 1110-1138 | Beyond. As `Thread`, the answered rows only |
| `Due`, `Messages` | none: not granted to a client | lines 1140-1197 | `403` `not granted`, as for every request |
| `Pending` with `wait` | both, as `Pending`; the wait holds the caller of its start | "The wait" | Beyond. Waits on the person's inbox |
| `Pending` with `undelivered=1` | `inbox`: the `409` `{"refused": "inbox not open"}` reads whether `caller.inbox` is open; the rows as `Pending` | "The wait"; `own_inbox_open` | Beyond. Answers `409` while the person's inbox is not open |

What the service answers where the table says "beyond", and why. The model decides each by the
attribute in the second column, and the service answers as the model (Open 1 B of the story: a
live delegation makes its agent the owner of the inbox of its principal), with two rules of its
own over a body: the `on_behalf_of` of a reply under the header, and the address of a
`close-inbox` (both below). The other choice left
is whether the listener refuses a route under the header; it is named below where it was
weighed, and refused each time.

- Opening and reopening the person's inbox: as the model (the story's Open 4, A). Closing it: as
  the model, and the service's rule over a close (below). The cost: an inbox the agent opens
  before the person does is owned by the agent, so the person's own key gets `not-owner` when it
  closes it, until a new start of the service drops it; after a new start, whoever opens the
  address first owns it. After the delegation lapses that inbox stays open: posts to it are
  accepted and the person reads it with its own key, but nobody closes it (the agent's requests
  with the header are `401`, the agent without the header and the person are `not-owner`) until
  the service starts anew, or a new delegation of the pair is in the key file at a new start. The person's own `open` of an inbox
  the agent opened is `already-open`, and a reopen by either sets the adapter it names. Safe
  because no caller gets an inbox it could not reach before without the operator's signed entry:
  the agent can only open an address a live delegation names. The alternative, an owner set from
  `caller.inbox`, changes `ess/`.
- `refuse`: as the model. The cost: a refusal carries no signed envelope, so the sender cannot
  tell a refusal of the agent from one of the person; that holds today for the person's own
  refusals too. Refusing (B: the listener answers `refuse` with the header `401`) would make one
  route an exception to the one rule of the header.
- `post`: as without the header. Options: (A) answered as without the header; (B) a post with the
  header refused (`401`) or answered `400`. Taken: A, so the header is checked per request and not
  per route, and a post with it can do nothing a post without it cannot.
- `collect`: nothing beyond what the agent's principal gets without the header.
- `Unanswered`, `Outbox`, `Settled`, `Thread`, `Replies`: what the person's own key reads of the
  person's inbox, which the agent needs to answer: safe as the agent may mark, read and answer
  those messages anyway. The agent does not get the person's own outgoing messages.
- The wait and `undelivered=1`: the wait of a delegation's agent on the person's inbox, and its
  `409` while that inbox is not open. A delegation that lapses during a wait is "The wait"'s.
- The agent's own inbox: in a request with the header the caller's `inbox` is the person's, so the
  agent's own inbox is no longer its inbox in that request: its rows from others leave its views,
  and its marks, reads, answers and refusals of them are `unknown-message` (`foreign-message`
  renamed) or `not-recipient`. It handles its own inbox in requests without the header.

**Under the header, a reply speaks for the person of the inbox it answers in.** In a request that
carries `Inbox-On-Behalf-Of`, an `answer` whose reply envelope has an `on_behalf_of` other than
the principal with the text of the header's value is refused `authority-invalid`: `502`
`{"outcome":"authority-invalid","published":[],"error":"inbox.messaging.NoAuthority"}`, the
generated answer of that outcome (`generated/rust/inbox/src/server/inbox_service.rs`, lines
662-671), and nothing changes. A reply `on_behalf_of` the agent itself is refused so as well, with
or without an `authority`, and so is one `on_behalf_of` another principal the agent holds a live
delegation of. Without the rule such a reply would reach the person's message and record
`answered_by: principal` for a principal the sender did not ask, which no request could do before
the header, since only the person's key reached that inbox.

The rule answers at the place of `authority-invalid` in the order of `Answer`: the generated
`Answer` asks the request's context for that external branch after `not-sender`, `no-authority`,
`already-expired` and `signature-invalid` and before it reads the addressed message, whether or
not the reply names an `authority` (`generated/rust/inbox/src/behaviour.rs`, lines 148-167), and
the context the listener builds for a request with the header answers it `true` when the reply's
`on_behalf_of` is not the person, before it asks the verifier about the `authority`. So the
model's order holds, and each broken member of a reply has one outcome: `from` not the agent is
`not-sender`; an `on_behalf_of` other than `from` with no `authority` is `no-authority`; an
`expires` at or before `created` is `already-expired`; a signature that does not verify is
`signature-invalid`; an `on_behalf_of` other than the person is `authority-invalid`; an `authority`
that is no live delegation of the pair (agent, person) is `authority-invalid`. A reply that breaks
two of them is answered by the first in that order: `from` and `on_behalf_of` both wrong is
`not-sender`; `on_behalf_of` wrong and the signature bad is `signature-invalid`; `on_behalf_of`
wrong in a reply to an id no message carries, or to a message of another inbox, is
`authority-invalid`, before the message is read, so it tells nothing about the message. Refusing
before the generated `dispatch` instead would answer `authority-invalid` ahead of the 4 refusals
before it, and a case could not state one outcome per broken member.

The rule is the service's and not the model's: the model knows of the inbox a request acts for
only `caller.inbox`, and binds no member of a reply to it, so it cannot say that the reply speaks
for the owner of that inbox. The conformance run over HTTP sends no header, and the run against
the core builds no context with one, so neither is touched by it. Without the header nothing
changes: a reply `on_behalf_of` another principal with a live `authority`, in the signer's own
inbox, is answered as before.

Everything else of the reply stays the model's. The request admits no particular delegation, so
a reply `on_behalf_of` the person whose `authority` names another live delegation of the pair
than any the request was admitted by is `answered`, and one whose `authority` names a lapsed
delegation of the pair is `authority-invalid`, though another live one admitted the request.

**A close closes the caller's own inbox.** A `close-inbox` whose address is an inbox the service
holds and is not the caller's `inbox` is answered `not-owner`: `403`
`{"outcome":"not-owner","published":[],"error":"inbox.messaging.NotRecipient"}`, the generated
answer of the model's own `not-owner`, and nothing changes. The model checks only `owner !=
caller.principal`, and makes the agent the `owner` of an inbox it opens under the header (Open 4
A), so without the rule the agent would close that inbox also without the header, after its
delegation lapsed too, again after every reopen by the person, and every post to the person
would then be refused until a new start. With the rule the agent closes it only under the header
while its delegation is live, and under the header it does not close its own inbox, which is
not the caller's `inbox` in that request. Without the header and for the caller's own inbox
nothing changes: a caller without the header has its own address as `inbox`, so the rule refuses
it only addresses the model refuses it too, except the inbox an agent opened for a person.

The rule answers at the model's step for `not-owner`, after `unknown-inbox`: an address no inbox
has is `unknown-inbox` (`404`) with the header and without it, as the model answers it, so the
rule tells a caller nothing about which inboxes exist that the model did not tell it before; a
body the route refuses (`400`) is refused as before. The listener decides it inside the
backend's section, before the generated `dispatch`, from the decoded body and the row it reads
there (`closes_another_inbox` in `crates/inbox-server/src/service.rs`), and not in
`RequestContext`, because the generated `CloseInbox` asks its context no branch, only
`try_caller_principal`. Cases: `crates/inbox-server/tests/delegation.rs` and
`crates/inbox-server/tests/security_delegation.rs`.

The other commands under the header carry no member of this kind. A `post` is the agent's own
envelope: its `from` is the agent (`not-sender`), its `on_behalf_of` and `authority` are checked as
in a post without the header, against the delegation the `authority` names, and the inbox the
request acts for enters neither the post nor the stored message, so the recipient reads whose
word the post is from the envelope alone, as before the header. `refuse` takes a message id and a
reason and names no principal; the refused message records the reason, not who refused (above).
`collect` takes a message id and is decided by the caller's `principal` alone, so the header
changes nothing there. `mark-delivered` and `read` take a message id only. `open-inbox` names an
address, which must be the person's, and records the agent as owner (Open 4 A, above);
`close-inbox` names an address, which must be the person's too (the rule over a close, above).
The rule of a reply under the header is `RequestContext` of `crates/inbox-server/src/service.rs`,
a context around the core's `ServiceContext`; the core is not changed by it. Cases:
`crates/inbox-server/tests/delegation.rs`.

The caller is the sender: a `post` or an
`answer` whose envelope's `from` is not the caller's `principal` is the specification's
`not-sender` (`403`, `NotSender`), answered before every other refusal of the command and
storing nothing. The rule is the model's (`ess/domains/messaging.yaml`); the listener holds no
copy of it. So a principal that sends another principal's signed envelope under its own request
signature is refused, whether or not the first message is still held.

A caller that is neither the owner of the inbox in `to` nor the sender in `from` of a stored
message gets the model's `foreign-message` from `mark-delivered`, `read`, `answer`, `refuse` and
`collect` (decision 5 of `docs/design/inbox.md`). Step 6a is the one place where this service's
answer to a command differs from the generated one: the generated surface answers that outcome
`403` under its own name, and the service answers it with the status and the body of
`unknown-message`, byte for byte what an id no message carries gets from the same route: `404`
`{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}`. So from
these 5 commands a third principal cannot tell a message between others from no message, in any
state of the message. A `post` to an address no inbox has, or into a closed inbox, answers alike
for the id of another sender's message and a free id: `409` with the body of `unknown-inbox` or
of `inbox-closed`, or of `duplicate-key` when a live message of the caller carries the post's
dedup key. The row refusals of a post come in the order `unknown-inbox`, `duplicate`
(the sender's own stored message), `duplicate-key`, `inbox-closed`, `id-taken` (decision 6 of
`docs/design/inbox.md`); `id-taken` is `409`
`{"outcome":"id-taken","published":[],"error":"inbox.messaging.MessageIdTaken"}`, the status of
the other 4. Into an open inbox a principal that holds a message id still learns whether it is
live (`id-taken` against `posted`), and can take a free id first
(`story:post-does-not-tell-a-message-id-exists`; `story:message-id-per-sender` removes it). ESS
0.96.0 can give
two outcomes one answer only through an `ess-http/1` binding this repository does not have. The
rename is the core's, once: `inbox_core::as_unknown`, which `inbox_core::handle` and
`handle_with_events` apply too, so an embedding of the core answers a third principal alike. An
embedding may hand in a client without an inbox or a principal, or with a context that cannot
read one; the model cannot decide such a command on a stored message without the attribute and
refuses it as an unmet obligation, and `handle` and `handle_with_events` answer that refusal of
the 5 commands as `unknown-message` too, with no event, so an id is answered alike whether a
message carries it or not (`handle` in `crates/inbox-core/src/lib.rs` states the rule). The
listener hands in both attributes for every client and never meets it. The listener calls
`as_unknown` on each command answer and writes a renamed answer back with the status `404`;
it holds no copy of the rename. The rename changes the outcome and nothing else: it compares no
caller with no message, and who is a party is decided in the model. The headers of the answer depend only on the status, the body
and the start of the service (the epoch, above), and the outcome publishes nothing and changes no
row. Like every answer to a request that is not a
`GET`, it wakes the held-open waits, the same way an `unknown-message` answer does.
The sender still gets `not-recipient` (`403`) from the 4 commands of the owner and the owner
`not-sender` (`403`) from `collect`. Over HTTP the conformance run reads `unknown-message` where a
scenario expects `foreign-message`; the scenario that expects it,
`a-third-principal-finds-no-message`, is unsupported there (it has a step as the Sweeper), and
`crates/inbox-server/tests/foreign_message.rs` witnesses the answer. The run against the core
judges that scenario through `inbox_core::handle_as_specified`, the core's entry without the
rename.

`collect` answers `collected` with `200` and the settled record under `response` (the
generated contract; `Collect` declares a typed response): every member of the message's
`Outbox` row, `expires` and the reply among them, read by `inbox-core` from the row it deletes,
inside the one `Exclusive` section of that request. No answer or refusal can land between that
read and the delete, and the client prints what this answer carries. A client that gets a
success answer that is not `collected` with the settled record of the message it collected,
decoded as the declared response of `Collect` with each member once (a 0.2.0 service answers
`202` without `response`), exits 3 and keeps its outbox line. Every refusal
of `collect` (`unknown-message`, the renamed `foreign-message`, `not-sender`, `not-settled`)
carries no `response` and changes no row. An answer lost after the delete loses the record: the
service keeps no receipt, and a second `collect` is `unknown-message`.

No request over HTTP is the
`inbox.messaging.Sweeper`: `Expire`, `Due` and `Messages` are answered `403` with the generated
refusal `{"refused": "not granted", "actor": "inbox.messaging.Client"}`. Only the expiry task sends
`Expire`.

## The verifier

`RingVerifier` implements `inbox_core::SignatureVerifier` over the `KeyRing`, at the clock reading
step 3 authenticated the request at: `verifies` is `KeyRing::verify_envelope`, `authorizes` is
`KeyRing::check_delegation`. An envelope's own signature that does not verify is the
specification's `signature-invalid` (`502`, from `PostMessage` and `Answer`); it is not a `401`.

## The deadline

The context of each command (`inbox_core::ServiceContext`) carries the service's clock as an
`inbox_core::CommandClock`. `PostMessage`, `Answer`, `Refuse`, `MarkDelivered` and `MarkRead`
read it inside the backend's section, after the checks of the caller and the party, so a
principal that is no party to a message still gets `unknown-message` and its sender
`not-recipient`, and no other command runs between the reading and the change. The reading is
written as the expiry task writes `Due`'s and `Expire`'s `now`
(`format_reading`: RFC 3339, UTC, with the fraction of a second the clock holds), and every
comparison is of instants at full precision, so a message is due at its `expires` exactly, as
for `Expire`: a message due at `…:00.500Z` is due at a reading of `…:00.700Z`. So a late answer
and a sweep, in either order and at any reading, leave the message the same, decided on silence
with its `default_on_silence` or removed. A refused answer or refusal changes nothing and
publishes nothing. No member of a request replaces the reading.

`read` and `mark-delivered` of a message in `Sent`, `Delivered` or `Read` whose stored `expires`
is at or before the reading are refused `deadline-passed` (`Expired`) the same way: no row
changes and no event is published, and the message stays in its state until the expiry task
removes it or decides it on silence. So a due note read before the sweep and one read after it
both end with the sweep's `MessageExpired` to the sender, never with `MessageHandled`. A message
already answered or refused keeps the answers `already-read` and `already-delivered`. The client
sends `read` only for `inbox read <id>` and `mark-delivered` never: `inbox pending` and
`inbox wait` read the `Pending` view, which lists a due message until the sweep, and mark
nothing. A due row does not end a wait on `Pending` (see "The wait"), so a wake loop of `wait`
and `read` does not get the row it cannot read back at once, again and again, until the sweep.

A reply to a message in `Sent`, `Delivered` or `Read` is checked against the clock as a post is: a
reply whose own `expires` is at or before the reading is refused `expired-on-arrival`
(`AlreadyExpired`) and the message stays as it was. An answer to a message already answered or
refused is `already-settled` whatever its own `expires`, so a client that sends its answer again
learns that the first was taken.
Stored, its `expires` would be the message's, and the next sweep at that reading would remove
the answer before its sender could collect it, and with it the option a decision's silence takes.
`Refuse` carries no `expires` of its own: a refused message keeps the message's.

## The read rule

A row of a message view (every row that carries `to` or `from`) is answered only to the owner of
the inbox in `to` or to the sender in `from`: the row stays when its `to` is the caller's `inbox`
or its `from` is the caller's `principal`. The rule is the core's, `inbox_core::readable`, which
`inbox_core::handle` applies for an embedding; the listener runs the generated `dispatch` and
hands the view's answer to that same function, so it holds no copy of the rule and both entries
give one caller the same rows. The generated answer is filtered; no view is written a second
time. `Inboxes` rows carry neither (`address`, `owner`, `adapter`, `state`) and are answered
whole: the list of addresses a sender posts to. `Due` and `Messages` are not served to a client at
all (the generated grant, above); the expiry task reads `Due` as the `Sweeper`, which the rule
does not bind. ESS 0.97.3 refuses a view filter over the caller (`docs/design/inbox.md`, what the
specification cannot say).

## The wait

`GET /messaging/views/pending?inbox=<address>&wait=<seconds>`: `wait` is a whole number of seconds,
read by the generated query reader. The request is held open until the view, after the read rule,
has a row for the caller that is not due, or the wait ends, or the service stops; then it is
answered as without `wait`: every row, due ones included (an empty `rows` when it ended with
none). A row is due when its `expires` is at or before the service's clock, read after the view
and compared as `MarkRead` and `MarkDelivered` compare it (`inbox_core::due`): its owner can
neither read nor mark it, and only the expiry task takes it out of `Pending`. A wait with only
due rows sleeps as one with none; a row that becomes due while it sleeps wakes nothing. A sweep
of the expiry task that sent an `Expire` counts as an answered command, so a wait reads its view
again after it. A wait that ends at its bound or at the stop reads the view once more and
answers that read, what `pending` lists then, not the read it slept on. The wait is cut to
`max_wait`, which the embedding sets.
`wait=0` answers at once. A `wait` that is not a whole number of seconds is `400`
`{"refused": "wait is not a whole number of seconds"}`. `wait` on another view is ignored, as the
generated reader ignores any key a view does not declare.

`&undelivered=1` (`inbox wake` sends it, `inbox wait` and `inbox pending` do not) changes three
things of a read of `Pending`, with a `wait`, with `wait=0` and without a `wait` (which is then a
wait of no time):

- The answer holds only the rows that can end the wait: the caller's rows in `Sent` that are not
  due at the clock reading after the view was read. Rows in `Delivered` and due rows are left
  out; the other members of the view are as they were. So only such a row ends the wait: a row in
  `Delivered` sleeps as a due one does, and a `MarkDelivered`, an answered command, wakes the
  wait, which reads its view again and sleeps on when no such row is left. A wait that ends at its
  bound or at the stop answers its last read cut the same way (an empty `rows` when none is left).
- The read is never held for a caller whose own inbox is not open at the service: before each
  read of the view, the one at the bound included, the service looks the caller's inbox up, and
  while it is not stored or not `Open` the answer is `409` `{"refused": "inbox not open"}` at
  once. A close is an answered command, so it wakes a held read, which then gets that answer. A
  new start of the service holds no inbox, so the first read of a wake there is answered so at
  once, with the new start's `Inbox-Epoch`, and the client's reconnect can open the inbox again
  before it waits (`docs/design/wake.md`).
- Nothing else: the bound, the stop and the epoch are those of a wait without it.

Without the parameter every answer is byte for byte as before it existed. A value other than `1`
(`0`, `true`, an empty one) is `400` `{"refused": "undelivered is not 1"}`, also without `wait`;
the key given twice is `400` as `wait` given twice is. It is part of the request's target, so it
is signed as every other part of the query is. The refusal is the listener's own, as `400`, `408`
and `413` are; it is no outcome of the model, so no generated route changes.

The waiting request does not hold the backend's section: it reads the view inside the section,
leaves it, and sleeps until a command has been answered on another connection, then reads again.
Each connection has its own thread, so a waiting request does not hold another request; it holds
one of `max_connections` while it waits.

A wait is one request: step 3 authenticated it once, at the clock reading of its start, and each
read of the view during the wait is made for the caller of that start, with no second check of
the key or of a delegation. So a delegation that lapses while a wait of its agent on the person's
inbox sleeps (its `valid_until` passes during the wait) ends at the latest when that wait ends,
`max_wait` after its start at the most (60 s by default): until then a read of the wait still
answers the person's rows, and the next request that names the inbox is refused `401`. A key
that lapses during a wait is held the same way. With `undelivered=1` and the header, the inbox
whose state answers `409` is the person's (the caller's `inbox`, "The caller").

## The expiry task

A thread reads the clock every `sweep_interval`, reads `Due` at that reading as the
`inbox.messaging.Sweeper`, and sends `Expire` for each due message with `now` set to the reading
and `keep_until` to the reading plus `answer_retention`. Each view and each `Expire` goes through
`inbox_core::handle`, which runs it inside the backend's section; the core decides what `Expire`
does (removed, decided on silence, or not due any more). The clock is a parameter of the library
(`Clock`), so a test moves time without sleeping; the reading and `keep_until` are written as
RFC 3339 in UTC with the fraction of a second the clock holds (`format_reading`), and held to the
years 0000 to 9999 that RFC 3339 writes: a reading plus an `answer_retention` past 9999 is written
as `9999-12-31T23:59:59Z`. A request timestamp is written to the second (`format_instant`).

`inbox serve --clock-start <instant>` is the conformance run's flag, hidden from `--help` and
from the command-line reference: the clock starts at that instant and moves on by the system
clock's elapsed time since the start (`StartedClock`), so a suspend of the host moves it on as
it moves the system clock, and a step of the system clock moves it by the step (back to `start` at
the most); without it, the system clock. The run uses it to serve before the
fixed instants of the suite's envelopes. When it is set, `inbox serve` writes one line on
standard error at the start: `inbox serve: the service's clock starts at <instant>, not at the
system clock`.

`inbox serve --clock-file <path>` is a flag of a debug build only (`cargo build`, the build the
tests and `task acceptance` run), hidden from `--help` and from the command-line reference; a
release build (`cargo build --release`, `cargo install`) has no such flag and refuses it as wrong
usage, exit 2. It is wrong usage beside `--clock-start` too. The file holds one RFC 3339 instant
and at most one line end after it; the service's clock reads it each time it asks for the time
(`FileClock` of `crates/inbox-cli/src/clock_file.rs`), and does not move on by itself. A test
that starts `inbox serve` as a process moves the clock past an `expires` by writing the file
(written beside it and renamed over it, so no reading meets half an instant), and sleeps until no
instant passes. It gives no more than `--clock-start`, which sets the clock to any instant, and
the service still binds a loopback address only. At the start `inbox serve` writes one line on
standard error: `inbox serve: the service's clock reads the file <path>, at <instant> now, not the
system clock`. It fails closed:

| The file | At the start | While it serves |
|---|---|---|
| missing, or cannot be opened (no permission) | refused before binding, exit 4, `--clock-file <path>: <the OS error>` | the process ends at once, exit 4 |
| no regular file (a directory, a FIFO, a device); opened with `O_NONBLOCK`, so a FIFO without a writer does not hold the open | refused, exit 4, `it is no regular file` | the process ends at once, exit 4 |
| empty | refused, exit 4, `it is empty` | the process ends at once, exit 4 |
| longer than 64 bytes, or not one RFC 3339 instant | refused, exit 4, `it does not hold one RFC 3339 instant` | the process ends at once, exit 4 |
| an instant before the reading before it | — | the process ends at once, exit 4, `it went back to <instant>, before <instant>, the reading before it` |

While it serves, the end is one line on standard error, `inbox serve: the clock file <path>:
<cause>: stopping`, and exit 4: the request or the sweep that asked for the time is not answered
and changes nothing, nothing is decided at a reading of the system clock, and nothing is taken as
not due. No line repeats a byte of the file. The client commands sign with the system clock and the
service takes a request signed within 60 s of its reading (step 3), so a test keeps the file within
60 s of the system clock: the acceptance check sets it to the system clock's second at the start,
posts a request whose `expires` is 30 s after the system clock just before the post (the post's
`created` is the system clock, and `expires` must come after it), and moves the file 1 s past that
`expires`; the runner ends a check at 60 s.

## The limits

Each is set by the embedding (`Config`); the defaults are this crate's.

| Limit | Default | What happens beyond it |
|---|---|---|
| `max_connections` | 64 | the listener accepts no further connection until one ends; the kernel's backlog holds them |
| `request_timeout` | 10 s | a request not arrived in full by then, counted from the accept, is answered `408` and its connection closed, which frees its slot, step 1a |
| `max_body` | 1 MiB, the generated `MAX_BODY`, which is also the most it can be | `413`, step 2 |
| `max_messages` | 10 000 | a post is answered `503` before the command runs, step 5 |
| `max_wait` | 60 s | a longer `wait` is cut to it |
| `sweep_interval` | 10 s | — |
| `answer_retention` | 24 h | — |

The generated reader's own limits hold too: a head of at most `MAX_HEAD` bytes and 100 headers,
1 s without a byte while a request is read (`READ_TIMEOUT`), and 10 s for each write call that
cannot go on (`WRITE_TIMEOUT`, the socket's send timeout). That bound holds per call, not per
answer: a write call that has moved some bytes when it passes returns them, and the next call has
its 10 s again, so a caller that reads a large answer slowly, or stops reading in the middle of
it, can hold its connection, and the stop, longer than 10 s.

Any duration may be set, up to `Duration::MAX`: a deadline too far away to add to the clock is no
deadline (a `wait` then ends with a row or the stop, the expiry task sleeps until the stop, a
request is bound by the reader's gap bound only), and nothing panics on the arithmetic.

## Binding and stopping

The service binds a **loopback address only** in this stage and refuses any other before binding.
Port 0 binds an ephemeral port, and the bound address is handed back. It serves until told to stop:
it then accepts nothing more, ends every wait, cuts off every connection that has not sent its
request in full (answered `503 {"refused": "stopping"}`, step 1b, without waiting for
`request_timeout`), lets every request in flight (read in full) finish, and stops the expiry task
and the request timeout task. `Running::stop` and `Server::run` return then.

## TLS

With `Config::tls` set (`inbox serve --tls-file <file>`), the service answers every connection
over TLS 1.3 with that certificate, and over nothing else: a plain request to it is not read
(the handshake fails on its first bytes and the connection is closed). It still binds a loopback
address only. Without it, the service speaks plain HTTP as before.

| Item | What it is |
|---|---|
| The certificate | self-signed, X.509 version 3, over an Ed25519 key; subject and issuer `CN=inbox serve`; valid from its making with no end (`99991231235959Z`). `inbox tls new --out <file>` makes the key and the certificate, writes both into one file with mode 0600 (never overwriting a file, as `inbox key new`) and prints the pin; `inbox serve` refuses a file that grants a permission to its group or to others, or that is not such a file, before binding (exit 4) |
| The pin | the SHA-256 of the certificate's SubjectPublicKeyInfo, in the base64 of `signing.md` ("Encoding on the wire"): 44 characters. No command renews a certificate: each `inbox tls new` makes a new key, so a new pin |
| The library | rustls 0.23 with its `ring` provider, TLS 1.3 only (`inbox_server::tls`) |

The generated reader takes the socket itself (`http::read(&mut BufReader<TcpStream>)`), so TLS
cannot sit between the socket and the reader, and `generated/` is not changed. The service ends
TLS in front of it: for each accepted connection it makes a pair of connected loopback sockets
(the end it accepts is checked to come from the end it connected, so no third process is one end
of it), and two threads carry the bytes: one reads TLS records from the caller's socket, answers
the handshake and writes the plain bytes to the pair, one reads the answer from the pair and
writes it to the caller as TLS records, then closes TLS and shuts the caller's socket for writing
only. The socket is shut for reading after the connection's thread is done, so the rest of a
body past the bound, still arriving when its `413` is written, is read and dropped for the
bounded time of `http::linger`, and the caller reads the `413`, as over plain HTTP. A caller
still sending its body when a cut answers (`408` at the request timeout, `503` at the stop) gets
a failed write instead, over TLS as over plain HTTP: the cut has shut the connection for reading
(the caller's socket, or over TLS the plain end of the pair), nothing drains the rest, and the
caller's next bytes are reset. An answer cut short (the service's end of the pair broke off,
or a write to the caller failed) ends without `close_notify`, so the caller's read of it fails
and it is never taken for a whole answer; over plain HTTP the same caller gets a reset. The
connection's thread
serves the plain end exactly as a plain connection, so every bound of "One request, in order"
holds unchanged: `request_timeout` counts from the accept and covers the handshake (a request not
arrived in full by then is answered `408`), the stop cuts a request still arriving (`503`), the
body limits answer `413`, and `max_connections` counts the connection until both threads have
ended. A caller that does not finish the handshake is cut off at the same bound; it gets no
answer, since none can reach it. Cases: `crates/inbox-server/tests/tls.rs`.

The client: `INBOX_URL` is `https://<host>:<port>` and `INBOX_TLS_PIN` (`--tls-pin`) the pin; the
host must be a loopback address, as for `http://`. An `https://` URL without a pin, and a pin
beside an `http://` URL, are wrong usage (exit 2) and nothing is sent: a client given a pin never
falls back to plain HTTP. The client completes the handshake before it writes a byte of its
request (`inbox_server::tls::connect`), and accepts the certificate whose public key hashes to the
pin and no other, by no other check: no name, no validity time and no authority is consulted, and
the handshake signature is verified with that key. A certificate the pin does not name ends the
handshake: exit 3, `the certificate's public key does not match the pin`, and no request line,
header or body sent; like every transport failure, a post keeps its envelope in the outbox file.
Cases: `crates/inbox-cli/tests/tls.rs`.

What the pin does not protect against:

- Anyone who can read the TLS file holds the key and can pose as the service to every client
  given the pin. The file's mode 0600 and its owner are the only guard.
- There is no revocation. A key that leaked stays accepted by every client given its pin until
  that client is given another pin.
- Each `inbox tls new` makes a new key: a service started with it has a new pin, and every
  client needs that pin before it reaches the service again.

## Left out of stage 2

- A listener on a second host: epic:two-hosts-and-wake.
- A replay record for requests (above).
- Keep-alive and pipelining: one request per connection, as the generated reader reads it.
- Reloading the key file while serving: the embedding builds a new service.
