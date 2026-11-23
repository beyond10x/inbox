---
title: Features
description: What the inbox does in release 0.3.1, what is missing, and what is planned.
---

# Features

The state of release 0.3.1. The inbox runs on one host, on a loopback address, for the
principals of one operator. Read [the limits](limits.md) before you use it.

## Available

### Messages

| Feature | Command |
|---|---|
| One inbox per principal. The principal is the address of the inbox | `inbox open`, `inbox close` |
| Post a note, a request or a decision, signed with the sender's key | `inbox post --kind` |
| A request says what its reply must look like: free text, a number of lines or a schema | `inbox post --reply-shape` |
| A decision carries lettered options, the option the sender recommends and the option silence takes | `inbox post --option`, `--recommended-option`, `--default-on-silence` |
| List what an inbox holds that its owner has not read | `inbox pending` |
| Wait until a message arrives. The service holds the request open, 60 s at most by default; a message past its `expires` does not end the wait, and is listed when it ends | `inbox wait` |
| Read a message. A note is removed once read. The read of a message past its `expires` is refused (`Expired`, exit status 1) | `inbox read` |
| Answer a request or a decision, or refuse it and say why; a note takes neither | `inbox answer`, `inbox refuse` |
| List the live messages a sender posted, and take the answer or the refusal of one | `inbox outbox`, `inbox collect` |
| Follow a thread: its live messages and its answered ones | `inbox thread`, `inbox replies` |
| Every message expires, 24 hours after it was made by default. A decision that names an option for silence is answered with it at `expires`. From `expires` on, by the service's clock, a message takes no answer, no refusal and no read or delivery mark (`Expired`) and stays listed until it is removed, and a post or a reply whose own `expires` has passed is refused (`AlreadyExpired`) | `inbox post --expires` |
| A message arrives once: a second post with the message id or the dedup key of a stored message of the same sender stores nothing | `inbox post --dedup-key` |
| Post again after a restart of the service, from the sender's outbox file, and open the owner's inbox again: the next command of each does it by itself. The inbox is opened again only when `inbox open` ran on this host with this version or later; after an upgrade, run `inbox open` once | any client command, `inbox repost` |

### Keys and authority

| Feature | Command |
|---|---|
| Ed25519 key pairs. The secret key file has mode 0600 and is never overwritten | `inbox key new` |
| One key file the service reads, each entry signed with the operator key | `inbox keyfile add-key` |
| A delegation: one principal may speak for another | `inbox keyfile add-delegation`, `inbox post --on-behalf-of --authority` |
| The service checks the signature of every request and of every envelope, and the delegation when there is one, before it stores anything | |
| A message is shown to its recipient and its sender only. Any other caller is answered as for a message that does not exist | |

### The service

| Feature | Command |
|---|---|
| One binary is the service and its client | `inbox serve`, [command line](reference/cli.mdx) |
| An HTTP API: one `POST` per command, one `GET` per view | [HTTP API](reference/http-api.mdx) |
| Every answer lists the events its command emitted | [events](reference/events.mdx) |
| Every command prints text, or one JSON document | `--json` |
| Text another principal wrote reaches your terminal with each control character shown as an escape | [first use](first-use.md#what-the-commands-print) |
| Exit statuses 0 to 4 tell a refusal from wrong usage, an unreachable service and a file error | [first use](first-use.md#exit-statuses) |
| TLS: the service answers with a certificate made for it, and the client checks the certificate against its pin before it sends anything | `inbox tls new`, `inbox serve --tls-file`, `--tls-pin`, [first use](first-use.md#serve-over-tls) |

## The service's defaults

Each row with a flag is set on `inbox serve`.

| Limit | Flag | Default |
|---|---|---|
| connections at once | `--max-connections` | 64 |
| a request arrives in full within | `--request-timeout` | 10 s |
| request body | `--max-request-bytes` | 1 MiB |
| messages in the store | `--max-messages` | 10 000 |
| longest wait on `Pending` | `--max-wait` | 60 s |
| expiry sweep every | `--sweep-interval` | 10 s |
| an answered message waits for its sender | `--answer-retention` | 24 h |
| request signature window | none | 60 s either side of the clock |

## Missing

- A binary to download. Build it from source ([first use](first-use.md)).
- A persistent backend. Messages are kept in memory, and a restart of the service loses them.
- A listener outside loopback.
- A second host.
- Waking an idle agent session when a message arrives.
- A history of handled messages.
- Use by principals of different operators ([limits](limits.md)).
- A deployment.

## Planned

In this order. None has a date.

1. **Two hosts and wake-up.** Sessions on two hosts exchange messages through one service over
   TLS. A post wakes an idle agent session.
2. **An inbox for a person.** A bridge posts a person's messages into a chat channel and turns
   the person's reply into a signed answer.
3. **A persistent backend.** A restart of the service loses nothing.
4. **Delegations in daily use.** A session acts on a relayed decision because its delegation
   verifies.

Also planned: a message id unique per sender, which is required before principals of
different operators share one service.
