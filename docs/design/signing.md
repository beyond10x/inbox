# Design: signing, version 1

Status: written before the code of `crates/inbox-verify`, which implements it. Where this note and
`ess/domains/messaging.yaml` disagree, `ess/` holds.

What is signed, in which bytes, and what a verifier refuses. Three kinds of signature exist: an
**envelope** (a post or a reply, signed by the sender's key), an **entry** of the key file (a key
entry or a delegation entry, signed by the operator key), and a **request** to the service (signed
by the caller's key). All three are Ed25519 (RFC 8032, pure Ed25519: neither Ed25519ph nor
Ed25519ctx).

## The bytes a signature covers

```
signed bytes = context string || canonical JSON
```

The canonical JSON is the JSON Canonicalization Scheme (RFC 8785, JCS) of one JSON object, in
UTF-8. The context string fixes the kind:

| Kind | Context string (ASCII, ends in one line feed, byte 0x0A) | The object |
|---|---|---|
| envelope | `inbox.v1.envelope` + LF | the envelope without its member `signature` |
| entry | `inbox.v1.entry` + LF | the key entry or delegation entry without its member `operator_signature` |
| request | `inbox.v1.request` + LF | the five request members below, and a sixth when the request acts for an inbox |

No context string is a prefix of another, and each ends in a byte that JCS never writes first (a
JCS object starts with `{`), so a signature of one kind never verifies as another.

### Envelope

The object is the input of `PostMessage` or of `Answer` exactly as the generated encoder of the
`server` feature writes it (`encode_command_inbox_messaging_post_message`,
`encode_command_inbox_messaging_answer`), with the member `signature` removed, then put through
JCS. Consequences:

- An absent optional member (`authority`, `reply_shape`, `decision`, `default_on_silence`,
  `option`) is omitted, never written as `null`. The decoder reads a received `null` as absent, so
  the verifier signs over the omitted form, and a signer that writes `null` into its canonical
  bytes produces a signature that does not verify.
- The verifier canonicalizes the decoded envelope, not the received text: whitespace, member order,
  escape spelling and the spelling of a number (`3.0` for `3`) on the wire do not change the bytes.
- Timestamps are text and are signed as text: `2026-01-01T00:00:00Z` and `2026-01-01T00:00:00z`
  are two different envelopes.
- `signature.key_id` is not in the signed bytes (the whole member `signature` is removed). A key id
  is bound to the envelope through the check that the key's principal is `from` (below).
- A post and a reply share the context string; their members differ (`message_id`, `to`, `kind`
  against `in_reply_to`, `reply_id`), so the canonical bytes of one are never those of the other.
- **A receiver refuses a text in which any object, at any depth, carries a member name twice,
  before it decodes the text**, with `inbox_verify::duplicate_member` over the value the generated
  reader (`json::parse`) returns. That reader keeps every member, duplicates included, and the
  generated decoders take the first: without the check, a post carrying `body` twice decodes to
  the first `body`, verifies, and one signature covers two texts that differ to a reader that
  keeps the last.

**Numbers.** The only number in either envelope is `reply_shape.value` of a `lines` reply shape,
an `Integer`. No type of either envelope holds a number that is not an integer. JCS writes a
number as an IEEE 754 double, so an integer beyond 2^53 − 1 in magnitude would be rounded and two
envelopes would share one canonical form: signing and verifying both refuse an envelope holding an
integer outside −(2^53 − 1) … 2^53 − 1.

### Entry

The object is the key entry or delegation entry as the generated encoder writes it
(`encode_inbox_messaging_key_entry`, `encode_inbox_messaging_delegation_entry`), with the member
`operator_signature` removed, through JCS. A key entry signs `key_id`, `principal`, `public_key`,
`valid_until`; a delegation entry signs `agent`, `delegation_id`, `on_behalf_of`, `valid_until`
(JCS order). The two share the context string; their member names differ, so one never verifies as
the other.

### Request

The object has these members, all strings: the five of every request, and `on_behalf_of` only
in a request that acts for an inbox.

