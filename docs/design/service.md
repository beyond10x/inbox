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
| 3. Authenticate the request signature at the clock reading | no valid request signature | `401` | `{"refused": "unauthenticated"}` |
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

The signed bytes are `signing.md`'s (the request object of five members). They travel in three
headers; a header name is case-insensitive, as HTTP has it.

| Header | Carries |
|---|---|
| `Inbox-Key-Id` | the key id of the caller's key entry in the key file |
| `Inbox-Timestamp` | the RFC 3339 instant the caller signed at, as the text that was signed |
| `Inbox-Signature` | the Ed25519 signature over the request's signed bytes, 88 characters of canonical base64 |

`401 {"refused": "unauthenticated"}` answers each of: a header missing, a header sent twice, an
unknown or lapsed key, a timestamp that is not RFC 3339 or lies more than 60 s before or after the
clock reading (both bounds inclusive, `inbox_verify::KeyRing::verify_request`), a signature that is
not canonical or does not verify. The body says nothing more, so a caller learns nothing about the
key file from a refusal.

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
every envelope, prints its report and exits 4 with the local failure.

The value is not made from the `Clock` of `Config` (a clock a test sets does not move between two
starts) and carries no time and no secret. It is written in the one function that writes the head
of an answer, so the answers of every status carry it: the refusals of step 1 to 5 included, `401`
among them, which a caller without a key gets. Such a caller learns that the service was started
again, and nothing else. A connection dropped without an answer (a thread that cannot be started)
gets none. The generated `http::Response` and the generated `http::write` are not changed.

## The caller

The key id resolves to the principal of its key entry (`KeyRing::verify_request`). The caller is the
actor `inbox.messaging.Client` with that principal as its `principal` and, as its `inbox`, the
address with the same text: decision 5 of `docs/design/inbox.md` ("a caller carries a `principal`
and an `inbox` address, the same text in version 1"). The caller is the sender: a `post` or an
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

The waiting request does not hold the backend's section: it reads the view inside the section,
leaves it, and sleeps until a command has been answered on another connection, then reads again.
Each connection has its own thread, so a waiting request does not hold another request; it holds
one of `max_connections` while it waits.

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
1 s without a byte while a request is read (`READ_TIMEOUT`), 10 s for a stalled write
(`WRITE_TIMEOUT`).

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

## Left out of stage 2

- TLS, and a listener on a second host: epic:two-hosts-and-wake.
- A replay record for requests (above).
- Keep-alive and pipelining: one request per connection, as the generated reader reads it.
- Reloading the key file while serving: the embedding builds a new service.
