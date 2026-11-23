---
format: aep.planning-md/3
id: story:a-character-a-reader-cannot-see-is-shown-as-its-escape
kind: story
status: draft
title: A character a reader cannot see is shown as its escape
summary: Zero width and tag characters in a message's text are printed as sent, so 2 texts that differ can look the same on a terminal
revision: 1
---
## Outcome

A character a reader cannot see in a message's text is shown as its escape by every command
that prints text another principal wrote, so a line on a terminal holds no text that is there
and not visible.

## Why

`output::visible` shows control characters, the separators U+2028 and U+2029 and, since
story:the-terminal-adapter-shows-a-post, the characters that change the direction of text as
their escapes. The implementor's round 2 of that story named format characters it still prints
as sent and that take no width on a terminal: U+200B (zero width space), U+2060 to U+2064,
U+FEFF, U+00AD (soft hyphen) and the tag characters U+E0000 to U+E007F. A sender can put them
into a body, a reply, a refusal reason or an option text. They do not reorder a line; they make
2 texts that differ look the same, and they carry text a person does not see and a program that
reads the line does.

## Acceptance

- `pending`, `wait`, `wake`, `outbox`, `collect`, `thread` and `replies` show each of the
  characters the story decides on as its escape (`\u{200b}`), in every member a sender sets.
- U+200C and U+200D (the joiners that scripts and emoji sequences need) are printed as sent, or
  the story says why not.
- `--json` output is unchanged.
- A unit case of `output.rs` per range, each red with its range taken out; a case that text of
  a script that uses the joiners is printed as sent.

## Scope

`crates/inbox-cli/src/output.rs` (`visible`) and its unit cases; the sentence of
`website/docs/first-use.md` under "What the commands print"; the row of `crates/inbox-cli/` in
`AGENTS.md`; `docs/checks.md`; `CHANGELOG.md`.

## Decisions

Open: which characters. (A) the list above; (B) every character of the general category Cf
but U+200C and U+200D, which needs a table of the category or a new dependency; (C) none, and
the first-use page says that such characters are printed as sent. Default: A.
