---
format: aep.planning-md/3
id: story:inbox-wake-returns-on-a-post
kind: story
status: implemented
title: inbox wake holds the wait open until a message arrives and marks it delivered
summary: 'A client command every wake adapter runs: it waits again past the end of a wait and past a restart, and exits on a message after MarkDelivered.'
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:the-client-reconnects-after-a-restart
- informed_by: story:no-test-holds-a-wall-clock-time
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/failure.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/wake.rs
- confidence: inferred
  path: crates/inbox-server/src/service.rs
- confidence: cited
  path: docs/checks.md
- confidence: inferred
  path: docs/design/wake.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:15Z", actor: "agent:inbox", revision: 8, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:15Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-23T05:06:52Z", actor: "agent:inbox", revision: 11, decided_on: {"recorded":{"test_result":1,"review_outcome":22}}}
---
## Outcome

A new client command, `inbox wake`, is what every wake adapter runs. It holds the `Pending` view
of the caller own inbox open, starts the wait again when the service ends it without a row,
reconnects after a restart of the service (story:the-client-reconnects-after-a-restart), and
exits 0 when the inbox has a row, after it has printed the rows and sent `MarkDelivered` for
each. A harness that resumes a session when a background task exits resumes it on a post.

`inbox wake` also holds the one-wake-per-inbox rule for every adapter (Open 2): a second
`inbox wake` of the same inbox on the same host exits at once with a stated status and one line
on standard error, through a lock beside the outbox file. So no adapter writes a guard of its
own; story:the-claude-code-adapter-wakes-an-idle-session and
story:the-terminal-adapter-shows-a-post rely on this one.

## Acceptance

An `inbox wake` started on an empty inbox exits 0 with the posted message in its output after
another key posts to that inbox, and the message is in `Delivered`, where the wake kept running
across the end of the service wait and across a restart of the service before the post (state
transition: `inbox wait` returns an empty list when the service wait ends and no client command
moves a message from `Sent` to `Delivered`, to a wait that ends only on a message and delivers it).

## Context

`docs/design/inbox.md` "What the specification cannot say": "No blocking read. `inbox wait` is
the `Pending` view held open until it has a row" (line 298). `docs/design/service.md` "The wait"
(lines 215-228): the service ends a wait at `max_wait` (default 60 s,
`crates/inbox-server/src/config.rs:51`) with an empty `rows`, and wakes a held wait whenever a
command is answered on another connection, not by a clock. Today `inbox wait --seconds` makes one
such request and returns what it got (`crates/inbox-cli/src/main.rs:77-87`,
`crates/inbox-cli/src/commands.rs:239-258`).

`MarkDelivered` is declared: "An adapter reports that a live wake surface took the message. It is
sent only after the surface took it" (`ess/domains/messaging.yaml:545`), and the state
`Delivered` is "An adapter handed it to a live wake surface" (line 240). No client command sends
it (the commands of `crates/inbox-cli/src/main.rs`).

The design names no wake mechanism beyond the `Inbox` entity ("its wake adapter",
`docs/design/inbox.md:39`) and the risk "Waking an idle session relies on a harness resuming a
session when a background task exits" (line 395). This story writes that part of the design
(`docs/design/wake.md`, new), and the adapter stories
(story:the-claude-code-adapter-wakes-an-idle-session,
story:the-metaharness-adapter-wakes-an-idle-session,
story:the-terminal-adapter-shows-a-post) build on it.

Tests hold the folder rule "Tests stay fast": no test over 60 s, the test step within 120 s; no
case waits for the 60 s of `max_wait` (Open 3).

## Domain relations

- Message to Inbox: many messages reference one inbox through `to` (`ess/domains/messaging.yaml`,
  entity `inbox.messaging.Message`, relation `inbox`, `kind: references`, `cardinality: one`,
  `via: to`, lines 230-235). Inferable from the ess/1 document.
- Who moves a message to `Delivered`: only the owner of the inbox in `to`; the sender gets
  `not-recipient` and a third principal `unknown-message` (same file, command
  `inbox.messaging.MarkDelivered`, outcomes `foreign-message`, `not-recipient`, `delivered`,
  `already-delivered`, lines 542-571; transition `deliver` from `Sent`). Inferable from the ess/1
  document.
