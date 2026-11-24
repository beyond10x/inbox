---
title: What the inbox is
description: One durable, addressable inbox per agent or person, behind one service with interchangeable backends.
slug: /
---

# What the inbox is

The inbox gives every agent and every person one durable, addressable inbox, for messages
between agents in different sessions and on different hosts. Every message is a signed
envelope: a note, a request that says what shape of reply it wants, or a decision with lettered
options.

It is early. Release 0.7.0 runs on one host, on a loopback address, and security findings are
open. Read [the features](status.md) and [the limits](limits.md) first, then try it:
[first use](first-use.md).

## The problem

Agents run in separate sessions, in separate harnesses and on separate hosts. A message between
two of them travels over whatever the harness offers: it is not durable, it has no address that
outlives a session, and the receiver cannot check who sent it or on whose authority.

## The shape

- **One service** (`inbox serve`). Every client talks to the service only, with its own signing
  key. No client talks to a backend.
- **Interchangeable backends** inside the service, behind one storage contract. The first
  backend keeps messages in memory. A persistent backend is planned.
- **Signed envelopes.** The service checks the signature, and the delegation when the sender
  speaks for someone else, before it stores anything. A message that fails is refused, and the
  refusal is the sender's answer.
- **Specified first.** The behaviour is an [ESS](https://github.com/beyond10x/ess)
  specification in `ess/`. Code is generated from it or held to it; see
  [how it is built](how-it-is-built.md) and [the specification pages](/specification/).

## What a message goes through

1. A sender posts an envelope to an inbox.
2. The service checks the signature, and the delegation when there is one, before it stores
   anything.
3. The owner of the inbox reads the message.
4. A note is removed once read. A request or a decision waits for an answer or a refusal, which
   the sender collects; the message is removed then.
5. Every message carries `expires` and is removed when it passes. A decision that names an
   option for silence is answered with that option at `expires`.

Nothing is kept as history. The wire API is one `POST` per command and one `GET` per view
([HTTP API](reference/http-api.mdx)).

## What the inbox is not

- No history of handled messages. A handled or expired message is gone.
- No group inbox. A request to 11 recipients is 11 envelopes under one thread.
- No payload encryption. Envelopes are signed, not encrypted.
- No delegation chains. A delegation is one entry, signed by the operator key.
- No general identity service. Keys and delegations come from one key file signed by the
  operator key.

It is also not deployed anywhere, and not ready for principals of different operators to share
one service (see [the limits](limits.md)).
