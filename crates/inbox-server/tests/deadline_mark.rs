//! story:a-due-message-takes-no-mark over HTTP: once the service's clock reaches a message's
//! `expires`, `read` and `mark-delivered` are refused, change nothing and publish nothing, and the
//! expiry task removes the message as before; the mark before the sweep and the sweep before the
//! mark leave the same store. A third principal still finds no message. A wait on `Pending` is
//! not ended by a due row, which its owner can neither read nor mark: it ends at its bound or with
//! a row that is not due, and then lists every row. The clock is the test's
//! [`inbox_server::ManualClock`]; every key is created when the test runs.

use crate::support;

use std::net::SocketAddr;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant as Monotonic};

use inbox::behaviour::MessageStorage;
use inbox::messaging::{Kind, ReplyShape};
use inbox::primitives::Timestamp;
use support::{AGENT, LATER, OTHER, THIRD, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
const MARK_DELIVERED: &str = "/messaging/commands/mark-delivered";
const READ: &str = "/messaging/commands/read";
/// A second after the message's `expires` (`LATER`).
const AFTER: &str = "2026-12-01T00:00:01Z";
const DUE: &str =
    r#"{"outcome":"deadline-passed","published":[],"error":"inbox.messaging.Expired"}"#;

/// A note (or a request of free shape) from AGENT to OTHER, due at `LATER`.
fn send_message(world: &World, address: std::net::SocketAddr, kind: Kind) {
    let mut post = world.post(AGENT, OTHER, 1, "due at LATER");
    if kind == Kind::Request {
        post.kind = Kind::Request;
        post.reply_shape = Some(ReplyShape::Free(inbox::messaging::FreeShape {}));
        world.sign(&mut post);
    }
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&post));
    assert_eq!(reply.status, 202, "{}", reply.body);
}

fn mark_body() -> String {
    format!(r#"{{"message_id":"{}"}}"#, ulid(1))
}

#[test]
fn a_mark_on_a_due_message_is_refused_and_changes_nothing() {
    for (kind, target) in [
        (Kind::Note, READ),
        (Kind::Note, MARK_DELIVERED),
        (Kind::Request, READ),
        (Kind::Request, MARK_DELIVERED),
    ] {
        let world = World::new();
        let server = world.start(|_| {});
        let address = server.local_addr();
        world.open(address, OTHER);
        send_message(&world, address, kind);
        world.clock.set(instant(LATER));
        let before = format!("{:?}", MessageStorage::list(&world.backend));
        let due = world.send(address, OTHER, "POST", target, mark_body().as_bytes());
        assert_eq!(
            (due.status, due.body.as_str()),
            (502, DUE),
            "{kind:?} {target}"
        );
        assert_eq!(
            format!("{:?}", MessageStorage::list(&world.backend)),
            before,
            "{kind:?} {target}: a refused mark changes no row"
        );
        world.clock.advance(-1);
        let taken = world.send(address, OTHER, "POST", target, mark_body().as_bytes());
        assert_eq!(taken.status, 202, "{kind:?} {target}: {}", taken.body);
    }
}

#[test]
fn a_due_note_read_then_swept_and_swept_then_read_leave_the_same_store() {
    for target in [READ, MARK_DELIVERED] {
        let mark_first = World::new();
        let server = mark_first.start(|_| {});
        let address = server.local_addr();
        mark_first.open(address, OTHER);
        send_message(&mark_first, address, Kind::Note);
        mark_first.clock.set(instant(AFTER));
        let late = mark_first.send(address, OTHER, "POST", target, mark_body().as_bytes());
        assert_eq!((late.status, late.body.as_str()), (502, DUE), "{target}");
        assert_eq!(
            server.sweep_now(),
            1,
            "{target}: the sweep finds the note due"
        );
        assert!(MessageStorage::list(&mark_first.backend).is_empty());

        let sweep_first = World::new();
        let server = sweep_first.start(|_| {});
        let address = server.local_addr();
        sweep_first.open(address, OTHER);
        send_message(&sweep_first, address, Kind::Note);
        sweep_first.clock.set(instant(AFTER));
        assert_eq!(server.sweep_now(), 1);
        let gone = sweep_first.send(address, OTHER, "POST", target, mark_body().as_bytes());
        assert_eq!(gone.status, 404, "{target}: {}", gone.body);
        assert_eq!(gone.text("outcome"), "unknown-message");
        assert!(MessageStorage::list(&sweep_first.backend).is_empty());
    }
}

#[test]
fn a_third_principal_marking_a_due_message_finds_no_message() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    send_message(&world, address, Kind::Request);
    world.clock.set(instant(AFTER));
    for target in [READ, MARK_DELIVERED] {
        let foreign = world.send(address, THIRD, "POST", target, mark_body().as_bytes());
        assert_eq!(
            (foreign.status, foreign.body.as_str()),
            (
                404,
                r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#
            ),
            "{target}"
        );
    }
}

