//! The read rule at the entry of an embedding: through `inbox_core::handle` a client reads only
//! the rows of the messages it is a party of, whatever parameter it sends, and the views of the
//! expiry task stay whole behind their actor (story:the-core-applies-the-read-rule).

use inbox::actor::{Actor, Caller};
use inbox::messaging::{Address, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, MemoryBackend, ServiceContext};

use crate::support::{KeyIdVerifier, Post, Reply, World, answer, open_worker, outcome, post, text};

const SENDER: &str = "agent:ws/planner";
const RECIPIENT: &str = "agent:box/worker";
const THIRD: &str = "agent:box/other";

/// Stays `Sent`: a row of `Pending`.
const WAITING: &str = "01JGT0RR000000000000000001";
/// Read and not answered: a row of `Unanswered`.
const READ: &str = "01JGT0RR000000000000000002";
/// Answered: a row of `Settled` and `Replies`, with its reply.
const ANSWERED: &str = "01JGT0RR000000000000000003";

/// Each of the 6 views of messages a client may read, with the parameter of the sender, the
/// recipient or a thread of theirs.
fn views() -> [(&'static str, String); 6] {
    [
        (
            "inbox.messaging.Pending",
            format!(r#"{{"inbox":"{RECIPIENT}"}}"#),
        ),
        (
            "inbox.messaging.Unanswered",
            format!(r#"{{"inbox":"{RECIPIENT}"}}"#),
        ),
        (
            "inbox.messaging.Outbox",
            format!(r#"{{"sender":"{SENDER}"}}"#),
        ),
        (
            "inbox.messaging.Settled",
            format!(r#"{{"sender":"{SENDER}"}}"#),
        ),
        (
            "inbox.messaging.Thread",
            format!(r#"{{"thread":"{ANSWERED}"}}"#),
        ),
        (
            "inbox.messaging.Replies",
            format!(r#"{{"thread":"{ANSWERED}"}}"#),
        ),
    ]
}

/// Three requests from the sender to the recipient: one waiting, one read, one answered.
fn world() -> World {
    let world = World::new(KeyIdVerifier::accepting());
    open_worker(&world);
    for id in [WAITING, READ, ANSWERED] {
        assert_eq!(outcome(&post(&world, &Post::request(id, 3))), "posted");
    }
    for id in [READ, ANSWERED] {
        let read = world.client(
            RECIPIENT,
            "inbox.messaging.MarkRead",
            &format!(r#"{{"message_id":"{id}"}}"#),
        );
        assert_eq!(outcome(&read), "read");
    }
    let answered = answer(&world, &Reply::to(ANSWERED, "01JGT0RR000000000000000004"));
    assert_eq!(outcome(&answered), "answered");
    world
}

fn read(
    backend: &MemoryBackend,
    actor: Actor,
    caller: CallerAttributes,
    view: &str,
    params: &str,
) -> Result<Value, Refused> {
    let context = ServiceContext::new(
        caller,
        KeyIdVerifier::accepting(),
        backend.clone(),
        crate::support::clock(),
    );
    let params = json::parse(params).expect("the case writes JSON");
    inbox_core::handle(backend, context, Some(&Caller { actor }), view, params)
}

fn client(who: &str) -> CallerAttributes {
    CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()))
}

fn rows(answer: Result<Value, Refused>) -> Vec<Value> {
    match answer {
        Ok(value) => match value.member("rows") {
            Some(Value::Array(rows)) => rows.clone(),
            other => panic!("a view answers rows, not {other:?}"),
        },
        Err(refused) => panic!("the view was refused: {refused}"),
    }
}

#[test]
fn a_third_principal_reads_no_row_of_a_message_between_two_others_through_handle() {
    let world = world();
    for (view, params) in views() {
        let third = rows(read(
            &world.backend,
            Actor::Client,
            client(THIRD),
            view,
            &params,
        ));
        assert!(
            third.is_empty(),
            "{view} {params} answered {THIRD} the rows of others: {third:?}"
        );
    }
}

#[test]
fn the_parties_of_a_message_read_its_rows_through_handle() {
    let world = world();
    let party = |view: &str| match view {
        "inbox.messaging.Pending" | "inbox.messaging.Unanswered" => vec![RECIPIENT],
        "inbox.messaging.Outbox" | "inbox.messaging.Settled" => vec![SENDER],
        _ => vec![SENDER, RECIPIENT],
    };
    for (view, params) in views() {
        for who in party(view) {
            let read = rows(read(
                &world.backend,
                Actor::Client,
                client(who),
                view,
                &params,
            ));
            assert!(!read.is_empty(), "{view} {params} answered {who} no row");
            for row in &read {
                assert!(
                    text(row, "from") == who || text(row, "to") == who,
                    "{view} answered {who} a row it is no party of: {row:?}"
                );
            }
        }
    }
}

#[test]
fn a_client_whose_principal_and_inbox_differ_reads_by_either() {
    // The owner of the inbox in `to` reads by its `inbox`, the sender by its `principal`: a
    // caller carrying the recipient's address and another principal reads the recipient's rows.
    let world = world();
    let caller = CallerAttributes::new(Principal(THIRD.to_owned()), Address(RECIPIENT.to_owned()));
    let pending = rows(read(
        &world.backend,
        Actor::Client,
        caller.clone(),
        "inbox.messaging.Pending",
        &format!(r#"{{"inbox":"{RECIPIENT}"}}"#),
    ));
    assert_eq!(pending.len(), 1, "{pending:?}");
    let outbox = rows(read(
        &world.backend,
        Actor::Client,
        caller,
        "inbox.messaging.Outbox",
        &format!(r#"{{"sender":"{SENDER}"}}"#),
    ));
    assert_eq!(outbox.len(), 3, "the recipient's rows by `to`: {outbox:?}");
    let sender = CallerAttributes::new(Principal(SENDER.to_owned()), Address(THIRD.to_owned()));
    let thread = rows(read(
        &world.backend,
        Actor::Client,
        sender,
        "inbox.messaging.Thread",
        &format!(r#"{{"thread":"{WAITING}"}}"#),
    ));
    assert_eq!(thread.len(), 1, "the sender's row by `from`: {thread:?}");
}

#[test]
fn a_client_without_attributes_reads_no_row_of_a_message() {
    let world = world();
    for (view, params) in views() {
        let read = rows(read(
            &world.backend,
            Actor::Client,
            CallerAttributes::none(),
            view,
            &params,
        ));
        assert!(read.is_empty(), "{view}: {read:?}");
    }
    let inboxes = rows(read(
        &world.backend,
        Actor::Client,
        CallerAttributes::none(),
        "inbox.messaging.Inboxes",
        "{}",
    ));
    assert_eq!(inboxes.len(), 1, "`Inboxes` is answered whole: {inboxes:?}");
}

#[test]
fn due_and_messages_stay_whole_for_the_sweeper_and_refused_to_a_client() {
    let world = world();
    let due = r#"{"now":"2026-01-07T00:00:00Z"}"#;
    for (view, params) in [
        ("inbox.messaging.Due", due),
        ("inbox.messaging.Messages", "{}"),
    ] {
        let whole = rows(read(
            &world.backend,
            Actor::Sweeper,
            CallerAttributes::none(),
            view,
            params,
        ));
        assert_eq!(whole.len(), 3, "{view}: {whole:?}");
        for who in [SENDER, RECIPIENT, THIRD] {
            match read(&world.backend, Actor::Client, client(who), view, params) {
                Err(Refused::NotGranted(Some(actor))) => {
                    assert_eq!(actor, "inbox.messaging.Client");
                }
                other => panic!("{view} as {who}: {other:?}"),
            }
        }
    }
}
