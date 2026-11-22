---
format: aep.planning-md/3
id: story:a-case-joins-the-stand-in-it-starts
kind: story
status: draft
title: A case joins the stand-in it starts, so a request that never arrives fails the case
revision: 1
---
## Problem

`crates/inbox-cli/tests/adversary.rs`, case `a_redirect_answer_is_not_reported_as_posted`,
never joins its stand-in (`_seen`). When no request reaches the stand-in, its panic is
swallowed and the case stays green.

Found by adversary pass 1 on story:case-stand-ins-bound-their-wait
(review-result:adversary-case-stand-ins-bound-their-wait-pass-1, finding 6, pre-existing).

## Acceptance

The case fails with the stand-in's message when no request reaches the stand-in (state
transition: green without a request before, red after). The same check for every other case of
`crates/*/tests` that binds a stand-in handle to a name starting with `_`.

## Scope

- `crates/inbox-cli/tests/adversary.rs`
