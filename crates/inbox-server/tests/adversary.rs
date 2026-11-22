//! Adversary cases for story:http-service. Every key is created when the test runs.

use crate::support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use inbox::messaging::{DedupKey, DelegationEntry, DelegationId, KeyEntry, KeyFile, Principal};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_core::MemoryBackend;
use inbox_server::{ManualClock, request_headers};
use inbox_verify::{KeyRing, SecretKey, sign_delegation_entry, sign_key_entry};
use support::{
    AGENT, KEYS_VALID_UNTIL, Member, NOW, OTHER, Reply, THIRD, World, encoded, instant, send_raw,
    ulid,
};

const POST: &str = "/messaging/commands/post";

fn pending(principal: &str) -> String {
    format!("/messaging/views/pending?inbox={principal}")
}

/// A connection that sends a request head one byte at a time, a byte every 300 ms, until told
/// to stop: never a second without a byte, so the reader's per-byte timeout never fires.
fn trickle(address: SocketAddr, stop: Arc<AtomicBool>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut stream = TcpStream::connect(address).expect("the service accepts");
        let _ = stream.write_all(b"GET /messaging/views/inboxes HTTP/1.1\r\nX-Slow: ");
        while !stop.load(Ordering::SeqCst) {
            if stream.write_all(b"a").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(300));
        }
    })
}

/// Sends a signed request and reads what arrives; status 0 when the connection closed without
/// an answer.
fn send_signed_tolerant(
    world: &World,
    address: SocketAddr,
    principal: &str,
    target: &str,
) -> Reply {
    let member = world.member(principal);
    let headers = request_headers(
        "GET",
        target,
        b"",
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout");
    let mut head = format!("GET {target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: 0\r\n");
    for (name, value) in &headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).expect("the head is sent");
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    let answer = String::from_utf8_lossy(&answer).into_owned();
    match answer.split_once("\r\n\r\n") {
        Some((head, body)) => Reply {
            status: head
                .split(' ')
                .nth(1)
                .and_then(|status| status.parse().ok())
                .unwrap_or(0),
            body: body.to_owned(),
        },
        None => Reply {
            status: 0,
            body: answer,
        },
    }
}

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

// ---- one client holds the service -------------------------------------------------------------

#[test]
fn adv_a_slow_unauthenticated_connection_does_not_hold_a_signed_request() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_connections = 1);
    let address = server.local_addr();
    let stop = Arc::new(AtomicBool::new(false));
    let slow = trickle(address, Arc::clone(&stop));
    thread::sleep(Duration::from_millis(300));

    let (sent, answered) = mpsc::channel();
    let signed = Arc::clone(&world);
    thread::spawn(move || {
        let reply = signed.send(address, AGENT, "GET", "/messaging/views/inboxes", b"");
        let _ = sent.send(reply);
    });
    let served = answered.recv_timeout(Duration::from_secs(4));
    stop.store(true, Ordering::SeqCst);
    let _ = slow.join();
    let reply = served.expect(
        "a signed request is served within 4 s while one unauthenticated connection sends a \
         byte every 300 ms",
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
}

#[test]
fn adv_stop_returns_while_a_slow_unauthenticated_connection_is_open() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let stop = Arc::new(AtomicBool::new(false));
    let slow = trickle(address, Arc::clone(&stop));
    thread::sleep(Duration::from_millis(300));

    let (sent, stopped) = mpsc::channel();
    thread::spawn(move || {
        let _ = sent.send(server.stop());
    });
    let returned = stopped.recv_timeout(Duration::from_secs(5));
    stop.store(true, Ordering::SeqCst);
    let _ = slow.join();
    let _ = returned.expect(
        "Running::stop returns within 5 s while one unauthenticated connection sends a byte \
         every 300 ms",
    );
}

// ---- the wait ---------------------------------------------------------------------------------

#[test]
fn adv_a_wait_with_an_unbounded_max_wait_is_answered_not_dropped() {
    let world = World::new();
    let server = world.start(|config| config.max_wait = Duration::MAX);
    let address = server.local_addr();
    world.open(address, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "a note");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.status, 202, "{}", posted.body);

    // A row is already there, so the wait answers at its first read.
    let target = format!("{}&wait={}", pending(OTHER), u64::MAX);
    let reply = send_signed_tolerant(&world, address, OTHER, &target);
    assert_eq!(
        reply.status, 200,
        "the wait was answered with status {} (0: the connection closed without an answer): {}",
        reply.status, reply.body
    );
    assert_eq!(reply.ids(), vec![ulid(1)]);
}

