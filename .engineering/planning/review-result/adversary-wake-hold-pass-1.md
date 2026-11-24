---
format: aep.planning-md/3
id: review-result:adversary-wake-hold-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the wake that ended and its reader
relations:
- reviews: story:a-wake-that-ended-writes-no-second-line-when-its-reader-goes
revision: 1
---
ADVERSARY PASS 1

Story: story:a-wake-that-ended-writes-no-second-line-when-its-reader-goes, at 383ecd3. Every job ran on build-1d: the first line of each log reads `slot: synced to build-1d …`. Nothing ran on this host. I saw no stale binary: each mutant's diff was logged before its run, and the results match the diffs.

Bottom line: the product holds its promise, and the case and the release build are as the story says. The `--before-exit` point itself can hang the debug binary (findings 1 and 2), and the story's case does not hold the `--follow` end (finding 3).

Findings

1. The point hangs when the watch has already returned. In the case, the refused read ends the wake and the point's file appears. The case then drops the reader and waits 1.2 s, which is more than two looks of the watch, before it removes the file. The watch saw no reader with the hold set and returned. `before_exit` then reads `begun` and waits for `done > begun`, but no look ever comes.
   - Red line: `inbox wake hangs at the point, not ended within 10s: "inbox wake: refused: unauthenticated (403)\n"`
   - Case: `adversary_wake_hold::a_reader_gone_two_looks_before_the_file_is_removed_does_not_hang_the_point`.
   - The story's case has the same window, from its drop of the reader to the point's read of `begun`. It does not hit it now: 150 of 150 green, because the wake reaches the point tens of ms after its start and the first look comes at 500 ms. On a loaded host where the point is reached near a look, the case would hang and then fail at the 10 s bound.
   - Fix lands in `Watch::before_exit` and `Watch::watch` (`crates/inbox-cli/src/wake.rs`, lines 190 to 196). For example, a flag "watch running" that `start` sets and the watch clears before its `return`; `before_exit` waits only while it is set.

2. The point hangs when the watch never started. `before_exit` checks `self.pipe.is_none()`, not whether a watch thread runs. The doc comment says "when the watch runs", which the code does not check. A wake whose standard output is a pipe that already has no reader at its start ends with 4 before `surface.watched()`, so no thread exists, and the point waits for ever.
   - Red line: `inbox wake hangs at the point, not ended within 10s: "inbox wake: standard output: it is a pipe or a FIFO whose reader is gone: nothing is asked of the service\n"`
   - Case: `adversary_wake_hold::a_wake_without_a_reader_at_its_start_does_not_hang_at_the_point`.
   - By reading the code, not run: two more ends do the same. One is `--follow` refused by another adapter's record (`admits` runs before `watched`); the other is a failure to set up the signal handlers.
   - The sentence of `docs/design/wake.md` ("it exits only after the file is gone again and the watch has done a look it began after that") does not hold on these ends either.
   - Fix lands in the same place as finding 1.

3. The story's case does not hold the `--follow` end. My mutant m3 replaced `*watch.hold() = true;` with `*watch.hold() = !follow;`. The story's case stayed green 10 of 10. A green case of mine was red 10 of 10, exit 4 in place of 1:
   - Case: `adversary_wake_hold::a_follow_wake_refused_after_a_full_round_writes_one_line_when_its_reader_goes`.
   - What it does: `--follow`, one full round (a row in `Sent`, then its mark), then the next read is refused with 403, and the reader goes before the exit.
   - Why it matters: a successful round of `--follow` sets no hold of its own, so only the end of `run` holds the watch there.
   - Its time: 0.52 to 0.57 s, slowest of 10 runs 0.568 s.
   - The fix is to take that case into `tests/wake.rs`.

No product defect: with the hold in place, I found no end that writes a second line. The case of item 1 of the brief cannot pass without the hold:
- The point waits for a look whose `begun` was counted after it read `begun`.
- That read comes after the file's removal, so after the drop of the reader.
- Under m1 that look sees no reader and an unset hold, and calls `exit(4)` before the point returns.
- The watch returns only when the hold is set, and with the watch never started the point hangs (red).
- Measured: red 20 of 20.

