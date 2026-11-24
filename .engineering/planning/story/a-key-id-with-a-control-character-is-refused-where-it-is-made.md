---
format: aep.planning-md/3
id: story:a-key-id-with-a-control-character-is-refused-where-it-is-made
kind: story
status: implemented
title: A key id with a control character is refused where it is made
summary: inbox key new writes a key id with a line break; every client command then ends with exit 2 and a line that names no key file.
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/inbox-cli/src/bridge.rs
- confidence: cited
  path: crates/inbox-cli/src/client.rs
- confidence: cited
  path: crates/inbox-cli/src/keys.rs
- confidence: cited
  path: crates/inbox-cli/src/main.rs
- confidence: cited
  path: crates/inbox-cli/src/output.rs
- confidence: cited
  path: crates/inbox-cli/src/reference.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_acting_for.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_bridge_own_key.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_key_id_control.rs
- confidence: cited
  path: crates/inbox-cli/tests/key_id_control.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: cited
  path: docs/design/bridge.md
- confidence: cited
  path: docs/design/service.md
- confidence: cited
  path: docs/design/signing.md
- confidence: cited
  path: generated/cli/inbox.json
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T11:58:43Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T11:58:43Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T12:42:53Z", actor: "agent:inbox", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

`inbox key new` refuses a key id that holds a control character, and a client command that
reads a secret key file whose `key_id` holds one ends as for every other defect of that file:
exit 4 and one line that names the file.

Today `inbox key new` writes such a file without complaint (exit 0). Since
story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal the client makes no
request head with a line break in any of its parts, so every command with a key file whose
key id holds a line break ends with exit 2 and the line "a header value holds a line break;
no request is sent", which names neither the key file nor the key id (note 4 of
review-result:adversary-delegation-acts-pass-2). Before that story the client sent the head
with the line break in it. A key id with another control character (a tab, another C0
character, DEL, a C1 character) is sent today, and a service whose key file holds the same
key id takes the request (corrected after
review-result:adversary-key-id-control-pass-1: the first text of this section said exit 2
for every control character).

## Acceptance

- `inbox key new` with a key id that holds a line break or another control character exits 2
  with one line that names the flag, and writes no file (state transition: a key id with a
  control character moves `inbox key new` from exit 0 to exit 2).
- A client command given a secret key file whose `key_id` holds a control character exits 4
  with one line that names the file, sends no request and opens no connection.
- The case of the adversary that holds the order of the request head and the connection
  (`adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection`) uses such a key
  file today; it is given another part of a head that can hold a line break, or the story
  says why no such part is left and what holds the order then.
- Each new case takes under 5 s; `task check` exits 0.

## Context

A note of an adversary pass, filed as a draft. The key file is the caller's own, so no
boundary is crossed. It waits until the planned waves are delivered.

## Scope

Written after the build, from the diff against `main` at ab34af6: every path below is read
from it (cited). A scope report before the build named the same places in `crates/inbox-cli`;
it placed the sections of `CHANGELOG.md` in the wrong release, and it did not name the host
of `INBOX_URL`, which round 2 added.

Code, crate `crates/inbox-cli` only:

- `src/keys.rs`: `new` refuses a key id with a control character before a key is made;
  `caller` refuses a secret key file whose `key_id` holds one.
- `src/client.rs`: `host_holds_no_control`, called by `Service::from_url` before the host is
  handed to the resolver.
- `src/output.rs`: `quoted_cut`, the form a refusal shows a value in.
- `src/main.rs`: the help of `--key-id`. `src/reference.rs`: the rows of the exit statuses 2
  and 4. `src/bridge.rs`: the doc comment of `refused_key`.
- `tests/key_id_control.rs` (the cases of the story), `tests/adversary_key_id_control.rs`
  (the cases of the adversary pass), 2 `mod` lines in `tests/main.rs`.
- `tests/adversary_acting_for.rs` and `tests/adversary_bridge_own_key.rs`: one changed case
  each.

Documents and generated files:

- `docs/design/service.md`, `docs/design/signing.md`, `docs/design/bridge.md`.
- `AGENTS.md`: the Layout row of `crates/inbox-cli/`.
- `CHANGELOG.md`: one bullet under Changed, 2 under Fixed.
- `generated/cli/inbox.json`, written by `task cli-reference`.

