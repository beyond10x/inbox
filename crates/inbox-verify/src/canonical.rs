//! The bytes each kind of signature covers (`docs/design/signing.md`), and the signing side.

use std::fmt;

use inbox::messaging::{Answer, DelegationEntry, KeyEntry, PostMessage, Principal, Signature};
use inbox::server::json::{self, Value};
use inbox::server::wire;
use sha2::{Digest, Sha256};

use crate::jcs::{self, CanonError};
use crate::keys::{SecretKey, encode};

/// The context string of an envelope signature.
pub const ENVELOPE_CONTEXT: &[u8] = b"inbox.v1.envelope\n";
/// The context string of a key entry or delegation entry signature.
pub const ENTRY_CONTEXT: &[u8] = b"inbox.v1.entry\n";
/// The context string of a request signature.
pub const REQUEST_CONTEXT: &[u8] = b"inbox.v1.request\n";

/// The largest integer magnitude an envelope may hold: beyond it a double rounds, and JCS with it.
pub const MAX_SAFE_INTEGER: i64 = (1 << 53) - 1;

/// An envelope that could not be put into canonical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    /// RFC 8785 refused the value.
    Jcs(CanonError),
    /// An integer outside ±(2^53 − 1), which JCS would round.
    UnsafeInteger(String),
    /// The generated encoder wrote text its own reader refused.
    Encoding(String),
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jcs(error) => error.fmt(f),
            Self::UnsafeInteger(spelling) => {
                write!(f, "the integer {spelling} is outside ±(2^53 − 1)")
            }
            Self::Encoding(reason) => {
                write!(f, "the encoded envelope does not read back: {reason}")
            }
        }
    }
}

impl std::error::Error for CanonicalError {}

/// A signed envelope: the input of `PostMessage` or of `Answer`.
pub trait Envelope {
    /// The envelope's `signature`.
    fn signature(&self) -> &Signature;
    /// The envelope's `signature`, to be set by [`sign_envelope`].
    fn signature_mut(&mut self) -> &mut Signature;
    /// The principal that sends it.
    fn from(&self) -> &Principal;
    /// The principal whose word it is.
    fn on_behalf_of(&self) -> &Principal;
    /// Writes the envelope as JSON, with the generated encoder.
    fn encode(&self, out: &mut String);
}

impl Envelope for PostMessage {
    fn signature(&self) -> &Signature {
        &self.signature
    }
    fn signature_mut(&mut self) -> &mut Signature {
        &mut self.signature
    }
    fn from(&self) -> &Principal {
        &self.from
    }
    fn on_behalf_of(&self) -> &Principal {
        &self.on_behalf_of
    }
    fn encode(&self, out: &mut String) {
        wire::encode_command_inbox_messaging_post_message(self, out);
    }
}

impl Envelope for Answer {
    fn signature(&self) -> &Signature {
        &self.signature
    }
    fn signature_mut(&mut self) -> &mut Signature {
        &mut self.signature
    }
    fn from(&self) -> &Principal {
        &self.from
    }
    fn on_behalf_of(&self) -> &Principal {
        &self.on_behalf_of
    }
    fn encode(&self, out: &mut String) {
        wire::encode_command_inbox_messaging_answer(self, out);
    }
}

/// The fields of a request a caller signs. `path` is the request target in origin form, with the
/// query; `timestamp` is RFC 3339 text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Request<'a> {
    /// The HTTP method, as sent.
    pub method: &'a str,
    /// The path, then `?` and the query when there is one, as received.
    pub path: &'a str,
    /// The body, exactly as sent.
    pub body: &'a [u8],
    /// The key id the caller names.
    pub key_id: &'a str,
    /// The instant the caller signed at, as RFC 3339 text.
    pub timestamp: &'a str,
}

/// Reads what the generated encoder wrote and drops the member `drop`.
fn without(encoded: &str, drop: &str) -> Result<Value, CanonicalError> {
    match json::parse(encoded).map_err(|error| CanonicalError::Encoding(error.to_string()))? {
        Value::Object(members) => Ok(Value::Object(
            members
                .into_iter()
                .filter(|(name, _)| name != drop)
                .collect(),
        )),
        other => Err(CanonicalError::Encoding(format!(
            "{} instead of an object",
            other.describes()
        ))),
    }
}

