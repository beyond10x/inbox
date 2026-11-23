---
format: aep.planning-md/3
id: story:an-unreadable-epochs-file-keeps-its-other-lines
kind: story
status: draft
title: A change of the epochs file that cannot read it loses no other line
summary: A change of the epochs file reads an unreadable file as empty and replaces it with its own entry, so every other epoch and mark is lost without a warning.
revision: 1
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
