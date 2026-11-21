---
format: aep.planning-md/3
id: review-result:adversary-cli-client-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:cli-client (0c2426d): 8 red cases, 9 mutants the suite misses, 4 judgement findings'
relations:
- reviews: story:cli-client
revision: 1
---
17 findings: 8 red cases that show defects, plus 9 of 12 mutants that the unit's own suite misses. My guard cases catch all 9.

```
unit: story:cli-client, commit 0c2426d on impl/cli-client (base bec370e); covers the working tree at 0c2426d plus the untracked crates/inbox-cli/tests/adversary.rs
verdict: CONFIRMED
cases: executed 22→40, red 8
origin: introduced 17 / pre-existing 0 / undecided 0
wrote-outside-worktree: 12 paths (part 6)
needs-coordinator: none
```

## 1. Diff
`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short` shows one path:
```
?? crates/inbox-cli/tests/adversary.rs
```
That is a test file, uncommitted and unstaged. No implementation file was touched. All mutation work ran in a `git archive` copy under scratch, which is now deleted.

## 2. Cases added (`crates/inbox-cli/tests/adversary.rs`, 18 cases)
Red output below is from running the file alone (`cargo test -p inbox-cli --test adversary`), before the suite ran. Home paths are shortened.

Two corrections after the first run of the file:
- The first red of `repost_does_not_deliver_again…` was my typo: reading a note answers `handled`, not `read`. I fixed it.
- The body-limit case was rewritten to assert the exact claim it breaks.

Both outputs below come from the second run of the file alone.

**Red now (8):**

**F1. A note is delivered twice: `repost` re-posts a note the recipient already read** (`repost_does_not_deliver_again_a_note_its_recipient_already_read`, :141)
```
repost delivered the read note 01M7Z1HFB6CR7ZQ3125D9KPCEJ again: repost said {"reposted":[{…"arrived":true,"answer":{"outcome":"posted",…}}]}; bob's pending now holds ["01M7Z1HFB6CR7ZQ3125D9KPCEJ"]
```
- **Cause:** a note is never collectable, so it stays in the outbox file for 24 h. Once read it is no longer live, so the next `repost` posts it again (`commands.rs:483-519`, `outbox.rs:93`).
- **What reaches it:** the README's own first-use lines. I ran them verbatim in scratch with an added `inbox repost` at the end, and bob's `pending` showed "Hello, bob" again after he had read it.
- **Origin:** introduced. Severity: warning.

**F2. Two `keyfile add-key` runs at once: one exits 0 and its entry is lost** (`add_key_processes_at_once_lose_no_entry`, :465)
```
round 0: add-key exited 0 for ["c0-0", "c4-0"] and the key file does not hold them (ring: Ok([])); runs: [("c0-0", Some(0), ""), ("c1-0", Some(4), "inbox: keys-0.json: No such file or directory (os error 2)"), … 5 more exit 4 …]
```
- **Cause:** `replace` (`keys.rs:377-392`) writes to one fixed `<file>.new` path for every process, and nothing locks around read, append and rename. The processes overwrite each other's `.new` and steal each other's rename. The losers fail with a misleading ENOENT; a winner can rename a file holding someone else's entry.
- **What reaches it:** any script that adds keys in parallel.
- **Origin:** introduced. Severity: warning.

**F3. `add-key` widens the key file's mode** (`add_key_keeps_the_mode_of_the_key_file`, :546)
```
assertion `left == right` failed: the key file was 0644 after add-key, 0600 before
```
- **Cause:** `.new` is opened without a mode (`keys.rs:382-386`), so it takes 0666 minus the umask. The operator's 0600 is lost.
- **Origin:** introduced. Severity: note (the file holds public data).

**F4. `collect` loses the answer when the outbox file cannot be written** (`collect_does_not_lose_the_answer_when_the_outbox_file_cannot_be_written`, :414)
```
the answer is gone: collect exited 4 with stdout "" and stderr "inbox: …/state-ro/alice.outbox: Permission denied (os error 13)"; the service's outbox view now: {"rows":[]}
```
- **Cause:** at `commands.rs:169-170` the service has already removed the message when `outbox.remove(..)?` fails. The command returns before it prints the row it read, so the reply is gone for good.
- **What reaches it:** an outbox write failing after the post (disk full, quota, read-only directory).
- **Origin:** introduced. Severity: warning.

