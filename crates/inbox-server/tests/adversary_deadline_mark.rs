//! Adversary pass on story:a-due-message-takes-no-mark over HTTP: the deadline of a mark at a
//! fraction of a second, held to the expiry task's sweep at the same reading; the answer the
//! sender and a third principal get from a mark, byte for byte the same whether the message is
//! due or not; and what `wait` answers to the recipient of a due message whose read was refused.
//! The clock is the test's [`inbox_server::ManualClock`]; every key is created when the test
//! runs. The `wait` case waits on real time 3 times for the service's bound of a wait, 300 ms:
//! a due row does not end a wait (`waited` in `src/service.rs`).

use crate::support;

use std::net::SocketAddr;

use inbox::behaviour::MessageStorage;
use inbox::messaging::{FreeShape, Kind, ReplyShape};
use inbox::primitives::Timestamp;
use inbox_core::MemoryBackend;
use inbox_server::Running;
use support::{AGENT, LATER, OTHER, THIRD, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
const MARK_DELIVERED: &str = "/messaging/commands/mark-delivered";
const READ: &str = "/messaging/commands/read";
const DUE: &str =
    r#"{"outcome":"deadline-passed","published":[],"error":"inbox.messaging.Expired"}"#;
/// The service's bound of a wait in the `wait` case.
const BOUND: std::time::Duration = std::time::Duration::from_millis(300);

/// The 4 messages of a world: a note and a request, each marked by `read` and by
/// `mark-delivered`, message `n` of the list.
const MESSAGES: [(u32, Kind, &str); 4] = [
    (1, Kind::Note, READ),
    (2, Kind::Note, MARK_DELIVERED),
    (3, Kind::Request, READ),
    (4, Kind::Request, MARK_DELIVERED),
];

fn mark_body(n: u32) -> String {
    format!(r#"{{"message_id":"{}"}}"#, ulid(n))
}

/// Message `n` of `kind` from AGENT to OTHER, due at `expires`.
fn send_message(world: &World, address: SocketAddr, n: u32, kind: Kind, expires: &str) {
    let mut post = world.post(AGENT, OTHER, n, "due soon");
    post.kind = kind;
    if kind == Kind::Request {
        post.reply_shape = Some(ReplyShape::Free(FreeShape {}));
    }
    post.expires = Timestamp(expires.to_owned());
    world.sign(&mut post);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&post));
    assert_eq!(reply.status, 202, "{}", reply.body);
}

/// A started service with OTHER's inbox open and the 4 messages, each due at `expires`.
fn world_with_messages(expires: &str) -> (World, Running<MemoryBackend>) {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    for (n, kind, _) in MESSAGES {
        send_message(&world, address, n, kind, expires);
    }
    (world, server)
}

/// Messages due half a second into a second: a mark a nanosecond before and at that instant is
/// refused exactly where a sweep at that reading removes the messages, and taken where it does
/// not.
#[test]
fn a_mark_at_a_fraction_of_a_second_is_refused_exactly_where_the_sweep_removes_the_message() {
    const EXPIRES: &str = "2026-12-01T00:00:00.5Z";
    for (reading, due) in [
        ("2026-12-01T00:00:00.499999999Z", false),
        ("2026-12-01T00:00:00.5Z", true),
    ] {
        let (swept, server) = world_with_messages(EXPIRES);
        swept.clock.set(instant(reading));
        assert_eq!(
            server.sweep_now(),
            if due { MESSAGES.len() } else { 0 },
            "the sweep of the messages due at {EXPIRES}, at {reading}"
        );

        let (marked, server) = world_with_messages(EXPIRES);
        let address = server.local_addr();
        marked.clock.set(instant(reading));
        for (n, kind, target) in MESSAGES {
            let before = format!("{:?}", MessageStorage::list(&marked.backend));
            let reply = marked.send(address, OTHER, "POST", target, mark_body(n).as_bytes());
            if due {
                assert_eq!(
                    (reply.status, reply.body.as_str()),
                    (502, DUE),
                    "{kind:?} {target} at {reading}"
                );
                assert_eq!(
                    format!("{:?}", MessageStorage::list(&marked.backend)),
                    before
                );
            } else {
                assert_eq!(
                    reply.status, 202,
                    "{kind:?} {target} at {reading}: {}",
                    reply.body
                );
            }
        }
    }
}

/// The sender and a third principal get from `read` and `mark-delivered` the same status and the
/// same bytes a second before the deadline and a second after it: over HTTP no answer but the
/// recipient's tells that a message is due.
#[test]
fn the_sender_and_a_third_principal_cannot_tell_a_due_message_from_a_live_one() {
    let answers: Vec<Vec<(u16, String)>> = ["2026-11-30T23:59:59Z", "2026-12-01T00:00:01Z"]
        .into_iter()
        .map(|reading| {
            let (world, server) = world_with_messages(LATER);
            let address = server.local_addr();
            world.clock.set(instant(reading));
            let mut answers = Vec::new();
            for who in [AGENT, THIRD] {
                // The note (1) and the request (3).
                for n in [1, 3] {
                    for target in [READ, MARK_DELIVERED] {
                        let reply =
                            world.send(address, who, "POST", target, mark_body(n).as_bytes());
                        answers.push((reply.status, format!("{who} {n} {target} {}", reply.body)));
                    }
                }
            }
            answers
        })
        .collect();
    assert_eq!(answers[0], answers[1], "live, then due");
}

/// What the story leaves to a wake loop: once the recipient's `read` of a due note is refused,
/// nothing the recipient can send takes the note out of `Pending` until the expiry task runs
/// (here never; the default `--sweep-interval` is 10 s and the flag takes any number). So `wait`
/// does not end on the due note: every time, it holds until its bound (`max_wait`, 300 ms here)
/// and then lists the note. Rewritten from the adversary's
/// `wait_answers_a_due_note_at_once_every_time_after_its_read_was_refused`, which pinned the
/// answer at once that finding 1 of its report removed.
#[test]
fn wait_holds_a_due_note_to_its_bound_every_time_after_its_read_was_refused() {
    let world = World::new();
    let server = world.start(|config| config.max_wait = BOUND);
    let address = server.local_addr();
    world.open(address, OTHER);
    send_message(&world, address, 1, Kind::Note, LATER);
    world.clock.set(instant("2026-12-01T00:00:01Z"));
    let refused = world.send(address, OTHER, "POST", READ, mark_body(1).as_bytes());
    assert_eq!((refused.status, refused.body.as_str()), (502, DUE));
    let delivered = world.send(
        address,
        OTHER,
        "POST",
        MARK_DELIVERED,
        mark_body(1).as_bytes(),
    );
    assert_eq!((delivered.status, delivered.body.as_str()), (502, DUE));

    let target = format!("/messaging/views/pending?inbox={OTHER}&wait=1");
    for round in 0..3 {
        let started = std::time::Instant::now();
        let reply = world.send(address, OTHER, "GET", &target, b"");
        let held = started.elapsed();
        assert_eq!(reply.status, 200, "{}", reply.body);
        assert_eq!(reply.ids(), [ulid(1)], "round {round}");
        assert!(
            held >= BOUND,
            "round {round}: the wait ended after {held:?}, before its bound"
        );
        let again = world.send(address, OTHER, "POST", READ, mark_body(1).as_bytes());
        assert_eq!(
            (again.status, again.body.as_str()),
            (502, DUE),
            "round {round}"
        );
    }
}
