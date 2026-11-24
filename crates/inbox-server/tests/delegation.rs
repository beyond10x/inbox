//! story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal over HTTP: a request that
//! names the inbox it acts for (`Inbox-On-Behalf-Of`, the sixth signed member) is admitted only
//! under a live delegation of the pair (the key's principal, that inbox), its caller then has
//! `principal` the signer and `inbox` the named address, and under the header a reply speaks for
//! the person of that inbox (`docs/design/service.md`, "One request, in order", "The request
//! signature", "The caller", "The wait"). AGENT is the agent, OTHER the person, THIRD a sender.
//! Every key is created when the test runs.

use crate::support;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant as Monotonic};

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{
    Address, Answer, Body, DecisionAsk, DecisionOption, DelegationId, InboxState, Kind, MessageId,
    MessageState, OptionLetter, Principal, ReplyShape, Signature,
};
use inbox::primitives::Timestamp;
use inbox::server::json::Value;
use inbox::server::wire;
use inbox_server::{Clock, ManualClock, ON_BEHALF_OF_HEADER};
use inbox_verify::{Instant, sign_envelope};
use support::{
    AGENT, LATER, NOW, OTHER, Reply, THIRD, World, encoded, instant, send_raw, send_raw_answer,
    ulid,
};

const POST: &str = "/messaging/commands/post";
const MARK: &str = "/messaging/commands/mark-delivered";
const READ: &str = "/messaging/commands/read";
const ANSWER: &str = "/messaging/commands/answer";
const REFUSE: &str = "/messaging/commands/refuse";
const OPEN: &str = "/messaging/commands/open-inbox";
const CLOSE: &str = "/messaging/commands/close-inbox";

const UNAUTHENTICATED: &str = r#"{"refused":"unauthenticated"}"#;
const AUTHORITY_INVALID: &str =
    r#"{"outcome":"authority-invalid","published":[],"error":"inbox.messaging.NoAuthority"}"#;
const UNKNOWN: &str =
    r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#;
const NOT_OWNER: &str =
    r#"{"outcome":"not-owner","published":[],"error":"inbox.messaging.NotRecipient"}"#;
const UNKNOWN_INBOX: &str =
    r#"{"outcome":"unknown-inbox","published":[],"error":"inbox.messaging.UnknownInbox"}"#;

/// The delegation AGENT for OTHER, live until `UNTIL`.
const D: &str = "agent-for-other";
/// The instant `D` lapses at; keys and messages live past it.
const UNTIL: &str = "2026-11-25T00:00:00Z";
/// A second before `UNTIL`.
const BEFORE_UNTIL: &str = "2026-11-24T23:59:59Z";

/// The world of most cases: `D` in the key file.
fn delegated() -> World {
    World::with_delegations(&[(D, AGENT, OTHER, UNTIL)])
}

fn pending(inbox: &str) -> String {
    format!("/messaging/views/pending?inbox={inbox}")
}