| Member | Value |
|---|---|
| `body_sha256` | SHA-256 of the exact body bytes as sent, standard base64 with padding (44 characters). An empty body hashes the empty string |
| `key_id` | the key id the request names |
| `method` | the HTTP method as sent, case-sensitive (`POST`, not `post`) |
| `on_behalf_of` | only in a request that acts for an inbox: the address of that inbox, as text |
| `path` | the request target in origin form as received: the path, then `?` and the query when there is one. Not decoded, not normalized |
| `timestamp` | the RFC 3339 instant the caller sends, as text |

How these travel (header names) is story:http-service's to fix; the signed bytes do not depend on
it (`docs/design/service.md`, "The request signature": `on_behalf_of` travels in
`Inbox-On-Behalf-Of`). JCS writes the members in the order of the table, so `on_behalf_of` sits
between `method` and `path`.

A request that acts for no inbox has no member `on_behalf_of`, absent and not empty: its object
has the five members and its signed bytes are byte for byte those of a request before the member
existed, so every signature a client of an earlier version makes still verifies. A request that
acts for an inbox has the six. An absent member and an empty one are two different requests, and
an empty one is refused (below): it is never read as absent.

The value of `on_behalf_of` is the text of an `Address` (`ess/domains/messaging.yaml`, a newtype
of `String` with no invariant): non-empty UTF-8, without a leading or trailing space or tab. The
service holds no other grammar for an address, and needs none here: the value counts only when a
live delegation names a principal with the same text, byte for byte (below), so a text that names
no principal of the key file is refused by that check. Version 1 checks no form such as
`agent:<host>/<session-name>`.

`inbox_verify::request_bytes_for` writes the object of five or six members (`request_bytes` is
it with no member `on_behalf_of`), and `sign_request_for` signs it.

## Encoding on the wire

| Item | Encoding |
|---|---|
| a signature (`signature.value`, `operator_signature`, a request signature) | the 64 bytes of the Ed25519 signature, standard base64 alphabet with `=` padding: 88 characters |
| a public key (`public_key`, the pinned operator key) | the 32 bytes of the Ed25519 public key, the same base64: 44 characters |

Base64 must be canonical: the text decodes to exactly 64 (or 32) bytes, and encoding those bytes
again gives the same text (unused trailing bits are zero). No whitespace, no URL-safe alphabet, no
missing padding.

## What a verifier refuses

The verifier is strict (`verify_strict` of `ed25519-dalek`). It refuses:

- a signature whose base64 is not canonical or is not 64 bytes;
- a signature whose `S` is not a canonical scalar (not below the group order) or whose `R` is not
  the canonical encoding of the point it recomputes;
- a signature whose `R` is a point of small order;
- a public key that is not 32 bytes of canonical base64, does not decode to a curve point, is not
  the canonical encoding of that point, or is a point of small order (a weak key). Such a key entry
  is ignored when the key file is read, even when the operator signed it.

Nothing is accepted unverified. Every failure is a refusal; there is no warning mode.

### An envelope (`signature-invalid`)

In this order, the first failure refuses:

1. `signature.key_id` names a key entry that the pinned operator key signed (unknown otherwise).
2. That entry is not lapsed at the instant of the check.
3. The entry's `principal` is the envelope's `from`.
4. `signature.value` is a canonical signature encoding.
5. The envelope has a canonical form (it holds no integer outside −(2^53 − 1) … 2^53 − 1).
6. The signature verifies over the envelope's signed bytes under the entry's public key.

### A delegation (`authority-invalid`)

A delegation entry lets its `agent` do two things for its `on_behalf_of` until `valid_until`:
speak for that principal in every envelope it sends (`authority`), and act as the owner of the
inbox whose address has the text of that principal, in every request that names that inbox
(`on_behalf_of` of the request, below; decision 4 of `docs/design/inbox.md`). One entry grants
both; no entry grants one without the other. The chain length is one: the delegation itself,
signed by the operator key, and nothing it points to.

Checked when an envelope names `authority`:

1. The id names a delegation entry that the pinned operator key signed (unknown otherwise).
2. Its `agent` is the envelope's `from` and its `on_behalf_of` is the envelope's `on_behalf_of`
   (otherwise it names another pair).
