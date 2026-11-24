//! The key file, held as the entries the pinned operator key signed, and every check against it.

use std::collections::HashMap;
use std::fmt;

use inbox::messaging::{DelegationEntry, DelegationId, KeyEntry, KeyFile, Principal};
use inbox::server::{json, wire};

use crate::canonical::{
    Envelope, Request, delegation_entry_bytes, envelope_bytes, key_entry_bytes, request_bytes_for,
};
use crate::duplicate::duplicate_member;
use crate::instant::Instant;
use crate::keys::{EncodingError, PublicKey, signature_from_base64};

/// How far a request's timestamp may lie from the instant of the check, in either direction.
pub const REQUEST_WINDOW_SECONDS: i64 = 60;

/// Which external refusal of the specification a [`Refusal`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalKind {
    /// `signature-invalid`: the key is unknown or lapsed, or the signature does not verify.
    SignatureInvalid,
    /// `authority-invalid`: no live delegation with this id lets `from` speak for `on_behalf_of`.
    AuthorityInvalid,
}

/// Why a check refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// No signed key entry carries this key id.
    UnknownKey,
    /// The key entry is lapsed at the instant of the check.
    KeyLapsed,
    /// The key entry's principal is not the envelope's `from`.
    KeyNotFrom,
    /// The signature is not canonical base64 of 64 bytes.
    SignatureEncoding,
    /// The signature does not verify over the signed bytes.
    SignatureMismatch,
    /// The envelope cannot be put into canonical form (an integer outside ±(2^53 − 1)).
    NotCanonical,
    /// The request's timestamp is not an RFC 3339 instant.
    BadTimestamp,
    /// The request's timestamp is more than 60 s before the instant of the check.
    RequestStale,
    /// The request's timestamp is more than 60 s after the instant of the check.
    RequestEarly,
    /// No signed delegation entry carries this id.
    UnknownDelegation,
    /// The delegation names another pair than `from` and `on_behalf_of`.
    DelegationPair,
    /// The delegation is lapsed at the instant of the check.
    DelegationLapsed,
    /// The request names an inbox no live delegation lets the key's principal act for.
    NotActingFor,
}

impl Refusal {
    /// The external refusal of the specification this is.
    #[must_use]
    pub fn kind(&self) -> RefusalKind {
        match self {
            Self::UnknownDelegation | Self::DelegationPair | Self::DelegationLapsed => {
                RefusalKind::AuthorityInvalid
            }
            _ => RefusalKind::SignatureInvalid,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnknownKey => "the key id is unknown",
            Self::KeyLapsed => "the key is lapsed",
            Self::KeyNotFrom => "the key does not belong to the sender",
            Self::SignatureEncoding => "the signature is not canonical base64 of 64 bytes",
            Self::SignatureMismatch => "the signature does not verify",
            Self::NotCanonical => "the envelope has no canonical form",
            Self::BadTimestamp => "the request timestamp is not an RFC 3339 instant",
            Self::RequestStale => "the request timestamp is more than 60 s old",
            Self::RequestEarly => "the request timestamp is more than 60 s ahead",
            Self::UnknownDelegation => "the delegation id is unknown",
            Self::DelegationPair => "the delegation names another pair",
            Self::DelegationLapsed => "the delegation is lapsed",
            Self::NotActingFor => "no live delegation lets the key's principal act for the inbox",
        })
    }
}

impl std::error::Error for Refusal {}

/// Why an entry of the key file is not held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreReason {
    /// Its `operator_signature` is not canonical base64 of 64 bytes.
    SignatureEncoding,
    /// Its `operator_signature` does not verify under the pinned operator key.
    NotSignedByOperator,
    /// Its `public_key` is refused.
    PublicKey(EncodingError),
    /// Its `valid_until` is not an instant this library can read.
    ValidUntil,
    /// Its id is carried by more than one entry whose operator signature verifies.
    AmbiguousId,
}

/// Which list of the key file an ignored entry stood in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// `keys`.
    Key,
    /// `delegations`.
    Delegation,
}

/// An entry of the key file that is not held, for the operator to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ignored {
    /// The list it stood in.
    pub kind: EntryKind,
    /// Its `key_id` or `delegation_id`.
    pub id: String,
    /// Why.
    pub reason: IgnoreReason,
}

