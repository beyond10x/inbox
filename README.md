# inbox

One durable, addressable inbox per agent or person, for messages between agents in different
sessions and on different hosts.

Documentation: https://beyond10x.github.io/inbox/

## State

Early. The 5 waves of stage 2 of 6 are on `main`, and 2 more waves that close security
findings. Release 0.1.0 holds waves 1 to 3.

What works on `main`:

- The specification in [ESS](https://github.com/beyond10x/ess) (`ess/`): inboxes, messages,
  their lifecycle, every refusal, and 30 authored scenarios.
- The Rust crate ESS generates from it, the core that implements its 2 obligations, the
  in-memory backend, the signature verifier, the backend test kit, and the HTTP listener (a
  library, loopback only).
- The binary `inbox` (`crates/inbox-cli`): `inbox serve` starts the service on a loopback
  address over the in-memory backend, and the other commands are the client. It is built
  from source; no release holds it.
- The conformance suite: 103 scenarios pass on ESS's interpreted target and against the core,
  and every refusal of the specification is claimed by an authored scenario
  (`task coverage`: 93 of 93). ESS reports the run against the core as `passed`.
- The same suite against a running `inbox serve` over HTTP: 25 scenarios pass, and 78
  cannot be witnessed over HTTP and are listed with their reasons
  (`crates/inbox-conformance/unsupported.tsv`).
- 3 acceptance checks against the built binary (`task acceptance`): a signed request and
  its reply between 2 keys, removal after a read and after expiry, a repost after a
  restart.

What does not exist yet: a persistent
backend, a second host, wake-up of an idle session, TLS, and any deployment. No release
holds a binary.

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
- Not ready for principals of different operators to share one service. See the limits below.

## Known limits

Open security findings, each with its story in `.engineering/planning/story/`:

1. **A principal that holds a message id learns whether it is live**
   (`story:message-id-per-sender`, a draft): from a post into an open inbox it learns whether
   the id is live (`id-taken` against `posted`, which stores a message), and it can take a free
   id first, after which the sender's post is refused `id-taken`. This is accepted for
   principals of one operator. A message id unique per sender removes it, and the story is
   required before principals of different operators share one service. A message id is a ULID
   the sender's client makes; the client has no flag to choose one. A post to an address no inbox
   has, or into a closed inbox, answers the same for a held and a free id.
2. **No record of requests**: a signed request is accepted again inside its 60 s window.

The gate: principals of one operator (that operator's own sessions and hosts) may use the
service; a release, a listener outside loopback and a deployment each still need the operator's
own order, and `inbox serve` still refuses an address outside loopback.

Closed on `main` and in no release: the replay of a signed envelope under another caller
(`story:post-from-is-the-caller`; a post or an answer whose `from` is not the caller is refused
`not-sender`), the dedup key of another sender (`story:dedup-key-scope`; a key belongs to its
sender), and a post that told whether a message id of another sender exists for an address no
inbox has or a closed inbox (`story:post-does-not-tell-a-message-id-exists`; `duplicate` answers
the sender's own message only, and `id-taken` is new).

Other limits: loopback only and no TLS; the in-memory backend is the only backend, and a restart
loses every message; no release holds the binary; `inbox repost` delivers a note again that
its recipient already read; the run of the suite over HTTP witnesses 25 of the 103 scenarios;
gaps in the suite filed as draft stories; three ESS issues worked around; nothing deployed.

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
| `ess/` | The specification: inboxes, messages, their lifecycle, every refusal, and 30 scenarios |
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
own (`400`, `401`, `408`, `413`, `503`) keeps it. `inbox repost` posts the kept ones again after
the service was restarted: a note its recipient already read is delivered again.

## Check

```console
task check
```

validates the specification, fails when the scenario files differ from the script that writes
them or anything under `generated/` has drifted from the specification, runs
the conformance suite on ESS's interpreted target, against the core and against a running
`inbox serve`, runs the 3 acceptance checks, then formats, lints,
tests and builds the Rust workspace and the generated crate. It needs
[go-task](https://taskfile.dev), `ess` 0.97.1 and Rust 1.99.1 (`rust-toolchain.toml`).

## Releases

[`CHANGELOG.md`](CHANGELOG.md) lists what each release holds;
[`docs/releasing.md`](docs/releasing.md) is how a release is cut.

## Licence

Apache-2.0, see `LICENSE` and `NOTICE`.
