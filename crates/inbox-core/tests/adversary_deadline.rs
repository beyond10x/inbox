//! Adversary pass 1 on story:a-due-message-takes-no-answer, in the core: every order of the late
//! calls around the sweep, the boundary of `expires` written with a fraction or an offset, the
//! answered state, and a reply whose own `expires` the service's clock has already passed.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, MessageState, Principal};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::system::SystemEvent;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const ID: &str = "01JGT0DE000000000000000101";
const REPLY: &str = "01JGT0DE000000000000000102";
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

/// One command, as `who` (the Sweeper for an empty `who`), with the service's clock at `now`:
/// the outcome (or the refusal's text) and the events.
fn run(
    backend: &MemoryBackend,
    who: &str,
    now: &str,
    name: &str,
    input: &str,
) -> (String, Vec<SystemEvent>) {
    let (actor, caller) = if who.is_empty() {
        (Actor::Sweeper, CallerAttributes::none())
    } else {
        (
            Actor::Client,
            CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned())),
        )
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
    let outcome = match answer {
        Ok(value) => match value.member("outcome") {
            Some(Value::Text(text)) => text.clone(),
            other => format!("no outcome: {other:?}"),
        },
        Err(refused) => format!("refused: {refused}"),
    };
    (outcome, events)
}

