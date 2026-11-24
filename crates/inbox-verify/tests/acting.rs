//! story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal in the verifier: the
//! sixth member `on_behalf_of` of a signed request, and the check of a delegation by its pair
//! (`docs/design/signing.md`, "Request", "A delegation" and "A request"). Every key is created
//! when the test runs.

use crate::common;

use common::{
    AGENT, HUMAN, LATER, NOW, OTHER, generate, instant, now, ring, signed_delegation,
    signed_key_entry,
};
use inbox::messaging::{KeyFile, Principal};
use inbox_verify::{
    KeyRing, Refusal, Request, SecretKey, request_bytes, request_bytes_for, sign_request,
    sign_request_for,
};

/// The instant the delegations of the pair (AGENT, HUMAN) lapse at; the keys live past it.
const UNTIL: &str = "2026-11-25T00:00:00Z";

fn a_request(timestamp: &str) -> Request<'_> {
    Request {
        method: "POST",
        path: "/messaging/commands/read",
        body: b"",
        key_id: "agent-key",
        timestamp,
    }
}

/// The agent's key, valid until `LATER`, and these delegations: AGENT for HUMAN live until
/// `UNTIL` and one beside it lapsed at `NOW`; AGENT for OTHER lapsed at `NOW` only; OTHER for
/// AGENT live; and AGENT for `human:ignored`, signed by another key than the operator's.
fn fixture() -> (SecretKey, KeyRing) {
    let operator = generate();
    let impostor = generate();
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
            signed_delegation("for-human", AGENT, HUMAN, UNTIL, &operator),
            signed_delegation("for-human-lapsed", AGENT, HUMAN, NOW, &operator),
            signed_delegation("for-other-lapsed", AGENT, OTHER, NOW, &operator),
            signed_delegation("other-for-agent", OTHER, AGENT, LATER, &operator),
            signed_delegation("by-impostor", AGENT, "human:ignored", LATER, &impostor),
        ],
    };
    let ring = ring(&file, &operator);
    assert_eq!(ring.ignored().len(), 1, "{:?}", ring.ignored());
    (agent, ring)
}

#[test]
fn the_sixth_member_sits_between_method_and_path() {
    let request = a_request(NOW);
    let empty = "47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=";
    assert_eq!(
        String::from_utf8(request_bytes_for(&request, Some(HUMAN))).unwrap(),
        format!(
            "inbox.v1.request\n{{\"body_sha256\":\"{empty}\",\"key_id\":\"agent-key\",\"method\":\"POST\",\"on_behalf_of\":\"{HUMAN}\",\"path\":\"/messaging/commands/read\",\"timestamp\":\"{NOW}\"}}"
        )
    );
    // Without the member the bytes are those of the five, byte for byte.
    assert_eq!(request_bytes_for(&request, None), request_bytes(&request));
    // An empty member is a member: never the bytes of a request without one.
    let with_empty = String::from_utf8(request_bytes_for(&request, Some(""))).unwrap();
    assert!(
        with_empty.contains(r#""method":"POST","on_behalf_of":"","path""#),
        "{with_empty}"
    );
    assert_ne!(
        request_bytes_for(&request, Some("")),
        request_bytes(&request)
    );
}

#[test]
fn a_delegation_admits_the_pair_while_live() {
    let (_, ring) = fixture();
    let agent = Principal(AGENT.to_owned());
    let other = Principal(OTHER.to_owned());
    // Live, with a lapsed entry of the same pair beside it.
    assert!(ring.acts_for(&agent, HUMAN, now()));
    assert!(ring.acts_for(&agent, HUMAN, instant("2026-11-24T23:59:59.999999999Z")));
    // At `valid_until` the entry is lapsed: the bound is exclusive.
    assert!(!ring.acts_for(&agent, HUMAN, instant(UNTIL)));
    assert!(!ring.acts_for(&agent, HUMAN, instant("2026-11-25T01:00:00+01:00")));
    // Another agent, another person, the pair the other way round.
    assert!(!ring.acts_for(&other, HUMAN, now()));
    assert!(!ring.acts_for(&agent, "human:someone-else", now()));
    assert!(ring.acts_for(&other, AGENT, now()));
    // Every entry of the pair lapsed.
    assert!(!ring.acts_for(&agent, OTHER, now()));
    // An entry the operator did not sign counts for nothing.
    assert!(!ring.acts_for(&agent, "human:ignored", now()));
    // Byte for byte: no space, no case folded, and no own address without an entry.
    for text in [
        " human:operator",
        "human:operator ",
        "Human:operator",
        "",
        AGENT,
    ] {
        assert!(!ring.acts_for(&agent, text, now()), "{text:?}");
    }
}

#[test]
fn a_request_signed_with_the_member_verifies_only_with_it() {
    let (key, ring) = fixture();
    let request = a_request(NOW);
    let six = sign_request_for(&request, Some(HUMAN), &key);
    let five = sign_request(&request, &key);
    let agent = Principal(AGENT.to_owned());
    assert_eq!(
        ring.verify_request_for(&request, Some(HUMAN), &six, now()),
        Ok(&agent)
    );
    assert_eq!(sign_request_for(&request, None, &key), five);
    assert_eq!(
        ring.verify_request_for(&request, None, &five, now()),
        Ok(&agent)
    );
    // Signed with the member, checked without it, and the other way round.
    assert_eq!(
        ring.verify_request_for(&request, None, &six, now()),
        Err(Refusal::SignatureMismatch)
    );
    assert_eq!(
        ring.verify_request(&request, &six, now()),
        Err(Refusal::SignatureMismatch)
    );
    assert_eq!(
        ring.verify_request_for(&request, Some(HUMAN), &five, now()),
        Err(Refusal::SignatureMismatch)
    );
    // Checked with another text than the signed one.
    for text in ["human:operator ", "human:someone-else", ""] {
        assert_eq!(
            ring.verify_request_for(&request, Some(text), &six, now()),
            Err(Refusal::SignatureMismatch),
            "{text:?}"
        );
    }
}

#[test]
fn a_request_for_an_inbox_needs_a_live_delegation_of_the_pair() {
    let (key, ring) = fixture();
    let request = a_request(NOW);
    for text in [
        OTHER,
        "",
        " human:operator",
        "human:operator\t",
        AGENT,
        "human:ignored",
        "human:someone-else",
    ] {
        let signature = sign_request_for(&request, Some(text), &key);
        assert_eq!(
            ring.verify_request_for(&request, Some(text), &signature, now()),
            Err(Refusal::NotActingFor),
            "{text:?}"
        );
    }
    // At `valid_until` of the live entry, with a request signed at that instant.
    let at = a_request(UNTIL);
    let signature = sign_request_for(&at, Some(HUMAN), &key);
    assert_eq!(
        ring.verify_request_for(&at, Some(HUMAN), &signature, instant(UNTIL)),
        Err(Refusal::NotActingFor)
    );
    // The key is checked first: a request of an unknown key never reaches the delegation.
    let mut unknown = a_request(NOW);
    unknown.key_id = "no-such-key";
    let signature = sign_request_for(&unknown, Some(OTHER), &key);
    assert_eq!(
        ring.verify_request_for(&unknown, Some(OTHER), &signature, now()),
        Err(Refusal::UnknownKey)
    );
    // A signature that does not verify is refused as such, before the delegation is looked up.
    let foreign = sign_request_for(&request, Some(OTHER), &generate());
    assert_eq!(
        ring.verify_request_for(&request, Some(OTHER), &foreign, now()),
        Err(Refusal::SignatureMismatch)
    );
}
