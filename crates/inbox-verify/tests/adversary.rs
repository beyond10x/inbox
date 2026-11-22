//! Adversarial cases for story:signature-verifier. Every key is created at run time.

use crate::common;

use common::{
    AGENT, LATER, OTHER, answer, generate, key_entry, key_file_text, now, post, ring,
    signed_delegation, signed_key_entry,
};
use ed25519_dalek::Verifier;
use inbox::messaging::{DelegationId, KeyFile, Principal};
use inbox_verify::{
    EntryKind, IgnoreReason, KeyRing, Refusal, SecretKey, duplicate_member, envelope_bytes,
    sign_envelope, sign_key_entry,
};
use sha2::{Digest, Sha512};

fn base64(bytes: &[u8]) -> String {
    let mut quoted = String::new();
    inbox::server::json::push_base64(&mut quoted, bytes);
    quoted[1..quoted.len() - 1].to_owned()
}

// ---- scalar arithmetic modulo the group order L, for a hand-built signature ---------------------

/// L = 2^252 + 27742317777372353535851937790883648493, little-endian 64-bit limbs.
const ORDER: [u64; 4] = [
    0x5812_631a_5cf5_d3ed,
    0x14de_f9de_a2f7_9cd6,
    0,
    0x1000_0000_0000_0000,
];

fn at_least(a: &[u64; 4], b: &[u64; 4]) -> bool {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i] > b[i];
        }
    }
    true
}

fn subtract(a: &mut [u64; 4], b: &[u64; 4]) {
    let mut borrow = false;
    for i in 0..4 {
        let (d1, b1) = a[i].overflowing_sub(b[i]);
        let (d2, b2) = d1.overflowing_sub(u64::from(borrow));
        a[i] = d2;
        borrow = b1 || b2;
    }
}

fn add(a: &mut [u64; 4], b: &[u64; 4]) {
    let mut carry = false;
    for i in 0..4 {
        let (s1, c1) = a[i].overflowing_add(b[i]);
        let (s2, c2) = s1.overflowing_add(u64::from(carry));
        a[i] = s2;
        carry = c1 || c2;
    }
}

fn double_plus(a: &mut [u64; 4], bit: u64) {
    let mut carry = bit;
    for limb in a.iter_mut() {
        let next = *limb >> 63;
        *limb = (*limb << 1) | carry;
        carry = next;
    }
}

/// A little-endian integer of any length, reduced modulo L.
fn reduce(le: &[u8]) -> [u64; 4] {
    let mut r = [0_u64; 4];
    for byte in le.iter().rev() {
        for bit in (0..8).rev() {
            double_plus(&mut r, u64::from((byte >> bit) & 1));
            if at_least(&r, &ORDER) {
                subtract(&mut r, &ORDER);
            }
        }
    }
    r
}

fn multiply(a: &[u64; 4], b: &[u64; 4]) -> [u64; 4] {
    let mut r = [0_u64; 4];
    for i in (0..4).rev() {
        for bit in (0..64).rev() {
            double_plus(&mut r, 0);
            if at_least(&r, &ORDER) {
                subtract(&mut r, &ORDER);
            }
            if (b[i] >> bit) & 1 == 1 {
                add(&mut r, a);
                if at_least(&r, &ORDER) {
                    subtract(&mut r, &ORDER);
                }
            }
        }
    }
    r
}

fn to_le(a: &[u64; 4]) -> [u8; 32] {
    let mut out = [0_u8; 32];
    for (i, limb) in a.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&limb.to_le_bytes());
    }
    out
}

/// A signature of `message` under `key` whose `R` is the identity point (order one): `S = k·a`, so
/// `[S]B − [k]A` is the identity. The cofactorless non-strict check accepts it; `verify_strict`
/// refuses it for its small-order `R`. Built from the key's seed at run time; nothing is printed.
fn small_order_r_signature(key: &SecretKey, message: &[u8]) -> [u8; 64] {
    let seed = key.to_seed();
    let expanded = Sha512::digest(seed.as_ref());
    let mut scalar = [0_u8; 32];
    scalar.copy_from_slice(&expanded[..32]);
    scalar[0] &= 248;
    scalar[31] &= 127;
    scalar[31] |= 64;
    let a = reduce(&scalar);
    scalar.iter_mut().for_each(|byte| *byte = 0);
    let mut identity = [0_u8; 32];
    identity[0] = 1;
    let public = key.public_key().to_bytes();
    let k = reduce(
        &Sha512::new()
            .chain_update(identity)
            .chain_update(public)
            .chain_update(message)
            .finalize(),
    );
    let s = to_le(&multiply(&k, &a));
    let mut signature = [0_u8; 64];
    signature[..32].copy_from_slice(&identity);
    signature[32..].copy_from_slice(&s);
    signature
}

// ---- cases ---------------------------------------------------------------------------------------

/// The design note says the verifier is strict (`verify_strict`): a signature whose `R` is a point
/// of small order is refused. The case first shows the hand-built signature is one the non-strict
/// check accepts, so a verifier that is not strict accepts the envelope.
#[test]
fn a_signature_whose_r_is_of_small_order_is_refused() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry(
            "agent-key",
            AGENT,
            &agent,
            LATER,
            &operator,
        )],
        delegations: vec![],
    };
    let ring = ring(&file, &operator);
    let mut envelope = post(AGENT, AGENT, "a body");
    envelope.signature.key_id = "agent-key".to_owned();
    let bytes = envelope_bytes(&envelope).unwrap();
    let forged = small_order_r_signature(&agent, &bytes);

    let dalek = ed25519_dalek::VerifyingKey::from_bytes(&agent.public_key().to_bytes()).unwrap();
    let as_dalek = ed25519_dalek::Signature::from_bytes(&forged);
    assert!(
        dalek.verify(&bytes, &as_dalek).is_ok(),
        "the hand-built signature passes the non-strict check"
    );
    assert!(dalek.verify_strict(&bytes, &as_dalek).is_err());

    envelope.signature.value = base64(&forged);
    assert_eq!(
        ring.verify_envelope(&envelope, now()),
        Err(Refusal::SignatureMismatch)
    );
}

