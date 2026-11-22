//! story:the-core-answers-a-foreign-message-as-unknown: a client that is no party of a message
//! gets from `inbox_core::handle` and `handle_with_events` the answer of an id no message carries,
//! byte for byte, for each of the 5 commands that address a message, in every state the message
//! is in, and no event.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{ExternalCommand, TryContext, unmet_context};
use inbox::messaging::{Address, PostMessage, Principal};
use inbox::obligation::UnmetObligation;
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, MemoryBackend, PostExternals, ServiceContext};

use crate::adversary_read_rule::{
    ANSWERED, FAN_B, IDS, LEAD, PLANNER, SECOND, SENT, THIRD, WORKER, as_client, call, ok, world,
};
use crate::support::{KeyIdVerifier, Reply, World};

const MISSING_ID: &str = "01JGT0AR000000000000000098";

const COMMANDS: [&str; 5] = [
    "inbox.messaging.MarkDelivered",
    "inbox.messaging.MarkRead",
    "inbox.messaging.Answer",
    "inbox.messaging.Refuse",
    "inbox.messaging.Collect",
];

/// What the generated writer answers for an id no message carries.
const UNKNOWN: &str =
    r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#;

/// The input of `name` for message `id` as `who` writes it: an answer is a reply from `who`, so
/// the input guard of `Answer` (`not-sender`) does not answer first.
fn input(name: &str, id: &str, who: &str) -> String {
    match name {
        "inbox.messaging.Answer" => Reply {
            from: who.to_owned(),
            on_behalf_of: who.to_owned(),
            ..Reply::to(id, "01JGT0AR000000000000000097")
        }
        .json(),
        "inbox.messaging.Refuse" => format!(r#"{{"message_id":"{id}","reason":"x"}}"#),
        _ => format!(r#"{{"message_id":"{id}"}}"#),
    }
}

/// `name` as the client `who` through `inbox_core::handle`.
fn handled(world: &World, who: &str, name: &str, input: &str) -> Result<Value, Refused> {
    let context = ServiceContext::new(
        as_client(who),
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

#[test]
fn a_third_principal_cannot_tell_a_message_between_others_from_none_through_handle() {
    // The model's own summary of `foreign-message`: "so from this command a third principal
    // cannot tell a message between others from no message". The HTTP service renames the
    // outcome (`as_unknown` in crates/inbox-server/src/service.rs); an embedding that hands the
    // answer of `inbox_core::handle` on, as the story's finding has it, does not.
    let world = world();
    const MISSING: &str = "01JGT0AR000000000000000098";
    for name in [
        "inbox.messaging.MarkDelivered",
        "inbox.messaging.MarkRead",
        "inbox.messaging.Collect",
    ] {
        let answer = |id: &str| {
            let (answered, _) = call(
                &world.backend,
                Actor::Client,
                as_client(THIRD),
                name,
                &format!(r#"{{"message_id":"{id}"}}"#),
            );
            ok(answered, name)
        };
        assert_eq!(
            answer(SENT),
            answer(MISSING),
            "{name}: {THIRD} tells the message {SENT} between others from no message"
        );
    }
}

#[test]
fn each_command_answers_a_client_that_is_no_party_as_a_missing_id_in_every_state() {
    // The principal a delegate speaks for (LEAD) is no party either: `on_behalf_of` names none.
    let world = world();
    let unknown = json::parse(UNKNOWN).expect("the case writes JSON");
    let store = || world.sweeper_rows("inbox.messaging.Messages", "{}");
    let before = store();
    for who in [THIRD, LEAD] {
        for name in COMMANDS {
            let (missing, missing_events) = call(
                &world.backend,
                Actor::Client,
                as_client(who),
                name,
                &input(name, MISSING_ID, who),
            );
            assert_eq!(missing, Ok(unknown.clone()), "{name} as {who}, no message");
            assert_eq!(missing_events, 0, "{name} as {who}, no message");
            for id in IDS {
                let (answered, events) = call(
                    &world.backend,
                    Actor::Client,
                    as_client(who),
                    name,
                    &input(name, id, who),
                );
                assert_eq!(
                    answered, missing,
                    "{name} as {who}: {id} answers otherwise than no message"
                );
                assert_eq!(events, 0, "{name} as {who}: {id} handed back events");
                assert_eq!(
                    handled(&world, who, name, &input(name, id, who)),
                    missing,
                    "{name} as {who} through handle: {id} answers otherwise than no message"
                );
            }
        }
    }
    assert_eq!(store(), before, "a client that is no party moved the store");
}

/// `name` as the client `who` through the conformance runner's entry
/// `inbox_core::handle_as_specified`: the answer and the count of events handed back.
fn specified(world: &World, who: &str, name: &str, input: &str) -> (Result<Value, Refused>, usize) {
    let context = ServiceContext::new(
        as_client(who),
        KeyIdVerifier::accepting(),
        world.backend.clone(),
        crate::support::clock(),
    );
    let (answered, events) = inbox_core::handle_as_specified(
        &world.backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        name,
        json::parse(input).expect("the case writes JSON"),
    );
    (answered, events.len())
}

#[test]
fn the_conformance_entry_answers_the_models_foreign_message_and_handle_its_rename() {
    let world = world();
    for name in COMMANDS {
        for id in IDS {
            let input = input(name, id, THIRD);
            let (model, events) = specified(&world, THIRD, name, &input);
            let model = ok(model, name);
            assert_eq!(
                model.member("outcome"),
                Some(&Value::Text("foreign-message".to_owned())),
                "{name} {id}: {model:?}"
            );
            assert_eq!(events, 0, "{name} {id} handed back events");
            assert_eq!(
                handled(&world, THIRD, name, &input),
                Ok(inbox_core::as_unknown(model)),
                "{name} {id}"
            );
        }
    }
}

#[test]
fn the_parties_get_from_handle_what_the_model_answers() {
    // The same requests of the parties, in the same order, over two stores built alike: one
    // through `handle_with_events`, one through the entry without the rename. A party never
    // gets `foreign-message`, so the two answer alike, step by step, and leave the stores alike.
    let (renamed, model) = (world(), world());
    let mut outcomes: Vec<Value> = Vec::new();
    for id in IDS.into_iter().chain([MISSING_ID]) {
        let parties: &[&str] = match id {
            FAN_B => &[PLANNER, SECOND],
            MISSING_ID => &[PLANNER, WORKER, SECOND],
            _ => &[PLANNER, WORKER],
        };
        for name in COMMANDS {
            for &who in parties {
                let input = input(name, id, who);
                let through_handle = call(
                    &renamed.backend,
                    Actor::Client,
                    as_client(who),
                    name,
                    &input,
                );
                let through_model = specified(&model, who, name, &input);
                assert_eq!(through_handle, through_model, "{name} {id} as {who}");
                let outcome = ok(through_model.0, name)
                    .member("outcome")
                    .cloned()
                    .unwrap_or(Value::Null);
                assert_ne!(
                    outcome,
                    Value::Text("foreign-message".to_owned()),
                    "{name} {id}: {who} is a party"
                );
                outcomes.push(outcome);
            }
        }
    }
    assert_eq!(outcomes.len(), 105);
    for accepted in ["delivered", "read", "answered", "collected"] {
        assert!(
            outcomes.contains(&Value::Text(accepted.to_owned())),
            "no party was answered `{accepted}`: {outcomes:?}"
        );
    }
    let messages = |world: &World| world.sweeper_rows("inbox.messaging.Messages", "{}");
    assert_eq!(messages(&renamed), messages(&model));
}

/// The context of the third principal whose read of the inbox, of the principal or of both errs:
/// an embedding whose source of the caller's attributes failed.
struct Unanswering {
    inner: ServiceContext<KeyIdVerifier, MemoryBackend, inbox::primitives::Timestamp>,
    inbox: bool,
    principal: bool,
}

impl TryContext for Unanswering {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        match self.inbox {
            true => Err(unmet_context("caller inbox")),
            false => self.inner.try_caller_inbox(),
        }
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        match self.principal {
            true => Err(unmet_context("caller principal")),
            false => self.inner.try_caller_principal(),
        }
    }

    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        self.inner.try_external(command, outcome)
    }
}

impl PostExternals for Unanswering {
    fn expired_on_arrival(&mut self, input: &PostMessage) -> Result<bool, UnmetObligation> {
        self.inner.expired_on_arrival(input)
    }

    fn signature_invalid(&mut self, input: &PostMessage) -> bool {
        self.inner.signature_invalid(input)
    }

    fn authority_invalid(&mut self, input: &PostMessage) -> bool {
        self.inner.authority_invalid(input)
    }
}

#[test]
fn a_client_whose_context_cannot_read_an_attribute_gets_the_answer_of_no_message() {
    // `handle` reads an attribute the context cannot answer as none: the same rule as a context
    // that answers it as none (`adversary_foreign_unknown`), here for a read that errs.
    let world = world();
    let unknown = json::parse(UNKNOWN).expect("the case writes JSON");
    let store = || world.sweeper_rows("inbox.messaging.Messages", "{}");
    let before = store();
    let client = Caller {
        actor: Actor::Client,
    };
    for (inbox, principal) in [(true, false), (false, true), (true, true)] {
        let context = || Unanswering {
            inner: ServiceContext::new(
                as_client(THIRD),
                KeyIdVerifier::accepting(),
                world.backend.clone(),
                crate::support::clock(),
            ),
            inbox,
            principal,
        };
        for name in COMMANDS {
            for id in IDS.into_iter().chain([MISSING_ID]) {
                let request = || json::parse(&input(name, id, THIRD)).expect("JSON");
                let what = format!("{name} {id}, inbox errs {inbox}, principal errs {principal}");
                let (answered, events) = inbox_core::handle_with_events(
                    &world.backend,
                    context(),
                    Some(&client),
                    name,
                    request(),
                );
                assert_eq!(answered, Ok(unknown.clone()), "{what}");
                assert!(events.is_empty(), "{what}: {events:?}");
                assert_eq!(
                    inbox_core::handle(&world.backend, context(), Some(&client), name, request()),
                    Ok(unknown.clone()),
                    "{what} through handle"
                );
            }
        }
    }
    assert_eq!(
        store(),
        before,
        "a client without its attributes moved the store"
    );
}

#[test]
fn a_sender_without_an_inbox_gets_the_models_answer_where_it_decides_and_no_message_elsewhere() {
    // The rule of `handle` for a party known by one attribute: `Collect` needs only the
    // principal of the sender; `MarkDelivered` needs the inbox of the recipient, which the model
    // cannot compare, and the conformance entry hands on the model's unmet obligation.
    let world = world();
    let sender = CallerAttributes {
        principal: Some(Principal(PLANNER.to_owned())),
        inbox: None,
    };
    let context = || {
        ServiceContext::new(
            sender.clone(),
            KeyIdVerifier::accepting(),
            world.backend.clone(),
            crate::support::clock(),
        )
    };
    let client = Caller {
        actor: Actor::Client,
    };
    let request = |id: &str| json::parse(&format!(r#"{{"message_id":"{id}"}}"#)).expect("JSON");
    let delivered = "inbox.messaging.MarkDelivered";
    let (model, _) = inbox_core::handle_as_specified(
        &world.backend,
        context(),
        Some(&client),
        delivered,
        request(SENT),
    );
    assert!(matches!(model, Err(Refused::Unmet(_))), "{model:?}");
    let (answered, events) = call(
        &world.backend,
        Actor::Client,
        sender.clone(),
        delivered,
        &input(delivered, SENT, PLANNER),
    );
    assert_eq!(answered, Ok(json::parse(UNKNOWN).expect("JSON")));
    assert_eq!(events, 0);
    let collected = inbox_core::handle(
        &world.backend,
        context(),
        Some(&client),
        "inbox.messaging.Collect",
        request(ANSWERED),
    );
    let collected = ok(collected, "Collect");
    assert_eq!(
        collected.member("outcome"),
        Some(&Value::Text("collected".to_owned())),
        "{collected:?}"
    );
}

#[test]
fn as_unknown_renames_only_the_outcome_foreign_message_in_place() {
    let parsed = |text: &str| json::parse(text).expect("the case writes JSON");
    let foreign = parsed(
        r#"{"published":[],"outcome":"foreign-message","error":"inbox.messaging.UnknownMessage"}"#,
    );
    assert_eq!(
        inbox_core::as_unknown(foreign),
        parsed(
            r#"{"published":[],"outcome":"unknown-message","error":"inbox.messaging.UnknownMessage"}"#
        ),
        "the members keep their order"
    );
    for kept in [
        r#"{"outcome":"not-recipient","published":[],"error":"inbox.messaging.NotRecipient"}"#,
        r#"{"outcome":"Foreign-message"}"#,
        r#"{"outcome":["foreign-message"]}"#,
        r#"{"error":"foreign-message"}"#,
        r#"{"rows":[{"outcome":"foreign-message"}]}"#,
        r#""foreign-message""#,
        r#"[{"outcome":"foreign-message"}]"#,
    ] {
        assert_eq!(inbox_core::as_unknown(parsed(kept)), parsed(kept), "{kept}");
    }
}
