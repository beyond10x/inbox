---
format: aep.planning-md/3
id: story:public-docs-and-release
kind: story
status: implemented
title: A public repository with a documentation site, a changelog and a release process
summary: README for people, a project site with the specification pages, a changelog, a written release process and the first release.
scope:
- confidence: cited
  path: .github/workflows/b10x-docs-site.yml
- confidence: cited
  path: .github/workflows/pages.yml
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: docs/releasing.md
- confidence: cited
  path: generated/docs
- confidence: cited
  path: website
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T11:22:09Z", actor: "agent:inbox", revision: 12}
- {from: "proposed", to: "active", at: "2026-11-21T11:22:10Z", actor: "agent:inbox", revision: 13}
- {from: "active", to: "implemented", at: "2026-11-21T11:59:25Z", actor: "agent:inbox", revision: 14, decided_on: {"recorded":{"test_result":3,"verification":1}}}
---
## Outcome

`beyond10x/inbox` is a public repository that a reader can judge without asking anyone: a
`README.md` written for people, a documentation site at `https://beyond10x.github.io/inbox/`
that says what the inbox is, what works on `main`, which limits are open and what the
specification states (pages generated from `ess/`, held to it by the gate), a `CHANGELOG.md`, a
written release process (`docs/releasing.md`), and the first release cut from a green `main`.

## Acceptance

After this story `npm --prefix website run build` builds the site with no broken link, `task
check` fails when a specification page under `generated/docs` differs from `ess/`, the README and
the site's limits page both name the open replay finding (story:post-from-is-the-caller), and
`docs/releasing.md` states every step from the version to the GitHub Release; before it the
repository has no site, no changelog and no release process, and its README says the service is
not written.

## Context

The operator decided on 2026-11-21 that this repository is public like the other public
repositories of the organization, with public documentation, the usual gates and a first
release. That replaces the rule "private, no release, no tag" of the stage 2 approval. "No
deployment" stays.

The order the operator set: scan the history and every branch that becomes public (done before
this story: 29 commits, 414 text blobs, 0 findings); documentation; the gates and required
checks on `main`; then the visibility change, read back; then the release process and the first
release.

The site follows the route the organization's public repositories documented since October
use: a Docusaurus site under `website/` built on `@beyond10x/docs-system`, built by
`.github/workflows/pages.yml` and published by the organization's project-site workflow, called
from `.github/workflows/b10x-docs-site.yml`. The repository carries no `b10x.docs.yaml`.

The first release has no binary: the command-line client is story:cli-client. A release is an
annotated version tag written by the organization bot and a GitHub Release whose notes say what
works and which limits are open.

Not in this story: the visibility change, the rulesets and the Pages settings. They are settings
of the repository on the forge, taken by the coordinator after this story's files are on `main`.

## Scope

- `website/` (new)
- `.github/workflows/pages.yml`, `.github/workflows/b10x-docs-site.yml`,
  `.github/workflows/release.yml` (new)
- `generated/docs/` (new), `Taskfile.yml`
- `README.md`, `AGENTS.md`
- `CHANGELOG.md`, `docs/releasing.md` (new)
