---
title: Design notes
description: The three design notes of the inbox, with a link to each.
---

# Design notes

The design lives in three notes in the repository. Where a note and the specification in `ess/`
disagree, `ess/` holds.

## The inbox

[`docs/design/inbox.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/inbox.md):
the problem, the shape (one service, backends behind one storage contract, signed envelopes),
the model of two entities, nine commands and nine views, when a message is removed, the eight
decisions taken while modelling, and what ESS 0.97.1 cannot express.

## Signing

[`docs/design/signing.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/signing.md):
what each of the three signatures covers (an envelope, an entry of the key file, a request to
the service), the context strings and canonical bytes, the wire encodings, what a verifier
refuses, the rules of the key file, the 60 s window of a request signature, and what version 1
leaves out.

## The HTTP service

[`docs/design/service.md`](https://github.com/beyond10x/inbox/blob/main/docs/design/service.md):
one request in order from reading to the answer, the request signature headers, who the caller
is, the read rule, the wait on `Pending`, the expiry task, the limits and their defaults, and
binding to loopback and stopping.
