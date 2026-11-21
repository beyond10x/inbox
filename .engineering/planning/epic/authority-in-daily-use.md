---
format: aep.planning-md/3
id: epic:authority-in-daily-use
kind: epic
status: draft
title: 'Stage 6: authority in daily use'
summary: Sessions act on a relayed decision because its delegation verifies, with no standing rule in a session's instruction file.
revision: 1
---
## Outcome

A session that relays an operator's decision posts it with `on_behalf_of` and a delegation in
`authority`. The receiving sessions act on it because the service accepted the delegation, and a
harness plugin states that rule once. No session carries a hand-written trust rule for another
session's name.

## Acceptance

A decision relayed with a delegation is acted on by 3 sessions on 2 hosts whose instruction files
name no trusted session, and the same envelope without the delegation is refused with
`NoAuthority`, where before this epic a relayed decision is trusted because an instruction file
names the relaying session.

## Context

Stage 6 of `docs/design/inbox.md`. The version 1 verifier reads a key file signed by the operator
key and checks a delegation chain of length one. General agent identity (issuing, revoking and
chaining delegations) belongs to Mandate; this stage decides whether the key file is replaced by
it.