fn id_body(n: u32) -> Vec<u8> {
    format!(r#"{{"message_id":"{}"}}"#, ulid(n)).into_bytes()
}

/// A post of `kind` from `from` to `to`, sent without the header; asserts it was posted.
fn post(world: &World, address: SocketAddr, from: &str, to: &str, n: u32, kind: Kind) {
    let mut envelope = world.post(from, to, n, "a body");
    envelope.kind = kind;
    match kind {
        Kind::Request => envelope.reply_shape = Some(ReplyShape::Lines(3)),
        Kind::Decision => {
            envelope.reply_shape = Some(ReplyShape::Free(inbox::messaging::FreeShape {}));
            envelope.decision = Some(DecisionAsk {
                options: ["A", "B"]
                    .into_iter()
                    .map(|letter| DecisionOption {
                        letter: OptionLetter(letter.to_owned()),
                        text: format!("option {letter}"),
                    })
                    .collect(),
                recommended_option: Some(OptionLetter("A".to_owned())),
                request_ref: None,
            });
        }
        Kind::Note => {}
    }
    world.sign(&mut envelope);
    let posted = world.send(address, from, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
}

/// `route` for message `n` by `caller` without the header; asserts the outcome.
fn own(world: &World, address: SocketAddr, caller: &str, route: &str, n: u32, outcome: &str) {
    let reply = world.send(address, caller, "POST", route, &id_body(n));
    assert_eq!(reply.text("outcome"), outcome, "{route}: {}", reply.body);
}

/// The state of message `n`, or `None` when the store holds no such message.
fn state(world: &World, n: u32) -> Option<MessageState> {
    MessageStorage::get(&world.backend, &MessageId(ulid(n))).map(|held| held.state)
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

/// A reply to message `n` with option A, signed with the key of `from`.
fn reply(n: u32, reply_n: u32, from: &str, on_behalf_of: &str, authority: Option<&str>) -> Answer {
    Answer {
        in_reply_to: MessageId(ulid(n)),
        reply_id: MessageId(ulid(reply_n)),
        from: Principal(from.to_owned()),
        on_behalf_of: Principal(on_behalf_of.to_owned()),
        authority: authority.map(|id| DelegationId(id.to_owned())),
        created: Timestamp(NOW.to_owned()),
        expires: Timestamp(LATER.to_owned()),
        option: Some(OptionLetter("A".to_owned())),
        body: Body("A it is".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    }
}

fn signed(world: &World, mut answer: Answer) -> Answer {
    let member = world.member(&answer.from.0);
    sign_envelope(&mut answer, &member.key_id, &member.key).expect("canonicalizes");
    answer
}

fn wire_answer(answer: &Answer) -> Vec<u8> {
    let mut out = String::new();
    wire::encode_command_inbox_messaging_answer(answer, &mut out);
    out.into_bytes()
}

/// The signed reply from AGENT for OTHER under `D` to message `n`.
fn agents_reply(world: &World, n: u32, reply_n: u32) -> Vec<u8> {
    wire_answer(&signed(world, reply(n, reply_n, AGENT, OTHER, Some(D))))
}

/// THIRD's note `n` in OTHER's inbox, its request `n + 1` delivered by OTHER, and its decision
/// `n + 2` delivered and read by OTHER: in `Sent`, `Delivered` and `Read`.
fn three(world: &World, address: SocketAddr, n: u32) {
    post(world, address, THIRD, OTHER, n, Kind::Note);
    post(world, address, THIRD, OTHER, n + 1, Kind::Request);
    own(world, address, OTHER, MARK, n + 1, "delivered");
    post(world, address, THIRD, OTHER, n + 2, Kind::Decision);
    own(world, address, OTHER, MARK, n + 2, "delivered");
    own(world, address, OTHER, READ, n + 2, "read");
}

/// The 4 requests of the story by AGENT for OTHER's inbox, on the messages of [`three`]: a
/// `Pending` read, the delivery mark of note `n`, the read of request `n + 1`, the answer of
/// decision `n + 2`.
fn four(world: &World, address: SocketAddr, n: u32) -> [(&'static str, Reply); 4] {
    let as_agent = |route: &'static str, method: &str, target: &str, body: &[u8]| {
        (
            route,
            world.send_for(address, AGENT, OTHER, method, target, body),
        )
    };
    [
        as_agent("pending", "GET", &pending(OTHER), b""),
        as_agent("mark-delivered", "POST", MARK, &id_body(n)),
        as_agent("read", "POST", READ, &id_body(n + 1)),
        as_agent(
            "answer",
            "POST",
            ANSWER,
            &agents_reply(world, n + 2, 500 + n),
        ),
    ]
}

/// Each of the 4 requests on the messages `n` to `n + 2` is answered `401` as a bad signature,
/// and no message changes.
fn refused_alike(world: &World, address: SocketAddr, n: u32, why: &str) {
    let before = store(world);
    for (route, answered) in four(world, address, n) {
        assert_eq!(
            (answered.status, answered.body.as_str()),
            (401, UNAUTHENTICATED),
            "{why}: {route}"
        );
    }
    assert_eq!(store(world), before, "{why}: the store changed");
    assert_eq!(state(world, n), Some(MessageState::Sent), "{why}");
    assert_eq!(state(world, n + 1), Some(MessageState::Delivered), "{why}");
    assert_eq!(state(world, n + 2), Some(MessageState::Read), "{why}");
}

// ---- the checks of the story ---------------------------------------------------------------

#[test]
fn a_pending_read_for_the_person_answers_the_rows_of_its_inbox() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Note);
    post(&world, address, THIRD, OTHER, 2, Kind::Note);
    let read = world.send_for(address, AGENT, OTHER, "GET", &pending(OTHER), b"");
    assert_eq!(read.status, 200, "{}", read.body);
    assert_eq!(read.ids(), [ulid(1), ulid(2)]);
    let without = world.send(address, AGENT, "GET", &pending(OTHER), b"");
    assert_eq!(without.status, 200, "{}", without.body);
    assert!(without.ids().is_empty(), "{}", without.body);
}

#[test]
fn a_delivery_mark_for_the_person_moves_its_message_to_delivered() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Note);
    let without = world.send(address, AGENT, "POST", MARK, &id_body(1));
    assert_eq!((without.status, without.body.as_str()), (404, UNKNOWN));
    assert_eq!(state(&world, 1), Some(MessageState::Sent));
    let marked = world.send_for(address, AGENT, OTHER, "POST", MARK, &id_body(1));
    assert_eq!(marked.text("outcome"), "delivered", "{}", marked.body);
    assert_eq!(state(&world, 1), Some(MessageState::Delivered));
}

