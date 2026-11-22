---
format: aep.planning-md/3
id: story:an-argument-that-is-not-utf-8-is-a-usage-error
kind: story
status: draft
title: An argument that is not UTF-8 is a usage error
summary: An argument of inbox that is not UTF-8 ends the client with a panic and exit status 101, outside the documented statuses 0 to 4.
revision: 2
---
## Problem

`inbox` reads its arguments with `std::env::args()`. An argument that is not UTF-8 ends the
client with a panic and the exit status 101. `AGENTS.md` and the reference document of the
command line state the exit statuses 0 to 4, and 2 for wrong usage. The panic message shows
the argument in its escaped form, so no control sequence reaches the terminal.

Found by adversary pass 1 on
story:the-client-prints-no-control-sequence-of-another-principal (finding F7, pre-existing).

## Acceptance

An argument of any command that is not UTF-8 ends `inbox` with the exit status 2 and one line
on standard error that names the argument in an escaped form, and sends no request.

## Scope

- `crates/inbox-cli/src/main.rs`: the arguments are read with `std::env::args_os()`.
- One case in `crates/inbox-cli/tests/`.
