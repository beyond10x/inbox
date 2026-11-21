---
format: aep.planning-md/3
id: review-result:adversary-declared-coverage-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:declared-coverage (029f462): 3 findings, 0 red cases, 2 of 10 mutants pass the suite'
relations:
- reviews: story:declared-coverage
revision: 1
---
3 findings: 0 blockers, 2 warnings, 1 note. All 42 claims hold.

```
unit: story:declared-coverage, branch impl/declared-coverage at 029f462 (base bec370e); findings cover that commit
verdict: CONFIRMED
cases: executed 267→267 (cargo, unchanged; no cargo case added), red 0; scenario/mutant probes: 10 mutants, 2 survived
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 path (the adversary scratch directory, listed under 6)
needs-coordinator: these files still carry the pre-unit numbers, and docs/plan/stage-2.md gives them to the coordinator, not the unit: AGENTS.md Commands rows (`task check` lacks coverage; `task coverage` still reads "Exits 1 … 50 of 97 … Not in task check"; conform-interpreted and conform say 92 / 23 authored, now 93 / 24), README.md, website/docs/*.md, CHANGELOG.md. CHANGELOG [Unreleased] does not yet name the wire change to Pending/Unanswered against 0.1.0, which the story's Context says it does.
```

## 1. `git --no-pager diff --stat` of the worktree
Empty. `git status --short` is empty: I changed no file in the worktree and added no test file.

## 2. Cases added (all scripts under scratch, run against a tree copy built from HEAD)

**F1, mutants M8 and M9 survive `task conform`** (`p3-mutants.sh`, `mutate.py`)
- What it does: `decision-answered-by-the-operator` posts its decision without a thread. The helper then sets the thread to the message id (`tools/scenarios.py:577`, `asked = {"thread": m, …}` at :587).
- So the claims `ThreadId/invariant/at/Settled/thread` and `…/Replies/thread` read a value that is also the message id.
- No other scenario reads `thread` in Settled, Replies or Thread, at base or at head.
- The mutants change the projection in `generated/rust/inbox/src/behaviour.rs` (copy only):
  - M8: `settled` returns `thread: ThreadId(held.data.message_id)`.
  - M9: `replies` does the same.
- Output: `M8: conform exit=0 … 93 scenarios: 93 passed` and `M9: conform exit=0 … 93 passed`.
- What reaches it: any core or backend defect that puts the message id into `thread`. Clients use the thread to group a fan-out, as in `request-fanned-out-to-eleven`.
- Fix (not applied): post under a distinct thread, as `read-decision-waits-for-its-answer` already does with `t = mid("RW", 9)`. `fix_thread.py` + `p6-fix-kills-m8.sh` applies it in the copy:
  - coverage still exits 0 with 89 of 89 covered;
  - unmodified conform: 93 passed, `inconclusive`;
  - M8 then fails at step 21 and M9 at step 23 (exit 201).

**F2, the READS assertion accepts false claims** (`false_claim.py`, `p2-false-claim.sh`)
- The check is at `tools/scenarios.py:682-684`.
- Case A: the 6 Unanswered claims are written as plain id strings instead of through `inv()`/`wrapped()`, and the scenario's Unanswered check is removed. `if claim in READS` skips them.
  - Output: `scenarios exit=0`, `coverage exit=0`, `89 covered by authored`.
  - Coverage suite on the interpreted target: `conformance_status: passed`, 93 of 93 passed.
  - No scenario reads an Unanswered row at all.
- Case B: the claims stay built by the helpers, the decision is posted without `default_on_silence`, and the row asserts `default_on_silence: None`. `key in contains` accepts a null value.
  - Output: same as case A (`coverage exit=0`, interpreted run `passed`, 93 of 93).
  - The claimed OptionLetter invariant at `Unanswered.default_on_silence` is never exercised.
- What reaches it: the next edit of `COVERS`. All 42 current claims are built by the helpers and carry non-null values, so the defect is in the guard, not in today's numbers.
- Fix (not applied): assert every `ESS-SYNTH-013`-shaped id is in READS, and that the value at the key is not None.

## 3. Suite run (after the probes above)
`CARGO_INCREMENTAL=0 slot task check` on an unmodified copy, byte-equal to the worktree outside target/ and .out/: **exit 0**.
- coverage: `93 selected scenario(s), 24 authored source(s), 89 refusal occurrence(s), 89 covered by authored`.
- conform-interpreted: 93 passed.
- conform: 93 passed, `conformance_status: inconclusive`.
- cargo tests: 267 passed, 0 failed, 4 ignored.
- fmt, clippy and build-generated passed.

