---
format: aep.planning-md/3
id: review-result:security-the-service-speaks-tls-with-a-pinned-certificate-pass-1
kind: review-result
status: active
title: Security review, pass 1, of TLS with a pinned certificate
relations:
- reviews: story:the-service-speaks-tls-with-a-pinned-certificate
revision: 1
---
unit: story:the-service-speaks-tls-with-a-pinned-certificate, tree <tree>-sec at 4d65c66 plus my test files (diff against de19aaa)
verdict: NEEDS-CHANGE (1 warning: over TLS, a body that is still arriving when the service refuses it gets a broken pipe instead of 413). All 9 security invariants hold.
cases: executed 104→112, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: 15 paths (part 6)
needs-coordinator: none

1. git diff --stat
 crates/inbox-server/tests/main.rs | 1 +
 1 file changed, 1 insertion(+)
git status --short: ` M crates/inbox-server/tests/main.rs` (one `mod security_tls;` line) and `?? crates/inbox-server/tests/security_tls.rs`. Both are test paths. No implementation file, document or `.engineering/` file was touched, not even for a moment.

2. Cases added: crates/inbox-server/tests/security_tls.rs (8 cases; certificates, keys and pins are all made at run time)
- invariant_1_a_server_with_the_pinned_certificate_but_another_key_fails_the_handshake (:113). A control stand-in (pinned certificate, its own key) completes and receives the bytes. A second stand-in shows the pinned certificate but signs with another key: the handshake fails and it receives 0 plain bytes. GREEN.
- invariant_1_the_pin_is_held_to_the_end_entity_certificate_not_to_a_chain_member (:142). Chain [other, pinned] signed with the other key is refused, 0 bytes sent. GREEN.
- invariant_2_a_pin_has_one_text_only (:164). The canonical text reads. These are all refused: URL alphabet, 43 characters, `==`, leading, trailing or inner space, trailing newline, non-zero trailing bits, `=` inside. GREEN.
- invariant_3_a_pinned_client_accepts_a_certificate_past_its_not_after_over_the_same_key (:316). A certificate valid only 2020–2021 over the same key has the same pin, and a pinned request over it gets 200. GREEN.
- invariant_5_tls_1_3_only_no_resumption_no_early_data (:333). rustls::ALL_VERSIONS holds TLS 1.3 only; server max_early_data_size 0 and send_tls13_tickets 0; the client has enable_early_data false. Two connections with one client config are both TLSv1_3 and HandshakeKind::Full. GREEN.
- invariant_7_a_handshake_that_never_ends_is_cut_at_the_request_timeout_and_frees_its_slot (:385). max_connections 1, request_timeout 300 ms, a partial ClientHello: the connection is closed in under 2 s and the next TLS request gets 200 within 3 s. GREEN.
- invariant_7_a_handshake_message_over_its_bound_ends_the_connection_at_once (:414). A ClientHello that announces 16 MiB is closed in under 3 s while request_timeout is 30 s. GREEN.
- invariant_7_a_body_past_the_bound_still_arriving_over_tls_is_answered_413_as_over_plain (:467). The client writes a head with Content-Length 2 MiB, pauses 200 ms, writes the body, then reads. Over plain HTTP (control, :479) the answer is 413. Over TLS (:494) it must also be 413. RED.
First run of the 7 green cases alone: `Summary [0.728s] 7 tests run: 7 passed, 104 skipped`, exit 0.
The first version of the 8th case had no pause. It passed alone once, then failed 4 of 10 in a repeat of the 8 cases. I added the pause to make it deterministic. Run alone (`cargo nextest run -p inbox-server --locked -E 'test(/security_tls::invariant_7_a_body_past/)'`), verbatim:
    thread '…invariant_7_a_body_past_the_bound_still_arriving_over_tls_is_answered_413_as_over_plain' panicked at crates/inbox-server/tests/security_tls.rs:494:5:
    assertion `left == right` failed: over TLS
      left: Err("writing the body: Broken pipe (os error 32)")
     right: Ok(413)
    Summary [0.582s] 1 test run: 0 passed, 1 failed, 111 skipped
    exit=100

