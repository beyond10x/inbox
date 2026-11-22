//! story:a-due-message-takes-no-answer: a message whose `expires` is at or before the service's
//! clock takes no answer and no refusal, and a post due at the clock is refused. The clock is the
//! context's; no member the client sends carries it. The cases drive a fixed clock, never the
//! system's.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::system::SystemEvent;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const THIRD: &str = "agent:ws/third";
const ID: &str = "01JGT0DE000000000000000001";
const REPLY: &str = "01JGT0DE000000000000000002";
const CREATED: &str = "2026-01-05T09:00:00Z";
const EXPIRES: &str = "2026-01-05T10:00:00Z";
const BEFORE: &str = "2026-01-05T09:59:59Z";
const AFTER: &str = "2026-01-05T10:00:01Z";
const KEPT_UNTIL: &str = "2026-01-06T10:00:00Z";

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

/// Runs one command for `who` (the Sweeper where `actor` says so) with the service's clock reading
/// `now`.
fn run(
    backend: &MemoryBackend,
    actor: Actor,
    who: &str,
    now: &str,
    name: &str,
    input: &str,
) -> (Value, Vec<SystemEvent>) {
    let caller = match actor {
        Actor::Sweeper => CallerAttributes::none(),
        _ => CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned())),
    };
    let context = ServiceContext::new(
        caller,
        Accepting,
        backend.clone(),
        Timestamp(now.to_owned()),
    );
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

/// The worker's open inbox.
fn world() -> MemoryBackend {
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
    backend
}