Not touched: `ess/`, `generated/rust/`, the service (`crates/inbox-server`), the key file of
the service (`inbox keyfile add-key`, `add-delegation`), every page under `website/`.

## Decisions

- The story was a draft of the backlog (a note of an adversary pass that was there before
  its wave). The operator's session accepted it with 2 others as one wave, one pull request
  each, no release from them alone (2026-11-24). No panel of plan critics read it; one
  adversary pass reviewed the build, and no pass read round 2.
- What a control character is. The options were: (A) `char::is_control` (C0, DEL, C1), the
  test the client uses for `--for`; (B) also U+2028, U+2029 and the direction characters. A
  is taken: one rule for both header values; the others still pass `inbox key new` and are
  shown escaped where a line names them.
- The check of the key file sits where the key id becomes the caller's (`keys::caller`), not
  in the reader the operator key shares. `request_head` keeps its refusal of a line break as
  the last guard.
- The order of the refusals of a client command stays: a wrong `--for` (exit 2), a wrong
  `INBOX_URL` (exit 2), then the secret key file (exit 4).
- The named case `adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection`.
  The options were: (A) keep it, with key files the case writes (a key made at run time, its
  `key_id` member replaced), exit 4, the cause and the remedy in the line, no connection;
  (B) only a unit case of `request_head`. A is taken: it holds the order of the refusal and
  the connection at the binary.
- No other part of a request head can carry a control character from a client command. The
  method and the target come from the generated route table (`client::route`). The only
  caller text in a target is a query value, which `client::query_value` percent-encodes
  (every byte except `A-Za-z0-9-._~`); `wait` is a number and `undelivered=1` is fixed. The
  header names are constants and `Content-Length` is a number. The timestamp is formatted by
  the client from its own clock, and the signature is base64 made by the client. The value of
  `Inbox-On-Behalf-Of` is `--for`, refused with exit 2 when it holds a control character
  (`acting_for_address`). The key id is refused with exit 4 where it becomes the caller's
  (`keys::caller`), and `inbox key new` does not write one (exit 2). The value of `Host` is
  the host and port of `INBOX_URL` (or `--url`): `Service::from_url` refuses it with exit 2
  when it holds a control character, through `client::host_holds_no_control`, a check of the
  text alone, before the host is handed to the resolver. The port could not carry one into
  the resolver in any case, since it must read as a number before any lookup, and nothing
  but an optional `/` follows it. The order is held by these cases:
  `adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection`;
  `client::tests::a_host_with_a_control_character_is_wrong_usage`;
  `key_id_control::a_host_of_the_url_with_a_control_character_is_wrong_usage`;
  `client::tests::the_head_holds_no_line_break_a_caller_passes`;
  `adversary_acting_for::a_line_break_in_for_is_wrong_usage_and_sends_nothing`.
- A key file that was sent before now ends with exit 4. A key id with a tab, another C0
  character, DEL or a C1 character was sent before this story, and a service whose key file
  held the same id took the request (`inbox keyfile add-key` checks no id). The options
  were: (A) refuse every control character in the key file, as the acceptance says, and name
  the change under Changed in `CHANGELOG.md` with the remedy (a new key); (B) refuse only a
  line break in the key file and keep the others sent. A is taken: the acceptance names "a
  control character", and one rule holds for the place a key id is made and the place it is
  read. The release that carries this story names it as a breaking change.
- A line that refuses a value shows at most its first 64 characters, cut before they are
  escaped, and says how many more there are (`output::quoted_cut`). The options were: (A)
  cut; (B) do not show the value. A is taken: the first characters name the key.
- The host of `INBOX_URL`. The adversary pass found, outside this story's text, that a host
  with a control character other than a line break was handed to the resolver and, when it
  resolved to a loopback address, written into the header `Host`. The options were: (A) fix
  it in this story, since the story says no request head carries a control character; (B)
  keep it private and unfiled until a story of its own. A is taken, and the fix ships with
  this record. No case resolves a name: the unit case reads the text alone, and the case at
  the binary names a key file that is missing, which the refusal comes before.