**F5. An outbox entry that does not decode never leaves, even after it expires** (`an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires`, :237)
```
expires 2026-11-21T12:33:18Z has passed and the entry is still in the outbox file; post: exit 1 body.body: expected a value satisfying `value.utf8_bytes <= 65536`, found a string; repost: exit 0 {"reposted":[]}
```
- **Cause:** `post` does not check the envelope against the model before keeping it. The service refuses a body over 64 KiB with a 400, which is a service refusal, so the entry is kept. The generated decoder then refuses the line, and `unexpired` and `of_sender` keep any line that does not decode, forever (`outbox.rs:169-189`). `repost` never sends it and expiry never removes it.
- **What reaches it:** `--body-file` with more than 64 KiB.
- **Origin:** introduced. Severity: warning.

**F6. `serve` ignores SIGTERM once the reader of its stderr is gone** (`serve_stops_on_sigterm_after_the_reader_of_its_standard_error_is_gone`, :636)
```
inbox serve after SIGTERM, its stderr reader gone: None (None: still running 10 s later)
```
- **Cause:** `eprintln!` in the signal thread (`serve.rs:122`) panics on EPIPE before it calls `stopper.stop()`. The handlers stay installed, so after that only SIGKILL ends the process.
- **What reaches it:** `inbox serve … 2>&1 | head -n1`, or any supervisor that closes the pipe.
- **Origin:** introduced. Severity: warning.

**F7. `INBOX_URL=http://localhost:<port>` does not reach a service on 127.0.0.1** (`a_localhost_url_reaches_a_service_on_127_0_0_1`, :772)
```
status: 3, stderr: "inbox: transport: localhost:41145 ([::1]:41145): Connection refused (os error 111)"
```
- **Cause:** `client.rs:64-68` tries only the first resolved address, and on this machine that is ::1.
- **Origin:** introduced. Severity: note.

**F8. A 301 answer is reported as a successful post** (`a_redirect_answer_is_not_reported_as_posted`, :213)
```
a 301 was reported as a successful post: Run { status: 0, stdout: "{\"message_id\":\"01M7Z1EZX5JPS8TY9685CVWETN\",…,\"answer\":{}}\n" }
```
- **Cause:** `Reply::accepted` (`client.rs:167`) treats every status below 400 as success.
- **What reaches it:** an `INBOX_URL` that names something other than `inbox serve`.
- **Origin:** introduced. Severity: note.

**Green now (10).** Except for the last one below, each is a guard that fails on the mutant named in it.
- `post_keeps_the_envelope_before_the_request_reaches_the_service` (M02). A stand-in server reads the outbox file at the moment the request arrives.
- `repost_exits_1_when_an_envelope_did_not_arrive` (M04)
- `a_secret_key_file_whose_public_key_is_another_keys_is_refused` (M06)
- `sigterm_while_the_first_line_is_written_stops_serve_with_status_0` (M07). `serve` is held inside the write of its first line by a full stdout pipe, checked through `/proc/<pid>/syscall`, and gets SIGTERM there.
- `repost_drops_an_envelope_refused_with_a_declared_error` (M04, M08). A kept line is tampered with, so the service answers `SignatureInvalid`.
- `post_writes_expires_24_hours_after_created` (M09)
- `the_outbox_directory_is_0700_and_its_file_0600` (M10)
- `serve_takes_max_messages_and_a_503_keeps_the_envelope` (M11)
- `serve_takes_max_wait` (M12)
- `json_without_a_subcommand_prints_one_document`. My theory was wrong; this holds.

**F9 to F17. Mutants the unit's suite does not catch.** Each was applied to a scratch copy, with `cargo test -p inbox-cli` run on the unit's 22 cases. The copy's baseline was 22 passed.

