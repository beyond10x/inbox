---
format: aep.planning-md/3
id: story:no-unmapped-rule-in-the-specification
kind: story
status: active
title: The specification holds no UNMAPPED marker
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:58:01Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T04:58:01Z", actor: "agent:inbox", revision: 3}
---
## Outcome

`ess/` holds no `UNMAPPED:` marker, and the gate fails on one.

The operator, 2026-11-22: "There must be no unmapped".

On `main` at `8e9c3ab` the specification marks 2 rules as not expressible on ESS 0.97.1
(`ess/domains/messaging.yaml:416` and `:755`). Probes on ESS 0.97.1 show a spelling for both:

- **The letters name options of the decision.** `unknown-option` of `PostMessage` becomes a
  declared `when:` guard with `distinct` and `exists` over `decision.options`, in place of the
  `external:` refusal. The inputs `decision` and `default_on_silence` gain an `example:`;
  without them synthesis writes 57 to 59 scenarios instead of 113.
- **`MessageExpired` names the sender and the thread.** `expired` of `Expire` becomes a set
  effect (`instances: {where: message_id == input.message_id}`, `each: changed`) whose event
  reads `{changed: …}`; the event gains the fields `to` (the sender) and `thread`.

Both together: `ess specify validate` accepts, synthesis writes 112 scenarios (38 authored), 92
refusals, all covered, and the interpreted target passes 112 of 112.

The option part of `shape-mismatch` of `Answer` becomes a declared guard too
(`option-mismatch`, a `when_subject` over the decision's options); the reply shape part stays
`external:`.

## Acceptance

`grep -rn "UNMAPPED" ess/` prints nothing and `task check` exits 0; with an `UNMAPPED:`
comment planted in `ess/domains/messaging.yaml`, `task check` exits non-zero and names the
line (state transition: the 2 markers leave the specification and a new one is refused).

A core that publishes `MessageExpired` without `to` or `thread` fails a case of the suite
or of `crates/inbox-core`; a post whose `recommended_option` names no option is refused
`unknown-option` by the declared guard on the interpreted target and by the core.

## Decisions

- `MessageExpired` gains 2 fields on the wire (`to`, `thread`); a reader of 0.2.0 ignores
  members it does not know.
- The 2 `external:` refusals that stay (`signature-invalid`, `authority-invalid`) are not
  markers: a guard cannot verify a signature, and the delegation spelling loses 51 scenarios
  on ESS 0.97.1 (probe `11a`).
- Synthesis takes about 32 s instead of 12 s with the 2 examples. The cause is a defect of
  synthesis (a candidate for an `Optional` of a newtype ignores the newtype's invariant); it is
  filed on beyond10x/ess and named beside the examples.

## Scope

- `ess/domains/messaging.yaml`, `ess/scenarios/` (3 files), the scenario source
- `generated/` (every projection), `website/src/generated/animation.json`
- `crates/inbox-core/` (obligations of `PostMessage` and `Expire`), `crates/inbox-conformance/`
  (`unsupported.tsv`, `unsupported-core.tsv`), tests of `crates/inbox-server` and
  `crates/inbox-cli` that read `MessageExpired`
- `Taskfile.yml` (the check for a marker), `AGENTS.md`, `CHANGELOG.md`, `docs/checks.md`,
  `docs/design/inbox.md`
