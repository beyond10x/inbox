---
format: aep.planning-md/3
id: story:a-wake-that-ended-writes-no-second-line-when-its-reader-goes
kind: story
status: implemented
title: A wake that ended writes no second line when its reader goes
summary: 'The hold of the watch at the end of the wake has no case: with it removed, all 49 cases of the wake stay green.'
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/wake.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_wake_hold.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: cited
  path: crates/inbox-cli/tests/wake.rs
- confidence: cited
  path: docs/design/wake.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T11:58:49Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T11:58:49Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T15:25:24Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

A case holds that an `inbox wake` which ended for another reason writes no second line and
keeps its exit status when the reader of its standard output goes in the moment before the
process ends.

`crates/inbox-cli/src/wake.rs` sets `*watch.hold() = true;` at the end of `run` for that. With
the line replaced by a statement that does nothing, every case of the modules `wake`,
`reader_gone`, `adversary_reader_gone` and `follow` stays green (adversary pass 2 on
story:the-chat-bridge-posts-a-pending-message, 2026-11-23: 49 of 49 passed). The line is the
same on `main` since story:a-wake-whose-reader-is-gone-marks-nothing.

## Acceptance

- A case ends a wake with a refusal (exit status 1 and its one line) while the pipe of its
  standard output loses its reader before the process ends, and shows one line and the status
  1; with the hold removed the case is red (state transition: a wake that ended is not ended a
  second time by its watch).
- The case holds the moment without a pause that stands for a fact; if it needs a point in the
  code where a test can hold the process, that point is compiled for tests only and the
  release binary does not have it.
- The case takes under 5 s; `task check` exits 0.

## Context

A pre-existing finding of an adversary pass, filed as a draft. It waits until the planned
waves are delivered.

## Scope

Written after the build, from the diff against `main` at ab34af6: every path below is read
from it (cited). A scope report before the build named the same places; its counts of cases
were wrong (the modules `wake`, `reader_gone` and `adversary_reader_gone` held 17, 6 and 1
cases at ab34af6, 24 together; a module `follow` does not exist).

Code:

- `crates/inbox-cli/src/wake.rs`: the point before the exit (`Watch::before_exit`), the
  counters of the looks of the watch (`Looks`), both under `#[cfg(debug_assertions)]`; `run`
  takes the path of the point and calls it after the hold is set and the line written.
- `crates/inbox-cli/src/main.rs`: the hidden argument `--before-exit` of `inbox wake`, under
  `#[cfg(debug_assertions)]`; a build without debug assertions passes `None`.
- `crates/inbox-cli/tests/wake.rs`: the case of the story.
- `crates/inbox-cli/tests/adversary_wake_hold.rs` and one `mod` line in
  `crates/inbox-cli/tests/main.rs`: the cases of the adversary pass, when it leaves any.

Documents:

- `docs/design/wake.md`, section "A reader that is gone": the bullet on a wake that ended in
  another way names the case and the point.
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`.

Not touched: `generated/cli/inbox.json` (a hidden argument is not in the reference),
`CHANGELOG.md` (nothing a user sees changes), `docs/checks.md` (it counts no case of the
wake).

## Decisions

- The story was a draft of the backlog (a finding of an adversary pass that was there before
  its wave). The operator's session accepted it with 2 others as one wave, one pull request
  each, no release from them alone (2026-11-24). No panel of plan critics read it; one
  adversary pass reviewed the build.
- The case ends the wake by a refused read (the stand-in answers `403` with
  `{"refused":"unauthenticated"}`: exit 1, the line `inbox wake: refused: unauthenticated
  (403)`), not by a refused mark. A wake without `--follow` sets the hold in `deliver` before
  it sends its marks, so a refused mark never reaches the line of the story: a probe case
  with a refused mark passed 5 of 5 runs with the hold line of `run` removed. The options
  were: (A) a refused read; (B) a refused mark with `--follow`. A is taken: it is the
  shortest wake that ends at the line of the story.
- The point where a case holds the process. The options were: (A) a hidden argument of
  `inbox wake`, compiled under `#[cfg(debug_assertions)]` only, in the form the repository
  has for `--clock-file`; (B) an environment variable read in a debug build; (C) no point,
  and a case that closes the reader at a time it guesses. A is taken: `#[cfg(test)]` does
  not reach the binary the cases start, an environment variable is inherited by every child
  of a case, and C is the pause that stands for a fact the acceptance rules out.
- The point is `inbox wake --before-exit <PATH>`. Once `run` has set the hold and written
  the line of the wake's end, it creates the file (`File::create_new`), waits until the file
  is gone again, and then waits until the watch has done a look that it began after that
  (`Looks { begun, done }`, 2 counters the watch increments, in a debug build only). The
  case waits for the file, closes the reader, removes the file, and reads the end of the
  process: the look the point waits for began after the reader went, so it finds the pipe
  without a reader. With the hold in place that look returns; with the hold removed it ends
  the process with the second line and the status 4 before the point returns.
- A release binary does not have the point: the 2 `cfg` lines hold that, and no case does.
  The same holds for `--clock-file` of `inbox serve` and `inbox bridge`. The options were:
  (A) no release build in the gate, the `cfg` lines read, one run by hand recorded under
  Measured; (B) a release build of `inbox-cli` in `task check` with a case that passes the
  flag. A is taken: B costs a second build of the binary in every run of the gate for a
  flag that changes nothing but the moment of the exit.
- The behaviour of a release binary does not change, so `CHANGELOG.md` gets no bullet. The
  generated command-line reference leaves hidden arguments out
  (`crates/inbox-cli/src/reference.rs`, `!arg.is_hide_set()`), so `generated/cli/inbox.json`
  does not change.
