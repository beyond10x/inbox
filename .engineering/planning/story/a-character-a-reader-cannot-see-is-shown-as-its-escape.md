---
format: aep.planning-md/3
id: story:a-character-a-reader-cannot-see-is-shown-as-its-escape
kind: story
status: implemented
title: A character a reader cannot see is shown as its escape
summary: Zero width and tag characters in a message's text are printed as sent, so 2 texts that differ can look the same on a terminal
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T09:28:29Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T09:28:30Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-23T10:08:56Z", actor: "agent:inbox", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":4}}}
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

- Which characters (the story's Open item; options: (A) the list of the Why; (B) every
  character of the general category Cf but U+200C and U+200D, which needs a table of the
  category or a new dependency; (C) none, and the first-use page says that such characters are
  printed as sent). Taken by the coordinator when the story was accepted: A, with 3 ranges of
  the same kind added, each one arm of a `matches!`: U+180E (Mongolian vowel separator), U+206A
  to U+206F (the deprecated format characters) and U+FFF9 to U+FFFB (interlinear annotation).
  The set of `output::unseen` is U+00AD, U+180E, U+200B, U+2060 to U+2064, U+206A to U+206F,
  U+FEFF, U+FFF9 to U+FFFB and U+E0000 to U+E007F. B was not taken: Cf holds characters a
  terminal shows (the Arabic number signs U+0600 to U+0605, U+06DD), so the category is not the
  rule.
- Printed as sent: U+200C and U+200D (the acceptance), and with them U+034F (the combining
  grapheme joiner) and the variation selectors U+FE00 to U+FE0F and U+E0100 to U+E01EF: scripts
  and emoji sequences need them. The flag of a part of a country is an emoji tag sequence and is
  shown as a black flag followed by the escapes of its tags; the first-use page says so.
- The list is closed for this story. The implementor and the adversary named characters outside
  it that a terminal shows with no width or blank: U+1D173 to U+1D17A (musical format
  characters), U+1BCA0 to U+1BCA3 (shorthand format controls) and the Hangul fillers U+115F,
  U+1160, U+3164 and U+FFA0 (letters, shown blank). They are printed as sent. No draft is filed
  for them.
- The escape is not told apart from typed text: a sender who types the 8 characters `\u{200b}`
  gets the same line as one whose text holds U+200B, because a backslash is not escaped. That is
  the rule of release 0.6.0 for every escape (`docs/design/wake.md`, the first-use page) and is
  not changed here. The first-use page says what the escape does (no such character is hidden
  in a line) and no longer that 2 texts that differ cannot look the same (adversary pass 1,
  finding 1).
- Adversary pass 1 (review-result:adversary-unseen-characters-pass-1) found no defect of the
  product and 4 findings on documents and cases. The coordinator fixed them: the sentence of
  the first-use page, 2 statements of `docs/checks.md`, and the adversary's sweep of every
  scalar value moved into the unit cases of `output.rs`, because the story's cases held only
  the neighbours of each range and a character added away from them (U+00A0, U+3164) survived.

## Measured

- Red first: with the cases written and `visible` unchanged, `cargo nextest run -p inbox-cli
  --locked`: 278 run, 269 passed, 9 failed (the 8 range cases and the command case).
- Green: 278 passed (266 before the story); with the adversary's sweep case 279.
- Mutants, each red against a named case: each of the 8 ranges or characters taken out; a
  range widened by one (U+2060 to U+2065, U+FFF8 to U+FFFB, U+E0000 to U+E0080, U+00AC to
  U+00AD, U+200A to U+200B); the joiners, U+034F or the variation selectors added; U+00A0 or
  U+3164 added (red against the sweep only); the set applied to `--json`. A range widened to
  U+2069 changes nothing: that character is a bidirectional control and was escaped already.
- Times: the command case 1.0 to 2.8 s, the sweep of every scalar value 1.5 s, every other new
  case under 0.1 s.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-23T10:04:48Z to 2026-11-23T10:07:55Z, load average 71 to 74
  on 20 CPUs: exit 0, `894 tests run: 894 passed, 8 skipped`, the test step 56 s of
  its bound of 120 s.
