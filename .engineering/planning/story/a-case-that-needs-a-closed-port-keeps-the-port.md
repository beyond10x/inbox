---
format: aep.planning-md/3
id: story:a-case-that-needs-a-closed-port-keeps-the-port
kind: story
status: implemented
title: A case that needs a closed port keeps the port
summary: 10 places of the cases give a bound port back and expect it closed; another case's service can take it (red push run on main, 2026-11-24).
scope:
- confidence: cited
  path: crates/inbox-cli/Cargo.toml
- confidence: cited
  path: crates/inbox-cli/tests/adversary2.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_argument_not_utf_8.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_argument_not_utf_8_2.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_bridge.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_closed_port.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_reconnect.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_reconnect_2.rs
- confidence: cited
  path: crates/inbox-cli/tests/adversary_reconnect_3.rs
- confidence: cited
  path: crates/inbox-cli/tests/bridge.rs
- confidence: cited
  path: crates/inbox-cli/tests/main.rs
- confidence: cited
  path: crates/inbox-cli/tests/reconnect.rs
- confidence: cited
  path: crates/inbox-cli/tests/support/closed_port.rs
- confidence: cited
  path: crates/inbox-cli/tests/support/mod.rs
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T13:10:22Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T13:10:22Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-24T13:31:55Z", actor: "agent:inbox", revision: 9, decided_on: {"recorded":{"test_result":1,"review_outcome":3}}}
---
## Outcome

A case that needs a loopback address where nothing answers keeps the port for itself for as
long as it needs it, so that no other process can take the port while the case runs.

Today 10 places of `crates/inbox-cli/tests` get such an address by binding a listener on
port 0, reading its port and giving the listener back (`adversary_bridge.rs`, 2 cases;
`bridge.rs`, 1 case; the helpers `down_url`, `dead_url` and `closed_url` of
`reconnect.rs`, `adversary_reconnect.rs`, `adversary_reconnect_2.rs`,
`adversary_reconnect_3.rs`, `adversary_argument_not_utf_8.rs`,
`adversary_argument_not_utf_8_2.rs` and `adversary2.rs`). The cases of the binary run at
once, and each service of a case listens on a port the kernel picks, so the port given back
can go to the service of another case. On 2026-11-24 the push run of `inbox / task check` on
the release commit of 0.7.0 on `main` ended red that way:
`adversary_bridge::a_bridge_started_before_the_service_answers_waits_for_it` expects its
bridge to wait for 1 s on a closed port; the service of another case took the port and
answered 401, and the bridge ended with exit 1. The pull request run of the same commit and
the gate on the build node were green.

## Acceptance

- Every case of `crates/inbox-cli/tests` that needs an address where nothing answers holds
  that port itself while it needs it: bound by the case, never listened on, so that a
  connect is refused and no other process is given the port (state transition: the port of
  such a case moves from "free for any process" to "held by the case").
- One helper of `tests/support` makes the address; no case file keeps a helper of its own
  that gives a bound port back.
- A case holds the helper: a second bind of its port by another socket fails while it
  lives, and a connect to it is refused.
- No line of the product changes. `CHANGELOG.md` does not change.
- `task check` is green; no case over 5 s is added.

## Context

- The crate forbids unsafe code (`[workspace.lints.rust] unsafe_code = "forbid"`), so the
  socket is made with a safe call of a crate the workspace already locks (`rustix`), as a
  development dependency of `inbox-cli` only.
- A case that later starts a service on the same port gives the port back first; the time
  between the two is as short as the case can make it, and the story names each such case.
- Not in this story: cases of other crates, unless one has the same shape; the report of the
  implementation names every one it finds.

## Scope

Written after the build, from the diff against `main` at ed3f190: every path below is read
from it (cited).

Tests and one manifest (no file under a `src/` directory changes):

- `crates/inbox-cli/tests/support/closed_port.rs` (new): `support::ClosedPort`, a TCP socket
  bound to 127.0.0.1 on a port the kernel picks and never listened on, and its case.
- `crates/inbox-cli/tests/support/mod.rs`: the module and its export.
- `crates/inbox-cli/tests/adversary_bridge.rs` (2 cases) and
  `crates/inbox-cli/tests/bridge.rs` (1 case): the address of the bridge that starts before
  its service.
- `crates/inbox-cli/tests/reconnect.rs`, `adversary_reconnect.rs`, `adversary_reconnect_2.rs`,
  `adversary_reconnect_3.rs`, `adversary_argument_not_utf_8.rs`,
  `adversary_argument_not_utf_8_2.rs`, `adversary2.rs`: the helpers `down_url`, `dead_url`
  and `closed_url` are gone; their 10 callers hold a `ClosedPort`.
- `crates/inbox-cli/tests/adversary_closed_port.rs` (new), with its `mod` line in
  `crates/inbox-cli/tests/main.rs`: the case of the adversary pass.
- `crates/inbox-cli/Cargo.toml`: `rustix` with the feature `net` under `[dev-dependencies]`.
  `Cargo.lock` does not change.

