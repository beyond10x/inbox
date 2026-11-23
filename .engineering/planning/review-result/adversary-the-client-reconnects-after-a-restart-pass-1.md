---
format: aep.planning-md/3
id: review-result:adversary-the-client-reconnects-after-a-restart-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the reconnect story: 8 findings, 5 red cases'
relations:
- reviews: story:the-client-reconnects-after-a-restart
revision: 1
---
unit: story:the-client-reconnects-after-a-restart, tree inbox-s3-reconnect at commit 9f71690 (git diff 5c598a3), plus my test file only
verdict: CONFIRMED (5 red cases, 2 surviving mutants)
cases: executed 180→185, red 5 (180: the same suite run without the 5 new cases, and every mutant run in the copy ran 180)
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s3/reconnect/adversary/
needs-coordinator: finding 3 (FIFO). The brief calls any read that blocks for ever a blocker, but nothing in the product puts a FIFO there, so I need you to settle the severity.

Diff of the tree (`git diff --stat`, plus the untracked file): `crates/inbox-cli/tests/main.rs | 1 +` (`mod adversary_reconnect;`) and `?? crates/inbox-cli/tests/adversary_reconnect.rs`. Both are test files. I edited no implementation, document or `.engineering/` file. I took no worktree lease.

Cases (`crates/inbox-cli/tests/adversary_reconnect.rs`). I ran them alone first with `cargo nextest run -p inbox-cli --locked -E 'test(/^adversary_reconnect::/)'`: 5 run, 5 failed, exit 100. All 5 are red now. The output below is verbatim.

1. `a_request_posted_while_the_service_was_down_arrives_after_the_restart_by_itself` (1.5 s). Steps: bob opens, alice's post fails in transport (kept), restart, one command by bob, two by alice. Red at :75 with "assertion `left == right` failed: after the restart, bob's command and two commands of alice, the kept request is in bob's Pending once / left: [] / right: ["01M82XCM4X03R4RZ6M39BFQMF5"]".
2. `a_post_refused_unknown_inbox_by_the_new_start_arrives_once_the_owner_opened_again` (1.5 s). Steps: restart, alice posts before bob comes back and is refused `UnknownInbox` (kept), bob's command reopens the inbox, two commands by alice. Red at :106 with "two commands of alice after bob opened again: the kept note is in Pending once / left: [] / right: ["01M82XCMBMBVSA3Z7KVSJHJ0NG"]".
3. `a_close_whose_record_could_not_be_removed_does_not_open_the_inbox_after_a_restart` (1.2 s). Steps: bob opens, a directory is placed at `<outbox>.opened.new`, `inbox close` is done by the service but exits 4, the directory is removed, restart, bob runs `pending`. Red at :135 with "bob's command after the restart opened the inbox he closed: … stderr: \"inbox: the service started anew (ba5e…f424): opened agent:test/bob again with the adapter terminal (opened)\"".
4. `a_fifo_in_place_of_the_opened_file_does_not_hang_a_command` (16.3 s when red; the 15 s bound only ends the hang). Red at :169 with "`inbox pending` still ran 15 s later, blocked opening the FIFO …/bob.outbox.opened".
5. `the_typed_command_prints_its_own_answer_before_a_stalled_reconnect_ends` (16.4 s when red; same 15 s bound). Red at :259 with "the command's own answer was printed only after the reconnect's open ended".

