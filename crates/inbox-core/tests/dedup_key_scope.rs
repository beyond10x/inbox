//! story:dedup-key-scope against the core: `duplicate-key` of `PostMessage` compares the
//! `dedup_key` among the stored messages of the post's `from`. The key of one sender refuses no
//! post of another; the same sender is refused a second post under its key, to the same inbox
//! and to another; a key of a message that is gone refuses nothing.
//!
//! The order of `duplicate` against `duplicate-key` is stated by the 2 `duplicate_*` cases at the
//! end. No case here states the order of `duplicate` against `unknown-inbox` or `inbox-closed`.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const THIRD: &str = "agent:box/other";
const FIRST: &str = "01JGT0DS000000000000000001";
const SECOND: &str = "01JGT0DS000000000000000002";
const REPLY: &str = "01JGT0DS000000000000000050";
const KEY: &str = "deploy-2026-11-21";

#[derive(Debug, Clone, Default)]
struct Accepting;

impl SignatureVerifier for Accepting {
    fn verifies(&self, _envelope: Envelope<'_>) -> bool {
        true
    }

    fn authorizes(&self, _envelope: Envelope<'_>, _authority: &DelegationId) -> bool {
        true
    }
}

fn run(
    backend: &MemoryBackend,
    actor: Actor,
    caller: CallerAttributes,
    name: &str,
    input: &str,
) -> Value {
    let context = ServiceContext::new(caller, Accepting, backend.clone());
    inbox_core::handle(
        backend,
        context,
        Some(&Caller { actor }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
    .unwrap_or_else(|refused| panic!("{name}: {refused}"))
}

/// `name` as a `Client` whose `principal` and `inbox` are both `who`.
fn as_client(backend: &MemoryBackend, who: &str, name: &str, input: &str) -> Value {
    let caller = CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()));
    run(backend, Actor::Client, caller, name, input)
}

