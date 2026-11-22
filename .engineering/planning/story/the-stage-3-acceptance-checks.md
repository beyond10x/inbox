---
format: aep.planning-md/3
id: story:the-stage-3-acceptance-checks
kind: story
status: active
title: 'The acceptance checks of stage 3: two separated clients, a wake and a repost after a restart, over TLS'
summary: task acceptance holds the stage 3 statement against one inbox serve with a certificate and two clients that share no file.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:the-service-speaks-tls-with-a-pinned-certificate
- depends_on: story:the-client-reconnects-after-a-restart
- depends_on: story:inbox-wake-returns-on-a-post
- informed_by: story:no-test-holds-a-wall-clock-time
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: docs/checks.md
- confidence: cited
  path: tests/acceptance/src/lib.rs
- confidence: cited
  path: tests/acceptance/tests/checks.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

`task acceptance` holds the stage 3 statement of `docs/design/inbox.md` against the built binary:
two clients that share no file, as on two hosts, exchange a request and a reply through one
`inbox serve` over TLS; an `inbox wake` of the second exits with the post of the first; and after
a restart of the service the kept request of the first is in the `Pending` of the second once,
with nobody typing `inbox open` or `inbox repost`.

## Acceptance

`task acceptance` passes with 3 more checks against one `inbox serve` started with a certificate:
a request and its reply between two clients that share no key, outbox, state directory or
environment variable; an `inbox wake` of the second that exits with the post of the first; and
the kept request of the first once in the `Pending` of the second after a restart, with no typed
`open` or `repost` (state transition: the stage 2 checks over plain HTTP with typed reposts, to
the stage 3 statement held over TLS between separated clients).

## Context

Stage 3 in `docs/design/inbox.md` "Stages" (item 3, lines 367-369): "A session on each host
exchanges request and reply through one service; an idle session takes its next turn within 60 s
of a post; after a restart of the service the sender repost arrives once."
`tests/acceptance/tests/checks.rs` holds the 3 checks of stage 2 (`AGENTS.md`, the row of `task acceptance`): a
request and reply between 2 keys, expiry, and a restart with a typed `inbox open` and 2 typed
reposts. `Taskfile.yml` (task `acceptance`, line 105) fails when fewer than 3 checks pass; the
count changes with this story.

The test step makes no network request beyond loopback and no model call; no check runs over
60 s (`task acceptance` kills each at 60 s) and the step ends within 120 s (the folder rule "Tests
stay fast"). story:no-test-holds-a-wall-clock-time takes wall-clock bounds out of these checks and
gives the service a clock for tests. The epic Acceptance names an idle session on a second host
that takes its next turn within 60 s: a real session and a real second host are outside the gate
(Open 1 and 2).

## Domain relations

None of its own: it checks the behaviour of story:the-client-reconnects-after-a-restart and
story:inbox-wake-returns-on-a-post, whose bodies carry the relations they rest on.

## Open

1. How two hosts are proved on one machine without a network request beyond loopback. (A) two
   clients that share no file and no environment variable, each with its own key, outbox, state
   directory and pin, against one service over TLS on a loopback address: it proves that nothing
   of one client host is needed by the other, not the path between hosts; (B) a network namespace
   or a container per host: the path is real, but it needs privileges the shared host and CI may
   not grant; (C) A in the gate, and the operator runs the stage on two real hosts with a real
   session after story:the-service-binds-a-reachable-address and the operator order, recorded as
   evidence of the epic. Default: C.
2. How the 60 s of the epic is held. (A) the wake check asserts only that the wake exits with the
   post while the `--max-wait` of the service exceeds the check, as in
   story:inbox-wake-returns-on-a-post, and the 60 s is shown by the operator run of Open 1 C;
   (B) a bound on the wall clock in the check. Default: A.

## Scope

Order (parallel safety): this story edits no code file of `crates/inbox-cli`. It depends on the
TLS, reconnect and wake stories, whose behaviour it checks. It shares `AGENTS.md` and
`docs/checks.md` with story:the-service-binds-a-reachable-address and edits other parts of them
(below), so no edge orders the two.

Code:

- `tests/acceptance/tests/checks.rs` (3 more checks in the module `checks`): cited
- `tests/acceptance/src/lib.rs` (fixtures: a certificate, separated client environments): cited
- `Taskfile.yml` (task `acceptance`, line 105: the count of checks): cited

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `AGENTS.md`: the Commands row of `task acceptance` and the Layout row of `tests/acceptance/`:
  cited
- `docs/checks.md`: in "On `main`", the row "Acceptance": cited
- `website/docs/`: not edited: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited
