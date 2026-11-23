---
format: aep.planning-md/3
id: story:the-service-speaks-tls-with-a-pinned-certificate
kind: story
status: implemented
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:14Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:14Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-23T01:45:28Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":19}}}
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

## Decisions

The 4 open items were taken at their defaults (approval-record:stage-3-six-stories-on-default):
1 A (the service ends TLS and hands plain bytes to the generated reader over a loopback socket
pair), 2 A (`inbox tls new`), 3 A (rustls with the `ring` provider, plain `http://` kept on
loopback), 4 A (the pin is the SHA-256 of the SubjectPublicKeyInfo).

Taken while building:

- The TLS code is `crates/inbox-server/src/tls.rs`, and `inbox-cli` calls it, so there is one
  pinned verifier and `inbox-cli` gains no direct dependency.
- The certificate is X.509 v3 over an Ed25519 key, its DER written by the crate and read back by
  rustls-webpki; no crate that makes certificates is added. It holds no extension, and the
  pinned client reads no validity time.
- The key and the certificate share one file (JSON, mode 0600, read with the mode check of the
  key file).
- TLS 1.3 only on both sides, no session tickets, no resumption, no early data: every
  connection checks the certificate.
- A post refused by the pin keeps its envelope in the outbox, as every transport failure does.
- `inbox serve` prints the `https://` URL and not the pin; `inbox tls new` prints the pin.
- The client keeps `https://` to loopback hosts; another host is
  story:the-service-binds-a-reachable-address.
- A caller that never finishes the handshake is cut at the request timeout and gets no `408`:
  no answer can reach it (`docs/design/service.md`, TLS).
- The pin does not protect against a reader of the TLS file; there is no revocation; no command
  renews a certificate. The documents say so.
- An answer that is cut ends without `close_notify`, so a TLS caller never takes a short answer
  for a whole one (adversary pass 2, finding 1).
- A caller still sending its body when a cut answers (`408` at the request timeout, `503` at
  the stop) gets a failed write, over TLS as over plain HTTP; only the `413` of a body past the
  bound is read while the body still arrives. The design says so. The 2 cases of adversary
  pass 2 that asserted the status were rewritten by the coordinator's decision into cases that
  hold the same class of ending over both transports and a free slot after it. Changing the
  cut so that its status is read is story:bytes-after-a-request-end-without-a-reset.
- No third adversary pass: round 3 changed 2 conditions of `outbound`, and a mutant of each was
  run. One of the 2 mutants is red in 1 of 10 tries of its case only.
- `CHANGELOG.md` has its entry under "Unreleased" on this branch, written by the coordinator,
  as the entries of the other stories since 0.3.1 are.

Scope as built, beyond the list below: `crates/inbox-server/src/tls.rs` and `src/lib.rs`;
`crates/inbox-server/tests/adversary_tls.rs`, `adversary_tls2.rs`, `security_tls.rs` and
`crates/inbox-cli/tests/adversary_tls.rs` (the reviewers' cases); `README.md` (2 lines that
said "no TLS"); the exit status table of `website/docs/first-use.md`; `CHANGELOG.md`.

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

## Measured

- Failing first: 7 cases of `crates/inbox-cli/tests/tls.rs` red on the unchanged client
  (`error: unrecognized subcommand 'tls'`; `INBOX_URL … is not http://<host>:<port>`), green
  after.
- 44 new cases: `inbox-server` 123 (92 before), `inbox-cli` 168 (155 before), of them 19 in
  the files of the 3 reviews. One new case over 3 s in the runs of round 3: the `413` case of
  `crates/inbox-cli/tests/tls.rs`, 3.1 s and once 5.9 s under load.
- Security review, pass 1: 9 invariants hold; 1 finding (a body still arriving over TLS got a
  broken pipe in place of the 413), red 5 of 5 before the fix, green 10 of 10 after.
- Adversary pass 1: 9 findings. 4 mutants survived round 1 (the handshake signature unchecked,
  the accepted end of the pair unchecked, the threads not joined, the canonical check of the
  pin dropped); each is red against a case now. 4 findings on documents and one test bound,
  closed. 1 pre-existing: story:bytes-after-a-request-end-without-a-reset (a draft).
- Adversary pass 2, on round 2: no hang of `terminate` in any order of the ends it drove; 7
  findings. 1 defect: an answer cut short ended with `close_notify` in 9 of 10 tries; after the
  fix its case passed 200 tries in a row. 2 more mutants survived (the reading thread not
  joined, the plain end not shut on a failure); each is red against a unit case of 0.7 to
  1.9 s now. 3 findings on documents, a case name and the changelog, closed. 1 undecided, in
  the same draft story.
- Mutants after round 3, 9 run: 8 red against a named case; the ninth (the end of a reset pair
  taken as a clean end) is red in 1 of 10 tries of its case only.
- `website/docs/first-use.md` run as written in 2 shells, over TLS and plain, twice: every
  command exits 0 and the message is listed.
- A certificate made at run time, read by `openssl x509` and `asn1parse`: version 3, Ed25519,
  the signature verifies, and the pin equals the SHA-256 of the SubjectPublicKeyInfo.
- `Cargo.lock`: 99 to 118 entries; 9 new crates are built on Linux.
- Not checked: a published advisory for the pinned versions; a server that speaks TLS 1.2 only
  (rustls is built without it); a redirect from a TLS stand-in; a client that is not rustls;
  messages after the handshake (KeyUpdate).
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T01:39:29Z to 2026-11-23T01:45:02Z, load average 97 to 145
  on 20 CPUs: exit 0, `746 tests run: 746 passed, 5 skipped`, the test step 40 s of
  its bound of 120 s.
