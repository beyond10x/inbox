//! story:foreign-message-id-answers-as-unknown over HTTP: a principal that is neither the sender
//! nor the recipient of a message gets, from each of the 5 commands that address a message, the
//! status and the body bytes a missing id gets, in every state the message is in, and changes
//! nothing; the sender and the recipient get what they got before. Every key is created when the
//! test runs.

mod support;

use std::net::SocketAddr;

use inbox::behaviour::MessageStorage;
use inbox::messaging::{Answer, Body, Kind, MessageId, Principal, ReplyShape, Signature};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_verify::sign_envelope;
use support::{AGENT, LATER, NOW, OTHER, Reply, THIRD, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
const UNKNOWN: &str =
    r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#;
const NOT_RECIPIENT: &str =
    r#"{"outcome":"not-recipient","published":[],"error":"inbox.messaging.NotRecipient"}"#;
const NOT_SENDER: &str =
    r#"{"outcome":"not-sender","published":[],"error":"inbox.messaging.NotSender"}"#;

/// The 5 commands that address a message, by their route.
const ROUTES: [&str; 5] = [
    "/messaging/commands/mark-delivered",
    "/messaging/commands/read",
    "/messaging/commands/answer",
    "/messaging/commands/refuse",
    "/messaging/commands/collect",
];

const MISSING: u32 = 99;

/// The body of `route` for message `n`, as `caller` writes it: an answer is a reply from the
/// caller, signed with its key, so its `from` is the caller's own.
fn body(world: &World, route: &str, n: u32, caller: &str, reply_n: u32) -> Vec<u8> {
    match route {
        "/messaging/commands/answer" => {
            let mut answer = Answer {
                in_reply_to: MessageId(ulid(n)),
                reply_id: MessageId(ulid(reply_n)),
                from: Principal(caller.to_owned()),
                on_behalf_of: Principal(caller.to_owned()),
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
            let member = world.member(caller);
            sign_envelope(&mut answer, &member.key_id, &member.key).expect("canonicalizes");
            let mut out = String::new();
            wire::encode_command_inbox_messaging_answer(&answer, &mut out);
            out.into_bytes()
        }
        "/messaging/commands/refuse" => {
            format!(r#"{{"message_id":"{}","reason":"not mine"}}"#, ulid(n)).into_bytes()
        }
        _ => format!(r#"{{"message_id":"{}"}}"#, ulid(n)).into_bytes(),
    }
}

/// What the store holds, every field of every message, in a form two readings compare by.
fn store(world: &World) -> String {
    let mut held: Vec<String> = MessageStorage::list(&world.backend)
        .iter()
        .map(|message| format!("{message:?}"))
        .collect();
    held.sort();
    held.join("\n")
}

/// Each of the 5 routes as `caller`, for message `n` and for an id no message carries: the 2
/// answers, per route. The store is the same after as before.
fn ask(
    world: &World,
    address: SocketAddr,
    caller: &str,
    n: u32,
) -> Vec<(&'static str, Reply, Reply)> {
    let before = store(world);
    let answers = ROUTES
        .iter()
        .enumerate()
        .map(|(at, route)| {
            let reply_n = 1000 + n * 10 + u32::try_from(at).expect("5 routes");
            let held = world.send(
                address,
                caller,
                "POST",
                route,
                &body(world, route, n, caller, reply_n),
            );
            let missing = world.send(
                address,
                caller,
                "POST",
                route,
                &body(world, route, MISSING, caller, reply_n + 500),
            );
            (*route, held, missing)
        })
        .collect();
    assert_eq!(store(world), before, "{caller} changed the store");
    answers
}

/// THIRD gets, from each route, the status and the bytes of an id no message carries.
fn third_finds_none(world: &World, address: SocketAddr, n: u32, state: &str) {
    for (route, held, missing) in ask(world, address, THIRD, n) {
        assert_eq!(
            (held.status, held.body.as_str()),
            (missing.status, missing.body.as_str()),
            "{state}, {route}"
        );
        assert_eq!(
            (held.status, held.body.as_str()),
            (404, UNKNOWN),
            "{state}, {route}"
        );
    }
}

fn request(world: &World, address: SocketAddr, n: u32) {
    let mut envelope = world.post(AGENT, OTHER, n, "how many lines?");
    envelope.kind = Kind::Request;
    envelope.reply_shape = Some(ReplyShape::Lines(3));
    world.sign(&mut envelope);
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
}

fn command(world: &World, address: SocketAddr, caller: &str, route: &str, n: u32, outcome: &str) {
    let reply = world.send(
        address,
        caller,
        "POST",
        route,
        &body(world, route, n, caller, 900 + n),
    );
    assert_eq!(reply.text("outcome"), outcome, "{route}: {}", reply.body);
}

#[test]
fn a_third_principal_gets_the_bytes_of_a_missing_id_in_every_state_of_a_message() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    request(&world, address, 1);
    request(&world, address, 2);
    let note = world.post(AGENT, OTHER, 3, "short lived");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&note));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);

    third_finds_none(&world, address, 1, "sent");
    command(&world, address, OTHER, ROUTES[0], 1, "delivered");
    third_finds_none(&world, address, 1, "delivered");
    command(&world, address, OTHER, ROUTES[1], 1, "read");
    third_finds_none(&world, address, 1, "read");
    command(&world, address, OTHER, ROUTES[2], 1, "answered");
    third_finds_none(&world, address, 1, "answered");
    command(&world, address, OTHER, ROUTES[3], 2, "refused");
    third_finds_none(&world, address, 2, "refused");
    command(&world, address, AGENT, ROUTES[4], 1, "collected");
    third_finds_none(&world, address, 1, "collected and gone");
    third_finds_none(&world, address, 3, "a note before its expiry");
    world.clock.set(instant(LATER));
    assert!(server.sweep_now() >= 1, "the note is expired");
    assert!(
        MessageStorage::get(&world.backend, &MessageId(ulid(3))).is_none(),
        "the note is gone"
    );
    third_finds_none(&world, address, 3, "expired and gone");
}

