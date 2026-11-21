//! The acceptance case: the timeline of `ess/scenarios/post-read-answer.yaml`, step by step,
//! through the generated `handle` over `MemoryBackend`, then its assertions on the views.

use inbox::server::json::Value;

use crate::support::{KeyIdVerifier, World, outcome, published, text};

fn event(answer: &Value, name: &str) -> Value {
    let events = published(answer);
    assert_eq!(events.len(), 1, "one event: {events:?}");
    assert_eq!(events[0].0, name);
    events[0].1.clone()
}

#[test]
fn post_read_answer_settles_the_request_for_its_sender() {
    let world = World::new(KeyIdVerifier::accepting());

    let opened = world.client(
        "agent:box/worker",
        "inbox.messaging.OpenInbox",
        r#"{"address":"agent:box/worker","adapter":"claude_code"}"#,
    );
    assert_eq!(outcome(&opened), "opened");
    let payload = event(&opened, "inbox.messaging.InboxOpened");
    assert_eq!(text(&payload, "address"), "agent:box/worker");

    let posted = world.client(
        "agent:ws/planner",
        "inbox.messaging.PostMessage",
        r#"{
            "message_id": "01JGT0PA000000000000000001",
            "from": "agent:ws/planner",
            "on_behalf_of": "agent:ws/planner",
            "to": "agent:box/worker",
            "kind": "request",
            "thread": "01JGT0PA000000000000000001",
            "reply_shape": {"shape": "lines", "value": 3},
            "created": "2026-01-05T09:00:00Z",
            "expires": "2026-01-06T09:00:00Z",
            "dedup_key": "01JGT0PA000000000000000001",
            "body": "Which crate holds the retry rule?",
            "signature": {"key_id": "key-1", "value": "c2lnbmF0dXJl"}
        }"#,
    );
    assert_eq!(outcome(&posted), "posted");
    let payload = event(&posted, "inbox.messaging.MessagePosted");
    assert_eq!(text(&payload, "message_id"), "01JGT0PA000000000000000001");
    assert_eq!(text(&payload, "to"), "agent:box/worker");
    assert_eq!(text(&payload, "from"), "agent:ws/planner");
    assert_eq!(text(&payload, "thread"), "01JGT0PA000000000000000001");
    assert_eq!(text(&payload, "kind"), "request");

    let delivered = world.client(
        "agent:box/worker",
        "inbox.messaging.MarkDelivered",
        r#"{"message_id": "01JGT0PA000000000000000001"}"#,
    );
    assert_eq!(outcome(&delivered), "delivered");
    let payload = event(&delivered, "inbox.messaging.MessageDelivered");
    assert_eq!(text(&payload, "message_id"), "01JGT0PA000000000000000001");

    let read = world.client(
        "agent:box/worker",
        "inbox.messaging.MarkRead",
        r#"{"message_id": "01JGT0PA000000000000000001"}"#,
    );
    assert_eq!(outcome(&read), "read");
    let payload = event(&read, "inbox.messaging.MessageRead");
    assert_eq!(text(&payload, "message_id"), "01JGT0PA000000000000000001");

    let answered = world.client(
        "agent:box/worker",
        "inbox.messaging.Answer",
        r#"{
            "in_reply_to": "01JGT0PA000000000000000001",
            "reply_id": "01JGT0PA000000000000000002",
            "from": "agent:box/worker",
            "on_behalf_of": "agent:box/worker",
            "created": "2026-01-05T09:05:00Z",
            "expires": "2026-01-06T09:05:00Z",
            "body": "It is in the delivery module.",
            "signature": {"key_id": "key-2", "value": "c2lnbmF0dXJl"}
        }"#,
    );
    assert_eq!(outcome(&answered), "answered");
    let payload = event(&answered, "inbox.messaging.MessageAnswered");
    assert_eq!(text(&payload, "message_id"), "01JGT0PA000000000000000001");
    assert_eq!(text(&payload, "reply_id"), "01JGT0PA000000000000000002");
    assert_eq!(text(&payload, "to"), "agent:ws/planner");
    assert_eq!(text(&payload, "thread"), "01JGT0PA000000000000000001");

    let pending = world.rows(
        "inbox.messaging.Pending",
        r#"{"inbox": "agent:box/worker"}"#,
    );
    assert!(pending.is_empty(), "Pending: {pending:?}");
    let unanswered = world.rows(
        "inbox.messaging.Unanswered",
        r#"{"inbox": "agent:box/worker"}"#,
    );
    assert!(unanswered.is_empty(), "Unanswered: {unanswered:?}");

    let settled = world.rows(
        "inbox.messaging.Settled",
        r#"{"sender": "agent:ws/planner"}"#,
    );
    let row = settled
        .iter()
        .find(|row| text(row, "message_id") == "01JGT0PA000000000000000001")
        .unwrap_or_else(|| panic!("Settled holds the request: {settled:?}"));
    assert_eq!(text(row, "state"), "Answered");
    assert_eq!(text(row, "answered_by"), "principal");
    assert_eq!(text(row, "expires"), "2026-01-06T09:05:00Z");

    let replies = world.rows(
        "inbox.messaging.Replies",
        r#"{"thread": "01JGT0PA000000000000000001"}"#,
    );
    assert_eq!(replies.len(), 1, "Replies: {replies:?}");
}
