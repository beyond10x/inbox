---
format: aep.planning-md/3
id: review-result:adversary-repost-does-not-redeliver-pass-2
kind: review-result
status: active
title: 'Adversary pass 2 on story:repost-does-not-redeliver-a-handled-note (a9bd783): 4 findings (1 warning, 3 notes), all introduced by the first correction'
relations:
- reviews: story:repost-does-not-redeliver-a-handled-note
revision: 1
---
## What was reviewed

The correction `a9bd783` of story:repost-does-not-redeliver-a-handled-note on `e8266d5` (branch
`impl/w8-repost`), by one adversary on Opus, pass 2 of 2, on 2026-11-21. The pass covers the
correction of pass 1 and nothing else.

## Verdict

Broken: 4 findings, all introduced by the correction (1 warning, 3 notes). 1 of the adversary's
3 cases was red on `a9bd783` and green on `e8266d5`.

## Findings

- G1 (warning, introduced; `crates/inbox-cli/src/commands.rs`, `repost`): the correction
  records every epoch once after the loop. A run ended between 2 posts (a signal, a kill)
  records nothing for the posts that arrived. When the recipient reads such a note, the next
  `repost` posts it again and the service answers `posted`: the recipient sees a note it has
  read. `e8266d5` recorded each epoch right after its answer and delivered once. The story's
  Decisions leave open only a note read before a restart.
- G2 (note, introduced; `crates/inbox-cli/src/commands.rs`, `repost`): when a local failure
  meets a refusal or an unreadable line, the exit is 4 and not 1; with `--json` the top-level
  `error` is `local` and the report is under `done`. The refusal is still in the report and on
  standard error. No document stated that precedence.
- G3 (note, introduced; `crates/inbox-cli/tests/epoch.rs`): the case
  `a_post_refused_duplicate_key_records_no_epoch` drives `post`, which neither commit changed;
  it cannot fail for the correction of F2.
- G4 (note, introduced; `crates/inbox-cli/src/commands.rs`, `repost`): the unit's suite holds
  neither the record before a transport failure nor exit 4 on a local failure.

## Changed clients (4 built in a copy of the tree at `a9bd783`)

| Change | The cases of the unit and of pass 1 | The cases of pass 2 |
|---|---|---|
| a: `holds` is true for every refusal named `Duplicate` | fail, 1 case of pass 1 | pass |
| b: no record after the loop | fail, 10 cases | fail |
| c: the transport arm records nothing | pass | fail, 1 case |
| d: the local failure is dropped (exit 0) | pass | fail, 1 case |

## What held

- `keep` and `remove` never write the epochs file from an empty value.
- An envelope removed by another process, or expired, before the record gets no epoch.
- 2 lines with one message id: both are posted, and the epochs file names the id once.
- An epoch is taken only from an answer of the running start that says the id is stored; no
  sequence was found after which an envelope has the current epoch and the service never
  stored it.
- `Epochs::parse` keeps its rules: an id named twice has no epoch, and an empty id has none.

## Cases

3 cases in `crates/inbox-cli/tests/adversary_w8_repost2.rs`, each 0 to 2 s: 1 red on `a9bd783`
(G1, with an ignore attribute) and 2 green that fail the changed clients c and d.
