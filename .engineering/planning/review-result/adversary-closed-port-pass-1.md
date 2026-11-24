---
format: aep.planning-md/3
id: review-result:adversary-closed-port-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the case that keeps its closed port
relations:
- reviews: story:a-case-that-needs-a-closed-port-keeps-the-port
revision: 1
---
ADVERSARY PASS 1
unit: story:a-case-that-needs-a-closed-port-keeps-the-port, round 1 at 58f9b83 (base ed3f190), the adversary tree with my uncommitted case
verdict: CONFIRMED (notes only; no red case, nothing holds the release)
cases: executed 435→436, red 0
origin: introduced 2 / pre-existing 0 / undecided 1
wrote-outside-worktree: 9 paths in the scratch directory (part 6)
needs-coordinator: none

The bound held: only test files changed, and no product or non-test path is in the diff.

1. Diff stat (`git --no-pager diff --stat`), with the untracked file I added:
 crates/inbox-cli/tests/main.rs | 1 +
 1 file changed, 1 insertion(+)
 plus untracked: crates/inbox-cli/tests/adversary_closed_port.rs
I also left a git-ignored script in the tree: .engineering/drafts/adv-runs.sh.
Every mutant was a temporary edit in the adversary tree and was reverted. The diff above shows none of them.

2. Cases added (`crates/inbox-cli/tests/adversary_closed_port.rs`, one `mod` line in `tests/main.rs`, sorted):
- `a_child_started_while_a_port_is_held_does_not_inherit_its_socket`
  - What it does: while a `ClosedPort` lives, it starts `sleep 5` with null stdio, reads the child's open file descriptors under procfs, and asserts that none of them is a socket. It then ends the child.
  - Result: green now, 0.005 s.
  - Why it is kept: it kills a mutant that the story's cases let live. With `SocketFlags::CLOEXEC` replaced by `SocketFlags::empty()` in `support/closed_port.rs:28`, it is red:
    `FAIL [ 0.005s] adversary_closed_port::a_child_started_while_a_port_is_held_does_not_inherit_its_socket`
    `the child holds a socket: ["/dev/null", "/dev/null", "/dev/null", "socket:[99027589]"]`
    The story's helper case `support::closed_port::a_closed_port_is_held_...` stays PASS under that mutant.
- The first version of the file had 3 more cases. All were green on their first run alone (4 tests run, 4 passed, 0.49 s, job synced to build-1d). I removed them because none kills a mutant that the story's cases let live:
  - a held port is not given to any of 2000 port-0 listeners;
  - a held port cannot be bound on `0.0.0.0` or `[::]`;
  - 20 connects are refused in under 1 s.
  Under the SO_REUSEADDR mutant, the wildcard case went red, but so did the story's own helper case. The port-0 case stayed green.

3. Suite and other runs, after the case existed. The job lines named build-1d. For the script's runs only the filtered lines were kept.
- `cargo clippy -p inbox-cli --all-targets --locked -- -D warnings`: Finished, no warning.
- `cargo nextest run -p inbox-cli --locked --no-fail-fast --stress-count 10 -E '<the 13 changed cases, closed_port cases and 413_of_a_lowered>'`: `10/10 stress run iterations: 10 passed` (16.5 s).
- `cargo nextest run -p inbox-cli --locked --no-fail-fast`, run twice: `436 tests run: 436 passed, 0 skipped` (12.0 s and 12.1 s), exit 0.

Mutants:
- No CLOEXEC on the held socket: survived the story's cases. Killed by my case.
- SO_REUSEADDR set on the held socket: caught by the story's `support::closed_port` case (`a second bind of the held port ...: Ok(TcpListener ...)`).
- Start line printed on every try (`wake.rs` `say_once`: `if *failures == 0` changed to `if true`):
  - caught by `adversary_bridge::a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal`;
  - survived `adversary_bridge::a_bridge_started_before_the_service_answers_waits_for_it` (that case does not claim this);
  - survived `bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying` (finding 2).
- Bridge ends at its first transport failure (`tried_again`: Transport changed to false): caught by all 3 inline cases, including `bridge::...keeps_trying` after 10.0 s.
- `inbox close` does not put the opened record back after a transport failure (`commands.rs` close: `taken.filter(|_| false)`): caught by `reconnect::a_close_that_failed_in_transport_or_was_refused_keeps_the_record` (left: [] right: [record]).
- Transport not counted as unsent (`commands.rs` `unsent`: Transport changed to false; run in the same job as the close mutant): survived `adversary_reconnect::a_request_posted_while_the_service_was_down_arrives_after_the_restart_by_itself`. I did not check whether this case's path reaches `unsent`. The case's code is unchanged except where its URL comes from, so I am not raising this as a finding of this story.