/// The key's principal must be the envelope's `from` (design note, envelope step 3), for a reply
/// as for a post. A reply sent `from` one principal `on_behalf_of` the key's principal, under a
/// live delegation for that pair, is refused: the key does not belong to the sender.
#[test]
fn a_reply_from_another_principal_on_behalf_of_the_key_principal_is_refused() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry(
            "agent-key",
            AGENT,
            &agent,
            LATER,
            &operator,
        )],
        delegations: vec![signed_delegation("d", OTHER, AGENT, LATER, &operator)],
    };
    let ring = ring(&file, &operator);
    let mut reply = answer(OTHER, "yes");
    reply.on_behalf_of = Principal(AGENT.to_owned());
    reply.authority = Some(DelegationId("d".to_owned()));
    sign_envelope(&mut reply, "agent-key", &agent).unwrap();
    assert_eq!(
        ring.check_delegation(
            &DelegationId("d".to_owned()),
            &reply.from,
            &reply.on_behalf_of,
            now()
        ),
        Ok(())
    );
    assert_eq!(
        ring.verify_envelope(&reply, now()),
        Err(Refusal::KeyNotFrom)
    );
}

/// A key file whose entry carries one member twice is read by the generated reader, which keeps
/// the first: the verifier holds the signed principal while a reader that keeps the last (and the
/// operator reading the file) sees another. RFC 8785 input is I-JSON, and `jcs::canonicalize`
/// refuses a duplicate member; the key file reader does not.
#[test]
fn a_key_file_whose_entry_carries_a_member_twice_is_refused() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry("k", AGENT, &agent, LATER, &operator)],
        delegations: vec![],
    };
    let text = key_file_text(&file);
    // The signed member first (the one the generated reader keeps), another principal last.
    let signed = format!("\"principal\":\"{AGENT}\"");
    let duplicated = text.replacen(&signed, &format!("{signed},\"principal\":\"{OTHER}\""), 1);
    assert_ne!(duplicated, text);
    let read = KeyRing::parse(&duplicated, &operator.public_key());
    let held = read
        .as_ref()
        .ok()
        .map(|ring| ring.principal("k", now()).cloned());
    assert!(
        read.is_err(),
        "a key file with a duplicate member was read; it holds key `k` for {held:?}"
    );
}

/// The same for an envelope: a post whose text carries `body` twice decodes to the first and
/// verifies, so one signature covers two texts that differ in what a last-wins reader shows. The
/// generated decoder cannot refuse it, so a receiver refuses the text with
/// `inbox_verify::duplicate_member` before decoding it; this case holds that check.
#[test]
fn a_post_whose_text_carries_a_member_twice_is_refused_before_decoding() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry(
            "agent-key",
            AGENT,
            &agent,
            LATER,
            &operator,
        )],
        delegations: vec![],
    };
    let ring = ring(&file, &operator);
    let mut envelope = post(AGENT, AGENT, "the signed body");
    sign_envelope(&mut envelope, "agent-key", &agent).unwrap();
    let mut signed = String::new();
    inbox::server::wire::encode_command_inbox_messaging_post_message(&envelope, &mut signed);
    let text = signed.replacen(
        "\"body\":\"the signed body\"",
        "\"body\":\"the signed body\",\"body\":\"another body\"",
        1,
    );
    assert_ne!(text, signed);

    // The hazard: the generated reader keeps both members, the decoder takes the first, and the
    // signature verifies.
    let value = inbox::server::json::parse(&text).unwrap();
    let decoded = inbox::server::wire::decode_command_inbox_messaging_post_message(&value, "")
        .expect("the generated decoder takes the first `body`");
    assert_eq!(decoded.body.0, "the signed body");
    assert_eq!(ring.verify_envelope(&decoded, now()), Ok(()));

    // What this crate holds: the check a receiver runs before decoding refuses the text, and
    // passes the text as signed.
    assert_eq!(duplicate_member(&value), Some("body".to_owned()));
    let untouched = inbox::server::json::parse(&signed).unwrap();
    assert_eq!(duplicate_member(&untouched), None);
}

/// The unit claims an id carried by more than one signed entry is ignored in every one of them.
/// Two entries the operator signed carry the id `twice`; one has a public key that is refused.
/// The other is held, resolved by which of the two is well-formed.
#[test]
fn an_id_two_operator_signed_entries_carry_is_ignored_even_when_one_key_is_refused() {
    let operator = generate();
    let agent = generate();
    let mut malformed = key_entry("twice", AGENT, &agent, LATER);
    malformed.public_key = "not base64".to_owned();
    sign_key_entry(&mut malformed, &operator);
    let file = KeyFile {
        keys: vec![
            signed_key_entry("twice", AGENT, &agent, LATER, &operator),
            malformed,
        ],
        delegations: vec![],
    };
    let ring = ring(&file, &operator);
    let reasons: Vec<(EntryKind, String, IgnoreReason)> = ring
        .ignored()
        .iter()
        .map(|ignored| (ignored.kind, ignored.id.clone(), ignored.reason))
        .collect();
    assert_eq!(
        ring.principal("twice", now()),
        Err(Refusal::UnknownKey),
        "ignored: {reasons:?}"
    );
}