- Wait to Inbox: how many waits one inbox may have at once is declared nowhere; the service holds
  each on a connection of its own. Inferable only as the code does it, inferred from
  `docs/design/service.md` "The wait" (lines 225-228). The guard of one wake per inbox on one host
  is a rule of the client, Open 2; the service stays as it is.

## Open

1. When the surface "took" a message, the moment `MarkDelivered` may be sent. (A) when
   `inbox wake` has written the rows to its standard output, before it exits: the harness holds
   the output; (B) the session sends it when it reads the output (a command `inbox deliver`), and
   `inbox wake` sends nothing; (C) no `MarkDelivered`: the session reads with `inbox read`, which
   moves `Sent` to `Read`. Default: A.
2. One wake per inbox. The need: an adapter that starts `inbox wake` again after each turn (the
   claude_code adapter, Open 2 A of its story) must not start a second one while one runs, or one
   post wakes the session twice. This story owns the guard. (A, default) a guard inside
   `inbox wake`: a lock file beside the outbox file (`<outbox>.wake`), held while the wake runs; a
   second `inbox wake` of the same inbox on the same host exits with a new exit status 5 and one
   line on standard error, `inbox wake: another wake of <address> runs on this host`, and sends
   nothing; (B) no guard: two waits on one inbox are allowed, each wakes, the first
   `MarkDelivered` is `delivered` and the other `already-delivered`, and each adapter keeps its
   own count. Two waits of one principal on two hosts stay possible under both, as the service
   allows them. Default: A.
3. How the 60 s of the epic is held without a test that waits 60 s. (A) the case starts the
   service with a `--max-wait` longer than the case, so only the post can end the wait, and
   asserts that the wake exits with the message after the post, holding no time: the bound then
   follows from the design, which puts no timer between a post and the end of a held wait; the
   start again after the end of a wait is held with `--max-wait 1`; (B) the case holds a bound of a
   few seconds on the wall clock, which story:no-test-holds-a-wall-clock-time takes out of the
   gate. Default: A.
4. How long `inbox wake` keeps trying while the service is down. (A) until it is stopped, with a
   pause between tries that grows to at most 30 s, so a restart is met within the 60 s of the
   epic; (B) it exits non-zero after a bound, and the adapter starts it again. Default: A.
5. A new command or a flag. (A) `inbox wake`; (B) `inbox wait --until-message`. Default: A.

## Decisions

The 5 open items were taken at their defaults (approval-record:stage-3-six-stories-on-default):
1 A (`MarkDelivered` when `inbox wake` has written the rows to its standard output, before it
exits), 2 A (a guard inside `inbox wake`, a lock file beside the outbox file, exit status 5),
3 A (no case holds a wall-clock time; the start again after the end of a wait is held with
`--max-wait 1`), 4 A (it keeps trying until it is stopped, with a pause that grows to 30 s),
5 A (a new command, `inbox wake`).

The scope was widened once, by the operator's decision relayed on 2026-11-23: `inbox wake`
returns only for a message in `Sent`, and for that the story changes the wait of the listener
by one query parameter (`undelivered=1`) in `crates/inbox-server/src/service.rs`. The story
had named that file "expected unchanged". Not taken: the client filters the view and holds a
wait that every `Delivered` row ends at once.

Taken while building:

- The lock file is `<outbox>.wake.<16 hex digits>`, the 64-bit FNV-1a hash of the address,
  one file per address, held with `flock` and opened without blocking and without following a
  link. Not taken: one `<outbox>.wake` for every address (principals that share an outbox file
  would block each other); the percent-encoded address in the name (it fails on a long address).
  Two addresses whose hashes collide would share one lock, and the second wake would exit 5; the
  design says so.
- The wait of one request is 60 s, with no flag. Each round is one read with `wait=0` and then
  one held read, both with `undelivered=1`. Not taken: a `--seconds` flag; only held reads.
- With `undelivered=1` the service answers only the caller's rows in `Sent` that are not due at
  the service's clock, with `wait=0` as with a held wait, and a held wait ends only on such a
  row. The service has the clock, so the service decides which row is due. Without the
  parameter every answer is as before. The client prints and marks every row of the answer and
  keeps a guard that cuts to `Sent` against an older service. Not taken: the client compares
  `expires` with its own clock. A row that turns due between the answer and its mark is still
  refused `Expired`: exit 1, documented.
