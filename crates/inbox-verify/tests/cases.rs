//! The cases story:signature-verifier owes beside its acceptance statement.

mod common;

use common::{
    AGENT, HUMAN, LATER, NOW, OTHER, answer, generate, instant, key_entry, key_file_text, now,
    over_the_wire, post, ring, signed_delegation, signed_key_entry,
};
use inbox::messaging::{
    Body, DelegationId, KeyFile, MessageId, OptionLetter, Principal, ReplyShape,
};
use inbox_verify::{
    CanonicalError, ENTRY_CONTEXT, ENVELOPE_CONTEXT, EncodingError, EntryKind, IgnoreReason,
    Ignored, KeyRing, LoadError, MAX_SAFE_INTEGER, REQUEST_CONTEXT, Refusal, RefusalKind, Request,
    delegation_entry_bytes, duplicate_member, envelope_bytes, key_entry_bytes, request_bytes,
    sign_envelope, sign_request,
};

/// One agent key, signed by the operator, valid until `LATER`; the ring holds it.
struct Fixture {
    operator: inbox_verify::SecretKey,
    agent: inbox_verify::SecretKey,
    ring: KeyRing,
}

fn fixture() -> Fixture {
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
        delegations: vec![
            signed_delegation("for-human", AGENT, HUMAN, LATER, &operator),
            signed_delegation("lapsed", AGENT, HUMAN, NOW, &operator),
        ],
    };
    let ring = ring(&file, &operator);
    assert_eq!(ring.ignored(), &[]);
    Fixture {
        operator,
        agent,
        ring,
    }
}

fn signed_post(fixture: &Fixture) -> inbox::messaging::PostMessage {
    let mut envelope = post(AGENT, AGENT, "a body");
    sign_envelope(&mut envelope, "agent-key", &fixture.agent).unwrap();
    envelope
}

// ---- envelopes -------------------------------------------------------------------------------

#[test]
fn an_unknown_key_id_is_refused() {
    let fixture = fixture();
    let mut envelope = post(AGENT, AGENT, "a body");
    sign_envelope(&mut envelope, "no-such-key", &fixture.agent).unwrap();
    let refusal = fixture.ring.verify_envelope(&envelope, now()).unwrap_err();
    assert_eq!(refusal, Refusal::UnknownKey);
    assert_eq!(refusal.kind(), RefusalKind::SignatureInvalid);
}

#[test]
fn a_key_is_lapsed_at_its_valid_until_and_live_one_nanosecond_before() {
    let fixture = fixture();
    let envelope = signed_post(&fixture);
    assert_eq!(
        fixture
            .ring
            .verify_envelope(&envelope, instant("2026-11-30T23:59:59.999999999Z")),
        Ok(())
    );
    assert_eq!(
        fixture.ring.verify_envelope(&envelope, instant(LATER)),
        Err(Refusal::KeyLapsed)
    );
    assert_eq!(
        fixture
            .ring
            .verify_envelope(&envelope, instant("2027-01-01T00:00:00Z")),
        Err(Refusal::KeyLapsed)
    );
    assert_eq!(
        fixture
            .ring
            .verify_envelope(&envelope, instant("2026-12-01T01:00:00+01:00")),
        Err(Refusal::KeyLapsed)
    );
}

#[test]
fn a_key_entry_the_pinned_operator_did_not_sign_is_ignored() {
    let operator = generate();
    let impostor = generate();
    let agent = generate();
    let mut tampered = signed_key_entry("tampered", AGENT, &agent, NOW, &operator);
    tampered.valid_until.0 = LATER.to_owned();
    let mut unsigned = key_entry("unsigned", AGENT, &agent, LATER);
    unsigned.operator_signature = String::new();
    let file = KeyFile {
        keys: vec![
            signed_key_entry("by-impostor", AGENT, &agent, LATER, &impostor),
            tampered,
            unsigned,
            signed_key_entry("by-operator", AGENT, &agent, LATER, &operator),
        ],
        delegations: vec![],
    };
    let ring = ring(&file, &operator);
    assert_eq!(
        ring.ignored(),
        &[
            Ignored {
                kind: EntryKind::Key,
                id: "by-impostor".to_owned(),
                reason: IgnoreReason::NotSignedByOperator,
            },
            Ignored {
                kind: EntryKind::Key,
                id: "tampered".to_owned(),
                reason: IgnoreReason::NotSignedByOperator,
            },
            Ignored {
                kind: EntryKind::Key,
                id: "unsigned".to_owned(),
                reason: IgnoreReason::SignatureEncoding,
            },
        ]
    );
    for key_id in ["by-impostor", "tampered", "unsigned"] {
        let mut envelope = post(AGENT, AGENT, "a body");
        sign_envelope(&mut envelope, key_id, &agent).unwrap();
        assert_eq!(
            ring.verify_envelope(&envelope, now()),
            Err(Refusal::UnknownKey),
            "{key_id}"
        );
    }
    let mut envelope = post(AGENT, AGENT, "a body");
    sign_envelope(&mut envelope, "by-operator", &agent).unwrap();
    assert_eq!(ring.verify_envelope(&envelope, now()), Ok(()));
}

