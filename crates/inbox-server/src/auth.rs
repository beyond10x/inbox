//! The request signature on the wire, and the verifier port over the key file.

use std::sync::Arc;

use inbox::messaging::{DelegationId, Principal};
use inbox::server::http;
use inbox_core::{Envelope, SignatureVerifier};
use inbox_verify::{Instant, KeyRing, Request, SecretKey, sign_request, sign_request_for};

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

/// The header that names the inbox a request acts for: the text of the signed member
/// `on_behalf_of`.
pub const ON_BEHALF_OF_HEADER: &str = "Inbox-On-Behalf-Of";

/// [`request_headers`] for a request that acts for the inbox `on_behalf_of` names, or for none:
/// with it, the signature covers the sixth member and the fourth header carries its text.
#[must_use]
pub fn request_headers_for(
    method: &str,
    target: &str,
    body: &[u8],
    key_id: &str,
    key: &SecretKey,
    timestamp: &str,
    on_behalf_of: Option<&str>,
) -> Vec<(&'static str, String)> {
    let signature = sign_request_for(
        &Request {
            method,
            path: target,
            body,
            key_id,
            timestamp,
        },
        on_behalf_of,
        key,
    );
    let mut headers = vec![
        (KEY_ID_HEADER, key_id.to_owned()),
        (TIMESTAMP_HEADER, timestamp.to_owned()),
        (SIGNATURE_HEADER, signature),
    ];
    if let Some(on_behalf_of) = on_behalf_of {
        headers.push((ON_BEHALF_OF_HEADER, on_behalf_of.to_owned()));
    }
    headers
}

/// Every value of the header `name`, in the order they came.
fn values<'a>(request: &'a http::Request, name: &str) -> Vec<&'a str> {
    request
        .headers
        .iter()
        .filter(|(header, _)| header.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
        .collect()
}

/// The value of the header `name` when the request carries it exactly once.
fn single<'a>(request: &'a http::Request, name: &str) -> Option<&'a str> {
    match values(request, name).as_slice() {
        [value] => Some(*value),
        _ => None,
    }
}

/// Who a request was authenticated as: the principal of its key, and the inbox it acts for when
/// it named one that a live delegation of that principal covers.
#[derive(Debug, Clone)]
pub(crate) struct Authenticated {
    /// The principal of the request's key.
    pub(crate) principal: Principal,
    /// The person whose inbox the request acts for: the text of `Inbox-On-Behalf-Of`.
    pub(crate) on_behalf_of: Option<Principal>,
}

/// Who a request was signed by, verified at `at`, and the inbox it acts for; `None` for every
/// refusal, which the service answers alike: also a request that sends `Inbox-On-Behalf-Of` twice,
/// or names an inbox no live delegation of the pair covers (`docs/design/service.md`, "The
/// request signature").
pub(crate) fn authenticate(
    keys: &KeyRing,
    request: &http::Request,
    at: Instant,
) -> Option<Authenticated> {
    let key_id = single(request, KEY_ID_HEADER)?;
    let timestamp = single(request, TIMESTAMP_HEADER)?;
    let signature = single(request, SIGNATURE_HEADER)?;
    let on_behalf_of = match values(request, ON_BEHALF_OF_HEADER).as_slice() {
        [] => None,
        [value] => Some(*value),
        _ => return None,
    };
    let target = if request.query.is_empty() {
        request.path.clone()
    } else {
        format!("{}?{}", request.path, request.query)
    };
    let principal = keys
        .verify_request_for(
            &Request {
                method: &request.method,
                path: &target,
                body: &request.body,
                key_id,
                timestamp,
            },
            on_behalf_of,
            signature,
            at,
        )
        .ok()?
        .clone();
    Some(Authenticated {
        principal,
        on_behalf_of: on_behalf_of.map(|text| Principal(text.to_owned())),
    })
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