- The value is only `1`, read as a whole number like `wait`; another value is refused with
  `400`, also without `wait`; `undelivered=1` without `wait` is a wait of no time. Not taken:
  `true` and `false`; ignoring the parameter when there is no `wait`.
- A request with the parameter is never held for a caller whose own inbox is not open at the
  service: it is answered at once with `409 {"refused": "inbox not open"}`, also when the
  inbox is closed while the read is held. The check uses the caller's own inbox, not the
  address in the query. The refusal is in the listener's own form, so no file of `ess/` and no
  generated Rust file changes. Not taken: the held read carries the epoch the wake expects.
- On that answer the wake runs the reconnect and reads again once when the reconnect opened
  the inbox. It ends with exit 1 and one line that names `inbox open` in 3 cases, each with
  its own words: this host has no record of an `inbox open` of the inbox (never opened here,
  or closed by `inbox close`); the service refused to open it again; it is still not open
  right after an open that succeeded. A typed `inbox close` therefore ends the wake of that
  inbox, and the wake never opens an inbox its owner closed. Not taken: reconnecting again
  without a bound; a wake that opens the inbox itself.
- When the reconnect's open gets no answer, or an answer the wake tries again (`408`, from
  `500` on without a refusal), the wake takes the pause of a failure and reads again (round
  4; in round 3 it ended with exit 1 and said that no `inbox open` on this host had opened the
  inbox, which was not true). The reconnect hands the wake what its open came to
  (`Reconnected.open`); the `--json` member is unchanged.
- "Not open" again right after an open that succeeded ends the wake with 1 also when that
  answer names yet another start of the service: 2 restarts within one round end the wake.
  Not taken: comparing the epochs and following each new start; a service that is never open
  after an open would then be asked forever.
- An `inbox close` followed by an `inbox open` of the owner while a wake holds its read races
  with the wake: when the record of the open is written before the wake's reconnect looks for
  it, the wake reads again and waits on; else it ends with 1 though the inbox is open again.
  The design says so; no case holds it, because it would depend on timing.
- Tried again, with a pause that starts at 100 ms and doubles to at most 30 s: no answer, an
  answer cut before its end, `408`, and every status from `500` on that carries no refusal of
  the model (the rule of `unsent` in `commands.rs`). One line on standard error per run of
  failures. Ends the wake with exit 3 and one line: any other answer that is neither a success
  nor a refusal (a `3xx`, a `4xx` without `refused`, a `2xx` whose body is not the view), and a
  TLS certificate the pin does not name. Not taken: every such answer tried again forever; one
  line per try.
- A whole answer that is not JSON is the new `Failure::Unexpected` (exit 3, shown as
  `transport: …`, the `--json` error stays `transport`), for every command. An answer is whole
  when it fills its `Content-Length`, or has none, is not chunked and ended with its
  connection (round 4; in round 3 a `404` page without `Content-Length` counted as cut and
  was tried again forever). A body shorter than its `Content-Length` is a cut answer, and so
  is a chunked one: the client decodes no chunks, as before. `unsent` and the reconnect's
  `gone` count `Unexpected` as a transport failure, so a kept envelope keeps its mark as
  before. The "not open" answer is the new `Failure::NotOpen` (exit 1), and a certificate
  refusal `Failure::Certificate`. Not taken: reusing `Transport` and `Refused`; matching error
  text.
- A held read that comes back within 1 s without a row is followed by the same growing pause,
  counted on its own; the `wait=0` read is never followed by one. `inbox serve --max-wait 0`
  stays allowed. Not taken: refusing `--max-wait 0`; a pause after every quick answer.
- A row that is not in `Sent` gets no `MarkDelivered`. When a mark fails, every other mark is
  still tried and the exit status is that of the first failed mark (1 for a refusal, 3 for a
  transport failure): the output is complete, and the status tells the adapter that a row is
  still in `Sent`. Not taken: exit 0.
