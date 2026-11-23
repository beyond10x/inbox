---
format: aep.planning-md/3
id: review-result:adversary-the-service-speaks-tls-with-a-pinned-certificate-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on the TLS story, round 2: 7 findings, 3 red cases'
relations:
- reviews: story:the-service-speaks-tls-with-a-pinned-certificate
revision: 1
---
unit: story:the-service-speaks-tls-with-a-pinned-certificate, tree <tree> at 5c598a3 plus my test files (round 2 = e205b34..5c598a3)
verdict: NEEDS-CHANGE
cases: executed 286→289, red 3
origin: introduced 6 / pre-existing 0 / undecided 1
wrote-outside-worktree: 17 paths, all under <scratch>/s3/tls/adversary2/
needs-coordinator: the killing cases for the 2 surviving mutants (MC, MD) are unit tests of the private `terminate`, so they belong in `crates/inbox-server/src/tls.rs`, an implementation file. They are in scratch (killer_unit.rs) for the implementor to add.

Short version: I found no hang of `terminate`. Over TLS every phase I drove ends: the threads stop, the descriptors close and the slot comes back. 3 cases are red. (1) When an answer is cut short, the service still sends `close_notify` behind it, so a TLS client reads a short answer as complete. Over plain HTTP the same caller gets a reset. (2) service.md now says a caller still sending its body reads the 408 or 503 of a cut. It does not: it gets a broken pipe, over TLS and over plain HTTP alike. (3) The CLI 413 case written for S1 cannot fail. Mutants MC and MD survive the unit's suite.

1. Diff stat (`git --no-pager diff --stat`)
 crates/inbox-server/tests/main.rs | 1 +
Untracked: crates/inbox-server/tests/adversary_tls2.rs. `git status --short` shows only these 2 paths, both test files. No implementation file, document or `.engineering/` file was touched, not even for a moment. Every mutant ran in a copy in scratch.

2. Cases added: crates/inbox-server/tests/adversary_tls2.rs. All 3 are red now, and were red on their first run (each under 2 s).
- adv2_tls_a_408_cut_while_the_body_still_arrives_is_read_by_the_caller: `request_timeout` 400 ms, a signed head announcing 900,000 bytes, then the body in 8 KiB pieces with a 10 ms pause after each, then a read. It asserts Ok(408), as service.md:353-356 says.
- adv2_tls_a_503_cut_while_the_body_still_arrives_is_read_by_the_caller: the same with `request_timeout` 1 h and a stop after 400 ms. It asserts Ok(503).
- adv2_tls_an_answer_cut_short_is_never_ended_with_close_notify: 10 tries of GET /openapi.json (239,720 bytes) followed by 200,000 bytes the service never reads. It asserts that no try ends with `close_notify` and a body shorter than Content-Length.
Run alone: `cargo nextest run -p inbox-server --locked --no-fail-fast -E 'test(/adversary_tls2::/)'`, verbatim:
    panicked at crates/inbox-server/tests/adversary_tls2.rs:122:5: assertion `left == right` failed
      left: Err("writing the body: Broken pipe (os error 32) after 188416 of its bytes")
     right: Ok(503)
    panicked at crates/inbox-server/tests/adversary_tls2.rs:104:5: assertion `left == right` failed
      left: Err("writing the body: Broken pipe (os error 32) after 262144 of its bytes")
     right: Ok(408)
    panicked at crates/inbox-server/tests/adversary_tls2.rs:202:5:
    an answer cut short ended with close_notify: [CleanButShort { declared: 239720, got: 160768 }, CleanButShort { declared: 239720, got: 126976 }, Whole, CleanButShort { declared: 239720, got: 126976 }, CleanButShort { declared: 239720, got: 126976 }, CleanButShort { declared: 239720, got: 126976 }, CleanButShort { declared: 239720, got: 126976 }, CleanButShort { declared: 239720, got: 193536 }, CleanButShort { declared: 239720, got: 128000 }, CleanButShort { declared: 239720, got: 126976 }]
    Summary [   1.181s] 3 tests run: 0 passed, 3 failed, 118 skipped
    exit 100
