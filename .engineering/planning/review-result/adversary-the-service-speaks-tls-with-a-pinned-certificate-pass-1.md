---
format: aep.planning-md/3
id: review-result:adversary-the-service-speaks-tls-with-a-pinned-certificate-pass-1
kind: review-result
status: active
title: Adversary pass 1 on TLS with a pinned certificate
relations:
- reviews: story:the-service-speaks-tls-with-a-pinned-certificate
revision: 1
---
unit: story:the-service-speaks-tls-with-a-pinned-certificate, tree <tree> at 4d65c66 (diff against de19aaa)
verdict: NEEDS-CHANGE
cases: executed 267→275, red 0
origin: introduced 8 / pre-existing 1 / undecided 0
wrote-outside-worktree: 21 paths, all under <scratch>/s3/tls/adversary/
needs-coordinator: 3 killing cases (green on the real code, red under their mutants) are in scratch. Adding them to the unit's tests is the implementor's job. Finding 2 needs a test seam in tls.rs.

Short version: I found no behaviour defect, so no case is red. The code holds every bound I attacked over TLS. The suite has 4 gaps: 4 mutants survive the whole suite of both packages. The first-use page, followed as written, leaves the client terminal without the pin.

1. Diff stat (`git --no-pager diff --stat`)
 crates/inbox-cli/tests/main.rs    | 1 +
 crates/inbox-server/tests/main.rs | 1 +
Untracked: crates/inbox-server/tests/adversary_tls.rs and crates/inbox-cli/tests/adversary_tls.rs. Every path is a test file. No implementation file was changed, not even for a moment: every mutant ran in a copy in scratch (mut/).

2. Cases added (all green now and green when first run; none red)
crates/inbox-server/tests/adversary_tls.rs:
- adv_tls_threads_and_descriptors_end_with_every_kind_of_connection: 6 kinds of caller (TCP only, half a ClientHello, handshake then silence, answer never read, closed at once, plain garbage). It counts threads named inbox-tls-* in /proc/self/task (at least 4 while running, 0 after the request timeout) and checks /proc/self/fd is back at its baseline.
- adv_tls_a_caller_stalled_in_the_handshake_gives_its_slot_back_at_the_request_timeout (max_connections 1).
- adv_tls_a_request_sent_a_byte_per_record_is_served.
- adv_tls_a_wait_over_tls_ends_on_a_post.
crates/inbox-cli/tests/adversary_tls.rs:
- adv_tls_a_post_refused_by_the_pin_is_kept_and_reposted_with_the_right_pin: exit 3, stderr "does not match the pin", 1 outbox line, then `repost` with the right pin arrives.
- adv_tls_the_pin_flag_wins_over_the_environment: also a pin with a space or a newline, `HTTPS://` and an empty pin each give exit 2.
- adv_tls_sigterm_ends_the_service_while_callers_stall_in_and_after_the_handshake: under 5 s.
- adv_tls_wait_over_tls_returns_on_a_post.
Times: the longest is 2.5 s, none over 5 s.

3. Suite run, after the cases existed
Command: `cargo nextest run -p inbox-server -p inbox-cli --locked --no-fail-fast` (in the tree)
     Summary [  54.186s] 275 tests run: 275 passed, 0 skipped
exit 0
Where 267 comes from: the same 2 packages in the scratch copy, run after my server file existed with `-E 'not test(/adversary_tls/)'` (my 6 server cases of then deselected; the CLI file did not exist yet). Result: 267 passed, 6 skipped.

4. Findings (all cover 4d65c66)

F1 crates/inbox-server/src/tls.rs:381. Mutant M2 survives the whole suite: the pinned client stops checking the handshake signature.
- What reaches it: anyone who copies the public certificate could then pose as the service, if this line ever regresses.
- Killing case: scratch killer_signature.rs. A server shows the pinned certificate and signs the handshake with another key. Green on the real code. Red under M2:
  "the client completed a handshake signed by a key that is not the pinned certificate's"
- Fix: add that case to crates/inbox-server/tests/tls.rs.

F2 crates/inbox-server/src/tls.rs:489. Mutant M1 survives: `terminate` stops checking the end it accepted.
- `pair()` is private and the window between bind and connect is microseconds, so no integration test can put a third process at the pair's listener first. I wrote no case (INFEASIBLE from outside).
- What reaches it: on code reading the check holds. A foreign connection is accepted and dropped, and no request byte reaches it.
- Fix: a seam in tls.rs, for example `pair_on(listener)`, plus a unit test whose intruder connects before the near end.

