---
format: aep.planning-md/3
id: review-result:adversary-cli-client-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:cli-client (e411f79): 3 findings, 1 blocker, 4 red cases'
relations:
- reviews: story:cli-client
revision: 1
---
3 findings: 1 blocker, 2 notes.

```
unit: story:cli-client, the correction e411f79 on 0c2426d (branch impl/cli-client); findings cover the worktree at HEAD e411f79 plus the untracked crates/inbox-cli/tests/adversary2.rs
verdict: NEEDS-CHANGE
cases: executed 48→52 (plus 1 ignored, F1), red 4
origin: introduced 3 (2 by e411f79, 1 by 0c2426d) / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (the assigned scratch adversary2/), listed in part 6
needs-coordinator: F-A goes against fix-1's own order ("400 or 413 is final"). Deciding between that order and keeping the envelope is yours.
```

## 1. Diff of the worktree

`git --no-pager diff --stat` prints nothing: no tracked file changed. `git status --short` shows one line:
```
?? crates/inbox-cli/tests/adversary2.rs
```
That is a test file only. `rustfmt` and `clippy -D warnings` are clean on it.

## 2. Cases added (`crates/inbox-cli/tests/adversary2.rs`), all red now. Red output from running each case alone, before any suite run:

**F-A, part 1: `a_413_of_a_lowered_request_limit_does_not_drop_an_envelope_a_later_repost_delivers`** (line 77)
- **Steps:** alice posts while the service is down (exit 3, envelope kept). She reposts to an `inbox serve --max-request-bytes 1024`, which answers 413. The service is restarted with the default limit and bob opens his inbox. Her repost then delivers nothing.
- **Output:**
```
the envelope 01M7Z34D7B7TGARQAHGEX6CS58 was dropped on a 413 of a lowered limit and never delivered: repost to --max-request-bytes 1024 exited 1 (inbox repost: 01M7Z34D7B7TGARQAHGEX6CS58 to agent:test/bob: refused: body too large (413)
inbox: refused: body too large (413)), outbox lines after it: 0; repost to the default service exited 0 ({"reposted":[],"unreadable":[]}); bob's pending: []
```

**F-A, part 2: `a_400_for_a_request_whose_bytes_stalled_does_not_drop_the_envelope`** (line 225)
- **Steps:** a loopback relay in front of the real service passes on the head, waits 1.6 s, then passes on the body. The real service answers 400 from its generated reader. A direct repost afterwards has nothing left.
- **Output:**
```
the envelope 01M7Z34J3CXBX2X6RZETNG406G was dropped on the service's 400 for a stalled request and never delivered: post exited 1 with {"error":"the body was shorter than `Content-Length` announced: Resource temporarily unavailable (os error 11)","status":400,"answer":{...}}; outbox after it: []; repost exited 0 ({"reposted":[],"unreadable":[]}); bob's pending: []
```

**F-B: `the_key_file_refusals_read_as_one_sentence_without_a_run_of_spaces`** (line 263)
- **Output:**
```
a reused id: the message holds a run of spaces: "inbox: keys.json already carries the id `alice-1`: an id is never reused, a new entry takes a              new id"
```

**F-C: `sigterm_ends_serve_when_its_standard_error_is_a_full_pipe`** (line 307)
- **Output:**
```
inbox serve after SIGTERM with a full standard error pipe: None (None: still running 10 s later)
```

## 3. Suite runs, all after the cases existed

**`cargo test -p inbox-cli --no-fail-fast`, 5 times:** exit 101 each time, and all 5 runs gave the same result:
- unit 14 ok
- acceptance 2 ok
- adversary 17 ok + 1 ignored
- cases 15 ok
- adversary2 0 passed, 4 failed

After each run: 0 `inbox` processes left, 0 `inbox-cli-*` directories in `target/tmp`, `git status` shows only the new file.

**`slot task check`:** exit 201. fmt, clippy, conform (92/92) and the other crates pass. It fails only at:
```
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 10.68s
error: test failed, to rerun pass `-p inbox-cli --test adversary2`
```

## 4. Findings

**F-A: `crates/inbox-cli/src/commands.rs:383`, `final_refusal` drops on any 400 or 413.**
- **Severity:** blocker. NEEDS-CHANGE. Introduced by e411f79 (0c2426d kept every non-declared refusal).
- **Measured:** the two cases above.
- **Why the rule is wrong:**
  - Since the correction, `post` passes every envelope through the decoder the service uses (`admitted`). `repost` only sends lines that decode.
  - So no 400 or 413 that the client can reach is about the envelope text. One exception: a hand-edited line carrying a member twice.
  - Every other 400 and 413 comes from the transport or from the service's settings:
    - step 1, generated reader: a gap of more than 1 s between bytes gives 400 "body was shorter than Content-Length", or "a header could not be read";
    - step 2: the `max_body` of this run (`--max-request-bytes`) gives 413;
    - the `MAX_BODY` 413 cannot be reached: an admitted envelope peaks at about 393 KB, measured with 65536 control characters.
- **What reaches it:**
  - The 413: `inbox serve --max-request-bytes`, a flag listed in AGENTS.md, on any run whose limit is below a kept envelope. The post or repost exits 1, and the only copy of the signed envelope is deleted.
  - The 400: a client stalled for 1 s or more between its two `write_all` calls (`client.rs`, head then body), e.g. a loaded machine or a stopped process. The case builds this with a relay, so it is rare but real.
- **Fix (named only, not applied):** keep the envelope on the service's 400/413 (`refused` member). Drop it only on a declared refusal other than `UnknownInbox`, and possibly on 400 `duplicate member`. Then the AGENTS.md row and the `final_refusal` doc follow.

