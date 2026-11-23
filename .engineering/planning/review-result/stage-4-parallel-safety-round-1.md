---
format: aep.planning-md/3
id: review-result:stage-4-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic on the stage 4 stories, round 1
relations:
- reviews: story:the-chat-bridge-posts-a-pending-message
- reviews: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
- reviews: story:the-chat-bridge-marks-a-message-read-on-a-reaction
- reviews: story:the-chat-bridge-asks-a-decision-once
- reviews: story:the-chat-bridge-turns-a-thread-reply-into-an-answer
- reviews: story:the-operator-runs-the-chat-bridge-on-the-real-channel
revision: 1
---
needs-revision

Findings

story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal — Scope says "built beside story:the-chat-bridge-posts-a-pending-message and the stories after it, the one that lands second merges those parts", but its typed scope shares `crates/inbox-cli/src/main.rs`, `reference.rs` and `generated/cli/inbox.json` (all cited or inferred) with stories 1, 3 and 4. It also shares `AGENTS.md` with 1 and 3, and `website/docs/status.md` with 1. No `depends_on` edge orders the pairs, so `aep plan artifact waves` keeps them apart: story 2 lands in wave 1, story 1 in wave 2, and story 3 in wave 4. The prose says "beside" and the derived schedule says "before". Story 2 is also held for an unanswered architecture decision ("it waits for the operator's answer and is not built on a default"; rejected under answer A). So the derived order puts the whole bridge chain 1 to 6 behind that answer. Also, `generated/cli/inbox.json` is rewritten whole by `task cli-reference` and held by the case `the_cli_reference_document_is_current` (AGENTS.md:39, :90), so "merges" is wrong for it: the second lander regenerates. What would settle it: say which of two remedies the plan takes, without choosing here. (a) An ordering edge between story 2 and the bridge stories, with the shared files as its reason. (b) Split the shared surface so the two stop sharing it, for example the client flag and its reference in a part of its own. Either way the Scope sentence "built beside" has to match what the store derives, and for the generated file it has to say "regenerate". Citation: `.engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:177-184` (Scope) and `:52-55` (Outcome). Also the command `aep plan artifact waves --status draft`, whose collision lines list story 2 against stories 1, 3 and 4.

story:the-chat-bridge-turns-a-thread-reply-into-an-answer — the edge `depends_on: story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal` is recorded as unconditional, but the body says it "holds only under option B of Open 1 of that story", and that Open 1 is an operator decision nobody has answered. Story 6 reaches story 2 only through this edge, so `waves` and `graph` show a firm dependency on a story that answer A rejects. The decomposer filed no blocker artifact, as the brief allows. A `dependency-blocker` artifact (`blocks`) on the operator's answer to Open 1 of story 2 would make the conditional visible to the store. Alternatively, the scope sentence could say the edge is removed on answer A, as story 2's Outcome already does. Citation: `.engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md:11` (edge) and `:171-173` (Scope). Also `.engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md:144-155` (Open 1).

What I read: 6 stories in full, via `cat` on `.engineering/planning/story/*.md`. Commands: `aep plan artifact waves --status draft` (text and json, `--help`), `aep plan artifact graph`, `aep plan artifact relations`. Also `git grep` over `crates/inbox-cli`, `crates/inbox-server`, `crates/inbox-conformance`, `AGENTS.md` and `Taskfile.yml`.

Surfaces: all 6 items are placed. Each has cited paths, including story 6 (`website/docs/status.md`, `website/docs/limits.md`). For stories 1, 3, 4 and 5 the central file (`crates/inbox-cli/src/bridge.rs`, `tests/bridge*.rs`, `docs/design/bridge.md`) is inferred, and none of these exists yet. Story 1 creates them, and 3, 4, 5 and 6 only edit them after it, so no two items both create one file. 0 items are unplaceable.

Collisions in the set: stories 1, 3, 4, 5 and 6 are fully ordered by `depends_on` (1, then 3, then 4, then 5, then 6; 6 also depends on 4). Their shared files are `bridge.rs`, `main.rs`, `reference.rs`, `generated/cli/inbox.json`, `tests/main.rs`, `tests/support/`, `ess`, `docs/design/bridge.md` and the website pages. The only unordered pairs are story 2 against stories 1, 3 and 4 (finding 1). Story 2 against 5 and 6 is ordered by the edge from 5, which is conditional (finding 2).

Reads traced to a producer in the set: 10.
- Story 3 reads the record, stand-in `connectors` and ESS document of story 1.
- Story 4 reads story 3's ESS document and mark-read, plus story 1's stand-in.
- Story 5 reads story 4's expiry line, story 2's delegation, and the mark-read of story 3 and the stand-in of story 1.
- Story 6 reads stories 5, 4, 2 and 1.
- 6 of the 10 are direct `depends_on` edges: 3→1, 4→3, 5→4, 5→2, 6→5, 6→4.
- The other 4 run through the chain.
- 0 are unrecorded.
- Story 1 does not read story 2: it signs with a key that names the person as principal, which exists today.

What I could not establish:
- Whether story 2's header can travel from `ClientArgs` to `Service::send` through `client.rs` and `main.rs` alone. If it needs `commands.rs` (the `Client` struct) or `keys.rs` (`Caller`), story 2 also collides with stories 1, 3 and 4 on `commands.rs`. That would be an inferred collision.
- Story 2's check 5 needs `crates/inbox-conformance/src/wire_target.rs` to send the new header. `request_headers` there and in about 25 call sites in `crates/inbox-server/tests` is not in its scope. This collides with no item in this set. I did not guess about items outside it.
- Under non-default answers the surface changes. Open 1 B of story 1 (a new crate) moves every file of `crates/inbox-cli/src/`. Open 6 (i) of story 1 would synthesize a domain and change `generated/` and `unsupported.tsv`. Open 1 C and Open 4 B of story 2 edit `ess/domains/messaging.yaml`. The bare path `ess` in 4 stories would then collide with story 2. At the defaults it does not.
- Out of my lane, not counted: `AGENTS.md:146` and `task spec` fail on any `UNMAPPED:` marker in `ess/`, yet stories 1, 3, 4 and 5 acceptance say "every relation the sources do not settle left as an `UNMAPPED:` marker" in `ess/`. That is for the acceptance critic.

```findings
- file: .engineering/planning/story/an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal.md
  line: 177
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Scope says it is built beside the bridge stories and the later lander merges, but the typed scope shares main.rs, reference.rs, generated/cli/inbox.json, AGENTS.md and status.md with stories 1, 3 and 4 (partly inferred), no edge orders them, the store therefore puts story 2 in wave 1 ahead of story 1 and behind an unanswered operator decision, and the generated JSON is regenerated, not merged; record an ordering edge with the shared files as reason or split the shared surface."
- file: .engineering/planning/story/the-chat-bridge-turns-a-thread-reply-into-an-answer.md
  line: 11
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "The depends_on edge to the delegation story is recorded as unconditional while the body says it holds only under option B of an unanswered operator decision, so waves and graph show a dependency that answer A removes; file a dependency-blocker artifact for that answer or state the edge's removal on A in its Scope."
```
