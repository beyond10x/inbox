//! Adversary pass 1 on story:collect-returns-the-settled-record against the core: the
//! hand-written `CollectBehavior` held to the outcomes `ess/domains/messaging.yaml` orders, and
//! the record held member by member to the row the `Outbox` view showed just before the delete.

use std::sync::Barrier;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{MessageId, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, ServiceContext};

use crate::support::{
    KeyIdVerifier, Post, Reply, World, answer, open_worker, outcome, post, quoted, text,
};

const PLANNER: &str = "agent:ws/planner";
const WORKER: &str = "agent:box/worker";
const BOSS: &str = "agent:box/boss";
const REQUEST: &str = "01JGT0AC000000000000000001";
const DECISION: &str = "01JGT0AC000000000000000002";
const REPLY: &str = "01JGT0AC000000000000000050";
const COLLECT: &str = "inbox.messaging.Collect";

/// One request through `inbox_core::handle` as a `Client` carrying exactly `caller`.
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

fn collect_input(id: &str) -> String {
    format!("{{\"message_id\":{}}}", quoted(id))
}

fn held(world: &World, id: &str) -> bool {
    MessageStorage::get(&world.backend, &MessageId(id.to_owned())).is_some()
}

/// The model's `foreign-message` is `all(to != caller.inbox, from != caller.principal)` in
/// three-valued logic, and `not-sender` is `from != caller.principal`. For the sender of the
/// message whose caller carries its `principal` and no `inbox`, the first is `all(Unknown,
/// false)` = false and the second is false, so the model orders `collected` on a settled row and
/// `not-settled` on one that is not. The generated `Collect` of the base commit decided exactly
/// so (`decided(all(&[..]))` in `generated/rust/inbox/src/behaviour.rs` at 42c7eb1); the
/// hand-written one refuses both as no declared outcome.
#[test]
fn a_sender_whose_caller_carries_no_inbox_gets_the_outcome_the_model_orders() {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    assert_eq!(outcome(&post(&world, &Post::request(REQUEST, 3))), "posted");
    let sender_only = || CallerAttributes {
        principal: Some(Principal(PLANNER.to_owned())),
        inbox: None,
    };

    let early = run_as(&world, sender_only(), COLLECT, &collect_input(REQUEST));
    let early = early.map(|answer| outcome(&answer).to_owned());
    assert_eq!(
        early.as_deref().map_err(ToString::to_string),
        Ok("not-settled"),
        "the model orders `not-settled` for the sender of an unsettled message"
    );

    assert_eq!(
        outcome(&answer(&world, &Reply::to(REQUEST, REPLY))),
        "answered"
    );
    let settled = run_as(&world, sender_only(), COLLECT, &collect_input(REQUEST));
    let settled = settled.map(|answer| outcome(&answer).to_owned());
    assert_eq!(
        settled.as_deref().map_err(ToString::to_string),
        Ok("collected"),
        "the model orders `collected` for the sender of a settled message"
    );
}

/// A reply spoken for another principal under a delegation, naming an option, with a body of
/// quotes, a backslash, newlines, U+2028 and 4-byte characters: the record's `reply` is the
/// `reply` member the `Outbox` row showed the moment before, member for member, and so are the
/// row's `to`, `thread`, `state`, `answered_by` and `decided_option`.
#[test]
fn the_record_is_the_outbox_row_of_the_moment_before_member_for_member() {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    assert_eq!(
        outcome(&post(&world, &Post::decision(DECISION, None))),
        "posted"
    );
    let reply = Reply {
        on_behalf_of: BOSS.to_owned(),
        authority: Some("dlg-boss-1".to_owned()),
        option: Some("A".to_owned()),
        body: "Ship \"it\" \\ now.\nSecond line\u{2028}third\r\nend \u{1F680}\u{10FFFF}".to_owned(),
        ..Reply::to(DECISION, REPLY)
    };
    assert_eq!(outcome(&answer(&world, &reply)), "answered");

    let planner = || {
        CallerAttributes::new(
            Principal(PLANNER.to_owned()),
            inbox::messaging::Address(PLANNER.to_owned()),
        )
    };
    let outbox = run_as(
        &world,
        planner(),
        "inbox.messaging.Outbox",
        &format!("{{\"sender\":{}}}", quoted(PLANNER)),
    )
    .expect("the sender reads its outbox");
    let row = match outbox.member("rows") {
        Some(Value::Array(rows)) => rows
            .iter()
            .find(|row| text(row, "message_id") == DECISION)
            .cloned()
            .expect("the outbox shows the decision"),
        other => panic!("no rows: {other:?}"),
    };

    let collected =
        run_as(&world, planner(), COLLECT, &collect_input(DECISION)).expect("the sender collects");
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    let record = collected.member("response").expect("a record");
    for member in [
        "message_id",
        "to",
        "thread",
        "state",
        "answered_by",
        "decided_option",
        "reply",
        "refusal",
    ] {
        assert_eq!(
            record.member(member),
            row.member(member),
            "`{member}` of the record differs from the outbox row"
        );
    }
    assert_eq!(text(record.member("reply").unwrap(), "body"), reply.body);
    assert!(!held(&world, DECISION));
}