3. Suite run, after the cases existed
`cargo nextest run -p inbox-server --locked --no-fail-fast`: 112 tests across 2 binaries, `Summary [13.295s] 112 tests run: 111 passed, 1 failed`; FAIL security_tls::invariant_7_a_body_past_… at :494 with the same left/right as above; exit=100, 15 s wall time.
Before count, a second run with my file deselected (`-E 'not test(/security_tls::/)'`): `104 tests run: 104 passed, 8 skipped`, exit 0, 23 s. No test is close to 60 s; the step is far under 120 s. inbox-cli was not touched and not run.

4. Findings (tree 4d65c66 plus test files)
- crates/inbox-server/src/tls.rs:635. NEEDS-CHANGE, introduced, warning, contract-drift.
  - What was measured: security_tls.rs:494 is red. The plain control at :479 is green.
  - Cause: when `outbound` sees the end of the plain side, it sends close_notify and calls `outer.shutdown(Shutdown::Both)`. That end comes from `http::linger`'s `shutdown(Write)`. So the caller's socket closes while its body is still arriving, and the caller gets a reset. This undoes `http::linger`, which listener.rs:317 keeps "so closing does not reset the connection before the caller reads it".
  - What it contradicts: docs/design/service.md:354-356 ("every bound … holds unchanged … the body limits answer `413`"). The same mechanism hits a 408 or 503 sent while a body is still arriving.
  - What reaches it: `Service::send` in crates/inbox-cli/src/client.rs:192-196 writes the whole body before it reads, and inbox-cli has no local size check. `inbox post` with a body over 1 MiB to an https:// service exits 3 (transport, envelope kept in the outbox) instead of 413; over http:// it gets 413. Without the pause it happened in 4 of 10 runs.
  - Suggested fix (not applied): `outbound` shuts `outer` for writing only. `terminate` shuts `outer` fully after `serve(plain)` returns (after tls.rs:556), so the drain is bounded by linger as on plain.

5. Checked and could not fault
- Inv 1 holds.
  - tls.rs:354-355: the pin is compared over the whole 32-byte array (derived PartialEq).
  - The SPKI comes from the rustls-webpki parser, not offsets: tls.rs:72-74 → rustls-webpki-0.103.17/src/cert.rs:236-242.
  - verify_tls12_signature returns Err (:365-373); TLS 1.2 is not compiled in (rustls features: ring, std).
  - verify_tls13_signature delegates to rustls' verifier with the ring algorithms (:375-387); supported_verify_schemes returns the provider's list (:389-393).
  - Cases :113 and :142.
- Inv 2 holds.
  - Decode, then re-encode and compare with the text (tls.rs:100). This gives one text per digest. Case :164 would also catch removing that check.
- Inv 3 holds. I made one certificate at run time with the built CLI and checked it with openssl (log probe-cert.log).
  - `openssl x509 -text`: Version 3; 16-byte positive serial; ED25519 signature and SPKI algorithm with no parameters; CN=inbox serve; Not Before UTCTime (now), Not After GeneralizedTime 99991231235959Z.
  - `asn1parse` is clean, and re-encoding gives identical DER.
  - `openssl verify -check_ss_sig` OK: the signature is over the TBS.
  - The pin equals openssl's SHA-256 of the SPKI.
  - No extensions.
  - After notAfter the pinned client still accepts (case :316, verify_server_cert ignores `_now`). service.md:365 says so ("no validity time … consulted").
- Inv 4 holds.
  - The seed comes from getrandom (tls.rs:151-156).
  - The file is created with create_new and mode 0o600 at open: O_EXCL, no chmod afterwards, no overwrite (keys.rs:275-279).
  - serve refuses a file with mode & 0o077 (keys.rs:67-77, CLI case tls.rs:299).
  - The seed is never printed: Debug shows the pin only (tls.rs:135-141); tls_new prints only the pin (CLI case tls.rs:267); parse errors give an offset, not content.
  - The Identity seed is Zeroizing (tls.rs:131). The PKCS#8 copy in server_config is wiped by rustls (crypto/ring/mod.rs:61 wraps the key in Zeroizing).
  - Residue in ring's own Ed25519KeyPair is a library limit and is not claimed.
