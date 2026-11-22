//! story:a-due-message-takes-no-mark: a message whose `expires` is at or before the service's
//! clock takes no `MarkRead` and no `MarkDelivered`; it stays as it is until the expiry task
//! decides it, so the events the sender of a due note gets do not depend on when the task ran. The
//! cases drive a fixed clock, never the system's.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, MessageState, Principal};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::system::SystemEvent;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const THIRD: &str = "agent:ws/third";
const ID: &str = "01JGT0DM000000000000000001";
const CREATED: &str = "2026-01-05T09:00:00Z";
const EXPIRES: &str = "2026-01-05T10:00:00Z";
const BEFORE: &str = "2026-01-05T09:59:59Z";
const AFTER: &str = "2026-01-05T10:00:01Z";
const KEPT_UNTIL: &str = "2026-01-06T10:00:00Z";
const MARK_DELIVERED: &str = "inbox.messaging.MarkDelivered";
const MARK_READ: &str = "inbox.messaging.MarkRead";

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

/// The context of `who` (the Sweeper where `actor` says so), with the service's clock reading
/// `now`.
fn context(
    actor: Actor,
    who: &str,
    backend: &MemoryBackend,
    now: &str,
) -> ServiceContext<Accepting, MemoryBackend, Timestamp> {
    let caller = match actor {
        Actor::Sweeper => CallerAttributes::none(),
        _ => CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned())),
    };
    ServiceContext::new(
        caller,
        Accepting,
        backend.clone(),
        Timestamp(now.to_owned()),
    )
}

/// Runs one command for `who` through `inbox_core::handle_with_events`, with the service's clock
/// reading `now`.
fn run(
    backend: &MemoryBackend,
    actor: Actor,
    who: &str,
    now: &str,
    name: &str,
    input: &str,
) -> (Value, Vec<SystemEvent>) {
    let (answer, events) = inbox_core::handle_with_events(
        backend,
        context(actor, who, backend, now),
        Some(&Caller { actor }),
        name,
        json::parse(input).expect("the case writes JSON"),
    );
    (
        answer.unwrap_or_else(|refused| panic!("{name} refused: {refused}")),
        events,
    )
}

fn member<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    match value.member(name) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}

fn outcome(answer: &Value) -> &str {
    member(answer, "outcome").unwrap_or_else(|| panic!("no outcome in {answer:?}"))
}

/// Every field of every stored message, in a form two readings compare by.
fn store(backend: &MemoryBackend) -> String {
    let mut held: Vec<String> = MessageStorage::list(backend)
        .iter()
        .map(|message| format!("{message:?}"))
        .collect();
    held.sort();
    held.join("\n")
}

