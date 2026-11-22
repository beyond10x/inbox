//! Each of the 5 `external:` branches the service context answers, once taken and once not taken:
//! 2 of `PostMessage` and 3 of `Answer`; and the declared guard `unknown-option` of `PostMessage`,
//! which the core decides from the envelope.

use crate::support::{
    KeyIdVerifier, Post, Reply, World, answer, open_worker, outcome, post, published,
};

const REQUEST: &str = "01JGT0XB000000000000000001";
const DECISION: &str = "01JGT0XB000000000000000003";
const REPLY: &str = "01JGT0XB000000000000000002";

fn stored(world: &World) -> usize {
    world.sweeper_rows("inbox.messaging.Messages", "{}").len()
}

fn world() -> World {
    let world = World::new(
        KeyIdVerifier::accepting()
            .refusing_key("key-refused")
            .refusing_delegation("delegation-lapsed"),
    );
    open_worker(&world);
    world
}

/// A world holding one request, at most 1 line, posted and read.
fn world_with_request() -> World {
    let world = world();
    assert_eq!(outcome(&post(&world, &Post::request(REQUEST, 1))), "posted");
    let read = world.client(
        "agent:box/worker",
        "inbox.messaging.MarkRead",
        &format!("{{\"message_id\":\"{REQUEST}\"}}"),
    );
    assert_eq!(outcome(&read), "read");
    world
}

/// A world holding one decision between `A` and `B`.
fn world_with_decision() -> World {
    let world = world();
    let decision = Post::decision(DECISION, Some("B"));
    assert_eq!(outcome(&post(&world, &decision)), "posted");
    world
}

fn refused(world: &World, answer: &inbox::server::json::Value, expected: &str, before: usize) {
    assert_eq!(outcome(answer), expected, "{answer:?}");
    assert!(published(answer).is_empty(), "a refusal publishes nothing");
    assert_eq!(stored(world), before, "a refusal stores nothing");
}

#[test]
fn post_unknown_option_taken_for_a_silence_option_the_decision_does_not_offer() {
    let world = world();
    let answer = post(&world, &Post::decision(DECISION, Some("C")));
    refused(&world, &answer, "unknown-option", 0);
}