- Standard output is checked first of all, before the lock and the first request, and again
  before the rows are written: the wake duplicates descriptor 1 and reads its status; a failed
  duplicate or the `/dev/null` device ends it with 4 (the standard library opens a closed
  descriptor 1 as `/dev/null` before `main`). A wake started with `>/dev/null` therefore exits
  4 at once, takes no lock and sends no request (round 4; in round 3 the check ran only before
  the write, so such a wake held the lock and read on, and a working wake of the inbox exited
  5). The rows are written through a file made from the duplicate, so a write the kernel
  refuses (a descriptor open only for reading) is exit 4 with no mark. A mark is sent only
  after the write succeeded; a reader that exits without reading what the pipe took is not
  seen, and the design says so. A device that takes every write, such as `/dev/zero`, passes
  the check and its rows are marked; left as it is. Not taken: exit 3 for a closed output;
  the check only before the write.
- SIGINT and SIGTERM end the wake at once with nothing printed, also when they were ignored at
  its start. Not taken: keeping an inherited ignore; a wake adapter starts the wake from a
  script, and a wake that cannot be stopped is the worse failure.
- Every line of the wake on standard error starts with `inbox wake: `, except the reconnect's
  own line. Exit 5 with `--json` also prints `{"error":"another wake",…}`. The `--json`
  document is the answer with `rows` cut to `Sent`, its other members kept, plus `reconnected`
  from the last reconnect. Not taken: a new document shape.
- The pause is held in the tests by lower bounds on the gaps between requests. Not taken: a
  seam for the pause in the binary.
- `Forwarder` (in `tests/support/mod.rs`) reads its target before it records a connection.
- 2 older cases were changed with the check of standard output: they expected 1 request of a
  wake on a closed standard output or `/dev/null` and now expect none
  (`adversary_wake::a_wake_started_with_standard_output_closed_marks_nothing`, one assertion;
  `wake::a_wake_whose_standard_output_is_dev_null_marks_nothing_and_exits_4`).

