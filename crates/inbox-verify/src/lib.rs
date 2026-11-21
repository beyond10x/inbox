//! The version 1 signature verifier of the inbox.
//!
//! Ed25519 signatures over envelopes, key file entries and requests, each over a context string and
//! the RFC 8785 canonical JSON of what it signs, as `docs/design/signing.md` fixes them. A
//! [`KeyRing`] holds the entries of a key file that the pinned operator key signed, and checks an
//! envelope, a delegation or a request against them at an instant the caller passes: the library
//! reads no clock. A failed check refuses; nothing is accepted unverified.

mod canonical;
mod duplicate;
mod instant;
pub mod jcs;
mod keys;
mod ring;

pub use canonical::{
    CanonicalError, ENTRY_CONTEXT, ENVELOPE_CONTEXT, Envelope, MAX_SAFE_INTEGER, REQUEST_CONTEXT,
    Request, delegation_entry_bytes, envelope_bytes, key_entry_bytes, request_bytes,
    sign_delegation_entry, sign_envelope, sign_key_entry, sign_request,
};
pub use duplicate::duplicate_member;
pub use instant::{Instant, InstantError};
pub use keys::{EncodingError, PublicKey, RandomError, SecretKey};
pub use ring::{
    EntryKind, IgnoreReason, Ignored, KeyRing, LoadError, REQUEST_WINDOW_SECONDS, Refusal,
    RefusalKind,
};