/// A key file that was refused whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// The text is not JSON.
    Parse(String),
    /// An object of the text carries a member name twice; the path of the first.
    DuplicateMember(String),
    /// The JSON is not a key file.
    Decode(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(reason) => write!(f, "the key file is not JSON: {reason}"),
            Self::DuplicateMember(path) => {
                write!(f, "the key file carries the member `{path}` twice")
            }
            Self::Decode(reason) => write!(f, "the key file does not decode: {reason}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// A signed key entry, as held.
#[derive(Debug, Clone)]
struct HeldKey {
    principal: Principal,
    public_key: PublicKey,
    valid_until: Instant,
}

/// A signed delegation entry, as held.
#[derive(Debug, Clone)]
struct HeldDelegation {
    agent: Principal,
    on_behalf_of: Principal,
    valid_until: Instant,
}

/// The entries of a key file that the pinned operator key signed, and every check against them.
#[derive(Debug, Clone)]
pub struct KeyRing {
    keys: HashMap<String, HeldKey>,
    delegations: HashMap<String, HeldDelegation>,
    ignored: Vec<Ignored>,
}

/// Whether `text` is a strict signature of `bytes` under `key`.
fn verifies(key: &PublicKey, bytes: &[u8], text: &str) -> Result<(), IgnoreReason> {
    let signature = signature_from_base64(text).map_err(|_| IgnoreReason::SignatureEncoding)?;
    if key.verifies(bytes, &signature) {
        Ok(())
    } else {
        Err(IgnoreReason::NotSignedByOperator)
    }
}

/// Reads one list of the key file in three steps. First the entries whose operator signature
/// verifies are kept. Of those, every entry whose id more than one of them carries is ignored,
/// whatever the later checks say about each, so a malformed re-signing of an id cannot leave the
/// other entry live. Of the rest, an entry is held when `read` accepts it. An entry the operator
/// did not sign never counts towards an ambiguous id: nobody without the operator key adds an
/// entry or changes what an id means. Whoever can write the key file can remove an id, by
/// deleting its entry or by putting back another entry the operator signed for it (both are then
/// ignored); the verifier fails closed in both cases. An id is never reused.
fn admit<E, T>(
    entries: &[E],
    kind: EntryKind,
    id: impl Fn(&E) -> &str,
    signed: impl Fn(&E) -> Result<(), IgnoreReason>,
    read: impl Fn(&E) -> Result<T, IgnoreReason>,
    ignored: &mut Vec<Ignored>,
) -> HashMap<String, T> {
    let mut ignore = |entry: &E, reason| {
        ignored.push(Ignored {
            kind,
            id: id(entry).to_owned(),
            reason,
        });
    };
    let mut candidates = Vec::new();
    for entry in entries {
        match signed(entry) {
            Ok(()) => candidates.push(entry),
            Err(reason) => ignore(entry, reason),
        }
    }
    let mut count: HashMap<&str, usize> = HashMap::new();
    for entry in &candidates {
        *count.entry(id(entry)).or_default() += 1;
    }
    let (unique, ambiguous): (Vec<&E>, Vec<&E>) = candidates
        .into_iter()
        .partition(|entry| count[id(entry)] == 1);
    for entry in ambiguous {
        ignore(entry, IgnoreReason::AmbiguousId);
    }
    let mut held = HashMap::new();
    for entry in unique {
        match read(entry) {
            Ok(value) => {
                held.insert(id(entry).to_owned(), value);
            }
            Err(reason) => ignore(entry, reason),
        }
    }
    held
}

fn valid_until(timestamp: &inbox::primitives::Timestamp) -> Result<Instant, IgnoreReason> {
    Instant::try_from(timestamp).map_err(|_| IgnoreReason::ValidUntil)
}

impl KeyRing {
    /// Holds the entries of `file` that `operator` signed and whose id no other signed entry
    /// carries; every other entry is ignored and listed in [`KeyRing::ignored`].
    ///
    /// `file` is already decoded, so this cannot see a member name carried twice in its text. A
    /// caller that decodes the key file text itself owes that check: run
    /// [`duplicate_member`](crate::duplicate_member) on the parsed value and refuse the file before
    /// decoding it. [`KeyRing::parse`] does both.
    #[must_use]
    pub fn new(file: &KeyFile, operator: &PublicKey) -> Self {
        let mut ignored = Vec::new();
        let keys = admit(
            &file.keys,
            EntryKind::Key,
            |entry: &KeyEntry| entry.key_id.as_str(),
            |entry| verifies(operator, &key_entry_bytes(entry), &entry.operator_signature),
            |entry| {
                Ok(HeldKey {
                    principal: entry.principal.clone(),
                    public_key: PublicKey::from_base64(&entry.public_key)
                        .map_err(IgnoreReason::PublicKey)?,
                    valid_until: valid_until(&entry.valid_until)?,
                })
            },
            &mut ignored,
        );
        let delegations = admit(
            &file.delegations,
            EntryKind::Delegation,
            |entry: &DelegationEntry| entry.delegation_id.0.as_str(),
            |entry| {
                verifies(
                    operator,
                    &delegation_entry_bytes(entry),
                    &entry.operator_signature,
                )
            },
            |entry| {
                Ok(HeldDelegation {
                    agent: entry.agent.clone(),
                    on_behalf_of: entry.on_behalf_of.clone(),
                    valid_until: valid_until(&entry.valid_until)?,
                })
            },
            &mut ignored,
        );
        Self {
            keys,
            delegations,
            ignored,
        }
    }

    /// Reads the text of a key file through the generated codecs and holds what `operator` signed.
    ///
    /// # Errors
    ///
    /// [`LoadError`] when the text is not JSON, when an object in it carries a member name twice
    /// (refused before decoding: the generated decoder would take the first), or when it does
    /// not decode as a key file.
    pub fn parse(text: &str, operator: &PublicKey) -> Result<Self, LoadError> {
        let value = json::parse(text).map_err(|error| LoadError::Parse(error.to_string()))?;
        if let Some(path) = duplicate_member(&value) {
            return Err(LoadError::DuplicateMember(path));
        }
        let file = wire::decode_inbox_messaging_key_file(&value, "")
            .map_err(|error| LoadError::Decode(error.to_string()))?;
        Ok(Self::new(&file, operator))
    }

    /// The entries of the key file that are not held, and why.
    #[must_use]
    pub fn ignored(&self) -> &[Ignored] {
        &self.ignored
    }

    /// The live key entry with this id.
    fn live_key(&self, key_id: &str, at: Instant) -> Result<&HeldKey, Refusal> {
        let key = self.keys.get(key_id).ok_or(Refusal::UnknownKey)?;
        if at < key.valid_until {
            Ok(key)
        } else {
            Err(Refusal::KeyLapsed)
        }
    }

    /// The principal a key id speaks for, when the key is known and not lapsed at `at`.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownKey`] or [`Refusal::KeyLapsed`].
    pub fn principal(&self, key_id: &str, at: Instant) -> Result<&Principal, Refusal> {
        self.live_key(key_id, at).map(|key| &key.principal)
    }

    /// Verifies an envelope at `at`: its key id names a held key that is not lapsed and belongs to
    /// `from`, and its signature verifies over [`envelope_bytes`](crate::envelope_bytes).
    ///
    /// # Errors
    ///
    /// The first [`Refusal`] in that order; each is [`RefusalKind::SignatureInvalid`].
    pub fn verify_envelope<E: Envelope + ?Sized>(
        &self,
        envelope: &E,
        at: Instant,
    ) -> Result<(), Refusal> {
        let signature = envelope.signature();
        let key = self.live_key(&signature.key_id, at)?;
        if key.principal != *envelope.from() {
            return Err(Refusal::KeyNotFrom);
        }
        let value =
            signature_from_base64(&signature.value).map_err(|_| Refusal::SignatureEncoding)?;
        let bytes = envelope_bytes(envelope).map_err(|_| Refusal::NotCanonical)?;
        if key.public_key.verifies(&bytes, &value) {
            Ok(())
        } else {
            Err(Refusal::SignatureMismatch)
        }
    }

    /// Checks a delegation at `at`: it is held, lets `from` speak for `on_behalf_of`, and is not
    /// lapsed. The chain length is one.
    ///
    /// # Errors
    ///
    /// The first [`Refusal`] in that order; each is [`RefusalKind::AuthorityInvalid`].
    pub fn check_delegation(
        &self,
        id: &DelegationId,
        from: &Principal,
        on_behalf_of: &Principal,
        at: Instant,
    ) -> Result<(), Refusal> {
        let delegation = self
            .delegations
            .get(&id.0)
            .ok_or(Refusal::UnknownDelegation)?;
        if delegation.agent != *from || delegation.on_behalf_of != *on_behalf_of {
            return Err(Refusal::DelegationPair);
        }
        if at < delegation.valid_until {
            Ok(())
        } else {
            Err(Refusal::DelegationLapsed)
        }
    }

    /// Whether `agent` may act for the inbox whose address is the text `on_behalf_of`: a held
    /// delegation entry names `agent` and, byte for byte, that text, and is not lapsed at `at`.
    /// Any one such entry is enough; a lapsed entry beside it refuses nothing. Checked by the pair,
    /// not by an id (`docs/design/signing.md`, "A delegation").
    #[must_use]
    pub fn acts_for(&self, agent: &Principal, on_behalf_of: &str, at: Instant) -> bool {
        self.delegations.values().any(|delegation| {
            delegation.agent == *agent
                && delegation.on_behalf_of.0 == on_behalf_of
                && at < delegation.valid_until
        })
    }

    /// Verifies a request at `at` and answers the principal of its key, for a request that acts
    /// for the inbox `on_behalf_of` names or, with `None`, for none: steps 1 to 3 of
    /// [`KeyRing::verify_request`] over [`request_bytes_for`](crate::request_bytes_for), then,
    /// for `Some`, step 4: the text is not empty, has no leading or trailing space or tab, and
    /// [`KeyRing::acts_for`] admits the pair (the key's principal, that text) at `at`. Only a
    /// request whose signature verified reaches step 4.
    ///
    /// # Errors
    ///
    /// The first [`Refusal`] in that order; step 4 refuses [`Refusal::NotActingFor`].
    pub fn verify_request_for(
        &self,
        request: &Request<'_>,
        on_behalf_of: Option<&str>,
        signature: &str,
        at: Instant,
    ) -> Result<&Principal, Refusal> {
        let key = self.live_key(request.key_id, at)?;
        let signed_at = Instant::parse(request.timestamp).map_err(|_| Refusal::BadTimestamp)?;
        if signed_at < at.plus_seconds(-REQUEST_WINDOW_SECONDS) {
            return Err(Refusal::RequestStale);
        }
        if signed_at > at.plus_seconds(REQUEST_WINDOW_SECONDS) {
            return Err(Refusal::RequestEarly);
        }
        let value = signature_from_base64(signature).map_err(|_| Refusal::SignatureEncoding)?;
        if !key
            .public_key
            .verifies(&request_bytes_for(request, on_behalf_of), &value)
        {
            return Err(Refusal::SignatureMismatch);
        }
        match on_behalf_of {
            Some(text) if !(an_address(text) && self.acts_for(&key.principal, text, at)) => {
                Err(Refusal::NotActingFor)
            }
            _ => Ok(&key.principal),
        }
    }

    /// Verifies a request at `at` and answers the principal of its key: the key is held and not
    /// lapsed, the timestamp lies within 60 s of `at` either way, and `signature` verifies over
    /// [`request_bytes`](crate::request_bytes).
    ///
    /// # Errors
    ///
    /// The first [`Refusal`] in that order.
    pub fn verify_request(
        &self,
        request: &Request<'_>,
        signature: &str,
        at: Instant,
    ) -> Result<&Principal, Refusal> {
        self.verify_request_for(request, None, signature, at)
    }
}

/// Whether `text` may name an inbox a request acts for: not empty, and no space or tab at either
/// end. Version 1 holds no other grammar for an address (`docs/design/signing.md`, "Request").
fn an_address(text: &str) -> bool {
    let edge = [' ', '\t'];
    !text.is_empty() && !text.starts_with(edge) && !text.ends_with(edge)
}