## 4. Judgement finding
**F3** (`docs/design/inbox.md:51-53`, CONFIRMED, introduced, note)
- The doc removes `reply` and `decided_option` from Pending/Unanswered because "only an answered message carries either".
- The same holds for `answered_by` (set only by `Answer/answered` and `Expire/decided-on-silence`) and for `refusal` (set only by `Refuse/refused`, state Refused).
- Both stay in those views and are always absent there (`view_fields.py`).
- Removing them later would be a second wire change against 0.1.0. Removing only the 2 fields matches the host operator's decision, so this is a note.

## 5. Attacked and could not break
- All 42 claims (`verify_claims.py`). For each one there is a `contains` row on the claimed view, the field holds a non-null value, and the invariant holds on it. Every comparison is to a literal:
  - Body (9 claims): `utf8_bytes <= 65536`;
  - MessageId (9): `count == 26`;
  - ThreadId (3): `count == 26`;
  - OptionLetter (21): `count == 1`.
  - 0 bad.
- Invariant strength: the generated scenario would run `value_invariants` (count plus the alphabet declarations) on one arranged row. The authored scenario compares the same row whole to a valid literal, which implies the invariant, and the inbox-conformance `view_equal` compares nested values whole.
- Mutants killed (each turns `task conform` red in the claiming scenario):
  - M1: Settled `reply.body` changed;
  - M2: Thread `decided_option` set to Z;
  - M3: Due `reply` dropped;
  - M4: Messages `reply.reply_id` set to empty;
  - M5: Outbox `reply.option` dropped;
  - M6: Replies `decision.recommended_option` dropped;
  - M7: Unanswered `thread` set to the message id;
  - M10: Unanswered `default_on_silence` dropped.
- Assertion bypass by view-name prefix or nested key path: not possible. The view is compared exactly and the key is top-level.
- Reachability:
  - Model: only `Answer/answered` and `Expire/decided-on-silence` set the 2 fields, and both move the message to the terminal state Answered; `posted` clears them.
  - Core: `crates/inbox-core/src/obligations.rs:206-226` and `:250-271` do the same through the type-state transitions.
  - Interpreted target, base model: probe `adv-silence-from-read` (read decision, then decided-on-silence) passes with Unanswered and Pending empty.
  - Existing scenarios show the same after an answer (`post-read-answer`) and after silence from Delivered.
- Base numbers: 92 scenarios / 23 authored / 97 refused / 47 covered, exit 1. Exactly 8 of the uncovered refusals are `reply` or `decided_option` at Pending or Unanswered.
- Regeneration:
  - Every non-digest hunk is in the Pending or Unanswered row: struct, projection, wire encoder, OpenAPI `Pending.Row` / `Unanswered.Row`, docs.
  - No shared type, event, command or other view changed.
  - `task generate` plus `task projections` changed 0 files; `projections-drift` and `drift` exit 0.
- Readers of the removed fields: no crate changed and none reads them from Pending/Unanswered. inbox-core builds no view rows. No test was weakened.
- Numbers in `docs/design/inbox.md`: 93 / 24 / 89 / 89, 36 + 6 = 42, 47 others, interpreted `passed`, `task conform` `inconclusive`. All match the outputs; there are 24 files in `ess/scenarios/`.
- Taskfile: with one claim dropped, `coverage` exits 201 (88 of 89 covered) and `check` exits 201 at coverage. `coverage` writes only `.out/coverage` (git-ignored).
- Determinism: `task scenarios` twice gives an identical manifest, and the committed scenarios equal the script's output.
- Repository rules: no name, e-mail or home path in the diff; no comment in a manifest; `generated/` equals a fresh regeneration.

## 6. Paths written outside the worktree
`<scratch>/w4/declared-coverage/adversary/`
- It holds the probe scripts and the logs and suites in `out/`.
- The tree copy (`tree/`, with its target/) and the base `ess/` extract (`base/`) are deleted.

```findings
- file: tools/scenarios.py
  line: 587
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'decision-answered-by-the-operator posts with thread equal to its message id, so its ThreadId claims at Settled and Replies cannot tell the two apart and the mutants Settled.thread := message_id and Replies.thread := message_id pass task conform 93/93; posting under a distinct thread kills both'
- file: tools/scenarios.py
  line: 682
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the READS assertion skips any claim not built through inv()/wrapped() and accepts a contains key whose value is null, so a claim on a view the scenario never reads, or on a field the row does not carry, passes task scenarios and task coverage (89/89) and the interpreted run reports passed'
- file: docs/design/inbox.md
  line: 53
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the reason given for dropping reply and decided_option from Pending and Unanswered (only an answered message carries either) applies equally to answered_by and refusal, which those views still publish and which are always absent there'
```