Not touched: every product crate, `ess/`, `generated/`, `CHANGELOG.md`, `AGENTS.md` beyond
the row of the test step that every story record writes.

## Decisions

- The story was filed and accepted by the coordinator on 2026-11-24, after the push run of
  `inbox / task check` on the release commit of 0.7.0 on `main` ended red in one case. The
  operator's session decided that the fix goes to `main` first and that the tag 0.7.0 is made
  on that commit of `main`; the red run is not run again. No panel of plan critics read the
  story.
- How the port is held. The options were: (A) a socket bound to 127.0.0.1 and never listened
  on, which the case keeps: a connect is refused and the kernel gives the port to no other
  socket; (B) a listener the case keeps and never accepts on: a connect succeeds, so the
  client waits for an answer where the cases expect a refused connect at once; (C) a fixed
  port outside the range the kernel picks from. A is taken: it is what the cases say they
  need, and it rests on no setting of the host.
- The workspace forbids unsafe code, so the socket is made with the safe calls of
  `rustix::net`. `rustix` with the feature `net` is a development dependency of `inbox-cli`;
  the product's dependency keeps its features. `Cargo.lock` does not change: the feature only
  turns on features of a crate that is already locked.
- The socket is close-on-exec: a child process a case starts does not keep the port.
- No case starts a service on a port it held closed: the reconnect cases bring their service
  back on a port of its own. So no case gives a port back before a service starts.
- The cases that use the fixed ports 1 and 9 of 127.0.0.1 as an address where nothing answers
  (6 files of `inbox-cli`, the adapter cases of `inbox-xtask`) stay as they are: the kernel
  never picks those ports for a socket that asks for any port.
- `CHANGELOG.md` does not change: no shipped code and nothing a user of the release sees.

## Measured

- The red run: `inbox / task check` on the push of ed3f190 to `main`, 2026-11-24, 1 of 1127
  cases, `adversary_bridge::a_bridge_started_before_the_service_answers_waits_for_it`
  (`tests/adversary_bridge.rs:333`): the bridge ended with exit 1 after `transport: …
  Connection refused (os error 111); trying again until the service answers` and then
  `refused: unauthenticated (401)`. The pull request run of the same commit and the gate on
  the build node were green.
- A probe on the build node, 50 rounds of 2000 listeners on a port the kernel picks, all
  kept: with a port bound and given back (the old way), 25 of 100,000 binds got that port, in
  25 of the 50 rounds; with a `ClosedPort` alive, 0 of 100,000.
- The case of the helper in round 1: 0.004 s. With the helper giving its socket back at once
  (a mutant), red: `a second bind of the held port` succeeded where `AddrInUse` is expected
  (`tests/support/closed_port.rs:83`).
- The 3 cases of the bridge that starts before its service, 20 of 20 green in a row: 1.022
  to 1.150 s, 1.633 to 1.769 s, 0.028 to 0.197 s. No case over 5 s.
- Round 1: the cases of `inbox-cli` 434 → 435, 2 runs in a row `435 tests run: 435 passed, 0
  skipped`. 10 places changed: 3 cases of the bridge and the 7 helpers `down_url`, `dead_url`
  and `closed_url`, which are gone; their 10 callers hold a `ClosedPort`.
- Adversary pass 1 (review-result:adversary-closed-port-pass-1): 3 findings, all notes, 1
  case, green (0.005 s): `adversary_closed_port::a_child_started_while_a_port_is_held_does_not_inherit_its_socket`,
  red without the close-on-exec flag, which the case of the helper let live. It checked: every
  changed case keeps its `ClosedPort` in a named local to the end of the function; the held
  port goes to none of 2000 listeners that ask for any port, cannot be bound on the wildcard
  addresses of IPv4 and IPv6, and refuses 20 connects in under 1 s; the product's build does
  not get the feature `net`; the changed cases 10 of 10 green in a row and the crate twice,
  `436 tests run: 436 passed, 0 skipped`. 6 mutants: the bridge that ends at its first
  transport failure is red in all 3 cases of the bridge; the start line on every try is red
  in one of them and passes `bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying`
  (note 2, filed as a draft). It did not reach: the stand-ins that end after a fixed number
  of requests, the directory `adapters/` and the cases outside `crates/`.
- After the pass, by the coordinator: the adversary's case is in the suite (note 1). The case
  of the helper no longer binds the port again after the drop, which a listener of another
  case could take in between (note 3); it is
  `support::closed_port::a_closed_port_is_held_while_its_value_lives`. That a drop gives the
  port back is held by no case. No pass read that change.
- The gate runs of round 1 on the build node (`task check`): 1128 passed, 8 skipped, the test
  step 21 s, exit 0; once more before the pass, 1128 passed, 8 skipped, 19 s, exit 0.
- `task check` on the branch, on top of `plan/stage-3`, 2026-11-24T13:27:59Z to 2026-11-24T13:28:38Z, on the build
  node of the pool: exit 0, `1129 tests run: 1129 passed, 8 skipped`, the test step 19 s of
  its bound of 120 s.
