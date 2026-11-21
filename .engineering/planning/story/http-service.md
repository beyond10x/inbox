---
format: aep.planning-md/3
id: story:http-service
kind: story
status: active
title: '`inbox serve`: the HTTP service over the core'
summary: The routes of `generated/openapi`, an authenticated caller, the long-poll wait and the expiry task.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:core-and-memory-backend
- depends_on: story:signature-verifier
scope:
- confidence: inferred
  path: crates/inbox-server
- confidence: inferred
  path: docs/design/service.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`crates/inbox-server` is a library that puts a listener in front of the generated `server`
feature. The route table, the JSON codecs and the transport-free `handle` of
`generated/rust/inbox` answer every command and view (`POST /messaging/commands/<wire>`,
`GET /messaging/views/<wire>`); this crate writes no second route table and no second codec. It
adds what the generated surface leaves out by decision, concurrency and authentication, and what
the specification leaves to the service:

- The caller. Every request carries a request signature (the request functions of
  story:signature-verifier). The key id resolves to a principal, and that principal is the
  caller's `principal` and `inbox`. A request without a valid request signature is answered `401`
  with `{"refused": "unauthenticated"}` before any command runs. `SignatureInvalid` is the
  specification's answer for an envelope's own signature and comes from `PostMessage` and
  `Answer` only.
- The verifier port. An adapter implements `inbox-core`'s `SignatureVerifier` trait over
  `inbox-verify`.
- Reads. A view row is answered only to the owner of the inbox in `to` or to the sender in
  `from`; `Due` and `Messages` are not served to clients.
- Wait. `GET /messaging/views/pending` with a `wait` parameter is held open until the view has a
  row or the wait ends.
- Expiry. A task reads the clock every sweep interval, lists the due messages and sends `Expire`
  for each, with `keep_until` set to the reading plus the answer retention.
- Limits, set by the embedding binary: connections, request size, and a message count beyond
  which a post is answered `503` before the command runs.

## Acceptance

`cargo test -p inbox-server` sends two correctly signed requests to the server on a loopback
socket and is answered `posted` for an envelope as it was signed and `SignatureInvalid` for the
same envelope with its `body` changed after signing, where before this story the generated
binary exits at startup with unmet obligations and no code answers an HTTP request.

## Context

The service listens on a loopback address in this stage; TLS and a second host are
epic:two-hosts-and-wake. The read rule and the wait are not expressible in ESS 0.96.0 (no
per-caller view filter, no blocking read; `docs/design/inbox.md`, "What the specification cannot
say"), so this story checks them with its own tests and states them in `docs/design/service.md`.

The binary `inbox serve` that starts this library, with the flags for the limits, the sweep
interval and the answer retention, is story:cli-client.

The request signature carries its timestamp; a request older than 60 s is answered `401`. The
header names are written into `docs/design/service.md` before code.

Since wave 2: the listener calls the entry `inbox_core::handle` once per request, with a clone
of the backend and a `ServiceContext` over that request's caller; the entry runs one command at a
time per store. The verifier (`inbox-verify`) reads no clock and keeps no record of requests it
has seen: the listener passes the instant of each check, and a signed request can be sent again
inside its 60 s window.

Added by the host operator on 2026-11-21 (decision on finding 4 of
review-result:adversary-signature-verifier-pass-1): the listener runs the check
`inbox_verify::duplicate_member` on every request body after parsing and before decoding, and
refuses a body in which an object carries a member name twice, at any depth, before any command
runs. The generated reader keeps the first of two members, so without the check one signature
covers two texts. One call and one case; `docs/design/service.md` names the refusal.

## Scope

- `crates/inbox-server/` (new)
- `docs/design/service.md` (new): request signing, the two service refusals, read rule, wait,
  expiry task
