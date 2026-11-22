//! story:foreign-message-id-answers-as-unknown against the core: the order of the refusals of the
//! 5 commands that address a message. `unknown-message` first, then `foreign-message` for a caller
//! that is neither party, then `not-recipient` (or `not-sender` of `Collect`) for the other
//! party, then the refusals of the message's state. A third principal changes nothing and
//! publishes nothing.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const OTHER: &str = "agent:box/other";
const REQUEST: &str = "01JGT0FM000000000000000001";
const MISSING: &str = "01JGT0FM000000000000000099";

const MARK_DELIVERED: &str = "inbox.messaging.MarkDelivered";
const MARK_READ: &str = "inbox.messaging.MarkRead";
const ANSWER: &str = "inbox.messaging.Answer";
const REFUSE: &str = "inbox.messaging.Refuse";
const COLLECT: &str = "inbox.messaging.Collect";
const COMMANDS: [&str; 5] = [MARK_DELIVERED, MARK_READ, ANSWER, REFUSE, COLLECT];

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

fn store(backend: &MemoryBackend) -> Vec<String> {
    let mut held: Vec<String> = MessageStorage::list(backend)
        .iter()
        .map(|message| format!("{message:?}"))
        .collect();
    held.sort();
    held
}

/// The input of `command` for message `id`, sent by `who`: an answer is a reply from `who`.
fn input(command: &str, id: &str, who: &str) -> String {
    match command {
        ANSWER => format!(
            "{{\"in_reply_to\":\"{id}\",\"reply_id\":\"01JGT0FM000000000000000050\",\
             \"from\":\"{who}\",\"on_behalf_of\":\"{who}\",\"created\":\"2026-01-05T09:05:00Z\",\
             \"expires\":\"2026-01-06T09:05:00Z\",\"body\":\"One line.\",\
             \"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
        REFUSE => format!("{{\"message_id\":\"{id}\",\"reason\":\"No.\"}}"),
        _ => format!("{{\"message_id\":\"{id}\"}}"),
    }
}

/// `command` as `who` for message `id`: its outcome, after asserting that a refusal moved nothing
/// and published nothing.
fn refused(backend: &MemoryBackend, who: &str, command: &str, id: &str) -> String {
    let before = store(backend);
    let answer = as_client(backend, who, command, &input(command, id, who));
    assert_eq!(published(&answer), 0, "{command} as {who}: {answer:?}");
    assert_eq!(store(backend), before, "{command} as {who} moved the store");
    let taken = outcome(&answer);
    if taken == "foreign-message" || taken == "unknown-message" {
        assert_eq!(
            error(&answer).as_deref(),
            Some("inbox.messaging.UnknownMessage"),
            "{command} as {who}"
        );
    }
    taken
}

/// The worker's inbox open, a request from the planner in it.
fn world() -> MemoryBackend {
    world_from(PLANNER, PLANNER)
}

/// The worker's inbox open, a request in it whose `from` and `on_behalf_of` are these; one that
/// speaks for another names a delegation, which the accepting verifier lets pass.
fn world_from(from: &str, on_behalf_of: &str) -> MemoryBackend {
    let authority = if from == on_behalf_of {
        String::new()
    } else {
        "\"authority\":\"delegation-7\",".to_owned()
    };
    let backend = MemoryBackend::new();
    let opened = as_client(
        &backend,
        WORKER,
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(outcome(&opened), "opened");
    let posted = as_client(
        &backend,
        from,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{REQUEST}\",\"from\":\"{from}\",\"on_behalf_of\":\"{on_behalf_of}\",\
             {authority}\"to\":\"{WORKER}\",\"kind\":\"request\",\"thread\":\"{REQUEST}\",\
             \"reply_shape\":{{\"shape\":\"lines\",\"value\":3}},\
             \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
             \"dedup_key\":\"{REQUEST}\",\"body\":\"b\",\
             \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    );
    assert_eq!(outcome(&posted), "posted");
    backend
}

/// The refusal the other party gets from `command`: the sender from the 4 commands of the
/// recipient, the recipient from `Collect`.
fn party(command: &str) -> (&'static str, &'static str) {
    if command == COLLECT {
        (WORKER, "not-sender")
    } else {
        (PLANNER, "not-recipient")
    }
}

#[test]
fn unknown_message_answers_before_foreign_message_for_every_caller() {
    let backend = world();
    for command in COMMANDS {
        for who in [OTHER, PLANNER, WORKER] {
            assert_eq!(
                refused(&backend, who, command, MISSING),
                "unknown-message",
                "{command} as {who}"
            );
        }
    }
}

#[test]
fn foreign_message_answers_before_the_party_refusals_and_the_accepting_branches() {
    let backend = world();
    for command in COMMANDS {
        assert_eq!(
            refused(&backend, OTHER, command, REQUEST),
            "foreign-message",
            "{command}"
        );
        let (who, expected) = party(command);
        assert_eq!(
            refused(&backend, who, command, REQUEST),
            expected,
            "{command}"
        );
    }
}

#[test]
fn foreign_message_and_the_party_refusals_answer_before_the_state_refusals() {
    // Collect of a message not settled holds `not-settled`.
    let backend = world();
    assert_eq!(
        refused(&backend, OTHER, COLLECT, REQUEST),
        "foreign-message"
    );
    assert_eq!(refused(&backend, WORKER, COLLECT, REQUEST), "not-sender");
    assert_eq!(refused(&backend, PLANNER, COLLECT, REQUEST), "not-settled");

    // Read: a second mark or read holds `already-delivered` or `already-read`.
    for command in [MARK_READ, MARK_DELIVERED] {
        let read = as_client(
            &backend,
            WORKER,
            MARK_READ,
            &input(MARK_READ, REQUEST, WORKER),
        );
        assert!(matches!(outcome(&read).as_str(), "read" | "already-read"));
        assert_eq!(
            refused(&backend, OTHER, command, REQUEST),
            "foreign-message",
            "{command}"
        );
        assert_eq!(
            refused(&backend, PLANNER, command, REQUEST),
            "not-recipient",
            "{command}"
        );
    }
    assert_eq!(
        refused(&backend, WORKER, MARK_READ, REQUEST),
        "already-read"
    );

    // Answered: an answer or a refusal holds `already-settled`.
    let answered = as_client(&backend, WORKER, ANSWER, &input(ANSWER, REQUEST, WORKER));
    assert_eq!(outcome(&answered), "answered");
    for command in [ANSWER, REFUSE] {
        assert_eq!(
            refused(&backend, OTHER, command, REQUEST),
            "foreign-message",
            "{command}"
        );
        assert_eq!(
            refused(&backend, PLANNER, command, REQUEST),
            "not-recipient",
            "{command}"
        );
        assert_eq!(
            refused(&backend, WORKER, command, REQUEST),
            "already-settled",
            "{command}"
        );
    }
    assert_eq!(
        refused(&backend, OTHER, COLLECT, REQUEST),
        "foreign-message"
    );
    assert_eq!(refused(&backend, WORKER, COLLECT, REQUEST), "not-sender");
}

#[test]
fn the_input_guard_of_answer_stays_ahead_of_foreign_message() {
    let backend = world();
    for id in [REQUEST, MISSING] {
        let before = store(&backend);
        let answer = as_client(&backend, OTHER, ANSWER, &input(ANSWER, id, WORKER));
        assert_eq!(outcome(&answer), "not-sender", "{id}");
        assert_eq!(published(&answer), 0);
        assert_eq!(store(&backend), before);
    }
}

/// A session that relays a message for the operator: `from` is the session, `on_behalf_of` the
/// operator.
const RELAY: &str = "agent:ws/host-operator";
const OPERATOR: &str = "human:operator";

#[test]
fn the_relaying_sender_of_a_message_is_its_sender_and_collects() {
    let backend = world_from(RELAY, OPERATOR);
    for command in [MARK_DELIVERED, MARK_READ, ANSWER, REFUSE] {
        assert_eq!(
            refused(&backend, RELAY, command, REQUEST),
            "not-recipient",
            "{command}"
        );
    }
    assert_eq!(refused(&backend, RELAY, COLLECT, REQUEST), "not-settled");
    let answered = as_client(&backend, WORKER, ANSWER, &input(ANSWER, REQUEST, WORKER));
    assert_eq!(outcome(&answered), "answered");
    let collected = as_client(&backend, RELAY, COLLECT, &input(COLLECT, REQUEST, RELAY));
    assert_eq!(outcome(&collected), "collected");
}

#[test]
fn the_principal_a_message_is_relayed_for_is_a_third_principal() {
    let backend = world_from(RELAY, OPERATOR);
    for command in COMMANDS {
        assert_eq!(
            refused(&backend, OPERATOR, command, REQUEST),
            "foreign-message",
            "{command}"
        );
    }
    let answered = as_client(&backend, WORKER, ANSWER, &input(ANSWER, REQUEST, WORKER));
    assert_eq!(outcome(&answered), "answered");
    assert_eq!(
        refused(&backend, OPERATOR, COLLECT, REQUEST),
        "foreign-message"
    );
}
