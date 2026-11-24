---
format: aep.planning-md/3
id: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
kind: story
status: implemented
title: The chat bridge runs with its own key under a delegation for the person's inbox
summary: 'Under the operator''s answer B on the bridge''s key: every request of the bridge names the person''s inbox, and the wake lock, the open record and the chat-post record are keyed by the person''s address.'
relations:
- decomposes: epic:human-inbox
- depends_on: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
scope:
- confidence: inferred
  path: crates/inbox-cli/src/bridge.rs
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
  path: crates/inbox-cli/src/wake.rs
- confidence: cited
  path: crates/inbox-cli/tests
- confidence: inferred
  path: crates/inbox-cli/tests/bridge_own_key.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: inferred
  path: docs/design/bridge.md
- confidence: cited
  path: docs/design/inbox.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: docs/design/wake.md
- confidence: cited
  path: generated/cli/inbox.json
- confidence: cited
  path: website/docs/first-use.md
- confidence: cited
  path: website/docs/limits.md
- confidence: cited
  path: website/docs/status.md
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T10:15:45Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-11-24T08:01:51Z", actor: "agent:inbox", revision: 10, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "active", to: "implemented", at: "2026-11-24T09:02:52Z", actor: "agent:inbox", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":9}}}
---
## Outcome

Under the operator's answer B (approval-record:d81-a-delegation-grants-acting-as-the-inbox-owner),
the chat bridge runs with a key of its own and the delegation id the person gave it, instead of
a key of the person. Every
request it sends names the person's inbox as the inbox it acts for, and the files it keeps
beside the outbox file are keyed by the person's address, not by its own principal. Posting,
the delivery mark, the read mark and the decision asked once work as before for the person's
inbox.

This is the switch the first 3 bridge stories leave out: they sign with the key they are given,
and their cases give them a key of the person. It is split from
story:the-chat-bridge-turns-a-thread-reply-into-an-answer to keep that story to one pull
request.

## Acceptance

A bridge started with its own key (principal `agent:demo/chat-bridge` in the cases) and a live
delegation for the person posts a note posted to the person's inbox as one chat post and moves
it to `Delivered`, where before this story the same bridge with its own key reads no row of the
person's inbox and posts nothing (state transition: `Sent` to `Delivered` by a bridge that holds
no key of the person).

Checks, each of which can fail:

1. Every request of the bridge names the person's inbox (the header of Open 2 A of
   story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal): both reads of each
   `Pending` round (`wait=0` and the held read, with `undelivered=1`), `mark-delivered`, `read`,
   the `thread` reads of story:the-chat-bridge-asks-a-decision-once and the `open-inbox` of the
   reconnect. A forwarder in front of the service that records the headers of each request sees
   the header with the person's address on every one; the case fails on the first request
   without it.
2. The lock of the wake, `<outbox>.wake.<16 hex digits>`, is the hash of the person's address,
   not of the bridge's principal (today `client.caller.principal`, `crates/inbox-cli/src/wake.rs`,
   `run` and `terminal_only`): a second bridge for the same person with the same outbox file and
   the same key exits 5 and posts nothing; a bridge for a second person with the same outbox file
   and the same key runs and posts that person's messages.
3. The record of `inbox open`, `<outbox>.opened`: the open of the bridge writes the line of the
   person's inbox with the adapter `slack`; after a restart of the service its reconnect opens
   the person's inbox again (the `Inboxes` view lists it `Open` with `slack`), not an inbox of
   the bridge's own address (the view lists none).
4. The record of the chat posts, `<outbox>.chat.<16 hex digits>`, is keyed by the person's
   address: a bridge that ran with a key of the person and is started again with its own key for
   the same person reads the same record, and the stand-in records no second post operation for a
   message already posted.
5. The check of the adapter (Open 4 A of story:the-chat-bridge-posts-a-pending-message) reads
   the person's line of `<outbox>.opened`: with a line `terminal` there the bridge with its own
   key ends with 1 and its one line, and posts nothing.
