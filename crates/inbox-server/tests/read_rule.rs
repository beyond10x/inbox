//! The read rule is one rule for both entries (story:the-core-applies-the-read-rule): for the
//! sender, the recipient and a third principal, `inbox_core::handle` and the HTTP service answer
//! the same rows of each of the 6 views of messages; the wait on `Pending` wakes only for a row of
//! the caller; the expiry task still reads every due row.

use crate::support;

use std::net::SocketAddr;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::messaging::{Address, Answer, Body, Kind, MessageId, Principal, ReplyShape, Signature};
use inbox::primitives::Timestamp;
use inbox::server::json::Value;
use inbox::server::wire;
use inbox_core::{CallerAttributes, ServiceContext};
use inbox_server::{RingVerifier, sweep};
use inbox_verify::sign_envelope;
use support::{AGENT, LATER, NOW, OTHER, Reply, THIRD, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";

/// A request from AGENT to OTHER, signed.
fn request(world: &World, address: SocketAddr, n: u32) {
    let mut envelope = world.post(AGENT, OTHER, n, "Which crate holds the retry rule?");
    envelope.kind = Kind::Request;
    envelope.reply_shape = Some(ReplyShape::Lines(3));
    world.sign(&mut envelope);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(reply.status, 202, "{}", reply.body);
}

fn command(world: &World, address: SocketAddr, principal: &str, route: &str, body: &[u8]) {
    let reply = world.send(address, principal, "POST", route, body);
    assert_eq!(reply.status, 202, "{route}: {}", reply.body);
}

/// Three requests from AGENT to OTHER: message 1 waiting, 2 read, 3 answered.
fn three_requests(world: &World, address: SocketAddr) {
    world.open(address, OTHER);
    world.open(address, THIRD);
    for n in 1..=3 {
        request(world, address, n);
    }
    for n in [2, 3] {
        let body = format!(r#"{{"message_id":"{}"}}"#, ulid(n));
        command(
            world,
            address,
            OTHER,
            "/messaging/commands/read",
            body.as_bytes(),
        );
    }
    let mut answer = Answer {
        in_reply_to: MessageId(ulid(3)),
        reply_id: MessageId(ulid(4)),
        from: Principal(OTHER.to_owned()),
        on_behalf_of: Principal(OTHER.to_owned()),
        authority: None,
        created: Timestamp(NOW.to_owned()),
        expires: Timestamp(LATER.to_owned()),
        option: None,
        body: Body("crates/inbox-core".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    let member = world.member(OTHER);
    sign_envelope(&mut answer, &member.key_id, &member.key).expect("the reply canonicalizes");
    let mut body = String::new();
    wire::encode_command_inbox_messaging_answer(&answer, &mut body);
    command(
        world,
        address,
        OTHER,
        "/messaging/commands/answer",
        body.as_bytes(),
    );
}

/// Each view of messages a client reads: its qualified name, its route, and its parameter.
fn views() -> Vec<(&'static str, &'static str, (&'static str, String))> {
    let mut views = Vec::new();
    for (name, route) in [
        ("inbox.messaging.Pending", "pending"),
        ("inbox.messaging.Unanswered", "unanswered"),
    ] {
        for inbox in [OTHER, AGENT, THIRD] {
            views.push((name, route, ("inbox", inbox.to_owned())));
        }
    }
    for (name, route) in [
        ("inbox.messaging.Outbox", "outbox"),
        ("inbox.messaging.Settled", "settled"),
    ] {
        for sender in [AGENT, OTHER, THIRD] {
            views.push((name, route, ("sender", sender.to_owned())));
        }
    }
    for (name, route) in [
        ("inbox.messaging.Thread", "thread"),
        ("inbox.messaging.Replies", "replies"),
    ] {
        for n in 1..=3 {
            views.push((name, route, ("thread", ulid(n))));
        }
    }
    views
}

/// The rows `inbox_core::handle` answers `principal` (its principal and inbox the same text, as
/// the service makes its caller).
fn core_rows(world: &World, principal: &str, view: &str, param: &(&str, String)) -> Vec<Value> {
    let caller = CallerAttributes::new(
        Principal(principal.to_owned()),
        Address(principal.to_owned()),
    );
    let verifier = RingVerifier::new(Arc::new(world.ring.clone()), instant(NOW));
    let context = ServiceContext::new(
        caller,
        verifier,
        world.backend.clone(),
        inbox::primitives::Timestamp(NOW.to_owned()),
    );
    let params = Value::Object(vec![(param.0.to_owned(), Value::Text(param.1.clone()))]);
    let answer = inbox_core::handle(
        &world.backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        view,
        params,
    )
    .unwrap_or_else(|refused| panic!("{view} as {principal}: {refused}"));
    match answer.member("rows") {
        Some(Value::Array(rows)) => rows.clone(),
        other => panic!("{view} answered no rows: {other:?}"),
    }
}

#[test]
fn handle_and_the_service_answer_each_party_and_a_third_the_same_rows() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    three_requests(&world, address);
    let mut seen = 0;
    for principal in [AGENT, OTHER, THIRD] {
        for (view, route, param) in views() {
            let target = format!("/messaging/views/{route}?{}={}", param.0, param.1);
            let wire = world.send(address, principal, "GET", &target, b"");
            assert_eq!(wire.status, 200, "{principal} {target}: {}", wire.body);
            let core = core_rows(&world, principal, view, &param);
            assert_eq!(
                wire.rows(),
                core,
                "{principal} {target}: the service and inbox_core::handle differ"
            );
            if principal == THIRD {
                assert!(core.is_empty(), "{target} answered the third: {core:?}");
            }
            seen += core.len();
        }
    }
    // The parties read rows: the comparison is not of empty answers only.
    assert!(seen >= 12, "{seen} rows in all");
}

/// Starts a wait of `principal` on the `Pending` of `inbox`; its answer arrives on the channel.
fn wait_on(
    world: &Arc<World>,
    address: SocketAddr,
    principal: &'static str,
    inbox: &'static str,
) -> mpsc::Receiver<Reply> {
    let (sent, answered) = mpsc::channel();
    let world = Arc::clone(world);
    let target = format!("/messaging/views/pending?inbox={inbox}&wait=2");
    thread::spawn(move || {
        let _ = sent.send(world.send(address, principal, "GET", &target, b""));
    });
    answered
}

#[test]
fn a_wait_on_another_inbox_does_not_wake_for_its_rows() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = Duration::from_secs(2));
    let address = server.local_addr();
    world.open(address, OTHER);
    let third = wait_on(&world, address, THIRD, OTHER);
    let owner = wait_on(&world, address, OTHER, OTHER);
    assert!(owner.recv_timeout(Duration::from_millis(300)).is_err());
    let envelope = world.post(AGENT, OTHER, 1, "a note");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.status, 202, "{}", posted.body);
    // The owner's wait wakes for the row; the third's ends at its bound with none.
    let reply = owner
        .recv_timeout(Duration::from_millis(1500))
        .expect("the owner's wait wakes for its row");
    assert_eq!(reply.ids(), vec![ulid(1)], "{}", reply.body);
    let reply = third
        .recv_timeout(Duration::from_secs(5))
        .expect("the third's wait ends at its bound");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(reply.rows().is_empty(), "{}", reply.body);
}

#[test]
fn the_expiry_task_reads_every_due_row_whoever_it_is_between() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    three_requests(&world, address);
    world.open(address, AGENT);
    let envelope = world.post(THIRD, AGENT, 5, "a note");
    let posted = world.send(address, THIRD, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.status, 202, "{}", posted.body);
    let keys = Arc::new(world.ring.clone());
    let expired = sweep(
        &world.backend,
        &keys,
        instant(LATER),
        Duration::from_secs(60),
    );
    assert_eq!(expired, 4, "the sweep sent Expire for every due message");
}
