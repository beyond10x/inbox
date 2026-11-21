//! Adversary cases of stage 2, wave 7, story:post-does-not-tell-a-message-id-exists, over HTTP.
//! Each case names the document sentence it drives the service with. Every key is created when
//! the test runs.

mod support;

use inbox::messaging::{DedupKey, MessageId};
use support::{AGENT, OTHER, THIRD, World, encoded, ulid};

const POST: &str = "/messaging/commands/post";

/// `docs/design/service.md`, the paragraph on a third principal: "A `post` to an address no inbox
/// has, or into a closed inbox, answers alike for the id of another sender's message and a free
/// id: `409` with the body of `unknown-inbox` or of `inbox-closed`, or of `duplicate-key` when a
/// live message of the caller carries the post's dedup key." Here a third principal posts into
/// its own closed inbox under a dedup key its own live message carries: the answer is
/// `duplicate-key`, alike for the held and the free id, so nothing is told. Until the sentence
/// named the third body this case failed on it.
#[test]
fn adv_w7_a_post_into_a_closed_inbox_answers_with_a_body_service_md_names() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    world.open(address, THIRD);
    let between = world.post(AGENT, OTHER, 1, "between two others");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&between));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
    let own = world.post(THIRD, THIRD, 7, "the third principal's own message");
    let posted = world.send(address, THIRD, "POST", POST, &encoded(&own));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);
    let close = format!(r#"{{"address":"{THIRD}"}}"#);
    let closed = world.send(
        address,
        THIRD,
        "POST",
        "/messaging/commands/close-inbox",
        close.as_bytes(),
    );
    assert_eq!(closed.text("outcome"), "closed", "{}", closed.body);

    let probe = |id: u32| {
        let mut post = world.post(THIRD, THIRD, id, "probe");
        post.message_id = MessageId(ulid(id));
        post.dedup_key = DedupKey(ulid(7));
        world.sign(&mut post);
        world.send(address, THIRD, "POST", POST, &encoded(&post))
    };
    let held = probe(1);
    let free = probe(99);
    assert_eq!(
        (held.status, held.body.as_str()),
        (free.status, free.body.as_str())
    );
    assert_eq!(held.text("outcome"), "duplicate-key", "{}", held.body);
    let named = [
        r#"{"outcome":"unknown-inbox","published":[],"error":"inbox.messaging.UnknownInbox"}"#,
        r#"{"outcome":"inbox-closed","published":[],"error":"inbox.messaging.UnknownInbox"}"#,
        r#"{"outcome":"duplicate-key","published":[],"error":"inbox.messaging.Duplicate"}"#,
    ];
    assert!(
        held.status == 409 && named.contains(&held.body.as_str()),
        "service.md names unknown-inbox, inbox-closed or duplicate-key; the service answered {} {}",
        held.status,
        held.body
    );
}