| # | Mutant | Unit's suite | My guard case |
|---|---|---|---|
| M01 | no outbox lock | caught | — |
| M02 | keep after send (`commands.rs:391`) | **missed** | caught |
| M03 | no mode check on the secret key file | caught | — |
| M04 | repost counts every refusal as arrived (`commands.rs:499`) | **missed** | caught |
| M05 | a refusal exits 0 | caught | — |
| M06 | no `public_key` check (`keys.rs:101`) | **missed** | caught |
| M07 | signal handlers after the first line (`serve.rs:97`) | **missed** | caught |
| M08 | repost never drops an envelope (`commands.rs:502`) | **missed** | caught |
| M09 | default lifetime 1 h (`commands.rs:33`) | **missed** | caught |
| M10 | outbox modes 0755/0644 (`outbox.rs:112`) | **missed** | caught |
| M11 | `--max-messages` ignored (`serve.rs:86`) | **missed** | caught |
| M12 | `--max-wait` ignored (`serve.rs:87`) | **missed** | caught |

All origin introduced, severity note. The guard logs show the specific guard case FAILED for each mutant.

## 3. Suite run (after the cases existed)
`CARGO_INCREMENTAL=0 cargo test -p inbox-cli --no-fail-fast`, exit 101:
```
Running unittests src/main.rs   test result: ok. 10 passed; 0 failed
Running tests/acceptance.rs     test result: ok. 2 passed; 0 failed
Running tests/adversary.rs      test result: FAILED. 10 passed; 8 failed
Running tests/cases.rs          test result: ok. 10 passed; 0 failed
```
- The unit's 22 cases, 10 runs in a loop (`--bins --test acceptance --test cases`, adversary file deselected): every run gave 10p / 2p / 10p and exited 0.
- No `inbox-cli-*` directory was left under `target/tmp`, and no process outlived a run.
- I did not run the gate (`slot task check`); with my red file present it fails by design.

## 4. Judgement findings (no cargo case; tree 0c2426d)
- **J1** (`keys.rs:343`, CONFIRMED, introduced, note). `add-key` with an operator key other than the one that signed the file exits 0 and leaves a file signed by two operators. The service pinned to the first operator ignores the new entry. Checked with a script.
- **J2** (`keys.rs:216-222`, CONFIRMED, introduced, note). `--valid-until 2000-01-01T00:00:00Z` is accepted, exit 0: a key that has already lapsed is added without a word. Checked with a script.
- **J3** (`client.rs:50-73`, CONFIRMED, introduced, note). `Service::from_url` accepts any host, so the client sends signed requests and envelopes over plain HTTP to a non-loopback host, silently. I did not test this live, because the rules allow loopback only.
- **J4** (`keys.rs:79` and `keys.rs:156-169`, CONFIRMED, introduced, note). Two seed copies outlive their use (found by reading the code):
  - the parsed `Value` tree of the secret key file holds the seed's base64 in a plain `String`, which is dropped without being zeroed;
  - `key new` grows a `Zeroizing<String>` by pushing to it, and each reallocation frees the old buffer unzeroed.