Suite, run after the cases existed: `cargo nextest run -p inbox-cli --locked --no-fail-fast` gave "Summary [73.088s] 185 tests run: 180 passed, 5 failed, 0 skipped", exit 100. The only failures are the 5 cases above. `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exit 0. `rustfmt --edition 2024 --check` clean.

Findings

F1 (cases 1 and 2), `crates/inbox-cli/src/outbox.rs:212`. A kept request that never reached the service, or that the new start refused `UnknownInbox`, is never posted again by itself. The reason is that `Outbox::due` reposts only when some envelope has an epoch recorded and that epoch is another one. An envelope with no epoch is posted only if some other envelope happens to be due (the implementor's note at commands.rs:893 is that same side effect).
- What reaches it: any post made during the restart window or before the owner's next command.
- Why it matters: `limits.md` ("the next command of the sender posts its kept envelopes again, by themselves"), `status.md` and the story's Context ("arrives on a later command") promise it. The acceptance case passes only because its request arrived before the restart.
- Fix lands in one of two places: in `due`, also treat envelopes with no epoch as due (one post per kept envelope per command); or in the pages, which then say a post that did not arrive still needs a typed `inbox repost`.

F2 (case 3), `crates/inbox-cli/src/commands.rs:218`. If removing the record fails after the service confirmed the close, the record stays, and the next restart opens the closed inbox again. A post to it is then accepted.
- What reaches it: a full disk or an unwritable state directory at close time. This host has per-user quotas. The user sees exit 4 with "closed" printed.
- Fix lands in `close`: remove the record before sending `CloseInbox` and put it back when the close is refused (other than `UnknownInbox`) or fails in transport. Or say it in the help of `open` and `close`.

F3 (case 4), `crates/inbox-cli/src/outbox.rs:205`. `Outbox::due` uses `fs::read_to_string` on `.opened` (and on `.epochs` and the outbox file) after every client command whose answer names an epoch. A FIFO there blocks every command for ever. Since this unit that includes `pending`, `wait` and `read`, which on the base read no file beside the outbox.
- This contradicts the module documentation: "Reading it never fails a command".
- What reaches it: nothing in the product. Only a process of the same account places a FIFO there. `--clock-file` was hardened against exactly this case. Severity is the brief's rule.
- Fix lands in `due` and `change_opened`: open with `O_NONBLOCK` and refuse anything that is not a regular file, as `src/clock_file.rs` does.

F4 (case 5), `crates/inbox-cli/src/main.rs:192`. The typed command's own output is printed only after the reconnect ends. Story Open 2 A says the command "answers as before and reports the reconnect after it". As built, a stalled service delays the output by up to 30 s per reconnect request (one open, plus one post per kept envelope). A kill in that window loses output the command already earned, for example the message id of an accepted post.
- What reaches it: any reconnect against a slow or half-up service.
- Fix lands in `main`: print the text output before running the reconnect (`--json` has to wait for the `reconnected` member).

F5 (mutant m6b, survived), `crates/inbox-cli/src/outbox.rs:277`. Diff: `let lock = self.lock()?;` followed by the added line `lock.unlock().map_err(local)?;` in `change_opened`. Result: "180 tests run: 180 passed". The concurrency case writes identical records, so it cannot tell whether the lock is held.
- Fix lands in `tests/reconnect.rs`: hold `<outbox>.lock` from the test, run `inbox open`, and assert that the command waits (a waiter in `/proc/locks`) and that `.opened` stays unwritten until the lock is released.

F6 (mutant m7, survived), `crates/inbox-cli/src/commands.rs:893`. Diff: `- let repost = due.repost && typed != Typed::Repost;` becomes `+ let repost = due.repost;`. Result: "180 tests run: 180 passed". With this mutant a typed `inbox repost` at a new start, before the owner reopens, posts every envelope twice and adds a `reconnected` member. No case types `repost` at a new start.
- Fix lands in `tests/reconnect.rs`: a typed `repost` against a `Recorder` with every answer at a new start; assert one post per kept envelope and no `reconnected` member.

F7 (judgement), `crates/inbox-cli/tests/reconnect.rs:660`. The loop that looks for "POST", "HTTP/1.1", "Inbox-Signature" and "open-inbox" in the bytes sent to the service with the other certificate cannot fail. A client that skipped the pin check would send them TLS-encrypted, so they would never show in plain text. The `UnknownInbox` check at :669-677 is the part that holds. Fix: drop the loop or replace it with a byte count.

F8 (contract drift), `website/docs/limits.md:47`. `limits.md` and `status.md` promise that "the next command of the owner opens its inbox again" with no condition. It only happens when `.opened` holds a record, so an inbox opened with 0.3.1, or before this version on the host, is never reopened. `first-use.md` names a condition ("opened on this host"), but not the upgrade. Fix: one sentence in the two pages and in CHANGELOG.md (run `inbox open` once after upgrading).

Mutants (run in the copy, `cargo nextest run -p inbox-cli --locked`, 180 cases each):
- m1, the reconnect opens before the typed request: caught (23 failed, among them `reconnect::the_reconnect_opens_with_the_last_adapter_and_starts_no_reconnect_of_its_own` and `reconnect::a_client_that_never_opened_its_inbox_only_reposts`).
- m2, close keeps the record: caught by `reconnect::a_closed_inbox_is_not_opened_again`.
- m3, the reconnect's answers start another reconnect: caught by `reconnect::the_reconnect_opens_with_the_last_adapter_…` and `reconnect::a_service_gone_again_…`.
- m4, a refused envelope gets its epoch recorded: caught by `reconnect::a_refused_repost_…` and `epoch::a_post_refused_unknown_inbox_is_posted_again_in_the_same_start_until_it_arrives`.
- m5, fixed adapter "terminal": caught by `reconnect::after_a_restart_…` and `reconnect::the_reconnect_opens_with_the_last_adapter_…`.
- m6, no lock at all: caught, but only by accident. Without `lock()` the state directory is never created, so 4 unrelated cases exit 4 with "outbox.jsonl.opened: No such file or directory". m6b above isolates the lock itself.
- m6b, lock taken and released before the write: survived (F5).
- m7, typed repost guard removed: survived (F6).

What I attacked and could not break:
- A request in Pending twice within one start: the service answers a second post of a held message id `duplicate`.
- Two restarts in a row, and the owner's open arriving after the sender's repost: the epoch stays at the old start and the request arrives on the next command.
- Loops: one reconnect per command, bounded by 1 open + 1 post per kept envelope, also against a stand-in that answers every request from a new start.
- Whose inbox: the address is always `own_inbox()`. Records of other principals in a shared outbox are ignored, so `not-owner` cannot be reached.
- The pin: every request of the reconnect goes through `Service::send`, which checks the pin for each connection.
- `--json`: `with_member` re-serialises the document with numbers kept as written. Members a 0.3.1 script reads are unchanged.
- The `.opened` file: mode 0600, no secret, control characters escaped by `warn`.
- A close that fails in transport keeps the record.
- `reference.rs` matches `generated/cli/inbox.json`.

Not checked:
- The implementor's own race note (a close while another command's reconnect open is in flight): read only, no case written.
- A full disk at the rename, a symlink to `/dev/zero` (endless read), and lines of megabytes: not run.
- A newline in a service's refusal text breaking the "one line" on stderr: not run.

Paths written outside the worktree, all under <scratch>/s3/reconnect/adversary/:
- `mutants.sh`
- `copy/`: a git clone at 9f71690, files restored to HEAD, its `target` deleted
- `logs/`: alone.log, suite.log, clippy.log, m*.diff, m*.log, summary.txt, summary-first.txt, runner.out, runner2.out

```findings
[
{"file":"crates/inbox-cli/src/outbox.rs","line":212,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"A kept request with no recorded epoch (posted while the service was down, or refused UnknownInbox by the new start) is never posted again by any later command, although the pages say the sender's next command posts its kept envelopes by itself."},
{"file":"crates/inbox-cli/src/commands.rs","line":218,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"When removing the record fails after a close the service confirmed, the record stays and the next restart opens the closed inbox again, which then accepts posts."},
{"file":"crates/inbox-cli/src/outbox.rs","line":205,"category":"boundary","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"Outbox::due reads <outbox>.opened with fs::read_to_string after every client command, so a FIFO there blocks every command for ever, against the module's promise that reading it never fails a command."},
{"file":"crates/inbox-cli/src/main.rs","line":192,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The typed command's own output is printed only after the reconnect's requests end, so a stalled service delays it by up to 30 s per request, against default 2 A that the command answers as before."},
{"file":"crates/inbox-cli/src/outbox.rs","line":277,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Releasing the outbox lock before writing <outbox>.opened survives all 180 cases of inbox-cli, so no case shows that the record is written under the lock."},
{"file":"crates/inbox-cli/src/commands.rs","line":893,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Removing the typed != Typed::Repost guard survives the suite, so a typed repost at a new start may post every envelope twice and no case notices."},
{"file":"crates/inbox-cli/tests/reconnect.rs","line":660,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The needle loop over the bytes sent to the service with the other certificate cannot fail, because a client that skipped the pin check would send those words TLS-encrypted."},
{"file":"website/docs/limits.md","line":47,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"limits.md and status.md promise the owner's inbox is reopened with no condition, but an inbox opened before this version has no record in <outbox>.opened and is never reopened."}
]
```