#[test]
fn post_unknown_option_taken_for_a_recommended_option_the_decision_does_not_offer() {
    let world = world();
    let mut decision = Post::decision(DECISION, None);
    decision.decision =
        Some(r#"{"options":[{"letter":"A","text":"Ship."}],"recommended_option":"Z"}"#.to_owned());
    let answer = post(&world, &decision);
    refused(&world, &answer, "unknown-option", 0);
}

#[test]
fn post_unknown_option_taken_for_two_options_with_one_letter() {
    let world = world();
    let mut decision = Post::decision(DECISION, Some("A"));
    decision.decision = Some(
        r#"{"options":[{"letter":"A","text":"Ship."},{"letter":"A","text":"Hold."}]}"#.to_owned(),
    );
    let answer = post(&world, &decision);
    refused(&world, &answer, "unknown-option", 0);
}

#[test]
fn post_unknown_option_not_taken_for_options_the_decision_offers() {
    let world = world();
    let answer = post(&world, &Post::decision(DECISION, Some("B")));
    assert_eq!(outcome(&answer), "posted");
    assert_eq!(stored(&world), 1);
}

#[test]
fn post_signature_invalid_taken_for_a_key_the_verifier_refuses() {
    let world = world();
    let mut note = Post::note(REQUEST);
    note.key_id = "key-refused".to_owned();
    let answer = post(&world, &note);
    refused(&world, &answer, "signature-invalid", 0);
}

#[test]
fn post_signature_invalid_not_taken_for_a_key_the_verifier_accepts() {
    let world = world();
    let answer = post(&world, &Post::note(REQUEST));
    assert_eq!(outcome(&answer), "posted");
    assert_eq!(stored(&world), 1);
}

fn relayed(authority: &str) -> Post {
    let mut note = Post::note(REQUEST);
    note.from = "agent:ws/host-operator".to_owned();
    note.on_behalf_of = "human:operator".to_owned();
    note.authority = Some(authority.to_owned());
    note
}

#[test]
fn post_authority_invalid_taken_for_a_delegation_the_verifier_refuses() {
    let world = world();
    let answer = post(&world, &relayed("delegation-lapsed"));
    refused(&world, &answer, "authority-invalid", 0);
}

#[test]
fn post_authority_invalid_not_taken_for_a_delegation_the_verifier_accepts() {
    let world = world();
    let answer = post(&world, &relayed("delegation-7"));
    assert_eq!(outcome(&answer), "posted");
    assert_eq!(stored(&world), 1);
}

#[test]
fn answer_signature_invalid_taken_for_a_key_the_verifier_refuses() {
    let world = world_with_request();
    let mut reply = Reply::to(REQUEST, REPLY);
    reply.key_id = "key-refused".to_owned();
    let answered = answer(&world, &reply);
    refused(&world, &answered, "signature-invalid", 1);
}

#[test]
fn answer_signature_invalid_not_taken_for_a_key_the_verifier_accepts() {
    let world = world_with_request();
    let answered = answer(&world, &Reply::to(REQUEST, REPLY));
    assert_eq!(outcome(&answered), "answered");
}

#[test]
fn answer_authority_invalid_taken_for_a_delegation_the_verifier_refuses() {
    let world = world_with_request();
    let mut reply = Reply::to(REQUEST, REPLY);
    reply.on_behalf_of = "human:operator".to_owned();
    reply.authority = Some("delegation-lapsed".to_owned());
    let answered = answer(&world, &reply);
    refused(&world, &answered, "authority-invalid", 1);
}

#[test]
fn answer_authority_invalid_not_taken_for_a_delegation_the_verifier_accepts() {
    let world = world_with_request();
    let mut reply = Reply::to(REQUEST, REPLY);
    reply.on_behalf_of = "human:operator".to_owned();
    reply.authority = Some("delegation-7".to_owned());
    let answered = answer(&world, &reply);
    assert_eq!(outcome(&answered), "answered");
}

#[test]
fn answer_shape_mismatch_taken_for_more_lines_than_the_request_allows() {
    let world = world_with_request();
    let mut reply = Reply::to(REQUEST, REPLY);
    reply.body = "First line.\nSecond line.\nThird line.".to_owned();
    let answered = answer(&world, &reply);
    refused(&world, &answered, "shape-mismatch", 1);
}

#[test]
fn answer_shape_mismatch_not_taken_for_a_reply_that_fits_the_request() {
    let world = world_with_request();
    let answered = answer(&world, &Reply::to(REQUEST, REPLY));
    assert_eq!(outcome(&answered), "answered");
}

#[test]
fn answer_shape_mismatch_taken_for_a_decision_answered_with_no_option_or_an_unoffered_one() {
    let world = world_with_decision();
    let mut reply = Reply::to(DECISION, REPLY);
    let answered = answer(&world, &reply);
    refused(&world, &answered, "shape-mismatch", 1);
    reply.option = Some("C".to_owned());
    let answered = answer(&world, &reply);
    refused(&world, &answered, "shape-mismatch", 1);
}

#[test]
fn answer_shape_mismatch_not_taken_for_a_decision_answered_with_an_offered_option() {
    let world = world_with_decision();
    let mut reply = Reply::to(DECISION, REPLY);
    reply.option = Some("A".to_owned());
    let answered = answer(&world, &reply);
    assert_eq!(outcome(&answered), "answered");
}

#[test]
fn answer_shape_mismatch_taken_for_a_request_answered_with_an_option() {
    let world = world_with_request();
    let mut reply = Reply::to(REQUEST, REPLY);
    reply.option = Some("A".to_owned());
    let answered = answer(&world, &reply);
    refused(&world, &answered, "shape-mismatch", 1);
}

/// `decision-without-options` also takes a decision whose `options` list is empty: no letter
/// could answer it.
#[test]
fn post_decision_without_options_taken_for_an_empty_options_list() {
    let world = world();
    let mut decision = Post::decision(DECISION, None);
    decision.decision = Some(r#"{"options":[]}"#.to_owned());
    let answer = post(&world, &decision);
    refused(&world, &answer, "decision-without-options", 0);
    assert_eq!(
        crate::support::text(&answer, "error"),
        "inbox.messaging.ShapeMismatch"
    );
}

/// A note asks for nothing: `Refuse` of it answers `note` and leaves the stored note as it was.
#[test]
fn refuse_of_a_note_answers_note_and_leaves_it_as_it_was() {
    let world = world();
    assert_eq!(outcome(&post(&world, &Post::note(REQUEST))), "posted");
    let before = world.sweeper_rows("inbox.messaging.Messages", "{}");
    let refused_note = world.client(
        "agent:box/worker",
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":\"{REQUEST}\",\"reason\":\"Not for this session.\"}}"),
    );
    refused(&world, &refused_note, "note", 1);
    assert_eq!(
        crate::support::text(&refused_note, "error"),
        "inbox.messaging.NotAnswerable"
    );
    assert_eq!(world.sweeper_rows("inbox.messaging.Messages", "{}"), before);
}
