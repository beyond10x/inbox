//! Adversary cases for story:dedup-key-scope (stage 2, wave 7), over HTTP against a running
//! service. Every key is created when the test runs.

use crate::support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use inbox::messaging::{
    DedupKey, DelegationEntry, DelegationId, KeyEntry, KeyFile, PostMessage, Principal,
};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_core::MemoryBackend;
use inbox_server::{ManualClock, request_headers};
use inbox_verify::{KeyRing, SecretKey, sign_delegation_entry, sign_key_entry};
use support::{AGENT, KEYS_VALID_UNTIL, Member, NOW, OTHER, THIRD, World, encoded, instant};

const POST: &str = "/messaging/commands/post";
const KEY: &str = "deploy-2026-11-21";

/// A world whose key file also holds one delegation: AGENT may speak for THIRD.
fn world_with_delegation() -> World {
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
        delegation_id: DelegationId("delegation-1".to_owned()),
        agent: Principal(AGENT.to_owned()),
        on_behalf_of: Principal(THIRD.to_owned()),
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

/// The whole answer to a signed POST as it arrived: status line, headers and body.
fn raw_post(world: &World, address: SocketAddr, principal: &str, body: &[u8]) -> String {
    let member = world.member(principal);
    let headers = request_headers(
        "POST",
        POST,
        body,
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .expect("a read timeout");
    let mut head = format!(
        "POST {POST} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in &headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).expect("the head is sent");
    stream.write_all(body).expect("the body is sent");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("the answer is read");
    String::from_utf8(answer).expect("the answer is UTF-8")
}

fn keyed(world: &World, from: &str, to: &str, n: u32, key: &str) -> PostMessage {
    let mut post = world.post(from, to, n, "b");
    post.dedup_key = DedupKey(key.to_owned());
    world.sign(&mut post);
    post
}

/// The answer with the post's own message id (and thread, the same text) taken out, and the
/// `Content-Length` it changes not at all (ids have one length).
fn without_id(answer: &str, post: &PostMessage) -> String {
    answer.replace(&post.message_id.0, "ID")
}

#[test]
fn adv_w7_a_delegates_key_refuses_no_post_of_the_principal_it_speaks_for() {
    let world = world_with_delegation();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);

    let mut relayed = keyed(&world, AGENT, OTHER, 1, KEY);
    relayed.on_behalf_of = Principal(THIRD.to_owned());
    relayed.authority = Some(DelegationId("delegation-1".to_owned()));
    world.sign(&mut relayed);
    let answer = raw_post(&world, address, AGENT, &encoded(&relayed));
    assert!(answer.contains("\"outcome\":\"posted\""), "{answer}");

    let own = keyed(&world, THIRD, OTHER, 2, KEY);
    let answer = raw_post(&world, address, THIRD, &encoded(&own));
    assert!(answer.contains("\"outcome\":\"posted\""), "{answer}");
}

/// Over HTTP, into the caller's own inbox, into the other sender's inbox and into a third inbox:
/// a post under a key another sender holds is answered byte for byte as a post under a key
/// nobody holds, the post's own id aside.
#[test]
fn adv_w7_a_key_another_sender_holds_answers_byte_for_byte_as_a_free_key() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    for who in [AGENT, OTHER, THIRD] {
        world.open(address, who);
    }
    let mut n = 10;
    for to in [THIRD, OTHER, AGENT] {
        n += 3;
        // OTHER holds this round's key in its own inbox.
        let key = format!("{KEY}-{n}");
        let held = keyed(&world, OTHER, OTHER, n, &key);
        let answer = raw_post(&world, address, OTHER, &encoded(&held));
        assert!(answer.contains("\"outcome\":\"posted\""), "{answer}");

        let under_held = keyed(&world, THIRD, to, n + 1, &key);
        let under_free = keyed(&world, THIRD, to, n + 2, &format!("free-{n}"));
        let a = raw_post(&world, address, THIRD, &encoded(&under_held));
        let b = raw_post(&world, address, THIRD, &encoded(&under_free));
        assert!(a.starts_with("HTTP/1.1 202"), "to {to}: {a}");
        assert_eq!(
            without_id(&a, &under_held),
            without_id(&b, &under_free),
            "to {to}"
        );
    }
}

// ---- Second adversary agent: the views' `dedup_key` member. ----

/// Every view a client may read, as a third principal, at each step of a request's life (sent,
/// read, answered): no answer carries the key AGENT used for OTHER. The recipient and the sender
/// read it (the control that the key is in the rows at all).
#[test]
fn adv_w7_a_third_principal_reads_no_other_senders_key_from_any_view() {
    use inbox::messaging::{Answer, Body, FreeShape, Kind, MessageId, ReplyShape, Signature};
    use inbox_verify::sign_envelope;

    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    for who in [AGENT, OTHER, THIRD] {
        world.open(address, who);
    }
    let key = format!("{KEY}-views-{}", support::ulid(77));
    let mut request = world.post(AGENT, OTHER, 1, "between two");
    request.kind = Kind::Request;
    request.reply_shape = Some(ReplyShape::Free(FreeShape {}));
    request.dedup_key = DedupKey(key.clone());
    world.sign(&mut request);
    let answer = raw_post(&world, address, AGENT, &encoded(&request));
    assert!(answer.contains("\"outcome\":\"posted\""), "{answer}");
    let id = request.message_id.0.clone();

    let views = [
        format!("/messaging/views/pending?inbox={OTHER}"),
        format!("/messaging/views/unanswered?inbox={OTHER}"),
        format!("/messaging/views/outbox?sender={AGENT}"),
        format!("/messaging/views/settled?sender={AGENT}"),
        format!("/messaging/views/thread?thread={id}"),
        format!("/messaging/views/replies?thread={id}"),
    ];
    let third_reads_nothing = |step: &str| {
        for view in &views {
            let read = world.send(address, THIRD, "GET", view, b"");
            assert_eq!(read.status, 200, "{step} {view}: {}", read.body);
            assert!(!read.body.contains(&key), "{step} {view}: {}", read.body);
        }
    };
    // The sender reads its key in its Outbox at every step.
    let sender_reads_it = |step: &str| {
        let outbox = world.send(address, AGENT, "GET", &views[2], b"");
        assert!(outbox.body.contains(&key), "{step}: {}", outbox.body);
    };

    third_reads_nothing("sent");
    sender_reads_it("sent");
    let pending = world.send(address, OTHER, "GET", &views[0], b"");
    assert!(pending.body.contains(&key), "sent: {}", pending.body);

    let body = format!(r#"{{"message_id":"{id}"}}"#);
    let read = world.send(
        address,
        OTHER,
        "POST",
        "/messaging/commands/read",
        body.as_bytes(),
    );
    assert_eq!(read.text("outcome"), "read", "{}", read.body);
    third_reads_nothing("read");
    sender_reads_it("read");

    let member = world.member(OTHER);
    let mut reply = Answer {
        in_reply_to: MessageId(id.clone()),
        reply_id: MessageId(support::ulid(50)),
        from: Principal(OTHER.to_owned()),
        on_behalf_of: Principal(OTHER.to_owned()),
        authority: None,
        created: request.created.clone(),
        expires: request.expires.clone(),
        option: None,
        body: Body("done".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    sign_envelope(&mut reply, &member.key_id, &member.key).expect("the reply canonicalizes");
    let mut text = String::new();
    wire::encode_command_inbox_messaging_answer(&reply, &mut text);
    let answered = world.send(
        address,
        OTHER,
        "POST",
        "/messaging/commands/answer",
        text.as_bytes(),
    );
    assert_eq!(answered.text("outcome"), "answered", "{}", answered.body);
    third_reads_nothing("answered");
    sender_reads_it("answered");
}
