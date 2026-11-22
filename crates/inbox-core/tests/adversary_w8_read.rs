//! Adversary cases for story:read-decision-expires-on-silence-scenario against the core: what the
//! new authored scenario `read-decision-is-decided-on-silence` does not hold about a message its
//! recipient read and that then expires, each case stated at the core (`Expire` over a stored row
//! in the state `Read`), the layer a wrong core is written in.

use inbox::server::json::Value;

use crate::support::{
    KeyIdVerifier, Post, World, open_worker, outcome, post, published, quoted, text,
};

const ID: &str = "01JGT0W8000000000000000001";
const EXPIRES: &str = "2026-01-05T10:00:00Z";

fn world() -> World {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    world
}

fn read(world: &World, id: &str) {
    let answer = world.client(
        "agent:box/worker",
        "inbox.messaging.MarkRead",
        &format!("{{\"message_id\":{}}}", quoted(id)),
    );
    assert!(
        ["read", "handled"].contains(&outcome(&answer)),
        "{answer:?}"
    );
}

fn delivered(world: &World, id: &str) {
    let answer = world.client(
        "agent:box/worker",
        "inbox.messaging.MarkDelivered",
        &format!("{{\"message_id\":{}}}", quoted(id)),
    );
    assert_eq!(outcome(&answer), "delivered", "{answer:?}");
}

fn expire(world: &World, id: &str, now: &str, keep_until: &str) -> Value {
    world.sweeper(
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":{},\"now\":{},\"keep_until\":{}}}",
            quoted(id),
            quoted(now),
            quoted(keep_until)
        ),
    )
}

fn messages(world: &World) -> Vec<Value> {
    world.sweeper_rows("inbox.messaging.Messages", "{}")
}

fn state_of(world: &World, id: &str) -> Option<String> {
    messages(world)
        .iter()
        .find(|row| text(row, "message_id") == id)
        .map(|row| text(row, "state").to_owned())
}

fn posted_decision(default_on_silence: Option<&str>) -> World {
    let world = world();
    let mut decision = Post::decision(ID, default_on_silence);
    decision.expires = EXPIRES.to_owned();
    assert_eq!(outcome(&post(&world, &decision)), "posted");
    world
}

/// The claim of the unit, at the core, with every field the scenario reads and the event payload
/// it checks, and the instant to the second: one second before `expires` is `not-due`.
#[test]
fn a_read_decision_with_a_silence_option_is_decided_on_silence_at_expires_and_not_a_second_before()
{
    let world = posted_decision(Some("B"));
    read(&world, ID);
    assert_eq!(state_of(&world, ID).as_deref(), Some("Read"));

    let early = expire(&world, ID, "2026-01-05T09:59:59Z", "2026-01-06T09:59:59Z");
    assert_eq!(outcome(&early), "not-due", "{early:?}");
    assert_eq!(state_of(&world, ID).as_deref(), Some("Read"));

    let decided = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&decided), "decided-on-silence", "{decided:?}");
    let events = published(&decided);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "inbox.messaging.DecisionTakenOnSilence");
    assert_eq!(text(&events[0].1, "to"), "agent:ws/planner");

    let settled = world.rows(
        "inbox.messaging.Settled",
        r#"{"sender": "agent:ws/planner"}"#,
    );
    assert_eq!(settled.len(), 1, "{settled:?}");
    assert_eq!(text(&settled[0], "state"), "Answered");
    assert_eq!(text(&settled[0], "answered_by"), "silence");
    assert_eq!(text(&settled[0], "decided_option"), "B");
    assert_eq!(text(&settled[0], "expires"), "2026-01-06T10:00:00Z");
}

/// Silence option `A`: the scenario names `B`, which is also the recommended option, so a core
/// that decides a read decision with the recommended option, or with `B`, passes it.
#[test]
fn a_read_decision_is_decided_with_the_option_it_names_for_silence_when_that_is_not_the_recommended_one()
 {
    let world = posted_decision(Some("A"));
    read(&world, ID);
    let decided = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&decided), "decided-on-silence", "{decided:?}");
    let settled = world.rows(
        "inbox.messaging.Settled",
        r#"{"sender": "agent:ws/planner"}"#,
    );
    assert_eq!(text(&settled[0], "decided_option"), "A", "{settled:?}");
}