#[test]
fn a_key_that_belongs_to_another_principal_is_refused() {
    let fixture = fixture();
    let mut envelope = post(OTHER, OTHER, "a body");
    sign_envelope(&mut envelope, "agent-key", &fixture.agent).unwrap();
    assert_eq!(
        fixture.ring.verify_envelope(&envelope, now()),
        Err(Refusal::KeyNotFrom)
    );
}

/// The key must belong to `from`, not to `on_behalf_of`: a post sent `from` one principal
/// `on_behalf_of` the key's principal, under a live delegation for that pair, is refused.
#[test]
fn a_post_from_another_principal_on_behalf_of_the_key_principal_is_refused() {
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
    let mut envelope = post(OTHER, AGENT, "a body");
    envelope.authority = Some(DelegationId("d".to_owned()));
    sign_envelope(&mut envelope, "agent-key", &agent).unwrap();
    assert_eq!(
        ring.check_delegation(
            &DelegationId("d".to_owned()),
            &envelope.from,
            &envelope.on_behalf_of,
            now()
        ),
        Ok(())
    );
    assert_eq!(
        ring.verify_envelope(&envelope, now()),
        Err(Refusal::KeyNotFrom)
    );
}

#[test]
fn a_signature_by_another_key_under_a_known_key_id_is_refused() {
    let fixture = fixture();
    let mut envelope = post(AGENT, AGENT, "a body");
    sign_envelope(&mut envelope, "agent-key", &generate()).unwrap();
    assert_eq!(
        fixture.ring.verify_envelope(&envelope, now()),
        Err(Refusal::SignatureMismatch)
    );
}

#[test]
fn every_field_of_a_post_is_covered() {
    let fixture = fixture();
    let signed = signed_post(&fixture);
    type Change = fn(&mut inbox::messaging::PostMessage);
    let changes: [(&str, Change); 10] = [
        ("message_id", |e| {
            e.message_id = MessageId("01JD0000000000000000000009".to_owned())
        }),
        ("on_behalf_of", |e| {
            e.on_behalf_of = Principal(HUMAN.to_owned())
        }),
        ("authority", |e| {
            e.authority = Some(DelegationId("for-human".to_owned()))
        }),
        ("to", |e| e.to.0 = AGENT.to_owned()),
        ("kind", |e| e.kind = inbox::messaging::Kind::Request),
        ("reply_shape", |e| {
            e.reply_shape = Some(ReplyShape::Lines(3))
        }),
        ("default_on_silence", |e| {
            e.default_on_silence = Some(OptionLetter("A".to_owned()))
        }),
        ("created", |e| {
            e.created.0 = "2026-11-21T12:00:00z".to_owned()
        }),
        ("expires", |e| {
            e.expires.0 = "2026-12-02T00:00:00Z".to_owned()
        }),
        ("body", |e| e.body = Body("a body ".to_owned())),
    ];
    for (field, change) in changes {
        let mut changed = signed.clone();
        change(&mut changed);
        assert_eq!(
            fixture.ring.verify_envelope(&changed, now()),
            Err(Refusal::SignatureMismatch),
            "{field}"
        );
    }
}

#[test]
fn a_reply_verifies_and_a_changed_reply_is_refused() {
    let fixture = fixture();
    let mut reply = answer(AGENT, "yes");
    reply.option = Some(OptionLetter("B".to_owned()));
    sign_envelope(&mut reply, "agent-key", &fixture.agent).unwrap();
    assert_eq!(fixture.ring.verify_envelope(&reply, now()), Ok(()));
    let mut changed = reply.clone();
    changed.option = Some(OptionLetter("A".to_owned()));
    assert_eq!(
        fixture.ring.verify_envelope(&changed, now()),
        Err(Refusal::SignatureMismatch)
    );
}