- What the story's text had wrong, left as written in Outcome: it names a module `follow`,
  which does not exist, and "49 of 49", which is the count of another selection; at ab34af6
  the modules `wake`, `reader_gone` and `adversary_reader_gone` hold 24 cases.
- After the adversary pass: the point waits for a look only while a watch thread runs. A
  fact of a debug build, "the watch runs", is set just before the thread is started
  (cleared again when the start fails) and cleared by the watch before it returns. Before
  round 2 the point waited for ever when the watch had returned (the reader gone with the
  hold set) and when no watch was started (no reader at the start of the wake, `--follow`
  refused by another adapter's record). The case of the story still cannot pass with the
  hold removed: the watch returns, and clears the fact, only when it finds the reader gone
  and the hold set; with the hold removed that look ends the process with the second line
  and the status 4.
- The end of a wake with `--follow` is held by the adversary's case
  (`adversary_wake_hold::a_follow_wake_refused_after_a_full_round_writes_one_line_when_its_reader_goes`),
  which stays where it is; `docs/design/wake.md` names both cases.
- Not held by a case, said here: the wait of the point itself. With the hold in place, the
  look the point waits for changes nothing a case can read (no line, no status, no file), so
  "the point waited" and "the point returned at once" differ only in time. 2 mutants of the
  point live (the fact never set; the fact cleared before the look), and each of them
  together with the hold removed is red only through a pause or a race (see Measured). The
  end where the signal handlers cannot be set up is held by reading: it returns before the
  watch starts, the path the 2 cases without a watch hold.

## Measured

- The case of the story, `wake::a_wake_ended_by_a_refusal_writes_no_second_line_when_its_reader_goes_before_the_exit`:
  green 10 of 10 with the product; with the hold line of `run` replaced by `();` red 10 of
  10 (`tests/wake.rs:656`: status 4, standard error `inbox wake: refused: unauthenticated
  (403)` and then `inbox wake: standard output: it is a pipe or a FIFO whose reader is gone:
  no message is marked delivered`). A probe case with a refused mark passed 5 of 5 with the
  hold removed, so the case refuses the read.
- Round 1: the cases of `inbox-cli` 434 → 435, 2 runs in a row `435 tests run: 435 passed, 0
  skipped`. 4 mutants: the hold removed and the hold set after the point, each red 10 of 10
  in the case; the watch returns whenever the reader is gone, red in 4 of 6 cases of
  `reader_gone` and one of `wake`; the point compiled in every build, which no check
  notices.
- A release binary has no point: `cargo build --release --locked -p inbox-cli` and then
  `inbox wake --before-exit x` on the build node gave `error: unexpected argument
  '--before-exit' found`, exit 2 (round 1, and again in the adversary pass). `--clock-file`
  of `inbox serve` and `inbox bridge` gave the same.
- Adversary pass 1 (review-result:adversary-wake-hold-pass-1): 3 findings, 3 cases, 2 of
  them red: the point hung when the watch had returned (the reader gone 1.2 s, more than 2
  looks, before the file is removed) and when no watch was started (no reader at the start
  of the wake), each `inbox wake hangs at the point, not ended within 10s`; the hold set to
  `!follow` passed the case of the story 10 of 10 and was red 10 of 10 in its green case
  with `--follow`. It found no end that writes a second line with the hold in place. The
  case of the story passed 150 of 150 there, its slowest run 0.579 s. It did not run: the
  2 other ends without a watch, a loaded host, `task check`.
- Round 2 on those findings (all 3 recorded as fixed): the 2 red cases green, each 10 of
  10; one case added, `wake::a_follow_wake_refused_by_another_adapter_does_not_hang_at_the_point`,
  red before the change (`inbox wake did not end within 10 s`). The cases of `inbox-cli` 438
  → 439, 2 runs in a row `439 tests run: 439 passed, 0 skipped`; the modules `wake`,
  `reader_gone`, `adversary_reader_gone` and `adversary_wake_hold` 10 of 10. The release
  build compiles without a warning (`cargo clippy --release --workspace --all-targets
  --locked -- -D warnings` exit 0).
- Mutants of round 2, over the 5 cases of the point, 10 runs each: the hold removed, red 10
  of 10 in the case of the story, the case with `--follow` and the case with 2 looks; the
  hold set to `!follow`, red 10 of 10 in the case with `--follow`; the fact never cleared,
  red 10 of 10 in the case with 2 looks (it hangs at its bound). 2 mutants of the point
  live: the fact never set, and the fact cleared before the look. With the hold removed as
  well, the first is red 10 of 10 only in the case with 2 looks, through its pause of 1.2 s
  (the case of the story and the case with `--follow` stay green: the point returns at once
  and the process ends before the next look); the second is red 10 of 10 in all 3, by a
  race that went one way 10 of 10. The wait `<=` changed to `<` with the hold removed: red
  10 of 10 only in the case with 2 looks.
- The cases: 2 of the story in `wake`, 3 of the adversary in `adversary_wake_hold`. Times
  over 10 runs: the case of the story 0.53 to 0.62 s, the case with `--follow` 0.53 to 0.60
  s, the case with 2 looks 1.23 to 1.31 s, the 2 cases without a watch 0.03 to 0.10 s. No
  case over 5 s.
- Not held by a case: that a release binary has no point (the `cfg` lines, and the run by
  hand above); the wait of the point itself; the end where the signal handlers cannot be set
  up. No adversary pass read round 2.
- The gate runs of the rounds on the build node (`task check`): round 1 1128 passed, 8
  skipped, the test step 21 s, exit 0; round 2 1132 passed, 8 skipped, 19 s, exit 0.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T15:21:33Z to 2026-11-24T15:22:30Z, on the build
  node of the pool: exit 0, `1150 tests run: 1150 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
