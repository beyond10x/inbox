//! Adversary cases of stage 2, wave 6, story:foreign-message-id-answers-as-unknown, over HTTP.
//! Each case is red on the tree it was written against and names what it shows. Every key is
//! created when the test runs.

mod support;

use inbox::behaviour::MessageStorage;
use inbox::messaging::{DedupKey, MessageId};
use support::{AGENT, OTHER, THIRD, World, encoded, ulid};

const POST: &str = "/messaging/commands/post";

/// An address no inbox has: a post to it is refused and stores nothing.
const NOBODY: &str = "agent:host-z/nobody";

fn store(world: &World) -> String {
    let mut held: Vec<String> = MessageStorage::list(&world.backend)
        .iter()
        .map(|message| format!("{message:?}"))
        .collect();
    held.sort();
    held.join("\n")
}

/// A third principal posts, as itself and signed with its own key, an envelope to an address
/// no inbox has, once with the `message_id` of a message between two others and once with an id
/// nobody used. Neither post stores anything. Before story:post-does-not-tell-a-message-id-exists
/// `post` answered `duplicate` for the first (`existing_instance`, before `unknown-inbox`) and
/// `unknown-inbox` for the second, so a principal that is neither party read the existence of the
/// message off a side-effect-free post.
#[test]
fn adv_w6_a_third_principal_cannot_tell_a_message_between_others_from_no_message_by_post() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let between = world.post(AGENT, OTHER, 1, "between two others");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&between));
    assert_eq!(posted.text("outcome"), "posted", "{}", posted.body);

    let before = store(&world);
    let probe = |id: u32, key: u32| {
        let mut post = world.post(THIRD, NOBODY, id, "probe");
        post.message_id = MessageId(ulid(id));
        post.dedup_key = DedupKey(ulid(key));
        world.sign(&mut post);
        world.send(address, THIRD, "POST", POST, &encoded(&post))
    };
    let held = probe(1, 501);
    let missing = probe(99, 502);
    assert_eq!(store(&world), before, "a probe stored something");
    assert_eq!(
        (held.status, held.body.as_str()),
        (missing.status, missing.body.as_str()),
        "the id of a message between others against an id nobody used"
    );
}

/// The same pair of posts into the third principal's own closed inbox: both answer
/// `inbox-closed`, with the same status and body, and neither stores anything.
#[test]
fn adv_w6_a_third_principal_cannot_tell_a_held_id_by_post_into_a_closed_inbox() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    world.open(address, THIRD);
    let between = world.post(AGENT, OTHER, 1, "between two others");
    let posted = world.send(address, AGENT, "POST", POST, &encoded(&between));
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

    let before = store(&world);
    let probe = |id: u32, key: u32| {
        let mut post = world.post(THIRD, THIRD, id, "probe");
        post.message_id = MessageId(ulid(id));
        post.dedup_key = DedupKey(ulid(key));
        world.sign(&mut post);
        world.send(address, THIRD, "POST", POST, &encoded(&post))
    };
    let held = probe(1, 501);
    let missing = probe(99, 502);
    assert_eq!(store(&world), before, "a probe stored something");
    assert_eq!(held.text("outcome"), "inbox-closed", "{}", held.body);
    assert_eq!(
        (held.status, held.body.as_str()),
        (missing.status, missing.body.as_str()),
        "the id of a message between others against an id nobody used"
    );
}
