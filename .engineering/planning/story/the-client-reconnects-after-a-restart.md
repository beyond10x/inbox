---
format: aep.planning-md/3
id: story:the-client-reconnects-after-a-restart
kind: story
status: implemented
title: A client that meets a new start of the service opens its inbox again and reposts by itself
summary: On the first answer from a new start the client sends OpenInbox with its last adapter and reposts its kept envelopes, with no typed open or repost.
relations:
- decomposes: epic:two-hosts-and-wake
- depends_on: story:the-service-speaks-tls-with-a-pinned-certificate
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: cited
  path: crates/inbox-cli/src/commands.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/outbox.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: crates/inbox-cli/tests/reconnect.rs
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
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T22:02:14Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-11-22T22:02:14Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "active", to: "implemented", at: "2026-11-23T03:16:28Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":24}}}
---
## Outcome

After a restart of the service a client opens its own inbox again and posts its kept envelopes
again by itself, on the first command that meets the new start, so nobody types `inbox open` or
`inbox repost`.

## Acceptance

After a restart of the service, the first command of the owner opens its inbox again with the
adapter it last opened it with, and the next command of the sender puts its kept unanswered
request into the owner `Pending` once, with neither `inbox open` nor `inbox repost` run (state
transition: a restarted service that holds no inbox and no message until both are typed, to one
that holds them after the next command of each client).

## Context

`docs/design/inbox.md` "Shape" (lines 26-28): "The sender keeps each message until it is handled
and posts it again after a reconnect; the owner opens its inbox again." `OpenInbox` says the
same: "A client sends this after every reconnect, because a restarted service holds no inbox"
(`ess/domains/messaging.yaml:336`). The epic: "the client reposting its outbox by itself after a
reconnect". Stage 2 left both to a typed command (story:cli-client): `inbox open`
(`crates/inbox-cli/src/main.rs:54-60`) and `inbox repost` (`crates/inbox-cli/src/commands.rs:634`).

A client tells a new start by the header `Inbox-Epoch` every answer carries
(`docs/design/service.md`, "The epoch", lines 77-98; `crates/inbox-cli/src/client.rs:153-250`),
and `repost` already posts only the kept envelopes the epochs file records at another epoch or at
none (`crates/inbox-cli/src/outbox.rs`). Until the owner opened its inbox again a repost is
refused `unknown-inbox`, which keeps the envelope and records no epoch (`AGENTS.md`, the row of
`crates/inbox-cli/`), so it arrives on a later command. The restart rule of stage 3 (`inbox.md`,
"Stages", lines 368-369: "after a restart of the service the sender repost arrives once") is
checked across client hosts by story:the-stage-3-acceptance-checks.

The limit of `website/docs/limits.md` lines 54-66 stays: the first repost after a restart
delivers once more a note its recipient read before it. This story makes that repost happen
without a typed command; it changes nothing of the limit.

The adapter the client last opened its inbox with is kept on the client host, in a file beside
the outbox file as the epochs file is. It is no entity of `ess/`: the service holds the value in
`Inbox.adapter`, and the file is what the client needs to send `OpenInbox` again.

## Domain relations

- Client to Inbox: a client opens one inbox, its own. `OpenInbox` is refused `not-owner` when
  `address != caller.inbox` (`ess/domains/messaging.yaml`, command `inbox.messaging.OpenInbox`,
  outcome `not-owner`, line 341), and the actor `inbox.messaging.Client` carries one `inbox`
  attribute (same file, `actors`, line 252). Inferable from the ess/1 document (an outcome guard
  and an actor attribute, not a `relations:` entry).
- Inbox to Adapter: each inbox has one adapter, set by `opened` and by `reopened` (same file,
  entity `inbox.messaging.Inbox`, field `adapter`, line 196; `OpenInbox`, `sets: {adapter:
  input.adapter}`). Inferable from the ess/1 document (a field: `Adapter` is an enum, so there is
  no `relations:` entry).
- Kept envelope to epoch: zero or one recorded epoch per message id, replaced by each answer that
  says the service holds the message. Inferable, inferred from `crates/inbox-cli/src/outbox.rs`
  (the epochs file, `record_epoch`, line 131) and `docs/design/service.md` "The epoch"; no ess/1
  document declares it.

## Open

1. Which commands reconnect. (A) every client command whose answer carries an `Inbox-Epoch` other
   than the last one the client recorded; (B) only `inbox wake` and `inbox post`: this ties the
   reconnect to `inbox wake`, the command story:inbox-wake-returns-on-a-post adds, so picking B
   reverses the `depends_on` edge between the two stories (this story then depends on the wake
   story, and the wake story loses its edge on this one) and cannot be picked without that change
   of the edges; (C) a long-running client process per host that reconnects for every session.
   Default: A.
2. In which order. (A) the command own request first, then the open and the repost, so a typed
   command answers as before and reports the reconnect after it; (B) the open and the repost
   first. Default: A.
3. How a reconnect shows. (A) one line on standard error through `src/output.rs` and a member
   `reconnected` in the `--json` document; (B) nothing. Default: A.