- Not in this story, said in `docs/design/signing.md`: the `principal` of a key, the key id
  of `inbox keyfile add-key`, the delegation id of `inbox keyfile add-delegation`, an empty
  key id, a key id with white space around it. None of them reaches a request head as a
  header value. The older refusals of a URL still show the whole URL.
- One existing case lost a character: `adversary_bridge_own_key::odd_characters_in_the_key_id_keep_the_refused_end_one_line`
  made a key id with ESC through `inbox key new`; it keeps U+2028 and U+202E.
- The coordinator changed one doc comment in the adversary's file after round 2
  (`a_key_id_of_one_mib_is_one_line` said the line is not cut). No assertion of that file
  changed.

## Measured

- Red first, round 1, before the product change (build node): `4 tests run: 2 passed, 2
  failed`: `key_id_control::key_new_refuses_a_key_id_with_a_control_character_and_writes_no_file`
  (status 0, expected 2) and `adversary_acting_for::a_line_break_in_the_key_id_opens_no_connection`
  (status 2 with "a header value holds a line break; no request is sent", expected 4).
- The lines as the binary writes them (build node), exits 2, 4 and 2:
  `inbox: usage: --key-id "k\r\nX-Injected: 1" holds a control character, which no request
  can carry; no key is made`; `inbox: c.key: its key id "k\u{1b}[2J" holds a control
  character, which no request can carry; make a client key with` and the command; `inbox:
  usage: INBOX_URL: its host "in\tbox.invalid:7700" holds a control character, which no
  request can carry; no request is sent`. A key id of 101 characters shows its first 64 and
  `(and 37 more characters)`.
- Round 1: the cases of `inbox-cli` 434 → 436, 2 runs in a row `436 tests run: 436 passed, 0
  skipped`; 9 mutants, each red in a named case (the check of `key new` removed; moved after
  the file is written; `is_ascii_control` in `key new` and in `caller`; the check of the key
  file removed; moved after the connection; the line without the file name; the key id raw
  in each of the 2 lines).
- Adversary pass 1 (review-result:adversary-key-id-control-pass-1): 3 findings (1 warning, 2
  notes), 8 cases, all green, each under 0.5 s. It checked: `wake`, `wake --follow` and
  `bridge` refuse the key file before any connection; `key new` and the client agree on
  every C0 character but NUL, on DEL, C1, NBSP, U+2028, U+2029, U+202E, U+200B and U+FEFF
  at the end of the key id; JSON escapes in the file are refused like the raw characters;
  a refused `key new` leaves an existing file alone. 2 mutants lived through the story's
  cases (the line without its remedy, the line without its cause) and were red in its cases.
  It did not check: a real service with such a key id; a host of `INBOX_URL` that is no
  loopback literal.
- Round 2 on those findings (all 3 recorded as fixed) and on the host of `INBOX_URL`: red
  before the change, `14 tests run: 11 passed, 3 failed` (a line of 1,048,744 bytes for a key
  id of 1 MiB; of 100,100 bytes from `key new`; the whole key id of 65 characters shown);
  the unit case of the host against a stub: `11 tests run: 10 passed, 1 failed`. After it
  the cases of `inbox-cli` 444 → 450, 2 runs in a row `450 tests run: 450 passed, 0 skipped`.
  7 mutants, each red in a named case: the cut at 63; at 65; the escape before the cut; the
  count plus 1; `is_ascii_control` for the host; only CR and LF for the host; the remedy
  removed from the line of the key file (red in the named case of the story). The mutant
  "the call of the host check removed from `from_url`" was not run: it would hand a name to
  the resolver.
- The cases: 6 in `key_id_control`, 8 of the adversary in `adversary_key_id_control`, 2 unit
  cases (`client`, `output`), 2 existing cases changed. No case over 5 s: the slowest of the
  new ones took 0.46 s.
- Not held by a case: that the resolver is not reached for a host with a control character
  rests on the order of the lines in `Service::from_url` and on the unit case. No adversary
  pass read round 2.
- The gate runs of the rounds on the build node (`task check`): round 1 1129 passed, 8
  skipped, the test step 19 s, exit 0; round 2 1143 passed, 8 skipped, 18 s, exit 0.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T12:41:44Z to 2026-11-24T12:42:38Z, on the build
  node of the pool: exit 0, `1143 tests run: 1143 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
