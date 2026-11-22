---
format: aep.planning-md/3
id: story:a-variable-that-is-not-utf-8-is-a-usage-error
kind: story
status: draft
title: An INBOX variable that is not UTF-8 is a usage error, not an unset variable
revision: 1
---
## Problem

`inbox serve` reads `INBOX_BACKEND` with `std::env::var(..).ok()`. A value that is not UTF-8
reads as unset, so the service starts with the backend `memory` without a word. An unknown
backend name in UTF-8 is refused with the exit status 2.

Found by adversary pass 2 on story:an-argument-that-is-not-utf-8-is-a-usage-error
(review-result:adversary-an-argument-that-is-not-utf-8-pass-2, finding 4, pre-existing).

## Acceptance

`inbox serve` with an `INBOX_BACKEND` that is not UTF-8 ends with the exit status 2 and one
line of printable ASCII on standard error that names the variable, and opens no listener
(state transition: the service starts with `memory` before, exit status 2 after). The same
holds for every other `INBOX_*` variable the client or the service reads as text.

## Scope

- `crates/inbox-cli/src/main.rs`: the reads of `INBOX_*` variables.
- Cases in `crates/inbox-cli/tests/`.
