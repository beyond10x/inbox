//! Adversary cases for story:post-from-is-the-caller over HTTP: a post or an answer whose `from`
//! is not the caller is answered `403` `not-sender`, stores nothing, takes no id or key, and
//! answers alike whatever the store holds. Every key is created when the test runs.

use crate::support;

use std::sync::Arc;

use inbox::messaging::{
    Answer, Body, DelegationEntry, DelegationId, KeyEntry, KeyFile, MessageId, Principal,
    ReplyShape, Signature,
};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_core::MemoryBackend;
use inbox_server::ManualClock;
use inbox_verify::{KeyRing, SecretKey, sign_delegation_entry, sign_envelope, sign_key_entry};
use support::{
    AGENT, KEYS_VALID_UNTIL, LATER, Member, NOW, OTHER, THIRD, World, encoded, instant, ulid,
};

const POST: &str = "/messaging/commands/post";
const ANSWER: &str = "/messaging/commands/answer";
const NOT_SENDER: &str =
    r#"{"outcome":"not-sender","published":[],"error":"inbox.messaging.NotSender"}"#;

/// The 3 principals of `support`, and one delegation: OTHER may speak for AGENT.
fn world_where_other_speaks_for_agent() -> World {
    let operator = SecretKey::generate().expect("the OS random source answers");
    let members: Vec<Member> = [AGENT, OTHER, THIRD]
        .into_iter()
        .enumerate()
        .map(|(at, principal)| Member {
            principal,
            key_id: format!("key-{at}"),
            key: SecretKey::generate().expect("the OS random source answers"),
        })
        .collect();
    let keys = members
        .iter()
        .map(|member| {
            let mut entry = KeyEntry {
                key_id: member.key_id.clone(),
                principal: Principal(member.principal.to_owned()),
                public_key: member.key.public_key().to_base64(),
                valid_until: Timestamp(KEYS_VALID_UNTIL.to_owned()),
                operator_signature: String::new(),
            };
            sign_key_entry(&mut entry, &operator);
            entry
        })
        .collect();
    let mut delegation = DelegationEntry {
        delegation_id: DelegationId("other-for-agent".to_owned()),
        agent: Principal(OTHER.to_owned()),
        on_behalf_of: Principal(AGENT.to_owned()),
        valid_until: Timestamp(KEYS_VALID_UNTIL.to_owned()),
        operator_signature: String::new(),
    };
    sign_delegation_entry(&mut delegation, &operator);
    let file = KeyFile {
        keys,
        delegations: vec![delegation],
    };
    let mut text = String::new();
    wire::encode_inbox_messaging_key_file(&file, &mut text);
    let ring = KeyRing::parse(&text, &operator.public_key()).expect("the key file reads");
    World {
        members,
        ring,
        clock: Arc::new(ManualClock::new(instant(NOW))),
        backend: MemoryBackend::new(),
    }
}

/// A reply from `from` to message `n`, signed with the key of `signer`.
fn reply(world: &World, n: u32, reply_n: u32, from: &str, signer: &str) -> Vec<u8> {
    let mut answer = Answer {
        in_reply_to: MessageId(ulid(n)),
        reply_id: MessageId(ulid(reply_n)),
        from: Principal(from.to_owned()),
        on_behalf_of: Principal(from.to_owned()),
        authority: None,
        created: Timestamp(NOW.to_owned()),
        expires: Timestamp(LATER.to_owned()),
        option: None,
        body: Body("one line".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    let member = world.member(signer);
    sign_envelope(&mut answer, &member.key_id, &member.key).expect("the reply canonicalizes");
    let mut out = String::new();
    wire::encode_command_inbox_messaging_answer(&answer, &mut out);
    out.into_bytes()
}

fn messages(world: &World) -> usize {
    inbox::behaviour::MessageStorage::list(&world.backend).len()
}

#[test]
fn adv_w6_a_replay_before_the_sender_posts_takes_neither_the_id_nor_the_key() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "not yet sent");
    let bytes = encoded(&envelope);

    let replayed = world.send(address, OTHER, "POST", POST, &bytes);
    assert_eq!((replayed.status, replayed.body.as_str()), (403, NOT_SENDER));
    assert_eq!(messages(&world), 0);

    let posted = world.send(address, AGENT, "POST", POST, &bytes);
    assert_eq!(posted.status, 202, "{}", posted.body);
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
}

#[test]
fn adv_w6_a_non_sender_learns_nothing_about_ids_keys_or_inboxes() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let held = world.post(AGENT, OTHER, 1, "held");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&held));
    assert_eq!(posted.status, 202, "{}", posted.body);

    // THIRD sends envelopes from AGENT: the held one, a fresh id under the held key, a fresh id
    // and key, and one to an inbox nobody opened. Each is answered with the same bytes.
    let mut same_key = world.post(AGENT, OTHER, 2, "same key");
    same_key.dedup_key = held.dedup_key.clone();
    world.sign(&mut same_key);
    let fresh = world.post(AGENT, OTHER, 3, "fresh");
    let nowhere = world.post(AGENT, "agent:host-z/nobody", 4, "nowhere");
    for (case, envelope) in [
        ("held id", &held),
        ("held key", &same_key),
        ("fresh", &fresh),
        ("no inbox", &nowhere),
    ] {
        let reply = world.send(address, THIRD, "POST", POST, &encoded(envelope));
        assert_eq!(
            (reply.status, reply.body.as_str()),
            (403, NOT_SENDER),
            "{case}"
        );
    }
    assert_eq!(messages(&world), 1);
}

