---
format: aep.planning-md/3
id: review-result:adversary-declared-coverage-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:declared-coverage (d35d455): 2 findings, 0 red cases, 0 of 6 mutants pass the suite'
relations:
- reviews: story:declared-coverage
revision: 1
---
2 findings. Neither is a defect in the correction itself, and nothing I tried breaks it.

```
unit: story:declared-coverage, the correction d35d455 on impl/declared-coverage (diff against 029f462); the findings cover d35d455 with a clean working tree
verdict: NEEDS-CHANGE (finding 1, which the coordinator has to route); finding 2 is CONFIRMED
cases: executed 93→93, red 0 (I added no case to the worktree; every probe ran in copies under the scratch directory)
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 36 paths, all under <scratch>/adversary2/ (listed in part 6)
needs-coordinator: finding 1. Replies.refusal falls under the host operator's standing rule for view members no row can carry, and only you can route it
```

## 1. Diff stat of the worktree
`git --no-pager diff --stat` prints nothing and `git status --short` prints 0 lines. I edited no file in the worktree and ran no git write command.

## 2. Findings

**Finding 1: no row of `Replies` can carry its `refusal` member.**
- **Where:** `ess/domains/messaging.yaml:918`.
- **What was measured:** `Replies` admits only `state == Answered`. The only outcome that sets `refusal` is `Refuse/refused`, which moves the message to `Refused`. `Answered` and `Refused` are both terminal, and `Refuse` on an `Answered` message answers `already-settled` and writes nothing. In the generated core, `refuse` returns `AlreadySettled` for any state other than Sent, Delivered or Read.
- **Script:** `<scratch>/adversary2/view_reach.py`, output in `out/view-reach-head.txt`.
  ```
  inbox.messaging.Replies      states ['Answered']: never carried ['refusal']
  1 view member(s) no row can carry
  ```
  The same script reports 5 such members at 029f462 and 9 at bec370e. The extra 4 and 8 are the fields of `Pending` and `Unanswered` that the unit has now removed.
- **What reaches it:** every read of `GET /messaging/views/replies`. The member is published there and is always absent.
- **Why it matters:** the story's standing rule says such a member, found before the next release, "is removed in the same window without asking first, and the wave report says so".
- **Fix (not applied):** remove `refusal` from `Replies` (1 line), then regenerate with `task generate` and `task projections`, and add it to the changelog entry.
- **Origin:** pre-existing. It is already at `bec370e:ess/domains/messaging.yaml:926`.
- **Verdict:** NEEDS-CHANGE, severity warning.

**Finding 2: the new guard runs only after the script has written the scenario files, and no gate step runs the script.**
- **Where:** `tools/scenarios.py:710`.
- **What was measured:** each `s.write()` (`:354`) writes `ess/scenarios/<name>.yaml` before the guard loop runs. `task check` (`Taskfile.yml:12-24`) does not run `task scenarios`, and nothing compares `ess/scenarios/` with the script's output.
- **Script:** `<scratch>/adversary2/p1-guard.sh X`, which uses `false_claim2.py`. Case X keeps the 6 `Unanswered` claims of `read-decision-waits-for-its-answer`, drops the read of the decision, and turns the row from `contains` into `excludes`. The output, from `out/p1-summary.log`:
  ```
  X: task scenarios exit 201
  AssertionError: read-decision-waits-for-its-answer claims inbox.messaging.Body/invariant/at/inbox.messaging.Unanswered/body and reads no value at `body` in a row of Unanswered
  X: task coverage exit 0
  93 selected scenario(s), 24 authored source(s), 89 refusal occurrence(s), 89 covered by authored
    view steps of scenario 24 in the suite: [('query_view', None), ('expect_view', 'excludes')]
  X: task conform-interpreted exit 0
    93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported
  ```
  The guard fails as claimed. But the false claim is already on disk, and the coverage and interpreted steps of the gate pass on it.
- **What reaches it:** an author who commits after a red `task scenarios`, or anyone who edits `ess/scenarios/*.yaml` by hand. `AGENTS.md:95` asks for the script and the files to be committed together, but nothing enforces it.
- **Fix (not applied):**
  - Run the guard before any file is written: build the documents, assert, then write.
  - Add a step to `task check` that holds `ess/scenarios/` and `ess/ess-inputs.yaml` to the script. Either run the script into a scratch directory and diff, or run `task scenarios` and then `git diff --exit-code -- ess/`.
- **Origin:** introduced by the unit. The assertion after the writes dates from f6d8147, and the correction rewrote the guard in the same place.
- **Verdict:** CONFIRMED, severity warning.

## 3. What held (one line each)
- **Reachability of `answered_by` and `refusal` in the model:** `view_reach.py` finds that no outcome writes a message without a move. Sent, Delivered and Read may hold only `authority`, `decision`, `default_on_silence` and `reply_shape`.
- **Reachability in `inbox-core`:**
  - `PostMessage` stores `Sent` with all 4 fields `None` (`crates/inbox-core/src/obligations.rs:222-225`).
  - `Expire` sets `answered_by` only on the move to `Answered` (`:261-269`).
  - The generated `deliver` and `read` moves copy the row unchanged, and `answer` and `refuse` go only to `Answered` or `Refused`.
  - No other `MessageStorage::put` exists outside the test kit.