## 5. What held
- **Secrets:** no seed in any error, `--json` output, panic or `--help` (decode errors do not quote values).
- **Secret file:** modes 0640 and 0604 refused with exit 4; 0400 accepted; a symbolic link to a 0644 file refused.
- **`key new`:** refuses a dangling link and a directory (the link's target is not created); mode 0600 under umask 000 and 077.
- **Outbox:** 8 posts at once lose nothing; directory 0700 and file 0600.
- **Outbox `from`:** `repost` sends only lines whose `from` is the caller, so it does not make story:post-from-is-the-caller easier.
- **Declared refusals:** `UnknownInbox` keeps the envelope, `ShapeMismatch` drops it, `SignatureInvalid` on repost drops it.
- **Service refusals:** `503 message limit` keeps the envelope and exits 1; 401 names `unauthenticated`.
- **`serve` startup:** handlers are in place before the first line; `INBOX_BACKEND=nats` and `0.0.0.0` refused with exit 2; flags at 0 and at u64::MAX start and serve; a negative value is a usage error.
- **README:** the first use works exactly as written and states the limit to test keys and loopback.
- **Repository rules:** no names, home paths, manifest comments or `unsafe`; nothing under `generated/`, `ess/` or `.engineering/` changed.

## 6. Paths written outside the worktree
All under `<scratch>/w4/cli-client/adversary/`:
- `mutants.py`, `run-mutants.sh`, `loop10.sh`, `readme-first-use.sh`, `probe-keyfile.sh`, `probe-serve-flags.sh`
- `logs/`: adversary-alone-1/2, suite, suite-nff, loop10-1..10 and loop10.txt, mut-baseline, M01..M12, `logs/guard/M02..M12`, summary.txt, mutants-progress.txt, guard-progress.txt

The tree copy `mut/` and the probe directories `readme/`, `probe-keyfile/` and `probe-serve/` are deleted. I acquired a session lease (`adversary-cli-client-w4`) on the worktree and released it. No tool call was denied.

## 7. Findings
```findings
- file: crates/inbox-cli/src/commands.rs
  line: 483
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'a note stays in the outbox file after its recipient read it, and inbox repost posts it again as a new message counted arrived (README first use reproduces it)'
- file: crates/inbox-cli/src/keys.rs
  line: 377
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'keyfile add-key has no lock and one fixed .new path, so concurrent runs exit 0 with their entry missing from the key file and the others fail with ENOENT'
- file: crates/inbox-cli/src/keys.rs
  line: 382
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'add-key rewrites the key file with the umask default mode, widening a 0600 key file to 0644'
- file: crates/inbox-cli/src/commands.rs
  line: 170
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'collect returns exit 4 before printing when the outbox write fails after the service already collected, so the answer or refusal is lost for good'
- file: crates/inbox-cli/src/outbox.rs
  line: 173
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'an envelope the generated decoder refuses (body over 65536 bytes, kept after the 400) is never reposted and never leaves the outbox file, even after its expires'
- file: crates/inbox-cli/src/serve.rs
  line: 122
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'eprintln in the signal thread panics once the stderr reader is gone, so Stopper::stop is never called and inbox serve ignores SIGTERM until SIGKILL'
- file: crates/inbox-cli/src/client.rs
  line: 67
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'only the first resolved address is tried, so INBOX_URL=http://localhost:<port> fails with exit 3 against a service on 127.0.0.1'
- file: crates/inbox-cli/src/client.rs
  line: 167
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'any status below 400, a 301 included, is taken as success, so post exits 0 and prints a message id for an answer that is not posted'
- file: crates/inbox-cli/src/commands.rs
  line: 391
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M02 keeping the envelope after sending instead of before survives the unit suite'
- file: crates/inbox-cli/src/commands.rs
  line: 499
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M04 repost counting every refusal as arrived survives the unit suite'
- file: crates/inbox-cli/src/keys.rs
  line: 101
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M06 dropping the public_key-against-seed check survives the unit suite'
- file: crates/inbox-cli/src/serve.rs
  line: 97
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M07 installing the signal handlers after the first line survives the unit suite'
- file: crates/inbox-cli/src/commands.rs
  line: 502
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M08 repost never dropping an envelope after a declared refusal survives the unit suite'
- file: crates/inbox-cli/src/commands.rs
  line: 33
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M09 a default lifetime of 1 h instead of 24 h survives the unit suite'
- file: crates/inbox-cli/src/outbox.rs
  line: 112
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M10 outbox directory 0755 and file 0644 survives the unit suite'
- file: crates/inbox-cli/src/serve.rs
  line: 86
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M11 serve ignoring --max-messages survives the unit suite'
- file: crates/inbox-cli/src/serve.rs
  line: 87
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'M12 serve ignoring --max-wait survives the unit suite'
- file: crates/inbox-cli/src/keys.rs
  line: 343
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'add-key under an operator key that did not sign the existing file exits 0 and leaves a mixed file whose new entry the pinned service ignores'
- file: crates/inbox-cli/src/keys.rs
  line: 216
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'add-key accepts a --valid-until in the past and adds a lapsed key without a word'
- file: crates/inbox-cli/src/client.rs
  line: 50
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the client sends signed requests and envelopes over plain HTTP to any host INBOX_URL names, with no loopback check or warning'
- file: crates/inbox-cli/src/keys.rs
  line: 79
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the parsed Value of the secret key file and the growing Zeroizing String in key new leave unzeroed heap copies of the seed'
```
