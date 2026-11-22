---
format: aep.planning-md/3
id: story:the-service-speaks-tls-with-a-pinned-certificate
kind: story
status: draft
title: The service answers over TLS and the client checks the certificate against its pin
summary: inbox serve takes a certificate and answers over TLS on loopback; the client talks https and refuses a certificate that does not match its pin.
relations:
- decomposes: epic:two-hosts-and-wake
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/inbox-cli/Cargo.toml
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: inferred
  path: crates/inbox-cli/src/keys.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/src/serve.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/tls.rs
- confidence: cited
  path: crates/inbox-server/Cargo.toml
- confidence: cited
  path: crates/inbox-server/src/config.rs
- confidence: cited
  path: crates/inbox-server/src/listener.rs
- confidence: cited
  path: crates/inbox-server/tests/main.rs
- confidence: inferred
  path: crates/inbox-server/tests/tls.rs
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
revision: 4
---
## Outcome

`inbox serve` answers over TLS with a certificate it is started with, and the client talks to it
at an `https://` URL and checks the certificate against a pin it was given before it sends a
request. The listener stays on a loopback address in this story;
story:the-service-binds-a-reachable-address lets it bind elsewhere.

## Acceptance

A client given the pin of the certificate gets `posted` from an `inbox serve` started with that
certificate on a loopback address, and a client given another pin exits 3 and the recipient
`Pending` stays empty (state transition: a post that travelled as plain HTTP travels only over
TLS whose certificate the client checked).

## Context

Stage 3 of `docs/design/inbox.md` ("Stages", item 3, line 367) and the epic Outcome: "The service
is reached over TLS with a certificate the clients pin". `docs/design/service.md` "Left out of
stage 2" (line 284) names TLS for this epic. Today the client sends plain HTTP/1.1 over a
`TcpStream` (`crates/inbox-cli/src/client.rs:2`) to `http://<host>:<port>` only
(`client.rs:45-80`). The listener hands each accepted socket to the generated reader, whose
signature takes that concrete type: `pub fn read(reader: &mut
std::io::BufReader<std::net::TcpStream>)` (`generated/rust/inbox/src/server/http.rs:178`, called
at `crates/inbox-server/src/listener.rs:272`). `service.md` "One request, in order" (lines
41-50) bounds that reader from outside by shutting the socket for reading, and chose this over a
wrapper around the stream because "the reader takes the socket itself". So TLS cannot sit
between the socket and the generated reader without one of the ways of Open 1, and
`generated/` is never edited by hand (`AGENTS.md`, "Rules").

The certificate and its pin are configuration of the service and of the client, as the key file
and the pinned operator public key are (decision 4 of `docs/design/inbox.md`: keys and
delegations are not entities of this domain). This story adds no entity to `ess/`; the
specification says of transport only `reached_by: network` (`ess/components.yaml:33`).

Tests hold the budget of the folder rule "Tests stay fast": no test over 60 s, the test step
within 120 s, no network request beyond loopback. A certificate a case needs is made in process
or committed as a fixture, never by a call to an outside tool per test.

## Domain relations

None: the story touches no entity of `ess/domains/messaging.yaml`.

## Open

1. How TLS reaches the generated reader, which takes a `BufReader<TcpStream>` (`http.rs:178`).
   (A) the service ends TLS in a thread of its own and hands the plain bytes to the generated
   reader over a loopback socket pair, so the reader and the watch stay as they are; (B) an issue
   in the `ess` repository asks for a generated reader over any `Read + Write` stream, and this
   story waits for that release; (C) the service stays plain HTTP behind a TLS-ending tunnel or
   proxy, the client checks its pin against that proxy, and this story changes the client only.
   Default: A (no change of `generated/`, no wait on another repository).
2. Where the certificate and its pin come from. (A) `inbox tls new` makes a key pair and a
   self-signed certificate, writes the secret with mode 0600 and prints the pin, which the
   operator gives each client host; (B) the operator key signs the certificate public key as an
   entry of the key file, and the client checks that signature instead of a pin; (C) a
   certificate authority the operator runs, and the pin is that authority. Default: A.
3. Which TLS library becomes a dependency of `inbox-server` and `inbox-cli`, and whether plain
   `http://` stays allowed on a loopback address beside `https://`. (A) rustls with the `ring`
   provider, plain HTTP kept for loopback; (B) rustls, plain HTTP removed; (C) another library.
   Default: A.
4. What the pin is a digest of. (A) the SHA-256 of the certificate SubjectPublicKeyInfo, in the
   base64 of `docs/design/signing.md` ("Encoding on the wire"), so a renewed certificate over the
   same key keeps its pin; (B) the SHA-256 of the whole certificate. Default: A.

## Scope

Order (parallel safety): the code files of `crates/inbox-cli` that 5 stories of the epic edit,
`src/client.rs`, `src/main.rs`, `src/reference.rs`, `generated/cli/inbox.json` and the module list
of `tests/main.rs`, are edited in one order: this story, then
story:the-client-reconnects-after-a-restart, story:inbox-wake-returns-on-a-post,
story:the-terminal-adapter-shows-a-post and story:the-service-binds-a-reachable-address, each
with a `depends_on` edge on the one before. This story is first and waits for none.

Code:

- `crates/inbox-server/src/listener.rs` (accept, `connection`, `write`): cited
- `crates/inbox-server/src/config.rs` (the certificate in `Config`): cited
- `crates/inbox-server/Cargo.toml`, `crates/inbox-cli/Cargo.toml`, `Cargo.lock` (the TLS
  dependency): cited
- `crates/inbox-cli/src/client.rs` (`Service::from_url`, `connect`, the pin): cited
- `crates/inbox-cli/src/serve.rs`, `crates/inbox-cli/src/main.rs` (flags of `inbox serve` and
  of the client, the URL `inbox serve` prints): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json` (`task cli-reference`): cited
- `crates/inbox-server/tests/tls.rs` and `crates/inbox-cli/tests/tls.rs` (new), one `mod` line each
  in `crates/inbox-server/tests/main.rs` and `crates/inbox-cli/tests/main.rs`: inferred (the
  directories and `main.rs` files are cited)
- `crates/inbox-cli/src/keys.rs` (with Open 2 A: `inbox tls new` beside `inbox key new`):
  inferred

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/service.md`: a new section `## TLS`, and the first bullet of "Left out of stage 2"
  (the word TLS): cited
- `website/docs/first-use.md`: a new section `## Serve over TLS` after "Keys and the service":
  cited
- `website/docs/status.md`: one row appended to the table of "The service", and in "Missing" the
  words "and TLS" of the bullet "A listener outside loopback, and TLS.": cited
- `website/docs/limits.md`: in "The service", the words "no TLS" of the bullet "Loopback only, no
  TLS.": cited
- `docs/checks.md`: in "On `main`", the rows "The HTTP listener" and "The binary `inbox`"; in "Open
  limits and their stories", the words "no TLS" of the bullet on line 86: cited
- `AGENTS.md`: the Layout rows of `crates/inbox-server/` and `crates/inbox-cli/`, the Commands
  rows of `inbox serve` and of the client commands: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it (`AGENTS.md`, its Layout
  row): cited