// ---- the order of the steps -------------------------------------------------------------------

#[test]
fn adv_an_unauthenticated_body_over_max_body_is_413_before_401() {
    let world = World::new();
    let server = world.start(|config| config.max_body = 64);
    let body = vec![b' '; 100];
    let reply = send_raw(server.local_addr(), "POST", POST, &[], &body);
    assert_eq!(reply.status, 413, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"body too large","limit":64}"#);
}

// ---- the verifier port: delegations -----------------------------------------------------------

#[test]
fn adv_a_post_on_behalf_of_another_with_an_unknown_authority_is_authority_invalid() {
    let world = world_with_delegation();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let mut envelope = world.post(AGENT, OTHER, 1, "for third");
    envelope.on_behalf_of = Principal(THIRD.to_owned());
    envelope.authority = Some(DelegationId("no-such-delegation".to_owned()));
    world.sign(&mut envelope);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(reply.text("outcome"), "authority-invalid", "{}", reply.body);
}

#[test]
fn adv_a_post_on_behalf_of_another_under_its_delegation_is_posted() {
    let world = world_with_delegation();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let mut envelope = world.post(AGENT, OTHER, 1, "for third");
    envelope.on_behalf_of = Principal(THIRD.to_owned());
    envelope.authority = Some(DelegationId("delegation-1".to_owned()));
    world.sign(&mut envelope);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(reply.status, 202, "{}", reply.body);
    assert_eq!(reply.text("outcome"), "posted", "{}", reply.body);

    // The delegation names AGENT for THIRD, not THIRD for AGENT.
    let mut reverse = world.post(THIRD, OTHER, 2, "for agent");
    reverse.on_behalf_of = Principal(AGENT.to_owned());
    reverse.authority = Some(DelegationId("delegation-1".to_owned()));
    world.sign(&mut reverse);
    let reply = world.send(address, THIRD, "POST", POST, &encoded(&reverse));
    assert_eq!(reply.text("outcome"), "authority-invalid", "{}", reply.body);
}

// ---- the caller -------------------------------------------------------------------------------

#[test]
fn adv_a_principal_cannot_post_an_envelope_whose_from_is_another_principal() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "said once");
    let bytes = encoded(&envelope);
    let posted = world.send(address, AGENT, "POST", POST, &bytes);
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);

    // OTHER reads the note: it is handled and removed.
    let read = format!(r#"{{"message_id":"{}"}}"#, ulid(1));
    let reply = world.send(
        address,
        OTHER,
        "POST",
        "/messaging/commands/read",
        read.as_bytes(),
    );
    assert_eq!(reply.text("outcome"), "handled", "{}", reply.body);

    // OTHER, authenticated as itself, posts AGENT's envelope as it received it.
    let replayed = world.send(address, OTHER, "POST", POST, &bytes);
    assert_ne!(
        replayed.status, 202,
        "a caller authenticated as {OTHER} posted an envelope from {AGENT}: {}",
        replayed.body
    );
}

// ---- what a refusal tells a third principal ---------------------------------------------------

#[test]
fn adv_a_third_principal_cannot_tell_a_message_between_others_from_no_message() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "between two");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);

    let read = |id: String| {
        let body = format!(r#"{{"message_id":"{id}"}}"#);
        world.send(
            address,
            THIRD,
            "POST",
            "/messaging/commands/read",
            body.as_bytes(),
        )
    };
    let existing = read(ulid(1));
    let missing = read(ulid(99));
    assert_eq!(
        (existing.status, existing.text("outcome")),
        (missing.status, missing.text("outcome")),
        "existing: {} / missing: {}",
        existing.body,
        missing.body
    );
}

#[test]
fn adv_a_third_principal_cannot_take_another_senders_dedup_key() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    world.open(address, THIRD);

    // THIRD posts into its own inbox under a dedup key.
    let mut own = world.post(THIRD, THIRD, 1, "mine");
    own.dedup_key = DedupKey("deploy-2026-11-21".to_owned());
    world.sign(&mut own);
    let reply = world.send(address, THIRD, "POST", POST, &encoded(&own));
    assert_eq!(reply.text("outcome"), "posted", "{}", reply.body);

    // AGENT posts to OTHER under the same text: a key of its own, between two other principals.
    let mut theirs = world.post(AGENT, OTHER, 2, "theirs");
    theirs.dedup_key = DedupKey("deploy-2026-11-21".to_owned());
    world.sign(&mut theirs);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&theirs));
    assert_eq!(reply.text("outcome"), "posted", "{}", reply.body);
}