#[test]
fn a_read_for_the_person_reads_a_request_and_handles_a_note() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Request);
    own(&world, address, OTHER, MARK, 1, "delivered");
    post(&world, address, THIRD, OTHER, 2, Kind::Note);
    for n in [1, 2] {
        let without = world.send(address, AGENT, "POST", READ, &id_body(n));
        assert_eq!(
            (without.status, without.body.as_str()),
            (404, UNKNOWN),
            "{n}"
        );
    }
    let read = world.send_for(address, AGENT, OTHER, "POST", READ, &id_body(1));
    assert_eq!(read.text("outcome"), "read", "{}", read.body);
    assert_eq!(state(&world, 1), Some(MessageState::Read));
    let handled = world.send_for(address, AGENT, OTHER, "POST", READ, &id_body(2));
    assert_eq!(handled.text("outcome"), "handled", "{}", handled.body);
    assert_eq!(state(&world, 2), None, "the note is gone");
}

#[test]
fn an_answer_for_the_person_moves_its_decision_to_answered() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Decision);
    own(&world, address, OTHER, MARK, 1, "delivered");
    own(&world, address, OTHER, READ, 1, "read");
    let body = agents_reply(&world, 1, 101);
    let without = world.send(address, AGENT, "POST", ANSWER, &body);
    assert_eq!((without.status, without.body.as_str()), (404, UNKNOWN));
    assert_eq!(state(&world, 1), Some(MessageState::Read));

    let answered = world.send_for(address, AGENT, OTHER, "POST", ANSWER, &body);
    assert_eq!(answered.text("outcome"), "answered", "{}", answered.body);
    let outbox = world.send(
        address,
        THIRD,
        "GET",
        &format!("/messaging/views/outbox?sender={THIRD}"),
        b"",
    );
    let rows = outbox.rows();
    assert_eq!(rows.len(), 1, "{}", outbox.body);
    let row = &rows[0];
    let text = |value: Option<&Value>| match value {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("{other:?} in {}", outbox.body),
    };
    assert_eq!(text(row.member("state")), "Answered");
    assert_eq!(text(row.member("answered_by")), "principal");
    assert_eq!(text(row.member("decided_option")), "A");
    let held = row.member("reply").expect("the row carries the reply");
    assert_eq!(text(held.member("from")), AGENT);
    assert_eq!(text(held.member("on_behalf_of")), OTHER);
    assert_eq!(text(held.member("authority")), D);
}

#[test]
fn without_a_delegation_the_four_requests_are_401_and_the_messages_stay() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    three(&world, address, 1);
    refused_alike(&world, address, 1, "no delegation");
}

#[test]
fn a_delegation_at_its_valid_until_is_lapsed_and_one_second_before_is_live() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    three(&world, address, 1);
    three(&world, address, 11);

    world.clock.set(instant(BEFORE_UNTIL));
    let [pending, marked, read, answered] = four(&world, address, 11);
    assert_eq!(pending.1.status, 200, "{}", pending.1.body);
    assert_eq!(
        pending.1.ids(),
        [ulid(1), ulid(2), ulid(11), ulid(12)],
        "the rows in Sent and Delivered"
    );
    assert_eq!(marked.1.text("outcome"), "delivered", "{}", marked.1.body);
    assert_eq!(read.1.text("outcome"), "read", "{}", read.1.body);
    assert_eq!(
        answered.1.text("outcome"),
        "answered",
        "{}",
        answered.1.body
    );

    world.clock.set(instant(UNTIL));
    refused_alike(&world, address, 1, "at valid_until");
}

#[test]
fn a_delegation_of_another_agent_is_401() {
    let world = World::with_delegations(&[("third-for-other", THIRD, OTHER, UNTIL)]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    three(&world, address, 1);
    refused_alike(&world, address, 1, "another agent's");
}

#[test]
fn a_delegation_for_another_person_is_401() {
    let world = World::with_delegations(&[("agent-for-third", AGENT, THIRD, UNTIL)]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    three(&world, address, 1);
    refused_alike(&world, address, 1, "another person's");
}

#[test]
fn a_request_without_the_header_is_answered_as_before() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, AGENT);
    world.open(address, OTHER);
    post(&world, address, THIRD, AGENT, 1, Kind::Note);
    post(&world, address, THIRD, OTHER, 2, Kind::Note);
    let mine = world.send(address, AGENT, "GET", &pending(AGENT), b"");
    assert_eq!(mine.ids(), [ulid(1)], "{}", mine.body);
    let persons = world.send(address, AGENT, "GET", &pending(OTHER), b"");
    assert!(persons.ids().is_empty(), "{}", persons.body);
    own(&world, address, AGENT, MARK, 1, "delivered");
    let foreign = world.send(address, AGENT, "POST", MARK, &id_body(2));
    assert_eq!((foreign.status, foreign.body.as_str()), (404, UNKNOWN));
    assert_eq!(state(&world, 2), Some(MessageState::Sent));
}

