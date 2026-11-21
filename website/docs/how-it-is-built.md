---
title: How it is built
description: Specification first, generated code held by a drift check, a conformance suite, an adversary pass per unit, one gate.
---

# How it is built

## The specification comes first

Every behaviour is written in [ESS](https://github.com/beyond10x/ess) in `ess/` before code
exists for it: the entities, commands, views, events, every refusal, and 32 authored scenarios.
`task spec` validates it. A rule ESS 0.97.1 cannot state carries an `UNMAPPED:` marker in `ess/`
and is listed in `docs/design/inbox.md`; it is implemented by the service and tested outside the
suite.

## Generated code is held to it

From `ess/`, ESS writes:

- the Rust crate `generated/rust/inbox`: model types, command behaviours, view queries, storage
  and context ports, the route table and JSON codecs;
- the OpenAPI, AsyncAPI and JSON Schema projections under `generated/`;
- the [specification pages](/specification/) of this site, under `generated/docs`.

Nothing under `generated/` is edited by hand. `task check` fails when any of it differs from a
fresh generation from `ess/`. The generated crate leaves two obligations (`PostMessage` and
`Expire`), which `crates/inbox-core` implements outside it.

## The conformance suite

ESS synthesizes a conformance suite from `ess/`: 105 scenarios on ESS 0.97.1. `task check` runs
it three times: on ESS's interpreted target, against the core in process, and against a
running `inbox serve` over HTTP. It fails on any failed or errored scenario. Against the core
all 105 pass and ESS reports `passed`. Over HTTP 25 pass and 80 cannot be witnessed; they are
listed with their reasons, and the gate fails when the list and the run differ (see
[what works](status.md)). `task check` also fails when a refusal of the specification is
claimed by no authored scenario (`task coverage`), and it runs 3 acceptance checks against
the built binary (`task acceptance`).

## Each unit is attacked after it is written

Work is planned as stories and run in waves. Each story is implemented by one agent, then read
by a separate adversary agent that tries to break it: it writes cases that fail, and the
implementor fixes what is found. Each unit of stage 2 had two adversary passes. A finding that
was there before the unit is filed as a draft story; the open ones are on the
[limits](limits.md) page, and the adversary's case stays in the tests, marked ignored, until
its story lands.

## One gate

`task check` is the gate, for every change: validate `ess/`, check every generated output for
drift, run the conformance suite on the 3 targets and the 3 acceptance checks, then
`cargo fmt --check`, `cargo clippy` with
warnings denied, `cargo test`, and a build of the generated crate. The same command runs on
every pull request and on every push to `main` (`.github/workflows/check.yml`). `unsafe` code is
forbidden in every crate of the workspace.

## The record

The plan, the stories, the review results and the evidence of each gate run are kept in the
repository, in [`.engineering/`](https://github.com/beyond10x/inbox/tree/main/.engineering),
written with the `aep` tool. The wave plan of stage 2 and the result of each wave are in
[`docs/plan/stage-2.md`](https://github.com/beyond10x/inbox/blob/main/docs/plan/stage-2.md).
Every commit on `main` is made by the organization's bot after the gate is green.
