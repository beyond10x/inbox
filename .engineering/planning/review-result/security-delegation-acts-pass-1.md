---
format: aep.planning-md/3
id: review-result:security-delegation-acts-pass-1
kind: review-result
status: active
title: 'Security review pass 1 on the delegation in the service: 2 findings, 4 cases added, 1 red'
relations:
- reviews: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
revision: 1
---
SECURITY REVIEW PASS 1
unit: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal, the service and the verifier, at 6de297b (worktree inbox-bl-s4-deleg-sec), plus the cases listed below, uncommitted
verdict: CONFIRMED (1 warning, 1 note)
cases: executed 67→69 (inbox-verify), 154→156 (inbox-server), red 1
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 10 paths, all under the brief's scratch directory sec1/ (listed in part 6)
needs-coordinator: none

The 67 and 154 "before" counts come from the runs of my cases alone: nextest skipped 67 and 154 cases there. I did not make a separate suite run without my cases.

**Commands that compiled or tested without the `slot` prefix and outside the shim: none.**
- Every compile and test was `cargo nextest run` or `cargo clippy` through the shim. All 9 build logs start with `slot: synced to build-1d`.
- Every cargo call ran from my managed worktree. There was no bare `task`, no `cargo run`, no `cargo xtask`, no `slot --here`, and no build from a tree under `~/.cache`.
- `scratch/sec1/suite.sh` was started without `slot`. It does `cd` into the worktree and contains only shim-wrapped `cargo nextest` and `cargo clippy` calls.
- I ran `rustfmt` locally on my two case files. It only formats and does not compile.

**1. git --no-pager diff --stat** (in my tree, after every mutant was reverted)
```
 crates/inbox-server/tests/main.rs | 1 +
 crates/inbox-verify/tests/main.rs | 1 +
 2 files changed, 2 insertions(+)
```
Untracked: `crates/inbox-server/tests/security_delegation.rs` and `crates/inbox-verify/tests/security_acting.rs`. Every changed path is a test file. Mutants were temporary edits in my tree only, and each was reverted by an inverse edit.

**2. Cases added** (each run alone before the suite, on build-1d)

`crates/inbox-verify/tests/security_acting.rs`:
- `a_signed_entry_for_a_text_that_is_no_address_admits_no_request`: GREEN, 0.007 s.
  - The setup: the operator signs live entries for the agent with the texts "", " x", "x ", "\tx" and "x\t". `acts_for` admits each of these pairs.
  - The assertion: `verify_request_for` still refuses each text `NotActingFor`.
  - It kills two `an_address` mutants that the story's cases let live.
- `the_pair_is_compared_byte_for_byte_and_grants_no_second_step`: GREEN, 0.005 s.
  - An NFD text against an NFC entry is refused.
  - A text that differs only in letter case is refused.
  - Holding AGENT-for-OTHER and OTHER-for-HUMAN does not admit AGENT for HUMAN.
- Output of the run: `2 tests run: 2 passed, 67 skipped`, exit 0.

`crates/inbox-server/tests/security_delegation.rs`:
- `a_lapsed_delegation_leaves_its_agent_no_close_of_the_persons_inbox`: RED, 0.008 s. The steps:
  1. The agent opens the person's inbox under the header.
  2. The clock moves to `valid_until`.
  3. A request with the header is 401, as it should be.
  4. The agent then sends close-inbox without the header. The case asserts `not-owner` and that the inbox stays Open.
  - Output, verbatim:
    ```
    assertion `left == right` failed: after valid_until the agent closed the person's inbox: {"outcome":"closed","published":[{"event":"inbox.messaging.InboxClosed","payload":{"address":"agent:host-b/session-two"}}]}
      left: ("closed", Some(Closed))
     right: ("not-owner", Some(Open))
    ```
    Summary line: `2 tests run: 1 passed, 1 failed, 154 skipped`, exit 100.
- `the_agent_answers_its_own_decision_in_the_persons_place`: GREEN, 0.009 s. The agent posts a decision to the person's open inbox, then marks, reads and answers it under the header for the person. The outcome is `answered`.

**3. Suite runs** (after the cases existed)
- `cargo nextest run -p inbox-verify --locked --no-fail-fast`: `69 tests run: 69 passed`, exit 0.
- `cargo nextest run -p inbox-server --locked --no-fail-fast`: `156 tests run: 155 passed, 1 failed`, exit 100. The one failure is `security_delegation::a_lapsed_delegation_leaves_its_agent_no_close_of_the_persons_inbox`. Step time 3.4 s; the slowest case took 2.1 s.
- `cargo clippy -p inbox-verify -p inbox-server --all-targets --locked -- -D warnings`: exit 0. The only warning is the old unused variable in `generated/rust/inbox/src/server/wire.rs:343`.

**Per invariant**
1. Header never honoured unsigned: HELD.
   - The story's cases cover: signed without the member and sent with the header, the reverse, a different value, the header twice, an empty value, and a trailing space.
   - Read, not run: the reader lower-cases and trims header names, and trims values with `str::trim` (Unicode whitespace), at `generated/rust/inbox/src/server/http.rs:240-241`.
   - The service then verifies the trimmed text, so a padded signed member fails closed (401).
   - Non-UTF-8 is refused 400 at line 226, before authentication, the same for every header.
   - There was no replay cache before either (signing.md lists it as a non-goal). With the member added or removed the signature no longer verifies (story cases).