4. Findings:
- F1 (mutant, note, CONFIRMED, introduced), `crates/inbox-cli/tests/support/closed_port.rs:28`
  - Measured: the helper's doc comment promises close on exec, and no case of the story checks it. The mutant without CLOEXEC is green against the story's cases.
  - What reaches it: every case that spawns the binary while it holds a `ClosedPort`. A child that inherited the socket would keep the port after the case ended.
  - Fix: take `adversary_closed_port.rs` into the story.
- F2 (mutant, note, CONFIRMED, undecided), `crates/inbox-cli/tests/bridge.rs:1018`
  - Measured: `a_bridge_started_before_the_service_answers_says_so_and_keeps_trying` counts the stderr lines about 27 ms after the first line. That is before the second try, since the first pause is 100 ms. So "one line for the run of failures" passes when the line is printed on every try.
  - The story changed only the URL source of this case. I did not run it against the base, so the origin is undecided.
  - Fix: wait past at least two tries before counting, as the `adversary_bridge` case does with 1600 ms.
- F3 (concurrency, note, INFEASIBLE, introduced), `crates/inbox-cli/tests/support/closed_port.rs:87`
  - Measured: the helper's own case drops the value and then asserts that a bind of the same port succeeds. Between the drop and that bind, a port-0 listener of a concurrent case can take the port. This is the same race the story removes; the implementor's probe measured 25 hits in 100,000 port-0 binds, against the old pattern of a port bound and given back.
  - I could not show it red; the window is microseconds.
  - Fix: accept AddrInUse after the drop, or show the release by reading the socket table instead of binding again.
- No pre-existing security findings, so there is no private file.

5. What I attacked and could not break:
- Lifetimes: every changed case keeps `closed_port` as a named local until the end of the function. There is no `let _ =`, and no helper returns only a URL. In the `bridge.rs` case, `bridge` is declared after `closed_port`, so the bridge is dropped first. The `adversary_bridge` cases kill their child before returning.
- Port given back before a service starts: there is no such case. Reconnect cases restart on a new port the kernel picks.
- The held port is not given to a port-0 listener (2000 kept binds, also under the SO_REUSEADDR mutant). It cannot be bound on `0.0.0.0` or `[::]`, and a connect is refused at once (20 connects well under 1 s).
- IPv6: every URL of the helper is the `127.0.0.1` literal, so no client tries `::1`.
- Manifest: `Cargo.lock` is unchanged since ed3f190. `cargo tree -e features,normal -i rustix` shows only `alloc`, `std` and `process` for the product build. `net` appears only with dev edges.
- Remaining listener places in `crates/*/tests`: the stand-ins keep their listener in a thread that accepts. Fixed ports `127.0.0.1:1` and `:9` are never picked for port 0.
- Not reached: every finite stand-in (for example `adversary_w8_repost.rs`, `adversary_collect_record_2.rs`) for a later request that expects a refusal after the stand-in thread ended; the `adapters` directory and the top-level `tests` directory; the path of `unsent` in the reconnect case.
- Build commands I ran without the `slot` prefix: none. Every cargo call went through the shim; the first lines I kept name build-1d.

6. Paths written outside the worktree (all in the scratch directory, subdirectory adversary1):
- cases-alone.log
- mutant-cloexec.log
- mutant-reuseaddr.log
- mutant-say-every-try.log
- mutant-end-at-transport.log
- mutant-keep.log
- runs.log
- closed_port.rs.orig
- commands.rs.orig

7.
```findings
[
  {"file": "crates/inbox-cli/tests/support/closed_port.rs", "line": 28, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The close-on-exec flag of the held socket is promised in the helper's doc but no case of the story fails when it is removed; the adversary case on the child's descriptors kills that mutant."},
  {"file": "crates/inbox-cli/tests/bridge.rs", "line": 1018, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "undecided", "message": "The case counts the bridge's stderr lines about 27 ms after the first line, before the second try, so it stays green when the start line is printed on every try."},
  {"file": "crates/inbox-cli/tests/support/closed_port.rs", "line": 87, "category": "concurrency", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The helper's own case binds the freed port again after the drop, which a port-0 listener of a concurrent case can take in between, the same race the story removes."}
]
```