fn store(backend: &MemoryBackend) -> String {
    let mut held: Vec<String> = MessageStorage::list(backend)
        .iter()
        .map(|message| format!("{message:?}"))
        .collect();
    held.sort();
    held.join("\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ask {
    DecisionWithDefault,
    DecisionWithoutDefault,
    Request,
    Note,
}

fn envelope(ask: Ask, expires: &str) -> String {
    let extra = match ask {
        Ask::DecisionWithDefault => {
            "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"},\
             {\"letter\":\"B\",\"text\":\"Hold.\"}],\"recommended_option\":\"A\"},\
             \"default_on_silence\":\"B\","
        }
        Ask::DecisionWithoutDefault => {
            "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"},\
             {\"letter\":\"B\",\"text\":\"Hold.\"}],\"recommended_option\":\"A\"},"
        }
        Ask::Request => "\"reply_shape\":{\"shape\":\"lines\",\"value\":1},",
        Ask::Note => "",
    };
    let kind = match ask {
        Ask::DecisionWithDefault | Ask::DecisionWithoutDefault => "decision",
        Ask::Request => "request",
        Ask::Note => "note",
    };
    format!(
        "{{\"message_id\":\"{ID}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{PLANNER}\",\
         \"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{ID}\",{extra}\
         \"created\":\"{CREATED}\",\"expires\":\"{expires}\",\"dedup_key\":\"{ID}\",\
         \"body\":\"b\",\"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

fn reply(ask: Ask, created: &str, expires: &str) -> String {
    let option = match ask {
        Ask::DecisionWithDefault | Ask::DecisionWithoutDefault => "\"option\":\"A\",",
        _ => "",
    };
    format!(
        "{{\"in_reply_to\":\"{ID}\",\"reply_id\":\"{REPLY}\",\"from\":\"{WORKER}\",\
         \"on_behalf_of\":\"{WORKER}\",\"created\":\"{created}\",\"expires\":\"{expires}\",\
         {option}\"body\":\"A.\",\"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

/// The worker's open inbox and the planner's message to it, posted at `CREATED`, moved to `state`
/// one second before its deadline.
fn world(ask: Ask, expires: &str, state: MessageState) -> MemoryBackend {
    let backend = MemoryBackend::new();
    let (opened, _) = run(
        &backend,
        WORKER,
        CREATED,
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(opened, "opened");
    let (posted, _) = run(
        &backend,
        PLANNER,
        CREATED,
        "inbox.messaging.PostMessage",
        &envelope(ask, expires),
    );
    assert_eq!(posted, "posted", "{ask:?} {expires}");
    let id = format!("{{\"message_id\":\"{ID}\"}}");
    if matches!(state, MessageState::Delivered | MessageState::Read) {
        assert_eq!(
            run(
                &backend,
                WORKER,
                BEFORE,
                "inbox.messaging.MarkDelivered",
                &id
            )
            .0,
            "delivered"
        );
    }
    if state == MessageState::Read {
        assert_eq!(
            run(&backend, WORKER, BEFORE, "inbox.messaging.MarkRead", &id).0,
            "read"
        );
    }
    backend
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Call {
    Answer,
    Refuse,
    Sweep,
    MarkRead,
    MarkDelivered,
    Collect,
}

fn call(backend: &MemoryBackend, ask: Ask, now: &str, what: Call) -> (String, Vec<SystemEvent>) {
    let id = format!("{{\"message_id\":\"{ID}\"}}");
    match what {
        Call::Answer => run(
            backend,
            WORKER,
            now,
            "inbox.messaging.Answer",
            &reply(ask, CREATED, KEPT_UNTIL),
        ),
        Call::Refuse => run(
            backend,
            WORKER,
            now,
            "inbox.messaging.Refuse",
            &format!("{{\"message_id\":\"{ID}\",\"reason\":\"late\"}}"),
        ),
        Call::Sweep => run(
            backend,
            "",
            now,
            "inbox.messaging.Expire",
            &format!(
                "{{\"message_id\":\"{ID}\",\"now\":\"{now}\",\"keep_until\":\"{KEPT_UNTIL}\"}}"
            ),
        ),
        Call::MarkRead => run(backend, WORKER, now, "inbox.messaging.MarkRead", &id),
        Call::MarkDelivered => run(backend, WORKER, now, "inbox.messaging.MarkDelivered", &id),
        Call::Collect => run(backend, PLANNER, now, "inbox.messaging.Collect", &id),
    }
}

fn permutations(items: &[Call]) -> Vec<Vec<Call>> {
    if items.len() <= 1 {
        return vec![items.to_vec()];
    }
    let mut all = Vec::new();
    for (at, first) in items.iter().enumerate() {
        let mut rest = items.to_vec();
        rest.remove(at);
        for mut tail in permutations(&rest) {
            tail.insert(0, *first);
            all.push(tail);
        }
    }
    all
}

/// Attack 1. At the deadline (the reading equal to `expires`; past it the comparison is the same
/// `<=`, and `answer_refusal_post_and_sweep_meet_the_deadline_at_the_same_reading` holds the
/// readings around it), in every state a message of `ask` can be in, every order of {Answer,
/// Refuse, sweep, MarkRead, MarkDelivered, Collect} leaves the store, the events and the outcomes
/// of the other calls exactly as the same order without the late `Answer` and `Refuse` does: a
/// late answer or refusal changes, suppresses or reorders nothing, and the sender ends with the
/// same record. The run without them is made once per order of the other 4.
fn every_order_holds(ask: Ask, states: &[MessageState]) {
    let calls = [
        Call::Answer,
        Call::Refuse,
        Call::Sweep,
        Call::MarkRead,
        Call::MarkDelivered,
        Call::Collect,
    ];
    let orders = permutations(&calls);
    assert_eq!(orders.len(), 720);
    let now = EXPIRES;
    let mut checked = 0;
    for &state in states {
        let mut reference: std::collections::HashMap<Vec<Call>, (Vec<String>, String)> =
            std::collections::HashMap::new();
        for order in &orders {
            let rest: Vec<Call> = order
                .iter()
                .copied()
                .filter(|what| !matches!(what, Call::Answer | Call::Refuse))
                .collect();
            let (without_log, without_store) = reference
                .entry(rest.clone())
                .or_insert_with(|| {
                    let without = world(ask, EXPIRES, state);
                    let log = rest
                        .iter()
                        .map(|what| {
                            let (outcome, events) = call(&without, ask, now, *what);
                            format!("{what:?} {outcome} {events:?}")
                        })
                        .collect();
                    (log, store(&without))
                })
                .clone();
            let with = world(ask, EXPIRES, state);
            let mut with_log = Vec::new();
            for what in order {
                let (outcome, events) = call(&with, ask, now, *what);
                if matches!(what, Call::Answer | Call::Refuse) {
                    assert!(
                        events.is_empty(),
                        "{ask:?} in {state:?}, order {order:?}: late {what:?} answered \
                         {outcome} and published {events:?}"
                    );
                    assert!(
                        !matches!(outcome.as_str(), "answered" | "refused"),
                        "{ask:?} in {state:?}, order {order:?}: late {what:?} was taken"
                    );
                    continue;
                }
                with_log.push(format!("{what:?} {outcome} {events:?}"));
            }
            assert_eq!(
                with_log, without_log,
                "{ask:?} in {state:?}, order {order:?}"
            );
            assert_eq!(
                store(&with),
                without_store,
                "{ask:?} in {state:?}, order {order:?}"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, states.len() * 720);
}

const OPEN: [MessageState; 3] = [
    MessageState::Sent,
    MessageState::Delivered,
    MessageState::Read,
];

#[test]
fn every_late_order_leaves_a_decision_with_default_as_without_answer_and_refusal() {
    every_order_holds(Ask::DecisionWithDefault, &OPEN);
}

#[test]
fn every_late_order_leaves_a_decision_without_default_as_without_answer_and_refusal() {
    every_order_holds(Ask::DecisionWithoutDefault, &OPEN);
}

#[test]
fn every_late_order_leaves_a_request_as_without_answer_and_refusal() {
    every_order_holds(Ask::Request, &OPEN);
}

/// A note is never in `Read`: `MarkRead` removes it.
#[test]
fn every_late_order_leaves_a_note_as_without_answer_and_refusal() {
    every_order_holds(Ask::Note, &[MessageState::Sent, MessageState::Delivered]);
}

/// Attack 2. For `expires` written with a fraction, an offset or in lower case, at readings on
/// both sides of it, `Answer` and `Refuse` are refused `deadline-passed` exactly where the sweep at
/// the same reading finds the message due, and a post is refused on arrival exactly there too.
#[test]
fn answer_refusal_post_and_sweep_meet_the_deadline_at_the_same_reading() {
    let deadlines = [
        "2026-01-05T10:00:00.500Z",
        "2026-01-05T10:00:00.000000001Z",
        "2026-01-05T12:00:00+02:00",
        "2026-01-05T09:30:00-00:30",
        "2026-01-05t10:00:00z",
        "2026-01-05T10:00:00Z",
    ];
    let readings = [
        "2026-01-05T09:59:59Z",
        "2026-01-05T10:00:00Z",
        "2026-01-05T10:00:00.400Z",
        "2026-01-05T10:00:00.500Z",
        "2026-01-05T10:00:01Z",
        "2026-01-05T11:00:00+01:00",
    ];
    for expires in deadlines {
        for now in readings {
            let swept = world(Ask::Request, expires, MessageState::Sent);
            let (sweep, _) = call(&swept, Ask::Request, now, Call::Sweep);
            let due = match sweep.as_str() {
                "not-due" => false,
                "expired" => true,
                other => panic!("sweep of {expires} at {now}: {other}"),
            };

            let answered = world(Ask::Request, expires, MessageState::Sent);
            let (answer, _) = call(&answered, Ask::Request, now, Call::Answer);
            let refused = world(Ask::Request, expires, MessageState::Sent);
            let (refusal, _) = call(&refused, Ask::Request, now, Call::Refuse);
            assert_eq!(
                (answer.as_str(), refusal.as_str()),
                if due {
                    ("deadline-passed", "deadline-passed")
                } else {
                    ("answered", "refused")
                },
                "expires {expires}, reading {now}: the sweep says due = {due}"
            );

            let backend = MemoryBackend::new();
            run(
                &backend,
                WORKER,
                CREATED,
                "inbox.messaging.OpenInbox",
                &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
            );
            let (post, _) = run(
                &backend,
                PLANNER,
                now,
                "inbox.messaging.PostMessage",
                &envelope(Ask::Request, expires),
            );
            assert_eq!(
                post,
                if due { "expired-on-arrival" } else { "posted" },
                "post of expires {expires} at reading {now}: the sweep says due = {due}"
            );
        }
    }
}

/// Attack 5 and mutant 3. A decision answered on silence carries `keep_until` as its `expires`.
/// Past that instant and before the sweep removes it, `Answer` and `Refuse` still answer
/// `already-settled`, as the specification's `deadline-passed` names only the open states, and
/// the sender still collects it.
#[test]
fn an_answered_message_past_its_retention_is_settled_not_due() {
    let backend = world(Ask::DecisionWithDefault, EXPIRES, MessageState::Sent);
    let (decided, _) = run(
        &backend,
        "",
        AFTER,
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":\"{ID}\",\"now\":\"{AFTER}\",\"keep_until\":\"2026-01-05T10:00:05Z\"}}"
        ),
    );
    assert_eq!(decided, "decided-on-silence");
    let late = "2026-01-05T10:00:06Z";
    let (answer, events) = call(&backend, Ask::DecisionWithDefault, late, Call::Answer);
    assert_eq!(answer, "already-settled", "{events:?}");
    let (refusal, _) = call(&backend, Ask::DecisionWithDefault, late, Call::Refuse);
    assert_eq!(refusal, "already-settled");
    let (collected, _) = call(&backend, Ask::DecisionWithDefault, late, Call::Collect);
    assert_eq!(collected, "collected");
}

/// Attack 5. `Answer` is documented as "the service checks it as it checks a post", and a post
/// whose `expires` the service's clock has reached is now refused on arrival. A reply written
/// with its own two timestamps in order, both before the service's clock, is still taken one
/// second before the message's deadline, stored with that past `expires`, and the next sweep at
/// the same reading removes it: the sender never collects the answer, and for a decision with
/// `default_on_silence` the silence default is gone too.
#[test]
fn a_reply_already_expired_at_the_service_clock_is_not_taken_and_then_swept() {
    for ask in [Ask::Request, Ask::DecisionWithDefault] {
        let backend = world(ask, EXPIRES, MessageState::Read);
        let late_reply = reply(ask, "2026-01-05T09:10:00Z", "2026-01-05T09:20:00Z");
        let (answer, _) = run(
            &backend,
            WORKER,
            BEFORE,
            "inbox.messaging.Answer",
            &late_reply,
        );
        let (swept, _) = call(&backend, ask, BEFORE, Call::Sweep);
        let (collected, _) = call(&backend, ask, BEFORE, Call::Collect);
        assert!(
            answer != "answered" || collected == "collected",
            "{ask:?}: the reply was {answer}, the sweep at the same reading {swept}, and the \
             sender's collect {collected}"
        );
    }
}
