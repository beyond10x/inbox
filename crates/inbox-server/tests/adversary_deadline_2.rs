//! Adversary pass 2 on story:a-due-message-takes-no-answer over HTTP: the 4 places that compare an
//! `expires` with the service's clock (`PostMessage`, `Answer`'s reply, `Refuse`, the sweep) meet
//! it at the clock's full precision, one nanosecond apart, and a reply already expired leaves a
//! decision to its silence.

use crate::support;

use std::net::SocketAddr;

use inbox::behaviour::MessageStorage;
use inbox::messaging::{
    Answer, Body, DecisionAsk, DecisionOption, FreeShape, Kind, MessageId, OptionLetter, Principal,
    ReplyShape, Signature,
};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_verify::sign_envelope;
use support::{AGENT, OTHER, Reply, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
const ANSWER: &str = "/messaging/commands/answer";
const REFUSE: &str = "/messaging/commands/refuse";
/// A deadline with a fraction of a second.
const DEADLINE: &str = "2026-12-01T00:00:00.5Z";
/// One nanosecond before it.
const JUST_BEFORE: &str = "2026-12-01T00:00:00.499999999Z";

/// A request from AGENT to OTHER, or a decision between A and B whose silence takes B, due at
/// `expires`, posted at the clock's reading.
fn ask(world: &World, address: SocketAddr, decision: bool, expires: &str) -> Reply {
    let mut post = world.post(AGENT, OTHER, 1, "A or B?");
    post.kind = Kind::Request;
    post.reply_shape = Some(ReplyShape::Free(FreeShape {}));
    if decision {
        post.kind = Kind::Decision;
        post.decision = Some(DecisionAsk {
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
        post.default_on_silence = Some(OptionLetter("B".to_owned()));
    }
    post.expires = Timestamp(expires.to_owned());
    world.sign(&mut post);
    world.send(address, AGENT, "POST", POST, &encoded(&post))
}

/// OTHER's answer to message 1, whose own `expires` is `expires`.
fn answer(world: &World, address: SocketAddr, option: Option<&str>, expires: &str) -> Reply {
    let mut answer = Answer {
        in_reply_to: MessageId(ulid(1)),
        reply_id: MessageId(ulid(2)),
        from: Principal(OTHER.to_owned()),
        on_behalf_of: Principal(OTHER.to_owned()),
        authority: None,
        created: Timestamp("2026-11-21T12:00:00Z".to_owned()),
        expires: Timestamp(expires.to_owned()),
        option: option.map(|letter| OptionLetter(letter.to_owned())),
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

fn refuse(world: &World, address: SocketAddr) -> Reply {
    let body = format!(r#"{{"message_id":"{}","reason":"no"}}"#, ulid(1));
    world.send(address, OTHER, "POST", REFUSE, body.as_bytes())
}

/// A started service with OTHER's inbox open and the clock at `NOW`.
fn service(world: &World) -> (inbox_server::Running<inbox_core::MemoryBackend>, SocketAddr) {
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    (server, address)
}

fn outcome(reply: &Reply) -> (u16, String) {
    (reply.status, reply.text("outcome"))
}

#[test]
fn a_post_meets_the_clock_at_full_precision() {
    let world = World::new();
    let (_server, address) = service(&world);
    world.clock.set(instant(JUST_BEFORE));
    let taken = ask(&world, address, false, DEADLINE);
    assert_eq!(
        outcome(&taken),
        (202, "posted".to_owned()),
        "{}",
        taken.body
    );

    let world = World::new();
    let (_server, address) = service(&world);
    world.clock.set(instant(DEADLINE));
    let refused = ask(&world, address, false, DEADLINE);
    assert_eq!(
        outcome(&refused),
        (502, "expired-on-arrival".to_owned()),
        "{}",
        refused.body
    );
}

#[test]
fn a_reply_meets_the_clock_at_full_precision() {
    let world = World::new();
    let (_server, address) = service(&world);
    assert_eq!(
        ask(&world, address, false, "2026-12-02T00:00:00Z").status,
        202
    );
    world.clock.set(instant(DEADLINE));
    let before = format!("{:?}", MessageStorage::list(&world.backend));
    let refused = answer(&world, address, None, DEADLINE);
    assert_eq!(
        (refused.status, refused.body.as_str()),
        (
            502,
            r#"{"outcome":"expired-on-arrival","published":[],"error":"inbox.messaging.AlreadyExpired"}"#
        )
    );
    assert_eq!(
        format!("{:?}", MessageStorage::list(&world.backend)),
        before
    );
    world.clock.set(instant(JUST_BEFORE));
    let taken = answer(&world, address, None, DEADLINE);
    assert_eq!(
        outcome(&taken),
        (202, "answered".to_owned()),
        "{}",
        taken.body
    );
}

#[test]
fn a_refusal_meets_the_clock_at_full_precision() {
    let world = World::new();
    let (_server, address) = service(&world);
    assert_eq!(ask(&world, address, false, DEADLINE).status, 202);
    world.clock.set(instant(DEADLINE));
    let due = refuse(&world, address);
    assert_eq!(
        outcome(&due),
        (502, "deadline-passed".to_owned()),
        "{}",
        due.body
    );
    world.clock.set(instant(JUST_BEFORE));
    let taken = refuse(&world, address);
    assert_eq!(
        outcome(&taken),
        (202, "refused".to_owned()),
        "{}",
        taken.body
    );
}

/// The expiry task finds a message due at its `expires` exactly and not one nanosecond before:
/// between the two readings an answer is refused `deadline-passed`, so the sweep must take it.
#[test]
fn the_sweep_meets_the_clock_at_full_precision() {
    let world = World::new();
    let (server, address) = service(&world);
    assert_eq!(ask(&world, address, false, DEADLINE).status, 202);
    world.clock.set(instant(JUST_BEFORE));
    assert_eq!(server.sweep_now(), 0);
    world.clock.set(instant("2026-12-01T00:00:00.7Z"));
    let late = answer(&world, address, None, "2026-12-02T00:00:00Z");
    assert_eq!(
        outcome(&late),
        (502, "deadline-passed".to_owned()),
        "{}",
        late.body
    );
    assert_eq!(
        server.sweep_now(),
        1,
        "the message the answer was refused for"
    );
    assert!(MessageStorage::list(&world.backend).is_empty());
}

/// A decision whose silence takes B: OTHER's reply A, already expired by the service's clock, is
/// refused; the sweep at the deadline decides on silence and stores B, answered by silence.
#[test]
fn an_expired_reply_to_a_decision_leaves_it_to_silence() {
    let world = World::new();
    let (server, address) = service(&world);
    assert_eq!(ask(&world, address, true, DEADLINE).status, 202);
    world.clock.set(instant("2026-11-30T00:00:00Z"));
    let refused = answer(&world, address, Some("A"), "2026-11-29T00:00:00Z");
    assert_eq!(
        outcome(&refused),
        (502, "expired-on-arrival".to_owned()),
        "{}",
        refused.body
    );
    assert_eq!(server.sweep_now(), 0);
    world.clock.set(instant(DEADLINE));
    assert_eq!(server.sweep_now(), 1);
    let held = MessageStorage::list(&world.backend);
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(format!("{:?}", held[0].data.answered_by), "Some(Silence)");
    assert_eq!(
        held[0]
            .data
            .decided_option
            .as_ref()
            .map(|letter| letter.0.as_str()),
        Some("B")
    );
    assert!(held[0].data.reply.is_none(), "{held:?}");
}