#[test]
fn the_sender_and_the_recipient_are_told_their_role_and_a_missing_id_is_unknown() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    request(&world, address, 1);

    // The sender against the 4 commands of the recipient.
    for (route, held, missing) in ask(&world, address, AGENT, 1) {
        let expected = if route == ROUTES[4] {
            // Collect by the sender of a message not yet settled.
            (
                422,
                r#"{"outcome":"not-settled","published":[],"error":"inbox.messaging.NotSettled"}"#,
            )
        } else {
            (403, NOT_RECIPIENT)
        };
        assert_eq!(
            (held.status, held.body.as_str()),
            expected,
            "sender, {route}"
        );
        assert_eq!(
            (missing.status, missing.body.as_str()),
            (404, UNKNOWN),
            "sender, {route}"
        );
    }
    // The recipient against Collect.
    let collected = world.send(
        address,
        OTHER,
        "POST",
        ROUTES[4],
        &body(&world, ROUTES[4], 1, OTHER, 0),
    );
    assert_eq!(
        (collected.status, collected.body.as_str()),
        (403, NOT_SENDER)
    );
    let missing = world.send(
        address,
        OTHER,
        "POST",
        ROUTES[4],
        &body(&world, ROUTES[4], MISSING, OTHER, 0),
    );
    assert_eq!((missing.status, missing.body.as_str()), (404, UNKNOWN));
}

#[test]
fn an_answer_whose_from_is_not_the_third_principal_is_still_not_sender() {
    // The input guard of Answer reads no row: it stays first, for a message between others too.
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    request(&world, address, 1);
    for n in [1, MISSING] {
        let bytes = body(&world, ROUTES[2], n, OTHER, 50 + n);
        let reply = world.send(address, THIRD, "POST", ROUTES[2], &bytes);
        assert_eq!(
            (reply.status, reply.body.as_str()),
            (403, NOT_SENDER),
            "message {n}"
        );
    }
}

#[test]
fn a_third_principals_refusals_show_in_no_view() {
    // A refusal publishes nothing: the recipient's Pending reads the same before and after, and
    // the third principal reads no row of the sender's Outbox.
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    request(&world, address, 1);
    let pending = format!("/messaging/views/pending?inbox={OTHER}");
    let before = world.send(address, OTHER, "GET", &pending, b"");
    third_finds_none(&world, address, 1, "sent");
    let after = world.send(address, OTHER, "GET", &pending, b"");
    assert_eq!((before.status, before.body), (after.status, after.body));
    let outbox = format!("/messaging/views/outbox?sender={AGENT}");
    let read = world.send(address, THIRD, "GET", &outbox, b"");
    assert_eq!(read.status, 200, "{}", read.body);
    assert!(read.rows().is_empty(), "{}", read.body);
}
