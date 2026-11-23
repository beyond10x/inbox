---
format: aep.planning-md/3
id: review-result:adversary-the-client-reconnects-after-a-restart-pass-3
kind: review-result
status: active
title: 'Adversary pass 3 on the reconnect unit, round 3: 3 red cases, 2 surviving mutants of 6'
relations:
- reviews: story:the-client-reconnects-after-a-restart
revision: 1
---
unit: story:the-client-reconnects-after-a-restart, round 3, tree inbox-s3-reconnect at HEAD ae2a65e (round 3 = ee3c728..ae2a65e); cases run in a copy of HEAD
verdict: NEEDS-CHANGE
cases: executed 197→201 (the 5th added case ran alone, at HEAD and under m3), red 3
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (part 6)
needs-coordinator: whether finding 3 (a 503 "stopping" clears the mark) changes decision F3; the F2 deviation (note 6)

1. git --no-pager diff --stat of the tree: empty. `git status --short` is empty too. I wrote nothing in the tree. My two changes are in the copy only: the new file `crates/inbox-cli/tests/adversary_reconnect_3.rs` and one `mod adversary_reconnect_3;` line in `crates/inbox-cli/tests/main.rs`.

2. Cases, in `<scratch>/s3/reconnect/adversary3/copy/crates/inbox-cli/tests/adversary_reconnect_3.rs`. The first 4 were written before anything ran and first run alone (`-E 'test(/adversary_reconnect_3/)'`, log `alone.log`). Each takes about 2 s.
- a_reconnects_close_after_closed_meanwhile_does_not_undo_a_typed_open_that_came_between: RED now.
  `left: 1 right: 0` "bob's last command opened his inbox (Run { status: 0, stdout: "already-open agent:test/bob\n" ..}); the reconnect's CloseInbox that reached the service after it closed it: Run { status: 1, stderr: "inbox: refused: inbox.messaging.UnknownInbox (409)\n" }; pending said "... agent:test/bob was closed meanwhile: not opened again""
- a_reconnects_open_on_its_way_does_not_overwrite_the_record_of_a_typed_open_after_a_close: RED now.
  `left: "terminal" right: "slack"` "bob's last typed open chose `slack`; the reconnect's open that was on its way wrote its own adapter over that record; pending said "... opened agent:test/bob again with the adapter terminal (already-open)""
- a_post_the_stopping_service_answered_503_is_posted_again_by_a_later_command: RED now.
  `left: [] right: ["01M834G20XBJ62GR57T3NNJ9R7"]` "the post the stopping service answered 503 never reached it, and alice's next command after the restart did not post it again (epochs file after the 503: [])"
- two_commands_that_post_one_envelope_at_once_leave_one_epoch_and_no_mark: GREEN (2.7 s). It holds attack 3: both posts exit 0, Pending holds each message once, and the epochs file has one epoch per envelope and no mark.
- a_typed_post_refused_duplicate_key_exits_1_and_keeps_nothing: GREEN at HEAD (2.1 s), RED under m3 (`left: 0 right: 1`, `m3case-m3.log`). It was added after m3 survived the suite.

