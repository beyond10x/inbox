//! Adversary pass 1 on story:a-due-message-takes-no-answer over HTTP: where the reading comes from
//! (the section, the request's own timestamp), a third principal at a due message, and an
//! `expires` with a fraction against a clock that reads past it.

use crate::support;

use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;

use inbox::behaviour::MessageStorage;
use inbox::messaging::{
    Answer, Body, DecisionAsk, DecisionOption, Kind, MessageId, OptionLetter, Principal,
    ReplyShape, Signature,
};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_core::Exclusive;
use inbox_verify::sign_envelope;
use support::{AGENT, LATER, OTHER, Reply, THIRD, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
const ANSWER: &str = "/messaging/commands/answer";
const REFUSE: &str = "/messaging/commands/refuse";
const BEFORE: &str = "2026-11-30T23:59:59Z";
const AFTER: &str = "2026-12-01T00:00:01Z";
const DUE: &str =
    r#"{"outcome":"deadline-passed","published":[],"error":"inbox.messaging.Expired"}"#;

/// A decision from AGENT to OTHER between A and B whose silence takes B, due at `expires`.
fn decide(world: &World, address: SocketAddr, expires: &str) {
    let mut decision = world.post(AGENT, OTHER, 1, "A or B?");
    decision.kind = Kind::Decision;
    decision.reply_shape = Some(ReplyShape::Free(inbox::messaging::FreeShape {}));
    decision.decision = Some(DecisionAsk {
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
    decision.default_on_silence = Some(OptionLetter("B".to_owned()));
    decision.expires = Timestamp(expires.to_owned());
    world.sign(&mut decision);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&decision));
    assert_eq!(reply.status, 202, "{}", reply.body);
}

/// `who`'s answer A to message `n`, written before the deadline.
fn answer_body(world: &World, who: &str, n: u32) -> Vec<u8> {
    let mut answer = Answer {
        in_reply_to: MessageId(ulid(n)),
        reply_id: MessageId(ulid(2)),
        from: Principal(who.to_owned()),
        on_behalf_of: Principal(who.to_owned()),
        authority: None,
        created: Timestamp("2026-11-21T12:00:00Z".to_owned()),
        expires: Timestamp("2026-12-02T00:00:00Z".to_owned()),
        option: Some(OptionLetter("A".to_owned())),
        body: Body("A.".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    let member = world.member(who);
    sign_envelope(&mut answer, &member.key_id, &member.key).expect("canonicalizes");
    let mut out = String::new();
    wire::encode_command_inbox_messaging_answer(&answer, &mut out);
    out.into_bytes()
}

fn answer(world: &World, address: SocketAddr) -> Reply {
    world.send(
        address,
        OTHER,
        "POST",
        ANSWER,
        &answer_body(world, OTHER, 1),
    )
}

/// Attack 3 and mutant 5. The answer arrives one second before the deadline and is authenticated
/// at that reading, then waits for the backend's section while another command holds it; the
/// clock passes the deadline before the section is released. The command reads the clock inside
/// the section, so the answer is refused `deadline-passed`. A reading taken before the wait (the
/// authentication's `at`, or one taken when the context is built) answers it.
#[test]
fn the_reading_is_taken_after_the_wait_for_the_section() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address, LATER);
    world.clock.set(instant(BEFORE));
    let backend = world.backend.clone();
    let clock = world.clock.clone();
    let (held, holding) = mpsc::channel();
    let holder = std::thread::spawn(move || {
        backend.exclusive(|| {
            held.send(()).expect("the case waits");
            std::thread::sleep(Duration::from_millis(1500));
            clock.set(instant(AFTER));
        });
    });
    holding.recv().expect("the section is held");
    let late = answer(&world, address);
    holder.join().expect("the holder ends");
    assert_eq!((late.status, late.body.as_str()), (502, DUE));
    let held = MessageStorage::list(&world.backend);
    assert!(held[0].data.reply.is_none(), "{held:?}");
}

/// Attack 3. The request's own `Inbox-Timestamp` is not the reading: one signed 59 seconds before a
/// clock that is at the deadline is still refused, and one signed 29 seconds ahead of a clock 30
/// seconds before the deadline (after the deadline by its own timestamp) is taken.
#[test]
fn the_request_timestamp_does_not_move_the_reading() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address, LATER);
    world.clock.set(instant(LATER));
    let body = answer_body(&world, OTHER, 1);
    let early = world.send_at(
        address,
        OTHER,
        "POST",
        ANSWER,
        &body,
        "2026-11-30T23:59:01Z",
    );
    assert_eq!((early.status, early.body.as_str()), (502, DUE));

    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address, LATER);
    world.clock.set(instant("2026-11-30T23:59:30Z"));
    let body = answer_body(&world, OTHER, 1);
    let ahead = world.send_at(
        address,
        OTHER,
        "POST",
        ANSWER,
        &body,
        "2026-11-30T23:59:59Z",
    );
    assert_eq!(ahead.status, 202, "{}", ahead.body);
    assert_eq!(ahead.text("outcome"), "answered");
}

/// Attack 4 over HTTP. At a due message a third principal's answer (under its own `from`) and
/// refusal get the very answer an id no message carries gets: status and body alike.
#[test]
fn a_third_principal_gets_the_same_answer_for_a_due_message_as_for_none() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address, LATER);
    world.clock.set(instant(AFTER));
    let due_answer = world.send(
        address,
        THIRD,
        "POST",
        ANSWER,
        &answer_body(&world, THIRD, 1),
    );
    let none_answer = world.send(
        address,
        THIRD,
        "POST",
        ANSWER,
        &answer_body(&world, THIRD, 9),
    );
    assert_eq!(
        (due_answer.status, due_answer.body.as_str()),
        (none_answer.status, none_answer.body.as_str())
    );
    assert_eq!(due_answer.status, 404, "{}", due_answer.body);
    let refuse = |n: u32| format!(r#"{{"message_id":"{}","reason":"no"}}"#, ulid(n));
    let due_refusal = world.send(address, THIRD, "POST", REFUSE, refuse(1).as_bytes());
    let none_refusal = world.send(address, THIRD, "POST", REFUSE, refuse(9).as_bytes());
    assert_eq!(
        (due_refusal.status, due_refusal.body.as_str()),
        (none_refusal.status, none_refusal.body.as_str())
    );
    assert_eq!(due_refusal.status, 404, "{}", due_refusal.body);
}

/// Attack 2. The service's clock holds the fraction of a second (`ManualClock`, `SystemClock`),
/// and `docs/design/service.md` says a message "is due at its `expires` exactly". A decision due
/// at `…:00.500Z`, answered when the clock reads `…:00.700Z`, is past its deadline by the service's
/// own clock, and is still answered: the reading is cut to the second before it is compared.
#[test]
fn an_answer_after_a_fractional_deadline_by_the_service_clock_is_refused() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address, "2026-12-01T00:00:00.500Z");
    world.clock.set(instant("2026-12-01T00:00:00.700Z"));
    let late = answer(&world, address);
    assert_eq!(
        (late.status, late.body.as_str()),
        (502, DUE),
        "the clock reads 0.2 s past the decision's expires"
    );
}