F3 crates/inbox-server/src/tls.rs:557. Mutant M9 survives: the 2 threads are not joined.
- With M9, `max_connections` stops counting a TLS connection until both threads end, which `docs/design/service.md` (TLS) states.
- Killing case: scratch killer_slot.rs. A caller never reads a pending answer of about 3.9 MB, and a second caller must wait for the one slot. Green on the real code (10.6 s, over the 5 s mark: fewer posts would shorten it). Red under M9:
  "a second connection was served while the first still held its answer: Ok("HTTP/1.1 200 OK…")"
- Fix: add that case to the unit's tests/tls.rs.

F4 crates/inbox-server/src/tls.rs:100. Mutant M13 survives: the canonical check of `Pin::parse` is never exercised.
- The generated decoder ignores the 2 unused bits of character 43, so a non-canonical text decodes to the same 32 bytes. Harmless to security; the documented "canonical" is untested.
- Killing case: scratch killer_pin.rs. Green on the real code. Red under M13:
  "a pin with non-zero unused bits is taken as canonical"

F5 website/docs/first-use.md:67. I ran first-use as written in 2 shells: keys, `TLS_PIN=$(inbox tls new …)` and `inbox serve --tls-file` in the first; in the second ("In a second terminal"), the export block and "One message" with the printed URL.
- Output: serve printed `https://127.0.0.1:42499`. `inbox open`, `post` and `pending` each failed with exit 2: "INBOX_TLS_PIN: a pin is the SHA-256 of a certificate's public key: 44 characters of canonical base64".
- Cause: `$TLS_PIN` exists only in the shell where serve runs in the foreground. And if a reader does run "One message" in the shell that holds the pin, its literal `export INBOX_URL=http://…` beside the exported pin gives exit 2 as well.
- Fix: the "Serve over TLS" section tells the reader to copy the printed pin into the client terminal, and "One message" says what to change for TLS.
- Script: scratch first-use.sh.

F6 website/docs/first-use.md:110. The exit status table still has the old texts for 2 and 3. `reference.rs` and AGENTS.md now name the pin cases (an `https://` URL without a pin, a pin beside `http://`, a certificate the pin does not name). The story's scope allowed only the new section of this page.

F7 crates/inbox-cli/src/reference.rs:13. DESCRIPTION, and so generated/cli/inbox.json, says `inbox serve` runs the service "over TLS with the certificate `inbox tls new` makes". TLS is optional and plain HTTP is still the default.

F8 crates/inbox-server/tests/tls.rs:298. `a_tls_request_not_arrived_within_the_request_timeout_is_408` asserts under 900 ms with a 300 ms timeout, and the bound includes the handshake. Measured 0.403 s at a load average of about 140, so 500 ms of slack on a host the brief puts at 100 to 150.

F9 crates/inbox-server/src/listener.rs:308 (pre-existing, plain HTTP only). A caller that sends bytes after its whole request during `pending?wait=1` gets its whole answer (148 bytes) and then a reset, so read_to_end fails: "plain: Err(Os { code: 104, kind: ConnectionReset … }) after 148 bytes".
- Reproduced at the base de19aaa in a scratch archive.
- Over TLS the same caller gets its answer cleanly.
- What reaches it: a pipelining client; `inbox` sends none.
- Case: scratch pre_existing_plain.rs.