/// A refusal whose reason is the empty string: the record carries `refusal` as `""`, not absent.
#[test]
fn an_empty_refusal_reason_is_in_the_record() {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    assert_eq!(outcome(&post(&world, &Post::request(REQUEST, 3))), "posted");
    let refused = world.client(
        WORKER,
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":{},\"reason\":\"\"}}", quoted(REQUEST)),
    );
    assert_eq!(outcome(&refused), "refused");
    let collected = world.client(PLANNER, COLLECT, &collect_input(REQUEST));
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    let record = collected.member("response").expect("a record");
    assert_eq!(record.member("refusal"), Some(&Value::Text(String::new())));
    assert_eq!(text(record, "state"), "Refused");
}

/// Two `Collect` of one settled message at once, each over its own clone of the backend, and an
/// `Answer` racing a third: exactly one record, it carries the reply, and a `Collect` that ran
/// before the answer removed nothing.
#[test]
fn collects_at_once_give_one_record_and_an_answer_racing_a_collect_is_never_lost() {
    for _ in 0..50 {
        let world = World::new(KeyIdVerifier::accepting());
        open_worker(&world);
        assert_eq!(outcome(&post(&world, &Post::request(REQUEST, 3))), "posted");
        assert_eq!(
            outcome(&answer(&world, &Reply::to(REQUEST, REPLY))),
            "answered"
        );
        let barrier = Barrier::new(2);
        let answers: Vec<Value> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..2)
                .map(|_| {
                    let own = World {
                        backend: world.backend.clone(),
                        verifier: world.verifier.clone(),
                    };
                    let barrier = &barrier;
                    scope.spawn(move || {
                        barrier.wait();
                        own.client(PLANNER, COLLECT, &collect_input(REQUEST))
                    })
                })
                .collect();
            threads.into_iter().map(|t| t.join().unwrap()).collect()
        });
        let records: Vec<&Value> = answers
            .iter()
            .filter_map(|a| a.member("response"))
            .collect();
        assert_eq!(records.len(), 1, "{answers:?}");
        let mut outcomes: Vec<&str> = answers.iter().map(outcome).collect();
        outcomes.sort_unstable();
        assert_eq!(outcomes, ["collected", "unknown-message"]);
    }

    for _ in 0..50 {
        let world = World::new(KeyIdVerifier::accepting());
        open_worker(&world);
        assert_eq!(outcome(&post(&world, &Post::request(REQUEST, 3))), "posted");
        let barrier = Barrier::new(2);
        let (answered, collected) = std::thread::scope(|scope| {
            let one = World {
                backend: world.backend.clone(),
                verifier: world.verifier.clone(),
            };
            let two = World {
                backend: world.backend.clone(),
                verifier: world.verifier.clone(),
            };
            let barrier = &barrier;
            let a = scope.spawn(move || {
                barrier.wait();
                answer(&one, &Reply::to(REQUEST, REPLY))
            });
            let c = scope.spawn(move || {
                barrier.wait();
                two.client(PLANNER, COLLECT, &collect_input(REQUEST))
            });
            (a.join().unwrap(), c.join().unwrap())
        });
        assert_eq!(outcome(&answered), "answered");
        match outcome(&collected) {
            "collected" => {
                let record = collected.member("response").expect("a record");
                assert_eq!(
                    text(record.member("reply").expect("the reply"), "reply_id"),
                    REPLY
                );
                assert!(!held(&world, REQUEST));
            }
            "not-settled" => assert!(held(&world, REQUEST), "the answered message stays"),
            other => panic!("{other}: {collected:?}"),
        }
    }
}
