---
format: aep.planning-md/3
id: story:reference-pages-for-the-api-and-the-cli
kind: story
status: implemented
title: Reference pages for the HTTP API, the events and the command line, rendered by the shared docs system
summary: 3 docs pages rendered from the OpenAPI and AsyncAPI projections and from a CLI reference document the binary writes; each links its document.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T01:15:19Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T01:15:19Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-22T10:39:34Z", actor: "agent:inbox", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The documentation site has 3 reference pages in its docs layout, each rendered by a component
of the shared docs system (`@beyond10x/docs-system`) from a generated document, and each with a
link that downloads the document:

- the HTTP API, from `generated/openapi/openapi/inbox-service.yaml`;
- the events, from `generated/asyncapi/asyncapi/inbox-service.yaml`;
- the command line, from a CLI reference document (`b10x-cli-reference/1`) written from the
  clap definition of `crates/inbox-cli` by the test module `src/reference.rs`, committed as
  `generated/cli/inbox.json` and held by a case of `cargo test`, a step of `task check`. The
  released binary gains no command.

## Acceptance

The built site serves the 3 pages and the 3 documents; a page test fails when a page lacks the
operation `post`, the event `MessagePosted` or the command `inbox post`. The case
`the_cli_reference_document_is_current` of `cargo test` fails with
`edited: generated/cli/inbox.json` when a flag of `crates/inbox-cli/src/main.rs` changes
without the document, where before this story the site links the OpenAPI file on GitHub and
names the commands in prose. `task cli-reference` rewrites the document.

## Context

The operator, 2026-11-22: generate the OpenAPI and AsyncAPI files, "link them on the
documentation", "nicely render them via our shared docs system", "Do this with an example for
the inbox service API"; and a CLI documentation, specified and rendered the same way.

The CLI is not written as an ESS CLI binding. A probe on ESS 0.97.1 (`ess specify cli`,
`ess-cli/2`): the binding resolves input, result and error types against `types:` only, so the
unchanged specification refuses `open` (`unresolved input type`); it has no key for an
environment variable, a default, a repeatable flag, a help text per flag or a shared client
flag, and its runtime has exit statuses 0, 1, 2 and 130 where `inbox` has 0 to 4. The shared
component reads a CLI reference document; the docs system also converts a compiled ESS CLI plan
into that document, for a CLI that an ESS binding can describe.

The rendering components and the option of the product site that mounts them are a change of
the repository `beyond10x/docs-system`; this story pins the commit that holds them.

## Scope

- `crates/inbox-cli/src/reference.rs` (new, a test module), `crates/inbox-cli/src/main.rs`
  (the module and 2 test cases)
- `generated/cli/inbox.json`, `Taskfile.yml`
- `website/package.json`, `website/package-lock.json`, `website/docusaurus.config.ts`,
  `website/sidebars.ts`, `website/docs/reference/`
- `website/tests/`
- `AGENTS.md`, `CHANGELOG.md`
