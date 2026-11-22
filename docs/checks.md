# State of the checks

For the people and agents who build the inbox. The numbers the gate prints, what the checks
do not cover, and the story of each open limit. The end-user pages under `website/docs` carry
none of this: they list features and limits ([features](https://beyond10x.github.io/inbox/docs/status), [limits](https://beyond10x.github.io/inbox/docs/limits)).

## On `main`

Numbers from 2026-11-22, ESS 0.97.3, Rust 1.99.1. Case counts are the passed cases of
`task test` (cargo-nextest) in one run of `task check`.

| Part | What it does | Number | Proof |
|---|---|---|---|
| The specification | The domain `inbox.messaging`: inboxes, messages, their lifecycle, every refusal | 2 entities, 9 commands, 9 views, 10 events, 14 errors, 2 actors; 38 authored scenarios | [`ess/`](../ess), `task spec` |
| The generated crate | Model types, command behaviours, view queries, storage and context ports, route table and JSON codecs, written by ESS from `ess/` | 86 of 88 capabilities generated; 2 obligations left to hand-written code (`PostMessage`, `Expire`) | [`generated/rust/inbox/`](../generated/rust/inbox), `task drift` |
| The core | The 2 obligations over a backend; one command at a time per store | 133 cases | [`crates/inbox-core/`](../crates/inbox-core) |
| The in-memory backend | Both storage ports in memory; part of the core crate (`MemoryBackend`) | in the 133 cases above, and in the test kit's | [`crates/inbox-core/`](../crates/inbox-core) |
| The verifier | Ed25519 signatures over RFC 8785 canonical JSON for envelopes, key file entries and requests; the key ring | 63 cases | [`crates/inbox-verify/`](../crates/inbox-verify), [`docs/design/signing.md`](../docs/design/signing.md) |
| The backend test kit | The checks every backend passes: the 2 storage ports and the command section | 29 cases | [`crates/inbox-backend-testkit/`](../crates/inbox-backend-testkit) |
| The HTTP listener | A library: request signatures, the read rule, the wait on `Pending`, the expiry task, the limits; loopback only | 60 cases, none ignored | [`crates/inbox-server/`](../crates/inbox-server), [`docs/design/service.md`](../docs/design/service.md) |
| The binary `inbox` | `inbox serve` starts the listener over the in-memory backend on a loopback address; the other commands are the client: signed requests, the outbox file and its epochs file, exit statuses 0 to 4 | 86 cases, none ignored | [`crates/inbox-cli/`](../crates/inbox-cli), [first use](https://beyond10x.github.io/inbox/docs/first-use) |
| Conformance, interpreted | The suite ESS synthesizes from `ess/`, run on ESS's interpreted target | 112 scenarios: 112 passed, 0 failed, 0 error, 0 unsupported | `task conform-interpreted` |
| The conformance gate | The same suite run against the core in process, the report written by ESS and held to the suite | 112 scenarios: 112 passed, 0 failed, 0 error, 0 unsupported; `conformance_status: passed` | [`crates/inbox-conformance/`](../crates/inbox-conformance), `task conform` |
| The service over HTTP | The same suite against a running `inbox serve`, a fresh one per scenario; every request and envelope signed, each answer held to the HTTP status of the generated contract | 112 scenarios: 31 passed, 0 failed, 0 error, 81 unsupported, each listed with its reasons; `conformance_status: failed` (see below) | [`unsupported.tsv`](../crates/inbox-conformance/unsupported.tsv), `task conform` |
| Acceptance | 3 checks against the built binary: a signed request and its reply between 2 keys; a read note and an expired request in no view; a repost after a restart, and a second repost that posts nothing | 3 passed | [`tests/acceptance/`](../tests/acceptance), `task acceptance` |
| The conformance runner | Reads the suite, runs it against both targets, holds each report | 79 cases, 1 ignored | [`crates/inbox-conformance/`](../crates/inbox-conformance) |
| Declared coverage | Every refusal of the specification is claimed by an authored scenario; `task check` fails on one that is not | 93 refused generated scenarios, 93 covered; the suite reports `conformance_status: passed` on the interpreted target | `task coverage` |
| Repository tasks | The drift check of the generated crate, the scenario checks, the animation writer, the words of the end-user pages, the path decision of the CI job and the test step. The cases of the drift check read one synthesis of `ess/` per run of `task test`; the 2 that change `ess/` run their own | 79 cases | [`crates/inbox-xtask/`](../crates/inbox-xtask) |

## What wave 8 added

Wave 8 of stage 2 closes gaps in the checks that the adversary passes of wave 3 found, and
narrows one limit of the client.

- The service sends the header `Inbox-Epoch` on every answer, and `inbox repost` leaves alone
  a kept envelope that the epochs file records at the epoch the service names now. The limit
  that stays is on the [limits](https://beyond10x.github.io/inbox/docs/limits) page.
- 2 authored scenarios: a decision its recipient read is decided on silence at `expires`, and
  the row a decision decided on silence leaves keeps its other fields. Both are unsupported
  over HTTP, as steps of the Sweeper.
- 6 authored scenarios hold the order in which `PostMessage` refuses when 2 refusals apply:
  the signature, the option, the delegation and the reply shape are refused before any row is
  read, and the 5 row refusals answer in one order. All 6 run over HTTP. The suite has 38
  authored scenarios and 111 in all.
- Cases of the core: one occurrence of each event `Expire` hands back, the order of the 3
  external refusals of `PostMessage`, a read decision decided on silence, and the fields
  `Expire` leaves unchanged.

## Why 31 of 112 over HTTP

Against the core in process all 112 scenarios pass, and ESS reports `passed`. Against the
running service a scenario runs only where every step can be sent and witnessed over HTTP.
81 cannot: a caller whose principal and inbox differ (70 scenarios; over HTTP both
are the principal of the caller's key), a step as the Sweeper (37; only the service's own
expiry task is the Sweeper), a step as no actor (9; a request without a signature is
answered `401`), a claim on the stored `signature` (1; the run signs each envelope with
its own key), one delegation id for two pairs of sender and principal (2; an id names one
entry of the key file). A scenario can have several reasons. ESS reports a run with an
unsupported scenario as `failed`; `task conform` fails unless the unsupported scenarios and their
reasons are those of the list, and no scenario failed or errored.

## Open limits and their stories

The limits are stated on the limits page of the site. Their stories are in
`.engineering/planning/story/`.

- Security finding 1 (a principal that holds a message id learns whether it is live):
  `story:message-id-per-sender`, a draft. Required before principals of different operators
  share one service.
- Security finding 2 (a signed request is accepted again inside its 60 s window): no story is
  filed. Source: `docs/design/service.md`, "The request signature".
- `inbox repost` delivers a note once more that was read before a restart:
  `story:repost-does-not-redeliver-a-handled-note`.
- Loopback only, no TLS, no second host, no wake-up: `epic:two-hosts-and-wake`.
- One backend, in memory: `epic:jetstream-backend`.

## Security findings, closed in release 0.2.0

Closed in waves 6 and 7 of stage 2 and in release 0.2.0. Release 0.1.0 holds the listener
without them.

- **A signed envelope can be posted again under another caller**
  (`story:post-from-is-the-caller`). `post` and `answer` are refused `not-sender` (403) when the
  envelope's `from` is not the caller's principal, as a rule of the specification and before
  every other refusal. Before, a principal that held another principal's signed envelope could
  post the same bytes again under its own request signature, and once the first message was
  handled and removed the service answered `posted` again. The case
  `adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal` in
  `crates/inbox-server/tests/adversary.rs` runs in `task check`.
- **5 commands told a third principal whether a message exists**
  (`story:foreign-message-id-answers-as-unknown`): `read`, `collect`, `refuse`, `answer` and
  `mark-delivered` answer a caller that is neither the recipient nor the sender of a message
  with `unknown-message` (404), the answer of an id no message carries. Before, they answered
  that caller 403. The case
  `adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message` in the same file
  runs in `task check`.
- **A dedup key was shared by every sender** (`story:dedup-key-scope`, wave 7). `duplicate-key`
  compares the dedup key among the stored messages of the post's sender. A sender that uses
  one key for two inboxes is still refused the second post. Before, a key of any sender refused
  the post, so a third principal learned from a post whether a key was in use and could take
  another sender's key first.
- **A post told whether a message id of another sender exists, for an address no inbox has or a
  closed inbox** (`story:post-does-not-tell-a-message-id-exists`, wave 7). `duplicate` answers the
  sender's own stored message only, and the new refusal `id-taken` comes last of the row
  refusals: `unknown-inbox`, `duplicate`, `duplicate-key`, `inbox-closed`, `id-taken`. Before,
  `duplicate` answered for any stored id and came before `unknown-inbox`. The part that stays is
  finding 1 above.

## What the checks do not cover

- **The run over HTTP witnesses 31 of the 112 scenarios.** All 112 pass against the core in
  process. Against a running `inbox serve`, 81 are unsupported and listed with their
  reasons in
  [`unsupported.tsv`](../crates/inbox-conformance/unsupported.tsv)
  (the reasons are above). ESS reports that run as `failed`, and the
  gate holds it to the list.
- **The run over HTTP and the 3 acceptance checks do not exercise every rule of the
  listener.** Every request they send is validly signed, with keys and delegations that
  do not lapse, and the run reads a view as every key and joins the rows. A service
  without the read rule, without the check of the request signature or its 60 s window,
  without the lapse of a key or a delegation, without the binding of a key to `from` or
  of a delegation to its pair, or one that serves `Messages` and `Due` to a client,
  passes both (8 of the 12 changed services the adversary built). The tests of
  `crates/inbox-server` and `crates/inbox-verify` fail on each of the 8, and
  `task check` runs them.
- Rules the specification cannot state on ESS 0.97.3 (who may read which rows, the blocking
  read, and others listed in
  [`docs/design/inbox.md`](../docs/design/inbox.md))
  are implemented by the service and tested outside the suite.
- Gaps in the suite found by the adversary passes, each a draft story:
  - `story:post-order-scenarios-hold-the-input-guards`: the suite holds the signature, the
    option, the delegation and the reply shape of a post before every row refusal, and the 5
    row refusals in one order. No scenario holds `no-authority`, `already-expired` or
    `decision-without-options` before a row refusal, or `not-sender` before `unknown-inbox` and
    `inbox-closed`; a core that answers the row refusal first there passes `task conform`
    (cases of the core hold 3 of these pairs). A `duplicate` rule that also compares
    `on_behalf_of` passes it too.
  - `story:silence-scenarios-claim-the-whole-message`: the scenarios on a decision decided on
    silence claim 10 members of the stored message, from one start state. A core that changes
    another member, or a claimed one only for a delivered or read decision, passes
    `task conform`; cases of the core hold both. No scenario and no case holds the other
    messages of the thread.
  - `story:expiry-scenarios-hold-the-remaining-cases`: 5 behaviours of `Expire` that no scenario
    holds (cases of the core do); a core that gets one wrong passes `task conform`: a
    read decision that names no silence option is decided on silence, a decision is decided 1
    second before `expires`, a delivered request is decided on silence, `thread` of
    `DecisionTakenOnSilence` is the message id, a refused decision that names a silence option
    is counted as open.
- ESS issues the code works around or waits for:
  - [beyond10x/ess#798](https://github.com/beyond10x/ess/issues/798): the `PLAN.md` that ESS
    writes states an order of refusals that the interpreted target does not follow. The
    contract text of `PostMessage` in `generated/rust/inbox/PLAN.md` states such an order; the
    core follows the order the interpreted target answers, not the text.
  - [beyond10x/ess#799](https://github.com/beyond10x/ess/issues/799): for a cleared optional
    member of a view row the suite claims `null`, while the generated Rust server leaves the
    member out. The runner treats an absent top-level member and `null` as one value; one
    adversary case that needs the exact comparison is ignored until the issue is closed. On
    ESS 0.97.3 the rule is still needed: with an exact comparison the run against the core
    fails `PostMessage/outcome/posted` on a row of `Due`.
  - [beyond10x/ess#804](https://github.com/beyond10x/ess/issues/804): with the guard `not-sender` in the specification,
    synthesis writes no stored message under it, and 49 generated scenarios leave the suite
    (measured on ESS 0.96.0); on ESS 0.97.3, without the example, 15 generated scenarios are
    refused and `task coverage` fails. Until it is fixed the input `from` of `PostMessage`
    carries the example value `principal-262144`, the value synthesis gives its first caller
    and not an address anybody holds. It shows in the generated OpenAPI documents and the JSON Schema of
    `PostMessage` (3 files) and changes no decoder. With it, 19 generated scenarios lost a
    second run under a second caller (also measured on ESS 0.96.0).
  - beyond10x/ess issue 809 ([link](https://github.com/beyond10x/ess/issues/809)): synthesis
    draws a candidate for an `Optional` of a newtype without the newtype's invariant. Under the
    guard `unknown-option` it then refused `posted` on ESS 0.97.1 and 54 generated scenarios
    left the suite; on ESS 0.97.3 it refuses `unknown-option` and 16 leave, and
    `task coverage` fails. Until it is fixed the inputs `decision` and `default_on_silence` of
    `PostMessage` carry an example each. On ESS 0.97.1 they made synthesis take about 4 times
    as long as before the guard; on ESS 0.97.3 synthesis takes as long with them as without.