3. Suite, run after the first 4 cases existed: `SLOT_TIMEOUT=30m cargo nextest run -p inbox-cli --locked --no-fail-fast` gave "Summary [23.594s] 201 tests run: 198 passed, 3 failed, 0 skipped", exit 100. The 3 failures are my 3 red cases. `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0; its only warning is in generated `wire.rs`.

4. Findings
F1. The reconnect's CloseInbox closes an inbox the owner opened again (`crates/inbox-cli/src/commands.rs:1010`, `closed_meanwhile` at :1075).
- What was measured: case 1. Bob runs `close`, the reconnect's open finds no record and sends CloseInbox (held by the proxy), then bob runs `open`, which writes a record. The held close then closes the inbox. The record says open, the service holds it closed, and alice's post is refused UnknownInbox until the next restart.
- What reaches it: bob types `close` then `open` while another of his commands reconnects. It is the same race shape as pass 2's case 4. `docs/design/service.md:115` covers only an open and a close typed at once, not a close done by the reconnect after a typed open.
- Where the fix lands: `closed_meanwhile`. After its CloseInbox, read the record again; when one is there now, open again with its adapter.
- NEEDS-CHANGE, introduced (round 3).

F2. The reconnect's open overwrites a typed open's record (`crates/inbox-cli/src/outbox.rs:353`, `record_opened` with `held_only` checks only that some record exists).
- What was measured: case 2. Bob runs `close` and then `open --adapter slack` while the reconnect's open (`terminal`) is on its way. The service keeps `slack` (`already-open`), and the record is rewritten to `terminal`. The adapter bob chose last is lost, and the reconnect after the next restart opens with the old one.
- Where the fix lands: `record_opened`. With `held_only`, replace the record only while it is the one the reconnect started from (compare-and-set on adapter and epoch).
- NEEDS-CHANGE, introduced.

F3. A post the stopping service answered `503` loses its mark and is never posted by itself (`crates/inbox-cli/src/commands.rs:658`).
- What was measured: case 3, with a stand-in giving the listener's own answer `503 {"refused":"stopping"}` (`crates/inbox-server/src/listener.rs:313`). The listener writes that answer for a request still arriving when the service stops, and stores nothing. `post` takes it as "a refusal of the service's own" and clears the mark, so the sender's next command after the restart does not post the envelope.
- The same path clears the mark on a `408` (stalled bytes, also not stored) and on the `503` "message limit".
- It contradicts `CHANGELOG.md:35` ("was stopped on its way ... keeps its mark") and `website/docs/first-use.md:151` ("has not reached the service ... or was stopped before an answer").
- What reaches it: a restart of the service (the subject of this story) while a post's bytes are still arriving. The window is narrow but it is the restart itself.
- Where the fix lands: `post` and `repost_at`. Treat 408 and 503 like a transport failure (keep the mark); clear it only for refusals tied to the request itself (400, 413, and 401 by decision).
- Warning, NEEDS-CHANGE, introduced (this story's mark mechanism).

F4. Surviving mutant m2: the mark is kept on an arrival without an epoch (`commands.rs:633`).
- Diff: `-                None => client.outbox.clear_unsent(&message_id, now()),` / `+                None => Ok(()),`. The suite stays green: 197 passed.
- What reaches it: an answer to the post without `Inbox-Epoch` (an older service) while a later answer carries one. Nothing found beyond an upgrade.
- Note, CONFIRMED, introduced.

F5. Surviving mutant m6: `keep` writes the mark in a second change (`outbox.rs:158`).
- Diff: `-        let unmarked = self.change_both(now, true, |lines, epochs| { lines.push(..); epochs.mark_unsent(..) })?;` / `+        self.change(now, false, |lines, _| lines.push(text.to_owned()))?;` / `+        let unmarked = decode(text).and_then(|e| self.mark_unsent(&e.message_id.0, now).err());`. The suite stays green: 197 passed.
- No case holds the "same change" property. Only a process killed between the two renames shows it, and I found no deterministic way to stop the client there.
- Note, CONFIRMED, introduced.

F6. The implementation differs from coordinator decision F2 (`docs/design/service.md:108`).
- The round-3 brief decided that `duplicate-key` in the reconnect's repost records the epoch. The code clears the mark and writes no epoch, and the design says so.
- Mutant m4 (record the epoch) is caught by the older case `adversary_w8_repost::a_post_refused_duplicate_key_is_posted_again_once_the_message_holding_the_key_is_gone`. That case contradicts decision F2, which is probably why the implementor deviated.
- Such an envelope stays in the outbox until its `expires`, which `--expires` sets with no upper bound. Every typed `inbox repost` posts it again and reports it "arrived (Duplicate)".
- The coordinator decides whether this stands. Note, CONFIRMED, introduced.

Mutants (suite without my file, `mutants.summary`):
- m1, the mark cleared on a transport failure in `post`: killed by `reconnect::a_marked_envelope_refused_413_by_a_reconnect_is_not_posted_again`, `adversary_reconnect::a_request_posted_while_the_service_was_down_arrives_after_the_restart_by_itself`, `adversary_reconnect_2::an_envelope_the_service_answered_duplicate_key_...` and `tls::a_client_with_the_pin_posts_over_tls_...` (4 failed).
- m2, the mark kept on an arrival without an epoch: SURVIVED (F4).
- m3, `own_duplicate` true for `duplicate-key`: SURVIVED the suite (197 passed). Now caught by my case `a_typed_post_refused_duplicate_key_exits_1_and_keeps_nothing`.
- m4, `repost_at` records an epoch for `duplicate-key`: killed by `adversary_w8_repost::a_post_refused_duplicate_key_is_posted_again_once_the_message_holding_the_key_is_gone`.
- m5, `closed_meanwhile` sent also when the record is there: killed by 9 cases, among them `reconnect::after_a_restart_the_next_commands_open_the_inbox_again_...` and `reconnect::the_reconnect_opens_with_the_last_adapter_...`.
- m6, the mark written after the envelope in a second change: SURVIVED (F5).

5. Attacked and could not break:
- `own_duplicate` exits 0 only for a message the service holds. `duplicate` needs a live message of this sender with this id, `post` takes no id from the command line, and the `--json` `answer` is the service's own refusal body.
- Two commands at once, in both orders: one message in Pending, both exit 0, one epoch and no mark (case 4). The restart variant was checked by reading only.
- `change_both`: when only the epochs file fails, the envelope is kept once without its mark. When the outbox file fails, `keep` fails before the request, as before.
- A pin mismatch or wrong usage after `keep`: the mark stays, and no reconnect runs without an answer.

Not checked:
- Three commands at once.
- The restart-between-requests case as a test.
- The failure of `closed_meanwhile`'s CloseInbox (one warn line, exit unchanged), checked by reading only.
- The `blocked_opening_a_fifo` fallback on aarch64.
- The doc and changelog wording beyond the lines named in F3 and F6.

6. Written outside the tree:
- `<scratch>/s3/reconnect/adversary3/`, holding:
  - `copy/`: sources only, `target` deleted. It holds `crates/inbox-cli/tests/adversary_reconnect_3.rs` and the `mod` line in `crates/inbox-cli/tests/main.rs` for you to move. Its `src` matches tree HEAD.
  - Logs: `alone.log`, `suite.log`, `mutant-m1..m6.log` and `.diffstat`, `mutants.summary`, `m3case-head.log`, `m3case-m3.log`, `clippy.log`.
  - `src.diff`.
  - Scratch scripts `mutants.sh` and `m3case.sh`, not for the tree.

```findings
[
  {"file": "crates/inbox-cli/src/commands.rs", "line": 1010, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The CloseInbox that closed_meanwhile sends reaches the service after a typed inbox open that came between, so the record says open while the service holds the inbox closed and posts to it are refused until the next restart."},
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 353, "category": "concurrency", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "record_opened with held_only replaces any record present, so a reconnect's open that was on its way overwrites the adapter of the owner's later close-then-open with the old adapter."},
  {"file": "crates/inbox-cli/src/commands.rs", "line": 658, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A post answered 503 stopping (or 408) was never stored, yet post clears its mark, so no command posts it after the restart, contrary to CHANGELOG.md:35 and first-use.md:151."},
  {"file": "crates/inbox-cli/src/commands.rs", "line": 633, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The mutant that keeps the mark on an arrival whose answer names no epoch survives the inbox-cli suite."},
  {"file": "crates/inbox-cli/src/outbox.rs", "line": 158, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The mutant that writes the mark in a second change after the envelope survives the suite; no case holds that keep writes both in one change."},
  {"file": "docs/design/service.md", "line": 108, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "duplicate-key in the reconnect clears the mark and records no epoch, unlike coordinator decision F2, so the envelope stays until its unbounded expires and every typed repost posts it again."}
]
```