// ---- the wait on `Pending` and a due message ---------------------------------------------------

/// The pending view of OTHER held open for `wait` seconds, on a thread; the answer and the instant
/// it arrived come on the channel.
fn wait_on(
    world: &Arc<World>,
    address: SocketAddr,
    wait: &str,
) -> mpsc::Receiver<(support::Reply, Monotonic)> {
    let (sent, answered) = mpsc::channel();
    let world = Arc::clone(world);
    let target = format!("/messaging/views/pending?inbox={OTHER}&wait={wait}");
    thread::spawn(move || {
        let reply = world.send(address, OTHER, "GET", &target, b"");
        let _ = sent.send((reply, Monotonic::now()));
    });
    answered
}

/// Note `n` from AGENT to OTHER that expires at `expires`.
fn send_note(world: &World, address: SocketAddr, n: u32, expires: &str) {
    let mut post = world.post(AGENT, OTHER, n, "a note");
    post.expires = Timestamp(expires.to_owned());
    world.sign(&mut post);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&post));
    assert_eq!(reply.status, 202, "{}", reply.body);
}

/// A day after `AFTER`: a note that expires here is not due at `AFTER`.
const NEXT_DAY: &str = "2026-12-02T00:00:00Z";
/// The service's bound of a wait in the cases that wait for it.
const BOUND: Duration = Duration::from_millis(300);

#[test]
fn a_wait_with_only_a_due_note_ends_at_its_bound_and_lists_the_note() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = BOUND);
    let address = server.local_addr();
    world.open(address, OTHER);
    send_message(&world, address, Kind::Note);
    world.clock.set(instant(AFTER));
    let refused = world.send(address, OTHER, "POST", READ, mark_body().as_bytes());
    assert_eq!((refused.status, refused.body.as_str()), (502, DUE));
    for round in 0..2 {
        let started = Monotonic::now();
        let (reply, at) = wait_on(&world, address, "3600")
            .recv_timeout(Duration::from_secs(5))
            .expect("the wait ends at its bound");
        assert_eq!(reply.status, 200, "{}", reply.body);
        assert_eq!(
            reply.ids(),
            [ulid(1)],
            "round {round}: the due note is listed"
        );
        assert!(
            at - started >= BOUND,
            "round {round}: the wait ended after {:?}, before its bound",
            at - started
        );
    }
}

#[test]
fn a_post_of_a_live_note_ends_a_wait_that_holds_a_due_one() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = Duration::from_secs(10));
    let address = server.local_addr();
    world.open(address, OTHER);
    send_message(&world, address, Kind::Note);
    world.clock.set(instant(AFTER));
    let answered = wait_on(&world, address, "10");
    assert!(
        answered.recv_timeout(BOUND).is_err(),
        "a wait with only a due note is held open"
    );
    let started = Monotonic::now();
    send_note(&world, address, 2, NEXT_DAY);
    let (reply, at) = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the post ends the wait");
    assert_eq!(reply.status, 200, "{}", reply.body);
    let mut ids = reply.ids();
    ids.sort();
    assert_eq!(ids, [ulid(1), ulid(2)]);
    assert!(at - started < Duration::from_secs(5));
}

#[test]
fn a_note_that_is_not_due_ends_a_wait_at_once() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = Duration::from_secs(10));
    let address = server.local_addr();
    world.open(address, OTHER);
    send_note(&world, address, 1, NEXT_DAY);
    world.clock.set(instant(AFTER));
    let started = Monotonic::now();
    let (reply, at) = wait_on(&world, address, "10")
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait is answered at once");
    assert_eq!(reply.ids(), [ulid(1)], "{}", reply.body);
    assert!(at - started < Duration::from_secs(5));
}
