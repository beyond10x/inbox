---
format: aep.planning-md/3
id: approval-record:d89-the-chat-bridge-posts-under-a-standing-grant-of-connectors
kind: approval-record
status: draft
title: 'D89: the chat bridge posts under a standing grant of connectors, option B on the operator''s word'
relations:
- decides: story:the-chat-bridge-posts-a-pending-message
- decides: story:the-operator-runs-the-chat-bridge-on-the-real-channel
revision: 1
---
## Decision

Option B of D89: the chat bridge posts under a standing grant of `connectors`, a new feature
built in the `connectors` repository. The bridge passes no proof per write
(`--approval-file`) and runs no `connectors approvals` command.

The question. story:the-chat-bridge-posts-a-pending-message, Open 5, was accepted at its
default A (i): the operator sets the write policy of the chat connection so that the post
operation runs without a proof per write. The public documentation of `connectors` says that
every write is a mutation that requires an approval, issued for the digest of its whole input;
a policy that admits a write without it does not exist. The options put to the operator:

- A: the bridge issues the proof itself for each post (`approvals prepare`, `approvals issue`,
  then `invoke`), with an approval key of its own on the host.
- B: `connectors` gets a standing grant: an entry that admits one operation to one fixed
  target without a proof per write, bounded in time and in writes per hour, issued and revoked
  by the operator. The bridge then calls `connectors invoke` as the story says.
- C: the bridge posts through the chat provider's interface itself, without `connectors`.

## Who decided

The operator, on 2026-11-23 at about 11:35 CET, in the terminal of the host operator's
session: "please pick the most correct way here, even if it delays some things, we dont build
workarounds here". The host operator's session picked B on that word and relayed it: B is the
narrower grant, built in `connectors`; A is the workaround B would later replace.

## What it grants

- The bridge is built and tested against a stand-in `connectors` that answers in the shape the
  `connectors` session gave for the standing grant: `connectors --output json invoke
  <connector>/<instance> chat.postMessage --input-stdin --idempotency-key <key>`; success is
  exit status 0 with one JSON document on standard output
  (`result.mutation.classification` `applied`); every failure is exit status 1 with one JSON
  document on standard error (`error.data.code`: `approval_required`, `rate_limited` with
  `retry_after_seconds`, `idempotency_conflict`, `unavailable`); a usage error is exit status
  2.
- The stories of the bridge are built in their order while the feature of `connectors` is not
  released.

## What it does not grant

- story:the-operator-runs-the-chat-bridge-on-the-real-channel waits for the release of
  `connectors` that carries the standing grant. Until then a released `connectors` refuses
  the bridge's post, and the status page of the website lists the bridge as planned.
- The standing grant itself (`connectors approvals grant-standing`), the link of the chat
  connection and the channel are the operator's to make when that story is reached.