// ---- a reply under the header ----------------------------------------------------------------

/// The world of the reply cases: `D`, and `D2` AGENT for THIRD, both live; OTHER's and THIRD's
/// inboxes open; THIRD's decision 1 to OTHER in `Read`.
fn replying() -> (World, inbox_server::Running<inbox_core::MemoryBackend>) {
    let world = World::with_delegations(&[(D, AGENT, OTHER, UNTIL), ("d2", AGENT, THIRD, UNTIL)]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    world.open(address, THIRD);
    post(&world, address, THIRD, OTHER, 1, Kind::Decision);
    own(&world, address, OTHER, MARK, 1, "delivered");
    own(&world, address, OTHER, READ, 1, "read");
    (world, server)
}

/// Sends `answer` as AGENT for OTHER's inbox and asserts its status and body; the store does not
/// change.
fn refused_reply(world: &World, address: SocketAddr, answer: &Answer, expected: (u16, &str)) {
    let before = store(world);
    let answered = world.send_for(address, AGENT, OTHER, "POST", ANSWER, &wire_answer(answer));
    assert_eq!(
        (answered.status, answered.body.as_str()),
        expected,
        "{answer:?}"
    );
    assert_eq!(store(world), before, "{answer:?}");
}

#[test]
fn a_reply_under_the_delegation_is_still_held_to_the_model() {
    let (world, server) = replying();
    let address = server.local_addr();
    let not_sender = (
        403,
        r#"{"outcome":"not-sender","published":[],"error":"inbox.messaging.NotSender"}"#,
    );
    let no_authority = (
        422,
        r#"{"outcome":"no-authority","published":[],"error":"inbox.messaging.NoAuthority"}"#,
    );
    let cases = [
        // `from` the person.
        (reply(1, 201, OTHER, OTHER, None), not_sender),
        // `on_behalf_of` another principal, under the delegation of the person.
        (
            reply(1, 202, AGENT, THIRD, Some(D)),
            (502, AUTHORITY_INVALID),
        ),
        // `on_behalf_of` another principal the agent holds a live delegation of.
        (
            reply(1, 203, AGENT, THIRD, Some("d2")),
            (502, AUTHORITY_INVALID),
        ),
        // `on_behalf_of` the agent itself, with no authority.
        (reply(1, 204, AGENT, AGENT, None), (502, AUTHORITY_INVALID)),
        // `on_behalf_of` the person, with no authority.
        (reply(1, 205, AGENT, OTHER, None), no_authority),
    ];
    for (answer, expected) in cases {
        refused_reply(&world, address, &signed(&world, answer), expected);
        assert_eq!(state(&world, 1), Some(MessageState::Read));
    }
}

#[test]
fn a_reply_that_breaks_two_members_is_answered_by_the_first_in_the_order_of_answer() {
    let (world, server) = replying();
    let address = server.local_addr();
    // OTHER's decision 2 to THIRD's inbox: a message of another inbox than the person's.
    post(&world, address, OTHER, THIRD, 2, Kind::Decision);
    // `from` and `on_behalf_of` both wrong.
    refused_reply(
        &world,
        address,
        &signed(&world, reply(1, 301, OTHER, THIRD, Some("d2"))),
        (
            403,
            r#"{"outcome":"not-sender","published":[],"error":"inbox.messaging.NotSender"}"#,
        ),
    );
    // `on_behalf_of` wrong and a signature that does not verify.
    let mut tampered = signed(&world, reply(1, 302, AGENT, THIRD, Some("d2")));
    tampered.body = Body("B after all".to_owned());
    refused_reply(
        &world,
        address,
        &tampered,
        (
            502,
            r#"{"outcome":"signature-invalid","published":[],"error":"inbox.messaging.SignatureInvalid"}"#,
        ),
    );
    // `on_behalf_of` wrong, to an id no message carries and to a message of another inbox:
    // answered before the message is read.
    for n in [99, 2] {
        refused_reply(
            &world,
            address,
            &signed(&world, reply(n, 303 + n, AGENT, THIRD, Some("d2"))),
            (502, AUTHORITY_INVALID),
        );
    }
    assert_eq!(state(&world, 1), Some(MessageState::Read));
}

#[test]
fn without_the_header_a_reply_on_behalf_of_another_principal_is_answered_as_before() {
    let world = World::with_delegations(&[("d2", AGENT, THIRD, UNTIL)]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, AGENT);
    post(&world, address, OTHER, AGENT, 1, Kind::Decision);
    own(&world, address, AGENT, MARK, 1, "delivered");
    own(&world, address, AGENT, READ, 1, "read");
    let answer = signed(&world, reply(1, 401, AGENT, THIRD, Some("d2")));
    let answered = world.send(address, AGENT, "POST", ANSWER, &wire_answer(&answer));
    assert_eq!(answered.text("outcome"), "answered", "{}", answered.body);
}

#[test]
fn a_reply_may_name_another_live_delegation_of_the_pair_and_not_a_lapsed_one() {
    let world = World::with_delegations(&[
        (D, AGENT, OTHER, UNTIL),
        ("d1-lapsed", AGENT, OTHER, NOW),
        ("d3", AGENT, OTHER, LATER),
    ]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Decision);
    own(&world, address, OTHER, MARK, 1, "delivered");
    own(&world, address, OTHER, READ, 1, "read");
    refused_reply(
        &world,
        address,
        &signed(&world, reply(1, 501, AGENT, OTHER, Some("d1-lapsed"))),
        (502, AUTHORITY_INVALID),
    );
    let answer = signed(&world, reply(1, 502, AGENT, OTHER, Some("d3")));
    let answered = world.send_for(address, AGENT, OTHER, "POST", ANSWER, &wire_answer(&answer));
    assert_eq!(answered.text("outcome"), "answered", "{}", answered.body);
}

// ---- the header inside the signature -----------------------------------------------------------

/// A world with `D` and OTHER's note 1 in its open inbox, and the answer to a `Pending` read of
/// OTHER's inbox by AGENT with these headers: the 3 that sign it over `member`, then `extra`.
fn read_with(member: Option<&str>, extra: &[(&str, &str)]) -> Reply {
    let world = World::with_delegations(&[(D, AGENT, OTHER, UNTIL), ("d4", AGENT, THIRD, UNTIL)]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Note);
    let target = pending(OTHER);
    let mut headers: Vec<(&str, String)> = world
        .signed_headers(AGENT, "GET", &target, b"", member)
        .into_iter()
        .collect();
    headers.extend(extra.iter().map(|&(name, value)| (name, value.to_owned())));
    send_raw(address, "GET", &target, &headers, b"")
}

fn unauthenticated(reply: &Reply, why: &str) {
    assert_eq!(
        (reply.status, reply.body.as_str()),
        (401, UNAUTHENTICATED),
        "{why}"
    );
}

#[test]
fn the_header_and_the_signed_member_admit_a_read_together() {
    let reply = read_with(Some(OTHER), &[(ON_BEHALF_OF_HEADER, OTHER)]);
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(reply.ids(), [ulid(1)]);
    // The header name is case-insensitive, as HTTP has it.
    let reply = read_with(Some(OTHER), &[("inbox-on-behalf-of", OTHER)]);
    assert_eq!(reply.ids(), [ulid(1)], "{}", reply.body);
}

#[test]
fn signed_without_the_member_and_sent_with_the_header_is_401() {
    unauthenticated(
        &read_with(None, &[(ON_BEHALF_OF_HEADER, OTHER)]),
        "5 signed, header sent",
    );
}

#[test]
fn signed_with_the_member_and_sent_without_the_header_is_401() {
    unauthenticated(&read_with(Some(OTHER), &[]), "6 signed, no header");
}

#[test]
fn a_header_other_than_the_signed_member_is_401() {
    // Both pairs are live: only the mismatch refuses.
    unauthenticated(
        &read_with(Some(OTHER), &[(ON_BEHALF_OF_HEADER, THIRD)]),
        "signed OTHER, sent THIRD",
    );
    unauthenticated(
        &read_with(Some(THIRD), &[(ON_BEHALF_OF_HEADER, OTHER)]),
        "signed THIRD, sent OTHER",
    );
    // The reader trims the header: a member with a trailing space differs from what arrives.
    let spaced = format!("{OTHER} ");
    unauthenticated(
        &read_with(Some(&spaced), &[(ON_BEHALF_OF_HEADER, &spaced)]),
        "a trailing space",
    );
}

#[test]
fn the_header_twice_is_401() {
    // Both values are the signed one and the pair is live: the first or the last would admit it.
    unauthenticated(
        &read_with(
            Some(OTHER),
            &[(ON_BEHALF_OF_HEADER, OTHER), (ON_BEHALF_OF_HEADER, OTHER)],
        ),
        "the same value twice",
    );
    unauthenticated(
        &read_with(
            Some(OTHER),
            &[(ON_BEHALF_OF_HEADER, OTHER), ("inbox-on-behalf-of", THIRD)],
        ),
        "the signed value first",
    );
    unauthenticated(
        &read_with(
            Some(OTHER),
            &[(ON_BEHALF_OF_HEADER, THIRD), ("inbox-on-behalf-of", OTHER)],
        ),
        "the signed value last",
    );
}

#[test]
fn an_empty_header_is_401_and_never_read_as_absent() {
    unauthenticated(
        &read_with(None, &[(ON_BEHALF_OF_HEADER, "")]),
        "5 signed, empty",
    );
    unauthenticated(
        &read_with(None, &[(ON_BEHALF_OF_HEADER, "   ")]),
        "5 signed, spaces",
    );
    unauthenticated(
        &read_with(Some(""), &[(ON_BEHALF_OF_HEADER, "")]),
        "an empty member signed",
    );
}

#[test]
fn the_signers_own_address_needs_a_delegation_too() {
    let read_own = |world: &World| {
        let server = world.start(|_| {});
        let address = server.local_addr();
        world.open(address, AGENT);
        post(world, address, THIRD, AGENT, 1, Kind::Note);
        world.send_for(address, AGENT, AGENT, "GET", &pending(AGENT), b"")
    };
    unauthenticated(&read_own(&delegated()), "no entry AGENT for AGENT");
    let admitted = read_own(&World::with_delegations(&[("self", AGENT, AGENT, UNTIL)]));
    assert_eq!(admitted.status, 200, "{}", admitted.body);
    assert_eq!(admitted.ids(), [ulid(1)]);
}

#[test]
fn an_uncovered_inbox_is_answered_as_a_bad_signature_byte_for_byte() {
    let world = World::with_delegations(&[("lapsed", AGENT, THIRD, NOW)]);
    let server = world.start(|_| {});
    let address = server.local_addr();
    let target = pending(OTHER);
    let headers = |member: Option<&str>, extra: &[(&'static str, &str)]| {
        let mut headers: Vec<(&str, String)> = world
            .signed_headers(AGENT, "GET", &target, b"", member)
            .into_iter()
            .collect();
        headers.extend(extra.iter().map(|&(name, value)| (name, value.to_owned())));
        headers
    };
    // A bad signature: AGENT's key id, THIRD's signature.
    let mut bad = headers(None, &[]);
    bad[2].1 = world.signed_headers(THIRD, "GET", &target, b"", None)[2]
        .1
        .clone();
    let (bad_head, bad_reply) = send_raw_answer(address, "GET", &target, &bad, b"");
    unauthenticated(&bad_reply, "a bad signature");
    for (why, sent) in [
        (
            "no delegation",
            headers(Some(OTHER), &[(ON_BEHALF_OF_HEADER, OTHER)]),
        ),
        (
            "a lapsed one",
            headers(Some(THIRD), &[(ON_BEHALF_OF_HEADER, THIRD)]),
        ),
        (
            "the header twice",
            headers(
                Some(OTHER),
                &[(ON_BEHALF_OF_HEADER, OTHER), (ON_BEHALF_OF_HEADER, OTHER)],
            ),
        ),
    ] {
        let (head, reply) = send_raw_answer(address, "GET", &target, &sent, b"");
        assert_eq!(
            (head.as_str(), reply.status, reply.body.as_str()),
            (bad_head.as_str(), bad_reply.status, bad_reply.body.as_str()),
            "{why}"
        );
    }
}

// ---- the wait, and what the agent gets beyond the story --------------------------------------

/// The world's clock, each reading counted, so a case knows when a wait has read its view.
#[derive(Debug)]
struct Counted {
    inner: Arc<ManualClock>,
    reads: AtomicUsize,
}

impl Clock for Counted {
    fn now(&self) -> Instant {
        self.reads.fetch_add(1, Ordering::SeqCst);
        self.inner.now()
    }
}

#[test]
fn a_wait_admitted_under_a_delegation_reads_on_after_it_lapses_until_it_ends() {
    let world = Arc::new(delegated());
    world.clock.set(instant(BEFORE_UNTIL));
    let counted = Arc::new(Counted {
        inner: Arc::clone(&world.clock),
        reads: AtomicUsize::new(0),
    });
    let server = world.start(|config| {
        config.clock = counted.clone();
        config.max_wait = Duration::from_secs(30);
    });
    let address = server.local_addr();
    world.open(address, OTHER);
    let (sent, answered) = mpsc::channel();
    let before = counted.reads.load(Ordering::SeqCst);
    let waiting = Arc::clone(&world);
    thread::spawn(move || {
        let target = format!("{}&wait=30", pending(OTHER));
        let reply = waiting.send_for(address, AGENT, OTHER, "GET", &target, b"");
        let _ = sent.send(reply);
    });
    // The wait has authenticated (one reading) and read its view (the reading after it).
    let started = Monotonic::now();
    while counted.reads.load(Ordering::SeqCst) < before + 2 {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the wait never read its view"
        );
        thread::sleep(Duration::from_millis(5));
    }
    world.clock.set(instant(UNTIL));
    post(&world, address, THIRD, OTHER, 1, Kind::Note);
    let reply = answered
        .recv_timeout(Duration::from_secs(10))
        .expect("the post ends the wait");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(reply.ids(), [ulid(1)]);
    // The next request that names the inbox is refused.
    unauthenticated(
        &world.send_for(address, AGENT, OTHER, "GET", &pending(OTHER), b""),
        "after valid_until",
    );
}

#[test]
fn a_wake_read_for_the_person_is_409_while_its_inbox_is_not_open() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, AGENT);
    let target = format!("{}&undelivered=1", pending(OTHER));
    let closed = world.send_for(address, AGENT, OTHER, "GET", &target, b"");
    assert_eq!(
        (closed.status, closed.body.as_str()),
        (409, r#"{"refused":"inbox not open"}"#)
    );
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Note);
    let open = world.send_for(address, AGENT, OTHER, "GET", &target, b"");
    assert_eq!(open.ids(), [ulid(1)], "{}", open.body);
}

