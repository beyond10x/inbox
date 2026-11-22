---
format: aep.planning-md/3
id: review-result:adversary-a-due-message-takes-no-mark-pass-1
kind: review-result
status: active
title: Adversary on the story of marks on a due message, pass 1
relations:
- reviews: story:a-due-message-takes-no-mark
revision: 1
---
unit: story:a-due-message-takes-no-mark, the uncommitted working tree on branch fix/due-message-takes-no-mark (base 7cd7416), <tree>
verdict: CONFIRMED (judgement findings only; no red case)
cases: executed 641→647 expected (641 is the implementing state's `task check` count; I did not run the whole gate, as the brief forbids). Filtered runs after my cases: inbox-core `/deadline_mark/` 7 (4 from the unit + 3 mine), inbox-server `/deadline_mark/` 6 (3 + 3). red 0
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 9 log files, all under <scratch>/d30/mark/adversary/
needs-coordinator: if my 6 cases are kept, the hand-typed counts in docs/checks.md go stale (core 190→193, server 81→84; no check enforces them). Finding 1 needs a decision: keep the behaviour as designed, or change it.

1. Diff stat (only my part of the tree; the rest of `git status` is the unit's)
- new: crates/inbox-core/tests/adversary_deadline_mark.rs (363 lines)
- new: crates/inbox-server/tests/adversary_deadline_mark.rs (173 lines)
- crates/inbox-core/tests/main.rs: +1 line `mod adversary_deadline_mark;`
- crates/inbox-server/tests/main.rs: +1 line `mod adversary_deadline_mark;`
- `git diff --stat` on the two main.rs files shows "2 ++" each: the unit's own `mod deadline_mark;` plus my line.
- Every path is a test file. No implementation, ess/, generated/, document, nextest config or test-step file was touched.

2. Cases added. All 6 are green. None was red when first run. Each pins a claim that no earlier case pinned.

inbox-core (fixed `Timestamp` clock, no real time):
- `a_due_message_answered_or_refused_keeps_already_read_and_already_delivered`: PASS, 0.10–0.15 s.
  - Covers a request answered before its deadline, a request refused before it, and a decision the sweep decided on silence.
  - At `expires` and 1 s after, MarkRead answers `already-read` and MarkDelivered `already-delivered`, with no event and no row changed. This is the claim at docs/design/service.md:194.
- `a_mark_is_refused_exactly_where_the_sweep_at_that_reading_takes_the_message`: PASS, 0.26–0.77 s.
  - `expires` is written 4 ways: with a fraction, `+01:00`, lower-case `t`/`z`, and `-01:00`.
  - Readings: 1 ns before, at, 1 ns after, and the whole second.
  - Note and request, both marks: the mark answers `deadline-passed` exactly where `Expire` at the same reading answers `expired`, and is taken where `Expire` answers `not-due`.
- `no_caller_but_the_recipient_tells_a_due_message_from_a_live_one`: PASS, 0.16 s.
  - Callers through `handle_with_events`: the sender; a third principal; the sender without its inbox; the sender's inbox without a principal; a third principal without its inbox; the recipient's principal with another inbox; a Client with no attributes; the Sweeper.
  - For each caller and each mark, the answer 1 s before and 1 s after the deadline is identical member for member, with no event, no row changed, and no `deadline-passed` or `Expired`.

inbox-server (the test's `ManualClock`):
- `a_mark_at_a_fraction_of_a_second_is_refused_exactly_where_the_sweep_removes_the_message`: PASS. `expires` is `…00:00:00.5Z`, readings are 1 ns before and at it. `sweep_now()` and `read`/`mark-delivered` agree for a note and a request (502 with the generated body, no row changed / 202).
- `the_sender_and_a_third_principal_cannot_tell_a_due_message_from_a_live_one`: PASS. Status and body bytes 1 s before and 1 s after the deadline are identical for the sender and a third principal, for both routes, note and request.
- `wait_answers_a_due_note_at_once_every_time_after_its_read_was_refused`: PASS. This is the evidence for finding 1. After a refused `read` and a refused `mark-delivered`, `GET pending?…&wait=1` answers the due note 3 times in a row, and each `read` after it is refused again. No wait on real time: `waited` (crates/inbox-server/src/service.rs:180) returns on the first read that has a row. My first version also asserted elapsed time under 500 ms. It failed once only because the host was loaded ("round 0: the wait held 1.156236652s"), not because of the code, so I removed that assertion.

Times, and cases over 5 s: the host load average was above 100, and times swung 2× between runs. In the last run the unit's own `a_mark_on_a_due_message_is_refused_and_changes_nothing` took 10.3 s. In that same run my server cases took:
- fraction case 14.3 s (12.2 s in the run before, which still had 3 readings)
- sender/third case 12.3 s (6.5 s in the run before)
- wait case 6.8 s (1.7 s in the run before)

All three are over 5 s on this host, mostly server start-up and signed requests in a debug build. I already cut the fraction case to 2 readings and the sender/third case to 2 messages and 2 services in total.

3. Suite runs (after the cases existed)
- `cargo nextest run -p inbox-core --locked -E 'test(/deadline_mark/)'` → "Summary [0.796s] 7 tests run: 7 passed, 186 skipped", exit 0.
- `cargo nextest run -p inbox-server --locked -E 'test(/deadline_mark/)'` → "Summary [14.413s] 6 tests run: 6 passed, 78 skipped", exit 0.
- No red case: the code does what the story's acceptance says.

4. Judgement findings (working tree on 7cd7416)

F1 (warning, CONFIRMED, introduced): a due message cannot leave `Pending`, so a `wait` loop spins until the sweep.
- Measured: the wait case above; docs/design/service.md:197-200 states the behaviour as intended.
- Before the unit, the recipient's `read` cleared a due row: a note was removed, a request moved to Read. Now `read` and `mark-delivered` are refused while `Pending` still lists the row, and `waited` returns at once on any row.
- Result: `inbox wait` ("Wait until a message arrives", website/docs/status.md:22) and any wake surface (`Pending` is "the read a wake surface makes", ess/domains/messaging.yaml:987) return at once, again and again, with a row the owner can do nothing about. This lasts until the expiry task runs: 10 s by default (crates/inbox-server/src/config.rs:53), and `--sweep-interval` takes any value (the unit's own CLI case uses 3600).
- What reaches it: every `wait`/`read` loop of a recipient that sees a message after its `expires` and before the next sweep.
- Possible fix (not applied): when the service answers `deadline-passed` to the recipient, run the expiry decision for that one message in the same section. The sender still gets `MessageExpired` in both orders and the row leaves `Pending`. Or keep the design and document the spin, including in `inbox wait`'s help.

F2 (note, CONFIRMED, introduced): clock docs still name only the old readers of the deadline.
- crates/inbox-server/src/clock.rs:13 ("the deadline of a message a post, an answer or a refusal meets").
- crates/inbox-server/src/clock.rs:117 ("the reading `Answer`, `Refuse` and `PostMessage` compare `expires` with").
- crates/inbox-cli/src/main.rs:205 ("The deadline of a post, an answer and a refusal … read this clock").
- The unit added `MarkDelivered` and `MarkRead` as readers and left these three lines unchanged.

F3 (note, CONFIRMED, introduced): user text says a note is always removed once read, and a due note is not.
- website/docs/status.md:23: "Read a message. A note is removed once read".
- `inbox read` help, crates/inbox-cli/src/main.rs:88: "a note is handled and removed".
- Both were true at the base. Neither mentions the refusal (`Expired`, exit 1).

F4 (note, CONFIRMED, introduced): CHANGELOG.md:88 dates the old behaviour to "In 0.2.0".
- 0.3.0 is the latest tag and has no `deadline-passed` at all (`git show 0.3.0:ess/domains/messaging.yaml | grep -c deadline-passed` → 0).
- So the old behaviour is also that of 0.3.0, and a reader would take it as fixed there. The sibling entries in [Unreleased] use the same wording.

5. Attacked and held
- The deadline instant: equal, 1 ns before and after, the whole second, fraction, offsets, lower case. Core and HTTP compare at the same precision as the sweep (`format_reading` / `Op::Le` on instants).
- The clock is read inside `ServiceContext`, behind the exclusive section. No request member replaces it: the mark input has only `message_id`.
- A message without `expires`: none can exist; the field is required. A stored `expires` that names no instant cannot be stored: the post checks it against `created`.
- Order of refusals for every caller shape, through `handle_with_events`, `handle_as_specified` (the unit's case) and HTTP:
  - the sender gets `not-recipient`;
  - a third principal and every caller without attributes get `unknown-message`;
  - the Sweeper gets the same answer whether the message is due or not.
  - No answer to a non-recipient changes when a message becomes due.
- Answered, Refused and decided-on-silence messages keep `already-*` after their deadline.
- A refused mark writes no row and publishes nothing (core store dumps; HTTP `MessageStorage::list`).
- The client: it never sends `mark-delivered` and sends `read` only for `inbox read <id>`. `collect`, `repost` and the epochs file do not touch marks. `accepted()` maps 502 with a declared error to a refusal (exit 1).
- Mutants, reasoned against the cases rather than run with `ess`:
  - `<` in place of `<=` is caught by the core case at `EXPIRES`.
  - Dropping Delivered or Read from the guard is caught by the core arrangements in Delivered and Read.
  - Moving the guard before `not-recipient` is caught by the core sender assertion.
  - The 2 scenarios alone exercise only Sent and the recipient, so they would miss the last two. The core cases cover them.
- Hand-typed counts: 44 scenario files, 44 manifest lines, matching crates/inbox-xtask/tests/tools/scenarios.rs.

6. Paths written outside the worktree
- <scratch>/d30/mark/adversary/core-adv.log
- <scratch>/d30/mark/adversary/server-adv.log
- <scratch>/d30/mark/adversary/server-adv-2.log
- <scratch>/d30/mark/adversary/server-adv-3.log
- <scratch>/d30/mark/adversary/suite-core.log
- <scratch>/d30/mark/adversary/suite-server.log
- <scratch>/d30/mark/adversary/suite-server-2.log
- (the directory itself, created for this pass)

7. Findings block

```findings
[
  {"file": "docs/design/service.md", "line": 197, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "A due message can no longer be read or delivered out of Pending, so `inbox wait` and any wake surface keep answering it at once (no waiting) until the next sweep (10 s by default, as long as --sweep-interval), where before the unit the recipient's read cleared it."},
  {"file": "crates/inbox-server/src/clock.rs", "line": 13, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The Clock docs here and at line 117 (and crates/inbox-cli/src/main.rs:205) still name only post, answer and refusal as readers of the deadline, not the two marks the unit added."},
  {"file": "website/docs/status.md", "line": 23, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "\"A note is removed once read\" (and the `inbox read` help at crates/inbox-cli/src/main.rs:88) is no longer true for a due note, whose read is now refused with Expired and exit 1."},
  {"file": "CHANGELOG.md", "line": 88, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The entry dates the old event-depends-on-sweep behaviour to \"In 0.2.0\", but the latest release 0.3.0 has no deadline-passed at all and behaves the same way."}
]
```
