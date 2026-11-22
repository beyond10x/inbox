//! Adversary pass 1 on story:the-core-applies-the-read-rule: the read rule through
//! `inbox_core::handle_with_events` in every state a message is in, with a delegation and a
//! fan-out, through the events and command answers a third principal gets, and the rule
//! `inbox_core::readable` itself on the boundaries the views never produce.

use inbox::actor::{Actor, Caller};
use inbox::messaging::{Address, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, MemoryBackend, ServiceContext};

use crate::support::{KeyIdVerifier, Post, Reply, World, answer, open_worker, outcome, post};

pub(crate) const PLANNER: &str = "agent:ws/planner";
pub(crate) const WORKER: &str = "agent:box/worker";
pub(crate) const SECOND: &str = "agent:box/second";
pub(crate) const THIRD: &str = "agent:box/other";
pub(crate) const LEAD: &str = "agent:ws/lead";

pub(crate) const SENT: &str = "01JGT0AR000000000000000001";
pub(crate) const DELIVERED: &str = "01JGT0AR000000000000000002";
pub(crate) const READ: &str = "01JGT0AR000000000000000003";
pub(crate) const ANSWERED: &str = "01JGT0AR000000000000000004";
pub(crate) const REFUSED: &str = "01JGT0AR000000000000000005";
pub(crate) const SILENCE: &str = "01JGT0AR000000000000000006";
pub(crate) const DELEGATED: &str = "01JGT0AR000000000000000007";
pub(crate) const FAN_A: &str = "01JGT0AR000000000000000008";
pub(crate) const FAN_B: &str = "01JGT0AR000000000000000009";
pub(crate) const IDS: [&str; 9] = [
    SENT, DELIVERED, READ, ANSWERED, REFUSED, SILENCE, DELEGATED, FAN_A, FAN_B,
];

pub(crate) fn as_client(who: &str) -> CallerAttributes {
    CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()))
}

pub(crate) fn call(
    backend: &MemoryBackend,
    actor: Actor,
    caller: CallerAttributes,
    name: &str,
    input: &str,
) -> (Result<Value, Refused>, usize) {
    let context = ServiceContext::new(
        caller,
        KeyIdVerifier::accepting(),
        backend.clone(),
        crate::support::clock(),
    );
    let input = json::parse(input).expect("the case writes JSON");
    let (answered, events) =
        inbox_core::handle_with_events(backend, context, Some(&Caller { actor }), name, input);
    (answered, events.len())
}

pub(crate) fn ok(answered: Result<Value, Refused>, what: &str) -> Value {
    answered.unwrap_or_else(|refused| panic!("{what}: {refused}"))
}

fn rows_of(value: &Value, what: &str) -> Vec<Value> {
    match value.member("rows") {
        Some(Value::Array(rows)) => rows.clone(),
        other => panic!("{what} answered no rows: {other:?}"),
    }
}

