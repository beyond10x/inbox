---
format: aep.planning-md/3
id: story:the-client-prints-no-control-sequence-of-another-principal
kind: story
status: implemented
title: The client prints no control sequence of another principal
summary: The text output of the client prints a reply body or refusal reason of another principal with its terminal control sequences.
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T15:39:36Z", actor: "agent:inbox", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-22T15:39:36Z", actor: "agent:inbox", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-22T16:45:32Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The command-line client writes no control character that another principal chose to the
terminal of the person or agent who runs it.

In 0.2.0 the text output of `inbox collect` prints the body of a reply and the reason of a
refusal as the recipient wrote them. A recipient can put terminal control sequences there: an
erase of the screen, a window title set by OSC, a carriage return that writes over the line, a
C1 control sequence introducer. The sender's terminal executes them. The same printer serves
`inbox outbox`, `inbox pending` and `inbox thread`, so a sender can do the same to whoever
lists an inbox.

## Acceptance

- In the text output of every command of the client, a field whose value another principal set
  (a body, a reason, an option, an address, a key id) contains no control character but the
  line end: each other control character (C0, DEL, C1) is shown in a visible escaped form
  (state transition: the bytes ESC `[2J` in a reply body reach the terminal in 0.2.0 and are
  shown as visible text after this story).
- The case `control_sequences_of_the_recipient_do_not_reach_the_senders_terminal` passes for
  `collect` of an answered and of a refused message; a case of the same form covers `pending`,
  `outbox` and `thread`.
- A line of the client's own (an error on standard error that repeats text of the service)
  holds to the same rule.
- With `--json` the value a JSON reader gets is unchanged, and no raw control character is in
  the output: a control character that JSON does not have to escape is written as a `\u`
  escape.
- A body without control characters is printed byte for byte as before.
- The stored message, the signature check and the service are unchanged: only what the client
  prints changes.

## Context

Found by the second adversary pass on story:collect-returns-the-settled-record
(review-result:adversary-pass-2-on-collect-returns-the-settled-record, finding 1). It existed
in the release 0.2.0 and is not among the findings of the source review of that release. Built
on the branch of that story and published with it
(approval-record:d22-the-client-prints-no-control-sequence-of-another-principal).

## Scope

- `crates/inbox-cli/src/output.rs`: the 3 sinks `print_text`, `print_document` and `warn`, through
  which every write of the client to standard output and standard error goes; `visible`
  (each control character but the line end as `\u{..}`), `plain_document` (DEL and the C1
  range as `\u00XX` in a JSON document); `indent` splits on the line end only. 3 unit cases,
  one of which fails when another file of `src/` writes to either stream.
- `crates/inbox-cli/src/main.rs`, `src/serve.rs`: their output goes through the sinks.
- `crates/inbox-cli/tests/control_sequences.rs` and `tests/adversary_control_sequences.rs` (new),
  `tests/main.rs`.
- `docs/checks.md`, `AGENTS.md`, `CHANGELOG.md`, `website/docs/first-use.md`,
  `website/docs/status.md`.
- `ess/`, `generated/`, the service and the clap definition are unchanged.

## Stays

- A line end in a value of another principal is printed as a line end.
- Format characters that are no control characters (the Unicode category Cf: bidirectional
  overrides, zero-width characters) are printed as they are.
- A backslash is not escaped, so a body that holds the text of an escape reads like one.
