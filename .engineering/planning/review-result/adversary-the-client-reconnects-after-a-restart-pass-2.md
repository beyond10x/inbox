---
format: aep.planning-md/3
id: review-result:adversary-the-client-reconnects-after-a-restart-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the reconnect unit, round 2: 4 red cases, 0 surviving mutants of 11'
relations:
- reviews: story:the-client-reconnects-after-a-restart
revision: 1
---
unit: story:the-client-reconnects-after-a-restart, tree inbox-s3-reconnect at local commit cc7affe (round 2 is git diff 9f71690 HEAD, the unit is git diff 5c598a3 HEAD), plus my test file only
verdict: CONFIRMED (4 red cases, 0 surviving mutants)
cases: executed 191→195, red 4 (191 is the implementor's green count for round 2; each of the 11 mutant runs in the copy at cc7affe also ran 191)
origin: introduced 7 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, <scratch>/s3/reconnect/adversary2/
needs-coordinator: F1 and F3 pull in opposite directions. Posting an unmarked envelope collides with a post still on its way; not posting it loses a killed post. The fix needs one decision about how a post on its way is told apart from a dead one.

1. Diff: `crates/inbox-cli/tests/main.rs | 1 +` (`mod adversary_reconnect_2;`), plus the untracked `?? crates/inbox-cli/tests/adversary_reconnect_2.rs`. Both are test files. I changed no implementation, document or `.engineering/` file. I took a session lease on the tree (`worktree hook session-start`) and released it at the end.

2. Cases, in `crates/inbox-cli/tests/adversary_reconnect_2.rs`. Run alone first: `cargo nextest run -p inbox-cli --locked --no-fail-fast -E 'test(/^adversary_reconnect_2::/)'` gave "4 tests run: 0 passed, 4 failed", exit 100. All 4 are red now and each takes 0.65 to 1.33 s.
- `a_post_on_its_way_is_not_answered_duplicate_because_another_command_reposted_it`. alice keeps one note marked `none` (service down). Her second `post` is held by a proxy while her `outbox` runs. Its reconnect posts both notes, and the held post then gets `Duplicate`. Red at :214: "`inbox post` of a note that arrived exits non-zero because another command of the sender posted it while its own request was on its way: (Some(1), "", "inbox: refused: inbox.messaging.Duplicate (409)\n")". The earlier asserts passed, so both notes are in Pending once.
- `an_envelope_the_service_answered_duplicate_key_is_not_posted_again_by_every_command`. A post with `--dedup-key` fails in transport (marked `none`). The second post with the same key arrives, and its reconnect gets `duplicate-key` for the first. Red at :257: "after the service answered duplicate-key, each later command posted the envelope again", left: (["01M831C7PYFBHTDB6CHJQ2QNDH"], ["01M831C7PYFBHTDB6CHJQ2QNDH"]), right: ([], []). Each of those commands also wrote "inbox: reconnect at the start … reposted 1 kept envelope, 1 arrived" to standard error.
- `a_post_killed_on_its_way_is_posted_again_by_a_later_command_after_a_restart`. The post is killed while a stand-in holds its request: kept, no epoch, no mark. Then a restart, bob's command reopens his inbox, and alice runs `outbox` twice. Red at :310: "the request whose post was killed is in bob's Pending once", left: [], right: ["01M831C7ECFR6HPR5X4KCX0WC3"].
- `a_close_while_a_reconnects_open_is_on_its_way_leaves_the_inbox_closed`. After a restart, the reconnect of bob's `pending` sends an open that a proxy holds. Meanwhile bob's `close` removes the record and is refused `UnknownInbox`; then the open is released. Red at :350: "the inbox bob closed takes a post … pending said \"inbox: reconnect at the start e695… of the service: opened agent:test/bob again with the adapter terminal (opened)\"", left: 0, right: 1.

3. Suite, run after the cases existed: `cargo nextest run -p inbox-cli --locked --no-fail-fast` gave "Summary [28.240s] 195 tests run: 191 passed, 4 failed, 0 skipped", exit 100. The only failures are the 4 cases above. `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exit 0. `rustfmt --edition 2024 --check` is clean on both files.

4. Findings (tree at cc7affe)

F1, `crates/inbox-cli/src/commands.rs:836` (warning, CONFIRMED, introduced). The reconnect posts every kept envelope that has no epoch, including one another command of the sender still has on its way.
- What breaks: that command's `post` gets `Duplicate`. `final_refusal` (:636) drops the envelope and exits 1, though the message arrived.
- What reaches it: two commands of one sender at once while one envelope is marked or at an old epoch. A caller that retries on exit 1 posts the message twice under a new id.
- The doc of `Outbox::due` says such an envelope "starts none". That holds only until something else starts the repost.
- Fix: in `post`, take `Duplicate` with the outcome `duplicate` for its own freshly drawn id as arrived (record the epoch, exit 0). Or post in the reconnect only the envelopes that `due` counts.

F2, `crates/inbox-cli/src/commands.rs:869` (warning, CONFIRMED, introduced). A marked envelope answered `duplicate-key` counts as arrived but `holds` is false.
- What breaks: no epoch is recorded and the mark stays, so every later command of the sender posts it again, with one line on standard error, until it expires (24 h by default). The same happens to a marked envelope refused `413`.
- What reaches it: retrying a post with the same dedup key after a transport failure, which is what a dedup key is for.
- Fix: record the answer's epoch for `duplicate-key` too (this start answered it), or drop the envelope.

F3, `crates/inbox-cli/src/commands.rs:608` (warning, NEEDS-CHANGE, introduced). The mark `none` is written only after the answer.
- What breaks: a post killed after `keep` and before the answer (the user stops a hung command) leaves an envelope with no epoch and no mark. No command posts it again, also after a restart.
- This contradicts `website/docs/limits.md:49`: "the next command of the sender posts its kept envelopes again, by themselves".
- Fix: write the mark with `keep`, in the same change under the lock. It needs a way to tell a post on its way from a dead one (for example the pid and start time of the posting process in the mark), because of F1. Or say in limits.md that such an envelope needs a typed `inbox repost`.

F4, `crates/inbox-cli/src/commands.rs:952` (warning, CONFIRMED, introduced). The reconnect reads `.opened`, then sends its open with no hold on the record across the request.
- What breaks: a `close` between the two is undone at the service, and the inbox the owner closed accepts posts for the rest of that start. `held_only` only keeps the record from coming back.
- By reading, the same shape exists between a typed `open` whose record lands after `close` took it and a close the service then does (record present, service closed).
- What reaches it: two commands of one owner at once after a restart (a polling agent and a typed `close`).
- Fix: when `record_opened(held_only)` finds no record after a successful open, the reconnect sends `CloseInbox` to undo it.

F5, `CHANGELOG.md` (note, CONFIRMED, introduced). Two old-version paths lose marks, and the upgrade note names only `inbox open`.
- 0.3.1's `Epochs::lines` (`git show 0.3.1:crates/inbox-cli/src/outbox.rs`, :291-303) writes only recorded epochs. So any 0.3.1 command that records an epoch drops every `none` line. 0.3.1 reads such a line as "no epoch" and does not fail.
- Envelopes that failed under 0.3.1 carry no mark, so the new binary never posts them by itself.
- Fix: add "run `inbox repost` once after the upgrade" beside "run `inbox open` once".

F6, `website/docs/first-use.md:143` (note, CONFIRMED, introduced). Two document lines disagree with the code.
- first-use.md:143 and `CHANGELOG.md:40` say `close` puts the record back when "the service refused it". The code does not put it back on `UnknownInbox` (commands.rs:213). `docs/design/service.md:95` gets this right.
- `docs/design/service.md:104` says a failed reconnect step "records nothing". `repost_at` (commands.rs:844) writes `none` for an unmarked envelope at no epoch refused `UnknownInbox`.

F7, `crates/inbox-cli/tests/reconnect.rs:896` (note, CONFIRMED, introduced). `blocked_opening_a_fifo` falls back to the first field `257` of `/proc/<pid>/syscall`.
- That field is `openat` on x86-64 only, and it shows any sleeping `openat`, for example one waiting on I/O before the lock is taken. On a stalled disk, part B can fail on correct code.
- On a non-x86 host where `wchan` is hidden or the function is inlined (`fifo_open`), part B fails on correct code after 10 s.
- What it needs from the kernel: `/proc/<pid>/wchan` with symbol names, or x86-64 `/proc/<pid>/syscall`, ptrace read access to the child (yama scope 1 or lower; the test is the parent), and `/proc/locks`.
- On this host `wchan` reads `wait_for_partner` (probed), and ubuntu-latest is x86-64, so it holds there.
- Fix: rely on `wchan == wait_for_partner` alone, or also check that the target of `/proc/<pid>/fd` is the FIFO. The `/proc/locks` waiter check in part A is sound.

F8, `crates/inbox-cli/src/outbox.rs:394` (note, CONFIRMED, pre-existing). A change with epochs reads an unreadable epochs file as empty, then rewrites it with only the change's own entry.
- "Unreadable" here means mode 000, not UTF-8, over the bound, or no regular file. Every other epoch and mark is lost, with no warn line for EACCES or non-UTF-8.
- The base reads it the same way: `fs::read_to_string(..).unwrap_or_default()` at 5c598a3:outbox.rs:210.
- The new `.opened` file behaves the same way for other inboxes' records; that loss is in the safe direction (an inbox is not reopened).
- No envelope is lost: the outbox file itself fails closed. Nothing in the product reaches this state.

Mutants: 11, run in the copy with `cargo nextest run -p inbox-cli --locked`, 191 cases each. All are caught.
- m1, the mark also written for an envelope with an epoch (`Epochs::mark_unsent` drops the epoch; `repost_at` without `entry.epoch.is_none()`): caught by `outbox::tests::a_reconnect_is_due_where_…` and `reconnect::a_refused_repost_changes_nothing_…`.
- m2, the mark never removed on arrival (`record` keeps `unsent`; `lines` writes `none` first): caught by `epoch::a_post_refused_unknown_inbox_is_posted_again_…`, `epoch::a_post_that_arrived_records_the_epoch_…` and the `due` unit case.
- m3, `due` does not leave out the typed command's own envelope: caught only by the unit case `outbox::tests::a_reconnect_is_due_where_…`; no integration case.
- m4, the record put back after `UnknownInbox`: caught by `reconnect::a_closed_inbox_is_not_opened_again`.
- m5, the record not put back after a transport failure: caught by `reconnect::a_close_that_failed_in_transport_or_was_refused_keeps_the_record`.
- m6, the bound of `.opened` raised to 1 TiB: caught by `reconnect::a_file_of_the_opened_inbox_over_its_bound_holds_nothing`.
- m7, the regular-file check dropped: caught by `reconnect::a_link_to_a_device_…`, through its message text only. O_NONBLOCK alone already keeps a FIFO from hanging.
- m8, the text output written after the reconnect: caught by `adversary_reconnect::the_typed_command_prints_its_own_answer_before_a_stalled_reconnect_ends`.
- m9 (extra), any envelope with no epoch counts as due: caught by `epoch::an_unreadable_epochs_file_…`, `tls::a_post_over_the_request_limit_…` and the `due` unit case.
- m10 (extra, pass 1's m6b), the lock released before `.opened` is written: caught by `reconnect::the_record_of_the_opened_inbox_is_written_under_the_lock` (":964 the open wrote the staged file without the lock: holders [], not 2670771").
- m11 (extra), `put_back_opened` overwrites a newer record: caught only by the `due` unit case.

5. What I attacked and could not break:
- A marked envelope the service already holds (the answer was lost): by reading, the repost gets `duplicate`, the epoch is recorded, the mark is removed, and it is in Pending once.
- A mark lost to a concurrent rewrite: every change of the epochs file is a read, change and replace under `<outbox>.lock`.
- A model refusal or `AlreadyExpired` of a marked envelope drops it, and its line goes with the next write. An expired envelope is left out by `unexpired`.
- `close`, killed between remove and send or between send and put back: the record is gone and the service is open, which is the safe direction.
- A close that failed in transport after the service did it gets its record put back and is reopened after a restart. The user saw exit 3, and a retry of `close` gets `UnknownInbox`. Not raised.
- The outbox file taken as holding nothing and then rewritten: it cannot happen. `change` reads it with `read_bounded`, and any error fails the command with exit 4 before a write.
- Bounded reads: the bound is held by `take(most+1)` also for a file that grows. The `.opened` bound is covered by a case and m6. A link to a regular file is followed without a word, as the doc comment says. A directory is refused with a warn line. A socket and mode 000 hold nothing silently.
- Output order: `print` flushes after each line, so a pipe or file gets the command's text before the first request of the reconnect. `--json` prints exactly one document, also when the reconnect fails. The exit status comes from the typed command only.
- The 4 changed older cases are not weakened. `tls::a_client_with_the_pin_…` still asserts that bob's Pending is empty right after the wrong-pin post; at base it never counted bytes. `epoch.rs` and `adversary_w8_repost.rs` now assert more lines, not fewer.
- Help of `--json`: `generated/cli/inbox.json` matches `main.rs`.

Not checked: a 64 MiB outbox at the bound and 1 byte over (not run, short disk); a downgrade run of a 0.3.1 binary (code read only).

6. Paths written outside the worktree, all under <scratch>/s3/reconnect/adversary2/:
- `copy/`: a `git archive` of cc7affe, its sources restored to pristine, its `target` deleted.
- `pristine-src/`: a copy of `crates/inbox-cli/src` at cc7affe.
- `mutants/m1` … `mutants/m11`: the mutated files.
- `mutants.sh`, `fifo-probe.sh`.
- `logs/`: alone.log, suite.log, clippy.log, copy-build.log, m1.log … m11.log, summary.txt, mutants-runner.out.

7.
```findings
[
{"file":"crates/inbox-cli/src/commands.rs","line":836,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"The reconnect posts every kept envelope at no epoch, also one another command of the sender has on its way, so that command's post is answered Duplicate, drops its envelope and exits 1 for a message that arrived."},
{"file":"crates/inbox-cli/src/commands.rs","line":869,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A marked envelope answered duplicate-key (or 413) keeps its mark and no epoch, so every later command of the sender posts it again with a line on standard error until it expires."},
{"file":"crates/inbox-cli/src/commands.rs","line":608,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The mark none is written only after the answer, so a post killed on its way leaves an envelope that no later command posts again, also after a restart, against limits.md's promise that the sender's next command posts its kept envelopes by itself."},
{"file":"crates/inbox-cli/src/commands.rs","line":952,"category":"concurrency","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"A close that runs while a reconnect's open is on its way is undone at the service, so the inbox the owner closed accepts posts while the client's record says closed."},
{"file":"CHANGELOG.md","category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"0.3.1 drops every none line when it rewrites the epochs file and envelopes that failed under 0.3.1 carry no mark, yet the upgrade note names only inbox open and not a typed inbox repost."},
{"file":"website/docs/first-use.md","line":143,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"first-use.md and CHANGELOG.md say close puts the record back when the service refused it, which is false for UnknownInbox, and service.md:104 says a failed reconnect step records nothing although repost_at writes none marks."},
{"file":"crates/inbox-cli/tests/reconnect.rs","line":896,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The fallback on /proc/<pid>/syscall number 257 is x86-64 only and matches any sleeping openat, so part B of the lock case can fail on correct code on a stalled disk or on a non-x86 host without a readable wchan."},
{"file":"crates/inbox-cli/src/outbox.rs","line":394,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"An unreadable epochs file is read as empty inside a change and rewritten with only that change's entry, silently dropping every other epoch and mark."}
]
```
