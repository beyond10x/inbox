---
format: aep.planning-md/3
id: review-result:adversary-signature-verifier-pass-1
kind: review-result
status: active
title: adversary, pass 1, story:signature-verifier at 4125267
relations:
- reviews: story:signature-verifier
revision: 1
---
6 findings. None is a blocker. Of the 6, 3 are cases red on the unit as it stands, 2 are mutants the existing suite misses (the adversary's cases catch them), and 1 is a judgement note.

unit: story:signature-verifier, commit 4125267 on f13624f, worktree inbox-s2-w2-verify, plus the uncommitted crates/inbox-verify/tests/adversary.rs
verdict: CONFIRMED
cases: executed 49→54, red 3 (both mutant-catching cases go red on their mutant)
origin: introduced 5 / pre-existing 1 / undecided 0
wrote-outside-worktree: 7 paths (listed in part 6)
needs-coordinator: whether a duplicate JSON member must be refused (findings 3 and 4). It touches the generated reader, which the unit cannot edit.

## 1. Diff of the worktree
`git --no-pager diff --stat` is empty. `git status --short` shows only `?? crates/inbox-verify/tests/adversary.rs`. That is a test file: no source, `generated/`, `ess/`, `docs/` or `.engineering/` path was touched.

## 2. The cases
All in `crates/inbox-verify/tests/adversary.rs`; first run of the file alone, `cargo test -p inbox-verify --locked --test adversary`, exit 101.

- **F1, warning, mutant, introduced.** `a_reply_from_another_principal_on_behalf_of_the_key_principal_is_refused` (line 186).
  - **What breaks:** the check that a reply's key belongs to its sender has no test. The case sends a reply `from` OTHER `on_behalf_of` AGENT, signed with AGENT's key, under a live delegation for that pair. It expects `KeyNotFrom`.
  - **Now:** green.
  - **Mutant:** `src/canonical.rs:90` changed from `&self.from` to `&self.on_behalf_of` in `impl Envelope for Answer`, in a scratch copy. The existing suite stays green: 12/1/29/7, exit 0. This case goes red: `left: Ok(()) right: Err(KeyNotFrom)`.
  - **What reaches it:** the service checks every reply with `verify_envelope`. If someone breaks that line, a key holder can send replies as another principal and every existing test still passes.
- **F2, note, mutant, introduced.** `a_signature_whose_r_is_of_small_order_is_refused` (line 148).
  - **What breaks:** the suite never tests that verification is strict.
  - **The case:** it builds a signature whose R is the identity point, using the seed the test generates at run time. It first proves the plain `ed25519_dalek` `verify` accepts that signature and `verify_strict` refuses it, then expects `SignatureMismatch` from the verifier.
  - **Now:** green.
  - **Mutant:** `src/keys.rs:95` changed from `verify_strict` to `Verifier::verify`. The existing suite stays green: 12/1/29/7, exit 0. This case goes red: `left: Ok(()) right: Err(SignatureMismatch)`.
  - **What reaches it:** only the owner of a key can produce such a signature, so this is malleability rather than forgery. But `docs/design/signing.md` names this as a refusal and no test enforces it.
- **F3, note, judgement, introduced.** `a_key_file_whose_entry_carries_a_member_twice_is_refused` (line 224).
  - **What breaks:** `KeyRing::parse` (`src/ring.rs:277-281`) accepts a key entry that carries `principal` twice. The generated reader keeps the first value.
  - **Red now:** `adversary.rs:241: a key file with a duplicate member was read; it holds key k for Some(Ok(Principal("agent:host-a/session-one")))`
  - **What reaches it:** the verifier holds the value that was signed, so nothing is forged. A reader that keeps the last value, such as jq or the operator reading the file, sees a different principal. `jcs::canonicalize` refuses duplicate members; the key file path does not.
- **F4, note, judgement, pre-existing.** `a_post_whose_text_carries_a_member_twice_is_not_decoded` (line 250).
  - **What breaks:** a post whose text carries `body` twice decodes to the first value and verifies. One signature then covers two texts.
  - **Red now:** `adversary.rs:279: a post with body twice decoded (body Ok("the signed body")) and verified: Some(Ok(()))`
  - **Where it comes from:** the generated `Value::member` takes the first match (`generated/rust/inbox/src/server/json.rs:46`), and `closed_at` does not refuse duplicates.
  - **Origin:** `git diff --quiet f13624f HEAD -- generated` is clean, so this code is identical at the base. Not run at the base.
  - **What reaches it:** the decode path of story:http-service. Nothing in this unit calls it yet; the case builds the text itself.
- **F5, note, contract-drift, introduced.** `an_id_two_operator_signed_entries_carry_is_ignored_even_when_one_key_is_refused` (line 290).
  - **What breaks:** the commit message and the brief say an id carried by more than one signed entry is ignored in every one of them. Here two entries the operator signed carry the id `twice`, and one has a public key that is refused. The other is held.
  - **Red now:** `adversary.rs:309: ignored: [(Key, "twice", PublicKey(Base64))] left: Ok(Principal("agent:host-a/session-one")) right: Err(UnknownKey)`
  - **Doc and code agree:** `docs/design/signing.md` limits the rule to entries that passed the checks above, and `src/ring.rs:263` does that. The drift is between that and the stated claim.
  - **What reaches it:** an operator re-signs an id with a typo in the key. The old key stays live instead of both being ignored.

## 3. Suite run
After the cases existed: `CARGO_INCREMENTAL=0 cargo test -p inbox-verify --locked --no-fail-fast`, exit 101.
- lib: 12 passed
- acceptance: 1 passed
- adversary: 2 passed, 3 failed
- cases: 29 passed
- jcs: 7 passed
- `error: 1 target failed`

The 49 before comes from the unit's stated 12+1+29+7, matched by the counts of this same run with the adversary's file excluded. `cargo fmt --check -p inbox-verify` exits 0, and `cargo clippy -p inbox-verify --all-targets --locked -- -D warnings` exits 0.

## 4. Judgement finding without a case
- **J1, note, judgement, introduced.** `crates/inbox-verify/Cargo.toml:15` adds `getrandom` 0.3. `Cargo.lock` already holds `getrandom` 0.4.3 through `uuid`, and the unit's lock now has 0.2.17, 0.3.4 and 0.4.3. Using 0.4 would not add a third version.

## 5. What held
- **Every field of the envelope:**
  - changing `thread`, `dedup_key`, `decision` (empty or not), `authority` set to `""`, or the offset spelling of `created` on a post is refused;
  - changing `in_reply_to`, `reply_id`, `on_behalf_of`, `authority`, `created` or `expires` on a reply is refused;
  - wire text with `null`, reordered members, escapes or `3.0` gives the same signed bytes.
- **Signatures and keys:**
  - S plus L, and an all-zero signature, are refused;
  - an all-zero public key and the y = −1 key are ignored as weak;
  - a key entry with its principal, public key or key id changed under a kept signature is ignored;
  - a ring with zero entries refuses every key id.
- **Request window and paths:**
  - with a fractional instant, both ±60 s bounds are inclusive, and 1 ns past them or a fraction truncated past 9 digits is refused;
  - `..`, `//`, `%2F`, reordered or re-encoded query, a dropped query, and method case `Post` are each refused;
  - an empty body verifies against the hash of the empty string.
- **Instants:** `-00:00`, a `+23:59` offset, year 0000, a leap second at 9999-12-31, and a refused `+24:00` / `+0100` all read as the design note says. `Instant::parse` has no panic path, because `date_time` fixes every index first.
- **Contexts and canonical form:** the context strings and the member names keep the kinds apart. The canonical form, the duplicate-member refusal and the escapes of `jcs.rs` are correct for envelope values.
- **Key material:** the `SecretKey` `Debug` output holds the seed in none of base64, hex or byte array.
- **Dependencies:** curve25519-dalek 4.1.3 is the patched version for RUSTSEC-2024-0344, ed25519-dalek 2.2.0 for RUSTSEC-2022-0093, and sha2 0.10.9 is outside RUSTSEC-2021-0100 (the local advisory database of cargo).
- **Repository rules:** no `unsafe` (the workspace forbids it), no comment in the manifest, `generated/` unchanged, and no name or home path in the diff.

## 6. Paths written outside the worktree
All under `<scratch>/w2/signature-verifier/adversary/`: `run1.log`, `mutants.sh`, `mutants.log`, `mutant-a.diff`, `mutant-b.diff`, `suite.log`, `suite2.log`.

The tree copy `tree/` (with the throwaway probe file `tests/probe.rs`) was deleted. No tool call was denied.

## 7. Findings block
```findings
- file: crates/inbox-verify/src/canonical.rs
  line: 90
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'The sender binding of a reply can become on_behalf_of without any of the 49 cases failing; tests/adversary.rs:186 catches it.'
- file: crates/inbox-verify/src/keys.rs
  line: 95
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'Replacing verify_strict with the non-strict verify leaves the suite green; tests/adversary.rs:148 catches it with a small-order R signature.'
- file: crates/inbox-verify/src/ring.rs
  line: 278
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'KeyRing::parse accepts a key entry with a duplicate member and holds the first value, so other JSON readers see a different principal.'
- file: generated/rust/inbox/src/server/json.rs
  line: 46
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: 'A post whose text carries body twice decodes first-wins and verifies, so one signature covers two texts that differ to a last-wins reader.'
- file: crates/inbox-verify/src/ring.rs
  line: 263
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'An id carried by two operator-signed entries is not ignored when one has a refused public key, unlike the claim in the commit message.'
- file: crates/inbox-verify/Cargo.toml
  line: 15
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'getrandom 0.3 adds a third getrandom version to Cargo.lock beside 0.2.17 and the 0.4.3 that uuid already pulls in.'
```
