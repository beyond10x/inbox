---
format: aep.planning-md/3
id: story:bytes-after-a-request-end-without-a-reset
kind: story
status: draft
title: A caller that sends bytes after its request reads its answer without a reset
summary: Over plain HTTP a caller that sends bytes after its whole request gets its answer and then a connection reset; over TLS it ends cleanly.
revision: 3
---
## Problem

Over plain HTTP, a caller that sends bytes after its whole request while the service still
holds the request (`GET /messaging/views/pending?wait=1`) gets its whole answer (148 bytes in
the run) and then a connection reset, so a read to the end of the stream fails with
`ConnectionReset` after the answer. The service closes a socket whose receive buffer still
holds unread bytes (`crates/inbox-server/src/listener.rs:308`).

Found by adversary pass 1 on story:the-service-speaks-tls-with-a-pinned-certificate
(review-result:adversary-the-service-speaks-tls-with-a-pinned-certificate-pass-1, finding 9),
reproduced on the commit before that story. Over TLS the same caller gets its answer and a
clean end. The client `inbox` sends one request per connection and nothing after it, so it
does not meet this; a pipelining client would.


## Also seen (adversary pass 2 on the same story)

review-result:adversary-the-service-speaks-tls-with-a-pinned-certificate-pass-2, findings 2 and
7, both over plain HTTP and so older than TLS:

- A caller still sending its body when the service cuts the request (`408` at the request
  timeout, `503` at the stop) gets a failed write (broken pipe) and never reads the status: the
  cut shuts the socket for reading and the linger then shuts it for writing, so the caller's
  next bytes are reset. Over TLS the caller sees the same. The `413` of a body past the bound
  is read.
- A caller that never reads a 7.8 MB answer held the stop for 31 s in one run, measured once:
  the 10 s bound of a stalled write holds for each write call, not for the whole answer.

Open before it is accepted, in addition: whether a cut drains what still arrives within a bound
so its status is read; whether the write of one answer gets a bound of its own.

## Acceptance

A caller that sends bytes after its whole request reads its whole answer and then the end of
the stream, without a reset, over plain HTTP as over TLS (state transition: the read to the
end fails with a reset before, and returns the answer after). Or the design says that
bytes after a request are a protocol error and names the answer.

## Open before it is accepted

- Whether bytes after one request are drained within the linger bound, or refused.

## Scope

- `crates/inbox-server/src/listener.rs`, `crates/inbox-server/tests/`
- `docs/design/service.md` ("One request, in order")
