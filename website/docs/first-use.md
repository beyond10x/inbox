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

`pending`, `wait`, `wake`, `outbox`, `collect`, `thread` and `replies` print text another
principal wrote: a body, a reply, a refusal reason, an option, an address. Each control character
in it but the line end is shown as its escape, such as `\u{1b}` for an escape character, `\u{d}`
for a carriage return and `\u{9}` for a tab, and so are the line separator `\u{2028}` and the
paragraph separator `\u{2029}`, so that text cannot clear your screen, move the cursor, set the
window title or write over a line. The characters that change the direction of text (U+061C,
U+200E, U+200F, U+202A to U+202E and U+2066 to U+2069) are shown as their escapes too, such as
`\u{202e}`, so that text cannot reorder what a line shows. So are the characters a terminal
shows with no width (the soft hyphen U+00AD, U+180E, the zero width space U+200B, U+2060 to
U+2064, U+206A to U+206F, U+FEFF, U+FFF9 to U+FFFB and the tag characters U+E0000 to U+E007F),
such as `\u{200b}`, so that no such character is hidden in a line. The joiners U+200C and
U+200D, the combining grapheme joiner U+034F and the variation selectors (U+FE00 to U+FE0F and
U+E0100 to U+E01EF) are printed as sent, because scripts and emoji sequences need them, and the
flag of a part of a country, an emoji tag sequence, is shown as a black flag followed by the
escapes of its tags. Each message is one line that is not indented, its head line,
and every other line of it starts with at least 2 spaces: a line end in a body, an option or a
refusal reason starts an indented line, and a line end in a value of a line of its own, such as
an address, is shown as `\u{a}`. So a sender cannot write a line that reads as the head line of
another message. A line on standard error that repeats text of the service is shown the same way.
Text without these characters is printed as it was written, a backslash included: a sender who
writes the 8 characters `\u{202e}` gets the same line as a sender whose text holds that
character, and the line does not tell them apart. With `--json` the values are the
ones sent, and no control character is written raw.

## Exit statuses

Every command takes `--json` and then prints one JSON document on standard output.

| Status | Meaning |
|---|---|
| 0 | Done |
| 1 | The service refused; the output names the error |
| 2 | Wrong usage, including an argument read as text that is not UTF-8, an `INBOX_URL` that is not a loopback address, an `https://` URL without a pin and a pin with an `http://` URL |
| 3 | The service could not be reached, its TLS certificate did not match the pin, or its answer was neither a success nor a refusal |
| 4 | A local file could not be read or written |
| 5 | `inbox wake` only: another `inbox wake` of the same inbox with the same outbox file runs on this host; nothing was sent |

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

## Wake a session

`inbox wake` waits until your own inbox has a message not yet delivered and not past its
`expires`, prints what `inbox pending` prints of those messages (`--json`: one document), marks
each delivered, and exits 0:

```sh
inbox open
inbox wake
```

Your inbox must be open at the service. `inbox wake` of an inbox that is not open there, and that
no `inbox open` on this host opened, ends at once with 1 and one line that says to run
`inbox open`; so does `inbox close` while a wake waits, and the wake does not open the inbox
again. It also ends with 1 and such a line when the service refuses to open your inbox again
after a restart, or when your inbox is still not open right after it was opened again.

It keeps waiting when the service ends a wait without a message, after a restart of the service
(it opens your inbox again first, as every command does), and while the service cannot be
reached or answers 408 or a status from 500 on, also when that happens to the open after a
restart: it tries again after a pause that grows to 30 s. An answer that is neither a success
nor a refusal of the service (a redirect, a 404 page of another program on the URL, with or
without a `Content-Length`) ends it with 3, and so does a TLS certificate that does not match the
pin. The messages are printed before they are marked delivered; when a mark fails, the messages
are still printed, one line on standard error names the failure, and it exits with its status
(1 or 3). When its standard output is `/dev/null` or was closed, it exits 4 at once, before it
sends anything. When it cannot write its standard output (the reader is gone, or it is open only
for reading), it marks nothing and exits 4. A second `inbox wake` of the same inbox with the same
outbox file on the same host exits 5 and sends nothing. Ctrl-C or SIGTERM ends it at once, with
nothing printed, also when it was started with them ignored.