6. The bridge with its own key ends on a refused key or delegation, as Open 4 says. Under its
   default, the case runs 2 causes: the delegation removed from the key file or its
   `valid_until` at or before the clock of the service when the service starts; and the key
   entry of the bridge lapsed. In each, every request is answered `401`, and the bridge exits
   with status 1 after one line on standard error, the same line for both causes, that names the
   key id, the delegation id, `inbox keyfile add-key`, `inbox keyfile add-delegation`, and that
   the service reads the key file at its start. It posts nothing and sends no further request
   (the forwarder sees none after the first `401`). The case fails when the bridge goes on,
   exits 0, or its line lacks one of the 2 ids or one of the 2 commands. This story builds that
   end; no earlier story has it.
7. The cases run against a real `inbox serve` on loopback, with a key file made in the case that
   holds the bridge's key entry and the delegation, and the stand-in `connectors` of
   story:the-chat-bridge-posts-a-pending-message. No case reaches a chat provider, a network
   beyond loopback or a model; no case over 60 s, none holds a wall-clock time, the test step
   within 120 s, no new Python. A build or a test runs through `slot`, and a pool job ends after
   10 minutes unless it names its time with `slot --timeout` (30 minutes at most).

## Context

Every file the client keeps beside the outbox file is found from the caller: the lock of the
wake by the caller's address (`docs/design/wake.md`, "One wake per inbox on a host"), the record
of `inbox open` per inbox (`crates/inbox-cli/src/outbox.rs`, `opened`, read with
`client.caller.principal` in `crates/inbox-cli/src/wake.rs`), and the record of the chat posts by
the hash of the address (Open 6 A of story:the-chat-bridge-posts-a-pending-message). With a key
of the person, the caller's principal is the person's address and all 3 are right; with the
bridge's own key each would be found under the bridge's principal. The outbox file itself and
its lock, `<outbox>.lock`, hold the sender's envelopes; the bridge posts none.

## Domain relations

- Client to Inbox: the caller carries `inbox` beside `principal` (`ess/domains/messaging.yaml`,
  actor `inbox.messaging.Client`, lines 252-258). Inferable from the ess/1 document; that a
  delegation makes the two differ is story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.
- Inbox to owner after an open by the bridge: `OpenInbox` sets `owner: {caller: principal}`
  (same file, outcome `opened`, line 348), so the bridge becomes the owner; whether that is
  wanted is Open 4 of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.
  Inferable from the ess/1 document as it stands.

## Open

1. How the bridge is told the inbox it acts for. (A) flags `--for <address>` (one flag or two:
   Open 3) and `--authority <delegation id>`; the address is also the key of the 3 files; (B) the
   bridge reads the delegation from the key file: it would read a file the client does not read
   today. Default: A.
2. Whether the cases of the first 3 bridge stories run with both keys. (A) the acceptance case of
   each runs once more with the bridge's own key (3 cases more, each timed and named in the
   report); (B) only the checks above. Default: A.