Mutants (in the scratch copy, the whole suite of both packages, baseline 267 passed):
- M1 accepted end unchecked (tls.rs:489): survived. `-        if from == expected {` / `+        if true {`
- M2 handshake signature unchecked (tls.rs:381): survived. `-        verify_tls13_signature(message, cert, dss, &self.provider.signature_verification_algorithms,)` / `+        { let _ = (message, cert, dss); Ok(HandshakeSignatureValid::assertion()) }`
- M3 connect skips the handshake (the brief's "request written before the handshake completed"): caught by tls::a_client_given_another_pin_ends_the_handshake_and_posts_nothing, tls::a_client_given_another_pin_sends_no_byte_of_its_request and cli tls::a_client_with_the_pin_posts_over_tls_and_one_with_another_pin_exits_3. rustls holds plaintext until the handshake completes, so even M3 sends no request byte.
- M4 any certificate accepted: caught by the same 3 cases.
- M5 a pin beside http:// sent plain: caught by client::tests::an_https_url_needs_a_pin_and_a_pin_needs_an_https_url and tls::a_client_given_a_pin_sends_nothing_to_an_http_url.
- M6 the request timeout does not reach TLS: caught by tls::a_tls_request_not_arrived_within_the_request_timeout_is_408 and tls::a_tls_connection_beyond_the_limit_waits_until_one_ends.
- M7 `inbox tls new` overwrites: caught by tls::tls_new_writes_a_0600_file_never_overwrites_it_and_prints_only_the_pin.
- M8 the TLS file read at mode 0644: caught by tls::serve_refuses_a_tls_file_others_can_read_or_that_is_no_tls_file.
- M9 threads not joined (tls.rs:556-558): survived. `     serve(plain);` / `-    let _ = outbound.join();` `-    let _ = inbound.join();` / `+    drop((outbound, inbound));`
- M11 no close_notify: caught by 8 cases.
- M12 https:// without a pin sent plain: caught by tls::an_https_url_without_a_pin_is_wrong_usage and 2 unit tests.
- M13 pin canonical check dropped (tls.rs:100): survived. `-        if pin.to_base64() != text {` / `+        if false && pin.to_base64() != text {`
The killing cases above turn M2, M9 and M13 red, each one on its own mutant only.

5. Attacked and could not break
- Request timeout over TLS for a caller sending nothing over TCP, one with half a hello, and one silent after the handshake: each is cut, and its threads and descriptors end.
- A stalled handshake frees the one slot at the request timeout.
- A body over the reader's bound still arriving (1.2, 2 and 3 MB) is answered 413 over TLS exactly as over plain HTTP.
- A request sent one byte per TLS record is served.
- close_notify right after the request (half close) is answered 200.
- A pending wait over TLS ends on a post (library and CLI).
- SIGTERM with stalled callers ends in under 5 s.
- Wrong pin: exit 3, the text on standard error, the outbox keeps the envelope, and a later `repost` with the right pin arrives.
- The flag wins over the environment. A pin with spaces, `HTTPS://` and an empty pin each give exit 2.
- Socket pair: checked on code reading only (F2).
Not checked:
- A TLS 1.2-only server. rustls is built without tls12 and I tried no openssl stand-in.
- A redirect or `101` from a TLS stand-in, and a close without close_notify. The CLI tests have no rustls dependency for such a stand-in. On code reading, a 3xx gives exit 3, and rustls reports an unclean end as UnexpectedEof.
- The extra ephemeral port each TLS connection's pair takes: my TIME_WAIT probe was too noisy on this shared host, so I report nothing on it.
- The cryptographic side, which the other reviewer covers.
- No worktree lease was taken.

6. Paths written outside the worktree, all under <scratch>/s3/tls/adversary/:
- mut/: copy of the tree, plus mut/target (520 MB)
- base/: archive of de19aaa, plus base/target (166 MB)
- mutants/: one log and diff per mutant, summary.txt
- killers/: logs, summary.txt
- scripts: mutants.sh, killers.sh, first-use.sh
- cases: killer_signature.rs, killer_slot.rs, killer_pin.rs, pre_existing_plain.rs, pre_existing_bytes_after_request.rs
- logs: mutants.out, killers.out, base-run.log, probe_tw.log, run1.log to run4.log, suite.log
Key files from the first-use run were deleted, test leftovers under both target/tmp were deleted, and pin-like strings in 2 mutant logs were redacted.

```findings
[
{"file":"crates/inbox-server/src/tls.rs","line":381,"category":"mutant","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"No case shows a server holding the pinned certificate without its key, so a client that skips the handshake signature check passes the whole suite."},
{"file":"crates/inbox-server/src/tls.rs","line":489,"category":"mutant","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The check that the accepted end of the loopback pair comes from the end this process connected has no case, and none can reach the private pair() without a test seam."},
{"file":"crates/inbox-server/src/tls.rs","line":557,"category":"mutant","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Nothing tests that max_connections counts a TLS connection until both threads have ended: with the joins removed the whole suite passes."},
{"file":"crates/inbox-server/src/tls.rs","line":100,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The canonical check of Pin::parse is never exercised: a pin with non-zero unused bits in character 43 is the input that would show it."},
{"file":"website/docs/first-use.md","line":67,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Run as written, the client terminal never receives the pin (TLS_PIN stays in the shell running inbox serve), so every client command exits 2."},
{"file":"website/docs/first-use.md","line":110,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The exit status table still gives the old texts for 2 and 3 and names none of the pin cases, which the CLI reference and AGENTS.md now name."},
{"file":"crates/inbox-cli/src/reference.rs","line":13,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The CLI reference description reads as if inbox serve always speaks TLS, while plain HTTP stays the default without --tls-file."},
{"file":"crates/inbox-server/tests/tls.rs","line":298,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The 408 case holds a 900 ms wall-clock bound that includes the handshake; measured 0.403 s at a load average of about 140, about 500 ms of slack."},
{"file":"crates/inbox-server/src/listener.rs","line":308,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"Over plain HTTP, a caller that sends bytes after its whole request gets a reset after its answer, so read_to_end fails; reproduced at de19aaa, not over TLS."}
]
```
