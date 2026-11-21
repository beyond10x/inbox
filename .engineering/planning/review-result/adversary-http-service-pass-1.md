---
format: aep.planning-md/3
id: review-result:adversary-http-service-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:http-service (df9d07b): 8 findings, 6 red cases'
relations:
- reviews: story:http-service
revision: 1
---
8 findings

```
unit: story:http-service, df9d07b on impl/http-service (base c582901), plus one uncommitted adversary test file
verdict: CONFIRMED
cases: executed 21→30, red 6
origin: introduced 5 / pre-existing 3 / undecided 0
wrote-outside-worktree: 1 directory (the brief's scratch directory, `adversary/`)
needs-coordinator: decide whether a caller must be the envelope's `from` (F6); decide whether the slow-client hold (F1/F2), which the note lists as left out, holds the unit
```

## 1. Diff stat

`git --no-pager diff --stat` shows nothing: nothing tracked changed. `git status --short` shows one untracked file: `?? crates/inbox-server/tests/adversary.rs`. It is a test file, and no implementation path was touched.

## 2. Cases added (`crates/inbox-server/tests/adversary.rs`), each run alone before the suite

`CARGO_INCREMENTAL=0 cargo test -p inbox-server --test adversary`: 3 passed, 6 failed, exit 101.

- `adv_a_slow_unauthenticated_connection_does_not_hold_a_signed_request` (:144), red. With `max_connections = 1`, an unsigned connection sends one header byte every 300 ms, and a signed GET is not served within 4 s. Output: `panicked at crates/inbox-server/tests/adversary.rs:161:24: a signed request is served within 4 s while one unauthenticated connection sends a byte every 300 ms: Timeout`
- `adv_stop_returns_while_a_slow_unauthenticated_connection_is_open` (:169), red. Output: `panicked at ...adversary.rs:184:22: Running::stop returns within 5 s while one unauthenticated connection sends a byte every 300 ms: Timeout`
- `adv_a_wait_with_an_unbounded_max_wait_is_answered_not_dropped` (:193), red. With `max_wait = Duration::MAX` and `wait=18446744073709551615`, a row is already there, but the connection thread panics and the connection closes without an answer. Output: `thread 'inbox-connection' panicked at crates/inbox-server/src/service.rs:156:20: overflow when adding duration to instant`
- `adv_an_unauthenticated_body_over_max_body_is_413_before_401` (:216), green on the tree, red on mutant L (below).
- `adv_a_post_on_behalf_of_another_with_an_unknown_authority_is_authority_invalid` (:228), green, red on mutant J.
- `adv_a_post_on_behalf_of_another_under_its_delegation_is_posted` (:242), green, red on mutant K.
- `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` (:267), red. The second principal reads the first one's note, which is handled and removed. The second principal, signed as itself, then posts the first one's envelope bytes again, and they are accepted. Output: `assertion left != right failed: a caller authenticated as agent:host-b/session-two posted an envelope from agent:host-a/session-one: {"outcome":"posted",...} left: 202 right: 202`
- `adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message` (:300), red. Output: `existing: {"outcome":"not-recipient",...} / missing: {"outcome":"unknown-message",...} left: (403, "not-recipient") right: (404, "unknown-message")`
- `adv_a_third_principal_cannot_take_another_senders_dedup_key` (:331), red. A third principal posts into its own inbox under a dedup key, and another sender's later post under the same text is refused. Output: `{"outcome":"duplicate-key","published":[],"error":"inbox.messaging.Duplicate"} left: "duplicate-key" right: "posted"`

## 3. Suite run, after the cases existed

- `CARGO_INCREMENTAL=0 cargo test -p inbox-server --no-fail-fast` exited 101: lib 4 ok, acceptance 1 ok, cases 16 ok, adversary 3 passed and 6 failed (the 6 above).
- `cargo test -p inbox-server --lib --test cases --test acceptance` (adversary excluded) exited 0, with 21 cases.

