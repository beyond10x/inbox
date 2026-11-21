---
format: aep.planning-md/3
id: story:service-acceptance
kind: story
status: active
title: The suite and the stage's acceptance checks against a running service
summary: The conformance suite over HTTP, two sessions exchanging a signed request and reply, and the removal and restart checks.
relations:
- decomposes: epic:service-in-memory
- depends_on: story:cli-client
- depends_on: story:conformance-gate
- depends_on: story:declared-coverage
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/inbox-conformance
- confidence: inferred
  path: tests/acceptance
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`crates/inbox-conformance` gains a second target that drives a running `inbox serve` over HTTP,
and `task conform` runs the suite against both targets. A scenario step the wire cannot observe
is `unsupported` with its reason in `unsupported.tsv`. The wire target starts the service with
a key file it writes and signs every request and every envelope with those keys. For a step
that expects `signature-invalid` or `authority-invalid` it sends a signature that does not
verify or a delegation the key file does not hold. A step that expects `unknown-option` or
`shape-mismatch` from an envelope that carries no such mismatch is `unsupported`.

`tests/acceptance` holds the checks a
scenario cannot make, each starting its own `inbox serve` on a free loopback port with the
`inbox` binary of story:cli-client:

- two clients with different keys exchange a signed request and a reply;
- a note that was read is in no view of the service, and neither is an unanswered request 2
  sweep intervals after its `expires` (the service runs with `--sweep-interval 1`);
- after the service is stopped and started again, the owner runs `inbox open`, the sender runs
  `inbox repost` twice, and `Pending` holds the message once.

`task acceptance` runs them, and `task check` runs `task conform` and `task acceptance`.

## Acceptance

`task check` exits 0 after it starts `inbox serve`, runs the suite against it over HTTP with 0
failed and 0 errored and runs the 3 checks of `tests/acceptance` (the signed exchange, the
removal, the repost after a restart), where before this story `task check` starts no
service.

Added at the opening of wave 5 (2026-11-21, under Context): the report `task conform` writes
for the run against the core says `conformance_status: passed`, where before this story it
says `inconclusive`.

## Context

This story closes the acceptance of epic:service-in-memory. One story carries both of the
stage's observations (the signed exchange and the repost after a restart) because they need the
same harness: a service process started and stopped by the test, and two clients with their own
keys.

The scenario `ess/scenarios/repost-after-restart-arrives-once.yaml` models the restart as the
empty state a scenario starts from; the restart itself is checked here.

As settled in wave 4 of stage 2. The binary is `crates/inbox-cli`:
`inbox serve --listen 127.0.0.1:0 --key-file <file> --operator-public-key <base64>` prints the
URL it serves on as its first line, takes `--sweep-interval` in seconds and stops on SIGTERM
with status 0. `crates/inbox-cli/tests/support/mod.rs` starts the service and makes keys at run
time; the checks of this story make their keys the same way and hold no key literal. The client
exits 0 done, 1 refused, 2 wrong usage, 3 transport, 4 a local file. `inbox repost` posts a note
again after its recipient read it (story:repost-does-not-redeliver-a-handled-note), so the
repost check of this story reposts a message its recipient has not read.

`task coverage` exits 0, and the suite it writes (`ess verify conform synthesize
--suite-format 5`, an `ess-conformance/65` document) reports `conformance_status: passed` on the
interpreted target. `task conform` still synthesizes without declared coverage, so its report
says `inconclusive`, and the runner refuses the coverage suite (the key `coverage`, and row
paths with a dot). The host operator's condition of the stage 2 approval is that the stage is
not done while the status is `inconclusive`. This story owns the runner and the `conform` task,
so it also makes the runner read the coverage suite, makes `task conform` run that suite against
the core and against the service, and makes the gate accept `passed` for a run with no
unsupported scenario. ESS reports `failed` for a run with an unsupported scenario: the gate holds
the run over HTTP to its list of unsupported scenarios, and the unit reports the status ESS
gives that run.

## Scope

- `crates/inbox-conformance/`: the wire target
- `crates/inbox-conformance/unsupported.tsv`
- `tests/acceptance/` (new)
- `Taskfile.yml`: `conform` runs the coverage suite against both targets; task `acceptance`;
  `check` gains it
- `Cargo.toml`, `Cargo.lock`: the workspace member `tests/acceptance`, where it is one
- `AGENTS.md`: Commands rows
