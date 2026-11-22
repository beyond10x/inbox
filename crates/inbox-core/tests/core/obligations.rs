//! 2 of the 3 obligations beyond their external branches: dedup of `PostMessage` through `list`,
//! its other refusals in the order of the contract, and the 3 outcomes of `Expire` on a stored
//! row. The cases of `Collect` are in `tests/collect_record.rs`.

use crate::support::{
    KeyIdVerifier, Post, World, open_worker, outcome, post, published, quoted, text,
};

const FIRST: &str = "01JGT0DK000000000000000001";
const SECOND: &str = "01JGT0DK000000000000000002";

/// The fields `tests/adversary.rs` (`posted_stores_every_envelope_field_as_sent`) does not read
/// back: the identity, the kind, and the letter and text of each option, in order.
#[test]
fn posted_stores_the_identity_the_kind_and_every_option() {
    use inbox::behaviour::MessageStorage;
    use inbox::messaging::{Kind, MessageId};

    let world = world();
    assert_eq!(
        outcome(&post(&world, &Post::decision(FIRST, Some("B")))),
        "posted"
    );
    let row = MessageStorage::get(&world.backend, &MessageId(FIRST.to_owned())).expect("stored");
    assert_eq!(row.data.message_id.0, FIRST);
    assert_eq!(row.data.kind, Kind::Decision);
    let options: Vec<(String, String)> = row
        .data
        .decision
        .expect("decision stored")
        .options
        .into_iter()
        .map(|option| (option.letter.0, option.text))
        .collect();
    assert_eq!(
        options,
        vec![
            ("A".to_owned(), "Ship the release today.".to_owned()),
            ("B".to_owned(), "Hold the release until Monday.".to_owned()),
        ]
    );
}

fn world() -> World {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    world
}

fn ids(world: &World) -> Vec<String> {
    world
        .sweeper_rows("inbox.messaging.Messages", "{}")
        .iter()
        .map(|row| text(row, "message_id").to_owned())
        .collect()
}

fn expire(
    world: &World,
    message_id: &str,
    now: &str,
    keep_until: &str,
) -> inbox::server::json::Value {
    world.sweeper(
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":{},\"now\":{},\"keep_until\":{}}}",
            quoted(message_id),
            quoted(now),
            quoted(keep_until)
        ),
    )
}

#[test]
fn duplicate_key_refuses_a_second_message_with_a_live_dedup_key() {
    let world = world();
    let mut first = Post::note(FIRST);
    first.dedup_key = "deploy-42".to_owned();
    assert_eq!(outcome(&post(&world, &first)), "posted");

    let mut second = Post::note(SECOND);
    second.dedup_key = "deploy-42".to_owned();
    let answer = post(&world, &second);
    assert_eq!(outcome(&answer), "duplicate-key");
    assert_eq!(text(&answer, "error"), "inbox.messaging.Duplicate");
    assert!(published(&answer).is_empty());
    assert_eq!(ids(&world), vec![FIRST.to_owned()]);
}

#[test]
fn duplicate_key_not_taken_for_another_key_or_once_the_first_is_gone() {
    let world = world();
    let mut first = Post::note(FIRST);
    first.dedup_key = "deploy-42".to_owned();
    assert_eq!(outcome(&post(&world, &first)), "posted");

    let mut other = Post::note(SECOND);
    other.dedup_key = "deploy-43".to_owned();
    assert_eq!(outcome(&post(&world, &other)), "posted");

    let read = world.client(
        "agent:box/worker",
        "inbox.messaging.MarkRead",
        &format!("{{\"message_id\":{}}}", quoted(FIRST)),
    );
    assert_eq!(outcome(&read), "handled");
    let mut again = Post::note("01JGT0DK000000000000000003");
    again.dedup_key = "deploy-42".to_owned();
    assert_eq!(outcome(&post(&world, &again)), "posted");
}

#[test]
fn duplicate_is_answered_before_duplicate_key_for_a_repeated_message_id() {
    let world = world();
    assert_eq!(outcome(&post(&world, &Post::note(FIRST))), "posted");
    let answer = post(&world, &Post::note(FIRST));
    assert_eq!(outcome(&answer), "duplicate");
    assert_eq!(ids(&world), vec![FIRST.to_owned()]);
}