2. Only a live delegation of the pair admits: HELD.
   - `acts_for` (`ring.rs:397`) checks the agent, the byte-equal text, and `at < valid_until`. Only entries the operator signed are held (`admit`).
   - An entry has no start instant, and the person's key is not consulted.
   - No transitive step and no Unicode or case folding: my verifier case.
   - The key file is read once at start (`config.rs:68`). Removing a delegation takes effect only at a new start, as the story's Open 5 A states.
3. No oracle: HELD.
   - The story's `an_uncovered_inbox_is_answered_as_a_bad_signature_byte_for_byte` holds it.
   - `crates/inbox-server/src` writes no log line.
   - `NotActingFor.kind()` is `SignatureInvalid`, visible only through the library API.
4. Every route asks the same question: HELD.
   - The `Caller` is built only from `Authenticated`, which exists only after `verify_request_for` returned Ok (`service.rs:88`).
   - Every view's rows pass `readable(caller.attributes)`, and commands are decided by the caller's attributes.
   - The wait and `undelivered=1` use the same caller (story's 409 case).
5. The person's inbox and no more: HELD as the design table says, with one gap in the design text (finding 2).
6. Open and close: NOT HELD. Red case `security_delegation.rs:68` (finding 1).
7. The wait: HELD. `wait.long` is cut to `max_wait` (`service.rs:229`). Story case `a_wait_admitted_under_a_delegation_reads_on_after_it_lapses_until_it_ends`.

**Mutants**
- `ring.rs:466`, dropped the `!text.ends_with(edge)` check: the story's 67 verifier cases stayed green. Caught by `security_acting::a_signed_entry_for_a_text_that_is_no_address_admits_no_request`.
- `ring.rs:464`, `an_address` returns true: the story's cases stayed green. Caught by the same case.
- `service.rs:146`, reply rule disabled (`&& false`): caught by `delegation::a_reply_under_the_delegation_is_still_held_to_the_model` and `delegation::a_reply_that_breaks_two_members_is_answered_by_the_first_in_the_order_of_answer`.
- `service.rs` `own_inbox_open` reading `principal` instead of `inbox`: caught by `delegation::a_wake_read_for_the_person_is_409_while_its_inbox_is_not_open`.

**4. Findings** (they cover 6de297b plus my uncommitted cases)

1. CONFIRMED, introduced, warning: `crates/inbox-server/tests/security_delegation.rs:68`.
   - **What was measured:** after `valid_until`, the agent without the header closes the inbox it opened for the person. Outcome `closed`, inbox `Closed`.
   - **What reaches it:**
     - Open 4 A makes the agent the `owner`: `ess/domains/messaging.yaml:348`, `sets: {owner: {caller: principal}}`.
     - `close-inbox` checks only `owner != caller.principal` (`:374`).
     - The person cannot close the inbox, and a reopen keeps the owner (`:351-357`). So until a new start, an agent whose delegation lapsed can close the person's inbox again and again, and every post to the person is then refused. The repeated close was read from the model, not run.
   - **Design:** `docs/design/service.md:212` says the agent closes "with the header or without it". `signing.md` grants the delegation only "until `valid_until`". The design does not state the power after the lapse as a limit.
   - **Where the fix lands:** a service rule beside the reply rule in `RequestContext` (`service.rs:146`): close-inbox is `not-owner` when `address != caller.inbox`. Or Open 4 B, where `owner` is set from `caller.inbox` (a change of `ess/`).
2. CONFIRMED, introduced, note: `docs/design/service.md:216`.
   - **What was measured:** green case `security_delegation.rs:150`. The agent posts a decision to the person's inbox and answers it itself for the person. It is recorded as the person's answer (`answered_by: principal`).
   - **What reaches it:** any agent with a live delegation. The design's `answer` row does not say whether this is meant. It is inside the v1 grant (no narrower scope), so it holds nothing; the design should name it.

0 pre-existing findings are in the private file. The private file was not written.

**5. Reviewed and not faulted**
- Header-count handling (`auth.rs` `values`).
- The six-member byte layout and the `None` path being byte-equal to the five-member form (story case).
- Verification order: the key, then the signature, then the delegation.
- Equal refusal bodies and heads.
- Post, refuse and collect under the header.
- The agent's own inbox under the header.
- The reply-rule order.

**Not reached**
- The client (not built yet).
- A header line over `MAX_HEAD`, and a lone CR inside a value: read only, not run.
- Running the repeated close and reopen after the lapse.
- The timing of the lookup (the design says it is not constant).
- The `Inboxes` view.

**6. Paths written outside the worktree**, all under `<scratch>/s4/deleg/sec1/`:
- `verify-cases.log`
- `server-cases.log`
- `mutant-end.log`
- `mutant-grammar.log`
- `mutant-reply-rule.log`
- `mutant-wake-inbox.log`
- `suite.sh`
- `suite-verify.log`
- `suite-server.log`
- `clippy.log`

**7. Findings block**
```findings
[
  {"file": "crates/inbox-server/tests/security_delegation.rs", "line": 68, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "An agent that opened the person's inbox under a delegation still closes it without the header after valid_until, because close-inbox checks only owner == caller.principal, so the grant outlives its delegation until a new start."},
  {"file": "docs/design/service.md", "line": 216, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Under the header the agent can answer a decision it sent itself to the person's inbox, recorded as the person's answer, and the design does not say whether that is intended."}
]
```