The rest of the brief:
- **Item 2(b), a run without `--before-exit`:** only the per-look `fetch_add` calls (two per look, three call sites) and a call `before_exit(None)` that returns at once differ.
- **Item 2(c), `create_new` fails:** the point returns at once. The case then sees "the wake ended before the point", or a stale file. Both are red, never a false green.
- **Item 2(d), the other exits:**
  - Wrong usage (2) and the `Err` of `client.client()` do not reach the point.
  - A signal on `--follow` ends the process from the signal thread (128+n), so it does not reach the point either.
  - The lock held (5) and exit 4 of the check after the write do pass the point, with the watch running.
- **Item 4, release:**
  - `cargo build --release --locked -p inbox-cli && ./target/release/inbox wake --before-exit x` gave `error: unexpected argument '--before-exit' found` and `exit 2`.
  - `cargo clippy --release --workspace --all-targets --locked -- -D warnings` exits 0.
- **Item 5, the texts:**
  - The debug binary's `inbox wake --help` does not name the flag.
  - `generated/cli/inbox.json` names neither `--before-exit` nor `--clock-file`.
  - No page of the website lists the flags of `inbox wake`.
  - The `AGENTS.md` clause matches the code.
- **Item 7, stress 10:** the modules `wake`, `reader_gone` and `adversary_reader_gone`, plus my follow case, 28 cases, passed 10 of 10. The story's case passed 150 of 150; its slowest run took 0.579 s.

Mutants (each reverted; stress counts as given)
- m1, hold replaced by `();`: killed by the story's case 20 of 20 and by my follow case 20 of 20.
- m2, hold set after the point: killed by both, 20 of 20.
- m3, hold set to `!follow`: survived the story's case (10 of 10 green). Killed by my follow case, 10 of 10.
- m5, `done` counted before `reader_gone`, together with m1: killed by both, 20 of 20.
- m6, the point's wait `<=` changed to `<`, together with m1: survived both, 20 of 20 green. The point returns at once, so the story's case passes without a hold. No case holds the point's own wait. This is a mutant of the test hook, not of the product, so it is not a finding.
- m7, the point placed before the refusal's line, together with m1: killed by both, 10 of 10.
- m8, the watch returns at its next look once the hold is set, without asking the reader: survived both, 10 of 10. It is equivalent for the promise: the product still writes one line.
- I did not reach a mutant "ended read without the lock".

Not checked
- The other-adapter and signal-setup hang paths of finding 2: code reading only, no case run.
- No run on a loaded host to show the hang of finding 1 inside the story's own case.
- No `task check` and no full crate run.

Cases: `crates/inbox-cli/tests/adversary_wake_hold.rs` in the adversary tree, plus one `mod adversary_wake_hold;` line in `crates/inbox-cli/tests/main.rs`. Both are uncommitted. Two cases are red (findings 1 and 2), and one is green and kills m3.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0 with them.
- `git diff --stat` shows only `main.rs`; the new file is untracked.
- The mutant script and a copy of `wake.rs` sit in the git-ignored drafts folder of the adversary tree. Logs are in the scratch directory.

No pre-existing security flaw found; the private file was not written.

Build commands run without the `slot` prefix: none outside the shim.
- `cargo nextest` and `cargo clippy` went through the shim.
- The release build and `cargo run … wake --help` ran as `slot sh -c`.
- The mutant script started without `slot`; its cargo calls went through the shim.
- `rustfmt` ran on this host and compiles nothing.

```findings
[
  {"file": "crates/inbox-cli/src/wake.rs", "line": 193, "category": "test-hook-liveness", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Watch::before_exit waits for ever on done > begun when the watch thread already returned (reader gone with the hold set before begun is read), so a debug wake with --before-exit hangs; the story's case has the same window from its drop of the reader to the read of begun."},
  {"file": "crates/inbox-cli/src/wake.rs", "line": 190, "category": "test-hook-liveness", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "Watch::before_exit checks only that standard output is a pipe, not that the watch thread was started, so a wake that ends before surface.watched() (no reader at its start, --follow refused by another adapter's record) hangs at the point, against the sentence of docs/design/wake.md."},
  {"file": "crates/inbox-cli/tests/wake.rs", "line": 630, "category": "test-gap", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The story's case holds only a wake without --follow: the mutant that sets the end hold to !follow stays green under it and is killed only by the adversary case of a --follow wake refused after a full round."}
]
```
