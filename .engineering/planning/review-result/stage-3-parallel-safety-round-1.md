---
format: aep.planning-md/3
id: review-result:stage-3-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic on the stage 3 stories, round 1
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
needs-revision
story:the-client-reconnects-after-a-restart — shares `crates/inbox-cli/src/client.rs`, `main.rs`, `reference.rs`, `generated/cli/inbox.json` and a new `tests/main.rs` module with story:the-service-speaks-tls-with-a-pinned-certificate (cited, both bodies) and neither body nor edge orders them; `depends_on` or a stated order is needed, or split the surface — .engineering/planning/story/the-client-reconnects-after-a-restart.md:84-88
story:inbox-wake-returns-on-a-post — shares `crates/inbox-cli/src/client.rs`, `main.rs`, `reference.rs`, `generated/cli/inbox.json` and a new `tests/main.rs` module with story:the-service-speaks-tls-with-a-pinned-certificate (cited, both bodies), and its only edge is to the reconnect story; add an ordering edge recording the file, or split the surface — .engineering/planning/story/inbox-wake-returns-on-a-post.md:96-100
story:the-service-binds-a-reachable-address — shares `client.rs` (`Service::from_url`, lines 45-80), `main.rs`, `reference.rs`, `generated/cli/inbox.json` with story:the-client-reconnects-after-a-restart and story:inbox-wake-returns-on-a-post (cited, all three bodies), and its only edge is to the TLS story; add edges or split the surface — .engineering/planning/story/the-service-binds-a-reachable-address.md:79-82
story:the-terminal-adapter-shows-a-post — shares `main.rs`, `reference.rs`, `generated/cli/inbox.json` and a `tests/main.rs` module with story:the-service-speaks-tls-with-a-pinned-certificate and story:the-service-binds-a-reachable-address (cited, all three bodies), and its only edge is to the wake story; add edges or split the surface — .engineering/planning/story/the-terminal-adapter-shows-a-post.md:56-60
story:the-service-speaks-tls-with-a-pinned-certificate — lists `website/docs/first-use.md`, `status.md` and `limits.md`, also listed by the reconnect, binds, wake, claude-code, metaharness and terminal stories (cited), and no body says which section each edits; either order the stories or record which section belongs to which — .engineering/planning/story/the-service-speaks-tls-with-a-pinned-certificate.md:90
story:the-metaharness-adapter-wakes-an-idle-session — the claude-code and terminal stories also write a section of `docs/design/wake.md` (inferred; the file is created by story:inbox-wake-returns-on-a-post and does not exist yet) and `first-use.md`/`status.md` (cited), and the three adapters have no edge between them; order them or give each its own file — .engineering/planning/story/the-metaharness-adapter-wakes-an-idle-session.md:53
story:the-metaharness-adapter-wakes-an-idle-session — the only surface in this repository is `docs/design/wake.md` (inferred), `adapters/metaharness/` is inferred and under the default (Open 2 A) the adapter lives in another repository; its code surface is unassessed, not safe — .engineering/planning/story/the-metaharness-adapter-wakes-an-idle-session.md:52
story:the-stage-3-acceptance-checks — shares `AGENTS.md` and `docs/checks.md` (cited, both bodies) with story:the-service-binds-a-reachable-address, which has no edge to this story; Open 1 C only says the operator run follows the bind story, not who edits those files first; add an ordering edge or split the edits — .engineering/planning/story/the-stage-3-acceptance-checks.md:74-76
story:the-claude-code-adapter-wakes-an-idle-session — shares `AGENTS.md` (cited, both bodies) with story:the-stage-3-acceptance-checks and story:the-service-binds-a-reachable-address, and its test location `tests/acceptance/tests/` is inferred and may land on `checks.rs`; no edge orders these; add edges or split the surface, and pick one test location — .engineering/planning/story/the-claude-code-adapter-wakes-an-idle-session.md:69-73
story:the-service-speaks-tls-with-a-pinned-certificate — `aep plan artifact waves --kind story --status draft` reports all 8 stories unassessed ("no wave: nothing selected declares a scope") because the `## Scope` prose is never recorded with `aep artifact scope`, so the store cannot derive any collision and reads all 8 like safe ones; record each scope with its cited/inferred marks — command `aep plan artifact waves --kind story --status draft`

Pairs ordered by an edge, so not collisions: reconnect with wake (edge, transitively with claude-code, metaharness, terminal), TLS with binds, and the acceptance story with TLS, reconnect and wake.

