---
format: aep.planning-md/3
id: story:the-service-binds-a-reachable-address
kind: story
status: draft
title: inbox serve binds an address outside loopback only behind TLS
summary: With a certificate inbox serve binds an address outside loopback, without one it refuses before binding; the client goes outside loopback only over https with a pin.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:the-service-speaks-tls-with-a-pinned-certificate
- informed_by: story:message-id-per-sender
- depends_on: story:the-terminal-adapter-shows-a-post
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/src/serve.rs
- confidence: inferred
  path: crates/inbox-cli/tests/bind.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: cited
  path: crates/inbox-server/src/config.rs
- confidence: cited
  path: crates/inbox-server/src/listener.rs
- confidence: inferred
  path: crates/inbox-server/tests/bind.rs
- confidence: cited
  path: crates/inbox-server/tests/main.rs
- confidence: cited
  path: docs/checks.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 6
---
## Outcome

`inbox serve --listen` takes an address outside loopback when it is started with a certificate
(story:the-service-speaks-tls-with-a-pinned-certificate), and refuses it without one before
binding. The client sends to an address outside loopback only at an `https://` URL with a pin.
So a session on a second host reaches the one service.

## Acceptance

`inbox serve --listen` with an address outside loopback binds when a certificate is given and
exits 2 before binding when none is, and the client accepts an `INBOX_URL` outside loopback only
as `https://` with a pin (state transition: every address outside loopback refused by both, to
accepted behind TLS). The gate holds the refusals only: a bind outside loopback without a
certificate, and a client URL outside loopback that is plain `http://` or has no pin. The clause
"binds when a certificate is given" has as its evidence the operator run on two hosts, after the
operator order (Open 1), recorded on this story: a client on the second host gets `posted` from
the service on the first (Open 5 C; the same run is the evidence that
story:the-stage-3-acceptance-checks Open 1 C records for the epic). A bind on `0.0.0.0` of the
shared host is itself a listener outside loopback and needs that order, so no case of the gate
makes one.

## Context

`Server::bind` refuses any address that is not loopback before binding
(`crates/inbox-server/src/listener.rs:69-80`; `docs/design/service.md`, "Binding and stopping",
line 275), and the client refuses an `INBOX_URL` that resolves outside loopback
(`crates/inbox-cli/src/client.rs:71-75`). `AGENTS.md` "Rules" (lines 184-190): the gate is open
for principals of one operator, that operator own sessions and hosts, and "A release, a
listener outside loopback and a deployment each still need the operator own order". This story
is the change that order is about: it is a draft, and it lands only after the order is recorded
on it.

Open limits it meets: finding 2 of `website/docs/limits.md` (lines 35-41): a signed request is
accepted again inside its 60 s window (`docs/design/service.md`, "The request signature", lines
73-75; `docs/design/signing.md`, "The request window" and "Left out of version 1", lines 189-191
and 201-202); no story is filed for it (`docs/checks.md`, lines 72-73). Finding 1 (lines 16-33):
story:message-id-per-sender, which the gate requires before principals of different operators
share one service (`AGENTS.md`, lines 185-188), not before a second host of one operator.

## Domain relations

None: the story touches no entity of `ess/domains/messaging.yaml`.

## Open

1. The operator order for a listener outside loopback (`AGENTS.md`, line 189). No default: the
   story waits for it.
2. How the second host reaches the listener. (A) the service binds an address of the operator
   own network, and the client on the second host connects to it over TLS; (B) the service stays
   on loopback and a tunnel (an SSH port forward, a private network) carries the client to it,
   so the client reaches a loopback address of its own host and this story shrinks to nothing;
   (C) both, chosen per deployment. Default: A.
3. Whether finding 2 (replay inside the 60 s window) is closed before a listener outside
   loopback. (A) not first: under TLS a request cannot be read on the way, so a replay needs a
   party that already holds the request, and the limit stays stated for principals of one
   operator; (B) first: a story ahead of this one gives the service a record of the request
   signatures it accepted inside the window and refuses one seen again; (C) a shorter window.
   Default: A.
4. Whether story:message-id-per-sender is closed first. (A) not first, as the gate says for
   principals of one operator; (B) first, because a second host widens who can post. Default: A.
5. How a case proves a bind outside loopback while the test step makes no network request beyond
   loopback. (A) bind the unspecified address `0.0.0.0:0` and connect through `127.0.0.1`: no
   packet leaves the host, but the port is open on every interface of the shared host while the
   case runs, which is itself a listener outside loopback and needs the operator order of Open 1;
   (B) a network namespace per case, which needs privileges the shared host and CI may not grant;
   (C) the cases hold the refusals and the rule that decides them, and the bind outside loopback
   is shown by the operator run on two hosts named in the Acceptance. Default: C.

## Scope

Order (parallel safety): this story is the last to edit `crates/inbox-cli/src/client.rs`
(`Service::from_url`), `src/main.rs`, `src/reference.rs`, `generated/cli/inbox.json` and the
module list of `crates/inbox-cli/tests/main.rs`: after story:the-terminal-adapter-shows-a-post
(edge `depends_on`), which edits them after story:inbox-wake-returns-on-a-post,
story:the-client-reconnects-after-a-restart and story:the-service-speaks-tls-with-a-pinned-certificate.
It waits for the operator order anyway (Open 1). It edits `crates/inbox-server/src/listener.rs`
and `config.rs` after the TLS story (edge `depends_on`).

Code:

- `crates/inbox-server/src/listener.rs` (`Server::bind`): cited
- `crates/inbox-server/src/config.rs` (the documentation of `address`, line 16): cited
- `crates/inbox-cli/src/client.rs` (`Service::from_url`, lines 45-80): cited
- `crates/inbox-cli/src/serve.rs`, `crates/inbox-cli/src/main.rs` (the help of `--listen`): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json`: cited
- `crates/inbox-server/tests/bind.rs` and `crates/inbox-cli/tests/bind.rs` (new), one `mod` line
  each in `crates/inbox-server/tests/main.rs` and `crates/inbox-cli/tests/main.rs`: inferred (the
  directories and `main.rs` files are cited)

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/service.md`: the section "Binding and stopping", and the first bullet of "Left out
  of stage 2" (the listener on a second host): cited
- `website/docs/limits.md`: the opening lines on loopback (lines 10-12), and in "The service" the
  words "Loopback only" and "No second host" of their two bullets: cited
- `website/docs/status.md`: in "Missing", the rest of the bullet "A listener outside loopback, and
  TLS." and the bullet "A second host."; in "Planned", item 1: cited
- `website/docs/first-use.md`: the warning box (lines 11-16): cited
- `docs/checks.md`: in "Open limits and their stories", the rest of the bullet on line 86: cited
- `AGENTS.md`: in "Rules", the bullet of the gate (lines 184-190); the Commands rows of
  `inbox serve` and of the client commands: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
