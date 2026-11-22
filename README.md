# inbox

One durable, addressable inbox per agent or person, for messages between agents in different
sessions and on different hosts.

Documentation: https://beyond10x.github.io/inbox/

## State

Early. Release 0.3.1 runs on one host, on a loopback address, for the principals of one
operator.

Available:

- One binary, `inbox` (`crates/inbox-cli`): `inbox serve` starts the service over the in-memory
  backend, and the other commands are the client. It is built from source; no release attaches
  a binary.
- Notes, requests that state the shape of their reply, and decisions with lettered options,
  each a signed envelope.
- Reading, answering, refusing and collecting a message, threads, expiry, and a decision
  answered on silence with the option it names.
- Ed25519 keys, one key file signed by the operator key, and delegations.
- An HTTP API: one `POST` per command and one `GET` per view.

Missing: a persistent backend, a second host, wake-up of an idle session, TLS, a binary to
download, and any deployment.

Every feature, what is missing and what is planned:
[features](https://beyond10x.github.io/inbox/docs/status).

## What the inbox is

Every message is a signed envelope: a note, a request that says what shape of reply it wants,
or a decision with lettered options. One service holds the envelope for its recipient. Every
client talks to the service only, with its own signing key; backends sit inside the service
behind one storage contract.

A sender posts an envelope to an inbox. The service checks the signature, and the delegation
when the sender speaks for someone else, before it stores anything. The owner of the inbox
reads the message. A note is removed once read. A request or a decision waits for an answer or
a refusal, which the sender collects; the message is removed then. Every message carries
`expires` and is removed when it passes. A decision that names an option for silence is answered
with that option at `expires`.

`generated/openapi/openapi/inbox-service.yaml` is the wire API: one `POST` per command and one
`GET` per view.

## What it is not

- Not a history: a handled or expired message is gone.
- Not a group messenger: there is no group inbox. A request to 11 recipients is 11 envelopes
  under one thread.
- Not encrypted: envelopes are signed, not encrypted.
- Not an identity service: keys and delegations come from one key file signed by the operator
  key, with no delegation chains.
- Not ready for principals of different operators to share one service. See the limits below.

## Known limits

Two security findings are open:

1. **A principal that holds a message id learns whether it is live**: from a post into an open
   inbox it learns whether the id is live (`id-taken` against `posted`, which stores a
   message), and it can take a free id first, after which the sender's post is refused
   `id-taken`. This is accepted for principals of one operator. A message id unique per sender
   removes it, and that change is required before principals of different operators share one
   service. A message id is a ULID the sender's client makes; the client has no flag to choose
   one. A post to an address no inbox has, or into a closed inbox, answers the same for a held
   and a free id.
2. **No record of requests**: a signed request is accepted again inside its 60 s window.

Use the service for the principals of one operator only: that operator's own sessions and
hosts. `inbox serve` refuses an address outside loopback.

Other limits: loopback only and no TLS; the in-memory backend is the only backend, and a restart
loses every message; no release attaches a binary; the first `inbox repost` after a restart
delivers once more a note its recipient read before it; nothing is deployed.

All of them, stated in full: [limits](https://beyond10x.github.io/inbox/docs/limits).

## How it is built

The specification comes first, in `ess/`. ESS generates a Rust crate, the API projections and
the specification pages from it, and `task check` fails when any of them differs from a fresh
generation. Hand-written crates implement what the generated crate leaves open. The conformance
suite ESS synthesizes runs on ESS's interpreted target, against the core and against a running
`inbox serve`. Every change goes through `task check`.

For contributors: the numbers the gate prints, what the checks do not cover and the story of
each open limit are in [`docs/checks.md`](docs/checks.md). The plan and its evidence are in
`.engineering/`.

## Where things are

| Path | Contents |
|---|---|
| `ess/` | The specification: inboxes, messages, their lifecycle, every refusal, and the authored scenarios |
| `generated/rust/inbox/` | The Rust crate ESS generates from `ess/`; never edited by hand |
| `generated/{openapi,asyncapi,schema}/` | OpenAPI, AsyncAPI and JSON Schema projections of the specification |
| `generated/docs/` | The specification pages ESS generates; the site shows them |
| `crates/inbox-core/` | The core and the in-memory backend |
| `crates/inbox-verify/` | The signature verifier |
| `crates/inbox-backend-testkit/` | The checks every backend passes |
| `crates/inbox-server/` | The HTTP listener |
| `crates/inbox-cli/` | The binary `inbox`: `inbox serve` and the command-line client |
| `crates/inbox-conformance/` | The conformance gate: the suite run against the core and against a running `inbox serve` |
| `tests/acceptance/` | The 3 acceptance checks against the built binary |
| `crates/inbox-xtask/` | Repository tasks: the drift check of the generated crate |
| `docs/design/` | The design notes: the inbox, signing, the HTTP service |
| `docs/checks.md` | The numbers the gate prints, what the checks do not cover, the story of each open limit |
| `docs/plan/` | The wave plan of each stage and its results |
| `website/` | The documentation site |
| `.engineering/` | The plan |
| `AGENTS.md` | The working rules, written for the coding agents that build the inbox |

## Build and first use

The service is for a loopback address, and for the principals of one operator: limit 1 above
is open for principals of different operators. Make keys on the spot, as below, and never give
it a key that matters.

### Build

```sh
cargo build --release -p inbox-cli
export PATH="$PWD/target/release:$PATH"
```

builds the one binary, `target/release/inbox`. It needs Rust 1.99.1 (`rust-toolchain.toml`).

### First use

On one machine, in an empty directory. An operator key signs the key file the service reads;
each client has its own key, and its secret key file names its key id and its principal, which
is also the address of its inbox. `inbox key new` writes the secret with mode 0600 and prints
only the public key.

```sh
OPERATOR_PUBLIC_KEY=$(inbox key new --out operator.key)
ALICE_PUBLIC_KEY=$(inbox key new --out alice.key --key-id alice-1 --principal agent:demo/alice)
BOB_PUBLIC_KEY=$(inbox key new --out bob.key --key-id bob-1 --principal agent:demo/bob)
inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id alice-1 --principal agent:demo/alice --public-key "$ALICE_PUBLIC_KEY"
inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id bob-1 --principal agent:demo/bob --public-key "$BOB_PUBLIC_KEY"
inbox serve --listen 127.0.0.1:0 --key-file keys.json --operator-public-key "$OPERATOR_PUBLIC_KEY"
```

`inbox serve` prints the URL it serves on as its first line, then serves until Ctrl-C. In a
second terminal, in the same directory, bob opens his inbox, alice posts a note to it, and bob
reads it:

```sh
# the URL inbox serve printed
export INBOX_URL=http://127.0.0.1:43117
inbox open --secret-key-file bob.key
MESSAGE_ID=$(inbox post --secret-key-file alice.key --to agent:demo/bob --body "Hello, bob")
inbox pending --secret-key-file bob.key
inbox read --secret-key-file bob.key "$MESSAGE_ID"
```

Every command takes `--json` and then prints one JSON document. It exits 1 when the service
refuses (naming the error), 2 on wrong usage (an `INBOX_URL` that is not a loopback address
included), 3 when the service cannot be reached or gives an answer that is neither a success
nor a refusal, and 4 when a local file cannot be read or written. `inbox --help` lists the other
commands: `close`, `wait`, `answer`, `refuse`, `outbox`, `collect`, `thread`, `replies`,
`repost` and `keyfile add-delegation`. `inbox post` keeps each envelope in an outbox file
(`$XDG_STATE_HOME/inbox/outbox.jsonl` by default) until it is collected, refused with a model
error other than `UnknownInbox`, or expired; `id-taken` is such an error, so `post` and `repost`
exit 1 and drop the line. A failed connection or a refusal of the service's
own (`400`, `401`, `408`, `413`, `503`) keeps it. Every answer of the service carries the header
`Inbox-Epoch`, which changes with each start, and `inbox post` records it in `<outbox>.epochs`
beside the outbox file. `inbox repost` is for after the service was restarted: it posts again
the kept envelopes that the current start of the service did not answer, and leaves alone those
it did. A note its recipient read before the restart is delivered once more by the first
`repost` after it.

## Check

```console
task check
```

validates the specification, fails when the scenario files differ from the script that writes
them or anything under `generated/` has drifted from the specification, runs
the conformance suite on ESS's interpreted target, against the core and against a running
`inbox serve`, runs the 3 acceptance checks, then formats, lints,
tests and builds the Rust workspace and the generated crate. It needs
[go-task](https://taskfile.dev), `ess` 0.97.3 and Rust 1.99.1 (`rust-toolchain.toml`).

## Releases

[`CHANGELOG.md`](CHANGELOG.md) lists what each release holds;
[`docs/releasing.md`](docs/releasing.md) is how a release is cut.

## Licence

Apache-2.0, see `LICENSE` and `NOTICE`.
