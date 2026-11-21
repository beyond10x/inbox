# Design: the HTTP service, stage 2

Status: written before the code of `crates/inbox-server`, which implements it. Where this note and
`ess/domains/messaging.yaml` disagree, `ess/` holds; where it and `docs/design/signing.md`
disagree on a signature, `signing.md` holds.

`crates/inbox-server` is a library: a listener in front of the `server` feature of
`generated/rust/inbox`. The generated route table, JSON codecs and `dispatch` answer every command
and view; this crate writes no second route table and no second codec. It adds what the generated
surface leaves out by decision (concurrency, authentication) and what the specification leaves to
the service (the read rule, the wait, the expiry task, the limits). The binary `inbox serve` that
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

## The caller

The key id resolves to the principal of its key entry (`KeyRing::verify_request`). The caller is the
actor `inbox.messaging.Client` with that principal as its `principal` and, as its `inbox`, the
address with the same text: decision 5 of `docs/design/inbox.md` ("a caller carries a `principal`
and an `inbox` address, the same text in version 1"). No request over HTTP is the
`inbox.messaging.Sweeper`: `Expire`, `Due` and `Messages` are answered `403` with the generated
refusal `{"refused": "not granted", "actor": "inbox.messaging.Client"}`. Only the expiry task sends
`Expire`.

## The verifier

`RingVerifier` implements `inbox_core::SignatureVerifier` over the `KeyRing`, at the clock reading
step 3 authenticated the request at: `verifies` is `KeyRing::verify_envelope`, `authorizes` is
`KeyRing::check_delegation`. An envelope's own signature that does not verify is the
specification's `signature-invalid` (`502`, from `PostMessage` and `Answer`); it is not a `401`.

## The read rule

A row of a message view (every row that carries `to` or `from`) is answered only to the owner of
the inbox in `to` or to the sender in `from`: the row stays when its `to` is the caller's `inbox`
or its `from` is the caller's `principal`. The generated answer is filtered; no view is written a
second time. `Inboxes` rows carry neither (`address`, `owner`, `adapter`, `state`) and are answered
whole: the list of addresses a sender posts to. `Due` and `Messages` are not served to a client at
all (the generated grant, above).

## The wait

`GET /messaging/views/pending?inbox=<address>&wait=<seconds>`: `wait` is a whole number of seconds,
read by the generated query reader. The request is held open until the view, after the read rule,
has a row for the caller, or the wait ends, or the service stops; then it is answered as without
`wait` (an empty `rows` when it ended). The wait is cut to `max_wait`, which the embedding sets.
`wait=0` reads once. A `wait` that is not a whole number of seconds is `400`
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
(`Clock`), so a test moves time without sleeping; an instant is written as RFC 3339 in UTC,
truncated to the second, and held to the years 0000 to 9999 that RFC 3339 writes: a reading plus
an `answer_retention` past 9999 is written as `9999-12-31T23:59:59Z`.

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
