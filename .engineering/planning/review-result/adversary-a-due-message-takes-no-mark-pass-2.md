---
format: aep.planning-md/3
id: review-result:adversary-a-due-message-takes-no-mark-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the due message that takes no mark: 4 findings of pass 1 closed, 4 new'
relations:
- reviews: story:a-due-message-takes-no-mark
revision: 1
---
unit: story:a-due-message-takes-no-mark, round 2 of the implementor. Uncommitted working tree on branch fix/due-message-takes-no-mark (base 7cd7416), <tree>
verdict: NEEDS-CHANGE (1 red case: a wait that slept on due rows answers rows the sweep already removed)
cases: executed 393→399 for inbox-core + inbox-server + inbox-cli (393 is the after-run minus my 6 cases), red 1. For the gate's test step this means 650→656.
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 logs in <scratch>/d30/mark/adversary2/, plus that directory itself
needs-coordinator: the branch's base 7cd7416 predates release 0.3.1 (c187e22 on main). The CHANGELOG entry correctly says "up to and including 0.3.1", but this tree has no [0.3.1] heading, and [Unreleased] will conflict when the branch is rebased or merged onto main.

1. Diff stat (only my part; everything else in `git status` belongs to the unit)
- `git diff --stat -- crates/*/tests/main.rs`: each main.rs shows the unit's lines plus mine. Mine are 1 line in each of crates/inbox-core/tests/main.rs (`mod adversary_due_api;`) and crates/inbox-server/tests/main.rs (`mod adversary_deadline_wait;`).
- New files: crates/inbox-server/tests/adversary_deadline_wait.rs, crates/inbox-core/tests/adversary_due_api.rs.
- Every path I touched is a test file. No implementation, ess/, generated/, planning or document file was touched.

2. Cases added (first run: the new cases alone, `ESS_TOOLCHAIN=0.97.3 SLOT_TIMEOUT=20m cargo nextest run -p inbox-server -p inbox-core --locked -E 'test(/adversary_deadline_wait|adversary_due_api/)'`, result: 6 run, 5 passed, 1 failed)
- The server cases read a probe clock: the world's ManualClock, with every reading logged together with the thread that took it. On a wait's thread, reading 1 is the request's instant and reading 2 is the one after the view read, just before `wait_for_command`.
- RED `a_wait_that_slept_on_due_rows_answers_what_pending_lists_after_the_sweep` (0.6 to 0.7 s). Setup: a due note; the wait (max_wait 300 ms) has read its view and slept; `sweep_now()` expires the note (returns 1). Plain `pending` then lists nothing, but the wait answers the removed note. Verbatim:
  panicked at crates/inbox-server/tests/adversary_deadline_wait.rs:182:5:
  assertion `left == right` failed: the wait answered a row the sweep removed before the wait ended: {"rows":[{"message_id":"01JD0000000000000000000001",…,"expires":"2026-12-01T00:00:00Z",…,"state":"Sent"}]}
    left: ["01JD0000000000000000000001"]
   right: []
- GREEN `a_live_post_between_the_view_read_and_the_sleep_ends_the_wait_every_round` (0.9 to 1.8 s). The lost wake-up race, 5 rounds in one service. Each round posts a live note on the wait's own thread, inside its reading 2, so the post lands between the view read and the sleep every time. The wait ends within 5 s with the note listed. Each round asserts exactly 3 readings on the wait's thread, which proves the post landed in that window and the wait read the view once more.
- GREEN `one_wait_of_a_second_on_a_note_due_at_the_reading_reads_its_view_once` (1.3 s). The note's `expires` equals the clock reading. With `wait=1` and max_wait 5 s, the wait holds at least 1 s and under 3 s, lists the note, and takes exactly 2 readings, so the view is read once. No spin.
- GREEN `a_live_note_of_another_sender_neither_ends_nor_shows_in_a_senders_wait` (0.6 to 0.75 s). AGENT waits on OTHER's Pending holding only its own due note. A live note from THIRD to OTHER does not end AGENT's wait before the bound, and AGENT's answer lists only [1].
- GREEN `stopping_the_service_ends_a_wait_that_holds_only_a_due_note` (0.24 to 0.6 s). The stop ends a wait sleeping on due rows: answer 200, well within 5 s.
- GREEN core `due_answers_as_its_documentation_says_for_instants_and_for_text_that_names_none` (0.01 to 0.04 s). Covers equality, a fraction, offsets, lower case `t`/`z`, and 1 ns after. Empty, non-time, 30 February, no zone and a space separator give None, as does an empty `now`.
- All 6 cases stay under 3 s. The red case also stays red in the full run.

