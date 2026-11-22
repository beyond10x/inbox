//! story:a-due-message-takes-no-answer over HTTP: once the service's clock reaches a message's
//! `expires`, an answer and a refusal are refused and change nothing, and the expiry task decides
//! the message as before; the late answer before the sweep and the sweep before the late answer
//! leave the same message. A post due at the service's clock is refused. The clock is the test's
//! [`inbox_server::ManualClock`]; every key is created when the test runs.

use crate::support;

use std::net::SocketAddr;
use std::time::Duration;

use inbox::behaviour::MessageStorage;
use inbox::messaging::{
    Answer, Body, DecisionAsk, DecisionOption, Kind, MessageId, OptionLetter, Principal,
    ReplyShape, Signature,
};
use inbox::primitives::Timestamp;
use inbox::server::json::Value;
use inbox::server::wire;
use inbox_verify::sign_envelope;
use support::{AGENT, LATER, NOW, OTHER, Reply, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
const ANSWER: &str = "/messaging/commands/answer";
const REFUSE: &str = "/messaging/commands/refuse";
/// A second after the decision's `expires` (`LATER`).
const AFTER: &str = "2026-12-01T00:00:01Z";
const DUE: &str =
    r#"{"outcome":"deadline-passed","published":[],"error":"inbox.messaging.Expired"}"#;

/// A decision from AGENT to OTHER between A and B whose silence takes B, due at `LATER`.
fn decide(world: &World, address: SocketAddr) {
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
    world.sign(&mut decision);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&decision));
    assert_eq!(reply.status, 202, "{}", reply.body);
}

/// OTHER's answer A, written after the deadline.
fn answer(world: &World, address: SocketAddr) -> Reply {
    let mut answer = Answer {
        in_reply_to: MessageId(ulid(1)),
        reply_id: MessageId(ulid(2)),
        from: Principal(OTHER.to_owned()),
        on_behalf_of: Principal(OTHER.to_owned()),
        authority: None,
        created: Timestamp(AFTER.to_owned()),
        expires: Timestamp("2026-12-02T00:00:00Z".to_owned()),
        option: Some(OptionLetter("A".to_owned())),
        body: Body("A.".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    let member = world.member(OTHER);
    sign_envelope(&mut answer, &member.key_id, &member.key).expect("canonicalizes");
    let mut out = String::new();
    wire::encode_command_inbox_messaging_answer(&answer, &mut out);
    world.send(address, OTHER, "POST", ANSWER, out.as_bytes())
}

/// The fields of each stored message the decision leaves, without the signatures, which differ
/// between two worlds' keys.
fn decided(world: &World) -> Vec<String> {
    MessageStorage::list(&world.backend)
        .iter()
        .map(|message| {
            let data = &message.data;
            format!(
                "{:?} {:?} {:?} {:?} {:?} {}",
                message.state,
                data.answered_by,
                data.decided_option,
                data.expires,
                data.message_id,
                data.reply.is_none()
            )
        })
        .collect()
}

#[test]
fn a_late_answer_and_the_sweep_leave_the_same_decision_in_either_order() {
    let late_first = World::new();
    let server = late_first.start(|_| {});
    let address = server.local_addr();
    late_first.open(address, OTHER);
    decide(&late_first, address);
    late_first.clock.set(instant(AFTER));
    let before = format!("{:?}", MessageStorage::list(&late_first.backend));
    let late = answer(&late_first, address);
    assert_eq!((late.status, late.body.as_str()), (502, DUE));
    assert_eq!(
        format!("{:?}", MessageStorage::list(&late_first.backend)),
        before,
        "a refused answer changes no row"
    );
    assert_eq!(server.sweep_now(), 1);

    let sweep_first = World::new();
    let server = sweep_first.start(|_| {});
    let address = server.local_addr();
    sweep_first.open(address, OTHER);
    decide(&sweep_first, address);
    sweep_first.clock.set(instant(AFTER));
    assert_eq!(server.sweep_now(), 1);
    let settled = answer(&sweep_first, address);
    assert_eq!(settled.status, 409, "{}", settled.body);
    assert_eq!(settled.text("outcome"), "already-settled");

    let held = decided(&late_first);
    assert_eq!(held, decided(&sweep_first));
    assert_eq!(held.len(), 1, "{held:?}");
    assert!(held[0].contains("Some(Silence)"), "{held:?}");
    assert!(held[0].contains("Some(OptionLetter(\"B\"))"), "{held:?}");
}

#[test]
fn a_refusal_at_the_deadline_is_refused_and_one_before_it_is_taken() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address);
    let refuse = format!(r#"{{"message_id":"{}","reason":"not mine"}}"#, ulid(1));
    world.clock.set(instant(LATER));
    let due = world.send(address, OTHER, "POST", REFUSE, refuse.as_bytes());
    assert_eq!((due.status, due.body.as_str()), (502, DUE));
    world.clock.advance(-1);
    let taken = world.send(address, OTHER, "POST", REFUSE, refuse.as_bytes());
    assert_eq!(taken.status, 202, "{}", taken.body);
    assert_eq!(taken.text("outcome"), "refused");
}

#[test]
fn a_post_due_at_the_service_clock_is_refused_and_stores_nothing() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let mut post = world.post(AGENT, OTHER, 1, "too late");
    post.created = Timestamp("2026-11-21T11:00:00Z".to_owned());
    post.expires = Timestamp(NOW.to_owned());
    world.sign(&mut post);
    let refused = world.send(address, AGENT, "POST", POST, &encoded(&post));
    assert_eq!(
        (refused.status, refused.body.as_str()),
        (
            502,
            r#"{"outcome":"expired-on-arrival","published":[],"error":"inbox.messaging.AlreadyExpired"}"#
        )
    );
    assert!(MessageStorage::list(&world.backend).is_empty());
}

/// The sweep writes `keep_until`, the reading plus the answer retention, with the reading's
/// fraction of a second: a decision decided on silence at `…:00.25Z` waits for its sender until
/// `…:00.25Z` an hour later, not until the whole second before it.
#[test]
fn the_sweep_keeps_a_decision_until_the_reading_plus_retention_with_its_fraction() {
    let world = World::new();
    let server = world.start(|config| config.answer_retention = Duration::from_secs(3600));
    let address = server.local_addr();
    world.open(address, OTHER);
    decide(&world, address);
    world.clock.set(instant("2026-12-01T00:00:00.25Z"));
    assert_eq!(server.sweep_now(), 1);
    let settled = world.send(
        address,
        AGENT,
        "GET",
        &format!("/messaging/views/settled?sender={AGENT}"),
        b"",
    );
    let rows = settled.rows();
    assert_eq!(rows.len(), 1, "{}", settled.body);
    assert_eq!(
        rows[0].member("expires"),
        Some(&Value::Text("2026-12-01T01:00:00.25Z".to_owned())),
        "{}",
        settled.body
    );
    let held = MessageStorage::list(&world.backend);
    assert_eq!(held[0].data.expires.0, "2026-12-01T01:00:00.25Z");
}