#[test]
fn the_wire_spelling_of_an_envelope_does_not_change_its_signed_bytes() {
    let fixture = fixture();
    let mut envelope = post(AGENT, AGENT, "line one\nline \"two\" \u{20ac}");
    envelope.reply_shape = Some(ReplyShape::Lines(3));
    envelope.kind = inbox::messaging::Kind::Request;
    sign_envelope(&mut envelope, "agent-key", &fixture.agent).unwrap();
    let text = format!(
        concat!(
            "{{ \"signature\": {{\"value\": \"{value}\", \"key_id\": \"agent-key\"}},",
            " \"body\": \"line one\\u000aline \\\"two\\\" \\u20ac\", \"authority\": null,",
            " \"reply_shape\": {{\"value\": 3.0, \"shape\": \"lines\"}},",
            " \"message_id\": \"01JD0000000000000000000000\", \"from\": \"{from}\",",
            " \"on_behalf_of\": \"{from}\", \"to\": \"{to}\", \"kind\": \"request\",",
            " \"thread\": \"01JD0000000000000000000000\", \"created\": \"{now}\",",
            " \"expires\": \"{later}\", \"dedup_key\": \"01JD0000000000000000000000\" }}"
        ),
        value = envelope.signature.value,
        from = AGENT,
        to = OTHER,
        now = NOW,
        later = LATER,
    );
    let value = inbox::server::json::parse(&text).unwrap();
    let received =
        inbox::server::wire::decode_command_inbox_messaging_post_message(&value, "").unwrap();
    assert_eq!(received, over_the_wire(&envelope));
    assert_eq!(fixture.ring.verify_envelope(&received, now()), Ok(()));
}

#[test]
fn an_integer_jcs_would_round_is_refused_on_both_sides() {
    let fixture = fixture();
    let mut largest = post(AGENT, AGENT, "a body");
    largest.reply_shape = Some(ReplyShape::Lines(MAX_SAFE_INTEGER));
    sign_envelope(&mut largest, "agent-key", &fixture.agent).unwrap();
    assert_eq!(fixture.ring.verify_envelope(&largest, now()), Ok(()));

    let mut beyond = post(AGENT, AGENT, "a body");
    beyond.reply_shape = Some(ReplyShape::Lines(MAX_SAFE_INTEGER + 1));
    assert_eq!(
        sign_envelope(&mut beyond, "agent-key", &fixture.agent),
        Err(CanonicalError::UnsafeInteger("9007199254740992".to_owned()))
    );
    let mut forged = largest.clone();
    forged.reply_shape = Some(ReplyShape::Lines(-MAX_SAFE_INTEGER - 1));
    assert_eq!(
        fixture.ring.verify_envelope(&forged, now()),
        Err(Refusal::NotCanonical)
    );
}

// ---- signature encoding ------------------------------------------------------------------------

/// Signature texts that are not canonical base64 of 64 bytes, built from a real one.
fn malformed(signature: &str) -> Vec<(String, &'static str)> {
    let short = inbox_verify_base64(&[7_u8; 63]);
    let long = inbox_verify_base64(&[7_u8; 65]);
    // 64 bytes end in a group of one byte: two characters, then `==`. The second character
    // carries the last 2 bits of the byte and 4 unused bits, which must be zero.
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let carrier = ALPHABET
        .iter()
        .position(|byte| *byte == signature.as_bytes()[85])
        .expect("a base64 character");
    assert_eq!(carrier & 0b1111, 0, "the encoder writes zero unused bits");
    let mut last_bit = signature.to_owned();
    last_bit.replace_range(85..86, &char::from(ALPHABET[carrier | 1]).to_string());
    vec![
        (String::new(), "empty"),
        (short, "63 bytes"),
        (long, "65 bytes"),
        (
            signature.trim_end_matches('=').to_owned(),
            "padding dropped",
        ),
        (format!("-{}", &signature[1..]), "URL-safe character"),
        (format!(" {signature}"), "leading space"),
        (
            format!("{}\n{}", &signature[..44], &signature[44..]),
            "line break",
        ),
        (last_bit, "unused bits set"),
        (format!("{signature}===="), "trailing padding"),
    ]
}

fn inbox_verify_base64(bytes: &[u8]) -> String {
    let mut quoted = String::new();
    inbox::server::json::push_base64(&mut quoted, bytes);
    quoted[1..quoted.len() - 1].to_owned()
}

#[test]
fn an_envelope_signature_of_the_wrong_length_or_encoding_is_refused() {
    let fixture = fixture();
    let signed = signed_post(&fixture);
    for (text, what) in malformed(&signed.signature.value) {
        let mut envelope = signed.clone();
        envelope.signature.value = text;
        let refusal = fixture.ring.verify_envelope(&envelope, now()).unwrap_err();
        assert_eq!(refusal, Refusal::SignatureEncoding, "{what}");
    }
}

#[test]
fn a_request_signature_of_the_wrong_length_or_encoding_is_refused() {
    let fixture = fixture();
    let request = a_request(b"{}", NOW);
    let signature = sign_request(&request, &fixture.agent);
    for (text, what) in malformed(&signature) {
        assert_eq!(
            fixture.ring.verify_request(&request, &text, now()),
            Err(Refusal::SignatureEncoding),
            "{what}"
        );
    }
}