Checks: `rustfmt --edition 2024 --check` passes. `cargo clippy -p inbox-server --all-targets --locked -- -D warnings` exits 0; its only warning is the existing one in the generated `inbox` crate.

3. Suite run, after the cases existed
`cargo nextest run -p inbox-server -p inbox-cli --locked --no-fail-fast` (in the tree):
    FAIL adversary_tls2::adv2_tls_a_408_cut_while_the_body_still_arrives_is_read_by_the_caller
    FAIL adversary_tls2::adv2_tls_a_503_cut_while_the_body_still_arrives_is_read_by_the_caller
    FAIL adversary_tls2::adv2_tls_an_answer_cut_short_is_never_ended_with_close_notify
    Summary [  45.619s] 289 tests run: 286 passed, 3 failed, 0 skipped
    exit 100
Where 286 comes from: the same command with `-E 'not test(/adversary_tls2::/)'` (my 3 cases deselected): `Summary [  60.600s] 286 tests run: 286 passed, 3 skipped`, exit 0. The 60.6 s run overlapped with my mutant runs on the pool. 286 = 118 + 168, which matches docs/checks.md.

4. Findings (tree 5c598a3 plus my test files)

F1 crates/inbox-server/src/tls.rs:646. NEEDS-CHANGE, introduced, warning.
- What was measured: adversary_tls2.rs:202 is red; 9 of 10 cut answers ended cleanly.
- Cause: `outbound` calls `send_close_notify` even when `failed`. Here the service closed its end of the pair with the caller's extra bytes unread, which resets the pair, and the rest of the answer is lost. The caller then sees a short answer behind a clean `close_notify`.
- Over plain HTTP the same exchange ends with ConnectionReset. Scratch probe J, extra 100,000 bytes: plain 7/10 cut, every one a reset; TLS 8/10 cut, every one ending Ok.
- What reaches it: a TLS client that sends bytes after its request (pipelining, or a body longer than its Content-Length) and gets an answer larger than what is in flight. Nothing in the repository: `inbox` sends one exact request, and a cut answer fails its JSON read (exit 3).
- Fix: on `failed`, shut `outer` without `close_notify`, so the caller sees an unclean end.

F2 docs/design/service.md:355. NEEDS-CHANGE, introduced (the sentence is new in round 2), warning.
- What was measured: adversary_tls2.rs:104 and :122 are red.
- Over plain HTTP it is the same: scratch probe BC got a broken pipe in all 4 plain variants. So "as over plain HTTP" holds, but "the caller reads the answer (… the 408 or 503 of a cut)" holds for neither.
- Cause: the cut shuts the socket for reading. After `http::linger`'s `shutdown(Write)` the socket has both directions shut, and the kernel resets the caller's next bytes. A scratch socket probe shows a peer of a socket shut both ways gets BrokenPipe.
- What reaches it: an `inbox post` still sending at `request_timeout` (10 s) or at SIGTERM. That is rare.
- Fix: in the document, say that a caller still sending its body when a cut answers gets a reset, over TLS as over plain.

