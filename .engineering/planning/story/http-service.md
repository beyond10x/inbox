---
format: aep.planning-md/3
id: story:http-service
kind: story
status: implemented
title: '`inbox serve`: the HTTP service over the core'
summary: The routes of `generated/openapi`, an authenticated caller, the long-poll wait and the expiry task.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:core-and-memory-backend
- depends_on: story:signature-verifier
scope:
- confidence: cited
  path: crates/inbox-server
- confidence: cited
  path: docs/design/service.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:04Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-21T11:42:35Z", actor: "agent:inbox", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":12,"verification":1}}}
---
## Outcome

`crates/inbox-server` is a library that puts a listener in front of the generated `server`
feature. The route table, the JSON codecs and the `dispatch` of `generated/rust/inbox` answer
every command and view (`POST /messaging/commands/<wire>`, `GET /messaging/views/<wire>`); this
crate writes no second route table and no second codec. It adds what the generated surface leaves
out by decision, concurrency and authentication, and what the specification leaves to the
service:

- The caller. Every request carries a request signature (the request functions of
  story:signature-verifier). The key id resolves to a principal, and that principal is the
  caller's `principal` and `inbox`. A request without a valid request signature is answered `401`
  with `{"refused": "unauthenticated"}` before any command runs. `SignatureInvalid` is the
  specification's answer for an envelope's own signature and comes from `PostMessage` and
  `Answer` only.
- The verifier port. An adapter implements `inbox-core`'s `SignatureVerifier` trait over
  `inbox-verify`.
- Duplicate members. A body in which an object carries a member name twice, at any depth, is
  answered `400` before any command runs.
- Reads. A view row is answered only to the owner of the inbox in `to` or to the sender in
  `from`; `Due` and `Messages` are not served to clients.
- Wait. `GET /messaging/views/pending` with a `wait` parameter is held open until the view has a
  row or the wait ends.
- Expiry. A task reads the clock every sweep interval, lists the due messages and sends `Expire`
  for each, with `keep_until` set to the reading plus the answer retention.
- Limits, set by the embedding binary (`Config`): connections, the time a request may take to
  arrive in full (`request_timeout`, default 10 s, answered `408`), request size, and a message
  count beyond which a post is answered `503` before the command runs.
- Stopping. `stop` answers every connection that has not sent its whole request with `503` and
  `{"refused": "stopping"}`; a request read in full is still answered.

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
interval and the answer retention, is story:cli-client. It needs one flag more than this story
named at acceptance: `request_timeout`.

The request signature travels in 3 headers (`Inbox-Key-Id`, `Inbox-Timestamp`,
`Inbox-Signature`); a request whose timestamp lies more than 60 s from the clock reading is
answered `401`. The verifier (`inbox-verify`) reads no clock and keeps no record of requests it
has seen: the listener passes the instant of each check, and a signed request can be sent again
inside its 60 s window.

As built in wave 3 of stage 2:

- The listener does not call the entry `inbox_core::handle`, which the story named before
  dispatch. It runs the generated `dispatch` inside the backend's `Exclusive` section, over
  `inbox_core::system` with a clone of the backend and a `ServiceContext` for that request's
  caller. The generated `dispatch` answers the HTTP statuses; `handle` answers outcomes only.
  One command at a time per store holds as before.
- The duplicate member check is the host operator's decision of 2026-11-21 on finding 4 of
  review-result:adversary-signature-verifier-pass-1: the generated reader keeps the first of two
  members, so without the check one signature covers two texts. The call is
  `inbox_verify::duplicate_member`, after authentication and before decoding;
  story:refuse-duplicate-members is closed as covered by this story.
- `request_timeout` and the `503` on stop answer findings 1 and 2 of adversary pass 1: a slow
  unauthenticated connection held a connection slot, and `stop`, without end. The generated
  reader bounds the gap between two bytes only; a task of the service shuts the read side of a
  connection at its deadline or at stop, and the reader itself is unchanged.
- Adversary pass 1 returned 8 findings: 5 introduced and fixed, 3 that were in the model before
  the unit. Pass 2 returned 2 notes on missing cases, closed by the adversary's own cases
  (review-result:adversary-http-service-pass-1, -pass-2).
- The 3 findings that were in the model are backlog drafts, and each keeps its case in
  `tests/adversary.rs` with an ignore attribute naming the story: story:post-from-is-the-caller
  (a recipient sends a sender's signed envelope again under its own request signature and it is
  posted once the first message is gone), story:foreign-message-id-answers-as-unknown and
  story:dedup-key-scope. The host operator decided on 2026-11-21 that the guard of the first
  goes into `ess/` and no copy of it is written in the listener, and that the 3 drafts land
  before a listener is bound outside loopback, before a NATS backend wave, before a deployment
  and before use by a real session.
- `cargo test -p inbox-server` runs 38 cases, and 3 more are ignored.

## Scope

- `crates/inbox-server/` (new)
- `docs/design/service.md` (new): request signing, the service's own refusals, read rule, wait,
  expiry task, limits, binding and stopping