Scope as built, beyond the list below: `crates/inbox-cli/src/wake.rs` (new, the command);
`crates/inbox-cli/src/output.rs` (the check of standard output; the file's own unit test
allows `stdout(` only there); `crates/inbox-cli/tests/adversary_wake.rs` and
`tests/adversary_wake_2.rs` (the adversary's cases) and
`crates/inbox-cli/tests/support/mod.rs` (`Forwarder`);
`crates/inbox-server/src/service.rs` (the parameter, as widened above) with
`crates/inbox-server/tests/undelivered.rs` (new) and its `mod` line in
`crates/inbox-server/tests/main.rs`; `docs/design/service.md` ("The wait");
`docs/checks.md` (the counts of the 2 packages); the other rows of `AGENTS.md`,
`website/docs/status.md` and `docs/checks.md` that named exit statuses 0 to 4;
`CHANGELOG.md`, whose entry under "Unreleased" the coordinator wrote on this branch.
`crates/inbox-cli/src/outbox.rs` was not changed.


## Scope

Order (parallel safety): this story edits `crates/inbox-cli/src/client.rs`, `src/main.rs`,
`src/reference.rs`, `generated/cli/inbox.json` and the module list of
`crates/inbox-cli/tests/main.rs` after story:the-client-reconnects-after-a-restart (edge
`depends_on`), which edits them after story:the-service-speaks-tls-with-a-pinned-certificate.
story:the-terminal-adapter-shows-a-post edits them after this one.

Code:

- `crates/inbox-cli/src/commands.rs` (`pending`, the new command, `MarkDelivered`): cited
- `crates/inbox-cli/src/main.rs` (the command, its help, exit status 5): cited
- `crates/inbox-cli/src/client.rs` (a request that outlives the connect timeout; a retry): cited
- `crates/inbox-cli/src/outbox.rs` (the path of the wake lock beside the outbox file): cited
- `crates/inbox-cli/src/failure.rs` (exit status 5): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json`: cited
- `crates/inbox-cli/tests/wake.rs` (new) and one `mod` line in `crates/inbox-cli/tests/main.rs`:
  inferred (the directory and `main.rs` are cited)
- `crates/inbox-server/src/service.rs` (the wait; expected unchanged): inferred

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/wake.md` (new): this story creates it with the shared part (`inbox wake`, the
  delivery, the one-wake guard, the 60 s) and one headed section per adapter, `## claude_code`,
  `## metaharness` and `## terminal`, each holding one line that the adapter is not built yet;
  each adapter story edits only under its own heading: inferred
- `website/docs/first-use.md`: a new section `## Wake a session` after "The outbox file", with one
  `###` heading per adapter (`claude_code`, `metaharness`, `terminal`), each holding one line that
  the adapter is not available yet, which each adapter story replaces under its own heading; and
  the row of exit status 5 in "Exit statuses": cited
- `website/docs/status.md`: in the table of "Messages", the row "Wait until a message arrives";
  in "Missing", the bullet "Waking an idle agent session when a message arrives.": cited
- `website/docs/limits.md`: in "The service", the words "and no wake-up" of the bullet "No second
  host and no wake-up": cited
- `docs/checks.md`: in "Open limits and their stories", the words "no wake-up" of the bullet on
  line 86: cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`, the Commands row of the client commands,
  and a new Layout row for `docs/design/wake.md` after the row of `docs/design/inbox.md`: cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited

## Measured

- Failing first: the cases of round 1 in `crates/inbox-cli/tests/wake.rs` red on the unchanged
  client (`5/8 tests run: 0 passed, 5 failed`, stopped after 5 failures: the binary had no
  `wake` command), green after.
- 38 new cases: `inbox-cli` 241 (207 before), of them 16 in `tests/wake.rs`, 8 and 7 in the
  adversary's `tests/adversary_wake.rs` and `tests/adversary_wake_2.rs`; `inbox-server` 127
  (123 before), the 4 of `tests/undelivered.rs`.
- Round 3, 5 runs of the 2 packages in a row at a load average of 123 to 147 on 20 CPUs:
  `349 tests run: 349 passed, 0 skipped` each, 33 to 41 s. No new case ran over 3 s in them;
  the acceptance case took 1.78 to 2.62 s, and 3.55 s once of 4 runs alone at a load average
  of 147 (it waits on the 1 s of `--max-wait 1`, about 11 starts of the binary and one
  restart).
- Adversary pass 1, on round 1: 13 findings. 6 cases red (a wake that spins on a service
  that ends every wait at once, 176 reads in 2 s; a `404` of another server tried again
  forever; a link in place of the lock file followed; a due note that ends the wake with exit
  1; a row marked with standard output closed; a held read on a new start where the inbox is
  not open), the acceptance case red in 3 of 4 package runs, 2 surviving mutants with a case
  each, 4 notes. 11 are closed and the 8 cases pass as written; 2 are kept as built and
  written in the Decisions (an ignored SIGINT is overridden; the time of the acceptance case).
- Mutants: 5 in round 2 and 11 in round 3 run by the implementor, each red against a named
  case.
- Adversary pass 2, on rounds 2 and 3: 7 findings. 4 cases red (a `404` page without
  `Content-Length` tried again forever; a reconnect whose open got no answer ended the wake
  with exit 1 and a line that was not true; a row marked with standard output open only for
  reading; a wake on `/dev/null` that held the lock and read on, 14 reads in 10 s) and 3
  surviving mutants with a case each (`passing` moved by one at `500`; the guard against a
  second "not open" dropped; `Unexpected` dropped from `unsent`). Round 4 closed the 4, and
  the 7 cases pass as written. Not broken by the pass: the request is authenticated before
  the check of the open inbox in every path; the parameter is part of the signed target; the
  check is on the caller's own inbox, so nothing about another principal's inbox is learned.
- Round 4, 3 runs of the 2 packages in a row: `368 tests run: 368 passed, 0 skipped` each, 28
  to 38 s. `inbox-cli` 241 cases after it (7 of the adversary's second file, 2 more of the
  implementor). The 4 mutants of pass 2 run again: each red against its named case.
- No adversary pass ran on round 4.
- Open after round 4, written in the Decisions: 2 restarts within one round end the wake with
  1; a close followed by an open of the owner while a wake holds its read can end the wake
  with 1; a chunked answer counts as cut; `/dev/zero` as standard output passes the check.
- Not checked: the answers without the parameter byte for byte against a binary of `main`
  (the diff of `crates/inbox-server/src/service.rs` was read line by line instead);
  `max_connections` held reads with the parameter.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T05:00:55Z to 2026-11-23T05:03:38Z, load average 85 to 92
  on 20 CPUs: exit 0, `823 tests run: 823 passed, 5 skipped`, the test step 54 s of
  its bound of 120 s.
