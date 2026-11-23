---
format: aep.planning-md/3
id: review-result:adversary-unseen-characters-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on the characters a reader cannot see: no product defect, 4 findings on documents and cases'
relations:
- reviews: story:a-character-a-reader-cannot-see-is-shown-as-its-escape
revision: 1
---
Adversary pass 1 on story:a-character-a-reader-cannot-see-is-shown-as-its-escape (inbox, HEAD df5fbff).

I found no product defect. `unseen` escapes exactly the decided list, and no printing path skips `visible`. All 6 mutants named in the brief die, except one that cannot change behaviour. There are 3 findings in the prose and one gap in the cases (any character added away from the decided ranges goes unnoticed).

Where things are:
- Sweep case: <scratch>/bl/escape/adversary/output_cases.rs. It goes in `crates/inbox-cli/src/output.rs`, `mod tests`, after `a_document_keeps_a_character_text_shows_escaped_as_it_was_sent`.
- Copy: <scratch>/bl/escape/adversary/copy. The sweep is inserted in its output.rs between `// ADVERSARY-SWEEP-BEGIN`/`END` marker lines; leave those out when moving it.
- No `crates/inbox-cli/tests/adversary_unseen.rs` is delivered. My probe there was neither a red defect nor a mutant killer, so I removed it and its `mod` line from the copy.
- The copy's `target` is deleted. Logs are in the same scratch directory (`run1.log`, `run2.log`, `mutants.log`, `m-*.log`, `s-*.log`, `clippy.log`).

Findings

F1 (`website/docs/first-use.md:118`, introduced). The new sentence ends "such as `\u{200b}`, so that 2 texts that differ cannot look the same". The same paragraph (lines 126-128) says a sender who types the 8 characters `\u{202e}` gets the same line as one whose text holds the character. The same holds for the new set: `visible` gives `"a\\u{200b}b"` both for the typed text `a\u{200b}b` and for `a<U+200B>b`, because a backslash is not escaped. That is pre-existing and decided in `docs/design/wake.md:452`.
- The fix lands in first-use.md: say what the escape does ("so that such a character is not hidden in a line"), not that differing texts cannot look alike.

F2 (`docs/checks.md:197`, introduced). It says "10 unit cases of `output.rs` hold it". The change adds 11 `#[test]` functions: 8 for the ranges, plus the joiners case, the borrowed case and the document case. The brief's "10 unit cases" has the same miscount. Fix: write 11.

