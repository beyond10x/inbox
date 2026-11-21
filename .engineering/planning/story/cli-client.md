---
format: aep.planning-md/3
id: story:cli-client
kind: story
status: active
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
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":3}}}
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

## Scope

- `crates/inbox-cli/` (new)
- `README.md`: install and first use
- `AGENTS.md`: Commands rows
