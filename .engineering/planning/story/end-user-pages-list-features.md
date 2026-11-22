---
format: aep.planning-md/3
id: story:end-user-pages-list-features
kind: story
status: implemented
title: The end-user pages list features and limits and name no wave, story or count of the checks
summary: 'website/docs, the landing text and README State are written for a user: features available, missing, planned; the gate numbers move to docs/checks.md; task docs-words holds it.'
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T01:15:19Z", actor: "agent:inbox", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-22T01:15:19Z", actor: "agent:inbox", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-22T01:51:20Z", actor: "agent:inbox", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The pages of the documentation site (`website/docs`), the text of the landing page
(`website/product.json`, slide 3) and the sections State, Known limits and How it is built of
`README.md` are written for a user of the tool. `website/docs/status.md` is titled Features and
lists what is available with its command, the defaults of `inbox serve` with their flags, what
is missing and what is planned. `website/docs/limits.md` states each limit as the user meets
it. No end-user page names a wave, a stage, a story, an epic, an adversary pass or a count of
scenarios or test cases.

What the pages carried for the people and agents who build the inbox moves, unchanged in
content, to `docs/checks.md`: the numbers the gate prints per part, why the run over HTTP
witnesses part of the suite, what the checks do not cover, the closed security findings, and
the story of each open limit.

## Acceptance

`task docs-words`, part of `task check`, exits 1 and prints the matching lines on the pages as
they were at commit 8e306ef (54 lines), and exits 0 on the rewritten pages, where before this
story no check read the pages for such words.

## Context

The operator, 2026-11-22: "a wave is not something the user is concerned with. They just want to
use the tool. They don't care about your waves or your 31 or 111"; "This is end user
documentation"; "Just list the features, the current ones, the missing ones, the upcoming ones,
and so on."

The page tests of the landing page (`website/tests/landing.spec.ts`) held slide 3 to the
scenario counts of the status page. They now hold the count of open security findings to the
limits page and fail on a scenario count in the slide.

## Scope

- `website/docs/*.md`, `website/product.json`, `website/docusaurus.config.ts`
- `website/src/components/PitchSlider/index.tsx`, `website/src/components/PitchSlider/graphics.tsx`
- `website/tests/landing.spec.ts`
- `README.md`, `docs/checks.md`, `AGENTS.md`, `CHANGELOG.md`
- `tools/docs_words.py`, `Taskfile.yml`