/// The worker's open inbox and the planner's message of `kind` to it, due at `EXPIRES`.
fn world(kind: &str) -> MemoryBackend {
    let backend = MemoryBackend::new();
    let (opened, _) = run(
        &backend,
        Actor::Client,
        WORKER,
        CREATED,
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(outcome(&opened), "opened");
    let shape = if kind == "request" {
        "\"reply_shape\":{\"shape\":\"lines\",\"value\":1},"
    } else {
        ""
    };
    let (posted, _) = run(
        &backend,
        Actor::Client,
        PLANNER,
        CREATED,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{ID}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{PLANNER}\",\
             \"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{ID}\",{shape}\
             \"created\":\"{CREATED}\",\"expires\":\"{EXPIRES}\",\"dedup_key\":\"{ID}\",\
             \"body\":\"b\",\"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    );
    assert_eq!(outcome(&posted), "posted", "{posted:?}");
    backend
}

fn mark(backend: &MemoryBackend, who: &str, now: &str, name: &str) -> (Value, Vec<SystemEvent>) {
    run(
        backend,
        Actor::Client,
        who,
        now,
        name,
        &format!("{{\"message_id\":\"{ID}\"}}"),
    )
}

/// The expiry task at the clock reading `now`: its outcome and the events it published.
fn sweep(backend: &MemoryBackend, now: &str) -> (String, Vec<SystemEvent>) {
    let (expired, events) = run(
        backend,
        Actor::Sweeper,
        "",
        now,
        "inbox.messaging.Expire",
        &format!("{{\"message_id\":\"{ID}\",\"now\":\"{now}\",\"keep_until\":\"{KEPT_UNTIL}\"}}"),
    );
    (outcome(&expired).to_owned(), events)
}

/// Refused as due: the outcome and its error, and nothing published.
fn assert_due(name: &str, answered: &(Value, Vec<SystemEvent>)) {
    let (answer, events) = answered;
    assert_eq!(outcome(answer), "deadline-passed", "{name}: {answer:?}");
    assert_eq!(
        member(answer, "error"),
        Some("inbox.messaging.Expired"),
        "{name}: {answer:?}"
    );
    assert!(events.is_empty(), "{name}: {events:?}");
}

fn state(backend: &MemoryBackend) -> MessageState {
    MessageStorage::list(backend)
        .first()
        .map(|held| held.state)
        .expect("the message is held")
}

#[test]
fn a_mark_on_a_due_message_in_each_open_state_changes_nothing() {
    for name in [MARK_DELIVERED, MARK_READ] {
        for (kind, arrange) in [
            ("note", None),
            ("request", None),
            ("note", Some(MARK_DELIVERED)),
            ("request", Some(MARK_DELIVERED)),
            ("request", Some(MARK_READ)),
        ] {
            for now in [EXPIRES, AFTER] {
                let backend = world(kind);
                if let Some(first) = arrange {
                    let (taken, _) = mark(&backend, WORKER, CREATED, first);
                    assert!(
                        ["delivered", "read"].contains(&outcome(&taken)),
                        "{taken:?}"
                    );
                }
                let held = state(&backend);
                let before = store(&backend);
                assert_due(
                    &format!("{name} of a {kind} in {held:?} at {now}"),
                    &mark(&backend, WORKER, now, name),
                );
                assert_eq!(store(&backend), before, "{name} at {now} changes no row");
            }
        }
    }
}

#[test]
fn a_mark_one_second_before_the_deadline_is_taken() {
    let delivered = world("note");
    let (taken, events) = mark(&delivered, WORKER, BEFORE, MARK_DELIVERED);
    assert_eq!(outcome(&taken), "delivered", "{taken:?}");
    assert_eq!(events.len(), 1, "{events:?}");

    let read = world("request");
    let (taken, events) = mark(&read, WORKER, BEFORE, MARK_READ);
    assert_eq!(outcome(&taken), "read", "{taken:?}");
    assert_eq!(events.len(), 1, "{events:?}");

    let handled = world("note");
    let (taken, _) = mark(&handled, WORKER, BEFORE, MARK_READ);
    assert_eq!(outcome(&taken), "handled", "{taken:?}");
    assert!(MessageStorage::list(&handled).is_empty());
}

/// The two orders on a due note, mark then sweep and sweep then mark, give the sender the same
/// events and leave the same store.
#[test]
fn a_due_note_marked_then_swept_and_swept_then_marked_give_the_same_events() {
    for name in [MARK_READ, MARK_DELIVERED] {
        let mark_first = world("note");
        let (marked, mut first_events) = mark(&mark_first, WORKER, AFTER, name);
        assert_due(name, &(marked, first_events.clone()));
        let (swept, events) = sweep(&mark_first, AFTER);
        assert_eq!(swept, "expired");
        first_events.extend(events);

        let sweep_first = world("note");
        let (swept, mut second_events) = sweep(&sweep_first, AFTER);
        assert_eq!(swept, "expired");
        let (gone, events) = mark(&sweep_first, WORKER, AFTER, name);
        assert_eq!(outcome(&gone), "unknown-message", "{gone:?}");
        second_events.extend(events);

        assert_eq!(
            format!("{first_events:?}"),
            format!("{second_events:?}"),
            "{name}: the sender gets the same events in either order"
        );
        assert_eq!(first_events.len(), 1, "{first_events:?}");
        assert!(
            format!("{first_events:?}").contains("MessageExpired"),
            "{first_events:?}"
        );
        assert_eq!(store(&mark_first), store(&sweep_first));
        assert!(MessageStorage::list(&mark_first).is_empty());
    }
}

#[test]
fn a_third_principal_still_finds_no_message_when_it_marks_a_due_one() {
    let backend = world("request");
    for name in [MARK_DELIVERED, MARK_READ] {
        let (marked, events) = mark(&backend, THIRD, AFTER, name);
        assert_eq!(outcome(&marked), "unknown-message", "{name}: {marked:?}");
        assert!(events.is_empty(), "{events:?}");
        // The sender is no recipient, and is told that first.
        let (marked, _) = mark(&backend, PLANNER, AFTER, name);
        assert_eq!(outcome(&marked), "not-recipient", "{name}: {marked:?}");
    }
    // The model's own outcome for the third principal, without the core's rename.
    for name in [MARK_DELIVERED, MARK_READ] {
        let (answer, _) = inbox_core::handle_as_specified(
            &backend,
            context(Actor::Client, THIRD, &backend, AFTER),
            Some(&Caller {
                actor: Actor::Client,
            }),
            name,
            json::parse(&format!("{{\"message_id\":\"{ID}\"}}")).expect("JSON"),
        );
        let answer = answer.unwrap_or_else(|refused| panic!("{name} refused: {refused}"));
        assert_eq!(outcome(&answer), "foreign-message", "{name}: {answer:?}");
    }
}