- **A fifth member of `Pending` or `Unanswered` that no row can carry:** none. Of the other 7 views, only `Replies.refusal` (finding 1).
- **Regeneration against 029f462:** apart from digest lines, every changed line under `generated/` is a deleted `answered_by` or `refusal` of the 2 views.
- **Regeneration in a tree copy:** `task generate` and `task projections` changed 0 files against the worktree. `projections-drift` and `drift` both exit 0.
- **Determinism:** `slot task scenarios` run twice in the copy left `ess/` identical to the worktree both times.
- **Wrapped digests:** `digest_json.py` recomputes all 6 from the ESS site path, SHA-256 of `{"kind":"site","path":[…]}` with `op` first. All 6 match the paths the guard reads (`reply.body`, `reply.reply_id`, `reply.option`, the field itself, `decision.options[].letter`, `decision.recommended_option`).
- **Smallest false claims:**
  - A `contains` row beside `counts` in one assertion is refused by ESS: `ESS-AUTHOR-025 … state exactly one of contains, excludes, counts, ranked, at, satisfies or halts_after`.
  - An extra key is refused by ESS: `ESS-AUTHOR-001 unknown field`.
  - An `excludes` row is refused by the guard.
  - The only negative form is `excludes`, and the guard does not count it.
- **All 42 claims (`verify_claims.py`):** 42 parsed, 42 claimed, 0 bad. The 3 `ThreadId` claims now read a thread that differs from the message id. `task coverage` exits 0 with 89 of 89 covered.
- **Mutants, one at a time, `slot task conform`** (unmutated copy: 93 of 93 passed):

  | Mutant | Result |
  |---|---|
  | M8 (Settled publishes the message id as thread) | exit 201, fails at step 21 on `Settled` |
  | M9 (Replies, same mutation) | exit 201, fails at step 23 on `Replies` |
  | T1 (Thread selects by message id) | exit 201, 11 failed |
  | T2 (Replies selects by message id) | exit 201, 2 failed |
  | T3 (`MessageAnswered` thread is the message id) | exit 201, 2 failed |
  | T4 (Outbox publishes the message id as thread) | exit 201, 8 failed |

  None of the 6 passes.
- **`docs/design/inbox.md`:** 93/24/89, 47 + 42, 36 + 6, and the 8 of `Pending` and `Unanswered` all match the outputs. The sentence on the 4 fields is accurate.
- **Checked by reading only, not run:** a list path through an empty list gives `[None]` and fails the guard. A claim on a scenario the manifest does not list fails `assert name in names`, and the manifest is built from `names`.

## 4. Tool calls denied
1: a foreground `sleep` followed by reading a log. I waited on the output file with an until-loop instead, as the denial message instructs.

## 5. Tree copies
2, one at a time, both deleted: `<scratch>/adversary2/esscopy` (ess/, tools/, Taskfile.yml) and `<scratch>/adversary2/tree` (full copy with its own `target/`).

## 6. Paths written outside the worktree
36 paths in all, relative to the brief's scratch directory `…/w4/declared-coverage/`. You can delete all of them.
- Scripts (8): `adversary2/view_reach.py`, `adversary2/digest_json.py`, `adversary2/digest_from_suite.py`, `adversary2/false_claim2.py`, `adversary2/mutate2.py`, `adversary2/p1-guard.sh`, `adversary2/p2-tree.sh`, and the directory `adversary2/out/`.
- Files in `adversary2/out/` (28):
  - `digests.txt`, `view-reach-head.txt`, `messaging-029f462.yaml`, `messaging-bec370e.yaml`
  - `p1-summary.log`, and `p1-{C,F,X}-{scenarios,coverage,interpreted}.log`
  - `p2-summary.log`, `p2-scenarios-1.log`, `p2-scenarios-2.log`, `p2-generate.log`, `p2-projections.log`, `p2-pdrift.log`, `p2-drift.log`, `p2-coverage.log`, `p2-claims.log`, `p2-conform-unmutated.log`
  - `p2-mutant-{M8,M9,T1,T2,T3,T4}.log`

## 7. Findings block
```findings
- file: ess/domains/messaging.yaml
  line: 918
  category: judgement
  severity: warning
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: 'Replies admits only Answered rows and only Refuse/refused (which moves to the terminal Refused) sets refusal, so no Replies row can carry its refusal member; the standing rule removes such a member before the next release'
- file: tools/scenarios.py
  line: 710
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the covers guard asserts only after every scenario file is written and task check never runs the script or compares ess/scenarios with it, so a false claim the guard rejects stays on disk and passes task coverage (89 of 89) and conform-interpreted (93 passed)'
```
