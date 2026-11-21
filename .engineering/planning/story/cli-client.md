---
format: aep.planning-md/3
id: story:cli-client
kind: story
status: implemented
title: 'The `inbox` CLI: the client of the service'
summary: One binary that serves, signs, posts, reads, answers, refuses, collects and waits, and reposts what a restart dropped.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:http-service
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: crates/inbox-cli
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-11-21T13:41:08Z", actor: "agent:inbox", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":27,"verification":1}}}
---
## Outcome

`crates/inbox-cli` builds the binary `inbox` (clap derive).

`inbox serve` starts the service of story:http-service. Its flags: `--listen`, `--key-file`,
`--operator-public-key`, `--max-connections`, `--max-request-bytes`, `--max-messages`,
`--sweep-interval`, `--answer-retention`. The backend comes from `INBOX_BACKEND` (`memory` only
in this stage).

The client commands talk to the service at `INBOX_URL` with the caller's key: `open`, `close`,
`post`, `pending`, `wait`, `read`, `answer`, `refuse`, `outbox`, `collect`, `thread`, `replies`.
`post` writes the envelope: it draws the ids, sets `expires` to `created` plus 24 hours when none
is given, and signs. The client keeps every posted message in an outbox file until it is
collected, refused or expired, and `inbox repost` posts the kept messages again; a `Duplicate`
answer counts as arrived.

The key commands: `inbox key new` creates a key pair with mode 0600; `inbox keyfile add-key` and
`inbox keyfile add-delegation` append an entry signed with the operator key.

Every command prints one JSON document with `--json` and exits non-zero on a refusal, naming the
error.

## Acceptance

After the recipient runs `inbox open` against a running `inbox serve`, `inbox post` by the
sender followed by `inbox pending` by the recipient prints the posted message, where before this story the only client of the service is a test's hand-built
HTTP request.

## Context

Stage 2 has the outbox file and the manual `inbox repost`. Reposting by itself after a reconnect,
the harness plugin and the wake adapters are epic:two-hosts-and-wake. The check that a repost
after a restart arrives once is story:service-acceptance.

As settled in wave 3 of stage 2: the service is the library `crates/inbox-server` (`Config`,
`Server::bind`, `Server::run`, `Stopper`; `docs/design/service.md`), and `inbox serve` starts it
over `inbox_core::MemoryBackend`. `Config` has 2 settings the flag list above does not name, so
`inbox serve` has 2 flags more: `--request-timeout` and `--max-wait`. The service binds a
loopback address only and `inbox serve` reports the refusal of any other. A client request
carries the 3 headers `inbox_server::request_headers` writes (`Inbox-Key-Id`,
`Inbox-Timestamp`, `Inbox-Signature`). The client names the answers of the service itself as
refusals too: `401` unauthenticated, `400` for a member name twice, `408`, `413`, and `503` for
the message limit or a stopping service. While story:post-from-is-the-caller is open, the
README's first use says that the service is for a loopback address and test keys only.

As built in wave 4 of stage 2: `crates/inbox-cli` is the binary `inbox`. Exit statuses: 0 done
(a `2xx` answer only); 1 refused by the service (the model's error name, or the service's own
text for `400`, `401`, `408`, `413` and `503`) or an outbox line `repost` cannot read; 2 wrong
usage (an `INBOX_URL` outside loopback, a `--valid-until` in the past, an envelope the generated
decoder refuses); 3 a transport failure or an answer that is neither a success nor a refusal; 4
a local file. An envelope leaves the outbox file only when it is collected, refused with an
error of the model other than `UnknownInbox`, or expired; every refusal of the service itself
and every transport failure keeps it. `inbox keyfile add-key` and `add-delegation` write the key
file under a lock on `<file>.lock`, stage to `<file>.<pid>.new` and keep the file's mode.
`inbox serve` stops the listener before it writes the signal's line and does not join the signal
thread. The client refuses an `INBOX_URL` outside loopback, which this story did not ask for; it
answers a finding of adversary pass 1 while story:post-from-is-the-caller is open. Not built:
`inbox repost` cannot tell a note its recipient read from one a restart dropped, and posts both
again; the case is ignored and names story:repost-does-not-redeliver-a-handled-note. The crate
has 52 passing cases and 1 ignored. The adversary's 2 passes are
review-result:adversary-cli-client-pass-1 and review-result:adversary-cli-client-pass-2.

## Scope

- `crates/inbox-cli/` (new)
- `README.md`: install and first use
- `AGENTS.md`: Commands rows
