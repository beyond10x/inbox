---
format: aep.planning-md/3
id: review-result:adversary-delegation-acts-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on an agent that acts under a delegation: 4 findings, 5 cases added, 3 red'
relations:
- reviews: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
revision: 1
---
ADVERSARY PASS 1

I found 4 findings, all in the client's `--for`, and nothing wrong in the service. I ran 8 mutants and the story's cases caught all 8. My 5 cases are in `<tree>/crates/inbox-cli/tests/adversary_acting_for.rs` (uncommitted, with one sorted `mod` line in `crates/inbox-cli/tests/main.rs`). 3 are red and 2 are green. I added no service case file, because no mutant of the service survived.

Times of my cases, from one run:
- `for_given_twice…`: 0.097 s, green
- `the_header_on_the_wire_is_the_flag_byte_for_byte`: 0.112 s, green
- `a_control_character…`: 0.173 s, red
- `a_line_break…`: 0.178 s, red
- `a_value_the_service_trims…`: 0.303 s, red

Each case runs `pending`, `post` and `answer` with `--for <value>` against a stand-in on loopback that answers 401 and keeps each request head exactly as received.

Finding 1 (warning). A line break in `--for` puts an extra header line into the request. Red case: `adversary_acting_for::a_line_break_in_for_is_wrong_usage_and_sends_nothing`.
- Values tried: the person's address followed by `\r\nX-Injected: 1`, by `\nX-Injected: 1`, and by `\rX`.
- For all 9 runs the client sent a request and exited 1, not 2. The `\r\n` head ends in `Inbox-On-Behalf-Of: agent:test/bob\r\nX-Injected: 1`.
- So the client signs one text, and the service reads a shorter header plus a header of the caller's choosing.
- The service never honours such a request (signature mismatch, 401), so no privilege is gained. Still, the client puts bytes on the wire that differ from the bytes it signs.
- Fix: `ClientArgs::client` in `crates/inbox-cli/src/main.rs:268` refuses CR and LF with exit 2 (or `request_headers_for` refuses them), and the sentence in `docs/design/service.md` that says "nothing else of the value is checked" changes with it.

Finding 2 (note). A control character in `--for` is sent as given. Red case: `adversary_acting_for::a_control_character_in_for_is_wrong_usage_and_sends_nothing`.
- Values tried: the address followed by U+0001, by ESC `[2J`, and by U+007F. Each was sent, exit 1.
- Decision 2 of the client round says nothing but empty and padded values is checked. This attack brief asks for exit 2. The coordinator decides which holds. The fix would go in the same place as finding 1.

Finding 3 (note). The service trims more white space than the client and the design name. Red case: `adversary_acting_for::a_value_the_service_trims_is_wrong_usage_and_sends_nothing`.
- The service's header reader (`generated/rust/inbox/src/server/http.rs:241`, `value.trim()`) trims every Unicode white space, not only a space and a tab.
- So a value that ends or starts with U+00A0, U+3000, VT or FF is signed with that character and read without it. The service never honours it, which is exactly why the client refuses a space or a tab, yet the client sends it (exit 1, not 2).
- `docs/design/signing.md:206-207` says "a leading or trailing space or tab: the reader trims the header". The reader trims more than that.
- Fix: the client's check (`crates/inbox-cli/src/main.rs:268`) uses `char::is_whitespace` at both ends, and the sentence in `signing.md` and `service.md` names it.

Finding 4 (note). A help sentence this story made incomplete still stands. The `--on-behalf-of` help of `inbox answer` (`crates/inbox-cli/src/main.rs:521`, and `generated/cli/inbox.json:917`) still says "the caller by default". Under `--for` the default is the `--for` address (`commands.rs:804`, and the `--for` help itself). Fix: that help line, then `slot task cli-reference`.

0 pre-existing findings are in the private file (none found; no file written).

Checked and holding:
- The two green cases: `--for` given twice exits 2 and sends nothing. The header on the wire equals the flag byte for byte: a composed and a decomposed letter, upper case, `%20` and a colon, nothing folded or encoded.
- Every command that takes `--for` routes through `Client::exchange`, so the header is on every request of the command, including the repost of the reconnect.
- `open`, `close`, `wake` and `bridge` refuse `--for`. `serve`, `key`, `keyfile` and `tls` send no signed request.
- The reconnect under `--for` opens nothing and leaves the open record at its old epoch.
- Collect, outbox and post stay keyed by the signer's principal, so nothing of the person's inbox is stored under the agent's address.
- The close rule:
  - it runs inside `backend.exclusive`;
  - a duplicate member is 400 before it;
  - a body over the bound is 413 before authentication;
  - a body that is not an object, or has a member in another letter case, fails the same decoder the route uses;
  - the route path is matched exactly.
  The 404/403 difference tells nobody anything new, since the `Inboxes` view lists every inbox to every client.