F3 crates/inbox-cli/tests/tls.rs:81. CONFIRMED, introduced, note. A test that cannot fail.
- `a_post_over_the_request_limit_gets_the_services_413_over_https_as_over_http` passed under mutant MA (round 1's shutdown put back): PASS [3.297s].
- `--max-request-bytes` sets `max_body`, and `max_body` is checked after the reader has read the whole body (service.rs:70-72). So `linger` never runs, and the comment's "the client is still sending the request when the service answers" is false.
- `inbox` cannot send a body past MAX_BODY: a body is at most 65,536 characters, about 393 KB.
- S1 stays guarded only by security_tls.rs:497.

F4 crates/inbox-server/src/tls.rs:569. CONFIRMED, introduced, note.
- Mutant MD (no join of `inbound`) survives the whole suite of both packages.
- In production the plain end closes when `serve` returns, so `inbound` ends a moment later. The design's "counts the connection until both threads have ended" is asserted by nothing for that thread.
- Killing case: scratch `killer_md_terminate_waits_for_the_thread_that_reads_the_caller`. It is green on the real code (9.8 s). Under MD it is red: "terminate returned while the thread that reads the caller was still writing into the pair".

F5 crates/inbox-server/src/tls.rs:650. CONFIRMED, introduced, note.
- Mutant MC (`outbound` never shuts the plain end on failure) survives the whole suite.
- What reaches it: a caller that fills the pair with extra bytes, then resets while a large answer is written. The service's write then waits for its 10 s WRITE_TIMEOUT and holds the slot.
- Killing case: scratch `killer_mc_a_caller_gone_mid_answer_fails_the_services_write`. On the real code the service's write ended with ConnectionReset 50 ms after the reset; the case passes in 34 s, 32 s of which is the caller filling the pair. Under MC it was ended by the runner: TIMEOUT [60.063s].

F6 CHANGELOG.md:27. CONFIRMED, introduced, note. "New dependencies of inbox-server: rustls … and ring 0.17" leaves out `zeroize = "1.8"`, a new direct dependency (crates/inbox-server/Cargo.toml:22; absent at de19aaa).

F7 docs/design/service.md:317. CONFIRMED, undecided, note.
- Over plain HTTP, a stop with a caller that never reads a 7.8 MB answer took 31.1 s, where the document says "10 s for a stalled write". The bound applies to each write call, not to the answer.
- Measured once in scratch probe G. Its log was overwritten by the TLS run, which ended at the 60 s runner bound while it was still posting.
- Not run at de19aaa.

Mutants (scratch copy, both packages; baseline 288 passed = 286 + the 2 scratch killers):
- MA, round 1's shutdown put back in `outbound` (outer and inner shut Both): caught by security_tls::invariant_7_a_body_past_the_bound_… ("left: Err("writing the body: Broken pipe (os error 32)") right: Ok(413)"), and by scratch killer_md. Not caught by cli tls::a_post_over_the_request_limit_… (F3).
- MB, no full shutdown at the end of `terminate`: caught by tls::tests::terminate_returns_only_when_…, adversary_tls::adv_tls_threads_and_descriptors_… and adversary_tls::adv_tls_a_caller_stalled_…. Also hung until the runner's 60 s bound: tls::stop_answers_…_503, adversary_tls::adv_tls_a_wait_over_tls_ends_on_a_post and cli adv_tls_sigterm_….
- MC, survived the unit's suite:
  `-    if failed {`
  `+    if false && failed {`
  `         let _ = inner.shutdown(Shutdown::Both);`
- MD, survived the unit's suite:
  `     let _ = outer.shutdown(Shutdown::Both);`
  `-    let _ = inbound.join();`
  `+    drop(inbound);`
- MM1, `if from == expected` changed to `if true`: caught by tls::tests::an_intruder_that_connects_to_the_pair_first_… ("left: 127.0.0.1:52150 right: 127.0.0.1:52166").
- MM9, no join of `outbound`: caught by tls::tests::terminate_returns_only_when_… (through its `read_to_end` unwrap, UnexpectedEof at tls.rs:692, not its own assertion) and 18 other cases.

5. Attacked and could not break
- A 413 for a body past MAX_BODY, 2 to 12 MiB, with and without a pause: TLS behaves as plain HTTP. Both answer 413 up to 4 MiB and break somewhere from 5 to 8 MiB.
- A caller that sends for ever after a refused (413) or a whole request: it is cut with a broken pipe within 0.2 s. A second request on `max_connections` 1 is served in 18 to 60 ms, the TLS threads end, the descriptors return to their baseline, and the stop takes about 1 ms.
- The caller closes first: a TCP half close without `close_notify` in the middle of the body gets 400 with `close_notify`. Closed in the middle of the body, or after a whole request without reading: 0 TLS threads, descriptors at baseline, stop under 1 ms.
- A well-behaved client: `close_notify` comes before the end of every whole answer (probe I with 0, 1 or 1000 extra bytes, and probe J with 1000: all whole and Ok).
- Cases that lost their upper bound: none became unable to fail. The 408 case needs the cut to get 408; the 503 case needs the stop's cut. The 2 security HANG cases could never tell the request timeout from the reader's 1 s gap bound, not even with the old 2 s and 3 s bounds.
- The first-use page, run as written in 2 shells with the pin copied: open, post, pending and read each exit 0; serve exits 0 on SIGTERM. The literal pin placeholder gives exit 2, as a placeholder URL would.
- AGENTS.md:85, the exit texts in reference.rs, and generated `inbox.json` against the code: they agree.
Not checked:
- The stop with a caller that never reads a large answer over TLS (my probe ran out of time while posting; pass 1 measured about 10 s of slot holding).
- Post-handshake messages (KeyUpdate) after the answer.
- A client that is not rustls.
- F7 at the base.
- No worktree lease was taken.

6. Paths written outside the worktree, all under <scratch>/s3/tls/adversary2/:
- mut/: a copy of the tree at 5c598a3 plus probe2.rs and its mod line. Its target (552 MB) is deleted.
- mutants/: one diff and one log per mutant, base.log, summary.txt, tls.rs.ref.
- probes/: probe_a, b_c, d_e, g, h, i and j logs, summary.txt.
- Scripts: mutants.sh, probes.sh, waitfor.sh, first-use.sh, shutrd_probe.py (a scratch probe only, never committed).
- Killing cases: killer_unit.rs, killer_unit.instr.rs.
- Logs: build.log, cases-alone.log, clippy.log, suite.log, suite-before.log, first-use.log (pins redacted), mci.log.
- The key files of the first-use run (firstuse/) were deleted by the script.
- My test and clippy builds wrote to the worktree's own target/.

```findings
[
{"file":"crates/inbox-server/src/tls.rs","line":646,"category":"boundary","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"outbound sends close_notify after its read of the pair failed, so an answer cut short (the service closed its end with the caller's extra bytes unread) reaches a TLS caller as a complete stream, where plain HTTP gives a reset."},
{"file":"docs/design/service.md","line":355,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"The TLS section says a caller still sending its body reads the 408 or 503 of a cut, but it gets a broken pipe over TLS, as over plain HTTP, because the cut shut the socket for reading and linger then shuts it for writing."},
{"file":"crates/inbox-cli/tests/tls.rs","line":81,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The CLI 413 case passes with round 1's shutdown put back, because --max-request-bytes is checked after the whole body is read, so the client is never still sending when the 413 is written."},
{"file":"crates/inbox-server/src/tls.rs","line":569,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Removing the join of the inbound thread passes the whole suite; only the scratch case killer_md shows that terminate returns while that thread still writes into the pair."},
{"file":"crates/inbox-server/src/tls.rs","line":650,"category":"mutant","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"Making outbound never shut the plain end on failure passes the whole suite; the scratch case killer_mc shows the service's write then waits for its write timeout instead of failing at once."},
{"file":"CHANGELOG.md","line":27,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The Added entry names rustls and ring as the new dependencies of inbox-server and leaves out zeroize 1.8, also new in its Cargo.toml."},
{"file":"docs/design/service.md","line":317,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"undecided","message":"Over plain HTTP a caller that never reads a 7.8 MB answer held the stop for 31 s, while the limits table names 10 s for a stalled write: the bound applies to each write call."}
]
```
