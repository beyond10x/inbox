---
format: aep.planning-md/3
id: review-result:adversary-bridge-own-key-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the bridge with its own key: 5 findings, 1 red case'
relations:
- reviews: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
revision: 1
---
ADVERSARY PASS 1
unit: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation, adversary tree detached at 54029b1 (round 1 with 0345c87 merged in)
verdict: NEEDS-CHANGE
cases: executed 402→403, red 1
origin: introduced 5 / pre-existing 0 / undecided 0
wrote-outside-worktree: 22 files in the scratch directory (listed in part 6)
needs-coordinator: none

1. git --no-pager diff --stat (adversary tree)
 crates/inbox-cli/tests/main.rs | 1 +
 1 file changed, 1 insertion(+)
plus the untracked file crates/inbox-cli/tests/adversary_bridge_own_key.rs. Both are test files. They are left uncommitted.

2. Cases added (crates/inbox-cli/tests/adversary_bridge_own_key.rs, one mod line in tests/main.rs)
- a_line_break_in_authority_does_not_make_a_second_line_of_the_refused_end. The bridge has its own key and no delegation, and `--authority` holds a line break. The case expects exit 1 and exactly one line on standard error. It is RED now: 10 of 10 stress runs and the suite. Time 0.04 s.
  Red output from the first run of the case alone, word for word:
  assertion `left == right` failed: one line on standard error: "inbox bridge: the service refused the key chat-bridge-1 acting for agent:test/bob under the delegation chat-bridge-for-bob\ninbox bridge: a second line (401): its key file holds no live key entry chat-bridge-1 or no live delegation chat-bridge-for-bob\ninbox bridge: a second line of agent:demo/chat-bridge for agent:test/bob; add the missing one with `inbox keyfile add-key` or `inbox keyfile add-delegation` and start the service again: it reads the key file at its start only\n"
    left: 3
   right: 1
- I wrote 2 green cases and then removed them from the tree, because the rules keep only red cases and green cases that kill a surviving mutant. A copy is in the scratch directory as green-cases-not-kept.rs.txt.
  - The first case lapses the delegation through the service's clock file while the held read waits. alice then posts, so the bridge's next request is answered 401. The case expects exit 1, one key line, at most 1 bridge request after the clock moved, and a recorded post if one was made. It was green 15 of 15 runs, under 0.1 s.
  - The second case uses one outbox file for 2 persons. It runs `open --for` bob and alice, then `close --for` alice and a restart. bob's line stays and is opened again; alice's is gone and not opened again. It was green 10 of 10, under 0.05 s.
  - Answer to the brief's question about mutant 15: before this pass, no case held "no request after the first 401" for a 401 that is not the first request. The removed first case holds it for a 401 in the rounds' thread. No case holds it for a 401 in the poll's thread (finding 2).

3. Suite runs (after the case existed)
- cargo nextest run -p inbox-cli --locked --no-fail-fast, with the final file: Summary [8.444s] 403 tests run: 402 passed, 1 failed (the case above), exit 100. Before my case: 402.
- cargo nextest run -p inbox-cli --locked --no-fail-fast --stress-count 5 -E 'test(/^bridge_own_key::|^adversary_bridge_own_key::/)': every case of bridge_own_key passed in all 5 iterations. The slowest was the_acceptance_of_the_message_asked_once 3.3 s.
  - In iteration 4, the first version of my later-401 case failed: 0 requests after the clock moved. The request answered 401 had been carried just before the clock moved and was read by the service just after. That was a timing error of the case, not of the product. The case was loosened to "at most 1" and was then green 10 of 10.
- cargo clippy -p inbox-cli --all-targets --locked -- -D warnings: exit 0.
- The earlier suite run that included the green cases: 405 run, 404 passed, 1 failed (the same case). The cases of acting_for, adversary_acting_for (9), bridge, bridge_read, bridge_once and their adversary files are all green.
- Every job's first line named build-1d.

