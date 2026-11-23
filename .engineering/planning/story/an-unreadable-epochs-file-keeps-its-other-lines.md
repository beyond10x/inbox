---
format: aep.planning-md/3
id: story:an-unreadable-epochs-file-keeps-its-other-lines
kind: story
status: implemented
title: A change of the epochs file that cannot read it loses no other line
summary: A change of the epochs file reads an unreadable file as empty and replaces it with its own entry, so every other epoch and mark is lost without a warning.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T08:38:28Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T08:38:28Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T13:34:33Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":9}}}
---
## Problem

A change of the epochs file beside the outbox (`crates/inbox-cli/src/outbox.rs:394`) reads a
file it cannot read as empty and then replaces it with the change's own entry only. "Cannot
read" is: no permission to read, not UTF-8, over its bound, or no regular file. Every other
recorded epoch and every `none` mark is lost, and for a permission error or a file that is not
UTF-8 no warning line is written. The file `<outbox>.opened` is rewritten the same way; there
the loss means an inbox is not opened again by a reconnect.

No envelope is lost: the outbox file itself fails the command with exit status 4 before any
write. Nothing in the product brings the epochs file into such a state.

Found by adversary pass 2 on story:the-client-reconnects-after-a-restart
(review-result:adversary-the-client-reconnects-after-a-restart-pass-2, finding 8). The same
read is in the code before that story (`fs::read_to_string(..).unwrap_or_default()`).

## Acceptance

A command that has to change the epochs file and cannot read it fails with exit status 4 and
one line that names the file, and the file is unchanged (state transition: the file is replaced
with one entry before, and is byte-identical after). Or the design says that the file is a
cache that may be lost, and the command writes one warning line in each of the 4 cases.

## Open before it is accepted

- Whether the change fails (as for the outbox file) or warns and goes on. A failed change makes
  `inbox post` fail on a file that holds no message.
- Whether `<outbox>.opened` follows the same rule.

## Scope

- `crates/inbox-cli/src/outbox.rs`, `crates/inbox-cli/tests/`
- `docs/design/service.md` ("The epoch")

## Decisions

- Whether the change fails or warns (the first item of "Open before it is accepted"; options:
  (A) the change fails with exit status 4 and names the file, as for the outbox file; (B) the
  design calls the file a cache and the command writes a warning line). Taken by the
  coordinator when the story was accepted: A. An entry of the epochs file decides whether a
  reconnect opens an inbox again, so losing the file is not harmless.
- Whether `<outbox>.opened` follows the same rule (the second item): yes, it is written through
  the same function and read by the same reader.
- How many lines name the file (adversary pass 1, finding 2; options: (A) exactly one line in a
  run that ends with 4; (B) the design names the 3 lines the first build wrote). Taken: A.
- A repost that a transport failure ends (adversary pass 1, finding 1 and round 2; options: (A)
  the run keeps exit status 3 and prints the line of the local failure as well; (B) exit status
  4 wins). Taken: A. Exit status 3 tells a caller to try again, which is still true.
- What `replace` does with an entry at `<file>.new` (adversary pass 2, finding 1; options: (A)
  `replace` removes it and creates the staged file with `O_CREAT | O_EXCL` and mode 0600; (B)
  the failed `clear_unsent` of `inbox post` goes into the exit status and `replace` stays; (C)
  both; (D) a new draft). Taken: A. A link left there made the file a link to its target and
  lost every entry, and a FIFO there blocked the command for good; both were in release 0.6.0.
  The same rule is given to the `replace` of the key file (`crates/inbox-cli/src/keys.rs`) in
  this story, so that no document names the fault of one file while the other still has it.
- The case `reconnect::the_record_of_the_opened_inbox_is_written_under_the_lock` held the
  writer inside `replace` with a FIFO at `<outbox>.opened.new`. `replace` no longer opens that
  FIFO. Options: (A) the case checks that the FIFO is untouched while the command waits for the
  lock and gone once it holds it; (B) a case that stops the child at the moment the staged file
  appears. Taken: A. B fails on a loaded host. The case no longer holds that the lock is held
  until the staged file is renamed.
- `inbox repost` drops, in the run that ends with 4, the envelopes that expired and those the
  service refused for good (adversary pass 2, finding 2). That is kept: the design, the
  changelog and the row of `AGENTS.md` name it, and a case holds it.
- 2 adversary passes are recorded: review-result:adversary-unreadable-epochs-pass-1 (6
  findings, 2 red cases) and review-result:adversary-unreadable-epochs-pass-2 (3 notes, 2 red
  cases); every finding has its outcome in the evidence of this story.

## Measured

- `cargo nextest run -p inbox-cli --locked` after round 4, on the branch before `plan/stage-3`
  was brought in: 285 passed (266 before the story).
- Red first, round 4: against the old `replace` of `keys.rs` the case
  `key_file_staged::a_link_at_the_staged_place_of_the_key_file_is_removed_not_followed` failed
  (the key file had become a symbolic link). Round 3: against the old `replace` of `outbox.rs`
  the case `a_stale_entry_at_a_staged_file_is_removed_not_followed` ran into its 60 s bound (the
  command blocked on the FIFO).
- Mutants, each red against a named case, for each of the 2 `replace` functions: `create` and
  `truncate` back; the stale entry not removed; the mode at creation dropped.
- Times: the 8 rows of the stale-entry case 2.1 to 3.2 s, the repost case 2.0 to 2.7 s, the
  key file case 0.4 to 1.0 s, the unit case of `keys.rs` under 0.1 s, the changed reconnect
  case 0.9 to 1.6 s (10 s before the change).
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T13:16:40Z to 2026-11-23T13:33:58Z, load average 55 to 104
  on 20 CPUs: exit 0, `913 tests run: 913 passed, 8 skipped`, the test step 118 s of
  its bound of 120 s.
