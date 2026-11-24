---
format: aep.planning-md/3
id: review-result:adversary-delegation-acts-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on an agent that acts under a delegation: 4 notes, 4 cases added, all green'
relations:
- reviews: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
revision: 1
---
ADVERSARY PASS 2

Story: an agent with a delegation acts for the inbox of its principal. Attacked de575b5 in <tree> (the adversary tree), detached. Every job ran on build-1d. The tree's only change is crates/inbox-cli/tests/adversary_acting_for.rs, which is uncommitted (+123 lines, no pass-1 case changed). Logs and the two scripts are in <scratch>.

Result: no flaw in the product code of round 3. Three of the four mutants survived every story and pass-1 case. The new cases kill them. There is also one note about the key id.

Findings

1. The check of `--for` can move after the URL, the key and the outbox are read, and nothing catches it.
   - What I did: mutant A ran `acting_for_address` after `Service::from_url`, `keys::caller` and `Outbox::locate` in `ClientArgs::client`. With no URL, a missing key file or no outbox, HOME or state directory, a bad value then gets another line or exit 4, not the line that names `--for`.
   - Without the new case: mutant A survived every case of the story and of pass 1.
   - New case: `a_wrong_for_is_named_before_the_url_the_key_and_the_outbox` kills it. It also checks that no file is written and no request is sent.
   - Fix lands in: the story's cases (crates/inbox-cli/src/main.rs:267). The code is right.

2. Narrowing `is_control` to ASCII is not caught.
   - What I did: mutant B replaced `char::is_control` with `is_ascii_control`, so C1 controls inside the value (U+009B, the one-byte CSI; U+0085; U+009D) pass the check.
   - Without the new case: mutant B survived, because the pass-1 cases use only C0 and DEL.
   - New case: `a_c1_control_inside_for_is_wrong_usage_and_sends_nothing` kills it.
   - Fix lands in: the story's cases (crates/inbox-cli/src/main.rs:312).

3. Building the head after the connection is opened is not caught.
   - What I did: mutant D called `self.connect()` before `request_head`. The unit case calls `request_head` directly, so it cannot see the order.
   - The path a user can reach: `inbox key new` accepts a key id that holds CR LF (exit 0), and the client writes that key id as `Inbox-Key-Id`.
   - Without the new case: mutant D survived.
   - New case: `a_line_break_in_the_key_id_opens_no_connection` kills it. The stand-in saw an empty connection under the mutant and none under the real code.
   - Fix lands in: the story's cases (crates/inbox-cli/src/client.rs:169).

4. Note: a key file whose key id holds a line break now fails every client command with exit 2 and the line "a header value holds a line break; no request is sent".
   - The line does not name the key file. Exit 2 means wrong usage, while other key-file defects in this client exit 4.
   - Before round 3 the client sent such a head and injected the line on the wire. That was the caller's own file, so it was no boundary.
   - `inbox key new` still makes such a key without complaint.
   - Fix lands in: crates/inbox-cli/src/client.rs:238 (name the part's source), or a key id check in `inbox key new`. Low priority.

Checked and holds
- Item 1, the check of the value:
  - `str::trim` in the service's reader and `char::is_whitespace` both use the Unicode White_Space property. Neither has a character the other lacks.
  - U+FEFF and U+200B are neither trimmed nor refused, so they reach the wire as signed.
  - U+0085 is refused anywhere in the value (it is a control character). U+2028 and U+2029 are refused at either end and allowed inside; the reader splits lines only on LF, so the wire stays the signed text.
  - Non-UTF-8 is refused by clap (earlier cases).
  - The refusal line is always one line. It shows no raw U+202E, U+2028, U+2029, ESC, U+009B, U+0085, U+FEFF, U+200B or BEL; checked by `the_refusal_line_shows_no_raw_character_of_the_value`.
- Item 2, the head of a request:
  - Every request target is a fixed route or uses `query_value` (percent-encoded).
  - A host with CR or LF does not resolve, so `from_url` refuses it with exit 2.
  - The key id is the only header text from a file that can hold a line break; see findings 3 and 4.
  - No legal request of the build before is now refused.
- Item 3: the crate has 391 cases and all pass (7.9 s). None of the story or pass-1 cases was changed by round 3 beyond the diff shown.
- Item 4, the sentences: the help of `--for` and `--on-behalf-of`, generated/cli/inbox.json, docs/design/signing.md, docs/design/service.md and the AGENTS.md row match the code. website/docs/limits.md and CHANGELOG.md say nothing about the check, which is consistent.

Mutants
- A, the check after `Outbox::locate` (and after the URL and key): survived the story's and pass-1 cases. Killed by `a_wrong_for_is_named_before_the_url_the_key_and_the_outbox`.
- B, `is_control` narrowed to ASCII: survived. Killed by `a_c1_control_inside_for_is_wrong_usage_and_sends_nothing`.
- C, the head check only for header values: caught by the story's unit case `client::tests::the_head_holds_no_line_break_a_caller_passes`.
- D, the head made after the connection: survived. Killed by `a_line_break_in_the_key_id_opens_no_connection`.
All four were reverted. `git diff --stat` shows only the cases file.

Runs
- Times of the 4 new cases: 0.012 s, 0.029 s, 0.035 s, 0.223 s (all under 5 s).
- The `acting_for` and `adversary_acting_for` cases with `--stress-count 5`: 5 of 5 runs passed.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` passes. The one warning left is the old unused `value` in generated/rust/inbox/src/server/wire.rs:343, outside this crate.
- `rustfmt --edition 2024 --check` passes on the file.

0 pre-existing findings; no private file was written.

Not checked:
- the TLS path of the head order: mutant D was run on plain HTTP only;
- `bridge` and `wake` under a broken key id;
- whether `post` keeps an envelope in the outbox before the head is refused (finding 4);
- the whole gate, which runs in the story's tree.

Build commands run without the `slot` prefix: `cargo nextest` and `cargo clippy` through the shim (each job named build-1d in its first line), and `rustfmt` on one file. No other.

```findings
[
  {"file": "crates/inbox-cli/src/main.rs", "line": 267, "category": "test-gap", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No case of the story or of pass 1 fails when the check of --for runs after the URL, the key file and the outbox are read (mutant A); the pass-2 case a_wrong_for_is_named_before_the_url_the_key_and_the_outbox kills it."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 312, "category": "test-gap", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No case of the story or of pass 1 fails when char::is_control is narrowed to ASCII, so a C1 control such as U+009B inside --for would pass (mutant B); the pass-2 case a_c1_control_inside_for_is_wrong_usage_and_sends_nothing kills it."},
  {"file": "crates/inbox-cli/src/client.rs", "line": 169, "category": "test-gap", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No case of the story or of pass 1 fails when the request head is made after the connection is opened (mutant D), although a key id with CR LF reaches it; the pass-2 case a_line_break_in_the_key_id_opens_no_connection kills it."},
  {"file": "crates/inbox-cli/src/client.rs", "line": 238, "category": "diagnostics", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A key file whose key id holds a line break, which inbox key new makes without complaint, now fails every client command with exit 2 and a line that names a header value but not the key file, while other key-file defects in this client exit 4."}
]
```