/// Refuses an integer that JCS would round.
fn safe_integers(value: &Value) -> Result<(), CanonicalError> {
    match value {
        Value::Number(spelling) => match spelling.parse::<i64>() {
            Ok(integer) if (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(&integer) => Ok(()),
            _ => Err(CanonicalError::UnsafeInteger(spelling.clone())),
        },
        Value::Array(items) => items.iter().try_for_each(safe_integers),
        Value::Object(members) => members.iter().try_for_each(|(_, item)| safe_integers(item)),
        Value::Null | Value::Bool(_) | Value::Text(_) => Ok(()),
    }
}

fn prefixed(context: &[u8], value: &Value) -> Result<Vec<u8>, CanonicalError> {
    let canonical = jcs::canonicalize(value).map_err(CanonicalError::Jcs)?;
    let mut bytes = Vec::with_capacity(context.len() + canonical.len());
    bytes.extend_from_slice(context);
    bytes.extend_from_slice(canonical.as_bytes());
    Ok(bytes)
}

/// The bytes an envelope signature covers: the envelope context string, then the JCS of the
/// envelope without `signature`.
///
/// # Errors
///
/// [`CanonicalError`] when the envelope holds an integer outside ±(2^53 − 1).
pub fn envelope_bytes<E: Envelope + ?Sized>(envelope: &E) -> Result<Vec<u8>, CanonicalError> {
    let mut encoded = String::new();
    envelope.encode(&mut encoded);
    let value = without(&encoded, "signature")?;
    safe_integers(&value)?;
    prefixed(ENVELOPE_CONTEXT, &value)
}

/// The bytes the operator signature of a key entry covers.
#[must_use]
pub fn key_entry_bytes(entry: &KeyEntry) -> Vec<u8> {
    let mut encoded = String::new();
    wire::encode_inbox_messaging_key_entry(entry, &mut encoded);
    entry_bytes(&encoded)
}

/// The bytes the operator signature of a delegation entry covers.
#[must_use]
pub fn delegation_entry_bytes(entry: &DelegationEntry) -> Vec<u8> {
    let mut encoded = String::new();
    wire::encode_inbox_messaging_delegation_entry(entry, &mut encoded);
    entry_bytes(&encoded)
}

/// An entry holds strings only, so neither the reader nor JCS can refuse it.
fn entry_bytes(encoded: &str) -> Vec<u8> {
    without(encoded, "operator_signature")
        .and_then(|value| prefixed(ENTRY_CONTEXT, &value))
        .unwrap_or_else(|error| unreachable!("an entry of strings canonicalizes: {error}"))
}

/// The bytes a request signature covers.
#[must_use]
pub fn request_bytes(request: &Request<'_>) -> Vec<u8> {
    let text = |value: &str| Value::Text(value.to_owned());
    let value = Value::Object(vec![
        (
            "body_sha256".to_owned(),
            text(&encode(&Sha256::digest(request.body))),
        ),
        ("key_id".to_owned(), text(request.key_id)),
        ("method".to_owned(), text(request.method)),
        ("path".to_owned(), text(request.path)),
        ("timestamp".to_owned(), text(request.timestamp)),
    ]);
    prefixed(REQUEST_CONTEXT, &value)
        .unwrap_or_else(|error| unreachable!("five distinct strings canonicalize: {error}"))
}

/// Signs an envelope with the sender's key: sets `signature.key_id` to `key_id` and
/// `signature.value` to the signature over [`envelope_bytes`].
///
/// # Errors
///
/// [`CanonicalError`] when the envelope holds an integer outside ±(2^53 − 1).
pub fn sign_envelope<E: Envelope + ?Sized>(
    envelope: &mut E,
    key_id: &str,
    key: &SecretKey,
) -> Result<(), CanonicalError> {
    let bytes = envelope_bytes(envelope)?;
    let signature = envelope.signature_mut();
    signature.key_id = key_id.to_owned();
    signature.value = encode(&key.sign(&bytes));
    Ok(())
}

/// Signs a key entry with the operator key: sets `operator_signature`.
///
/// An id is never reused: a renewal or a new key takes a new `key_id`, and the operator removes
/// the entry it replaces or lets it lapse (`docs/design/signing.md`, the key file).
pub fn sign_key_entry(entry: &mut KeyEntry, operator: &SecretKey) {
    entry.operator_signature = encode(&operator.sign(&key_entry_bytes(entry)));
}

/// Signs a delegation entry with the operator key: sets `operator_signature`.
///
/// An id is never reused: a renewal or a changed delegation takes a new `delegation_id`, and the
/// operator removes the entry it replaces or lets it lapse (`docs/design/signing.md`, the key
/// file).
pub fn sign_delegation_entry(entry: &mut DelegationEntry, operator: &SecretKey) {
    entry.operator_signature = encode(&operator.sign(&delegation_entry_bytes(entry)));
}

/// The base64 signature of a request, for the caller to send beside it.
#[must_use]
pub fn sign_request(request: &Request<'_>, key: &SecretKey) -> String {
    encode(&key.sign(&request_bytes(request)))
}

#[cfg(test)]
mod tests {
    use super::{ENTRY_CONTEXT, ENVELOPE_CONTEXT, REQUEST_CONTEXT};
    use crate::keys::SecretKey;

    #[test]
    fn one_canonical_object_signed_under_one_context_verifies_under_no_other() {
        let key = SecretKey::generate().unwrap();
        let object = br#"{"key_id":"k","method":"POST"}"#;
        let contexts = [ENVELOPE_CONTEXT, ENTRY_CONTEXT, REQUEST_CONTEXT];
        for signed_as in contexts {
            let signature = key.sign(&[signed_as, object.as_slice()].concat());
            for checked_as in contexts {
                let message = [checked_as, object.as_slice()].concat();
                assert_eq!(
                    key.public_key().verifies(&message, &signature),
                    signed_as == checked_as,
                );
            }
        }
    }
}