Mutants, one at a time, in a scratch copy of the tree, running the unit's own 21 cases:

- J, `authorizes` always true (auth.rs:115): unit suite green; adversary case :228 red: `left: "posted" right: "authority-invalid"`
- K, `check_delegation` with `from` and `on_behalf_of` swapped: unit suite green; adversary case :242 red: `left: 502 right: 202`
- L, the 413 check moved after authentication (service.rs:67): unit suite green; adversary case :216 red: `left: 401 right: 413`
- `seen` read after the view in `waited` (service.rs:158): unit suite green; no deterministic case (a race).

The unit suite caught every one of these: dropped request signature, window, duplicate check, read rule for `to`, read rule for `from`, message count `>=` changed to `>`, loopback check, `max_body` check, and the cut to `max_wait`.

## 4. Findings

- F1, crates/inbox-server/src/listener.rs:116, CONFIRMED, introduced. One unsigned connection sending a byte every 300 ms holds a connection slot indefinitely. 64 of them (the default) stop every signed client. Case :144. What reaches it: any local process, unauthenticated, on loopback. `READ_TIMEOUT` bounds only the gap between bytes. service.md lists "a bound on the whole time a request may take to arrive" as left out, so the note does not promise otherwise.
- F2, crates/inbox-server/src/listener.rs:137, CONFIRMED, introduced. `Running::stop` and `Server::run` do not return while such a connection is open: `wait_idle` waits for it without end. Case :169. The note's "lets every request in flight finish" has no bound.
- F3, crates/inbox-server/src/service.rs:156, INFEASIBLE, introduced. `Instant::now() + wait` overflows when `max_wait` is near `Duration::MAX` and `wait` is huge. The connection thread panics and the caller gets no answer. `signals.rs:88` has the same shape for `sweep_interval` (the expiry thread would die). Case :193. Nothing reaches it yet: it needs an embedding that sets `max_wait` near 2^63 s. Fix: `checked_add`, saturating.
- F4, crates/inbox-server/src/auth.rs:115, CONFIRMED, introduced. No case in the unit's suite exercises `RingVerifier::authorizes`: mutants J and K pass all 21. Cases :228 and :242 close the gap if kept.
- F5, crates/inbox-server/src/service.rs:67, CONFIRMED, introduced. service.md's step 2 before step 3 (413 before 401) is not pinned: mutant L passes all 21. Case :216 pins it.
- F6, crates/inbox-server/src/service.rs:84, CONFIRMED, pre-existing. The HTTP caller is never tied to the envelope's `from`. A recipient can re-post a sender's handled envelope (`to` is unchanged, so it can only replay into its own inbox). It reappears in the sender's outbox, and for a decision with `default_on_silence` it gives the sender a fresh decided answer. Case :267. What reaches it: every recipient; Pending hands it the signed envelope. The specification's PostMessage has no `from == caller.principal` guard, and this crate passes that on.
- F7, ess/domains/messaging.yaml:473, CONFIRMED, pre-existing. A third principal tells "message exists between others" (403 `not-recipient`) from "no message" (404 `unknown-message`) through `read`. `collect`, `refuse`, `answer` and `mark-delivered` have the same order. Case :300. What reaches it: any client that knows or guesses a message id.
- F8, ess/domains/messaging.yaml:422, CONFIRMED, pre-existing. The dedup key is global across principals. A third principal learns whether a key exists and can take it first, so another sender's post is refused `duplicate-key`. Case :331. What reaches it: any client that can predict another sender's dedup key.

Residue with no case, not a finding: the lost-wakeup mutant of `waited` (`seen` read after the view) passes the suite. The code as written orders it correctly, and no deterministic case can force the interleaving.

## 5. Attacked and held

