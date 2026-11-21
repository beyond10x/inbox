---
format: aep.planning-md/3
id: review-result:adversary-post-from-is-the-caller-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:post-from-is-the-caller (3cb3eb9): 2 notes, 8 changed cores, no warning'
relations:
- reviews: story:post-from-is-the-caller
revision: 1
---
## What was reviewed

story:post-from-is-the-caller at `3cb3eb9` on base `dcaa193` (branch
`impl/post-from-is-the-caller`), by one adversary on Opus, pass 1 of at most 2, on 2026-11-21.
No second pass ran: the corrections changed no implementation file.

## Verdict

2 findings, both notes, both introduced by the unit. No blocker and no warning: on the running
service a post or an answer whose `from` is not the caller was refused in every way tried.

## Findings

- F1 (`crates/inbox-core/src/obligations.rs`, the `None` arm of the caller check): the claim
  that a `Client` without a principal is an unmet obligation was held by no test. A core that
  treats `None` as a match passed `cargo test --workspace`, both conformance lanes and the
  acceptance checks. Nothing in the repository reaches the arm: the listener always sets both
  caller attributes.
- F2 (`crates/inbox-core/src/obligations.rs`, the comparison): nothing held that the comparison
  of `from` with the caller's principal is exact. A core that compares without regard to ASCII
  case passed every gate of the unit. Over HTTP the check of the key's principal against `from`
  in `crates/inbox-verify` answers `signature-invalid` first.

## Changed cores (8, in tree copies)

| Change | `cargo test` | core lane | serve lane | acceptance |
|---|---|---|---|---|
| the check removed | 9 fail | 2 fail | 1 fail | passes |
| the check behind the external branches | 3 fail | 1 fail | passes | passes |
| the check behind the row reads | 5 fail | 2 fail | 1 fail | passes |
| `on_behalf_of` compared | 13 fail | 51 fail | 2 fail | passes |
| `None` counts as a match | only the adversary's case | passes | passes | passes |
| the generated guard of `Answer` off | only the adversary's 2 | 1 fail | 1 fail | passes |
| comparison without regard to case | only the adversary's 2 | passes | passes | passes |
| the interim example value answered by name | none | 1 fail | passes | passes |

The 3 acceptance checks notice none of the 8.

## What held

- The order of `not-sender` against the other refusals of `PostMessage` and `Answer`, on the
  interpreted target, the core and the service, with 2 scenarios written outside the suite.
- The answer to a non-sender is the same bytes for a held id, a held dedup key, a fresh
  envelope and an unopened inbox. A refused replay takes neither the id nor the key.
- A delegation to the caller does not let it send as the principal it speaks for.
- The claim of `answer-from-another-principal-refused` on `Answer/outcome/not-sender`: without
  the guard its steps answer `not-recipient`.
- `post-from-another-principal-refused` is the one scenario that witnesses the replay over
  HTTP.
- The interim example: removed, `task coverage` exits 1 with 47 scenarios; set to another
  number, exit 1. It appears in 3 generated files and changes no decoder. The 19 scenarios
  that lost a run lost reruns that witnessed nothing specific to a caller on the core and were
  unsupported over HTTP.
- `generated/` regenerates without a difference; `crates/inbox-server/src` is unchanged;
  `unsupported.tsv` matches the run.

## Cases

8 cases in `crates/inbox-core/tests/adversary_w6_from.rs` (3) and
`crates/inbox-server/tests/adversary_w6_from.rs` (5), all green on `3cb3eb9`, red on the
changed cores above.
