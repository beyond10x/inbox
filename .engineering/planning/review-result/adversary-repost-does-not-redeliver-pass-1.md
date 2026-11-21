---
format: aep.planning-md/3
id: review-result:adversary-repost-does-not-redeliver-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:repost-does-not-redeliver-a-handled-note (e8266d5): 7 findings (3 warnings, 4 notes), 6 introduced, 1 there before the unit'
relations:
- reviews: story:repost-does-not-redeliver-a-handled-note
revision: 1
---
## What was reviewed

story:repost-does-not-redeliver-a-handled-note at `e8266d5` on base `d52ba0f` (branch
`impl/w8-repost`), by one adversary on Opus, pass 1 of at most 2, on 2026-11-21.

## Verdict

Broken: 7 findings. 6 introduced by the unit (2 warnings, 4 notes) and 1 warning that was there
before it. 3 of the adversary's cases were red on `e8266d5` and green on the base.

## Findings

- F1 (warning, introduced; `crates/inbox-cli/src/commands.rs`, `repost`): when `repost` cannot
  write the epochs file it stops after the first envelope, so the other envelopes a restart
  dropped are never posted. The base wrote nothing on this path and posted them all. Measured
  with a state directory of mode 0500.
- F2 (warning, introduced; `crates/inbox-cli/src/commands.rs`, `repost`): the refusal
  `duplicate-key` has the error name of `duplicate`, so it recorded the epoch. The service
  stored nothing; once the message that holds the key is gone, no `repost` in that start posts
  the envelope.
- F3 (note, introduced; `crates/inbox-cli/src/outbox.rs`, `Epochs::parse`): every line is
  compared with every other line. An epochs file of 20 001 lines took 7.67 s to read in a debug
  build.
- F4 (note, introduced; `crates/inbox-server/src/listener.rs`): no case of the unit holds that
  the epoch carries no time. An epoch made from the clock in nanoseconds passed the unit's
  suite.
- F5 (note, introduced; `crates/inbox-cli/src/commands.rs`, `repost`): no case of the unit
  holds that a transport failure of the epoch question ends `repost` with exit 3. Reading that
  failure as "no epoch" passed the unit's suite.
- F6 (note, introduced; `crates/inbox-cli/src/commands.rs`, `post`): no case of the unit holds
  that `post` reports a failed write of the epochs file. Ignoring the error passed the unit's
  suite.
- F7 (warning, there before the unit; `crates/inbox-cli/tests/support/mod.rs`): the stand-in
  waits in `accept()` with no bound and its cases join it. A client that sends fewer requests
  than scripted hangs the case. Measured: one changed client hung the case
  `the_services_own_400_401_408_413_503_keep_the_envelope_on_post_and_on_repost` for 38
  minutes. The unit adds 5 more such joins in `crates/inbox-cli/tests/epoch.rs`.

## Changed clients and services (4 built in copies of the tree)

| Change | The unit's suite | The adversary's cases |
|---|---|---|
| A: `repost` skips an envelope whose epoch equals "none" | fails | fails, 1 case |
| B: the epoch is the clock in nanoseconds | passes | fails, 1 case |
| C: a transport failure of the epoch question read as no epoch | passes | fails, 1 case |
| D: `post` ignores a failed write of the epochs file | passes | fails, 1 case |

## What held

- An interruption between the 2 writes leaves "no epoch", and the envelope is posted again.
- Every change runs under the outbox lock; `record_epoch` checks that the outbox line exists.
- The epoch recorded is the one of the post's own answer, never the one of the question.
- A header value with a tab, a line break, upper-case hex, 31 or 33 digits or 2 values gives no
  epoch. An epochs file with CRLF, no final newline, no content, bytes that are not UTF-8 or
  mode 000 reads as "no epoch". The file and its `.new` are mode 0600.
- The epoch question is signed, changes nothing and wakes no wait.

## Cases

7 cases: 6 in `crates/inbox-cli/tests/adversary_w8_repost.rs`, 1 in
`crates/inbox-server/tests/adversary_w8_epoch.rs`. 3 were red on `e8266d5` (F1, F2, F3) and
carried an ignore attribute; 4 were green and fail the changed clients and services A to D.
