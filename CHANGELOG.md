# Changelog

What changed in each release of the inbox. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`. How a release is cut:
[`docs/releasing.md`](docs/releasing.md).

## [Unreleased]

## [0.1.0] - 2026-11-21

The first release. It ships no binary and publishes no crate: there is no program to run until
the command-line client exists.

### Added

- The specification in ESS (`ess/`): the domain `inbox.messaging` with 2 entities, 9 commands,
  9 views, 10 events and 13 errors, and 23 authored scenarios.
- The generated crate `generated/rust/inbox`, written by ESS 0.96.0 from `ess/` and held to it
  by `task drift`: 85 of 87 capabilities, with the obligations `PostMessage` and `Expire` left to
  the core.
- The core (`crates/inbox-core`): the 2 obligations over a backend, one command at a time per
  store, and the entry an embedding calls per request.
- The in-memory backend (`MemoryBackend` in `crates/inbox-core`). A restart loses every message.
- The signature verifier (`crates/inbox-verify`): Ed25519 over RFC 8785 canonical JSON for
  envelopes, key file entries and requests; the key ring of entries the operator key signed.
  What each signature covers: [`docs/design/signing.md`](docs/design/signing.md).
- The backend test kit (`crates/inbox-backend-testkit`): the checks every backend passes, for
  the 2 storage ports and the command section.
- The HTTP listener (`crates/inbox-server`), a library: request signatures, the read rule, the
  wait on `Pending`, the expiry task and the limits, on a loopback address only. Design:
  [`docs/design/service.md`](docs/design/service.md).
- The conformance gate (`crates/inbox-conformance`, `task conform`): the 92 scenarios ESS
  synthesizes, run against the core in process, all passed; `conformance_status` is
  `inconclusive`.
- The documentation site, <https://beyond10x.github.io/inbox/>, with the specification pages
  ESS generates (`generated/docs`), held to `ess/` by `task check`.

### Known limits

Stated in full on <https://beyond10x.github.io/inbox/docs/limits>.

- Security, open: a principal that holds another principal's signed envelope can post it again
  under its own request signature once the first message is gone
  (`story:post-from-is-the-caller`). Until that is fixed the service must not listen outside
  loopback and must not be used by a real session.
- Security, open: `not-recipient` (403) against `unknown-message` (404) tells any authenticated
  client whether a message id exists (`story:foreign-message-id-answers-as-unknown`).
- Security, open: a dedup key is compared over every sender's messages, so a third principal
  learns of a key or takes it first (`story:dedup-key-scope`).
- Security, open: no record of requests; a signed request is accepted again inside its 60 s
  window.
- No `inbox serve` binary and no command-line client (`story:cli-client`).
- Loopback only, no TLS. One backend, in memory. No second host, no wake-up.
- The conformance status is `inconclusive`: 50 refusals are claimed by no scenario
  (`story:declared-coverage`).
- Gaps in the suite, each a draft story: the order of `PostMessage` refusals
  (`story:post-refusal-order-scenarios`, `story:external-refusal-order-cases`), a read decision
  decided on silence (`story:read-decision-expires-on-silence-scenario`), the fields `Expire`
  leaves unchanged on silence (`story:silence-leaves-the-other-fields-scenario`), one event per
  `Expire` (`story:expire-publishes-once`).
- Two ESS issues are worked around:
  [beyond10x/ess#798](https://github.com/beyond10x/ess/issues/798) and
  [beyond10x/ess#799](https://github.com/beyond10x/ess/issues/799).
- Nothing is deployed.
