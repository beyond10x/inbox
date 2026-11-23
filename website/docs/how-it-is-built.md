---
title: How it is built
description: Specification first, generated code held by a drift check, a conformance suite, one gate.
---

# How it is built

## The specification comes first

Every behaviour is written in [ESS](https://github.com/beyond10x/ess) in `ess/` before code
exists for it: the entities, commands, views, events and every refusal. A rule ESS 0.98.0
cannot state is listed in
[`docs/design/inbox.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/inbox.md); it
is implemented by the service and tested there.

## Generated code is held to it

From `ess/`, ESS writes:

- the Rust crate `generated/rust/inbox`: model types, command behaviours, view queries, storage
  and context ports, the route table and JSON codecs;
- the OpenAPI, AsyncAPI and JSON Schema projections under `generated/`;
- the [specification pages](/specification/) of this site, under `generated/docs`.

Nothing under `generated/` is edited by hand. `task check` fails when any of it differs from a
fresh generation from `ess/`.

## The conformance suite

ESS synthesizes a conformance suite from `ess/`. `task check` runs it on ESS's interpreted
target, against the core in process, and against a running `inbox serve` over HTTP. It fails
on any failed or errored scenario, and when a refusal of the specification is covered by no
scenario. It also runs acceptance checks against the built binary.

## One gate

`task check` is the gate for every change: it validates `ess/`, checks every generated output
for drift, runs the conformance suite and the acceptance checks, then `cargo fmt --check`,
`cargo clippy` with warnings denied and `cargo test`. The same command runs on every pull
request and on every push to `main`. `unsafe` code is forbidden in every crate of the
workspace.

The numbers of the last gate run and what the checks do not cover are in
[`docs/checks.md`](https://github.com/beyond10x/inbox/blob/main/docs/checks.md).
