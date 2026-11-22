//! Adversary pass on story:the-core-answers-a-foreign-message-as-unknown: whether a client that
//! is no party of a message can tell it from an id no message carries through
//! `inbox_core::handle` and `handle_with_events` when its attributes are not the two equal ones
//! every case of the unit builds (`as_client`): one attribute absent, the two attributes naming
//! different principals, and after the message is removed.

use inbox::actor::{Actor, Caller};
use inbox::messaging::{Address, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, ServiceContext};

use crate::adversary_read_rule::{ANSWERED, IDS, PLANNER, THIRD, WORKER, call, world};
use crate::support::{KeyIdVerifier, Reply, World};

const MISSING_ID: &str = "01JGT0AR000000000000000096";

const COMMANDS: [&str; 5] = [
    "inbox.messaging.MarkDelivered",
    "inbox.messaging.MarkRead",
    "inbox.messaging.Answer",
    "inbox.messaging.Refuse",
    "inbox.messaging.Collect",
];

const UNKNOWN: &str =
    r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#;

/// The input of `name` for message `id`, an answer written as from `from`.
fn input(name: &str, id: &str, from: &str) -> String {
    match name {
        "inbox.messaging.Answer" => Reply {
            from: from.to_owned(),
            on_behalf_of: from.to_owned(),
            ..Reply::to(id, "01JGT0AR000000000000000095")
        }
        .json(),
        "inbox.messaging.Refuse" => format!(r#"{{"message_id":"{id}","reason":"x"}}"#),
        _ => format!(r#"{{"message_id":"{id}"}}"#),
    }
}

/// `name` as a client with `attributes` through `inbox_core::handle`.
fn handled(
    world: &World,
    attributes: &CallerAttributes,
    name: &str,
    input: &str,
) -> Result<Value, Refused> {
    let context = ServiceContext::new(
        attributes.clone(),
        KeyIdVerifier::accepting(),
        world.backend.clone(),
        crate::support::clock(),
    );
    inbox_core::handle(
        &world.backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
}

/// Every request of the 5 commands for each of the 9 messages that `attributes` tells from the
/// same request for an id no message carries, through either entry, as one line each.
fn told_apart(world: &World, attributes: &CallerAttributes, from: &str) -> Vec<String> {
    let mut told = Vec::new();
    for name in COMMANDS {
        let missing = call(
            &world.backend,
            Actor::Client,
            attributes.clone(),
            name,
            &input(name, MISSING_ID, from),
        );
        for id in IDS {
            let request = input(name, id, from);
            let answered = call(
                &world.backend,
                Actor::Client,
                attributes.clone(),
                name,
                &request,
            );
            if answered != missing {
                told.push(format!(
                    "{name} {id} through handle_with_events: {answered:?}, no message: {missing:?}"
                ));
            }
            let through_handle = handled(world, attributes, name, &request);
            if through_handle != missing.0 {
                told.push(format!(
                    "{name} {id} through handle: {through_handle:?}, no message: {:?}",
                    missing.0
                ));
            }
        }
    }
    told
}

#[test]
fn adv_a_client_with_one_attribute_absent_cannot_tell_a_message_between_others_from_none() {
    // `handle` documents a caller whose context cannot answer an attribute ("an attribute it
    // cannot answer is none"), and the read rule of the same entry is held for such clients
    // (`readable_compares_text_exactly_and_never_a_value_that_is_not_text`). A missing id is
    // answered `unknown-message` before any attribute is read; for a stored message the guard of
    // `foreign-message` reads both attributes, an absent one is Unknown, Unknown selects no
    // branch, and the generated behaviour refuses the request as an unmet obligation.
    let world = world();
    let shapes = [
        (
            "a principal without an inbox",
            CallerAttributes {
                principal: Some(Principal(THIRD.to_owned())),
                inbox: None,
            },
        ),
        (
            "an inbox without a principal",
            CallerAttributes {
                principal: None,
                inbox: Some(Address(THIRD.to_owned())),
            },
        ),
        ("no attribute", CallerAttributes::none()),
    ];
    let mut told = Vec::new();
    for (shape, attributes) in &shapes {
        for line in told_apart(&world, attributes, THIRD) {
            told.push(format!("{shape}: {line}"));
        }
    }
    assert!(
        told.is_empty(),
        "{} requests tell a message between others from no message, first: {}",
        told.len(),
        told.first().map(String::as_str).unwrap_or_default()
    );
}

#[test]
fn adv_attributes_that_name_two_parties_crosswise_are_answered_as_no_message() {
    // The principal of the recipient with the inbox of the sender is neither party of a message
    // from PLANNER to WORKER: `to` is not its inbox and `from` is not its principal.
    let world = world();
    let crosswise = CallerAttributes {
        principal: Some(Principal(WORKER.to_owned())),
        inbox: Some(Address(PLANNER.to_owned())),
    };
    let elsewhere = CallerAttributes {
        principal: Some(Principal(THIRD.to_owned())),
        inbox: Some(Address("agent:box/elsewhere".to_owned())),
    };
    let unknown = json::parse(UNKNOWN).expect("the case writes JSON");
    for (attributes, from) in [(&crosswise, WORKER), (&elsewhere, THIRD)] {
        let told = told_apart(&world, attributes, from);
        assert!(told.is_empty(), "{attributes:?}: {told:?}");
        for name in COMMANDS {
            assert_eq!(
                handled(&world, attributes, name, &input(name, MISSING_ID, from)),
                Ok(unknown.clone()),
                "{name} {attributes:?}"
            );
        }
    }
}

#[test]
fn adv_after_removal_the_former_parties_and_a_third_get_the_answer_of_no_message() {
    let world = world();
    let collected = world.client(
        PLANNER,
        "inbox.messaging.Collect",
        &format!(r#"{{"message_id":"{ANSWERED}"}}"#),
    );
    assert_eq!(
        collected.member("outcome"),
        Some(&Value::Text("collected".to_owned())),
        "{collected:?}"
    );
    let unknown = json::parse(UNKNOWN).expect("the case writes JSON");
    for who in [THIRD, PLANNER, WORKER] {
        let attributes = CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()));
        for name in COMMANDS {
            let (answered, events) = call(
                &world.backend,
                Actor::Client,
                attributes.clone(),
                name,
                &input(name, ANSWERED, who),
            );
            assert_eq!(answered, Ok(unknown.clone()), "{name} as {who}");
            assert_eq!(events, 0, "{name} as {who}");
        }
    }
}
