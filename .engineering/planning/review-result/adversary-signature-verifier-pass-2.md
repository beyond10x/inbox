---
format: aep.planning-md/3
id: review-result:adversary-signature-verifier-pass-2
kind: review-result
status: active
title: adversary, pass 2, story:signature-verifier at 746046b
relations:
- reviews: story:signature-verifier
revision: 1
---
3 findings. One is a regression of the correction (warning), and two are notes.

unit: story:signature-verifier, commits 4125267 + 746046b on f13624f (branch impl/signature-verifier), plus the uncommitted crates/inbox-verify/tests/adversary2.rs
verdict: CONFIRMED
cases: executed 59→61, red 2
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: 11 paths (part 6)
needs-coordinator: none

## 1. Diff of the worktree

`git --no-pager diff --stat` is empty. `git status --short` shows only `?? crates/inbox-verify/tests/adversary2.rs`, which is a test file. No source, `generated/`, `ess/`, `docs/` or `.engineering/` path was touched.

## 2. The cases

Both are in `crates/inbox-verify/tests/adversary2.rs`. The first run of the file alone was `CARGO_INCREMENTAL=0 cargo test -p inbox-verify --locked --test adversary2`, exit 101.

**F1: memory blow-up in `duplicate_member`. Warning, introduced, a regression of the correction.**

- **Case:** `reading_a_small_key_file_does_not_allocate_a_thousand_times_its_size` (line 25).
- **What breaks:** `duplicate_member` (`crates/inbox-verify/src/duplicate.rs:29` and `:36`) builds the full dotted path of every child before it visits it, and keeps all the children's paths of one object or array in `pending` at once. With one long member name above a long array, it holds (name length × item count) bytes.
- **Red output:**
  ```
  adversary2.rs:39: a key file of 32801 bytes raised the peak resident size by 164712 KiB (bound 32032 KiB)
  ```
  The suite run gave 164788 KiB.
- **Regression:** the same case run against a copy of 4125267 passes. At 4125267 the decoder refused this text at linear cost.
- **Scale:** this is arithmetic, not run (no load tests). A 1 MiB text with a 512 KiB name over 256 Ki items would hold about 128 GiB.
- **Deep values:** a value from `parse_written` (no depth cap) costs O(depth²) in path copying.
- **What reaches it:**
  - Today: `KeyRing::parse` on the key file, which the operator controls.
  - By prescription: `docs/design/signing.md` tells every receiver to run `duplicate_member` on each incoming text before decoding. The story:http-service listener would then run it on caller bodies, before authentication.
- **Fix to name:** build a path only when a duplicate is found, for example by keeping parent links or path segments and joining them on a hit.

**F2: "nobody without the operator key can knock an id out" is false. Note, introduced, not a regression.**

- **Case:** `an_earlier_signed_entry_put_back_into_the_file_does_not_knock_out_the_renewed_key` (line 52).
- **What breaks:** the new sentence in `docs/design/signing.md:155` (repeated at `crates/inbox-verify/src/ring.rs:190`). The case renews a key under its own id (same key, later `valid_until`), the way the note says revocation is replaced. It then puts the earlier signed entry, unchanged and already lapsed, back into the file. That needs no operator key. Step 2 ignores both entries.
- **Red output:**
  ```
  adversary2.rs:88: an unchanged, lapsed entry the operator signed earlier knocked the renewed key out; ignored: [Ignored { kind: Key, id: "agent-key", reason: AmbiguousId }, Ignored { kind: Key, id: "agent-key", reason: AmbiguousId }]
    left: Err(UnknownKey)
   right: Ok(Principal("agent:host-a/session-one"))
  ```
- **Not a regression:** the case is red at 4125267 too, because lapse is not checked when the file is loaded. The behaviour came with the unit. The claim came with the correction.
- **Gap in the note:** `signing.md` does not say whether the operator may reuse a key id or delegation id when renewing.
- **What reaches it:** anyone who can put an older signed entry back into the key file, for example from a stale copy or an earlier key-value revision. Such a writer can also delete the entry, so the impact is small. It is the overstated claim that matters.
- **Fix:** either make the note say ids are never reused, or let a lapsed entry not count towards ambiguity.

## 3. Suite run

After the cases existed: `CARGO_INCREMENTAL=0 cargo test -p inbox-verify --locked --no-fail-fast`, exit 101.

| Target | Passed | Failed |
|---|---|---|
| lib | 12 | 0 |
| acceptance | 1 | 0 |
| adversary | 5 | 0 |
| adversary2 | 0 | 2 |
| cases | 34 | 0 |
| jcs | 7 | 0 |