F3 (`docs/checks.md:192-193`, introduced). It says the list was "named by the implementor's round 2 of that story". Round 2 named U+00AD, U+200B, U+2060-2064, U+FEFF and the tags (the story's Why). U+180E, U+206A-206F and U+FFF9-FFFB were added by the coordinator's decision. Fix: say which part came from where.

F4 (`crates/inbox-cli/src/output.rs`, the story's unit cases, introduced, a gap in the cases). The cases hold only the neighbours of each range. A mutant that adds a character away from every range survives them all:
- `| '\u{a0}'` survived (story cases exit 0); the sweep is red with "U+00A0 escaped true".
- `| '\u{3164}'` survived the same way.

So the claim "nothing more is escaped" is unheld. The kill is the sweep case in output_cases.rs.
- `every_scalar_value_is_escaped_exactly_when_0_6_0_or_the_decided_list_escapes_it` runs every scalar U+0000..=U+10FFFF between 2 letters through `visible`.
- It expects an escape exactly when release 0.6.0 escaped it (control but `\n`, U+2028/2029, the bidirectional set; taken from 9c88a0d) or the character is in the decided list.
- It also checks the escape text, and that every other character comes back borrowed, as the same pointer.
- Green on HEAD in 1.499 s; clippy `-D warnings` exit 0; rustfmt `--check` clean.

What I checked that holds (no finding)

1. Printing paths:
   - Nothing in `crates/inbox-cli/src` outside output.rs calls `println!`, `eprintln!` or writes to stdout/stderr. Every text goes through `print_text`, `try_print_text`, `try_print_round` (`wake --follow`) or `warn`, and all four call `visible`. That covers `read`, `answer`, `refuse` and `post` (which print only an outcome and ids), the error lines, and serve's lines.
   - clap's `error.print()` repeats only the caller's own arguments.
   - `wake.sh` hands on rows that were already escaped. Its `cut -b 1-300` can split an escape such as `\u{20`, but the result is still visible ASCII, and the same cut existed before this change.
2. Which members a sender can set:
   - The client decoder refuses a thread id or option letter with U+200B (exit 2). Thread and message ids are Crockford base32 of length 26; a letter is A-Z, one character.
   - Posting to an address with U+200B is refused `inbox.messaging.UnknownInbox (409)`. Addresses and principals come from the operator's key records.
   - `dedup_key` is not limited, but the text output does not print it.
   - So the members a sender can fill are body, option text, reply and refusal reason, which the story's command case covers.
3. `--json`: the diff touches only `visible` and the cases. `document`, `plain_document`, `print_document` and `try_print_document` are byte-identical to c42f5aa, so the JSON output is unchanged by construction. The CHANGELOG's "No flag, exit status or `--json` output changes" holds.
4. Width: the client computes no width and makes no cut or wrap in `wake --follow`, so there is nothing to be off after escaping.
5. The cases themselves:
   - `each_shown_escaped` compares against an expected text built from the code point. A `visible` that escaped everything would escape the letters `a` and `b` too and fail.
   - `each_printed_as_written` and the borrowed cases use `std::ptr::eq` on `Cow::Borrowed`, so a copied text fails.
   - The command case's `--json` check would fail if `visible` were applied to the document: `\u{200b}` is not valid JSON, so the parse panics.
6. The AGENTS.md row and the CHANGELOG bullet match the code.

Mutants (story cases with the sweep excluded / sweep alone)
- U+2060..=U+2065: caught (the word-joiner case, the case beside the bidirectional controls, the borrowed case) / sweep caught.
- U+2069..=U+206F: no change in behaviour, because U+2069 is already a bidirectional control. Both stay green. Not a finding.
- U+FFF8..=U+FFFB: caught (interlinear case, borrowed case) / sweep caught.
- U+E0000..=U+E0080: caught (tag case, borrowed case) / sweep caught.
- U+00AC..=U+00AD: caught (soft hyphen case, borrowed case) / sweep caught.
- U+200A..=U+200B: caught (zero width space case, borrowed case) / sweep caught.
- `| '\u{a0}'` added: survived / sweep caught (F4).
- `| '\u{3164}'` added: survived / sweep caught (F4).
- `| '\u{e0100}'` added: caught (joiners case) / sweep caught.
- `| '\u{200d}'` added: caught (joiners, beside-bidirectional and borrowed cases) / sweep caught.

Times: the story's command case took 2.794 s, the sweep 1.499 s, the output.rs cases 1.58 s in all. Nothing came near 60 s.

Not checked:
- The "taken out" mutants and the "set applied to `--json`" mutant that the implementor reported; I did not rerun them.
- A full `nextest` run of the crate, and `task test`.
- A real terminal render.
- `docs/design/wake.md:449`, which still lists only the bidirectional set beside `visible`. It is not false, just incomplete.

```findings
[
  {"file": "website/docs/first-use.md", "line": 118, "category": "docs", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The new claim that the escapes mean 2 texts that differ cannot look the same is false: a typed `\\u{200b}` and a real U+200B print the same line, as the same paragraph already says for `\\u{202e}`."},
  {"file": "docs/checks.md", "line": 197, "category": "docs", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "It says 10 unit cases of output.rs hold the set; the change adds 11."},
  {"file": "docs/checks.md", "line": 192, "category": "docs", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "It says the whole list was named by the implementor's round 2, but U+180E, U+206A-206F and U+FFF9-FFFB were added by the coordinator's decision."},
  {"file": "crates/inbox-cli/src/output.rs", "line": 591, "category": "tests", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The story's unit cases hold only the neighbours of each range, so a mutant that also escapes a character away from them (U+00A0, U+3164) survives; the sweep case in output_cases.rs kills it in 1.5 s."}
]
```