#[test]
fn an_entry_whose_operator_signature_is_malformed_is_ignored() {
    let operator = generate();
    let agent = generate();
    let good = signed_key_entry("k", AGENT, &agent, LATER, &operator);
    let mut keys = Vec::new();
    for (index, (text, _)) in malformed(&good.operator_signature).into_iter().enumerate() {
        let mut entry = good.clone();
        entry.key_id = format!("k{index}");
        entry.operator_signature = text;
        keys.push(entry);
    }
    let count = keys.len();
    let ring = ring(
        &KeyFile {
            keys,
            delegations: vec![],
        },
        &operator,
    );
    assert_eq!(ring.ignored().len(), count);
    assert!(
        ring.ignored()
            .iter()
            .all(|ignored| ignored.reason == IgnoreReason::SignatureEncoding)
    );
}

#[test]
fn a_signed_entry_with_a_weak_or_malformed_public_key_is_ignored() {
    let operator = generate();
    let agent = generate();
    let mut identity = [0_u8; 32];
    identity[0] = 1;
    let cases = [
        (
            "weak",
            inbox_verify_base64(&identity),
            EncodingError::WeakKey,
        ),
        (
            "short",
            inbox_verify_base64(&[9_u8; 31]),
            EncodingError::Length,
        ),
        ("text", "not base64".to_owned(), EncodingError::Base64),
    ];
    let mut keys = Vec::new();
    for (key_id, public_key, _) in &cases {
        let mut entry = key_entry(key_id, AGENT, &agent, LATER);
        entry.public_key = public_key.clone();
        inbox_verify::sign_key_entry(&mut entry, &operator);
        keys.push(entry);
    }
    let ring = ring(
        &KeyFile {
            keys,
            delegations: vec![],
        },
        &operator,
    );
    let expected: Vec<Ignored> = cases
        .iter()
        .map(|(key_id, _, error)| Ignored {
            kind: EntryKind::Key,
            id: (*key_id).to_owned(),
            reason: IgnoreReason::PublicKey(*error),
        })
        .collect();
    assert_eq!(ring.ignored(), expected.as_slice());
}

#[test]
fn an_id_two_signed_entries_carry_is_ignored_in_both() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![
            signed_key_entry("twice", AGENT, &agent, LATER, &operator),
            signed_key_entry("twice", OTHER, &agent, LATER, &operator),
            signed_key_entry("once", AGENT, &agent, LATER, &operator),
        ],
        delegations: vec![
            signed_delegation("d", AGENT, HUMAN, LATER, &operator),
            signed_delegation("d", AGENT, OTHER, LATER, &operator),
        ],
    };
    let ring = ring(&file, &operator);
    let reasons: Vec<(EntryKind, &str, IgnoreReason)> = ring
        .ignored()
        .iter()
        .map(|ignored| (ignored.kind, ignored.id.as_str(), ignored.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            (EntryKind::Key, "twice", IgnoreReason::AmbiguousId),
            (EntryKind::Key, "twice", IgnoreReason::AmbiguousId),
            (EntryKind::Delegation, "d", IgnoreReason::AmbiguousId),
            (EntryKind::Delegation, "d", IgnoreReason::AmbiguousId),
        ]
    );
    assert_eq!(ring.principal("twice", now()), Err(Refusal::UnknownKey));
    assert_eq!(
        ring.principal("once", now()),
        Ok(&Principal(AGENT.to_owned()))
    );
    assert_eq!(
        ring.check_delegation(
            &DelegationId("d".to_owned()),
            &Principal(AGENT.to_owned()),
            &Principal(HUMAN.to_owned()),
            now()
        ),
        Err(Refusal::UnknownDelegation)
    );
}

// ---- one kind never verifies as another --------------------------------------------------------

#[test]
fn the_context_strings_are_distinct_and_none_is_a_prefix_of_another() {
    let contexts = [ENVELOPE_CONTEXT, ENTRY_CONTEXT, REQUEST_CONTEXT];
    for (i, a) in contexts.iter().enumerate() {
        assert_eq!(a.last(), Some(&b'\n'));
        for (j, b) in contexts.iter().enumerate() {
            if i != j {
                assert!(!b.starts_with(a), "{a:?} prefixes {b:?}");
            }
        }
    }
}