#[test]
fn an_inbox_the_agent_opens_for_the_person_is_closed_only_by_the_agent() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let address_body = format!(r#"{{"address":"{OTHER}","adapter":"terminal"}}"#);
    let without = world.send(address, AGENT, "POST", OPEN, address_body.as_bytes());
    assert_eq!(without.text("outcome"), "not-owner", "{}", without.body);
    let opened = world.send_for(address, AGENT, OTHER, "POST", OPEN, address_body.as_bytes());
    assert_eq!(opened.text("outcome"), "opened", "{}", opened.body);
    let close_body = format!(r#"{{"address":"{OTHER}"}}"#);
    let by_person = world.send(address, OTHER, "POST", CLOSE, close_body.as_bytes());
    assert_eq!(by_person.text("outcome"), "not-owner", "{}", by_person.body);
    let by_agent = world.send_for(address, AGENT, OTHER, "POST", CLOSE, close_body.as_bytes());
    assert_eq!(by_agent.text("outcome"), "closed", "{}", by_agent.body);
}

/// The state of the inbox at `at`, or `None` when nobody opened it.
fn inbox_state(world: &World, at: &str) -> Option<InboxState> {
    InboxStorage::get(&world.backend, &Address(at.to_owned())).map(|inbox| inbox.state)
}

/// The body of `open-inbox` for `at`.
fn opening(at: &str) -> Vec<u8> {
    format!(r#"{{"address":"{at}","adapter":"terminal"}}"#).into_bytes()
}

/// The body of `close-inbox` for `at`.
fn closing(at: &str) -> Vec<u8> {
    format!(r#"{{"address":"{at}"}}"#).into_bytes()
}

/// The rule of the service (`docs/design/service.md`, the table of commands): `close-inbox` for
/// an address that is not the caller's inbox is the model's own `not-owner`, byte for byte, also
/// for the agent that owns the row while it signs without the header and its delegation is live.
#[test]
fn without_the_header_the_agent_does_not_close_the_inbox_it_opened_for_the_person() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let opened = world.send_for(address, AGENT, OTHER, "POST", OPEN, &opening(OTHER));
    assert_eq!(opened.text("outcome"), "opened", "{}", opened.body);
    let by_person = world.send(address, OTHER, "POST", CLOSE, &closing(OTHER));
    assert_eq!(
        (by_person.status, by_person.body.as_str()),
        (403, NOT_OWNER)
    );
    let by_agent = world.send(address, AGENT, "POST", CLOSE, &closing(OTHER));
    assert_eq!(
        (by_agent.status, by_agent.body.as_str()),
        (by_person.status, by_person.body.as_str())
    );
    assert_eq!(inbox_state(&world, OTHER), Some(InboxState::Open));
}

/// Under the header the caller's inbox is the person's, so the agent's own inbox is not its own
/// for `close-inbox` either; without the header the agent closes it as before.
#[test]
fn under_the_header_the_agent_does_not_close_its_own_inbox() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, AGENT);
    let under = world.send_for(address, AGENT, OTHER, "POST", CLOSE, &closing(AGENT));
    assert_eq!((under.status, under.body.as_str()), (403, NOT_OWNER));
    assert_eq!(inbox_state(&world, AGENT), Some(InboxState::Open));
    let without = world.send(address, AGENT, "POST", CLOSE, &closing(AGENT));
    assert_eq!(without.text("outcome"), "closed", "{}", without.body);
    assert_eq!(inbox_state(&world, AGENT), Some(InboxState::Closed));
}