#[test]
fn adv_w6_a_from_spelled_almost_like_the_caller_is_not_sender() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, AGENT);
    let other = world.member(OTHER);
    let lookalike = OTHER.replacen('o', "\u{043e}", 1);
    let upper = OTHER.to_uppercase();
    let spaced = format!("{OTHER} ");
    let leading = format!(" {OTHER}");
    let slashed = format!("{OTHER}/");
    let nul = format!("{OTHER}\u{0}");
    let variants = [
        upper.as_str(),
        spaced.as_str(),
        leading.as_str(),
        slashed.as_str(),
        lookalike.as_str(),
        nul.as_str(),
        "",
    ];
    for (n, from) in (10..).zip(variants) {
        let mut envelope = world.post(OTHER, AGENT, n, "spelled");
        envelope.from = Principal(from.to_owned());
        envelope.on_behalf_of = Principal(from.to_owned());
        sign_envelope(&mut envelope, &other.key_id, &other.key).expect("canonicalizes");
        let reply = world.send(address, OTHER, "POST", POST, &encoded(&envelope));
        assert_eq!(
            (reply.status, reply.body.as_str()),
            (403, NOT_SENDER),
            "from {from:?}"
        );
    }
    assert_eq!(messages(&world), 0);
}

#[test]
fn adv_w6_a_delegation_to_the_caller_does_not_let_it_send_as_the_principal_it_speaks_for() {
    let world = world_where_other_speaks_for_agent();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, THIRD);

    // AGENT's own envelope, carrying the delegation's id, sent by OTHER.
    let mut as_agent = world.post(AGENT, THIRD, 1, "as agent");
    as_agent.authority = Some(DelegationId("other-for-agent".to_owned()));
    world.sign(&mut as_agent);
    let reply = world.send(address, OTHER, "POST", POST, &encoded(&as_agent));
    assert_eq!((reply.status, reply.body.as_str()), (403, NOT_SENDER));

    // `from` AGENT, `on_behalf_of` the caller.
    let mut reversed = world.post(AGENT, THIRD, 2, "reversed");
    reversed.on_behalf_of = Principal(OTHER.to_owned());
    reversed.authority = Some(DelegationId("other-for-agent".to_owned()));
    world.sign(&mut reversed);
    let reply = world.send(address, OTHER, "POST", POST, &encoded(&reversed));
    assert_eq!((reply.status, reply.body.as_str()), (403, NOT_SENDER));
    assert_eq!(messages(&world), 0);

    // What the delegation is for: OTHER is `from` and speaks for AGENT.
    let mut relayed = world.post(OTHER, THIRD, 3, "relayed");
    relayed.on_behalf_of = Principal(AGENT.to_owned());
    relayed.authority = Some(DelegationId("other-for-agent".to_owned()));
    world.sign(&mut relayed);
    let reply = world.send(address, OTHER, "POST", POST, &encoded(&relayed));
    assert_eq!(reply.status, 202, "{}", reply.body);
    assert_eq!(reply.text("outcome"), "posted", "{}", reply.body);
}

#[test]
fn adv_w6_a_reply_sent_by_another_principal_is_not_sender_and_leaves_the_request_open() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let mut request = world.post(AGENT, OTHER, 1, "how many lines?");
    request.kind = inbox::messaging::Kind::Request;
    request.reply_shape = Some(ReplyShape::Lines(3));
    world.sign(&mut request);
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&request));
    assert_eq!(posted.status, 202, "{}", posted.body);

    // OTHER's signed reply, sent by THIRD; for a held and for a missing message.
    let held = reply(&world, 1, 2, OTHER, OTHER);
    let missing = reply(&world, 99, 3, OTHER, OTHER);
    for (case, bytes) in [("held", &held), ("missing", &missing)] {
        let answered = world.send(address, THIRD, "POST", ANSWER, bytes);
        assert_eq!(
            (answered.status, answered.body.as_str()),
            (403, NOT_SENDER),
            "{case}"
        );
    }
    // The owner sends a reply naming THIRD as `from`, signed with THIRD's key.
    let named = reply(&world, 1, 4, THIRD, THIRD);
    let answered = world.send(address, OTHER, "POST", ANSWER, &named);
    assert_eq!((answered.status, answered.body.as_str()), (403, NOT_SENDER));

    // The request is still open: the owner's own reply is answered.
    let answered = world.send(address, OTHER, "POST", ANSWER, &held);
    assert_eq!(answered.status, 202, "{}", answered.body);
    assert_eq!(answered.text("outcome"), "answered", "{}", answered.body);

    // Settled now: a replay of the reply by THIRD is still `not-sender`, not `already-settled`.
    let replayed = world.send(address, THIRD, "POST", ANSWER, &held);
    assert_eq!((replayed.status, replayed.body.as_str()), (403, NOT_SENDER));
}