#[test]
fn the_signed_bytes_of_each_kind_are_its_context_then_a_jcs_object() {
    let fixture = fixture();
    let operator = &fixture.operator;
    let entry = signed_key_entry("k", AGENT, &fixture.agent, LATER, operator);
    let delegation = signed_delegation("d", AGENT, HUMAN, LATER, operator);
    let request = a_request(b"", NOW);
    let cases: [(&[u8], Vec<u8>); 4] = [
        (
            ENVELOPE_CONTEXT,
            envelope_bytes(&post(AGENT, AGENT, "a body")).unwrap(),
        ),
        (ENTRY_CONTEXT, key_entry_bytes(&entry)),
        (ENTRY_CONTEXT, delegation_entry_bytes(&delegation)),
        (REQUEST_CONTEXT, request_bytes(&request)),
    ];
    for (context, bytes) in cases {
        assert!(bytes.starts_with(context));
        assert_eq!(bytes[context.len()], b'{');
    }
    let mut public_key = String::new();
    inbox::server::json::push_text(&mut public_key, &entry.public_key);
    assert_eq!(
        String::from_utf8(key_entry_bytes(&entry)).unwrap(),
        format!(
            "inbox.v1.entry\n{{\"key_id\":\"k\",\"principal\":\"{AGENT}\",\"public_key\":{public_key},\"valid_until\":\"{LATER}\"}}"
        )
    );
    assert_eq!(
        String::from_utf8(delegation_entry_bytes(&delegation)).unwrap(),
        format!(
            "inbox.v1.entry\n{{\"agent\":\"{AGENT}\",\"delegation_id\":\"d\",\"on_behalf_of\":\"{HUMAN}\",\"valid_until\":\"{LATER}\"}}"
        )
    );
    assert_eq!(
        String::from_utf8(request_bytes(&request)).unwrap(),
        format!(
            "inbox.v1.request\n{{\"body_sha256\":\"47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=\",\"key_id\":\"agent-key\",\"method\":\"POST\",\"path\":\"/messaging/post?dry=1\",\"timestamp\":\"{NOW}\"}}"
        )
    );
}

#[test]
fn a_signature_of_one_kind_does_not_verify_as_another() {
    // One key signs as operator and as agent, so only the kind differs.
    let key = generate();
    let entry = signed_key_entry("self", HUMAN, &key, LATER, &key);
    let delegation = signed_delegation("d", HUMAN, HUMAN, LATER, &key);
    let ring = ring(
        &KeyFile {
            keys: vec![entry.clone()],
            delegations: vec![delegation.clone()],
        },
        &key,
    );
    assert_eq!(ring.ignored(), &[]);

    let mut envelope = post(HUMAN, HUMAN, "a body");
    sign_envelope(&mut envelope, "self", &key).unwrap();
    let mut request = a_request(b"a body", NOW);
    request.key_id = "self";
    let request_signature = sign_request(&request, &key);
    assert_eq!(ring.verify_envelope(&envelope, now()), Ok(()));
    assert_eq!(
        ring.verify_request(&request, &request_signature, now()),
        Ok(&Principal(HUMAN.to_owned()))
    );

    // An entry's or a request's signature, presented as an envelope's.
    for (what, foreign) in [
        ("key entry", entry.operator_signature.clone()),
        ("delegation entry", delegation.operator_signature.clone()),
        ("request", request_signature.clone()),
    ] {
        let mut forged = envelope.clone();
        forged.signature.value = foreign;
        assert_eq!(
            ring.verify_envelope(&forged, now()),
            Err(Refusal::SignatureMismatch),
            "{what}"
        );
    }
    // An envelope's or an entry's signature, presented as a request's.
    for (what, foreign) in [
        ("envelope", envelope.signature.value.clone()),
        ("key entry", entry.operator_signature.clone()),
    ] {
        assert_eq!(
            ring.verify_request(&request, &foreign, now()),
            Err(Refusal::SignatureMismatch),
            "{what}"
        );
    }
    // An envelope's or a request's signature, presented as an entry's.
    for (what, foreign) in [
        ("envelope", envelope.signature.value.clone()),
        ("request", request_signature),
    ] {
        let mut forged = entry.clone();
        forged.key_id = "forged".to_owned();
        forged.operator_signature = foreign;
        let file = KeyFile {
            keys: vec![forged],
            delegations: vec![],
        };
        let ring = KeyRing::parse(&key_file_text(&file), &key.public_key()).unwrap();
        assert_eq!(
            ring.ignored()[0].reason,
            IgnoreReason::NotSignedByOperator,
            "{what}"
        );
    }
}

// ---- delegations -------------------------------------------------------------------------------

fn check(
    ring: &KeyRing,
    id: &str,
    from: &str,
    on_behalf_of: &str,
    at: &str,
) -> Result<(), Refusal> {
    ring.check_delegation(
        &DelegationId(id.to_owned()),
        &Principal(from.to_owned()),
        &Principal(on_behalf_of.to_owned()),
        instant(at),
    )
}

#[test]
fn a_live_delegation_for_the_pair_is_accepted() {
    let fixture = fixture();
    assert_eq!(check(&fixture.ring, "for-human", AGENT, HUMAN, NOW), Ok(()));
    assert_eq!(
        check(
            &fixture.ring,
            "for-human",
            AGENT,
            HUMAN,
            "2026-11-30T23:59:59.999999999Z"
        ),
        Ok(())
    );
}