- Every route behind 401: an unknown path (404), the wrong method (405), `/openapi.json`, `/docs`, and a malformed body all go through `authenticate` before `dispatch`. Only the generated reader's own refusals come first, as the note says.
- Signed target against served target: method, raw path and raw query are verbatim in both, and `dispatch` reads no header. A GET signature fails on a POST. Bytes past `Content-Length`, two `Content-Length` headers, and `Transfer-Encoding` (411) give the same body to the check and to dispatch.
- Headers: a header sent twice is refused, names are compared without case, and the reader trims values.
- Duplicate check: it uses the same `json::parse` and `from_utf8` as the generated decoders. `\u` escapes are compared decoded, the parser refuses a byte order mark, trailing bytes and non-UTF-8, and nesting is capped at depth 64.
- Read rule: `Pending`, `Unanswered`, `Outbox`, `Settled`, `Thread` and `Replies` carry top-level `to`/`from` and are filtered. Rows shown through `wait` are filtered before `has_rows`. `Due`, `Messages` and `Expire` return 403.
- Wait: a negative number, a fraction, an empty value, `+5` and a duplicate `wait` are 400. A value beyond 64 bits is cut to `max_wait` (with the default `max_wait`). The wait ends on stop.
- Panic in a connection: the slot is given back by `Drop` and the section recovers from poisoning. Connection threads are detached and exit, so nothing grows with answered requests.
- Binding: `0.0.0.0`, `::` and `::ffff:127.0.0.1` are refused (`is_loopback`), and port 0 is handed back.
- Message limit: counted inside the section, `>=`, so two concurrent posts cannot pass it.
- Expiry: `keep_until` and `now` are the same reading, and one refused `Expire` does not stop the others.
- Refusal bodies: none carries a key id, a principal or a host path, and the crate logs nothing.
- Repository rules: no name, e-mail or home path in the diff, no `unsafe` (`forbid`), no comment in the manifest, nothing edited under `generated/`.
- No tool call was denied.

## 6. Paths written outside the worktree

All under the brief's scratch directory, `adversary/`: `adversary-alone.log`, `suite.log`, `suite-without-adversary.log`, `mutants.sh`, `mutants.log`, `mutant/*.suite.log` and `mutant/*.adv.log` (16 logs). The tree copies have been deleted.

## 7. Findings block

```findings
- file: crates/inbox-server/src/listener.rs
  line: 116
  category: boundary
  severity: warning
  origin: introduced
  message: 'one unauthenticated connection that sends a byte every 300 ms holds a connection slot without end, so max_connections such connections stop every signed client'
- file: crates/inbox-server/src/listener.rs
  line: 137
  category: boundary
  severity: warning
  origin: introduced
  message: 'Running::stop and Server::run never return while a slow unauthenticated connection is open, because wait_idle has no bound'
- file: crates/inbox-server/src/service.rs
  line: 156
  category: boundary
  severity: note
  origin: introduced
  message: 'Instant::now() + wait overflows and panics the connection thread when max_wait is near Duration::MAX, leaving the caller without an answer'
- file: crates/inbox-server/src/auth.rs
  line: 115
  category: mutant
  severity: warning
  origin: introduced
  message: 'no case of the unit exercises RingVerifier::authorizes; authorizes-always-true and from/on_behalf_of-swapped mutants pass all 21 cases'
- file: crates/inbox-server/src/service.rs
  line: 67
  category: mutant
  severity: note
  origin: introduced
  message: 'the documented order 413 before 401 is not pinned; moving the body limit after authentication passes all 21 cases'
- file: crates/inbox-server/src/service.rs
  line: 84
  category: judgement
  severity: warning
  origin: pre-existing
  message: 'a caller authenticated as one principal posts an envelope whose from is another principal, so a recipient re-posts a handled envelope of its sender'
- file: ess/domains/messaging.yaml
  line: 473
  category: judgement
  severity: note
  origin: pre-existing
  message: 'a third principal tells an existing message between others (403 not-recipient) from no message (404 unknown-message) through read'
- file: ess/domains/messaging.yaml
  line: 422
  category: judgement
  severity: note
  origin: pre-existing
  message: 'the dedup key is global across principals, so a third principal learns or takes another sender''s key and that post is refused duplicate-key'
```