/// The rule sits at the model's step for `not-owner`, after `unknown-inbox`: an address nobody
/// opened is `unknown-inbox` with the header and without it, as the model answers it.
#[test]
fn a_close_of_an_inbox_nobody_opened_is_unknown_inbox_under_the_rule() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let without = world.send(address, AGENT, "POST", CLOSE, &closing(THIRD));
    assert_eq!(
        (without.status, without.body.as_str()),
        (404, UNKNOWN_INBOX)
    );
    let under = world.send_for(address, AGENT, OTHER, "POST", CLOSE, &closing(THIRD));
    assert_eq!((under.status, under.body.as_str()), (404, UNKNOWN_INBOX));
    assert_eq!(inbox_state(&world, THIRD), None);
}

/// After `valid_until` the inbox the agent opened for the person stays open: posts to it are
/// accepted, the person reads it with its own key, and nobody closes it (the agent's header is
/// 401, the agent without it and the person are `not-owner`) until the service starts anew.
#[test]
fn after_the_lapse_the_persons_inbox_stays_open_and_nobody_closes_it() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let opened = world.send_for(address, AGENT, OTHER, "POST", OPEN, &opening(OTHER));
    assert_eq!(opened.text("outcome"), "opened", "{}", opened.body);
    world.clock.set(instant(UNTIL));
    post(&world, address, THIRD, OTHER, 1, Kind::Note);
    let read = world.send(address, OTHER, "GET", &pending(OTHER), b"");
    assert_eq!(read.ids(), [ulid(1)], "{}", read.body);
    let under = world.send_for(address, AGENT, OTHER, "POST", CLOSE, &closing(OTHER));
    assert_eq!((under.status, under.body.as_str()), (401, UNAUTHENTICATED));
    for caller in [AGENT, OTHER] {
        let closed = world.send(address, caller, "POST", CLOSE, &closing(OTHER));
        assert_eq!(
            (closed.status, closed.body.as_str()),
            (403, NOT_OWNER),
            "{caller}"
        );
    }
    assert_eq!(inbox_state(&world, OTHER), Some(InboxState::Open));
}