#[test]
fn an_unknown_delegation_is_refused() {
    let fixture = fixture();
    let refusal = check(&fixture.ring, "no-such", AGENT, HUMAN, NOW).unwrap_err();
    assert_eq!(refusal, Refusal::UnknownDelegation);
    assert_eq!(refusal.kind(), RefusalKind::AuthorityInvalid);
}

#[test]
fn a_lapsed_delegation_is_refused() {
    let fixture = fixture();
    assert_eq!(
        check(&fixture.ring, "lapsed", AGENT, HUMAN, NOW),
        Err(Refusal::DelegationLapsed)
    );
    assert_eq!(
        check(&fixture.ring, "for-human", AGENT, HUMAN, LATER),
        Err(Refusal::DelegationLapsed)
    );
    assert_eq!(
        Refusal::DelegationLapsed.kind(),
        RefusalKind::AuthorityInvalid
    );
}

#[test]
fn a_delegation_that_names_another_pair_is_refused() {
    let fixture = fixture();
    for (from, on_behalf_of) in [
        (OTHER, HUMAN),
        (AGENT, OTHER),
        (HUMAN, AGENT),
        (AGENT, AGENT),
    ] {
        let refusal = check(&fixture.ring, "for-human", from, on_behalf_of, NOW).unwrap_err();
        assert_eq!(
            refusal,
            Refusal::DelegationPair,
            "{from} for {on_behalf_of}"
        );
        assert_eq!(refusal.kind(), RefusalKind::AuthorityInvalid);
    }
}

#[test]
fn a_delegation_the_pinned_operator_did_not_sign_is_ignored() {
    let operator = generate();
    let mut tampered = signed_delegation("tampered", AGENT, HUMAN, LATER, &operator);
    tampered.agent = Principal(OTHER.to_owned());
    let file = KeyFile {
        keys: vec![],
        delegations: vec![
            signed_delegation("by-impostor", AGENT, HUMAN, LATER, &generate()),
            tampered,
        ],
    };
    let ring = ring(&file, &operator);
    assert_eq!(ring.ignored().len(), 2);
    assert!(
        ring.ignored()
            .iter()
            .all(|ignored| ignored.kind == EntryKind::Delegation
                && ignored.reason == IgnoreReason::NotSignedByOperator)
    );
    assert_eq!(
        check(&ring, "by-impostor", AGENT, HUMAN, NOW),
        Err(Refusal::UnknownDelegation)
    );
    assert_eq!(
        check(&ring, "tampered", OTHER, HUMAN, NOW),
        Err(Refusal::UnknownDelegation)
    );
}

// ---- requests ----------------------------------------------------------------------------------

fn a_request<'a>(body: &'a [u8], timestamp: &'a str) -> Request<'a> {
    Request {
        method: "POST",
        path: "/messaging/post?dry=1",
        body,
        key_id: "agent-key",
        timestamp,
    }
}

#[test]
fn a_signed_request_authenticates_its_key_principal() {
    let fixture = fixture();
    let request = a_request(b"{\"a\":1}", NOW);
    let signature = sign_request(&request, &fixture.agent);
    assert_eq!(
        fixture.ring.verify_request(&request, &signature, now()),
        Ok(&Principal(AGENT.to_owned()))
    );
    assert_eq!(
        fixture.ring.principal("agent-key", now()),
        Ok(&Principal(AGENT.to_owned()))
    );
}

#[test]
fn a_request_within_60_seconds_either_way_is_accepted() {
    let fixture = fixture();
    for timestamp in [
        "2026-11-21T11:59:00Z",
        "2026-11-21T12:01:00Z",
        "2026-11-21T13:01:00+01:00",
    ] {
        let request = a_request(b"", timestamp);
        let signature = sign_request(&request, &fixture.agent);
        assert_eq!(
            fixture.ring.verify_request(&request, &signature, now()),
            Ok(&Principal(AGENT.to_owned())),
            "{timestamp}"
        );
    }
}

#[test]
fn a_request_outside_the_60_second_window_is_refused_in_each_direction() {
    let fixture = fixture();
    let cases = [
        ("2026-11-21T11:58:59.999999999Z", Refusal::RequestStale),
        ("2026-11-21T11:00:00Z", Refusal::RequestStale),
        ("2026-11-21T12:01:00.000000001Z", Refusal::RequestEarly),
        ("2026-11-21T13:00:00Z", Refusal::RequestEarly),
        ("yesterday", Refusal::BadTimestamp),
    ];
    for (timestamp, expected) in cases {
        let request = a_request(b"", timestamp);
        let signature = sign_request(&request, &fixture.agent);
        assert_eq!(
            fixture.ring.verify_request(&request, &signature, now()),
            Err(expected),
            "{timestamp}"
        );
    }
}

