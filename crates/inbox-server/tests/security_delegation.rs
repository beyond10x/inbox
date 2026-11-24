//! Security review, pass 1, of story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal
//! over HTTP. A delegation lets its agent act for the person's inbox until `valid_until`
//! (`docs/design/signing.md`, "A delegation"): after it lapses the agent holds no power over that
//! inbox. And what the agent may do with a question it sent itself to the person's inbox. AGENT is
//! the agent, OTHER the person. Every key is created when the test runs.

use crate::support;

use inbox::behaviour::InboxStorage;
use inbox::messaging::{
    Address, Answer, Body, DecisionAsk, DecisionOption, DelegationId, FreeShape, InboxState, Kind,
    MessageId, OptionLetter, Principal, ReplyShape, Signature,
};
use inbox::primitives::Timestamp;
use inbox::server::wire;
use inbox_verify::sign_envelope;
use support::{AGENT, LATER, NOW, OTHER, World, encoded, instant, ulid};

const OPEN: &str = "/messaging/commands/open-inbox";
const CLOSE: &str = "/messaging/commands/close-inbox";
const POST: &str = "/messaging/commands/post";
const MARK: &str = "/messaging/commands/mark-delivered";
const READ: &str = "/messaging/commands/read";
const ANSWER: &str = "/messaging/commands/answer";
const UNAUTHENTICATED: &str = r#"{"refused":"unauthenticated"}"#;

/// The delegation AGENT for OTHER, live until `UNTIL`.
const D: &str = "agent-for-other";
const UNTIL: &str = "2026-11-25T00:00:00Z";

fn delegated() -> World {
    World::with_delegations(&[(D, AGENT, OTHER, UNTIL)])
}

fn state_of(world: &World, address: &str) -> Option<InboxState> {
    InboxStorage::get(&world.backend, &Address(address.to_owned())).map(|inbox| inbox.state)
}

fn id_body(n: u32) -> Vec<u8> {
    format!(r#"{{"message_id":"{}"}}"#, ulid(n)).into_bytes()
}

#[test]
fn a_lapsed_delegation_leaves_its_agent_no_close_of_the_persons_inbox() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let open_body = format!(r#"{{"address":"{OTHER}","adapter":"terminal"}}"#);
    let opened = world.send_for(address, AGENT, OTHER, "POST", OPEN, open_body.as_bytes());
    assert_eq!(opened.text("outcome"), "opened", "{}", opened.body);

    world.clock.set(instant(UNTIL));
    // The delegation is lapsed: a request that names the person's inbox is refused.
    let named = world.send_for(
        address,
        AGENT,
        OTHER,
        "GET",
        "/messaging/views/inboxes",
        b"",
    );
    assert_eq!((named.status, named.body.as_str()), (401, UNAUTHENTICATED));

    // Without the header the agent is a principal with no delegation of the person: it must not
    // close the person's inbox, which would refuse every post to the person.
    let close_body = format!(r#"{{"address":"{OTHER}"}}"#);
    let closed = world.send(address, AGENT, "POST", CLOSE, close_body.as_bytes());
    assert_eq!(
        (closed.text("outcome"), state_of(&world, OTHER)),
        ("not-owner".to_owned(), Some(InboxState::Open)),
        "after valid_until the agent closed the person's inbox: {}",
        closed.body
    );
}

/// A decision of `from` to `to`, signed with `from`'s key, on behalf of `from`.
fn decision(world: &World, from: &str, to: &str, n: u32) -> Vec<u8> {
    let mut envelope = world.post(from, to, n, "pick one");
    envelope.kind = Kind::Decision;
    envelope.reply_shape = Some(ReplyShape::Free(FreeShape {}));
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
    world.sign(&mut envelope);
    encoded(&envelope)
}

/// AGENT's reply to message `n` for OTHER under `D`, option A.
fn agents_reply(world: &World, n: u32, reply_n: u32) -> Vec<u8> {
    let mut answer = Answer {
        in_reply_to: MessageId(ulid(n)),
        reply_id: MessageId(ulid(reply_n)),
        from: Principal(AGENT.to_owned()),
        on_behalf_of: Principal(OTHER.to_owned()),
        authority: Some(DelegationId(D.to_owned())),
        created: Timestamp(NOW.to_owned()),
        expires: Timestamp(LATER.to_owned()),
        option: Some(OptionLetter("A".to_owned())),
        body: Body("A it is".to_owned()),
        signature: Signature {
            key_id: String::new(),
            value: String::new(),
        },
    };
    let member = world.member(AGENT);
    sign_envelope(&mut answer, &member.key_id, &member.key).expect("canonicalizes");
    let mut out = String::new();
    wire::encode_command_inbox_messaging_answer(&answer, &mut out);
    out.into_bytes()
}

/// What the design leaves unsaid (`docs/design/service.md`, "The caller", the row of `answer`):
/// the agent posts a decision to the person's inbox and answers it itself in the person's place,
/// and the store records the person's answer to the agent's own question. This case states what
/// the service does now; it is green.
#[test]
fn the_agent_answers_its_own_decision_in_the_persons_place() {
    let world = delegated();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let posted = world.send(
        address,
        AGENT,
        "POST",
        POST,
        &decision(&world, AGENT, OTHER, 1),
    );
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
    for (route, outcome) in [(MARK, "delivered"), (READ, "read")] {
        let moved = world.send_for(address, AGENT, OTHER, "POST", route, &id_body(1));
        assert_eq!(moved.text("outcome"), outcome, "{route}: {}", moved.body);
    }
    let answered = world.send_for(
        address,
        AGENT,
        OTHER,
        "POST",
        ANSWER,
        &agents_reply(&world, 1, 101),
    );
    assert_eq!(answered.text("outcome"), "answered", "{}", answered.body);
}