What I read: 8 stories with `aep plan artifact show`, `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph`, `aep plan artifact scope --help`, `aep plan artifact relations`, the story files' Scope line numbers, and the directory listings of `crates/inbox-cli/{src,tests}`, `tests/acceptance/tests`, `docs/design`. Surfaces: 7 cited (all but metaharness), 1 inferred (metaharness code surface; claude-code's `adapters/` is also inferred), 0 unplaceable by prose. 0 scopes recorded in the store, so the store itself has 0 placed. Acceptance reads traced to a producer in the set: 8 (wake on reconnect; binds on TLS; the acceptance story on TLS, reconnect and wake; claude-code, metaharness and terminal on wake), all 8 recorded by `depends_on`.

What I could not establish:
- Whether the metaharness adapter has any in-tree surface beyond docs, because its Open 1 is unread and its default is another repository.
- Whether `tests/acceptance/tests/` or `crates/inbox-cli/tests/` is the claude-code test location (Open, not decided).
- Whether the doc collisions are on different sections. The bodies do not say, so I treated them as one file each.
- Out of my lane, not counted in the verdict: the design critic's question whether the adapters share `wake.md` because the split is wrong; the claude-code and metaharness acceptances are operator-checked evidence, which is the acceptance critic's.

```findings
- file: .engineering/planning/story/the-client-reconnects-after-a-restart.md
  line: 84
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares crates/inbox-cli/src/client.rs, main.rs, reference.rs, generated/cli/inbox.json and a new tests/main.rs module with story:the-service-speaks-tls-with-a-pinned-certificate (cited, both bodies) and neither body nor edge orders them; depends_on or a stated order is needed, or split the surface"
- file: .engineering/planning/story/inbox-wake-returns-on-a-post.md
  line: 96
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares crates/inbox-cli/src/client.rs, main.rs, reference.rs, generated/cli/inbox.json and a new tests/main.rs module with story:the-service-speaks-tls-with-a-pinned-certificate (cited, both bodies), and its only edge is to the reconnect story; add an ordering edge recording the file, or split the surface"
- file: .engineering/planning/story/the-service-binds-a-reachable-address.md
  line: 79
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares client.rs (Service::from_url, lines 45-80), main.rs, reference.rs, generated/cli/inbox.json with story:the-client-reconnects-after-a-restart and story:inbox-wake-returns-on-a-post (cited, all three bodies), and its only edge is to the TLS story; add edges or split the surface"
- file: .engineering/planning/story/the-terminal-adapter-shows-a-post.md
  line: 56
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "shares main.rs, reference.rs, generated/cli/inbox.json and a tests/main.rs module with story:the-service-speaks-tls-with-a-pinned-certificate and story:the-service-binds-a-reachable-address (cited, all three bodies), and its only edge is to the wake story; add edges or split the surface"
- file: .engineering/planning/story/the-service-speaks-tls-with-a-pinned-certificate.md
  line: 90
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "lists website/docs/first-use.md, status.md and limits.md, also listed by the reconnect, binds, wake, claude-code, metaharness and terminal stories (cited), and no body says which section each edits; either order the stories or record which section belongs to which"
- file: .engineering/planning/story/the-metaharness-adapter-wakes-an-idle-session.md
  line: 53
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the claude-code and terminal stories also write a section of docs/design/wake.md (inferred; the file is created by story:inbox-wake-returns-on-a-post and does not exist yet) and first-use.md/status.md (cited), and the three adapters have no edge between them; order them or give each its own file"
- file: .engineering/planning/story/the-metaharness-adapter-wakes-an-idle-session.md
  line: 52
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the only surface in this repository is docs/design/wake.md (inferred), adapters/metaharness/ is inferred and under the default (Open 2 A) the adapter lives in another repository; its code surface is unassessed, not safe"
- file: .engineering/planning/story/the-stage-3-acceptance-checks.md
  line: 74
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares AGENTS.md and docs/checks.md (cited, both bodies) with story:the-service-binds-a-reachable-address, which has no edge to this story; Open 1 C only says the operator run follows the bind story, not who edits those files first; add an ordering edge or split the edits"
- file: .engineering/planning/story/the-claude-code-adapter-wakes-an-idle-session.md
  line: 69
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "shares AGENTS.md (cited, both bodies) with story:the-stage-3-acceptance-checks and story:the-service-binds-a-reachable-address, and its test location tests/acceptance/tests/ is inferred and may land on checks.rs; no edge orders these; add edges or split the surface, and pick one test location"
- file: aep plan artifact waves --kind story --status draft
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "all 8 stories are reported unassessed (no wave: nothing selected declares a scope) because the Scope prose is never recorded with aep artifact scope, so the store cannot derive any collision and reads all 8 like safe ones; record each scope with its cited/inferred marks"
```