/// The planner's post to the worker: a decision between A and B whose silence takes B, or a
/// request of one line.
fn envelope(kind: &str, expires: &str) -> String {
    let extra = match kind {
        "decision" => {
            "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"},\
             {\"letter\":\"B\",\"text\":\"Hold.\"}],\"recommended_option\":\"A\"},\
             \"default_on_silence\":\"B\","
        }
        _ => "\"reply_shape\":{\"shape\":\"lines\",\"value\":1},",
    };
    format!(
        "{{\"message_id\":\"{ID}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{PLANNER}\",\
         \"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{ID}\",{extra}\
         \"created\":\"{CREATED}\",\"expires\":\"{expires}\",\"dedup_key\":\"{ID}\",\
         \"body\":\"b\",\"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

fn post(backend: &MemoryBackend, kind: &str) {
    let (posted, _) = run(
        backend,
        Actor::Client,
        PLANNER,
        CREATED,
        "inbox.messaging.PostMessage",
        &envelope(kind, EXPIRES),
    );
    assert_eq!(outcome(&posted), "posted", "{posted:?}");
}

/// The worker's reply, option A for a decision: written and signed after the deadline, and
/// meaning to wait for the sender a day, so its own two timestamps are in order.
fn reply(kind: &str) -> String {
    let option = if kind == "decision" {
        "\"option\":\"A\","
    } else {
        ""
    };
    format!(
        "{{\"in_reply_to\":\"{ID}\",\"reply_id\":\"{REPLY}\",\"from\":\"{WORKER}\",\
         \"on_behalf_of\":\"{WORKER}\",\"created\":\"{AFTER}\",\"expires\":\"{KEPT_UNTIL}\",\
         {option}\"body\":\"A.\",\"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

fn answer(backend: &MemoryBackend, who: &str, now: &str, kind: &str) -> (Value, Vec<SystemEvent>) {
    run(
        backend,
        Actor::Client,
        who,
        now,
        "inbox.messaging.Answer",
        &reply(kind),
    )
}

fn refuse(backend: &MemoryBackend, who: &str, now: &str) -> (Value, Vec<SystemEvent>) {
    run(
        backend,
        Actor::Client,
        who,
        now,
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":\"{ID}\",\"reason\":\"Too late.\"}}"),
    )
}

/// The expiry task at the clock reading `now`.
fn sweep(backend: &MemoryBackend, now: &str) -> String {
    let (expired, _) = run(
        backend,
        Actor::Sweeper,
        "",
        now,
        "inbox.messaging.Expire",
        &format!("{{\"message_id\":\"{ID}\",\"now\":\"{now}\",\"keep_until\":\"{KEPT_UNTIL}\"}}"),
    );
    outcome(&expired).to_owned()
}

/// Refused as due: the outcome and its error, and nothing published.
fn assert_due(answered: &(Value, Vec<SystemEvent>)) {
    let (answer, events) = answered;
    assert_eq!(outcome(answer), "deadline-passed", "{answer:?}");
    assert_eq!(
        member(answer, "error"),
        Some("inbox.messaging.Expired"),
        "{answer:?}"
    );
    assert!(events.is_empty(), "{events:?}");
}

#[test]
fn a_late_answer_then_the_sweep_leave_the_decision_to_silence() {
    let late_first = world();
    post(&late_first, "decision");
    let before = store(&late_first);
    assert_due(&answer(&late_first, WORKER, AFTER, "decision"));
    assert_eq!(
        store(&late_first),
        before,
        "a refused answer changes no row"
    );
    assert_eq!(sweep(&late_first, AFTER), "decided-on-silence");

    let sweep_first = world();
    post(&sweep_first, "decision");
    assert_eq!(sweep(&sweep_first, AFTER), "decided-on-silence");
    let (settled, events) = answer(&sweep_first, WORKER, AFTER, "decision");
    assert_eq!(outcome(&settled), "already-settled", "{settled:?}");
    assert!(events.is_empty(), "{events:?}");

    assert_eq!(store(&late_first), store(&sweep_first));
    let held = MessageStorage::list(&late_first);
    assert_eq!(held.len(), 1);
    assert_eq!(
        format!("{:?}", held[0].data.answered_by),
        "Some(Silence)",
        "{held:?}"
    );
    assert_eq!(
        held[0]
            .data
            .decided_option
            .as_ref()
            .map(|letter| letter.0.as_str()),
        Some("B")
    );
    assert!(held[0].data.reply.is_none(), "{held:?}");
}

#[test]
fn a_late_refusal_then_the_sweep_remove_the_request() {
    let late_first = world();
    post(&late_first, "request");
    let before = store(&late_first);
    assert_due(&refuse(&late_first, WORKER, AFTER));
    assert_eq!(
        store(&late_first),
        before,
        "a refused refusal changes no row"
    );
    assert_eq!(sweep(&late_first, AFTER), "expired");
    assert!(MessageStorage::list(&late_first).is_empty());

    let sweep_first = world();
    post(&sweep_first, "request");
    assert_eq!(sweep(&sweep_first, AFTER), "expired");
    let (gone, _) = refuse(&sweep_first, WORKER, AFTER);
    assert_eq!(outcome(&gone), "unknown-message", "{gone:?}");
    assert_eq!(store(&late_first), store(&sweep_first));
}

#[test]
fn an_answer_at_the_deadline_is_due_and_one_second_before_is_taken() {
    let at = world();
    post(&at, "request");
    assert_due(&answer(&at, WORKER, EXPIRES, "request"));
    assert_due(&refuse(&at, WORKER, EXPIRES));

    let before = world();
    post(&before, "request");
    let (answered, events) = answer(&before, WORKER, BEFORE, "request");
    assert_eq!(outcome(&answered), "answered", "{answered:?}");
    assert_eq!(events.len(), 1, "{events:?}");

    let refused = world();
    post(&refused, "request");
    let (taken, _) = refuse(&refused, WORKER, BEFORE);
    assert_eq!(outcome(&taken), "refused", "{taken:?}");
}

#[test]
fn a_third_principal_still_finds_no_message_after_the_deadline() {
    let backend = world();
    post(&backend, "decision");
    let (answered, _) = answer(&backend, THIRD, AFTER, "decision");
    // The reply's `from` is the worker's: the third principal sends it as its own request and is
    // refused `not-sender` first. With its own `from` it reaches the party check.
    assert_eq!(outcome(&answered), "not-sender", "{answered:?}");
    let own = reply("decision").replace(WORKER, THIRD);
    let (answered, _) = run(
        &backend,
        Actor::Client,
        THIRD,
        AFTER,
        "inbox.messaging.Answer",
        &own,
    );
    assert_eq!(outcome(&answered), "unknown-message", "{answered:?}");
    let (refused, _) = refuse(&backend, THIRD, AFTER);
    assert_eq!(outcome(&refused), "unknown-message", "{refused:?}");
    // The sender is no recipient, and is told that first.
    let (refused, _) = refuse(&backend, PLANNER, AFTER);
    assert_eq!(outcome(&refused), "not-recipient", "{refused:?}");
}

#[test]
fn a_post_due_at_the_clock_is_refused_and_stores_nothing() {
    for now in [EXPIRES, AFTER] {
        let backend = world();
        let (posted, events) = run(
            &backend,
            Actor::Client,
            PLANNER,
            now,
            "inbox.messaging.PostMessage",
            &envelope("decision", EXPIRES),
        );
        assert_eq!(outcome(&posted), "expired-on-arrival", "{posted:?}");
        assert_eq!(
            member(&posted, "error"),
            Some("inbox.messaging.AlreadyExpired"),
            "{posted:?}"
        );
        assert!(events.is_empty(), "{events:?}");
        assert!(MessageStorage::list(&backend).is_empty());
    }
    let backend = world();
    let (posted, _) = run(
        &backend,
        Actor::Client,
        PLANNER,
        BEFORE,
        "inbox.messaging.PostMessage",
        &envelope("decision", EXPIRES),
    );
    assert_eq!(outcome(&posted), "posted", "{posted:?}");
}