/// The third state of the guard `state: [Sent, Delivered, Read]`: a decision nobody delivered or
/// read. The one scenario that reaches `decided-on-silence` before the unit marks the message
/// delivered, and the unit's reads it; no scenario expires a decision in the state `Sent`.
#[test]
fn a_sent_decision_with_a_silence_option_is_decided_on_silence_at_expiry() {
    let world = posted_decision(Some("B"));
    assert_eq!(state_of(&world, ID).as_deref(), Some("Sent"));
    let decided = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&decided), "decided-on-silence", "{decided:?}");
    assert_eq!(state_of(&world, ID).as_deref(), Some("Answered"));
}

/// A read decision with no silence option is removed at expiry, as one nobody read is
/// (`expire_removes_a_decision_without_default_on_silence`, which never reads it).
#[test]
fn a_read_decision_without_a_silence_option_is_removed_at_expiry() {
    let world = posted_decision(None);
    read(&world, ID);
    assert_eq!(state_of(&world, ID).as_deref(), Some("Read"));
    let due = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&due), "expired", "{due:?}");
    assert_eq!(published(&due)[0].0, "inbox.messaging.MessageExpired");
    assert_eq!(state_of(&world, ID), None);
}

/// A read request is removed at expiry; only a decision is decided on silence.
#[test]
fn a_read_request_is_removed_at_expiry_not_decided_on_silence() {
    let world = world();
    let mut request = Post::request(ID, 1);
    request.expires = EXPIRES.to_owned();
    assert_eq!(outcome(&post(&world, &request)), "posted");
    read(&world, ID);
    assert_eq!(state_of(&world, ID).as_deref(), Some("Read"));
    let due = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&due), "expired", "{due:?}");
    assert_eq!(state_of(&world, ID), None);
}

/// A request nobody read, and one that was delivered, are removed at expiry as well.
#[test]
fn a_delivered_request_is_removed_at_expiry_not_decided_on_silence() {
    let world = world();
    let mut request = Post::request(ID, 1);
    request.expires = EXPIRES.to_owned();
    assert_eq!(outcome(&post(&world, &request)), "posted");
    delivered(&world, ID);
    let due = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&due), "expired", "{due:?}");
    assert_eq!(state_of(&world, ID), None);
}

/// A note nobody read is removed at expiry, not decided on silence.
#[test]
fn a_sent_note_is_removed_at_expiry_not_decided_on_silence() {
    let world = world();
    let mut note = Post::note(ID);
    note.expires = EXPIRES.to_owned();
    assert_eq!(outcome(&post(&world, &note)), "posted");
    let due = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&due), "expired", "{due:?}");
    assert_eq!(state_of(&world, ID), None);
}

/// The answer a decided read decision waits with is kept until `keep_until` and no longer, as
/// the one a delivered decision gets (`expire_answers_a_decision_naming_default_on_silence_by_silence`).
#[test]
fn a_decided_read_decision_waits_until_keep_until_and_is_then_removed() {
    let world = posted_decision(Some("B"));
    read(&world, ID);
    let decided = expire(&world, ID, EXPIRES, "2026-01-06T10:00:00Z");
    assert_eq!(outcome(&decided), "decided-on-silence", "{decided:?}");
    let kept = expire(&world, ID, "2026-01-06T09:59:59Z", "2026-01-07T09:59:59Z");
    assert_eq!(outcome(&kept), "not-due", "{kept:?}");
    let removed = expire(&world, ID, "2026-01-06T10:00:00Z", "2026-01-07T10:00:00Z");
    assert_eq!(outcome(&removed), "expired", "{removed:?}");
    assert_eq!(state_of(&world, ID), None);
}