3. One flag or two for the inbox acted for. The client flag that names it comes with
   story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal (its Scope: "the flag that
   names that inbox", sent as the header of its Open 2 A); no source says whether `inbox bridge`
   takes that flag or one of its own. (A) one flag: the client flag of that story, on every client
   command, `inbox bridge` included, named `--for <address>` there; this story adds only
   `--authority`; (B) two flags: the client flag, and a `--for` of `inbox bridge` alone that the
   bridge turns into it. Default: A.
4. What the bridge does when every request is refused for its key or its delegation (moved here
   from story:the-chat-bridge-turns-a-thread-reply-into-an-answer, which held it as its Open 5):
   a key or a delegation that has lapsed or was never made, answered `401` (the refusal of Open 3
   A of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal). (A) the bridge ends
   with exit 1 and one line, whatever the cause, that names the key id and the delegation id,
   `inbox keyfile add-key` and `inbox keyfile add-delegation`, and that the service reads the
   key file at its start: both causes get the same `401` whose body says nothing more
   (`docs/design/service.md`, "The request signature"), and the secret key file of the bridge
   holds no `valid_until` (`crates/inbox-cli/src/keys.rs`), so the bridge cannot tell them apart;
   (B) it keeps trying after the pause of a failure and writes one line into the chat channel.
   Default: A.

## Scope

After story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal (edge `depends_on`),
and before
story:the-chat-bridge-turns-a-thread-reply-into-an-answer, which edits `bridge.rs` after it.
This story owns the end of the bridge on a refused key or delegation (check 6, Open 4) whole.

Code:

- `crates/inbox-cli/src/bridge.rs` (the flags, the address the 3 files are keyed by, the end on
  a refused request of check 6): inferred
- `crates/inbox-cli/src/failure.rs` (the failure of check 6, its exit status 1 and its line):
  cited
- `crates/inbox-cli/src/wake.rs` (the lock and the adapter check keyed by the inbox acted for):
  cited
- `crates/inbox-cli/src/outbox.rs` (the record of `inbox open` of the inbox acted for): cited
- `crates/inbox-cli/src/commands.rs` (the reconnect opens the inbox acted for): cited
- `crates/inbox-cli/src/main.rs` (`--authority` of `inbox bridge`; `--for` only under Open 3 B),
  `crates/inbox-cli/src/reference.rs`,
  `generated/cli/inbox.json` (regenerated with `task cli-reference`): cited
- `crates/inbox-cli/tests/bridge_own_key.rs` (new) and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: inferred

Documents:

- `docs/design/bridge.md`: a section on the key of the bridge: inferred
- `docs/design/wake.md`: "One wake per inbox on a host", the address of the lock: cited
- `website/docs/first-use.md`: the section of the bridge, the key and the delegation: cited
- `crates/inbox-cli/tests/` (the acceptance bodies of `bridge.rs`, `bridge_read.rs` and
  `bridge_once.rs` take the signer, so that the new module runs them with the bridge's own
  key; the helpers of `support/mod.rs`; the cases of `acting_for.rs` on the commands that
  refused `--for`): cited
- `docs/design/inbox.md` and `docs/design/service.md`: the sentences on `--for` of the
  story before that this story makes false (which commands take it, what the reconnect
  opens): cited
- `website/docs/limits.md` and `website/docs/status.md`: which commands take `--for` now:
  cited

## Decisions

- The 4 items of Open are answered at their defaults (the operator accepted the 7 stories of
  the stage with every open item at its default, 2026-11-23): the bridge is told the inbox it
  acts for by `--for <address>` and `--authority <delegation id>` (1 A); the acceptance case
  of each of the 3 bridge stories runs once more with the bridge's own key (2 A); the flag is
  the client flag of the story before, on every client command (3 A); a bridge with `--for`
  that is answered `401` ends with exit 1 and one line (4 A).
- `inbox bridge` takes `--for` only beside `--authority`, and `--authority` only beside
  `--for`: one without the other is wrong usage (exit 2, one line, nothing sent). The client
  reads no key file of the service and does not check the id; this story uses it only in the
  line of the refused end, and the story after this one sends it in a reply.
- One notion in the client, the inbox a command acts for: the `--for` address when given,
  else the caller's own address. It keys the lock of the wake, the line of `<outbox>.opened`
  and the record of the chat posts. The kept envelopes of the outbox file stay keyed by the
  sender's principal, which under `--for` is the agent's. Without `--for` every file name and
  every line is the one of the builds before, so a bridge that ran with a key of the person
  finds its record again (check 4).
- `inbox wake`, `inbox open` and `inbox close` take `--for` too. The options were: (A) the 3
  take it with this story, one case each, since their state on the host is now keyed by the
  inbox acted for; (B) only `inbox bridge` takes it and the 3 keep refusing it. A is taken:
  Open 3 A names the flag "on every client command". Under `--for` the reconnect of a command
  opens the inbox acted for again, with the header, from the line of that inbox. 2 cases of
  the story before change with it: the case that held the refusal of the 4 commands is
  replaced by the case of the bridge's 2 flags, and the doc comment of the case of the
  reconnect is written anew; its assertions are unchanged.
- The end on a refused request: the first `401` to any client of a bridge with `--for` sets
  one flag of the process; the running `connectors` child is ended, one line is written and
  the process exits with status 1; a thread that would send afterwards waits and sends
  nothing. The line is the same for every cause and names the key id, the delegation id,
  `inbox keyfile add-key`, `inbox keyfile add-delegation`, and that the service reads the key
  file at its start. A bridge without `--for` does on a `401` what it did before: on the open
  or a read of `Pending` it ends with exit 1 and `refused: unauthenticated (401)`; on
  `mark-delivered` it names the failure in one line and tries the round again after the
  pause; in the poll the poll fails, named once per run of failures.
- The acceptance bodies of `tests/bridge.rs`, `tests/bridge_read.rs` and
  `tests/bridge_once.rs` take the signer; each case of those files calls its body with a key
  of the person, with its assertions unchanged, and the new module calls the same body with
  the bridge's own key. No case file is copied.
- In the cases the bridge with its own key shares the outbox file of the person, as check 4
  asks.
- `website/docs/first-use.md` is unchanged, although the story's Scope names it: the page
  gets the bridge, the key and the delegation with the story of the real run, as the 3 bridge
  stories decided.
- This story was built beside the story before it, on that story's local branch, and holds
  its one commit merged in. 3 files conflicted and were resolved in this story's tree: the
  row of `crates/inbox-cli/` in `AGENTS.md` (this story's row with the 3 changes the last
  round of the story before made to it), the row of the test step (written anew by this
  story's record), one doc comment of `crates/inbox-cli/src/main.rs`, and
  `generated/cli/inbox.json`, written anew by its task.
- The line of the refused end and a control character. The adversary showed that a line
  break in `--authority` made 3 lines of that line. The options were: (A) the client refuses
  a control character in `--authority` with exit 2 before any request; (B) the line shows
  every control character of the key id and of the delegation id in visible form, the line
  end included. B is taken: the line is one line whatever was typed and whatever the key
  file holds, and the key id of that line comes from a file the client does not check
  either. Nothing else of `--authority` is checked on the client.
- The kept envelopes under `--for`. The adversary asked which envelopes a command under
  `--for` posts again after a new start of the service. Both an envelope the agent posted
  under `--for` and one it posted without are `from` the agent, and the outbox file keeps
  them by their sender: its next command posts both again once, with the header when that
  command runs under `--for`, and the command after it posts none. The options were: (A)
  keep that, and hold it by a case; (B) the outbox records for each envelope the inbox it
  was posted for, and a command posts again only what was posted for its inbox, which
  changes the form of the outbox file. A is taken: the service answers a post with the
  header as one without it (`docs/design/service.md`, the row of `post`), so both envelopes
  reach the inbox of their `to` either way; a step of the reconnect that fails leaves the
  envelope kept, so the next command tries again (the doc comment of `reconnect` in
  `crates/inbox-cli/src/commands.rs`). No case of this story holds a post again under a
  delegation that is no longer live.

## Measured

- Red first: the 12 new or changed cases against the client before the story: `18 tests run:
  6 passed, 12 failed`; after it `18 tests run: 18 passed`. The cases of `inbox-cli`, 2 runs
  in a row: `392 tests run: 392 passed, 0 skipped` each.
- Each check and its case, in `bridge_own_key` unless named: the acceptance of the 3 bridge
  stories with the bridge's own key (3 cases); check 1
  `every_request_of_the_bridge_names_the_persons_inbox` (the read with `wait=0`, the held
  read, the read of the poll, `mark-delivered`, `read`, the `thread` read, the `open-inbox` of
  the start and of the reconnect); check 2
  `the_lock_is_the_persons_and_a_bridge_for_another_person_runs_beside_it`; check 3
  `the_open_of_the_bridge_is_the_persons_line_and_a_restart_opens_the_persons_inbox_again`;
  check 4 `a_bridge_with_its_own_key_reads_the_record_of_a_bridge_with_the_persons_key`;
  check 5 `the_check_of_the_adapter_reads_the_persons_line`; check 6
  `a_refused_key_or_delegation_ends_the_bridge_with_one_line_and_no_further_request` (no
  delegation, a lapsed delegation, a lapsed key: each 1 request, exit 1, the same line); the
  2 flags `acting_for::the_bridge_takes_for_only_with_authority`; `wake`, `open` and `close`
  under `--for`, 2 cases.
- Mutants, 16, each red in a named case: the header removed from one kind of request (7: the
  read with `wait=0`, the held read, the read of the poll, `mark-delivered`, `read`, the
  `thread` read, the `open-inbox` of the reconnect); the lock keyed by the bridge's
  principal; the record of the chat posts keyed by it; the line of `.opened` written under
  the bridge's address; the adapter check reading the bridge's own line; the bridge going on
  after the `401`; exit 0 after it; the line without the delegation id; the end only on the
  second `401`; `--for` accepted without `--authority`. The mutant of the second `401` is red
  by the line and not by the count of requests: in the case the `401` answers the open, and
  the bridge sends 1 request either way.
- Adversary pass 1 (review-result:adversary-bridge-own-key-pass-1): 5 findings, 1 red case
  (`adversary_bridge_own_key::a_line_break_in_authority_does_not_make_a_second_line_of_the_refused_end`:
  3 lines on standard error where 1 is promised) and 4 mutants of its own that every case
  let live: the refused end dropped from the client of the poll; the running `connectors`
  child not ended; the kept envelopes taken by the inbox acted for; the guard that parks a
  thread after the first `401` switched off. 4 of the 16 mutants above were run again and
  were red. It did not reach: the header on a thread line after silence or expiry, on a
  repost and on the round after a failed round; 2 bridges for one person with different
  keys; an `.opened` file of a build before; a post half made at the `401`; `--json` on the
  refused end; the log of the service per cause; `open --for` on an inbox opened with
  another adapter; `close-inbox` on the agent's own inbox under the header.
- Round 2 on those findings: the line of the refused end writes the 4 ids with their line
  ends as an escape, and the adversary's case is green unchanged (red again with the raw
  values). 3 new cases, each red under the mutant of its finding:
  `a_401_to_the_read_of_the_poll_ends_the_bridge_with_one_line_and_no_further_request` (the
  mutant: the bridge does not end within the bound),
  `a_401_while_a_child_runs_ends_the_child_with_the_bridge` (the mutant: the child outlived
  the bridge) and
  `the_agents_kept_envelopes_are_posted_again_once_by_its_next_command_under_for` (the
  mutant: no envelope posted again). The guard gets no case: its window is too short for a
  case that holds no time; the design says what holds. 2 green cases of the pass are taken
  over: `a_delegation_that_lapses_while_the_bridge_runs_ends_it_with_one_line` (a `401` that
  is not the first request: at most 1 request after the clock moved) and
  `a_close_for_one_person_keeps_the_line_of_the_other_and_only_that_is_opened_again`.
- Adversary pass 2 (review-result:adversary-bridge-own-key-pass-2): no finding. 2 green
  cases added: `odd_characters_in_the_ids_keep_the_refused_end_one_line_on_both_streams`
  (`--authority` and the principal with a carriage return, the line and paragraph
  separators, a next-line character, an escape sequence of a terminal, a direction
  character, a character of no width and 20,000 more bytes, with `--json`: 1 line on
  standard error with each character as its escape, 1 document on standard output) and
  `odd_characters_in_the_key_id_keep_the_refused_end_one_line`. The cases of the 4 bridge
  modules and both adversary modules, 10 times in a row: `63 tests run: 63 passed` each
  time. The pass ran no mutant: a permission check of the session refused its first
  temporary edit of the product, and it stopped there; that its 2 cases are red under a
  mutant is read from the code and not measured. Not reached by either pass: the header on
  a thread line after silence or expiry, on a repost and on the round after a failed round;
  2 bridges for one person with different keys; an `.opened` file of a build before; the
  log of the service per cause of the `401`; `close-inbox` on the agent's own inbox under
  the header; a process of the child's group left after a `401`.
- The cases: 20 added, 1 of them in the place of one (round 1: 11 in `bridge_own_key`, 1 in
  `acting_for` in the place of one; 1 of adversary pass 1; round 2: 5 in `bridge_own_key`;
  2 of adversary pass 2). No case over 5 s: the 3 acceptance cases with the bridge's
  own key took 3.36 s, 3.12 s and, for the case of every request, 2.19 s; the 2 cases of
  round 2 with a `401` from the forwarder 1.07 s and 1.04 s; every other new case 0.24 s or
  less; the new cases together about 12 s of case time. The cases of `bridge_own_key` and
  `adversary_bridge_own_key`, run 10 times in a row after round 2: `10/10 stress run
  iterations: 10 passed`.
- The gate runs of the rounds on the build node (`task check`): round 1, on the local branch
  of the story before, 1053 passed, 8 skipped, the test step 17 s, exit 0; with the one commit
  of the story before merged in 1063 passed, 8 skipped, 15 s, exit 0; after round 2 1069
  passed, 8 skipped, 19 s, exit 0. The run with the 2 cases of adversary pass 2 is the one of
  the row of `task test` in `AGENTS.md`.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T09:02:05Z to 2026-11-24T09:02:42Z, on the build
  node of the pool: exit 0, `1071 tests run: 1071 passed, 8 skipped`, the test step 16 s of
  its bound of 120 s.