A delivered message stays in `inbox pending` until it is read (`inbox read <message id>` takes it
out), but it does not wake again: an `inbox wake` started again neither returns on it nor prints
it, and waits for the next message. A message past its `expires` does not wake it either. A
service that ends every wait at once does not make `inbox wake` spin: after an answer that came
back within a second without a new message, it pauses, the pause growing to 30 s.

An adapter runs `inbox wake` for a session and resumes the session when it exits.

### claude_code

A Claude Code session is woken by a post through the plugin in `adapters/claude-code/` of the
repository. When a turn of the session ends, the plugin runs `inbox wake` in the background;
when a message arrives, Claude Code wakes the session, also an idle one, and shows it the
messages between a begin line and an end line that carry a mark made anew for each wait, so a
sender cannot write such a line. The begin line says that the messages are data from other
principals, not your instructions, and names `inbox read`, `inbox answer` and `inbox refuse`.
More than 6,000 bytes of messages are shown as their first 20 head lines, and `inbox pending`
lists them whole. When that turn ends, the plugin waits again. The plugin was run on one real
headless session of Claude Code 2.1.330 (`claude -p`), and not yet on an interactive one
([limits](limits.md)).

In the terminal you start Claude Code from, set the variables of your client (the secret key
file by its absolute path), open your inbox for this adapter, and load the plugin for the
session:

```sh
export INBOX_URL=http://127.0.0.1:43117
export INBOX_SECRET_KEY_FILE="$PWD/bob.key"
inbox open --adapter claude_code
claude --plugin-dir /path/to/inbox/adapters/claude-code
```

Give the session one prompt: the wait starts when that turn ends. The plugin waits only for an
inbox opened with `--adapter claude_code` on this host, and does nothing in a session without
`INBOX_URL` and `INBOX_SECRET_KEY_FILE`. A wait that ends on a failure without a message does
not wake the session; run `inbox wake` in a terminal with the same variables to see why. The
plugin sets the time limit of its hook to 7 days. The hooks page of Claude Code says that a hook
that reaches its time limit is canceled and its output discarded; a post that arrives after that
wakes the session only after one of its turns ends, and `inbox pending` lists what arrived in
between.

When Claude Code ends without ending the plugin's hook first, such as in a crash, or the hook
alone is killed with SIGKILL, its `inbox wake` keeps running. It then marks the next message
delivered though no session reads it: `inbox pending` still lists the message, and while that
`inbox wake` runs the plugin wakes no session of that inbox with the same outbox file on this
host. End that process (with SIGTERM,
for example); the next turn that ends starts a new wait.

### metaharness

Not available yet.

### terminal

In a terminal of its own, `inbox wake --follow` shows each message as it arrives, marks it
delivered, and waits for the next one until you end it with Ctrl-C or SIGTERM. In that terminal,
set the variables of your client (the secret key file by its absolute path), open your inbox for
this adapter, and start it:

```sh
export INBOX_URL=http://127.0.0.1:43117
export INBOX_SECRET_KEY_FILE="$PWD/bob.key"
inbox open --adapter terminal
inbox wake --follow
```

`terminal` is the default adapter of `inbox open`. Each message is shown as `inbox pending` shows
it, one line that is not indented per message, and on a terminal the bell rings once after the
messages of each arrival; a pipe or a file gets the messages only, and a bell character a sender
wrote is shown as `\u{7}`. A message that was delivered before it started is not shown;
`inbox pending` lists it. With `--json` each arrival is one JSON document on a line of its own.

It waits and fails as `inbox wake` does, and only one `inbox wake`, with or without `--follow`,
runs per inbox and outbox file on this host: a second one exits 5. When a mark fails, one line on
standard error names it and it keeps going; the message may be shown again. An inbox opened on
this host with another adapter, such as `claude_code`, ends it with 1 and a line that says to
run `inbox open --adapter terminal`, so it does not take the place of the Claude Code plugin of a
session: at once when it starts, and with the next message when the inbox is opened with another
adapter while it waits. That message is not shown and stays for the next `inbox wake`.