- Answer under the header: `authority-invalid` is always asked, so `--on-behalf-of <agent>` without `--authority` is refused.
- The merge: the bridge cases are green (610 of 610 in the three crates). No conflict marks and no doubled sentence in `AGENTS.md`, `CHANGELOG.md`, `limits.md`, `status.md` or the design pages. The one repeated CHANGELOG line is already twice on `main`.
- Stress: `acting_for` with `--stress-count 5`: 5/5 iterations, 7 of 7 passed each time.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings` exits 0. Its one warning is in the generated `wire.rs`, not in my file.

Mutants (each reverted; at the end `git diff --stat` shows only `tests/main.rs`, and my new file is untracked). Each line names the story's cases that went red:
- m1, header sent but member not signed (the round-2 implementor's list): caught by `acting_for::an_agent_reads…`, `without_a_live_delegation…` and `under_for_the_reconnect…`.
- m2, member signed but header not sent (the round-2 implementor's list): caught by `acting_for::with_for_every_request…` and 3 more.
- m3, default `on_behalf_of` under `--for` dropped (the round-2 implementor's list): caught by `acting_for::an_agent_reads…`.
- m4, the reconnect opens under `--for` (the round-2 implementor's list): caught by `acting_for::under_for_the_reconnect…`.
- m5, the close rule compares with the signer's address (mine): caught by `delegation::under_the_header_the_agent_does_not_close_its_own_inbox` and `delegation::an_inbox_the_agent_opens_for_the_person_is_closed_only_by_the_agent`.
- m6, `--for` ignored on `refuse` (mine): caught by `acting_for::with_for_every_request…` and `without_a_live_delegation…`.
- m7, `own_inbox` ignores `--for`, so the `--inbox` default is wrong (mine): caught by `acting_for::an_agent_reads…`.
- m8, the header sent twice is read as the first (mine): caught by `delegation::the_header_twice_is_401`.

Not reached:
- No timing measurement of `inbox serve` between an uncovered inbox and a bad signature. Both run the ed25519 verify; the uncovered one adds only a scan of the delegations.
- No mutant of the reconnect's repost without the header (it would need a second client path).
- No `--for` over the header bound: Linux caps one argument at 128 KiB, below the service's head bound of about 1 MiB, so I did not run it.
- No end-to-end run of post under `--for` followed by a reconnect without it.

Build commands run without the `slot` prefix: none. All builds were cargo calls through the shim, and their first lines name build-1d. The scripts `mutants.sh` and `stress.sh` in `<scratch>/s4/deleg/adversary1/` started without the prefix and contain only shim cargo calls. `rustfmt` and `perl` (for the mutant edits) ran locally and compile nothing.

```findings
[
  {"file": "crates/inbox-cli/src/main.rs", "line": 268, "category": "security", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "A CR or LF in --for is sent, so the request head gains a header line of the caller's choosing and the wire differs from the signed member, where it should be exit 2 before any request (adversary_acting_for::a_line_break_in_for_is_wrong_usage_and_sends_nothing)."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 268, "category": "input-validation", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "A control character in --for (U+0001, ESC, U+007F) is sent and ends in 401 exit 1 rather than exit 2 before any request; decision 2 of the client round chose this, and the attack brief asks for exit 2 (adversary_acting_for::a_control_character_in_for_is_wrong_usage_and_sends_nothing)."},
  {"file": "docs/design/signing.md", "line": 206, "category": "input-validation", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The service's header reader trims every Unicode white space (str::trim), not only a space and a tab, so a --for value ending in U+00A0, U+3000, VT or FF is signed with it and read without it, is never honoured, and the client still sends it (adversary_acting_for::a_value_the_service_trims_is_wrong_usage_and_sends_nothing)."},
  {"file": "crates/inbox-cli/src/main.rs", "line": 521, "category": "docs", "severity": "note", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The --on-behalf-of help of inbox answer (and generated/cli/inbox.json:917) still says the caller by default, while under --for the default is the --for address."}
]
```
