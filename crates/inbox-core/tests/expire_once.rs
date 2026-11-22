//! story:expire-publishes-once: `Expire` through the entry `inbox_core::handle_with_events` hands
//! back exactly the events the model declares for each outcome, and each once. No step of the
//! conformance suite bounds the number of occurrences of an event of `Expire` from above, so a
//! core that publishes it twice is caught here, on the whole list. A step of the suite does
//! compare the payload of the event (`expect_event`), in scenarios whose thread is the message
//! id and whose sender speaks for itself; `adversary_w8_expire.rs` holds the count and the
//! payload off those points.

use inbox::actor::{Actor, Caller};
use inbox::messaging::{
    Address, DecisionTakenOnSilence, MessageExpired, MessageId, Principal, ThreadId,
};
use inbox::server::json::{self, Value};
use inbox::system::SystemEvent;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const ID: &str = "01JGT0EX000000000000000001";
const MISSING: &str = "01JGT0EX000000000000000099";
const EXPIRES: &str = "2026-01-05T10:00:00Z";
const KEPT_UNTIL: &str = "2026-01-06T10:00:00Z";

const OPEN: &str = "inbox.messaging.OpenInbox";
const POST: &str = "inbox.messaging.PostMessage";
const MARK_READ: &str = "inbox.messaging.MarkRead";
const EXPIRE: &str = "inbox.messaging.Expire";

#[derive(Debug, Clone, Default)]
struct Accepting;

impl SignatureVerifier for Accepting {
    fn verifies(&self, _envelope: Envelope<'_>) -> bool {
        true
    }

    fn authorizes(
        &self,
        _envelope: Envelope<'_>,
        _authority: &inbox::messaging::DelegationId,
    ) -> bool {
        true
    }
}

fn run(
    backend: &MemoryBackend,
    actor: Actor,
    who: &str,
    name: &str,
    input: &str,
) -> (Value, Vec<SystemEvent>) {
    let caller = match actor {
        Actor::Sweeper => CallerAttributes::none(),
        _ => CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned())),
    };
    let context = ServiceContext::new(caller, Accepting, backend.clone(), crate::support::clock());
    let (answer, events) = inbox_core::handle_with_events(
        backend,
        context,
        Some(&Caller { actor }),
        name,
        json::parse(input).expect("the case writes JSON"),
    );
    (
        answer.unwrap_or_else(|refused| panic!("{name} refused: {refused}")),
        events,
    )
}

