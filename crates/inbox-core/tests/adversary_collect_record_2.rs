//! Adversary pass 2 on story:collect-returns-the-settled-record against the core: the guards of
//! the hand-written `CollectBehavior` held, for every mix of present and absent caller attributes
//! and every state, to the outcome the generated `Collect` of the base decided; and the record
//! held member for member to the sender's `Outbox` row of the moment before for every way a
//! message gets settled.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, MessageId, MessageState, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, ServiceContext};

use crate::support::{
    KeyIdVerifier, Post, Reply, World, answer, open_worker, outcome, post, quoted, text,
};

const PLANNER: &str = "agent:ws/planner";
const WORKER: &str = "agent:box/worker";
const OTHER: &str = "agent:box/other";
const MESSAGE: &str = "01JGT0AD000000000000000001";
const REPLY: &str = "01JGT0AD000000000000000050";
const COLLECT: &str = "inbox.messaging.Collect";

fn run_as(
    world: &World,
    caller: CallerAttributes,
    name: &str,
    input: &str,
) -> Result<Value, Refused> {
    let context = ServiceContext::new(
        caller,
        world.verifier.clone(),
        world.backend.clone(),
        crate::support::clock(),
    );
    let input = json::parse(input).expect("the case writes JSON");
    inbox_core::handle(
        &world.backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        name,
        input,
    )
}

fn held(world: &World, id: &str) -> bool {
    MessageStorage::get(&world.backend, &MessageId(id.to_owned())).is_some()
}

/// A request of the planner to the worker, stored in `state`.
fn stored_in(state: MessageState) -> World {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    assert_eq!(outcome(&post(&world, &Post::request(MESSAGE, 3))), "posted");
    let mut snapshot = MessageStorage::get(&world.backend, &MessageId(MESSAGE.to_owned()))
        .expect("the posted request is stored");
    snapshot.state = state;
    let mut backend = world.backend.clone();
    MessageStorage::put(&mut backend, snapshot);
    world
}

/// What the generated `Collect` of the base decided for a request from `PLANNER` to `WORKER`,
/// written out by hand from its three-valued guards (`decided(all(&[to != inbox, from !=
/// principal]))`, then `decided(from != principal)`, then the state): `unmet` where a guard
/// stays Unknown, `settled` where the state decides between `collected` and `not-settled`.
const EXPECTED: [(Option<&str>, Option<&str>, &str); 9] = [
    (None, None, "unmet"),
    (None, Some(PLANNER), "settled"),
    (None, Some(OTHER), "unmet"),
    (Some(WORKER), None, "unmet"),
    (Some(WORKER), Some(PLANNER), "settled"),
    (Some(WORKER), Some(OTHER), "not-sender"),
    (Some(OTHER), None, "unmet"),
    (Some(OTHER), Some(PLANNER), "settled"),
    (Some(OTHER), Some(OTHER), "foreign-message"),
];

