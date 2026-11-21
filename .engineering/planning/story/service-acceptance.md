---
format: aep.planning-md/3
id: story:service-acceptance
kind: story
status: implemented
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
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-21T07:46:05Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-21T15:17:22Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":12,"verification":1}}}
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

As built in wave 5 of stage 2 (2026-11-21). The runner reads `ess-conformance/65` only, and one
suite file (`.out/coverage/suite.json`, synthesized once per `task` run) serves `task coverage`,
`task conform-interpreted` and `task conform`. Against the core: 93 of 93 scenarios passed, and
the report says `conformance_status: passed`; the gate refuses `inconclusive`. Against
`inbox serve` (`inbox-conformance run --target serve --inbox <binary>`, implementation name
`inbox-serve`, a fresh service per scenario): 20 passed, 0 failed, 0 errored, 73 unsupported,
and ESS reports `conformance_status: failed` for that run. The gate holds it to
`crates/inbox-conformance/unsupported.tsv`, ids and reasons, and holds the results document to
the report (`gate --results`). The core's list is `unsupported-core.tsv`, empty. The reasons (a
scenario can have several): a caller whose principal and inbox differ (64; over HTTP both are
the principal of the caller's key, decision 5 of `docs/design/inbox.md`), a step as the Sweeper
(35), a step as no actor (9), a row claim on the stored `signature` (7). `cargo test
--workspace` has 343 passing cases and 9 ignored.

Differs from the Outcome above:

- Events are witnessed over HTTP. Each answer of the service carries its `published` list, and
  the wire target reads the events from it; an event step does not make a scenario unsupported.
- A step that expects `unknown-option` or `shape-mismatch` is sent as the suite states it, and
  the service decides. The target does not test whether the envelope carries the mismatch: that
  would write a rule of the specification a second time. The 3 such scenarios of the suite carry
  the mismatch and pass; one without it would fail, never pass.
- Each answer is held to the HTTP status the generated OpenAPI document gives its outcome, a
  view's rows or the refusal as not granted.
- A view is read once as each key of the run and the rows are joined: the suite names no caller
  for a read, and the listener answers a message row only to its sender and its recipient.

The 3 checks are the package `inbox-acceptance` (`tests/acceptance`, a workspace member), ignored
under `cargo test --workspace` and run by `task acceptance` with `INBOX_BIN` naming the built
binary. Check 3 asserts that the 2 reposts are answered `posted`, then `duplicate`: "`Pending`
holds the message once" alone is true of a store keyed by message id.
`tests/acceptance/tests/adversary.rs` holds 1 ignored case no task runs; it asserts what check 3
asserts about the reposts. The implementor's removal of it was refused by a permission check,
and the coordinator did not remove it in its place.

Not held by this story's runs. The adversary built 12 changed services; 8 stay green in the core
lane, the serve lane and the 3 checks: the read rule dropped, the request signature unchecked,
its window widened, the lapse of a key and of a delegation unchecked, the binding of a key to
`from` unchecked, the delegation pair unchecked, `Messages` and `Due` served to a client. Every
request these runs send is validly signed with entries that do not lapse. The tests of
`crates/inbox-server` and `crates/inbox-verify` fail on each of the 8, and `task check` runs
them. Declined: counting the `401` an unsigned request gets as "not granted to no actor" (1 more
scenario; the model states no such mapping). The adversary's 2 passes are
review-result:adversary-service-acceptance-pass-1 and
review-result:adversary-service-acceptance-pass-2.

## Scope

- `crates/inbox-conformance/`: the wire target
- `crates/inbox-conformance/unsupported.tsv` (the service's list) and `unsupported-core.tsv`
  (the core's, empty)
- `tests/acceptance/` (new)
- `Taskfile.yml`: `conform` runs the coverage suite against both targets; task `acceptance`;
  `check` gains it
- `Cargo.toml`, `Cargo.lock`: the workspace member `tests/acceptance`, where it is one
- `AGENTS.md`: Commands rows
