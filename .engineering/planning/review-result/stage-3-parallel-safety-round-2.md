---
format: aep.planning-md/3
id: review-result:stage-3-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic on the stage 3 stories, round 2
relations:
- reviews: story:the-service-speaks-tls-with-a-pinned-certificate
- reviews: story:the-client-reconnects-after-a-restart
- reviews: story:the-service-binds-a-reachable-address
- reviews: story:inbox-wake-returns-on-a-post
- reviews: story:the-claude-code-adapter-wakes-an-idle-session
- reviews: story:the-metaharness-adapter-wakes-an-idle-session
- reviews: story:the-terminal-adapter-shows-a-post
- reviews: story:the-stage-3-acceptance-checks
revision: 1
---
approve

What I read: 8 stories with `aep plan artifact show` (revised Scope sections and the recorded scope of each), `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph`, and the `AGENTS.md` Layout and Commands rows (lines 37-44, 63, 72, 83), `crates/inbox-xtask/tests/` and `tools/mod.rs`.

Your three questions:
- Which land on one file: every pair among the TLS, reconnect, wake, terminal and bind stories on `client.rs`, `main.rs`, `reference.rs`, `generated/cli/inbox.json` and `crates/inbox-cli/tests/main.rs`. Round 1 found these unnamed.
- Does the plan say so: yes, now as one chain of `depends_on` edges, TLS then reconnect then wake then terminal then bind, with the shared file named in the `## Scope` of each. The edges are reconnect to TLS, wake to reconnect, terminal to wake, and bind to terminal and TLS. The three adapters and the acceptance story depend on wake. The acceptance story also depends on TLS and reconnect.
- Which name no surface: none now. The store records a scope for all 8, and the `waves` output lists 7 waves.
- One partial gap, not a finding: the metaharness adapter's code surface is unassessed. It lives in another repository under Open 2 default A, and its body says so (`the-metaharness-adapter-wakes-an-idle-session.md`, Scope lines 7-10). It cannot collide inside this tree.

Surfaces established: 7 stories cited, 1 inferred (metaharness in-repo docs only; claude-code's `adapters/claude-code/` is inferred as well), 0 unplaceable. Acceptance reads traced to a producer in the set: 8 (wake on reconnect, bind on TLS, the acceptance story on TLS, reconnect and wake, and the three adapters on wake). All 8 are recorded by `depends_on`.

Collisions still reported by `waves` (127 at whole-file granularity, because the recorded scopes are whole files) are deliberate, and each body says which part it edits:
- `docs/design/wake.md` and `website/docs/first-use.md` among the three adapters: each edits only its own heading, under stubs that the wake story creates.
- `AGENTS.md` and `docs/checks.md` for the acceptance story against the bind story and the claude-code adapter: different rows, with the acceptance story stating "no edge orders the two".
- `AGENTS.md` for the claude-code adapter against the chain: its insert after row 40 does not touch rows 42-44.

What I could not establish:
- The metaharness adapter's code surface (see above).
- Whether `adapters/claude-code/` and the new test files, which are inferred, are the paths the implementors will use.
- Out of my lane: a recorded scope at whole-file granularity makes `waves` print 127 collisions that the Scope prose explains. That is for the operator reading the waves, not a defect.

```findings
[]
```
