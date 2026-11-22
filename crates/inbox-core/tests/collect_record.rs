//! story:collect-returns-the-settled-record against the core: `Collect` answers with the settled
//! record it removes, read from the row in the request that deletes it: every member of its
//! `Outbox` row, among them its state, who answered, the decided option, the reply and the
//! refusal. A message that is not settled, a third principal and the recipient get no record, and
//! the row stays.

use crate::support::{
    KeyIdVerifier, Post, Reply, World, answer, open_worker, outcome, post, published, text,
};

use inbox::behaviour::MessageStorage;
use inbox::messaging::MessageId;
use inbox::server::json::Value;

const PLANNER: &str = "agent:ws/planner";
const WORKER: &str = "agent:box/worker";
const OTHER: &str = "agent:box/other";
const REQUEST: &str = "01JGT0CR000000000000000001";
const DECISION: &str = "01JGT0CR000000000000000002";
const REPLY: &str = "01JGT0CR000000000000000050";
const COLLECT: &str = "inbox.messaging.Collect";

fn collect(world: &World, who: &str, id: &str) -> Value {
    world.client(who, COLLECT, &format!("{{\"message_id\":\"{id}\"}}"))
}

fn response(answer: &Value) -> &Value {
    answer
        .member("response")
        .unwrap_or_else(|| panic!("no `response` in the answer {answer:?}"))
}

fn held(world: &World, id: &str) -> bool {
    MessageStorage::get(&world.backend, &MessageId(id.to_owned())).is_some()
}

fn requested() -> World {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    assert_eq!(outcome(&post(&world, &Post::request(REQUEST, 3))), "posted");
    world
}

#[test]
fn collect_answers_with_the_reply_of_the_row_it_deleted() {
    let world = requested();
    let reply = Reply {
        body: "It is in the delivery module.".to_owned(),
        ..Reply::to(REQUEST, REPLY)
    };
    assert_eq!(outcome(&answer(&world, &reply)), "answered");

    let collected = collect(&world, PLANNER, REQUEST);
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    let record = response(&collected);
    assert_eq!(text(record, "message_id"), REQUEST);
    assert_eq!(text(record, "to"), WORKER);
    assert_eq!(text(record, "thread"), REQUEST);
    assert_eq!(text(record, "state"), "Answered");
    assert_eq!(text(record, "answered_by"), "principal");
    let carried = record
        .member("reply")
        .expect("the record carries the reply");
    assert_eq!(text(carried, "reply_id"), REPLY);
    assert_eq!(text(carried, "from"), WORKER);
    assert_eq!(text(carried, "body"), "It is in the delivery module.");
    assert!(record.member("refusal").is_none(), "{record:?}");
    let events = published(&collected);
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].0, "inbox.messaging.MessageHandled");
    assert_eq!(text(&events[0].1, "message_id"), REQUEST);
    assert!(!held(&world, REQUEST), "a collected message is removed");

    let again = collect(&world, PLANNER, REQUEST);
    assert_eq!(outcome(&again), "unknown-message");
    assert!(again.member("response").is_none(), "{again:?}");
}

#[test]
fn collect_answers_with_the_refusal_of_the_row_it_deleted() {
    let world = requested();
    let refused = world.client(
        WORKER,
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":\"{REQUEST}\",\"reason\":\"Outside this repository.\"}}"),
    );
    assert_eq!(outcome(&refused), "refused");

    let collected = collect(&world, PLANNER, REQUEST);
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    let record = response(&collected);
    assert_eq!(text(record, "state"), "Refused");
    assert_eq!(text(record, "refusal"), "Outside this repository.");
    assert!(record.member("reply").is_none(), "{record:?}");
    assert!(record.member("answered_by").is_none(), "{record:?}");
    assert!(!held(&world, REQUEST));
}

#[test]
fn collect_answers_with_the_option_silence_took() {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    assert_eq!(
        outcome(&post(&world, &Post::decision(DECISION, Some("A")))),
        "posted"
    );
    let decided = world.sweeper(
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":\"{DECISION}\",\"now\":\"2026-01-06T09:00:00Z\",\
             \"keep_until\":\"2026-01-07T09:00:00Z\"}}"
        ),
    );
    assert_eq!(outcome(&decided), "decided-on-silence");

    let collected = collect(&world, PLANNER, DECISION);
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    let record = response(&collected);
    assert_eq!(text(record, "state"), "Answered");
    assert_eq!(text(record, "answered_by"), "silence");
    assert_eq!(text(record, "decided_option"), "A");
    assert!(record.member("reply").is_none(), "{record:?}");
}

