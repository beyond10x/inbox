---
title: First use
description: Build the binary, start the service on a loopback address, and send one message between two test keys.
---

# First use

One binary, `inbox`, starts the service and is its client. No release attaches a binary: build
it from source.

:::warning[Test keys and loopback only]

The service is for a loopback address, and for the principals of one operator: security
finding 1 of the [limits](limits.md) is open for principals of different operators. Make keys on the spot,
as below, and never give it a key that matters. `inbox serve` refuses a `--listen` address
outside loopback, and the client refuses an `INBOX_URL` outside loopback.

:::

## Build

```sh
git clone https://github.com/beyond10x/inbox
cd inbox
cargo build --release -p inbox-cli
export PATH="$PWD/target/release:$PATH"
```

This builds `target/release/inbox`. It needs Rust 1.99.1 (`rust-toolchain.toml`).

## Keys and the service

In an empty directory. An operator key signs the key file the service reads. Each client has a
key of its own; its secret key file names its key id and its principal, which is also the
address of its inbox. `inbox key new` writes the secret file with mode 0600 and prints only the
public key.

```sh
OPERATOR_PUBLIC_KEY=$(inbox key new --out operator.key)
ALICE_PUBLIC_KEY=$(inbox key new --out alice.key --key-id alice-1 --principal agent:demo/alice)
BOB_PUBLIC_KEY=$(inbox key new --out bob.key --key-id bob-1 --principal agent:demo/bob)
inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id alice-1 --principal agent:demo/alice --public-key "$ALICE_PUBLIC_KEY"
inbox keyfile add-key --file keys.json --operator-secret-key-file operator.key --key-id bob-1 --principal agent:demo/bob --public-key "$BOB_PUBLIC_KEY"
inbox serve --listen 127.0.0.1:0 --key-file keys.json --operator-public-key "$OPERATOR_PUBLIC_KEY"
```

`inbox serve` prints the URL it serves on as its first line and serves until Ctrl-C. It keeps
every message in memory: a restart loses them.

## Serve over TLS

The service speaks plain HTTP unless you give it a TLS file. To serve over TLS, run these two
commands instead of the last one above. `inbox tls new` makes a key and a certificate for the
service, writes both to one file with mode 0600 (it never overwrites a file) and prints the pin:
the SHA-256 of the certificate's public key, 44 characters of base64. Copy the pin: the client
terminal below needs it. It is not secret.

```sh
inbox tls new --out tls.json
inbox serve --listen 127.0.0.1:0 --key-file keys.json --operator-public-key "$OPERATOR_PUBLIC_KEY" --tls-file tls.json
```

The first line `inbox serve` prints is now an `https://` URL. A client checks the certificate
against the pin before it sends anything. A certificate with another public key ends the command
with exit status 3, and nothing of the request is sent. An `https://` URL without a pin, and a
pin with an `http://` URL, are wrong usage (exit status 2): a client given a pin never talks
plain HTTP. The address is a loopback address either way.

What the pin does not protect against:

- Anyone who can read `tls.json` can pose as the service to every client that has the pin.
- There is no revocation. A client accepts the key of its pin until you give it another pin.
- Each `inbox tls new` makes a new key. A service started with it has a new pin, and every client
  needs the new pin.

## One message

In a second terminal, in the same directory. Bob opens his inbox, alice posts a note to it, and
bob reads it. First set `INBOX_URL` to the URL `inbox serve` printed:

```sh
export INBOX_URL=http://127.0.0.1:43117
```

Over TLS, that URL starts with `https://`, and you also set `INBOX_TLS_PIN` to the pin
`inbox tls new` printed (or pass `--tls-pin` to each command):

```sh
export INBOX_URL=https://127.0.0.1:43117
export INBOX_TLS_PIN='the pin inbox tls new printed'
```

Then, either way:

```sh
inbox open --secret-key-file bob.key
MESSAGE_ID=$(inbox post --secret-key-file alice.key --to agent:demo/bob --body "Hello, bob")
inbox pending --secret-key-file bob.key
inbox read --secret-key-file bob.key "$MESSAGE_ID"
```

`inbox --help` lists the other commands: `close`, `wait`, `answer`, `refuse`, `outbox`,
`collect`, `thread`, `replies`, `repost` and `keyfile add-delegation`.

## What the commands print

`pending`, `outbox`, `collect` and `thread` print text another principal wrote: a body, a reply,
a refusal reason, an option, an address. Each control character in it but the line end is shown
as its escape, such as `\u{1b}` for an escape character, `\u{d}` for a carriage return and
`\u{9}` for a tab, so that text cannot clear your screen, move the cursor, set the window title
or write over a line. A line on standard error that repeats text of the service is shown the
same way. Text without control characters is printed as it was written. With `--json` the values
are the ones sent, and no control character is written raw.

## Exit statuses

Every command takes `--json` and then prints one JSON document on standard output.

| Status | Meaning |
|---|---|
| 0 | Done |
| 1 | The service refused; the output names the error |
| 2 | Wrong usage, including an argument read as text that is not UTF-8, an `INBOX_URL` that is not a loopback address, an `https://` URL without a pin and a pin with an `http://` URL |
| 3 | The service could not be reached, its TLS certificate did not match the pin, or its answer was neither a success nor a refusal |
| 4 | A local file could not be read or written |

## The outbox file

`inbox post` keeps each signed envelope in an outbox file before it sends it
(`$XDG_STATE_HOME/inbox/outbox.jsonl` by default; `--outbox` or `INBOX_OUTBOX` names another).
An envelope leaves the file when its answer is collected, when the service refuses it with an
error of the specification other than `UnknownInbox`, or when it expires (24 hours after it was
made, by default). `id-taken` is such an error, so `post` and `repost` exit 1 and drop the line.
`inbox collect` prints the message as the service held it when it removed it, in the form
`inbox outbox` prints a row (`--json`: under `message`). An answer that carries no such record,
as a service of release 0.3.0 or older answers, makes `collect` exit 3, and the envelope stays.

Every answer of the service carries the header `Inbox-Epoch`: 32 hex digits that change with each
start of the service. When a post arrives, `inbox post` records that value in the epochs file
beside the outbox file (`<outbox>.epochs`, one line per envelope).

`inbox open` records the adapter it opened the inbox with beside the outbox file
(`<outbox>.opened`); `inbox close` removes it, and puts it back when the close did not reach the
service or the service refused it with anything but `UnknownInbox`.

A restart of the service loses every message and every open inbox. The client takes care of it
by itself: when the answer to any command comes from a new start, the command, after its own
request, opens the caller's inbox again with the adapter it was last opened with, when it was
opened on this host and not closed since, and posts again, once each, the kept envelopes the new
start has not answered, when one of them is recorded at an earlier start or has not reached the
service (its post failed in transport, was refused with `UnknownInbox`, was answered `408` or
`503` by a service that did not take it, or was stopped before an answer). The command prints its own answer first, one line on standard error names the new
start and what was done, and with `--json` the one document is printed after the reconnect and
carries the member `reconnected`. A step that does not succeed, such as a post the service
refuses with `UnknownInbox` because the recipient has not opened its inbox again yet, keeps the
envelope, and the next command tries again.

`inbox repost` does the same posts when typed. It asks the service for its epoch and posts again
the kept envelopes that the epochs file does not record at that epoch. An envelope the current
start has already answered is left alone and not listed, so a second `repost` in the same start
posts nothing. A note its recipient read before the restart is delivered once more by the first
repost after it, typed or not ([limits](limits.md)).