3. It is not lapsed at the instant of the check.

Whether an envelope that speaks for another principal names a delegation at all is the
specification's guard `no-authority`, decided before the verifier is asked.

Checked when a request names an inbox (step 4 of a request, below), by the pair and not by an id:
the request names no delegation. The signer acts for the inbox when the verifier holds at least
one delegation entry whose `agent` is the principal of the request's key, whose `on_behalf_of` has
the same text as the request's `on_behalf_of`, and which is not lapsed at the instant the request
is authenticated at (`valid_until` exclusive, "Instants"). Which entry counts when several name the
pair: any one live entry is enough, and a lapsed entry beside it refuses nothing; the request
admits no particular entry, so the `authority` of an envelope in its body is checked by itself
(above), against any held entry of its own pair. A reply in a request that names an inbox speaks
for the principal of that inbox: one whose `on_behalf_of` is any other principal, the agent
included, is refused `authority-invalid` before its `authority` is looked up, a rule of the
service and not of the verifier (`docs/design/service.md`, "The caller"). Version 1 has no scope
narrower than the pair ("Left out of version 1"), so an agent with a live delegation answers any
message of the person's inbox in the person's place, one it sent itself included, and the record
says the person answered; a sender that must not answer its own question holds no delegation of
the person it asks. An entry the verifier ignores (not signed by the
operator, an ambiguous id, "The key file") counts for nothing here either. A request that names
the signer's own address is no exception: it needs a live entry whose `agent` and `on_behalf_of`
are both the signer. `KeyRing::acts_for` is this check.

### A request

1. The key id names a signed key entry that is not lapsed.
2. The timestamp is an RFC 3339 instant within the window below.
3. The signature is a canonical encoding and verifies over the request's signed bytes: the five
   members when the request carries no `on_behalf_of`, the six when it carries one.