#[test]
fn a_post_under_the_header_is_answered_as_without_it() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, THIRD);
    let envelope = world.post(AGENT, THIRD, 1, "from the agent");
    let posted = world.send_for(address, AGENT, OTHER, "POST", POST, &encoded(&envelope));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
    let held = MessageStorage::get(&world.backend, &MessageId(ulid(1))).expect("stored");
    assert_eq!(held.data.from.0, AGENT);
    // No post is made as the person: the person's envelope is not the agent's to send.
    let persons = world.post(OTHER, THIRD, 2, "from the person");
    let refused = world.send_for(address, AGENT, OTHER, "POST", POST, &encoded(&persons));
    assert_eq!(refused.text("outcome"), "not-sender", "{}", refused.body);
    assert_eq!(state(&world, 2), None);
}

#[test]
fn refuse_and_unanswered_for_the_person_answer_as_the_model() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    post(&world, address, THIRD, OTHER, 1, Kind::Request);
    own(&world, address, OTHER, MARK, 1, "delivered");
    own(&world, address, OTHER, READ, 1, "read");
    let unanswered = world.send_for(
        address,
        AGENT,
        OTHER,
        "GET",
        &format!("/messaging/views/unanswered?inbox={OTHER}"),
        b"",
    );
    assert_eq!(unanswered.ids(), [ulid(1)], "{}", unanswered.body);
    let body = format!(r#"{{"message_id":"{}","reason":"not now"}}"#, ulid(1));
    let refused = world.send_for(address, AGENT, OTHER, "POST", REFUSE, body.as_bytes());
    assert_eq!(refused.text("outcome"), "refused", "{}", refused.body);
    assert_eq!(state(&world, 1), Some(MessageState::Refused));
}

#[test]
fn under_the_header_the_agents_own_inbox_is_not_its_own() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, AGENT);
    post(&world, address, THIRD, AGENT, 1, Kind::Note);
    let read = world.send_for(address, AGENT, OTHER, "GET", &pending(AGENT), b"");
    assert_eq!(read.status, 200, "{}", read.body);
    assert!(read.ids().is_empty(), "{}", read.body);
    let marked = world.send_for(address, AGENT, OTHER, "POST", MARK, &id_body(1));
    assert_eq!((marked.status, marked.body.as_str()), (404, UNKNOWN));
    assert_eq!(state(&world, 1), Some(MessageState::Sent));
    let mine = world.send(address, AGENT, "GET", &pending(AGENT), b"");
    assert_eq!(mine.ids(), [ulid(1)], "{}", mine.body);
}