/// Every mix of an absent, the matching and another `inbox` and `principal`, in each of the 5
/// states: the outcome is the one the base's generated `Collect` decided, a guard left Unknown
/// is an unmet obligation, only `collected` carries a record, and only `collected` removes the
/// row.
#[test]
fn every_mix_of_caller_attributes_in_every_state_gets_the_outcome_of_the_base() {
    let states = [
        (MessageState::Sent, false),
        (MessageState::Delivered, false),
        (MessageState::Read, false),
        (MessageState::Answered, true),
        (MessageState::Refused, true),
    ];
    let mut wrong = Vec::new();
    for (state, settled) in states {
        for (inbox, principal, expected) in EXPECTED {
            let world = stored_in(state);
            let caller = CallerAttributes {
                principal: principal.map(|who| Principal(who.to_owned())),
                inbox: inbox.map(|who| Address(who.to_owned())),
            };
            let expected = match expected {
                "settled" if settled => "collected",
                "settled" => "not-settled",
                other => other,
            };
            let answered = run_as(
                &world,
                caller,
                COLLECT,
                &format!("{{\"message_id\":{}}}", quoted(MESSAGE)),
            );
            let got = match &answered {
                Ok(answer) => outcome(answer).to_owned(),
                Err(Refused::Unmet(_)) => "unmet".to_owned(),
                Err(other) => format!("refused: {other}"),
            };
            let record = answered
                .as_ref()
                .ok()
                .and_then(|answer| answer.member("response"))
                .is_some();
            let kept = held(&world, MESSAGE);
            let collected = expected == "collected";
            if got != expected || record != collected || kept == collected {
                wrong.push(format!(
                    "{state:?}, inbox {inbox:?}, principal {principal:?}: {got} (record {record}, \
                     row kept {kept}); the base: {expected}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// One way of settling the message `MESSAGE`, applied to a fresh world.
struct Settlement {
    name: &'static str,
    post: Post,
    settle: fn(&World),
}

fn refuse(world: &World, reason: &str) {
    let refused = world.client(
        WORKER,
        "inbox.messaging.Refuse",
        &format!(
            "{{\"message_id\":{},\"reason\":{}}}",
            quoted(MESSAGE),
            quoted(reason)
        ),
    );
    assert_eq!(outcome(&refused), "refused");
}

fn settlements() -> Vec<Settlement> {
    let delegated = Post {
        on_behalf_of: "agent:ws/lead".to_owned(),
        authority: Some("dlg-lead-1".to_owned()),
        reply_shape: Some("{\"shape\":\"free\",\"value\":{}}".to_owned()),
        body: "Line one\nLine \"two\" \\ \u{7f}\u{1F680}".to_owned(),
        ..Post::request(MESSAGE, 3)
    };
    vec![
        Settlement {
            name: "a request refused with a reason of two lines",
            post: Post::request(MESSAGE, 3),
            settle: |world| refuse(world, "Not here.\nAsk the release lead."),
        },
        Settlement {
            name: "a decision with a default taken by silence",
            post: Post::decision(MESSAGE, Some("A")),
            settle: |world| {
                let decided = world.sweeper(
                    "inbox.messaging.Expire",
                    &format!(
                        "{{\"message_id\":{},\"now\":\"2026-01-06T09:00:00Z\",\
                         \"keep_until\":\"2026-01-07T09:00:00Z\"}}",
                        quoted(MESSAGE)
                    ),
                );
                assert_eq!(outcome(&decided), "decided-on-silence");
            },
        },
        Settlement {
            name: "a decision with a default refused",
            post: Post::decision(MESSAGE, Some("B")),
            settle: |world| refuse(world, "Neither."),
        },
        Settlement {
            name: "a decision without a default answered with an option",
            post: Post::decision(MESSAGE, None),
            settle: |world| {
                let reply = Reply {
                    option: Some("B".to_owned()),
                    ..Reply::to(MESSAGE, REPLY)
                };
                assert_eq!(outcome(&answer(world, &reply)), "answered");
            },
        },
        Settlement {
            name: "a delegated free request answered without delegation",
            post: delegated,
            settle: |world| {
                let reply = Reply {
                    body: "Done.\r\nSecond line".to_owned(),
                    ..Reply::to(MESSAGE, REPLY)
                };
                assert_eq!(outcome(&answer(world, &reply)), "answered");
            },
        },
    ]
}

fn names(value: &Value) -> Vec<String> {
    let Value::Object(members) = value else {
        panic!("not an object: {value:?}")
    };
    let mut names: Vec<String> = members.iter().map(|(name, _)| name.clone()).collect();
    names.sort();
    names
}

/// For each way a message is settled, the record holds exactly the members of the sender's
/// `Outbox` row read just before the collect, each with the same value; an optional member the
/// row leaves out is left out of the record, not written as `null`.
#[test]
fn the_record_is_the_outbox_row_for_every_settlement() {
    let mut wrong = Vec::new();
    for settlement in settlements() {
        let world = World::new(KeyIdVerifier::accepting());
        open_worker(&world);
        assert_eq!(
            outcome(&post(&world, &settlement.post)),
            "posted",
            "{}",
            settlement.name
        );
        (settlement.settle)(&world);
        let outbox = world.client(
            PLANNER,
            "inbox.messaging.Outbox",
            &format!("{{\"sender\":{}}}", quoted(PLANNER)),
        );
        let row = match outbox.member("rows") {
            Some(Value::Array(rows)) => rows
                .iter()
                .find(|row| text(row, "message_id") == MESSAGE)
                .cloned()
                .unwrap_or_else(|| panic!("{}: no outbox row", settlement.name)),
            other => panic!("{}: no rows: {other:?}", settlement.name),
        };
        let collected = world.client(
            PLANNER,
            COLLECT,
            &format!("{{\"message_id\":{}}}", quoted(MESSAGE)),
        );
        assert_eq!(outcome(&collected), "collected", "{}", settlement.name);
        let record = collected.member("response").expect("a record");
        if names(record) != names(&row) {
            wrong.push(format!(
                "{}: the record's members {:?}, the row's {:?}",
                settlement.name,
                names(record),
                names(&row)
            ));
        }
        for name in names(&row) {
            if record.member(&name) != row.member(&name) {
                wrong.push(format!(
                    "{}: `{name}` is {:?} in the record and {:?} in the row",
                    settlement.name,
                    record.member(&name),
                    row.member(&name)
                ));
            }
        }
        let Value::Object(members) = record else {
            unreachable!("names() held it to an object")
        };
        if let Some((name, _)) = members.iter().find(|(_, value)| *value == Value::Null) {
            wrong.push(format!("{}: `{name}` is null", settlement.name));
        }
        if held(&world, MESSAGE) {
            wrong.push(format!("{}: the row stays", settlement.name));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
