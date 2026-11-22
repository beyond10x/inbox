//! story:post-does-not-tell-a-message-id-exists against the core: `duplicate` of `PostMessage`
//! answers a sender's post under the id of its own stored message, and `id-taken` a post under the
//! id of another sender's, after `unknown-inbox`, `duplicate-key` and `inbox-closed`. A post under
//! another sender's id to an unknown or closed inbox is answered as one under a free id.
//!
//! One case per pair of the order the ESS 0.97.3 interpreted target answers: `unknown-inbox`,
//! `duplicate`, `duplicate-key`, `inbox-closed`, `id-taken`, then `posted`. Each refused post
//! is held to store and publish nothing.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const THIRD: &str = "agent:box/other";
const NOBODY: &str = "agent:box/nobody";
const HELD: &str = "01JGT0PT000000000000000001";
const FREE: &str = "01JGT0PT000000000000000002";
const OWN: &str = "01JGT0PT000000000000000003";

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

/// `name` as a `Client` whose `principal` and `inbox` are both `who`.
fn as_client(backend: &MemoryBackend, who: &str, name: &str, input: &str) -> Value {
    let caller = CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()));
    let context = ServiceContext::new(caller, Accepting, backend.clone(), crate::support::clock());
    inbox_core::handle(
        backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
    .unwrap_or_else(|refused| panic!("{name} as {who}: {refused}"))
}

fn text(answer: &Value, member: &str) -> Option<String> {
    match answer.member(member) {
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

/// Every stored message, whole, sorted: a refused post must leave each field as it was.
fn stored(backend: &MemoryBackend) -> Vec<String> {
    let mut held: Vec<String> = MessageStorage::list(backend)
        .iter()
        .map(|row| format!("{row:?}"))
        .collect();
    held.sort();
    held
}

fn inbox_command(backend: &MemoryBackend, who: &str, name: &str, input: &str, outcome: &str) {
    let answer = as_client(backend, who, name, input);
    assert_eq!(
        text(&answer, "outcome").as_deref(),
        Some(outcome),
        "{answer:?}"
    );
}

fn open(backend: &MemoryBackend, who: &str) {
    let input = format!("{{\"address\":\"{who}\",\"adapter\":\"claude_code\"}}");
    inbox_command(backend, who, "inbox.messaging.OpenInbox", &input, "opened");
}

fn close(backend: &MemoryBackend, who: &str) {
    let input = format!("{{\"address\":\"{who}\"}}");
    inbox_command(backend, who, "inbox.messaging.CloseInbox", &input, "closed");
}

/// Posts a note `id` from `from` to `to` under the dedup key `key`, as `from`.
fn post(backend: &MemoryBackend, id: &str, from: &str, to: &str, key: &str) -> Value {
    as_client(
        backend,
        from,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{id}\",\"from\":\"{from}\",\"on_behalf_of\":\"{from}\",\
             \"to\":\"{to}\",\"kind\":\"note\",\"thread\":\"{id}\",\
             \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
             \"dedup_key\":\"{key}\",\"body\":\"b\",\
             \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    )
}

fn posted(backend: &MemoryBackend, id: &str, from: &str, to: &str, key: &str) {
    let answer = post(backend, id, from, to, key);
    assert_eq!(
        text(&answer, "outcome").as_deref(),
        Some("posted"),
        "{answer:?}"
    );
    assert_eq!(published(&answer), 1, "{answer:?}");
}

/// A post that is refused: `(outcome, error)`, after asserting that it stored, changed and
/// published nothing.
fn refused(backend: &MemoryBackend, id: &str, from: &str, to: &str, key: &str) -> (String, String) {
    let before = stored(backend);
    let answer = post(backend, id, from, to, key);
    assert_eq!(published(&answer), 0, "{answer:?}");
    assert_eq!(stored(backend), before, "a refused post changed the store");
    (
        text(&answer, "outcome").expect("an outcome"),
        text(&answer, "error").expect("an error"),
    )
}

fn pair(outcome: &str, error: &str) -> (String, String) {
    (outcome.to_owned(), format!("inbox.messaging.{error}"))
}

/// The worker's and the third principal's inboxes open, and the planner's message `HELD` stored
/// in the worker's inbox under the key `planner-key`.
fn world() -> MemoryBackend {
    let backend = MemoryBackend::new();
    open(&backend, WORKER);
    open(&backend, THIRD);
    posted(&backend, HELD, PLANNER, WORKER, "planner-key");
    backend
}

#[test]
fn own_id_to_an_address_no_inbox_has_answers_unknown_inbox() {
    let backend = world();
    assert_eq!(
        refused(&backend, HELD, PLANNER, NOBODY, "other-key"),
        pair("unknown-inbox", "UnknownInbox")
    );
}

#[test]
fn own_id_into_a_closed_inbox_answers_duplicate() {
    let backend = world();
    close(&backend, WORKER);
    assert_eq!(
        refused(&backend, HELD, PLANNER, WORKER, "other-key"),
        pair("duplicate", "Duplicate")
    );
}

#[test]
fn own_id_under_its_own_key_answers_duplicate_before_duplicate_key() {
    let backend = world();
    assert_eq!(
        refused(&backend, HELD, PLANNER, WORKER, "planner-key"),
        pair("duplicate", "Duplicate")
    );
}

#[test]
fn a_senders_repost_after_the_recipient_closed_the_inbox_answers_duplicate() {
    let backend = world();
    close(&backend, WORKER);
    assert_eq!(
        refused(&backend, HELD, PLANNER, WORKER, "planner-key"),
        pair("duplicate", "Duplicate")
    );
}

#[test]
fn another_senders_id_to_an_address_no_inbox_has_answers_as_a_free_id() {
    let backend = world();
    let held = refused(&backend, HELD, THIRD, NOBODY, "third-key");
    let free = refused(&backend, FREE, THIRD, NOBODY, "third-key-2");
    assert_eq!(held, pair("unknown-inbox", "UnknownInbox"));
    assert_eq!(held, free);
}

#[test]
fn another_senders_id_into_a_closed_inbox_answers_as_a_free_id() {
    let backend = world();
    close(&backend, THIRD);
    let held = refused(&backend, HELD, THIRD, THIRD, "third-key");
    let free = refused(&backend, FREE, THIRD, THIRD, "third-key-2");
    assert_eq!(held, pair("inbox-closed", "UnknownInbox"));
    assert_eq!(held, free);
}

#[test]
fn another_senders_id_under_a_held_key_of_the_caller_answers_duplicate_key() {
    let backend = world();
    posted(&backend, OWN, THIRD, THIRD, "third-key");
    assert_eq!(
        refused(&backend, HELD, THIRD, THIRD, "third-key"),
        pair("duplicate-key", "Duplicate")
    );
}

#[test]
fn another_senders_id_into_an_open_inbox_answers_id_taken_and_stores_nothing() {
    let backend = world();
    let before = stored(&backend);
    assert_eq!(
        refused(&backend, HELD, THIRD, THIRD, "third-key"),
        pair("id-taken", "MessageIdTaken")
    );
    assert_eq!(
        refused(&backend, HELD, THIRD, WORKER, "third-key"),
        pair("id-taken", "MessageIdTaken")
    );
    assert_eq!(stored(&backend), before, "the stored message changed");
    assert_eq!(
        refused(&backend, HELD, PLANNER, WORKER, "planner-key"),
        pair("duplicate", "Duplicate"),
        "the sender's own repost after another sender was refused"
    );
    posted(&backend, FREE, THIRD, THIRD, "third-key");
}