4. When the request carries `on_behalf_of`: the text is not empty and has no leading or trailing
   space or tab, and a delegation of the pair (the key's principal, that text) is live at the
   instant of the check ("A delegation", above). This step runs only after step 3 passed, so only
   the holder of a live key makes the verifier look a delegation up.

The caller is then the key entry's principal, and its inbox the address the request's
`on_behalf_of` names, or, without one, the address with the text of that principal
(`docs/design/service.md`, "The caller").

Each of these is refused, as a failure of step 3 or 4, and the service answers each as every
other refusal of a request (`docs/design/service.md`, "The request signature"):

- signed without the member `on_behalf_of` and sent with the header that carries it: the six
  members the service signs over are not the five that were signed;
- signed with the member and sent without the header: the five are not the six;
- sent with a header whose value differs from the signed member, in any byte;
- the header sent twice, with the same value or with two: it is read neither as the first nor as
  the last;
- the header with an empty value, or with one the reader trims to empty: never read as absent,
  also when the request was signed without the member;
- a value that differs from the signed member only by leading or trailing white space: the
  reader trims the header of every white space at its ends (`str::trim`, Unicode white space, not
  only a space or a tab), so it differs from the signed member;
- no live delegation of the pair: none held, every one lapsed at the instant, or each one held for
  another `agent` or another `on_behalf_of`.

A header value that is not UTF-8 never reaches these checks: the generated reader refuses such a
header line in step 1 of the service (`400`), as it refuses it in every other header.

`KeyRing::verify_request_for` runs the 4 steps (step 4 refuses `Refusal::NotActingFor`);
`KeyRing::verify_request` is it for a request without the member.

### The key file

The key file is read from its text by `KeyRing::parse`, through the generated codecs
(`decode_inbox_messaging_key_file`). `KeyRing::parse` refuses the file whole when it is not JSON,
when any object in it, at any depth, carries a member name twice (`duplicate_member`, checked
before decoding), or when it does not decode. `KeyRing::new` takes a file already decoded and
cannot see a duplicate member: a caller that decodes the text itself runs `duplicate_member` on
the parsed value first.

**An id is never reused.** A renewal, a new key and a changed delegation each take a new
`key_id` or `delegation_id`. The operator removes the entry they replace from the file, or lets
it lapse by its own `valid_until`. An earlier entry the operator signed stays validly signed, so
an entry put back beside its successor under the same id would make the id ambiguous (step 2).

Of a decoded file, each list (`keys`, `delegations`) is read in three steps, and an ignored entry
is kept with its id and the reason for the operator:

1. An entry whose `operator_signature` is not a canonical signature encoding, or does not verify
   under the pinned operator public key over the entry's signed bytes, is ignored. It counts for
   nothing after this step: an entry without the operator's signature cannot make the id of a
   held entry ambiguous. What holds is that nobody without the operator key adds an entry or
   changes what an id means. Whoever can write the key file can remove an id, by deleting its
   entry or by putting back another entry the operator signed for that id (both are then
   ignored, step 2); the verifier fails closed in both cases.
2. An id carried by more than one entry left after step 1 is ambiguous, and every entry that
   carries it is ignored, whatever step 3 would say about each (one with a refused public key
   still makes the other ignored). It is never resolved by order.
3. Of the rest, an entry is ignored when (key entry) its `public_key` is refused as above, or its
   `valid_until` is not an RFC 3339 instant (the decoder already refuses such a file; this reaches
   only a key file built in code).

The verifier holds only the entries left. It reads no clock and drops no entry for being lapsed:
lapse is decided at the instant of each check.

`inbox keyfile add-key` and `inbox keyfile add-delegation` change the key file under an exclusive
lock of `<file>.lock`: they remove whatever is at `<file>.<pid>.new` (a link there is not
followed, a directory there is a failure), create that file anew with the mode of the key file
they replace (0666 narrowed by the umask for a new key file, which holds no secret bytes), write
the whole file to it and rename it over the key file.

The key id of a client's secret key file is the one its requests name in a header: `inbox key new`
refuses a `--key-id` with a control character (`char::is_control`: C0, DEL, C1) as wrong usage
(exit 2) and writes no file, and a client command given a secret key file whose `key_id` holds one
exits 4 with one line that names the file, before any connection; U+2028, U+2029 and the
characters that change the direction of text pass. Not checked by
story:a-key-id-with-a-control-character-is-refused-where-it-is-made: the `principal` of a key, the
key id of `inbox keyfile add-key`, the delegation id of `inbox keyfile add-delegation`, an empty
key id and one with white space around it.

## Instants

Every check takes the instant as an argument; the library reads no clock.

An instant is read from RFC 3339 text as the generated reader admits it (`timestamp_at`): the
offset is applied, so instants compare on the UTC timeline; a fraction is truncated to
nanoseconds; a leap second (`:60`) reads as the last nanosecond of the second before it.

**Lapsed.** An entry with `valid_until` *v* is live at instant *t* when *t* < *v*, and lapsed when
*t* ≥ *v*. The bound is exclusive: at `valid_until` itself the entry is already lapsed. This is
the specification's reading of `expires` (`expires <= created` is already expired).

**The request window.** A request with timestamp *s*, checked at instant *t*, is accepted when
*t* − 60 s ≤ *s* ≤ *t* + 60 s. Both bounds are inclusive. A request older than 60 s is refused as
stale, one more than 60 s ahead of the instant is refused as early.

## Left out of version 1

- Revocation lists: a short `valid_until` and an edit of the key file replace them. The edit
  removes the entry; a renewal is a new entry under a new id, never the old id signed again.
- Delegation chains: a delegation is one entry, signed by the operator key.
- A delegation scope narrower than the pair (`agent`, `on_behalf_of`): an entry lets its agent
  speak for its principal in every envelope and act for that principal's inbox in every request,
  and no entry grants one of the two alone.
- Hardware-backed keys.
- Payload encryption.
- A replay cache for requests: a signed request may be repeated inside its 60 s window. Posts are
  idempotent by `message_id` and `dedup_key`.
- More than one operator key, and rotation of it: the one operator public key is pinned in the
  service's configuration.
- Re-verifying a reply after it is stored: `Reply` on a message keeps no `in_reply_to` and no
  `expires`, which the signed bytes of an `Answer` cover. The service verifies the `Answer` when it
  arrives.

General agent identity belongs to Mandate.
