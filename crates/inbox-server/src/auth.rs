//! The request signature on the wire, and the verifier port over the key file.

use std::sync::Arc;

use inbox::messaging::{DelegationId, Principal};
use inbox::server::http;
use inbox_core::{Envelope, SignatureVerifier};
use inbox_verify::{Instant, KeyRing, Request, SecretKey, sign_request};

/// The header that carries the key id of the caller's key entry.
pub const KEY_ID_HEADER: &str = "Inbox-Key-Id";
/// The header that carries the RFC 3339 instant the caller signed at, as the signed text.
pub const TIMESTAMP_HEADER: &str = "Inbox-Timestamp";
/// The header that carries the request signature: 88 characters of canonical base64.
pub const SIGNATURE_HEADER: &str = "Inbox-Signature";

/// The three headers that sign a request, for a client to send beside it.
///
/// `target` is the request target in origin form exactly as it is sent: the path, then `?` and the
/// query when there is one (`docs/design/signing.md`, the request). `body` is the exact bytes sent.
#[must_use]
pub fn request_headers(
    method: &str,
    target: &str,
    body: &[u8],
    key_id: &str,
    key: &SecretKey,
    timestamp: &str,
) -> [(&'static str, String); 3] {
    let signature = sign_request(
        &Request {
            method,
            path: target,
            body,
            key_id,
            timestamp,
        },
        key,
    );
    [
        (KEY_ID_HEADER, key_id.to_owned()),
        (TIMESTAMP_HEADER, timestamp.to_owned()),
        (SIGNATURE_HEADER, signature),
    ]
}

/// The value of the header `name` when the request carries it exactly once.
fn single<'a>(request: &'a http::Request, name: &str) -> Option<&'a str> {
    let mut found = request
        .headers
        .iter()
        .filter(|(header, _)| header.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str());
    let value = found.next()?;
    found.next().is_none().then_some(value)
}

/// The principal a request was signed by, verified at `at`; `None` for every refusal, which the
/// service answers alike.
pub(crate) fn authenticate(
    keys: &KeyRing,
    request: &http::Request,
    at: Instant,
) -> Option<Principal> {
    let key_id = single(request, KEY_ID_HEADER)?;
    let timestamp = single(request, TIMESTAMP_HEADER)?;
    let signature = single(request, SIGNATURE_HEADER)?;
    let target = if request.query.is_empty() {
        request.path.clone()
    } else {
        format!("{}?{}", request.path, request.query)
    };
    keys.verify_request(
        &Request {
            method: &request.method,
            path: &target,
            body: &request.body,
            key_id,
            timestamp,
        },
        signature,
        at,
    )
    .ok()
    .cloned()
}

/// The SignatureVerifier port of the specification over the key file, at one instant: the clock
/// reading the request was authenticated at.
#[derive(Debug, Clone)]
pub struct RingVerifier {
    keys: Arc<KeyRing>,
    at: Instant,
}

impl RingVerifier {
    /// The verifier over `keys` at `at`.
    #[must_use]
    pub fn new(keys: Arc<KeyRing>, at: Instant) -> Self {
        Self { keys, at }
    }
}

impl SignatureVerifier for RingVerifier {
    fn verifies(&self, envelope: Envelope<'_>) -> bool {
        match envelope {
            Envelope::Post(post) => self.keys.verify_envelope(post, self.at),
            Envelope::Answer(answer) => self.keys.verify_envelope(answer, self.at),
        }
        .is_ok()
    }

    fn authorizes(&self, envelope: Envelope<'_>, authority: &DelegationId) -> bool {
        self.keys
            .check_delegation(authority, envelope.from(), envelope.on_behalf_of(), self.at)
            .is_ok()
    }
}
