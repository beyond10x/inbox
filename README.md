# inbox

One durable, addressable inbox per agent or person, for messages between agents in different
sessions and on different hosts.

Documentation: https://beyond10x.github.io/inbox/

## State

Early. Stage 2 of 6 is in progress: waves 1 to 3 of its 5 are on `main`.

What works on `main`:

- The specification in [ESS](https://github.com/beyond10x/ess) (`ess/`): inboxes, messages,
  their lifecycle, every refusal, and 23 authored scenarios.
- The Rust crate ESS generates from it, the core that implements its 2 obligations, the
  in-memory backend, the signature verifier, the backend test kit, and the HTTP listener (a
  library, loopback only).
- The conformance suite: 92 scenarios pass on ESS's interpreted target and against the core.
  Its status is `inconclusive`, because 50 refusals are claimed by no scenario yet.

What does not exist yet: a program that starts the service (`inbox serve`), the command-line
client, the suite run over HTTP against a running service, a persistent backend, a second host,
wake-up of an idle session, TLS, and any deployment. The first release has no binary.

Numbers and the path that proves each: [what works](https://beyond10x.github.io/inbox/docs/status).

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
- Not ready for real use. See the limits below.

## Known limits

Open security findings, each a draft story in `.engineering/planning/story/`:

1. **Replay of a signed envelope under another caller** (`story:post-from-is-the-caller`). The
   service does not check that an envelope's `from` is the caller that signed the request. A
   principal that holds another principal's signed envelope (every recipient holds the ones it
   received) can post the same bytes again under its own request signature; once the first
   message has been handled and removed, the service answers `posted` again, and the message
   reappears under the original sender's name. Until this is fixed the service must not listen
   outside loopback and must not be used by a real session.
2. **A refusal tells whether a message exists** (`story:foreign-message-id-answers-as-unknown`):
   `not-recipient` (403) against `unknown-message` (404), for any authenticated client.
3. **A dedup key is shared by every sender** (`story:dedup-key-scope`): a third principal learns
   whether a key is in use, or takes it first.
4. **No record of requests**: a signed request is accepted again inside its 60 s window.

Other limits: loopback only and no TLS; the in-memory backend is the only backend, and a restart
loses every message; no binary and no client; the conformance status is `inconclusive`; gaps in
the suite filed as draft stories; two ESS issues worked around; nothing deployed.

All of them, stated in full: [limits](https://beyond10x.github.io/inbox/docs/limits).

## How it is built

The specification comes first, in `ess/`. ESS generates a Rust crate, the API projections and
the specification pages from it, and `task check` fails when any of them differs from a fresh
generation. Hand-written crates implement what the generated crate leaves open. The conformance
suite ESS synthesizes runs on ESS's interpreted target and against the core. Each unit of work
is implemented by one agent and then attacked by a separate adversary pass; a finding that
predates the unit becomes a draft story. Every change goes through `task check`. The plan and
its evidence are in `.engineering/`.

## Where things are

| Path | Contents |
|---|---|
| `ess/` | The specification: inboxes, messages, their lifecycle, every refusal, and 23 scenarios |
| `generated/rust/inbox/` | The Rust crate ESS generates from `ess/`; never edited by hand |
| `generated/{openapi,asyncapi,schema}/` | OpenAPI, AsyncAPI and JSON Schema projections of the specification |
| `generated/docs/` | The specification pages ESS generates; the site shows them |
| `crates/inbox-core/` | The core and the in-memory backend |
| `crates/inbox-verify/` | The signature verifier |
| `crates/inbox-backend-testkit/` | The checks every backend passes |
| `crates/inbox-server/` | The HTTP listener |
| `crates/inbox-conformance/` | The conformance gate: the suite run against the core |
| `crates/inbox-xtask/` | Repository tasks: the drift check of the generated crate |
| `docs/design/` | The design notes: the inbox, signing, the HTTP service |
| `docs/plan/` | The wave plan of each stage and its results |
| `website/` | The documentation site |
| `.engineering/` | The plan |
| `AGENTS.md` | The working rules, written for the coding agents that build the inbox |

## Build and first use

The service is for a loopback address and test keys only while `story:post-from-is-the-caller`
is open (limit 1 above): make keys on the spot, as below, and never give it a key that matters.

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
refuses (naming the error), 2 on wrong usage, 3 when the service cannot be reached, and 4 when
a local file cannot be read or written. `inbox --help` lists the other commands: `close`,
`wait`, `answer`, `refuse`, `outbox`, `collect`, `thread`, `replies`, `repost` and
`keyfile add-delegation`. `inbox post` keeps each envelope in an outbox file
(`$XDG_STATE_HOME/inbox/outbox.jsonl` by default) until it is collected, refused or expired;
`inbox repost` posts the kept ones again after the service was restarted.

## Check

```console
task check
```

validates the specification, fails when anything under `generated/` has drifted from it, runs
the conformance suite on ESS's interpreted target and against the core, then formats, lints,
tests and builds the Rust workspace and the generated crate. It needs
[go-task](https://taskfile.dev), `ess` 0.96.0 and Rust 1.99.1 (`rust-toolchain.toml`).

## Releases

[`CHANGELOG.md`](CHANGELOG.md) lists what each release holds;
[`docs/releasing.md`](docs/releasing.md) is how a release is cut.

## Licence

Apache-2.0, see `LICENSE` and `NOTICE`.