3. Suite run (after the cases existed)
- `ESS_TOOLCHAIN=0.97.3 cargo nextest list -p inbox-core -p inbox-server -p inbox-cli --locked`, exit 0.
- `ESS_TOOLCHAIN=0.97.3 SLOT_TIMEOUT=20m cargo nextest run -p inbox-server -p inbox-core -p inbox-cli --locked --no-fail-fast`: "Summary [51.764s] 399 tests run: 398 passed, 1 failed, 0 skipped", exit 100. The only failure is the red case above.
- Slowest tests (none are mine): cli adversary::add_key_processes_at_once_lose_no_entry 8.5 s; core adversary_deadline::every_late_order_leaves_a_decision_with_default… 7.2 s; server foreign_message::a_third_principal_gets_the_bytes… 7.1 s.
- Real case counts with my cases removed match docs/checks.md:
  - inbox-core: 194 listed − 1 = 193 (doc: 193).
  - inbox-server: 86 main + 6 lib = 92 − 5 = 87 (doc: 87).
  - inbox-cli: 87 main + 26 bin = 113 (doc: 113).

Pass-1 findings
- F1 closed. A due row no longer ends a wait. With only a due note, a 1 s wait holds 1 s and reads its view once (case above). The unit's `a_wait_with_only_a_due_note_ends_at_its_bound_and_lists_the_note` and the rewritten pass-1 case pass in the suite run. Residue: finding A below.
- F2 closed. crates/inbox-server/src/clock.rs:13 and :117, and the `--clock-start` doc at crates/inbox-cli/src/main.rs:207, now name both marks and the wait (`git diff crates/inbox-server/src/clock.rs crates/inbox-cli/src/main.rs`).
- F3 closed. website/docs/status.md:23 and the `inbox read` help (main.rs:89-91, generated/cli/inbox.json) now say that the read of a message past its `expires` is refused with `Expired`, exit 1.
- F4 closed. `git show c187e22:ess/domains/messaging.yaml | grep -n deadline-passed` shows only Answer (682) and Refuse (762): 0.3.1 had no mark deadline, so "up to and including 0.3.1" is right. The rebase residue is under needs-coordinator above.
- Rewritten pass-1 case: `diff <scratch>/d30/mark/adversary_deadline_mark.rs.orig crates/inbox-server/tests/adversary_deadline_mark.rs` changes only the module doc, adds a BOUND const, and renames and rewrites the one wait case. That case keeps every original assertion (status, ids == [1], read refused again each round) and adds `held >= BOUND`. The other 2 server cases are byte-identical. The core adversary file was last modified at 20:07, before the pass-1 record (20:17): untouched. Nothing was weakened.