## Decisions

The 3 open items were taken at their defaults (approval-record:stage-3-six-stories-on-default):
1 A (every client command whose answer names another `Inbox-Epoch` reconnects), 2 A (the
command's own request first, then the open and the repost), 3 A (one line on standard error and
a member `reconnected` in the `--json` document).

Taken while building:

- A reconnect starts when the record of the opened inbox names another epoch than the answer,
  when a kept envelope is recorded at another epoch, or when a kept envelope is marked as not
  arrived. Not taken: one client-wide record of the last epoch.
- An envelope gets the mark `none` in the epochs file in the change that keeps it, before its
  request is sent (round 3; in round 2 the mark was written after a failed answer, and a post
  stopped on its way was never posted again). The mark stays after a transport failure and
  after `UnknownInbox`, and the sender's next command posts the envelope again. An arrival
  records the epoch; a refusal that keeps the envelope (`413`) removes the mark. Not taken:
  every envelope without an epoch is due; tried in round 1, it posted an envelope refused `413`
  again on every command. Not taken: the process id and start time of the posting process in
  the mark, to tell a post on its way from a dead one.
- A second command of the sender may post an envelope whose first post is still on its way.
  The first post is then answered `Duplicate` for its own id; `inbox post` takes that as
  arrived, records the epoch and exits 0 (round 3).
- The reconnect's repost leaves an envelope as the typed post would for the same answer. For
  `duplicate-key` it removes the mark and writes no epoch, so no reconnect posts the envelope
  again and a typed `inbox repost` still does once the key is free. Not taken: record the
  epoch there; it broke the older case that a typed repost posts such an envelope later.
- The mark goes only when the answer says that the same request is refused again (`400`,
  `401`, `413`, a refusal of the model that keeps the envelope). `408`, `503` and every other
  answer that says the request was not taken keep the mark, as a transport failure does (round
  4; in round 3 a post answered `503` by a stopping service lost its mark).
- A reconnect whose open succeeded after an `inbox close` removed the record sends
  `CloseInbox` and says "closed meanwhile" (round 3). It is not counted as a failed step. It
  then reads the record once more and, when a typed `inbox open` wrote one meanwhile, opens the
  inbox again with that record's adapter (round 4).
- The reconnect replaces the record of the opened inbox only while it is the record it started
  from; a record another command wrote meanwhile is left as it is (round 4). The line on
  standard error says "was opened by another command meanwhile" and the `open` member has
  the outcome `opened-meanwhile`. Not taken: the word `reopened-meanwhile`.
- The second open after "closed meanwhile" adds the member `reopened` to the
  `closed-meanwhile` object and writes no record. A failure of it is named in the
  reconnect's line and is not a failed step: the record names the current epoch, so no
  later reconnect would try it again. It is tried also when the `CloseInbox` failed. Not
  taken: a new outcome value; counting the failure as a failed step.
- One rule (`unsent`) decides in `inbox post`, in the reconnect's repost and in a typed
  `inbox repost` whether an answer says the request was not taken: `UnknownInbox`, `408`
  and every status from `500` on that carries no refusal of the model. A typed `inbox
  repost` answered `408` or `5xx` therefore leaves its envelope marked (before round 4 it
  removed the mark). Not taken: a separate rule for the typed repost.
- The epochs file is replaced before the outbox file (round 4; the reverse before). A mark
  whose envelope was then not kept is dropped by the next write. The other direction is
  open: a removal that wrote the epochs file and then could not write the outbox file
  leaves a kept envelope without a line, which only a typed `inbox repost` posts. Not
  taken: a third write that puts the line back.
- Not closed: more than one typed `open` or `close` of the owner while a reconnect runs can
  leave the record and the service apart until the next typed `open` or `close`. The design
  and the limits page say so. Not taken: one lock held across the open request of a reconnect
  and of a typed `open` and `close`; a stalled service would then block the owner's commands.
- An envelope the epochs file has no line for (kept by an older version, or by a post that
  could not write the epochs file) starts no reconnect; a typed `inbox repost` posts it. After
  an upgrade: `inbox open` once and `inbox repost` once.
- The reconnect posts what a typed `inbox repost` posts, at the epoch of the command's answer,
  without the typed command's own envelope.
- No open step after a typed `inbox open` or `inbox close`, and no repost step after a typed
  `inbox repost`.
- `inbox close` removes the record of the opened inbox before it sends `CloseInbox` (exit 4 and
  nothing sent when it cannot), and puts it back after a transport failure or a refusal other
  than `UnknownInbox`. Where client and service can then disagree, the inbox stays open at the
  service and is not opened again after a restart.
- A transport failure of the open ends the reconnect; the repost is not tried.
- The member `reconnected` is in the `--json` document also when a step failed, with the
  failure's document. Without `--json` the command's own lines are written before the
  reconnect starts; with `--json` the one document is written after it.
- The line on standard error reads "reconnect at the start <epoch> of the service": a reconnect
  can happen without a restart.
