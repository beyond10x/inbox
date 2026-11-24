---
format: aep.planning-md/3
id: story:a-wake-that-ended-writes-no-second-line-when-its-reader-goes
kind: story
status: draft
title: A wake that ended writes no second line when its reader goes
summary: 'The hold of the watch at the end of the wake has no case: with it removed, all 49 cases of the wake stay green.'
revision: 1
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