`slot task check` then exits non-zero (`task: Failed to run task "check": exit status 101`), and only on adversary2. Every step before `cargo test` passed: spec, projections-drift, drift, conform-interpreted, `cargo fmt --check` and clippy with `-D warnings`.

## 4. Judgement finding without a case

**J1: the duplicate check is not stated as the caller's duty on `KeyRing::new`. Note, introduced, not a regression.**

- **Where:** `crates/inbox-verify/src/ring.rs:243`. `signing.md` says the key file is refused whole when a member is carried twice, but only `KeyRing::parse` runs that check. The public `KeyRing::new` takes an already decoded `KeyFile`, and its doc does not say the caller must run `duplicate_member` before decoding.
- **What reaches it:** nothing in the crate decodes key-file text without the check today. A future caller that decodes the file itself, such as a service reading it from a key-value bucket, would skip it silently.

## 5. What held

- **Mutants:** all 3 are caught by the 59 cases.
  - A, duplicate check removed from `parse`: red in `adversary.rs:241` and `cases.rs:983`.
  - B, the read step moved before the ambiguity step: red in `adversary.rs:315` and `cases.rs:1099`.
  - C, step 1 dropped so unsigned entries count: red in `cases.rs:1045`.
- **`json::parse` and `parse_written`** keep every member, so the check sees both.
- **Duplicates found:** a name against its escaped spelling (escapes are resolved before the compare); a duplicate inside an array of arrays; a duplicate whose two values are equal. An empty name is reported, with path "" at the root.
- **Normalization:** names that differ only by Unicode normalization are distinct. The closed decoders (`closed_at`) then refuse the unknown member, so nothing is decoded silently.
- **Paths:** answered in document order with an object's own names first, as documented.
- **Cost on plain shapes:** linear on a wide object of short names. Depth from `parse` is capped at 64.
- **Key id and delegation id:** separate lists, maps and lookups, as `signing.md` says ("each list").
- **Duplicate entries:** two byte-identical signed entries are both ignored, never resolved by order, as documented.
- **Cross-kind signatures:** a delegation signature cannot verify on a key entry, because the member names differ.
- **Ids:** case and normalization variants of an id are distinct exact strings, and only the operator can sign one. An empty id is only reachable by operator signature.
- **Order of `ignored()`:** it changed (step 1, then ambiguous, then step 3, per list). No contract or doc promises an order, and no pass-1 case pins one.
- **Rewrite of `KeyRing::new`:** every pass-1 "held" item that it could touch still passes (cases.rs 34/34, adversary.rs 5/5).
- **`signing.md` against the code:** steps 1 to 3 and the envelope duplicate sentence match the code, apart from F2.
- **getrandom 0.4.3:** no features needed. `fill` returns `Err` on OS failure and `SecretKey::generate` maps it with `?` to `RandomError`, so key creation fails closed. `Cargo.lock` holds 0.2.17 and 0.4.3.
- **Repository rules:** no `unsafe`, no comment in the manifest, `generated/` and `ess/` unchanged since f13624f, no name, e-mail or home path in the diff, no key literal (keys are created at run time).

## 6. Paths written outside the worktree

All are under `<scratch>/w2/signature-verifier/adversary2/`:

- `run1.log`, `suite.log`, `gate.log`
- `at-unit.sh`, `at-unit.log`
- `mutants.sh`, `mutants.log`
- `mutant-a.diff`, `mutant-b.diff`, `mutant-c.diff`
- `ring.orig.rs`

The tree copies `tree-unit/` and `tree-mut/` are deleted. No tool call was denied.

## 7. Findings block

```findings
- file: crates/inbox-verify/src/duplicate.rs
  line: 29
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'duplicate_member builds every child path before visiting it, so a 32 KiB key file with one long name over a long array raises peak memory by about 160 MiB, a regression of the correction caught by tests/adversary2.rs:25.'
- file: docs/design/signing.md
  line: 155
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'The claim that nobody without the operator key can knock an id out is false: putting back an earlier, lapsed, signed entry for a renewed id makes both ambiguous (tests/adversary2.rs:52), and the note does not say whether ids may be reused.'
- file: crates/inbox-verify/src/ring.rs
  line: 243
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'KeyRing::new is public and takes a decoded KeyFile without stating that the caller must run duplicate_member before decoding, so a caller that decodes the file itself skips the refusal signing.md promises.'
```
