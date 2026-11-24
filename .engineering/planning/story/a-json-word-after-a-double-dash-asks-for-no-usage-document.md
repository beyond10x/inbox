---
format: aep.planning-md/3
id: story:a-json-word-after-a-double-dash-asks-for-no-usage-document
kind: story
status: draft
title: A --json word after a bare -- asks for no usage document
revision: 1
---
## Outcome

A word `--json` after a bare `--` is a value, not the flag: a command line that is refused
before clap accepted it prints the usage document on standard output only when `--json`
stands before the `--`.

Today the path of a clap error in `crates/inbox-cli/src/main.rs` asks for the document with
`arguments.iter().any(|arg| arg == "--json")`, over every argument. `inbox read -- --json`
with an `INBOX_URL` that is not UTF-8 prints `{"error":"usage","detail":"INBOX_URL is not
UTF-8: …"}` on standard output, although `--json` there is the message id and the same line
without the variable prints no document. The same holds for an argument that is not UTF-8
beside such a word.

## Acceptance

- `inbox read -- --json` with `INBOX_URL` set to a value that is not UTF-8 exits 2 with the
  one line that names the variable and prints nothing on standard output (state transition:
  standard output moves from one document to empty).
- `inbox --json read -- x` and `inbox read --json -- x` with the same variable still print
  the usage document.
- The case of the adversary pass, below, is green in `crates/inbox-cli/tests`.
- `task check` is green.

## Context

Found by review-result:adversary-variable-not-utf-8-pass-1 (finding F1, a note, there before
story:a-variable-that-is-not-utf-8-is-a-usage-error: main at a1881d9 has the same expression
and printed the document with clap's message for this line). What reaches it: a message id
or a thread id spelled `--json` after `--`.

The pass's case, left out of the suite because it is red today (it uses the helpers of
`crates/inbox-cli/tests/adversary_variable_not_utf_8.rs`: `Dir`, `words`, `ran`, `os`,
`named`):

```rust
/// `--json` after `--` is a value, not the flag: the run without the bad variable prints no
/// document, and the run with it should not either.
#[test]
fn a_json_word_after_a_double_dash_asks_for_no_document() {
    let dir = Dir::new("adv-var-json-dash");
    let line = words(&["read", "--", "--json"]);
    let before = ran(&dir, &line, &[]);
    assert!(before.stdout.is_empty(), "{:?}", before.stdout());
    let after = ran(&dir, &line, &[("INBOX_URL", os(b"\xff"))]);
    assert_eq!(after.stderr(), named("INBOX_URL"));
    assert!(
        after.stdout.is_empty(),
        "a document though --json is the message id: {:?}",
        after.stdout()
    );
}
```