#[test]
fn input_refusals_answer_in_declaration_order() {
    let world = world();
    let mut relayed = Post::note(FIRST);
    relayed.on_behalf_of = "human:operator".to_owned();
    relayed.expires = relayed.created.clone();
    assert_eq!(outcome(&post(&world, &relayed)), "no-authority");

    let mut expired = Post::request(FIRST, 1);
    expired.reply_shape = None;
    expired.expires = expired.created.clone();
    assert_eq!(outcome(&post(&world, &expired)), "already-expired");

    let mut shapeless = Post::request(FIRST, 1);
    shapeless.reply_shape = None;
    assert_eq!(outcome(&post(&world, &shapeless)), "request-without-shape");

    let mut optionless = Post::decision(FIRST, Some("B"));
    optionless.decision = None;
    assert_eq!(
        outcome(&post(&world, &optionless)),
        "decision-without-options"
    );
    assert!(ids(&world).is_empty());
}

#[test]
fn unknown_and_closed_inboxes_refuse_a_post() {
    let world = world();
    let mut nowhere = Post::note(FIRST);
    nowhere.to = "agent:box/nobody".to_owned();
    assert_eq!(outcome(&post(&world, &nowhere)), "unknown-inbox");

    let closed = world.client(
        "agent:box/worker",
        "inbox.messaging.CloseInbox",
        r#"{"address":"agent:box/worker"}"#,
    );
    assert_eq!(outcome(&closed), "closed");
    assert_eq!(outcome(&post(&world, &Post::note(FIRST))), "inbox-closed");
    assert!(ids(&world).is_empty());
}

#[test]
fn expire_removes_a_note_past_expires_and_not_before() {
    let world = world();
    let mut note = Post::note(FIRST);
    note.expires = "2026-01-05T09:30:00Z".to_owned();
    assert_eq!(outcome(&post(&world, &note)), "posted");

    let early = expire(
        &world,
        FIRST,
        "2026-01-05T09:29:00Z",
        "2026-01-06T09:29:00Z",
    );
    assert_eq!(outcome(&early), "not-due");
    assert_eq!(text(&early, "error"), "inbox.messaging.NotDue");
    assert_eq!(ids(&world), vec![FIRST.to_owned()]);

    let due = expire(
        &world,
        FIRST,
        "2026-01-05T09:30:00Z",
        "2026-01-06T09:30:00Z",
    );
    assert_eq!(outcome(&due), "expired");
    let events = published(&due);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "inbox.messaging.MessageExpired");
    assert_eq!(text(&events[0].1, "message_id"), FIRST);
    assert!(ids(&world).is_empty());

    let gone = expire(
        &world,
        FIRST,
        "2026-01-05T09:31:00Z",
        "2026-01-06T09:31:00Z",
    );
    assert_eq!(outcome(&gone), "unknown-message");
}

#[test]
fn expire_answers_a_decision_naming_default_on_silence_by_silence() {
    let world = world();
    let mut decision = Post::decision(FIRST, Some("B"));
    decision.expires = "2026-01-05T10:00:00Z".to_owned();
    assert_eq!(outcome(&post(&world, &decision)), "posted");

    let decided = expire(
        &world,
        FIRST,
        "2026-01-05T10:00:00Z",
        "2026-01-06T10:00:00Z",
    );
    assert_eq!(outcome(&decided), "decided-on-silence");
    let events = published(&decided);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "inbox.messaging.DecisionTakenOnSilence");
    assert_eq!(text(&events[0].1, "message_id"), FIRST);
    assert_eq!(text(&events[0].1, "to"), "agent:ws/planner");
    assert_eq!(text(&events[0].1, "thread"), FIRST);

    let settled = world.rows(
        "inbox.messaging.Settled",
        r#"{"sender": "agent:ws/planner"}"#,
    );
    assert_eq!(settled.len(), 1, "{settled:?}");
    assert_eq!(text(&settled[0], "state"), "Answered");
    assert_eq!(text(&settled[0], "answered_by"), "silence");
    assert_eq!(text(&settled[0], "decided_option"), "B");
    assert_eq!(text(&settled[0], "expires"), "2026-01-06T10:00:00Z");

    let kept = expire(
        &world,
        FIRST,
        "2026-01-06T09:59:00Z",
        "2026-01-07T09:59:00Z",
    );
    assert_eq!(outcome(&kept), "not-due");
    let removed = expire(
        &world,
        FIRST,
        "2026-01-06T10:00:00Z",
        "2026-01-07T10:00:00Z",
    );
    assert_eq!(outcome(&removed), "expired");
    assert!(ids(&world).is_empty());
}

#[test]
fn expire_removes_a_decision_without_default_on_silence() {
    let world = world();
    let mut decision = Post::decision(FIRST, None);
    decision.expires = "2026-01-05T10:00:00Z".to_owned();
    assert_eq!(outcome(&post(&world, &decision)), "posted");
    let due = expire(
        &world,
        FIRST,
        "2026-01-05T10:00:00Z",
        "2026-01-06T10:00:00Z",
    );
    assert_eq!(outcome(&due), "expired");
    assert!(ids(&world).is_empty());
}