#[test]
fn a_request_with_a_changed_method_path_query_body_timestamp_or_key_is_refused() {
    let fixture = fixture();
    let operator = &fixture.operator;
    let second = generate();
    let file = KeyFile {
        keys: vec![
            signed_key_entry("agent-key", AGENT, &fixture.agent, LATER, operator),
            signed_key_entry("second-key", AGENT, &second, LATER, operator),
        ],
        delegations: vec![],
    };
    let ring = ring(&file, operator);
    let signed = a_request(b"{\"a\":1}", NOW);
    let signature = sign_request(&signed, &fixture.agent);
    let changes: [(&str, Request<'_>); 8] = [
        (
            "method",
            Request {
                method: "PUT",
                ..signed
            },
        ),
        (
            "method case",
            Request {
                method: "post",
                ..signed
            },
        ),
        (
            "path",
            Request {
                path: "/messaging/answer?dry=1",
                ..signed
            },
        ),
        (
            "query",
            Request {
                path: "/messaging/post?dry=2",
                ..signed
            },
        ),
        (
            "query dropped",
            Request {
                path: "/messaging/post",
                ..signed
            },
        ),
        (
            "body",
            Request {
                body: b"{\"a\":2}",
                ..signed
            },
        ),
        (
            "timestamp",
            Request {
                timestamp: "2026-11-21T12:00:01Z",
                ..signed
            },
        ),
        (
            "key id",
            Request {
                key_id: "second-key",
                ..signed
            },
        ),
    ];
    assert_eq!(
        ring.verify_request(&signed, &signature, now()),
        Ok(&Principal(AGENT.to_owned()))
    );
    for (what, changed) in changes {
        assert_eq!(
            ring.verify_request(&changed, &signature, now()),
            Err(Refusal::SignatureMismatch),
            "{what}"
        );
    }
}

#[test]
fn a_request_under_an_unknown_or_lapsed_key_is_refused() {
    let fixture = fixture();
    let unknown = Request {
        key_id: "no-such",
        ..a_request(b"", NOW)
    };
    let signature = sign_request(&unknown, &fixture.agent);
    assert_eq!(
        fixture.ring.verify_request(&unknown, &signature, now()),
        Err(Refusal::UnknownKey)
    );

    let late = a_request(b"", LATER);
    let signature = sign_request(&late, &fixture.agent);
    assert_eq!(
        fixture
            .ring
            .verify_request(&late, &signature, instant(LATER)),
        Err(Refusal::KeyLapsed)
    );
    assert_eq!(
        fixture.ring.principal("agent-key", instant(LATER)),
        Err(Refusal::KeyLapsed)
    );
}

// ---- the key file ------------------------------------------------------------------------------

#[test]
fn a_key_file_that_is_not_json_or_not_a_key_file_is_refused_whole() {
    let operator = generate().public_key();
    assert!(matches!(
        KeyRing::parse("{", &operator),
        Err(LoadError::Parse(_))
    ));
    assert!(matches!(
        KeyRing::parse("{\"keys\":[]}", &operator),
        Err(LoadError::Decode(_))
    ));
    assert!(matches!(
        KeyRing::parse("{\"keys\":[],\"delegations\":[],\"extra\":1}", &operator),
        Err(LoadError::Decode(_))
    ));
    assert!(KeyRing::parse("{\"keys\":[],\"delegations\":[]}", &operator).is_ok());
}

#[test]
fn a_key_file_holds_no_private_key() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry("k", AGENT, &agent, LATER, &operator)],
        delegations: vec![signed_delegation("d", AGENT, HUMAN, LATER, &operator)],
    };
    let text = key_file_text(&file);
    for key in [&operator, &agent] {
        assert!(!text.contains(&inbox_verify_base64(key.to_seed().as_ref())));
    }
    assert!(text.contains(&agent.public_key().to_base64()));
}

#[test]
fn a_key_file_with_a_member_twice_at_any_depth_is_refused_whole() {
    let operator = generate();
    let agent = generate();
    let file = KeyFile {
        keys: vec![signed_key_entry("k", AGENT, &agent, LATER, &operator)],
        delegations: vec![signed_delegation("d", AGENT, HUMAN, LATER, &operator)],
    };
    let text = key_file_text(&file);
    let public = operator.public_key();
    assert!(KeyRing::parse(&text, &public).is_ok());
    let agent_member = format!("\"agent\":\"{AGENT}\"");
    let cases = [
        (
            text.replacen("\"keys\":[", "\"keys\":[],\"keys\":[", 1),
            "keys",
        ),
        (
            text.replacen(
                &agent_member,
                &format!("{agent_member},\"agent\":\"{OTHER}\""),
                1,
            ),
            "delegations.0.agent",
        ),
        (
            text.replacen("\"key_id\":\"k\"", "\"key_id\":\"k\",\"key_id\":\"k\"", 1),
            "keys.0.key_id",
        ),
    ];
    for (duplicated, path) in cases {
        assert_ne!(duplicated, text);
        assert_eq!(
            KeyRing::parse(&duplicated, &public).err(),
            Some(LoadError::DuplicateMember(path.to_owned())),
            "{path}"
        );
    }
}

