//! Ed25519 keys and signatures, and their canonical base64 on the wire.

use std::fmt;

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use inbox::server::json::{self, Value};
use zeroize::Zeroizing;

/// A public key or a signature whose text was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingError {
    /// Not canonical base64 (standard alphabet, padded, unused bits zero).
    Base64,
    /// Canonical base64 of the wrong number of bytes.
    Length,
    /// Not the canonical encoding of a curve point.
    Point,
    /// A point of small order: a weak key, which verifies a signature over almost any message.
    WeakKey,
}

impl fmt::Display for EncodingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Base64 => "not canonical base64",
            Self::Length => "the wrong number of bytes",
            Self::Point => "not the canonical encoding of a curve point",
            Self::WeakKey => "a point of small order",
        })
    }
}

impl std::error::Error for EncodingError {}

/// The OS random source did not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomError(pub String);

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the OS random source failed: {}", self.0)
    }
}

impl std::error::Error for RandomError {}

/// An Ed25519 public key that is canonical and not of small order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublicKey(VerifyingKey);

impl PublicKey {
    /// Reads the 44-character base64 text of a public key, as a key entry or the configuration
    /// carries it.
    ///
    /// # Errors
    ///
    /// [`EncodingError`] when the text is not canonical base64 of 32 bytes, the bytes are not the
    /// canonical encoding of a curve point, or the point is of small order.
    pub fn from_base64(text: &str) -> Result<Self, EncodingError> {
        let bytes: [u8; 32] = decode_exact(text)?;
        Self::from_bytes(&bytes)
    }

    /// Reads the 32 bytes of a public key.
    ///
    /// # Errors
    ///
    /// As [`PublicKey::from_base64`], less the base64.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, EncodingError> {
        let key = VerifyingKey::from_bytes(bytes).map_err(|_| EncodingError::Point)?;
        if key.to_edwards().compress().to_bytes() != *bytes {
            return Err(EncodingError::Point);
        }
        if key.is_weak() {
            return Err(EncodingError::WeakKey);
        }
        Ok(Self(key))
    }

    /// The 32 bytes of the key.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// The 44-character base64 text of the key.
    #[must_use]
    pub fn to_base64(&self) -> String {
        encode(&self.0.to_bytes())
    }

    /// Whether `signature` is a strict Ed25519 signature of `message` under this key.
    pub(crate) fn verifies(&self, message: &[u8], signature: &[u8; 64]) -> bool {
        let signature = ed25519_dalek::Signature::from_bytes(signature);
        self.0.verify_strict(message, &signature).is_ok()
    }
}

/// An Ed25519 private key. It has no `Display`, its `Debug` prints only the public key, and its
/// bytes are zeroed when it is dropped.
#[derive(Clone)]
pub struct SecretKey(SigningKey);

impl fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretKey")
            .field("public_key", &self.public_key().to_base64())
            .finish_non_exhaustive()
    }
}

impl SecretKey {
    /// Creates a key pair from the OS random source. Nothing is written anywhere: the caller holds
    /// the private key, and its public half is [`SecretKey::public_key`].
    ///
    /// # Errors
    ///
    /// [`RandomError`] when the OS random source does not answer.
    pub fn generate() -> Result<Self, RandomError> {
        let mut seed = Zeroizing::new([0_u8; 32]);
        getrandom::fill(seed.as_mut()).map_err(|error| RandomError(error.to_string()))?;
        Ok(Self(SigningKey::from_bytes(&seed)))
    }

    /// The private key from its 32-byte seed, as the CLI stores it.
    #[must_use]
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self(SigningKey::from_bytes(seed))
    }

    /// The 32-byte seed, for the CLI to store. Zeroed when dropped.
    #[must_use]
    pub fn to_seed(&self) -> Zeroizing<[u8; 32]> {
        Zeroizing::new(self.0.to_bytes())
    }

    /// The public half.
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }

    /// The Ed25519 signature of `message`.
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.0.sign(message).to_bytes()
    }
}

/// The base64 text of bytes: standard alphabet, padded.
pub(crate) fn encode(bytes: &[u8]) -> String {
    let mut quoted = String::new();
    json::push_base64(&mut quoted, bytes);
    quoted[1..quoted.len() - 1].to_owned()
}

/// Exactly `N` bytes from canonical base64 text.
pub(crate) fn decode_exact<const N: usize>(text: &str) -> Result<[u8; N], EncodingError> {
    let bytes = json::bytes_at(&Value::Text(text.to_owned()), "", "base64")
        .map_err(|_| EncodingError::Base64)?;
    if encode(&bytes) != text {
        return Err(EncodingError::Base64);
    }
    bytes.try_into().map_err(|_| EncodingError::Length)
}

/// A signature from its 88-character base64 text.
pub(crate) fn signature_from_base64(text: &str) -> Result<[u8; 64], EncodingError> {
    decode_exact(text)
}

#[cfg(test)]
mod tests {
    use super::{EncodingError, PublicKey, SecretKey, decode_exact, encode};

    #[test]
    fn base64_round_trips_and_refuses_a_non_canonical_spelling() {
        assert_eq!(encode(&[0xff]), "/w==");
        assert_eq!(decode_exact::<1>("/w=="), Ok([0xff]));
        assert_eq!(decode_exact::<1>("/x=="), Err(EncodingError::Base64));
        assert_eq!(decode_exact::<1>("/w"), Err(EncodingError::Base64));
        assert_eq!(decode_exact::<1>("_w=="), Err(EncodingError::Base64));
        assert_eq!(decode_exact::<2>("/w=="), Err(EncodingError::Length));
    }

    #[test]
    fn a_small_order_key_is_refused() {
        // The identity point, y = 1: of order one.
        let mut identity = [0_u8; 32];
        identity[0] = 1;
        assert_eq!(
            PublicKey::from_bytes(&identity),
            Err(EncodingError::WeakKey)
        );
    }

    #[test]
    fn a_non_canonical_point_encoding_is_refused() {
        // y = p + 1, the non-canonical spelling of y = 1.
        let mut bytes = [0xff_u8; 32];
        bytes[0] = 0xee;
        bytes[31] = 0x7f;
        assert_eq!(PublicKey::from_bytes(&bytes), Err(EncodingError::Point));
    }

    #[test]
    fn the_debug_output_of_a_private_key_holds_only_its_public_key() {
        let key = SecretKey::generate().unwrap();
        let printed = format!("{key:?}");
        assert!(printed.contains(&key.public_key().to_base64()));
        assert!(!printed.contains(&encode(key.to_seed().as_ref())));
    }

    #[test]
    fn a_seed_round_trips() {
        let key = SecretKey::generate().unwrap();
        let again = SecretKey::from_seed(&key.to_seed());
        assert_eq!(again.public_key(), key.public_key());
    }
}
