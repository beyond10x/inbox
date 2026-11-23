---
format: aep.planning-md/3
id: story:the-stage-3-acceptance-checks
kind: story
status: implemented
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
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:16Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-23T06:42:57Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":8}}}
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

## Decisions

The 2 open items were taken at their defaults (approval-record:stage-3-six-stories-on-default):
1 C (in the gate, 2 clients that share no file and no environment variable against one service
over TLS on a loopback address; the run on 2 real hosts with a real session is the operator's,
after story:the-service-binds-a-reachable-address and the operator's order) and 2 A (the wake
check asserts that the wake exits with the post while `--max-wait` of the service exceeds the
check; no bound on the wall clock).

Taken while building:

- Each check starts its own `inbox serve`. Not taken: one service for the 3 checks, which
  cargo-nextest would start once per check anyway (a process per test).
- The fixture `TwoHosts` keeps 3 directories in the case's directory under `target/tmp`:
  `service/`, `alice/`, `bob/`. Each client command runs in its own client's directory with an
  empty environment and that client's 6 variables only (`HOME`, `XDG_STATE_HOME`,
  `INBOX_SECRET_KEY_FILE`, `INBOX_OUTBOX`, `INBOX_URL`, `INBOX_TLS_PIN`). 3 proofs: the 2
  directories are not nested and each path variable lies inside its own client's directory;
  around every client command the other client's directory tree (paths and bytes) is
  unchanged; while the wake of the second client runs, `/proc/<pid>/environ` and
  `/proc/<pid>/cwd` are its own. The path variables are named in a list, not told by the shape
  of a value: a pin can start with `/`.
- The clients' address is one the fixture owns: a TCP forwarder (`Route`) made at the first
  start, which each start of the service, on a port of its own, is put behind. So a restart
  changes neither client's environment and binds no port twice. Check 3 holds the old
  service's port with a socket of its own over the restart and stays green. Not taken: the
  restart binding the first start's port again (round 1; another socket can take the port in
  between, adversary pass 1 finding 5), a bounded retry of the start, a new URL in both
  clients. TLS runs end to end through the forwarder, so `Inbox-Epoch` and the lost request are
  as without it; a connection made while no service runs is closed by the forwarder and not
  refused, and no command of a check runs in that span.
- The wake's held read is shown by the byte counts of a second forwarder in front of the wake,
  with no sleep: its first connection was ended by the service, and on its second the client
  has sent at least the bytes of the first read's request after the service's handshake and
  the service has not ended it. After the wake ends: the service ended that connection and
  wrote on it more than on the empty first read, by at least the bytes the post's row must
  hold (its id twice and both addresses, 94 bytes). Not taken: the lock file held by the
  wake's pid (it shows a start, not a held read); an exact count of 3 connections (the
  adversary's proposal: a retry after a failure in transport or a 408 adds a connection and is
  no defect); a snapshot of the held connection just before the post (a wake that polls every
  300 ms passed it 5 of 5 times); a margin of the id once (26 bytes, near the 22 bytes of one
  more TLS record).
- Check 1 also posts with the pin of a second certificate, made by `inbox tls new` in the
  case: exit 3, and nothing in the other client's `Pending`. It comes after the exchange. Not
  taken: before it (a post refused in transport stays kept, and the sender's next command
  would post it again and change the exchange).
- No failure message of the fixture prints a pin or the value of a client's variable: a
  failed comparison of the wake's environment names the variables that are missing, extra or
  of another value; `inbox tls new` in the fixture prints its exit status and standard error
  on failure, never its standard output. The standard output and standard error of a failed
  client command stay in the message. Not taken: hiding all output of a failed command.
- The assertion of round 1 on the fixture's own log of typed commands is gone: no defect of
  the product could make it fail. The proof that nobody typed `open` or `repost` after the
  restart is the check's own text and the `reconnected` members of the 2 answers.
- The bound that ends a hang is 20 s per wait; 2 waits fit inside the 60 s at which a check
  is killed.
- `README.md` (the count of the checks, twice) was edited by the coordinator, outside the
  Scope below. The adversary's file of 3 cases was not moved into the tree: the 3 checks and
  the fixture hold what its cases held, and the count of `task acceptance` stays 6.


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

## Measured

- Failing first, round 1, each check against 2 or 3 planted defects. Check 1: the service
  without `--tls-file` (`inbox serve serves http://…, not https://`); the second client given
  the first's outbox (`INBOX_OUTBOX of bob is …, outside …/bob`). Check 2: `inbox wait` in
  place of `inbox wake` (`the wake holds its read of Pending: not within 20 s`); no
  `MarkDelivered` in the wake (`the wake marked the post delivered`, `Sent` against
  `Delivered`); client commands without an empty environment (`the wake runs with bob's
  variables and no other`). Check 3: the first client's command after the restart left out
  (`Pending holds the request once`, `[]`); the reconnect switched off (``no `reconnected` in``
  the answer). The edits in `crates/` were temporary; `git diff --stat crates/` was empty after.
- Round 1: `task acceptance: 6 checks passed`; 30 runs of the 3 new checks in a row, 0 red; no
  check over 1.6 s.
- Adversary pass 1, on round 1: 7 findings, 1 case red on purpose (a socket that takes the
  port during the restart: `inbox: --listen 127.0.0.1:…: Address already in use`). 2 mutants
  of the product passed the 3 checks: a client that accepts any certificate, and a wake that
  reads with `wait=0` every 300 ms (6 of 6 runs green). 4 notes: the post can reach the
  service before the held read's request; a failed assertion would print the wake's
  environment; an assertion that cannot fail; one row of `AGENTS.md`.
- Round 2 closed the 6 that are the implementor's, each changed check red first. Check 1
  against the client that accepts any certificate: `a post to a service whose certificate the
  pin does not name ends with 3`, 0 against 3. Check 2 against the polling wake, 5 of 5 runs
  red: `the post was not answered on the read the wake held: the service wrote 728 bytes on it
  and 728 on the read without a wait`. Check 3 with the forwarder taken out and the socket on
  the old port kept: `inbox serve printed no first line; its standard error: inbox: --listen
  127.0.0.1:…: Address already in use (os error 98)`.
- Round 2 at a load average of 93 to 103: the 3 new checks together 20 times, 0 red; each
  alone 5 times, 0 red. Over 26 runs per check, minimum, median and maximum: 1.07, 1.60 and
  3.82 s (2 clients that share no file); 0.67, 1.17 and 2.34 s (the wake); 1.00, 1.54 and
  3.04 s (the restart). No check over 5 s. `task acceptance` 8.2 s of wall time.
- The seventh finding (the row of `task test` in `AGENTS.md` named 5 skipped and 4 ignored
  cases of `tests/acceptance`) is closed by the sentence this change adds to that row: 8
  skipped, the 7 ignored cases of `tests/acceptance` and 1 of `inbox-conformance`.
- No adversary pass ran on round 2.
- Left open: the socket of check 3 does not get the old port when the new start's own port
  happens to be the old one; the check then fails at its assertion that it got the port. Not
  seen in 35 runs.
- Not run: the stage on 2 real hosts with a real session (the operator's, Open 1 C).
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T06:38:09Z to 2026-11-23T06:42:19Z, load average 97 to 120
  on 20 CPUs: exit 0, `861 tests run: 861 passed, 8 skipped`, the test step 62 s of
  its bound of 120 s.