#[test]
fn a_duplicate_member_is_found_at_any_depth_and_named_by_its_path() {
    let found = |text: &str| duplicate_member(&inbox::server::json::parse(text).unwrap());
    assert_eq!(found(r#"{"a":1,"b":[{"c":1},{"c":2}]}"#), None);
    assert_eq!(found(r#"[1,{"x":{}},"x"]"#), None);
    assert_eq!(found(r#"{"a":1,"a":1}"#), Some("a".to_owned()));
    assert_eq!(
        found(r#"{"a":[0,{"b":{"c":1,"c":2}}]}"#),
        Some("a.1.b.c".to_owned())
    );
    assert_eq!(found(r#"[{"k":1},{"k":1,"k":2}]"#), Some("1.k".to_owned()));
    // The first in document order.
    assert_eq!(found(r#"{"x":{"y":1,"y":2},"x":3}"#), Some("x".to_owned()));
    assert_eq!(
        found(r#"{"p":{"y":1,"y":2},"q":{"z":1,"z":2}}"#),
        Some("p.y".to_owned())
    );
    // A value nested deeper than the caller-facing reader admits is walked to the bottom.
    let deep = format!(
        "{}{{\"a\":1,\"a\":2}}{}",
        "[".repeat(1_000),
        "]".repeat(1_000)
    );
    let deep = inbox::server::json::parse_written(&deep).unwrap();
    assert_eq!(
        duplicate_member(&deep),
        Some(format!("{}a", "0.".repeat(1_000)))
    );
}

#[test]
fn an_entry_without_the_operator_signature_does_not_knock_out_a_held_id() {
    let operator = generate();
    let impostor = generate();
    let agent = generate();
    let mut unsigned = key_entry("agent-key", OTHER, &impostor, LATER);
    unsigned.operator_signature = String::new();
    let file = KeyFile {
        keys: vec![
            signed_key_entry("agent-key", AGENT, &agent, LATER, &operator),
            signed_key_entry("agent-key", OTHER, &impostor, LATER, &impostor),
            unsigned,
        ],
        delegations: vec![
            signed_delegation("d", AGENT, HUMAN, LATER, &operator),
            signed_delegation("d", OTHER, HUMAN, LATER, &impostor),
        ],
    };
    let ring = ring(&file, &operator);
    let reasons: Vec<(EntryKind, &str, IgnoreReason)> = ring
        .ignored()
        .iter()
        .map(|ignored| (ignored.kind, ignored.id.as_str(), ignored.reason))
        .collect();
    assert_eq!(
        reasons,
        [
            (
                EntryKind::Key,
                "agent-key",
                IgnoreReason::NotSignedByOperator
            ),
            (EntryKind::Key, "agent-key", IgnoreReason::SignatureEncoding),
            (
                EntryKind::Delegation,
                "d",
                IgnoreReason::NotSignedByOperator
            ),
        ]
    );
    assert_eq!(
        ring.principal("agent-key", now()),
        Ok(&Principal(AGENT.to_owned()))
    );
    assert_eq!(
        ring.check_delegation(
            &DelegationId("d".to_owned()),
            &Principal(AGENT.to_owned()),
            &Principal(HUMAN.to_owned()),
            now()
        ),
        Ok(())
    );
}

#[test]
fn an_id_two_operator_signed_entries_carry_is_ignored_whatever_the_later_checks_say() {
    let operator = generate();
    let agent = generate();
    let mut late = key_entry("bad-time", AGENT, &agent, LATER);
    late.valid_until.0 = "not an instant".to_owned();
    inbox_verify::sign_key_entry(&mut late, &operator);
    let mut weak = key_entry("bad-key", AGENT, &agent, LATER);
    let mut identity = [0_u8; 32];
    identity[0] = 1;
    weak.public_key = inbox_verify_base64(&identity);
    inbox_verify::sign_key_entry(&mut weak, &operator);
    let file = KeyFile {
        keys: vec![
            signed_key_entry("bad-time", AGENT, &agent, LATER, &operator),
            late,
            signed_key_entry("bad-key", AGENT, &agent, LATER, &operator),
            weak,
        ],
        delegations: vec![],
    };
    // Built in code: the decoder would refuse the `valid_until` that is not an instant.
    let ring = KeyRing::new(&file, &operator.public_key());
    assert!(
        ring.ignored()
            .iter()
            .all(|ignored| ignored.reason == IgnoreReason::AmbiguousId)
    );
    assert_eq!(ring.ignored().len(), 4);
    for key_id in ["bad-time", "bad-key"] {
        assert_eq!(
            ring.principal(key_id, now()),
            Err(Refusal::UnknownKey),
            "{key_id}"
        );
    }
}