4. Findings (working tree on 7cd7416)
- A (warning, NEEDS-CHANGE, introduced), crates/inbox-server/src/service.rs:204-206 (and crates/inbox-server/src/expiry.rs, `sweep`, which sends no signal).
  - Measured: the red case at adversary_deadline_wait.rs:182.
  - Cause: when the wait reaches its deadline it returns `answered`, the view read before it slept. The expiry task never calls `command_answered`, so a wait sleeping on due rows does not see that the sweep removed them, and answers rows that `pending` no longer lists.
  - This contradicts docs/design/service.md:231-232 ("answered as without `wait`: every row") and the CHANGELOG ("answers the whole view").
  - Introduced: at the base, a wait with any row returned at once, and only an empty view was ever held.
  - What reaches it: any wait on due rows while the expiry task runs (defaults: sweep every 10 s, max_wait 60 s). The wait ends up to about 50 s later listing removed messages; `inbox read` of one then answers `unknown-message`.
  - Fix (named, not applied): `sweep_now` calls `shared.signals.command_answered()` when it sent an Expire, or `waited` reads the view once more before answering at its deadline.
- B (note, CONFIRMED, introduced), ess/domains/messaging.yaml:987 and docs/design/inbox.md:305 still say `inbox wait` is Pending "held open until it has a row". A due row no longer ends it. The OpenAPI descriptions generated from the summary (generated/openapi/openapi/inbox-service.yaml:1041, :3007) inherit the stale text.
- C (note, CONFIRMED, introduced), crates/inbox-core/src/lib.rs:47: `inbox_core::due` is now public and re-exported, but neither the crate's module doc list (lib.rs:7-30) nor the crates/inbox-core row of AGENTS.md names it. Its behaviour matches its own doc comment (green core case).
- D (note, CONFIRMED, introduced), crates/inbox-cli/src/main.rs:91: the `inbox read` help says of a due message "the expiry task removes it". A decision with `default_on_silence` is instead decided on silence and kept for collection, as docs/design/service.md:195-196 says ("removes it or decides it on silence"). The same text is in generated/cli/inbox.json.

5. Attacked and not broken
- A live row beside a due row ends the wait at once (unit case `a_note_that_is_not_due_ends_a_wait_at_once` plus the race case).
- The lost wake-up between the view read and the sleep: 5 rounds, deterministic window, no loss.
- `expires` equal to the reading counts as due for the wait, as for the marks.
- No spin: exactly 2 clock readings for a 1 s wait.
- The request's own seconds bound holds (1 s under max_wait 5 s), and so does max_wait (300 ms under 3600, unit and pass-1 cases).
- The stop ends a due-only wait.
- A second sender's live row neither ends nor shows in a sender's wait. The read rule is applied before `has_rows_not_due`, so hidden rows never count.
- Marks over the core and HTTP on a due message, and both orders around the sweep: the unit's and pass-1 cases are green in the suite run.
- `inbox_core::due` gives no wrong answer for empty or unparsable text: it answers None. A row the wait cannot read counts as not due, but no such row can be stored, because the post refuses an unreadable `expires`.
- Not run: a ManualClock moved back while a wait sleeps (nothing wakes the wait; reached only by a backward step of the system clock), and a closed client connection, which this change does not touch.

6. Paths written outside the worktree
- <scratch>/d30/mark/adversary2/ (directory created for this pass)
- <scratch>/d30/mark/adversary2/new-cases.log
- <scratch>/d30/mark/adversary2/list.log
- <scratch>/d30/mark/adversary2/suite.log

7. Findings block

```findings
[
  {"file": "crates/inbox-server/src/service.rs", "line": 206, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A wait that slept on due rows answers, at its bound, the view it read before sleeping; the expiry task sends no command signal, so it lists messages the sweep already removed and that pending no longer lists (red case adversary_deadline_wait.rs:182)."},
  {"file": "ess/domains/messaging.yaml", "line": 987, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The Pending summary (and docs/design/inbox.md:305, and the generated OpenAPI descriptions from it) still says inbox wait is held open until it has a row, which a due row no longer ends."},
  {"file": "crates/inbox-core/src/lib.rs", "line": 47, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "inbox_core::due is now public and re-exported but is not named in the crate's module doc list or in the inbox-core row of AGENTS.md."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 91, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The inbox read help says the expiry task removes a due message, but a decision with default_on_silence is decided on silence and kept for collection, as docs/design/service.md says."}
]
```