#[test]
fn collect_of_a_message_not_settled_removes_nothing_and_answers_no_record() {
    let world = requested();
    let early = collect(&world, PLANNER, REQUEST);
    assert_eq!(outcome(&early), "not-settled");
    assert!(early.member("response").is_none(), "{early:?}");
    assert!(published(&early).is_empty());
    assert!(held(&world, REQUEST), "the message stays to collect");

    assert_eq!(
        outcome(&answer(&world, &Reply::to(REQUEST, REPLY))),
        "answered"
    );
    let collected = collect(&world, PLANNER, REQUEST);
    assert_eq!(outcome(&collected), "collected");
    assert_eq!(text(response(&collected), "state"), "Answered");
}

#[test]
fn a_third_principal_and_the_recipient_get_no_record() {
    let world = requested();
    assert_eq!(
        outcome(&answer(&world, &Reply::to(REQUEST, REPLY))),
        "answered"
    );
    for (who, expected) in [(OTHER, "unknown-message"), (WORKER, "not-sender")] {
        let refused = collect(&world, who, REQUEST);
        assert_eq!(outcome(&refused), expected, "{who}");
        assert!(refused.member("response").is_none(), "{who}: {refused:?}");
        assert!(published(&refused).is_empty(), "{who}");
        assert!(held(&world, REQUEST), "{who} removed the message");
    }
}

/// Every member the sender's `Outbox` row of the message shows the moment before the collect is
/// in the record, with the same value, and the record has no other: a decision spoken for another
/// principal under a delegation, with a reply shape, a default on silence, and a reply under a
/// delegation that names an option and replaces `expires` with its own (the instant its
/// signature covers).
#[test]
fn the_record_holds_every_member_of_the_outbox_row_of_the_moment_before() {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    let decision = Post {
        on_behalf_of: "agent:ws/lead".to_owned(),
        authority: Some("dlg-lead-1".to_owned()),
        reply_shape: Some("{\"shape\":\"lines\",\"value\":2}".to_owned()),
        dedup_key: "release-7".to_owned(),
        body: "Ship \"7\" \\ now?\nSecond line \u{1F680}".to_owned(),
        ..Post::decision(DECISION, Some("B"))
    };
    assert_eq!(outcome(&post(&world, &decision)), "posted");
    let reply = Reply {
        on_behalf_of: "agent:box/boss".to_owned(),
        authority: Some("dlg-boss-1".to_owned()),
        option: Some("A".to_owned()),
        body: "Ship it.".to_owned(),
        ..Reply::to(DECISION, REPLY)
    };
    assert_eq!(outcome(&answer(&world, &reply)), "answered");

    let rows = world.rows(
        "inbox.messaging.Outbox",
        &format!("{{\"sender\":\"{PLANNER}\"}}"),
    );
    let row = rows
        .iter()
        .find(|row| text(row, "message_id") == DECISION)
        .expect("the outbox shows the decision");
    let collected = collect(&world, PLANNER, DECISION);
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    let record = response(&collected);

    let names = |value: &Value| -> Vec<String> {
        let Value::Object(members) = value else {
            panic!("not an object: {value:?}")
        };
        let mut names: Vec<String> = members.iter().map(|(name, _)| name.clone()).collect();
        names.sort();
        names
    };
    assert_eq!(names(record), names(row), "the record's members");
    for name in names(row) {
        assert_eq!(
            record.member(&name),
            row.member(&name),
            "`{name}` of the record"
        );
    }
    for name in [
        "authority",
        "reply_shape",
        "decision",
        "default_on_silence",
        "decided_option",
        "reply",
    ] {
        assert!(
            row.member(name).is_some(),
            "the case sets `{name}`: {row:?}"
        );
    }
    assert_eq!(text(record, "expires"), reply.expires);
    assert_eq!(text(record, "body"), decision.body);
    assert!(!held(&world, DECISION));
}

/// The contract orders `foreign-message` (which `handle` answers as `unknown-message`) and
/// `not-sender` before `collected` and `not-settled`:
/// on a message that is not settled, a third principal and the recipient get their refusal, not
/// `not-settled`, which would tell them the message exists and is still open.
#[test]
fn on_an_unsettled_message_the_caller_refusals_come_before_not_settled() {
    let world = requested();
    for (who, expected) in [(OTHER, "unknown-message"), (WORKER, "not-sender")] {
        let refused = collect(&world, who, REQUEST);
        assert_eq!(outcome(&refused), expected, "{who}: {refused:?}");
        assert!(refused.member("response").is_none(), "{who}: {refused:?}");
        assert!(held(&world, REQUEST), "{who} removed the message");
    }
    assert_eq!(outcome(&collect(&world, PLANNER, REQUEST)), "not-settled");
}