- A typed `inbox open` whose record cannot be written exits 4 after it printed its outcome, as
  `inbox post` does for the epochs file.
- The record is the file `<outbox>.opened`: one JSON object per line (`inbox`, `adapter`,
  `epoch`), because one outbox file can serve several principals. It is changed under the
  outbox lock and replaced whole.
- The files beside the outbox are opened without blocking, as regular files only, under a bound
  (64 MiB; 1 MiB for `<outbox>.opened`). A command that only reads takes another file as
  empty, with one warning; a change of the outbox file fails.
- `CHANGELOG.md` has its entry under "Unreleased" on this branch, written by the coordinator.

Scope as built, beyond the list below: `crates/inbox-cli/src/output.rs` (`with_member`);
`crates/inbox-cli/tests/adversary_reconnect.rs`, `_2.rs` and `_3.rs` (the adversary's
cases); `website/docs/first-use.md` and `website/docs/limits.md`; 4 older cases that
asserted the old rule (`tests/epoch.rs` 2, `tests/adversary_w8_repost.rs` 1, `tests/tls.rs` 1);
`docs/checks.md` (the count of `inbox-cli`); `README.md` (the paragraph on `inbox repost`);
`website/docs/status.md` (the planned list no longer names the repost by itself);
`CHANGELOG.md`.


## Scope

Order (parallel safety): this story edits `crates/inbox-cli/src/client.rs`, `src/main.rs`,
`src/reference.rs`, `generated/cli/inbox.json` and the module list of
`crates/inbox-cli/tests/main.rs` after story:the-service-speaks-tls-with-a-pinned-certificate,
which edits the same files (edge `depends_on`). story:inbox-wake-returns-on-a-post edits them
after this one.

Code:

- `crates/inbox-cli/src/commands.rs` (`open`, `repost`, `command_in_epoch`, the commands that
  read an answer): cited
- `crates/inbox-cli/src/client.rs` (the epoch of an answer): cited
- `crates/inbox-cli/src/outbox.rs` (the epochs file; the file of the opened inbox beside it): cited
- `crates/inbox-cli/src/main.rs` (the help of `open` and `repost`): cited
- `crates/inbox-cli/src/reference.rs`, `generated/cli/inbox.json`: cited
- `crates/inbox-cli/tests/reconnect.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred (the directory and `main.rs` are cited)

Shared documents: this story edits only the parts named here, and nothing else in each file.

- `docs/design/service.md`: the last paragraph of "The epoch" (what `repost` reads it for): cited
- `website/docs/first-use.md`: the section "The outbox file": cited
- `website/docs/status.md`: in the table of "Messages", the row "Post again after a restart of the
  service, from the sender outbox file": cited
- `website/docs/limits.md`: the section "`inbox repost` can deliver a read note once more", and in
  "The service" the bullet "One backend, in memory": cited
- `AGENTS.md`: the Layout row of `crates/inbox-cli/` and the Commands row of the client commands:
  cited
- `CHANGELOG.md`: not edited; the coordinator of a release writes it: cited

## Measured

- Failing first: the 8 cases of round 1 in `crates/inbox-cli/tests/reconnect.rs` red on the
  unchanged client (`8 tests run: 0 passed, 8 failed`), green after.
- 39 new cases: `inbox-cli` 207 (168 before), of them 14 in the adversary's 3 files. No new case
  ran over 3 s in the 3 runs of round 4 (`207 tests run: 207 passed`, 16 to 36 s each); the
  slowest new case took 2.06 s beside a mutant run.
- Adversary pass 1, on round 1: 8 findings. 3 defects, the order of the output, 2 surviving
  mutants (the lock released before the write; the guard of a typed repost removed), 2
  statements of a test and the documents. Each is closed, and its 5 cases pass as written.
- Adversary pass 2, on round 2: 8 findings. 4 defects, each with a case that was red
  (`4 tests run: 0 passed, 4 failed`) and passes as written after round 3. 0 of 11 mutants
  survived. 3 notes on the changelog, 2 document lines and a test that reads `/proc`, closed. 1
  pre-existing: story:an-unreadable-epochs-file-keeps-its-other-lines (a draft).
- Adversary pass 3, on round 3 only: 6 findings. 3 defects, each with a case that was red and
  passes as written after round 4. 2 of 6 mutants survived; each has a case now. 1 note on a
  decision (`duplicate-key` in the reconnect), kept as built.
- Mutants after round 3, 6 run by the implementor, and after round 4, 7: each red against a
  named case.
- No adversary pass ran on round 4.
- Open after round 4, written in the Decisions: a failed second open after "closed meanwhile"
  is not tried again; a removal whose outbox write failed after the epochs file was written
  leaves an envelope only a typed `inbox repost` posts.
- Not checked: an outbox file at its bound of 64 MiB and 1 byte over; a 0.3.1 binary run
  against files this version wrote (its code was read).
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T03:13:06Z to 2026-11-23T03:15:52Z, load average 71 to 87
  on 20 CPUs: exit 0, `785 tests run: 785 passed, 5 skipped`, the test step 62 s of
  its bound of 120 s.