fn outcome(answer: &Value) -> String {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn error(answer: &Value) -> Option<String> {
    match answer.member("error") {
        Some(Value::Text(text)) => Some(text.clone()),
        _ => None,
    }
}

fn published(answer: &Value) -> usize {
    match answer.member("published") {
        Some(Value::Array(events)) => events.len(),
        other => panic!("no published list in {answer:?}: {other:?}"),
    }
}

/// The stored messages, as `(message_id, from, to, dedup_key)`, sorted.
fn stored(backend: &MemoryBackend) -> Vec<(String, String, String, String)> {
    let mut held: Vec<_> = MessageStorage::list(backend)
        .iter()
        .map(|row| {
            (
                row.data.message_id.0.clone(),
                row.data.from.0.clone(),
                row.data.to.0.clone(),
                row.data.dedup_key.0.clone(),
            )
        })
        .collect();
    held.sort();
    held
}

fn row(id: &str, from: &str, to: &str) -> (String, String, String, String) {
    (
        id.to_owned(),
        from.to_owned(),
        to.to_owned(),
        KEY.to_owned(),
    )
}

/// The inboxes of the worker and of the third principal, each opened by its owner.
fn world() -> MemoryBackend {
    let backend = MemoryBackend::new();
    for who in [WORKER, THIRD] {
        let opened = as_client(
            &backend,
            who,
            "inbox.messaging.OpenInbox",
            &format!("{{\"address\":\"{who}\",\"adapter\":\"claude_code\"}}"),
        );
        assert_eq!(outcome(&opened), "opened", "{who}");
    }
    backend
}

/// Posts message `id` of `kind` from `from` to `to` under the dedup key `KEY`, as `from`.
fn post(backend: &MemoryBackend, id: &str, kind: &str, from: &str, to: &str) -> Value {
    let shape = if kind == "request" {
        "\"reply_shape\":{\"shape\":\"lines\",\"value\":3},"
    } else {
        ""
    };
    as_client(
        backend,
        from,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{id}\",\"from\":\"{from}\",\"on_behalf_of\":\"{from}\",\
             \"to\":\"{to}\",\"kind\":\"{kind}\",\"thread\":\"{id}\",{shape}\
             \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
             \"dedup_key\":\"{KEY}\",\"body\":\"b\",\
             \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    )
}

fn posted(backend: &MemoryBackend, id: &str, kind: &str, from: &str, to: &str) {
    let answer = post(backend, id, kind, from, to);
    assert_eq!(
        outcome(&answer),
        "posted",
        "{id} from {from} to {to}: {answer:?}"
    );
    assert_eq!(published(&answer), 1, "{answer:?}");
}

/// A post that is refused: its outcome, after asserting that it stored and published nothing.
fn refused(backend: &MemoryBackend, id: &str, from: &str, to: &str) -> String {
    let before = stored(backend);
    let answer = post(backend, id, "note", from, to);
    assert_eq!(published(&answer), 0, "{answer:?}");
    assert_eq!(stored(backend), before, "a refused post changed the store");
    assert_eq!(
        error(&answer).as_deref(),
        Some("inbox.messaging.Duplicate"),
        "{answer:?}"
    );
    outcome(&answer)
}

#[test]
fn a_third_principals_key_refuses_no_post_of_another_sender_to_another_inbox() {
    let backend = world();
    posted(&backend, FIRST, "note", THIRD, THIRD);
    posted(&backend, SECOND, "note", PLANNER, WORKER);
    assert_eq!(
        stored(&backend),
        vec![row(FIRST, THIRD, THIRD), row(SECOND, PLANNER, WORKER)]
    );
}

#[test]
fn a_second_sender_under_the_same_key_to_the_same_inbox_is_posted() {
    let backend = world();
    posted(&backend, FIRST, "note", PLANNER, WORKER);
    posted(&backend, SECOND, "note", THIRD, WORKER);
    assert_eq!(
        stored(&backend),
        vec![row(FIRST, PLANNER, WORKER), row(SECOND, THIRD, WORKER)]
    );
}

#[test]
fn the_same_sender_under_its_key_is_refused_to_the_same_inbox() {
    let backend = world();
    posted(&backend, FIRST, "note", PLANNER, WORKER);
    assert_eq!(refused(&backend, SECOND, PLANNER, WORKER), "duplicate-key");
    assert_eq!(stored(&backend), vec![row(FIRST, PLANNER, WORKER)]);
}

#[test]
fn the_same_sender_under_its_key_is_refused_to_another_inbox() {
    let backend = world();
    posted(&backend, FIRST, "note", PLANNER, WORKER);
    assert_eq!(refused(&backend, SECOND, PLANNER, THIRD), "duplicate-key");
    assert_eq!(stored(&backend), vec![row(FIRST, PLANNER, WORKER)]);
}

#[test]
fn a_key_of_an_expired_and_removed_message_refuses_nothing() {
    let backend = world();
    posted(&backend, FIRST, "note", PLANNER, WORKER);
    let expired = run(
        &backend,
        Actor::Sweeper,
        CallerAttributes::none(),
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":\"{FIRST}\",\"now\":\"2026-01-07T09:00:00Z\",\
             \"keep_until\":\"2026-01-08T09:00:00Z\"}}"
        ),
    );
    assert_eq!(outcome(&expired), "expired", "{expired:?}");
    assert!(stored(&backend).is_empty());
    posted(&backend, SECOND, "note", PLANNER, WORKER);
    assert_eq!(stored(&backend), vec![row(SECOND, PLANNER, WORKER)]);
}

#[test]
fn a_key_of_a_collected_message_refuses_nothing() {
    let backend = world();
    posted(&backend, FIRST, "request", PLANNER, WORKER);
    let answered = as_client(
        &backend,
        WORKER,
        "inbox.messaging.Answer",
        &format!(
            "{{\"in_reply_to\":\"{FIRST}\",\"reply_id\":\"{REPLY}\",\
             \"from\":\"{WORKER}\",\"on_behalf_of\":\"{WORKER}\",\"created\":\"2026-01-05T09:05:00Z\",\
             \"expires\":\"2026-01-06T09:05:00Z\",\"body\":\"One line.\",\
             \"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    );
    assert_eq!(outcome(&answered), "answered", "{answered:?}");
    let collected = as_client(
        &backend,
        PLANNER,
        "inbox.messaging.Collect",
        &format!("{{\"message_id\":\"{FIRST}\"}}"),
    );
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    assert!(stored(&backend).is_empty());
    posted(&backend, SECOND, "note", PLANNER, WORKER);
    assert_eq!(stored(&backend), vec![row(SECOND, PLANNER, WORKER)]);
}

/// The order of `duplicate` against `duplicate-key`, for one sender: both hold, `duplicate`
/// answers.
#[test]
fn duplicate_answers_before_duplicate_key_for_the_same_sender() {
    let backend = world();
    posted(&backend, FIRST, "note", PLANNER, WORKER);
    assert_eq!(refused(&backend, FIRST, PLANNER, WORKER), "duplicate");
    assert_eq!(refused(&backend, FIRST, PLANNER, THIRD), "duplicate");
}

/// `duplicate` across senders: another sender posts the stored message id under the same key into
/// an open inbox. Its key is free, the id is not, and `duplicate` answers, as the model states
/// it after this story (`existing_instance: true`, decided by `message_id` alone).
/// story:post-does-not-tell-a-message-id-exists changes that answer; this is the case it rewrites.
#[test]
fn duplicate_answers_for_a_stored_message_id_across_senders() {
    let backend = world();
    posted(&backend, FIRST, "note", PLANNER, WORKER);
    assert_eq!(refused(&backend, FIRST, THIRD, THIRD), "duplicate");
}
