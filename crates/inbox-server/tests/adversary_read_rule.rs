//! Adversary pass 1 on story:the-core-applies-the-read-rule: the listener now parses a view
//! answer and writes it back through `inbox_core::readable`. For a party whose rows the rule
//! keeps whole, the bytes over HTTP are the bytes the generated `dispatch` wrote, escapes and
//! number spellings included.

use crate::support;

use std::sync::Arc;

use inbox::actor::{Actor, Caller};
use inbox::messaging::{Address, Kind, Principal, ReplyShape};
use inbox::server::{http, inbox_service};
use inbox_core::{CallerAttributes, ServiceContext};
use inbox_server::RingVerifier;
use support::{AGENT, NOW, OTHER, World, encoded, instant};

const POST: &str = "/messaging/commands/post";

/// The body the generated `dispatch` answers `principal` for `GET path?query`, with no rule.
fn generated(world: &World, principal: &str, path: &str, query: &str) -> String {
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
    let mut system = inbox_core::system(world.backend.clone(), context);
    let request = http::Request {
        method: "GET".to_owned(),
        path: path.to_owned(),
        query: query.to_owned(),
        headers: Vec::new(),
        body: Vec::new(),
    };
    let answer = inbox_service::dispatch(
        &mut system,
        Some(&Caller {
            actor: Actor::Client,
        }),
        &request,
    );
    assert_eq!(answer.status, 200, "{}", answer.body);
    answer.body
}

#[test]
fn a_partys_view_answer_is_the_generated_answer_byte_for_byte() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let bodies = [
        "a \"quoted\" word, a \\ backslash, a\ttab and a\nnew line",
        "caf\u{e9} \u{2028} \u{1f600} </script>",
    ];
    for (n, body) in (1..).zip(bodies) {
        let mut envelope = world.post(AGENT, OTHER, n, body);
        envelope.kind = Kind::Request;
        envelope.reply_shape = Some(ReplyShape::Lines(12));
        world.sign(&mut envelope);
        let posted = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
        assert_eq!(posted.status, 202, "{}", posted.body);
    }
    for (principal, path, query) in [
        (OTHER, "/messaging/views/pending", format!("inbox={OTHER}")),
        (AGENT, "/messaging/views/outbox", format!("sender={AGENT}")),
        (
            AGENT,
            "/messaging/views/thread",
            format!("thread={}", support::ulid(1)),
        ),
    ] {
        let wire = world.send(address, principal, "GET", &format!("{path}?{query}"), b"");
        assert_eq!(wire.status, 200, "{}", wire.body);
        let expected = generated(&world, principal, path, &query);
        assert!(
            expected.contains("\\\""),
            "the answer carries an escape: {expected}"
        );
        assert_eq!(
            wire.body, expected,
            "{principal} {path}: the listener wrote the answer back differently"
        );
    }
}