/// One message in each state a message is in, one spoken for another principal, and a fan-out
/// of one thread to two inboxes.
pub(crate) fn world() -> World {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    let opened = world.client(
        SECOND,
        "inbox.messaging.OpenInbox",
        &format!(r#"{{"address":"{SECOND}","adapter":"claude_code"}}"#),
    );
    assert_eq!(outcome(&opened), "opened");
    for id in [SENT, DELIVERED, READ, ANSWERED, REFUSED] {
        assert_eq!(outcome(&post(&world, &Post::request(id, 3))), "posted");
    }
    let silence = Post {
        expires: "2026-01-05T10:00:00Z".to_owned(),
        ..Post::decision(SILENCE, Some("A"))
    };
    assert_eq!(outcome(&post(&world, &silence)), "posted");
    let delegated = Post {
        on_behalf_of: LEAD.to_owned(),
        authority: Some("delegation-1".to_owned()),
        ..Post::request(DELEGATED, 3)
    };
    assert_eq!(outcome(&post(&world, &delegated)), "posted");
    assert_eq!(outcome(&post(&world, &Post::request(FAN_A, 3))), "posted");
    let fan_b = Post {
        to: SECOND.to_owned(),
        ..Post::request(FAN_B, 3)
    }
    .json()
    .replace(
        &format!(r#""thread":"{FAN_B}""#),
        &format!(r#""thread":"{FAN_A}""#),
    );
    let posted = world.client(PLANNER, "inbox.messaging.PostMessage", &fan_b);
    assert_eq!(outcome(&posted), "posted");
    let delivered = world.client(
        WORKER,
        "inbox.messaging.MarkDelivered",
        &format!(r#"{{"message_id":"{DELIVERED}"}}"#),
    );
    assert_eq!(outcome(&delivered), "delivered");
    for id in [READ, ANSWERED] {
        let read = world.client(
            WORKER,
            "inbox.messaging.MarkRead",
            &format!(r#"{{"message_id":"{id}"}}"#),
        );
        assert_eq!(outcome(&read), "read");
    }
    let answered = answer(&world, &Reply::to(ANSWERED, "01JGT0AR000000000000000099"));
    assert_eq!(outcome(&answered), "answered");
    let refused = world.client(
        WORKER,
        "inbox.messaging.Refuse",
        &format!(r#"{{"message_id":"{REFUSED}","reason":"Not mine."}}"#),
    );
    assert_eq!(outcome(&refused), "refused");
    let expired = world.sweeper(
        "inbox.messaging.Expire",
        &format!(
            r#"{{"message_id":"{SILENCE}","now":"2026-01-05T11:00:00Z","keep_until":"2026-01-08T00:00:00Z"}}"#
        ),
    );
    assert_eq!(outcome(&expired), "decided-on-silence");
    let all = world.sweeper_rows("inbox.messaging.Messages", "{}");
    assert_eq!(all.len(), IDS.len(), "every message is live: {all:?}");
    world
}

/// Every view of messages with every parameter a caller could send in this world.
fn every_read() -> Vec<(&'static str, String)> {
    let mut reads = Vec::new();
    for view in ["inbox.messaging.Pending", "inbox.messaging.Unanswered"] {
        for inbox in [WORKER, SECOND, PLANNER, LEAD, THIRD] {
            reads.push((view, format!(r#"{{"inbox":"{inbox}"}}"#)));
        }
    }
    for view in ["inbox.messaging.Outbox", "inbox.messaging.Settled"] {
        for sender in [PLANNER, WORKER, SECOND, LEAD, THIRD] {
            reads.push((view, format!(r#"{{"sender":"{sender}"}}"#)));
        }
    }
    for view in ["inbox.messaging.Thread", "inbox.messaging.Replies"] {
        for thread in IDS {
            reads.push((view, format!(r#"{{"thread":"{thread}"}}"#)));
        }
    }
    reads
}

#[test]
fn a_third_principal_reads_no_row_in_any_state_through_handle_with_events() {
    let world = world();
    let mut seen_by_parties = 0;
    for (view, params) in every_read() {
        let (answered, events) = call(
            &world.backend,
            Actor::Client,
            as_client(THIRD),
            view,
            &params,
        );
        let rows = rows_of(&ok(answered, view), view);
        assert!(
            rows.is_empty(),
            "{view} {params} answered {THIRD}: {rows:?}"
        );
        assert_eq!(events, 0, "{view} {params} handed {THIRD} events");
        for party in [PLANNER, WORKER, SECOND] {
            let (answered, _) = call(
                &world.backend,
                Actor::Client,
                as_client(party),
                view,
                &params,
            );
            seen_by_parties += rows_of(&ok(answered, view), view).len();
        }
    }
    assert!(
        seen_by_parties > 30,
        "the parties read rows: {seen_by_parties}"
    );
}

#[test]
fn the_principal_spoken_for_reads_no_row_by_the_rule_as_written() {
    // The story's rule is `to` or `from`; `on_behalf_of` names no party. The principal a
    // delegate speaks for is not the sender and owns no inbox in `to`, so it reads nothing.
    let world = world();
    for (view, params) in every_read() {
        let (answered, _) = call(
            &world.backend,
            Actor::Client,
            as_client(LEAD),
            view,
            &params,
        );
        let rows = rows_of(&ok(answered, view), view);
        assert!(rows.is_empty(), "{view} {params} answered {LEAD}: {rows:?}");
    }
    let (answered, _) = call(
        &world.backend,
        Actor::Client,
        as_client(PLANNER),
        "inbox.messaging.Thread",
        &format!(r#"{{"thread":"{DELEGATED}"}}"#),
    );
    let rows = rows_of(&ok(answered, "Thread"), "Thread");
    assert_eq!(
        rows.len(),
        1,
        "the delegate that sent it reads it: {rows:?}"
    );
}

#[test]
fn each_recipient_of_a_fan_out_reads_only_its_own_row_of_the_thread() {
    let world = world();
    let thread = format!(r#"{{"thread":"{FAN_A}"}}"#);
    for (who, id) in [(WORKER, FAN_A), (SECOND, FAN_B)] {
        let (answered, _) = call(
            &world.backend,
            Actor::Client,
            as_client(who),
            "inbox.messaging.Thread",
            &thread,
        );
        let rows = rows_of(&ok(answered, "Thread"), "Thread");
        let ids: Vec<&Value> = rows
            .iter()
            .filter_map(|row| row.member("message_id"))
            .collect();
        assert_eq!(ids, vec![&Value::Text(id.to_owned())], "{who}: {rows:?}");
    }
    let (answered, _) = call(
        &world.backend,
        Actor::Client,
        as_client(PLANNER),
        "inbox.messaging.Thread",
        &thread,
    );
    assert_eq!(rows_of(&ok(answered, "Thread"), "Thread").len(), 2);
}

/// Whether any text in `value`, at any depth, is one of `secrets`.
fn carries(value: &Value, secrets: &[&str]) -> bool {
    match value {
        Value::Text(text) => secrets.contains(&text.as_str()),
        Value::Array(items) => items.iter().any(|item| carries(item, secrets)),
        Value::Object(members) => members.iter().any(|(_, item)| carries(item, secrets)),
        _ => false,
    }
}

#[test]
fn a_third_principals_command_hands_back_no_event_and_no_content_of_the_message() {
    let world = world();
    let secrets = [
        PLANNER,
        WORKER,
        "Build 512 is green.",
        "One line.",
        "Not mine.",
    ];
    for id in IDS {
        for (name, input) in [
            (
                "inbox.messaging.MarkDelivered",
                format!(r#"{{"message_id":"{id}"}}"#),
            ),
            (
                "inbox.messaging.MarkRead",
                format!(r#"{{"message_id":"{id}"}}"#),
            ),
            (
                "inbox.messaging.Refuse",
                format!(r#"{{"message_id":"{id}","reason":"x"}}"#),
            ),
            (
                "inbox.messaging.Collect",
                format!(r#"{{"message_id":"{id}"}}"#),
            ),
        ] {
            let (answered, events) = call(
                &world.backend,
                Actor::Client,
                as_client(THIRD),
                name,
                &input,
            );
            let value = ok(answered, name);
            assert_eq!(events, 0, "{name} {id} handed {THIRD} events: {value:?}");
            assert!(
                !carries(&value, &secrets),
                "{name} {id} answered {THIRD} content of the message: {value:?}"
            );
        }
    }
}

fn client() -> Caller {
    Caller {
        actor: Actor::Client,
    }
}

fn answer_of(rows: &str) -> Value {
    json::parse(&format!(r#"{{"rows":{rows}}}"#)).expect("the case writes JSON")
}

fn kept(caller: &CallerAttributes, rows: &str) -> Vec<Value> {
    rows_of(
        &inbox_core::readable(Some(&client()), caller, answer_of(rows)),
        "readable",
    )
}

#[test]
fn readable_drops_a_row_that_names_another_party_in_one_member_only() {
    let third = as_client(THIRD);
    assert!(
        kept(&third, &format!(r#"[{{"from":"{PLANNER}"}}]"#)).is_empty(),
        "a row with `from` of another and no `to` is kept"
    );
    assert!(
        kept(&third, &format!(r#"[{{"to":"{WORKER}"}}]"#)).is_empty(),
        "a row with `to` of another and no `from` is kept"
    );
    assert_eq!(
        kept(&third, &format!(r#"[{{"from":"{THIRD}"}}]"#)).len(),
        1,
        "a row the caller sent, with no `to`, is dropped"
    );
}

#[test]
fn readable_compares_text_exactly_and_never_a_value_that_is_not_text() {
    let upper = CallerAttributes::new(
        Principal(PLANNER.to_uppercase()),
        Address(WORKER.to_uppercase()),
    );
    let row = format!(r#"[{{"to":"{WORKER}","from":"{PLANNER}"}}]"#);
    assert!(kept(&upper, &row).is_empty(), "case folded");
    let composed = CallerAttributes::new(
        Principal("agent:ws/pl\u{e4}nner".to_owned()),
        Address("agent:box/w\u{f6}rker".to_owned()),
    );
    let decomposed = "[{\"to\":\"agent:box/wo\u{308}rker\",\"from\":\"agent:ws/pla\u{308}nner\"}]";
    assert!(
        kept(&composed, decomposed).is_empty(),
        "Unicode forms folded"
    );
    let empty = CallerAttributes::new(Principal(String::new()), Address(String::new()));
    assert!(kept(&empty, &row).is_empty(), "an empty attribute matched");
    let inbox_as_principal = CallerAttributes {
        principal: Some(Principal(WORKER.to_owned())),
        inbox: None,
    };
    assert!(
        kept(
            &inbox_as_principal,
            &format!(r#"[{{"to":"{WORKER}","from":"{PLANNER}"}}]"#)
        )
        .is_empty(),
        "the principal was compared with `to`"
    );
    let principal_as_inbox = CallerAttributes {
        principal: None,
        inbox: Some(Address(PLANNER.to_owned())),
    };
    assert!(
        kept(
            &principal_as_inbox,
            &format!(r#"[{{"to":"{WORKER}","from":"{PLANNER}"}}]"#)
        )
        .is_empty(),
        "the inbox was compared with `from`"
    );
    let worker = as_client(WORKER);
    for not_text in [
        format!(r#"[{{"to":["{WORKER}"],"from":"{PLANNER}"}}]"#),
        format!(r#"[{{"to":{{"0":"{WORKER}"}},"from":"{PLANNER}"}}]"#),
        format!(r#"[{{"to":null,"from":"{PLANNER}"}}]"#),
    ] {
        assert!(kept(&worker, &not_text).is_empty(), "{not_text} was kept");
    }
}

#[test]
fn readable_binds_every_caller_but_the_sweeper_and_keeps_the_answer_in_order() {
    let rows = format!(r#"[{{"to":"{WORKER}","from":"{PLANNER}"}}]"#);
    let whole = inbox_core::readable(
        Some(&Caller {
            actor: Actor::Sweeper,
        }),
        &CallerAttributes::none(),
        answer_of(&rows),
    );
    assert_eq!(rows_of(&whole, "Sweeper").len(), 1);
    let none = inbox_core::readable(None, &CallerAttributes::none(), answer_of(&rows));
    assert!(rows_of(&none, "None").is_empty(), "no caller read the row");
    let ordered = json::parse(&format!(
        r#"{{"a":1,"rows":[{{"to":"{WORKER}"}},{{"to":"{THIRD}"}}],"b":[2]}}"#
    ))
    .expect("the case writes JSON");
    let held = inbox_core::readable(Some(&client()), &as_client(THIRD), ordered);
    let expected = json::parse(&format!(r#"{{"a":1,"rows":[{{"to":"{THIRD}"}}],"b":[2]}}"#))
        .expect("the case writes JSON");
    assert_eq!(held, expected);
    let command = json::parse(r#"{"outcome":"posted","rows":"x"}"#).expect("JSON");
    assert_eq!(
        inbox_core::readable(Some(&client()), &as_client(THIRD), command.clone()),
        command
    );
}