fn outcome(answer: &Value) -> &str {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text,
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn world() -> MemoryBackend {
    let backend = MemoryBackend::new();
    let (opened, _) = run(
        &backend,
        Actor::Client,
        WORKER,
        OPEN,
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(outcome(&opened), "opened");
    backend
}

fn envelope(kind: &str, extra: &str) -> String {
    format!(
        "{{\"message_id\":\"{ID}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{PLANNER}\",\
         \"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{ID}\",{extra}\
         \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"{EXPIRES}\",\"dedup_key\":\"{ID}\",\
         \"body\":\"b\",\"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

fn post(backend: &MemoryBackend, kind: &str, extra: &str) {
    let (posted, events) = run(
        backend,
        Actor::Client,
        PLANNER,
        POST,
        &envelope(kind, extra),
    );
    assert_eq!(outcome(&posted), "posted");
    assert_eq!(
        events.len(),
        1,
        "a post publishes MessagePosted: {events:?}"
    );
}

fn post_note(backend: &MemoryBackend) {
    post(backend, "note", "");
}

fn post_request(backend: &MemoryBackend) {
    post(
        backend,
        "request",
        "\"reply_shape\":{\"shape\":\"lines\",\"value\":3},",
    );
}

fn post_decision(backend: &MemoryBackend, default_on_silence: Option<&str>) {
    let default = default_on_silence
        .map(|letter| format!("\"default_on_silence\":\"{letter}\","))
        .unwrap_or_default();
    post(
        backend,
        "decision",
        &format!(
            "\"decision\":{{\"options\":[{{\"letter\":\"A\",\"text\":\"Ship the release today.\"}},\
             {{\"letter\":\"B\",\"text\":\"Hold the release until Monday.\"}}],\
             \"recommended_option\":\"B\"}},{default}"
        ),
    );
}

fn expire(
    backend: &MemoryBackend,
    id: &str,
    now: &str,
    keep_until: &str,
) -> (String, Vec<SystemEvent>) {
    let (answer, events) = run(
        backend,
        Actor::Sweeper,
        "",
        EXPIRE,
        &format!("{{\"message_id\":\"{id}\",\"now\":\"{now}\",\"keep_until\":\"{keep_until}\"}}"),
    );
    (outcome(&answer).to_owned(), events)
}

fn expired(id: &str) -> Vec<SystemEvent> {
    vec![SystemEvent::MessageExpired(MessageExpired {
        message_id: MessageId(id.to_owned()),
        to: Principal(PLANNER.to_owned()),
        thread: ThreadId(id.to_owned()),
    })]
}

fn taken_on_silence(id: &str) -> Vec<SystemEvent> {
    vec![SystemEvent::DecisionTakenOnSilence(
        DecisionTakenOnSilence {
            message_id: MessageId(id.to_owned()),
            to: Principal(PLANNER.to_owned()),
            thread: ThreadId(id.to_owned()),
        },
    )]
}

fn names(events: &[SystemEvent]) -> Vec<&'static str> {
    events.iter().map(SystemEvent::name).collect()
}

#[test]
fn a_decision_with_default_on_silence_publishes_decision_taken_on_silence_once() {
    let backend = world();
    post_decision(&backend, Some("B"));
    let (taken, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(taken, "decided-on-silence");
    assert_eq!(
        names(&events),
        vec!["inbox.messaging.DecisionTakenOnSilence"],
        "{events:?}"
    );
    assert_eq!(events, taken_on_silence(ID));
}

#[test]
fn a_decision_with_default_on_silence_publishes_it_once_in_each_open_state() {
    for step in [None, Some("delivered"), Some("read")] {
        let backend = world();
        post_decision(&backend, Some("B"));
        match step {
            None => {}
            Some("delivered") => {
                let (answer, _) = run(
                    &backend,
                    Actor::Client,
                    WORKER,
                    "inbox.messaging.MarkDelivered",
                    &format!("{{\"message_id\":\"{ID}\"}}"),
                );
                assert_eq!(outcome(&answer), "delivered");
            }
            Some(_) => {
                let (answer, _) = run(
                    &backend,
                    Actor::Client,
                    WORKER,
                    MARK_READ,
                    &format!("{{\"message_id\":\"{ID}\"}}"),
                );
                assert_eq!(outcome(&answer), "read");
            }
        }
        let (taken, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
        assert_eq!(taken, "decided-on-silence", "{step:?}");
        assert_eq!(events, taken_on_silence(ID), "{step:?}");
    }
}

#[test]
fn a_note_publishes_message_expired_once() {
    let backend = world();
    post_note(&backend);
    let (taken, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(taken, "expired");
    assert_eq!(
        names(&events),
        vec!["inbox.messaging.MessageExpired"],
        "{events:?}"
    );
    assert_eq!(events, expired(ID));
}

#[test]
fn a_request_publishes_message_expired_once() {
    let backend = world();
    post_request(&backend);
    let (taken, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired(ID));
}

#[test]
fn a_decision_without_default_on_silence_publishes_message_expired_once() {
    let backend = world();
    post_decision(&backend, None);
    let (taken, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired(ID));
}

#[test]
fn a_decision_taken_on_silence_publishes_message_expired_once_when_it_is_removed() {
    let backend = world();
    post_decision(&backend, Some("B"));
    let (first, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(first, "decided-on-silence");
    assert_eq!(events, taken_on_silence(ID));

    let (second, events) = expire(&backend, ID, KEPT_UNTIL, "2026-01-07T10:00:00Z");
    assert_eq!(second, "expired");
    assert_eq!(events, expired(ID));
}

#[test]
fn not_due_publishes_nothing() {
    for decision in [None, Some("B")] {
        let backend = world();
        post_decision(&backend, decision);
        let (taken, events) = expire(&backend, ID, "2026-01-05T09:59:59Z", KEPT_UNTIL);
        assert_eq!(taken, "not-due", "{decision:?}");
        assert_eq!(events, Vec::<SystemEvent>::new(), "{decision:?}");
    }
}

#[test]
fn an_unknown_message_publishes_nothing() {
    let backend = world();
    post_note(&backend);
    let (taken, events) = expire(&backend, MISSING, EXPIRES, KEPT_UNTIL);
    assert_eq!(taken, "unknown-message");
    assert_eq!(events, Vec::<SystemEvent>::new());
}

#[test]
fn a_message_already_expired_is_unknown_and_publishes_nothing() {
    let backend = world();
    post_note(&backend);
    let (first, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(first, "expired");
    assert_eq!(events, expired(ID));

    let (again, events) = expire(&backend, ID, EXPIRES, KEPT_UNTIL);
    assert_eq!(again, "unknown-message");
    assert_eq!(events, Vec::<SystemEvent>::new());
}