4. Findings
F1 (blocks nothing alone, NEEDS-CHANGE, introduced). The line of the refused end prints `--authority` as typed. output::warn shows every control character except the line end in visible form, so a line break in `--authority` makes 3 lines on standard error, and the second and third each start with "inbox bridge: ".
- What was measured: the red case above, at crates/inbox-cli/src/bridge.rs:203 (the format of `refused_key`).
- What reaches it: the operator types or pastes `--authority`. The client never checks it, while `--for` is refused for the same character.
- Where the fix lands: either main.rs (around :906) refuses a control character in `--authority` with exit 2 before any request, as `acting_for_address` does for `--for`, or the line shows it as `\u{a}`.
F2 (warning, introduced): mutant o2 survives. It drops `again.refused_key.clone_from(&self.refused_key)` at commands.rs:119, so the poll's client no longer ends the process on a 401. Every case of the story and every case of mine stays green.
- No case makes the poll's request the first one answered 401. The story's case gets its 401 on the open, before the poll starts.
- The fix lands in the story's cases: one where the poll's read or `read` is the first request answered 401.
F3 (note, introduced): mutant o3 survives. It replaces `wake::end_group(running)` at bridge.rs:200 with `drop(running)`, so the running `connectors` child is not ended on a 401. No case has a child running when the 401 arrives. The fix lands in the cases.
F4 (note, introduced): mutant o4 survives. It passes `own_inbox` as the sender to `Outbox::due` at commands.rs:1171. No case holds which kept envelopes a command under `--for` reposts. Brief point 2 (an envelope posted under `--for` and one posted for the agent itself, each reposted with the right header) has no case.
F5 (note, INFEASIBLE, introduced): mutant o1 survives. It switches off the park guard at commands.rs:134.
- The guard only matters in the window between the first 401 and `process::exit`.
- The design admits that a request another thread started at the same instant is not held back.
- I found no deterministic case that can hold it.

Mutants (each applied in my copy and restored from a scratch copy; the filter was bridge_own_key, adversary_bridge_own_key, acting_for and adversary_acting_for):
- m08, lock keyed by the caller's principal: caught by bridge_own_key::the_lock_is_the_persons…, a_wake_for_the_person….
- m09, record keyed by the caller's principal: caught by bridge_own_key::a_bridge_with_its_own_key_reads_the_record…, every_request…, the_acceptance_of_the_message_asked_once….
- m12, `end()` not called on a 401: caught by bridge_own_key::a_refused_key_or_delegation… and by my removed later-401 case.
- m13, exit 0 after the 401: caught by bridge_own_key::a_refused_key_or_delegation…, my removed later-401 case and the red case.
- o1, no park guard: survived. One failure in that run was the timing error of my early case, not the mutant.
- o2, poll client does not end on a 401: survived.
- o3, child not ended: survived.
- o4, kept envelopes taken by the inbox instead of the sender: survived.

Not checked or not reached:
- Point 1: the headers of the line written in a thread on silence or expiry, of a repost, and of the round after a failed round.
- Point 2: 2 bridges for the same person at the same moment with different keys; a `.opened` file written by a build before this story.
- Point 3: a post half made when the 401 arrives with the child running (the stand-in has no delay); `--json` on the refused end.
- Point 4: the log of `inbox serve` and the timing per cause.
- Point 5: `open --for` on an inbox the person opened with another adapter; `close-inbox` on the agent's own inbox under the header.
- Points 7 and 8: AGENTS.md, CHANGELOG.md, limits.md and the bridge design section were read only, not compared sentence by sentence with both stories.
- The implementor's m02 and m15 were not run.

Attacked and not broken:
- The lock, the record and the `.opened` line keyed by the person: m08, m09 and the two-person close case.
- Exit 1 and the one key line on a 401 after the start (the removed later-401 case, 15 runs).
- The value of `--for` is checked before any request for bridge, wake, open and close: `client()` runs `acting_for_address` for all of them, and adversary_acting_for is green.
- 0 pre-existing findings; no private file was written.

5. Commands that built or tested without the slot prefix and outside the cargo shim: none. Only rustfmt, perl and sh ran locally. The sh scripts called cargo through the shim with the adversary tree as the working directory.

6. Paths written outside the worktree (all in the scratch directory): red1.log, case2.log, mutants.sh, mutants/summary.txt, mutants/<8 mutant names>.log, orig/wake.rs, orig/bridge.rs, orig/commands.rs (the copies used to restore the mutants), final.sh, final.txt, stress.log, suite.log, clippy.log, stress2.log, clippy2.log, clippy3.log, suite2.log, green-cases-not-kept.rs.txt.

```findings
[
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 203, "category": "boundary", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A line break in --authority is printed raw in the line of the refused end, so the one promised line becomes 3 lines on standard error, each looking like a line of the bridge."},
  {"file": "crates/inbox-cli/src/commands.rs", "line": 119, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping the refused-key end from the poll's client leaves every case green, because no case makes the poll's request the first one answered 401."},
  {"file": "crates/inbox-cli/src/bridge.rs", "line": 200, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Not ending the running connectors child on a 401 leaves every case green, because no case has a child running when the 401 arrives."},
  {"file": "crates/inbox-cli/src/commands.rs", "line": 1171, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Taking the kept envelopes of a reconnect by the inbox acted for instead of the sender leaves every case green, so no case holds which envelopes a command under --for reposts and with which header."},
  {"file": "crates/inbox-cli/src/commands.rs", "line": 134, "category": "mutant", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "Switching off the park guard after the first 401 leaves every case green, and the window it guards is too short for a deterministic case."}
]
```
