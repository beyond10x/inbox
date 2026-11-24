---
format: aep.planning-md/3
id: review-result:adversary-key-id-control-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the key id with a control character
relations:
- reviews: story:a-key-id-with-a-control-character-is-refused-where-it-is-made
revision: 1
---
ADVERSARY PASS 1

story:a-key-id-with-a-control-character-is-refused-where-it-is-made, round 1, at 59c70b5. Every build and test ran on build-1d (each job's first line reads `slot: synced to build-1d …`). I found no red case. I have 3 findings, all introduced by this story: 1 warning and 2 notes.

My cases are in `crates/inbox-cli/tests/adversary_key_id_control.rs`, uncommitted in the adversary tree. I also added one `mod` line, in sorted order, to `crates/inbox-cli/tests/main.rs`. All 8 cases are green and each takes under 0.5 s:
- `a_key_file_with_an_odd_key_id_member_keeps_its_line`: 0.168 s
- `a_key_id_of_one_mib_is_one_line`: 0.267 s
- `a_key_id_with_json_escapes_is_refused_as_its_raw_form`: 0.319 s
- `a_refused_key_new_leaves_an_existing_file_alone`: 0.010 s
- `a_wrong_for_or_url_is_refused_before_the_key_file`: 0.162 s
- `key_new_and_the_client_agree_on_each_character_at_the_end_of_the_key_id`: 0.461 s
- `the_line_of_the_key_file_names_the_remedy_and_escapes_a_direction_character`: 0.163 s
- `wake_and_bridge_refuse_the_key_file_before_any_connection`: 0.175 s

Three of them kill mutants that the story's cases let live:
- `the_line_of_the_key_file_names_the_remedy_and_escapes_a_direction_character`
- `a_key_id_with_json_escapes_is_refused_as_its_raw_form`
- `key_new_and_the_client_agree_on_each_character_at_the_end_of_the_key_id`

Checks:
- Stress: the 4 modules `key_id_control`, `adversary_acting_for`, `adversary_bridge_own_key` and `adversary_key_id_control` ran with `--stress-count 5`. Result: "5/5 stress run iterations: 5 passed", 22 cases per run, 3.2 s.
- Clippy: `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0. The only warning is the old `unused variable: value` in the generated `wire.rs`.
- Format: `rustfmt --edition 2024 --check` reports no change for my file.

## Findings

1. **The CHANGELOG bullet is wrong for most control characters** (`CHANGELOG.md:106`, warning).
   - It says a client command with such a key file ended "before" with exit 2 and a line about a header value. That is true only for CR and LF.
   - At ab34af6, `request_head` refused only `\r` and `\n`. A key id with TAB, ESC, DEL or a C1 character was sent to the service. The implementor's mutant m3b confirms this: `c1.key` reached the stand-in and got 401.
   - `inbox keyfile add-key --key-id` does not check its id. So a service could hold such a key id, and a client key file with it worked before; now it ends with exit 4. The bullet does not mention this change.
   - Fix: the CHANGELOG bullet, and the matching clause in the story's outcome text.

2. **The line about the key file has no length bound** (`crates/inbox-cli/src/keys.rs:215`, note).
   - A key file whose key id is 1 MiB followed by U+0001 ends with exit 4. Standard error gets one line of 1,048,743 bytes: the whole key id in its escaped form, not cut, with no connection opened.
   - Case `a_key_id_of_one_mib_is_one_line` shows this; it asserts exit 4, one line and no connection, but not a length.
   - The `--key-id` line of `key new` has the same shape, but the argument length limit caps it.
   - Fix: cut the shown key id in `keys::caller`, or do not show it.

3. **The story's client case does not tell this refusal apart from other refusals of the same file** (`crates/inbox-cli/tests/adversary_acting_for.rs:287`, note).
   - `a_line_break_in_the_key_id_opens_no_connection` asserts only: exit 4, the file named, one line, no connection. Any other exit-4 line about the same file passes it too (a wrong mode, not a secret key file, the operator-key line).
   - Mutants o4 (remedy removed from the line) and o7 (cause "control character" removed from the line) survive all of the story's cases. My cases kill them.
   - Fix: that case also asserts "control character" and the remedy, or my cases are kept.

1 pre-existing finding is in the private file.

## Checked, no finding
- **Request head:**
  - Every client command builds its client through `ClientArgs::client`, which calls `keys::caller`. This includes `wake`, `wake --follow`, and `bridge` with and without `--for`/`--authority`: each exits 4 with its own prefix and opens no connection (case).
  - Targets come from `route` plus `query_value`. The header names are constants. The timestamp, signature and length are made by the client. `--for` is checked with `is_control`.
  - The implementor's paragraph holds for line breaks.
- **Key new and the client agree** on every C0 character except NUL (which cannot be passed as an argument), on DEL, C1, NBSP, U+2028, U+2029, U+202E, U+200B and U+FEFF, each as the last character of the key id. Each key id that `key new` takes reaches the stand-in exactly once.
- **JSON escapes:** `\u000a`, `\u0085`, `\u0000` and `\u001B` in the file are refused like the raw characters (exit 4). A surrogate pair passes.
- **Odd `key_id` members keep their old lines and exit 4:**
  - a number: "names no key id and principal"
  - two `key_id` members: "is there twice"
  - no `key_id`: "names no key id and principal"
  - a control character without a principal: the operator-key line
- **U+202E:** it appears in the line as `\u{202e}`. Both `{:?}` and `output::warn`'s `visible` escape it, so no raw direction character reaches the terminal.
- **`--json`:** stdout gets `{"error":"local","detail":…}` naming the file, and stderr still gets one line.
- **Refused `key new` and `--out`:** with an existing file, the key id refusal wins (exit 2, "no key is made") and the file is left untouched. A directory and a path in a missing directory also give exit 2.
- **Order of refusals:** a wrong `--for` or a wrong URL gives exit 2 before the key file is read.
- **The bridge case:** it lost only the ESC in the key id and in the expected line.
- **Documents:** `service.md`, `bridge.md`, `signing.md`, the `reference.rs` rows, `inbox.json` and the `AGENTS.md` row match the code. No website page became wrong.

## Mutants (all reverted; `git diff --stat` shows only the `main.rs` line, my case file is untracked)
- m3b, `is_ascii_control` in `caller` (implementor's): killed by `adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection` and 3 of my cases.
- m6, line without the file name (implementor's): killed by the same story case and 4 of my cases.
- m7a, key id written raw into the `key new` line (implementor's): killed by `key_id_control::key_new_refuses_…` and `a_refused_key_new_leaves_an_existing_file_alone`.
- o1, `caller` checks only when the principal is absent: killed by the story case and 6 of my cases.
- o2, `contains('\n')` in `caller`: killed by the story case and 5 of my cases.
- o3, `key new` checks the principal instead of the key id: killed by `key_id_control::key_new_refuses_…`, `adversary_bridge_own_key::odd_characters_in_the_ids_…` and 2 of my cases.
- o4, the line of the key file without the remedy: **the story's cases let it live**; killed only by `the_line_of_the_key_file_names_the_remedy_and_escapes_a_direction_character`.
- o5, `Failure::Usage` for the key file: killed by the story case and 5 of my cases.
- o6, `trim_end()` before the check in `caller`: killed by the story case (its `c1.key`) and 3 of my cases.
- o7, the line of the key file without the cause: **the story's cases let it live**; killed only by `a_key_id_with_json_escapes_is_refused_as_its_raw_form`.

## Not checked
- I did not prove against a real service that a key id with TAB or ESC held by the service was accepted before the story. Finding 1 rests on reading the code at ab34af6 and on the implementor's m3b log.
- `INBOX_URL` hosts that are not loopback literals were not probed, because that would resolve a name.
- `inbox keyfile add-key` and `add-delegation` read the operator key through `read_secret` and ignore its key id. `inbox serve` reads no client secret key file. I checked both by reading the code only, without running them.
- The implementor's mutants m2 and m5 were not rerun.

## Commands
Every build or test command went through the cargo shim to build-1d: nextest, clippy, and the cargo calls inside my mutant script. Commands that compile or test and ran without the `slot` prefix and outside the shim: none.
- The mutant script lies in the tree's git-ignored `.engineering/drafts/`. It was started without `slot` because it waits between builds.
- `rustfmt` ran on this host; it formats and does not compile.
- No refusal by slot and no permission refusal.

Logs are in the scratch directory: `run1.log`, `stress.log`, `mutants.log`, `clippy.log`.

```findings
[
  {"file": "CHANGELOG.md", "line": 106, "category": "docs", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The bullet says a client with such a key file ended with exit 2 before, which holds only for CR and LF; a key id with TAB, ESC, DEL or a C1 character was sent before (and worked when the service held it, since keyfile add-key checks no id) and now ends with exit 4, a change the bullet does not name."},
  {"file": "crates/inbox-cli/src/keys.rs", "line": 215, "category": "output", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The line of a key file whose key id holds a control character shows the whole key id, so a 1 MiB key id gives a 1,048,743-byte line on standard error; it is not cut."},
  {"file": "crates/inbox-cli/tests/adversary_acting_for.rs", "line": 287, "category": "test-gap", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The story's client case asserts only exit 4, the file named and one line, so mutants that drop the remedy (o4) or the cause (o7) from the new line survive every story case and are killed only by the adversary cases."}
]
```
