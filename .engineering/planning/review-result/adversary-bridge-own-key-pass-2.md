---
format: aep.planning-md/3
id: review-result:adversary-bridge-own-key-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the bridge with its own key: no finding, 2 green cases, no mutant run'
relations:
- reviews: story:the-chat-bridge-runs-with-its-own-key-under-a-delegation
revision: 1
---
ADVERSARY PASS 2

I found no defect in round 2. All the cases I ran are green, and the stress run was green 10 times out of 10. I could not run any mutant: the permission check refused the first edit of the product, so I stopped mutation testing as my orders say. The findings block is therefore empty.

Builds and tests: every job ran through slot. The first line of each job named the node ("slot: synced to build-1d …"). I ran no build or test command without the `slot` prefix (none).

The refusal: I tried a temporary edit that removed `again.refused_key.clone_from(&self.refused_key);` in `crates/inbox-cli/src/commands.rs` (mutant M1). It was refused with this line: "Permission for this action was denied by the Claude Code auto mode classifier. Reason: [Destructive or Unwanted Actions]." I did not try any other way to make the edit. The product was never changed. The nextest run I had queued behind it ran on the unchanged code (18 passed) and proves nothing about M1. `git diff --stat` of the adversary tree shows only the cases file.

What I did, by point of the brief:

1. Round 2's fix of the line (green)
- I added the case `odd_characters_in_the_ids_keep_the_refused_end_one_line_on_both_streams`, 0.031 s.
  - `--authority` gets a carriage return, U+2028, U+2029, U+0085, ESC `[2J`, U+202E, U+200B and 20,000 more bytes. The principal in the secret key file gets a line end and the same characters. The run uses `--json`.
  - Standard error is one line with one line end, at the end. It holds no raw control, separator or direction character, and shows every value as its escape in the order written.
  - Standard output is one JSON document on one line, `error` = `key refused`.
- I added the case `odd_characters_in_the_key_id_keep_the_refused_end_one_line`, 0.021 s.
  - The key id in the secret key file holds U+2028, U+202E and ESC `[1m`. The service answers `401`, and the refused end is one line with each character shown as its escape.
  - A line end in the key id never reaches the service: `client.rs` `request_head` refuses CR and LF in a header first.
- The first line of a bridge that starts well: `--authority` appears only in the line of the refused end (bridge.rs:207), so a good run never prints it.
- The JSON document keeps U+2028 and direction characters raw inside its `detail` string. That is the decided rule for documents (output.rs, `a_document_keeps_a_bidirectional_control_as_it_was_sent`), so it is not a finding.

2. Round 2's cases and the helper
- I read the 5 added cases and the 2 taken over. Each one waits on a fact (the forwarder's record, the stand-in's files, `/proc`); none waits on a wall-clock time.
  - The forwarder records a request before it sends it on (bridge_once.rs:370), so `refuse` cannot miss a line.
  - `pending_line("")` matches only the poll's whole read.
  - `a_401_while_a_child_runs` checks `gone(child)` right after the bridge exits and before `release-read`, so if the child were left running the case would be red.
  - The stand-in's wait (1000 × `sleep 0.01`) only ends a hang.
- Stress run of `bridge_own_key`, `bridge_once`, `bridge`, `bridge_read`, `adversary_bridge_own_key` and `adversary_bridge`, with `--stress-count 10`: every round "63 tests run: 63 passed", about 5.85 s each. No case was red once.

3. The child: by reading only, no case written.
- `refused_key` takes the child slot, kills the child's process group with SIGKILL (`end_group`) and holds the slot until exit. A child that is starting holds the slot while it starts, so the refused end waits for it and then kills it. The stand-in's `sleep` helpers are in that group.
- A post child killed by a `401` is not recorded. The next start posts again with the same idempotency key, which is the design (bridge.md, the paragraph on SIGKILL and replay).

4. What pass 1 did not reach
- `--json` on the refused end: covered by the first case above.
- `open --for` on an inbox the person opened with another adapter: by reading, the design answers `already-open` and the adapter at the service stays. I wrote no case.
- The rest was not reached; see the list below.

5. The sentences of round 2
- The 2 changed paragraphs of `docs/design/bridge.md`, its "Cases" paragraph and the `AGENTS.md` row of `crates/inbox-cli/` match the code and the cases: the line shown as `warn` shows it, the poll's client inheriting the refused end, the park after `KEY_REFUSED`, and the forwarder and stand-in as described.

Mutants (none could be run, see the refusal):
- M1, the clone_from line in commands.rs removed: not run (edit refused).
- M2, `end_group` → `drop(running)`: not run.
- Principal not inlined (bridge.rs:199): not run. By reading, no case of the story has a line end in the principal; `odd_characters_in_the_ids…` would catch it on the raw `\n`.
- The refused line written without `visible`: not run. By reading, the 2 new cases would catch it.
- m02 and m15 of round 1: not run.

Kept, uncommitted, in the adversary tree: `crates/inbox-cli/tests/adversary_bridge_own_key.rs`, 2 cases added and nothing existing changed (+175 lines). Both are green. I kept them because by reading they catch the 2 mutants above that the story's cases would let live; that is not measured. Drop them if a measured kill is required. Both checks pass: `rustfmt --edition 2024 --check` exit 0, and `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exit 0 (only the earlier unused `value` warning in the generated `wire.rs`).

0 pre-existing findings; I wrote no private file.

Not checked:
- Any mutant run, including m02 and m15 (refused).
- The header in the thread line of a decision taken on silence or expired, on a repost, and on the round after a failed round.
- 2 bridges for one person with different keys.
- A `.opened` file written by a build before this story.
- The log of `inbox serve` and the answers per cause of the `401`.
- `close-inbox` on the agent's own inbox under the header.
- No process of the child's group left after a `401` (only read, no case).

Logs (run1, stress, case1, case2, final, clippy) are in the scratch directory.

```findings
[]
```
