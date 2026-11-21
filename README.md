# inbox

One durable, addressable inbox per agent or person, for messages between agents in different
sessions and on different hosts. Every message is a signed envelope: a note, a request that says
what shape of reply it wants, or a decision with lettered options.

The inbox is one service with interchangeable backends, specified in
[ESS](https://github.com/beyond10x/ess) before any code. This repository holds the specification
and its conformance suite. The service is not written yet.

| Path | Contents |
|---|---|
| `ess/` | The specification: inboxes, messages, their lifecycle, every refusal, and 23 scenarios |
| `generated/` | OpenAPI, AsyncAPI and JSON Schema projections of the specification |
| `docs/design/inbox.md` | The design and its stages |
| `.engineering/` | The plan |

```console
task check
```

validates the specification, fails when `generated/` has drifted from it, and runs the conformance
suite ESS synthesizes from `ess/` on ESS's interpreted target. It needs
[go-task](https://taskfile.dev) and `ess` 0.96.0.

## What a message goes through

A sender posts an envelope to an inbox. The service checks the signature, and the delegation when
the sender speaks for someone else, before it stores anything. The owner of the inbox reads the
message. A note is removed once read. A request or a decision waits for an answer or a refusal,
which the sender collects; the message is removed then. Every message carries `expires` and is
removed when it passes. A decision that names an option for silence is answered with that option
at `expires`.

`generated/openapi/openapi/inbox-service.yaml` is the wire API: one `POST` per command and one
`GET` per view.

## Licence

Apache-2.0, see `LICENSE` and `NOTICE`.
