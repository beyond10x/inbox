//! Fixtures shared by the integration tests. Every key is created at run time from the OS random
//! source; no key material is written into this file.

#![allow(dead_code)]

use inbox::messaging::{
    Address, Answer, Body, DedupKey, DelegationEntry, DelegationId, KeyEntry, KeyFile, Kind,
    MessageId, PostMessage, Principal, Signature, ThreadId,
};
use inbox::primitives::Timestamp;
use inbox::server::{json, wire};
use inbox_verify::{Instant, KeyRing, SecretKey, sign_delegation_entry, sign_key_entry};

pub const AGENT: &str = "agent:host-a/session-one";
pub const HUMAN: &str = "human:operator";
pub const OTHER: &str = "agent:host-b/session-two";
pub const NOW: &str = "2026-11-21T12:00:00Z";
pub const LATER: &str = "2026-12-01T00:00:00Z";

pub fn now() -> Instant {
    Instant::parse(NOW).expect("a fixed instant reads")
}

pub fn instant(text: &str) -> Instant {
    Instant::parse(text).expect("a fixed instant reads")
}

pub fn generate() -> SecretKey {
    SecretKey::generate().expect("the OS random source answers")
}

pub fn key_entry(key_id: &str, principal: &str, key: &SecretKey, valid_until: &str) -> KeyEntry {
    KeyEntry {
        key_id: key_id.to_owned(),
        principal: Principal(principal.to_owned()),
        public_key: key.public_key().to_base64(),
        valid_until: Timestamp(valid_until.to_owned()),
        operator_signature: String::new(),
    }
}

pub fn signed_key_entry(
    key_id: &str,
    principal: &str,
    key: &SecretKey,
    valid_until: &str,
    operator: &SecretKey,
) -> KeyEntry {
    let mut entry = key_entry(key_id, principal, key, valid_until);
    sign_key_entry(&mut entry, operator);
    entry
}

pub fn signed_delegation(
    delegation_id: &str,
    agent: &str,
    on_behalf_of: &str,
    valid_until: &str,
    operator: &SecretKey,
) -> DelegationEntry {
    let mut entry = DelegationEntry {
        delegation_id: DelegationId(delegation_id.to_owned()),
        agent: Principal(agent.to_owned()),
        on_behalf_of: Principal(on_behalf_of.to_owned()),
        valid_until: Timestamp(valid_until.to_owned()),
        operator_signature: String::new(),
    };
    sign_delegation_entry(&mut entry, operator);
    entry
}

/// The key file as text, written by the generated encoder: what the service reads from disk.
pub fn key_file_text(file: &KeyFile) -> String {
    let mut out = String::new();
    wire::encode_inbox_messaging_key_file(file, &mut out);
    out
}

/// A ring read from the text of a key file, as the service reads it.
pub fn ring(file: &KeyFile, operator: &SecretKey) -> KeyRing {
    KeyRing::parse(&key_file_text(file), &operator.public_key()).expect("the key file reads")
}

pub fn post(from: &str, on_behalf_of: &str, body: &str) -> PostMessage {
    PostMessage {
        message_id: MessageId("01JD0000000000000000000000".to_owned()),
        from: Principal(from.to_owned()),
        on_behalf_of: Principal(on_behalf_of.to_owned()),
        authority: None,
        to: Address(OTHER.to_owned()),
        kind: Kind::Note,
        thread: ThreadId("01JD0000000000000000000000".to_owned()),
        reply_shape: None,
        decision: None,
        default_on_silence: None,
        created: Timestamp(NOW.to_owned()),
        expires: Timestamp(LATER.to_owned()),
        dedup_key: DedupKey("01JD0000000000000000000000".to_owned()),
        body: Body(body.to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    }
}

pub fn answer(from: &str, body: &str) -> Answer {
    Answer {
        in_reply_to: MessageId("01JD0000000000000000000000".to_owned()),
        reply_id: MessageId("01JD0000000000000000000001".to_owned()),
        from: Principal(from.to_owned()),
        on_behalf_of: Principal(from.to_owned()),
        authority: None,
        created: Timestamp(NOW.to_owned()),
        expires: Timestamp(LATER.to_owned()),
        option: None,
        body: Body(body.to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    }
}

/// A post as a client sends it: encoded, then decoded by the service.
pub fn over_the_wire(envelope: &PostMessage) -> PostMessage {
    let mut out = String::new();
    wire::encode_command_inbox_messaging_post_message(envelope, &mut out);
    let value = json::parse(&out).expect("the encoder writes JSON");
    wire::decode_command_inbox_messaging_post_message(&value, "").expect("the post decodes")
}