**F-B: `crates/inbox-cli/src/keys.rs:465` and `:439`, run of spaces in two refusal messages.**
- **Severity:** note. CONFIRMED. Introduced by e411f79: the `\` line continuation was lost.
- **Measured:** the case above. The J1 refusal shows the same run of spaces in the probe log.
- **Reached by:** `keyfile add-key` with a reused id, or with another operator key.

**F-C: `crates/inbox-cli/src/serve.rs:132` (`watcher.join()`) with the comment at `:123-124`.**
- **Severity:** note. INFEASIBLE. Introduced by 0c2426d, where `eprintln!` already blocked before the stop.
- **What it is:** the correction's comment says a write after the stop "cannot keep the service from stopping". But `serve` joins the signal thread. When standard error is a full pipe whose reader is still alive, the warning blocks and the process never exits after SIGTERM.
- **What reaches it:** nothing found. It needs a standard error pipe that is already full.

## 5. What held (one line each)

- **adversary.rs vs pass 1:** each assertion matches pass 1's description, and only the F5 status/kept lines and the F1 ignore changed. I could not compare byte for byte: there is no copy of pass 1's file in git or in scratch.
- **Key file lock:**
  - 8 `add-key` at once give 8 entries.
  - A lock held by a live process blocks the next run, and the kernel releases it when that process is killed with SIGKILL.
  - A stale lock file does not block.
  - A leftover `<file>.<pid>.new` does not trip the next run and holds no `seed`; strace shows only public files touched.
  - The lock file is created 0600.
  - A key file in a read-only directory gives exit 4.
- **Mode:** 0600 is kept.
- **J1:** a lapsed entry does not block `add-key`. A hand-edited entry is refused and named.
- **J2:** a `--valid-until` of now is exit 2; `add-delegation` with 2000-01-01 is exit 2.
- **F4:** in text mode the row is printed once, then exit 4. `--json` gives one document with `done`. `outbox` with a read-only directory still exits 0.
- **F5 decoder:**
  - 65535 and 65536 bytes post; 65537 is exit 2.
  - 32768 two-byte characters post; 32769 is exit 2.
  - 65536 control characters post to the default service.
  - The text sent is the signed text: `admitted` only reads it.
- **F6:** standard output closed before the first line, then SIGTERM: exit 0. `serve 2>&1 | head -n1` with SIGTERM and with SIGINT: exit 0.
- **F7/J3:**
  - `localhost` and `127.1` reach the service.
  - `[::1]` against a 127.0.0.1 service is exit 3.
  - `0.0.0.0` and `[::ffff:127.0.0.1]` are exit 2; a path in the URL is exit 2.
  - The check runs while the client is built, before anything is sent.
  - The `nip.example` name was not run, to resolve nothing over the network. From the code, a name that does not resolve is exit 2.
- **F8:**
  - 204, an empty 200, 100-then-200 and a 500 with a text body are exit 3.
  - A 418 with `{}` is exit 3 and names the status.
  - A 200 with an unexpected JSON body is exit 0.
  - A 302 with a declared body is exit 1.
  - Each prints one JSON document.
- **J4:** `to_seed` is `Zeroizing`, the read buffer is sized before the read, and no seed appears in any message.
- **README:** the first use works as written. The AGENTS.md rows and the README paragraph match what I observed.
- **Mutants (8, in one tree copy, now deleted):**
  - Killed: N1 no lock, N3 no mode restore, N4 3xx as success, N5 no loopback check (by the unit case; the 192.0.2.1 case was skipped so nothing went off loopback), N6 no decoder check, N7 unreadable line dropped.
  - Survived: N2 (fixed `.new` name) and N8 (write before `stop`). Both are equivalent: the lock serialises runs, and `warn` ignores EPIPE. Under a blocking standard error both hang, which is F-C.
- **Tool blocks:** a chained `sleep; tail` was blocked by the harness. I waited with an until-loop as the block message said; no permission was denied.

## 6. Paths written outside the worktree

Everything is under `<scratch>/w4/cli-client/adversary2/`:
- **Scripts:** `mutants2.py`, `run-mutants.sh`, `loop5.sh`, `standin.py`, `probe-client.sh`, `probe-keys-serve.sh`, `probe-collect.sh`
- **Logs:** `logs/` (build, case, suite, loop5-1..5, gate, gate2, gate3, clippy, probe logs) and `logs/mut/` (one log per mutant, `progress.txt`)
- **Deleted:** the tree copy `mut/` and the probe directories `probe-client/`, `probe-keys/`, `probe-collect/`.

Inside the worktree, the gate wrote its own `.out/conform/`.

## 7. Findings block

```findings
- file: crates/inbox-cli/src/commands.rs
  line: 383
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Since every kept or sent envelope passes the service decoder, a 400 or 413 from the service only ever reflects its run settings (--max-request-bytes) or a stalled transport, yet final_refusal drops the only copy of a deliverable envelope (introduced by e411f79).'
- file: crates/inbox-cli/src/keys.rs
  line: 465
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'The reused-id and foreign-operator refusals of keyfile add-key lost their line continuation and print a run of 14 to 22 spaces mid-sentence (introduced by e411f79; also line 439).'
- file: crates/inbox-cli/src/serve.rs
  line: 132
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'serve joins the signal thread, so a warning blocked on a full standard error pipe keeps the process alive after SIGTERM, contrary to the comment at line 123 (introduced by 0c2426d; only a pre-filled pipe reaches it).'
```