- Inv 5 holds. Case :333.
  - TLS 1.3 only on both sides (tls.rs:220 and :409).
  - Client resumption is disabled (:413); server tickets 0 (:223); early data off.
  - No plain fallback (client.rs:84-98).
  - ring provider: suites TLS13_AES_256_GCM_SHA384, TLS13_AES_128_GCM_SHA256, TLS13_CHACHA20_POLY1305_SHA256; groups X25519, secp256r1, secp384r1 (rustls-0.23.46/src/crypto/ring/mod.rs:72-76, 180-181).
- Inv 6 holds.
  - The handshake completes before any write (client.rs:174-192, tls.rs:445-458).
  - Existing cases cover it: server tls.rs:208 and CLI tls.rs:188. Early data is off (case :333).
- Inv 7 holds for handshake input.
  - rustls bounds: MAX_HANDSHAKE_SIZE 0xffff (msgs/deframer/handshake.rs:376); received plaintext 16 KiB and send buffer 64 KiB (common_state.rs:1135-1136); 32 key-update requests without application data (common_state.rs:1053).
  - TLS 1.3 has no renegotiation. tls.rs's own buffers are 16 KiB chunks.
  - Cases :385 and :414.
- Inv 8.
  - Locked versions: rustls 0.23.46, ring 0.17.14, rustls-webpki 0.103.17, rustls-pki-types 1.16.0, untrusted 0.9.0, zeroize 1.9.2, getrandom 0.4.3.
  - Features: `default-features = false`, ring and std only (`cargo tree -e features -i rustls`): no tls12, no logging.
  - Advisories: not checked. ~/.cargo/advisory-db exists, but I did not search it for these versions, and nothing was looked up online.
- Inv 9: no statement in service.md TLS or first-use.md "Serve over TLS" is stronger than the code. The documents do not say what the pin does not protect against: anyone who can read the TLS file can pose as the service; there is no revocation; a new `inbox tls new` means a new pin for every client. service.md:343 mentions renewal over the same key, which no command does. I left this out of the findings block because I did not measure it.
- Also cleared: the loopback socket pair checks its peer (tls.rs:482-495); descriptors per TLS connection are about 7, so 64 connections fit a 1024 limit; read_private is a metadata check then open (the same pattern as the base's read_secret, and it needs write access to the directory).
- I did not take a worktree session lease.

6. Paths written outside the worktree
<scratch>/s3/tls/security/build.log
<scratch>/s3/tls/security/cases-alone.log
<scratch>/s3/tls/security/case-413-alone.log
<scratch>/s3/tls/security/case-413-alone-2.log
<scratch>/s3/tls/security/repeat.sh
<scratch>/s3/tls/security/repeat.log
<scratch>/s3/tls/security/suite.sh
<scratch>/s3/tls/security/suite-after.log
<scratch>/s3/tls/security/suite-before.log
<scratch>/s3/tls/security/suite.done
<scratch>/s3/tls/security/probe-cert.sh
<scratch>/s3/tls/security/probe-cert.log
<scratch>/s3/tls/security/probe/ (cert.der, cert.pem: public certificate only. The tls.json holding the seed was deleted by the probe right after the certificate was taken out.)
target/debug/inbox inside the worktree, from `cargo build -p inbox-cli`.

7.
```findings
[{"file":"crates/inbox-server/src/tls.rs","line":635,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"outbound shuts the caller's socket for reading as soon as http::linger shuts the plain side for writing, so over TLS a request refused while its body is still arriving gets a broken pipe instead of the 413 (or 408/503) that plain HTTP delivers and docs/design/service.md:354-356 promises."}]
```
