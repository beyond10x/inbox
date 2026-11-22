//! Adversary pass on story:a-due-message-takes-no-mark, in the core: the claim that a due
//! message already answered or refused keeps `already-read` and `already-delivered`, the mark
//! held to the sweep at the same reading for every form an `expires` is written in, and the
//! answer every caller but the recipient gets from a mark, the same whether the message is due or
//! not. The cases drive a fixed clock, never the system's.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, MessageState, Principal};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::system::SystemEvent;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const THIRD: &str = "agent:elsewhere/third";
const ID: &str = "01JGT0AM000000000000000101";
const REPLY: &str = "01JGT0AM000000000000000102";
const CREATED: &str = "2026-01-05T09:00:00Z";
const EXPIRES: &str = "2026-01-05T10:00:00Z";
const READING: &str = "2026-01-05T09:30:00Z";
const BEFORE: &str = "2026-01-05T09:59:59Z";
const AFTER: &str = "2026-01-05T10:00:01Z";
const KEPT_UNTIL: &str = "2026-01-06T10:00:00Z";
const MARK_DELIVERED: &str = "inbox.messaging.MarkDelivered";
const MARK_READ: &str = "inbox.messaging.MarkRead";

#[derive(Debug, Clone, Copy)]
struct Accepting;

impl SignatureVerifier for Accepting {
    fn verifies(&self, _envelope: Envelope<'_>) -> bool {
        true
    }

    fn authorizes(&self, _envelope: Envelope<'_>, _authority: &DelegationId) -> bool {
        true
    }
}

/// One command through `inbox_core::handle_with_events` as `actor` with `attributes`, the
/// service's clock at `now`: the whole answer (or the refusal's text) and the events.
fn run_as(
    backend: &MemoryBackend,
    actor: Actor,
    attributes: CallerAttributes,
    now: &str,
    name: &str,
    input: &str,
) -> (Result<Value, String>, Vec<SystemEvent>) {
    let context = ServiceContext::new(
        attributes,
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
    (answer.map_err(|refused| refused.to_string()), events)
}

/// One command as the client `who` (the Sweeper for an empty `who`): the outcome and the events.
fn run(
    backend: &MemoryBackend,
    who: &str,
    now: &str,
    name: &str,
    input: &str,
) -> (String, Vec<SystemEvent>) {
    let (actor, attributes) = if who.is_empty() {
        (Actor::Sweeper, CallerAttributes::none())
    } else {
        (
            Actor::Client,
            CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned())),
        )
    };
    let (answer, events) = run_as(backend, actor, attributes, now, name, input);
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

fn state(backend: &MemoryBackend) -> Option<MessageState> {
    MessageStorage::list(backend).first().map(|held| held.state)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ask {
    Note,
    Request,
    DecisionWithDefault,
}

/// The worker's open inbox and the planner's message `ask` to it, due at `expires`.
fn world(ask: Ask, expires: &str) -> MemoryBackend {
    let backend = MemoryBackend::new();
    let (opened, _) = run(
        &backend,
        WORKER,
        CREATED,
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(opened, "opened");
    let (kind, extra) = match ask {
        Ask::Note => ("note", ""),
        Ask::Request => (
            "request",
            "\"reply_shape\":{\"shape\":\"lines\",\"value\":1},",
        ),
        Ask::DecisionWithDefault => (
            "decision",
            "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"},\
             {\"letter\":\"B\",\"text\":\"Hold.\"}],\"recommended_option\":\"A\"},\
             \"default_on_silence\":\"B\",",
        ),
    };
    let (posted, _) = run(
        &backend,
        PLANNER,
        CREATED,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{ID}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{PLANNER}\",\
             \"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{ID}\",{extra}\
             \"created\":\"{CREATED}\",\"expires\":\"{expires}\",\"dedup_key\":\"{ID}\",\
             \"body\":\"b\",\"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    );
    assert_eq!(posted, "posted", "{ask:?} {expires}");
    backend
}

fn id_input() -> String {
    format!("{{\"message_id\":\"{ID}\"}}")
}

fn sweep(backend: &MemoryBackend, now: &str) -> String {
    run(
        backend,
        "",
        now,
        "inbox.messaging.Expire",
        &format!("{{\"message_id\":\"{ID}\",\"now\":\"{now}\",\"keep_until\":\"{KEPT_UNTIL}\"}}"),
    )
    .0
}

/// `docs/design/service.md` ("The deadline"): "A message already answered or refused keeps the
/// answers `already-read` and `already-delivered`." A request answered, one refused, and a
/// decision the sweep decided on silence, each marked by its recipient at and after its
/// `expires`: the answer of 0.3.0, no event, no row changed.
#[test]
fn a_due_message_answered_or_refused_keeps_already_read_and_already_delivered() {
    let answered = world(Ask::Request, EXPIRES);
    let (taken, _) = run(
        &answered,
        WORKER,
        READING,
        "inbox.messaging.Answer",
        &format!(
            "{{\"in_reply_to\":\"{ID}\",\"reply_id\":\"{REPLY}\",\"from\":\"{WORKER}\",\
             \"on_behalf_of\":\"{WORKER}\",\"created\":\"{READING}\",\
             \"expires\":\"{KEPT_UNTIL}\",\"body\":\"A.\",\
             \"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    );
    assert_eq!(taken, "answered");

    let refused = world(Ask::Request, EXPIRES);
    let (taken, _) = run(
        &refused,
        WORKER,
        READING,
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":\"{ID}\",\"reason\":\"no\"}}"),
    );
    assert_eq!(taken, "refused");

    let silenced = world(Ask::DecisionWithDefault, EXPIRES);
    assert_eq!(sweep(&silenced, EXPIRES), "decided-on-silence");

    for (what, backend, settled) in [
        ("answered", &answered, MessageState::Answered),
        ("refused", &refused, MessageState::Refused),
        ("decided on silence", &silenced, MessageState::Answered),
    ] {
        assert_eq!(state(backend), Some(settled), "{what}");
        for now in [EXPIRES, AFTER] {
            for (name, kept) in [
                (MARK_READ, "already-read"),
                (MARK_DELIVERED, "already-delivered"),
            ] {
                let before = store(backend);
                let (outcome, events) = run(backend, WORKER, now, name, &id_input());
                assert_eq!(outcome, kept, "{name} of a {what} message at {now}");
                assert!(events.is_empty(), "{what} {name}: {events:?}");
                assert_eq!(store(backend), before, "{what} {name} at {now}");
            }
        }
    }
}

/// The message's `expires` in every form RFC 3339 writes one (a fraction, an offset, lower case),
/// against a reading a nanosecond before it, at it and a nanosecond after it: a mark is refused
/// `deadline-passed` exactly where the sweep at that reading takes the message, and taken where
/// the sweep finds it `not-due`. The comparison the two make is the one the story rests on: the
/// sender gets the same event whichever runs first.
#[test]
fn a_mark_is_refused_exactly_where_the_sweep_at_that_reading_takes_the_message() {
    const FORMS: [&str; 4] = [
        "2026-01-05T10:00:00.5Z",
        "2026-01-05T11:00:00.5+01:00",
        "2026-01-05t10:00:00.500000000z",
        "2026-01-05T09:00:00.5-01:00",
    ];
    const READINGS: [&str; 4] = [
        "2026-01-05T10:00:00.499999999Z",
        "2026-01-05T10:00:00.5Z",
        "2026-01-05T10:00:00.500000001Z",
        "2026-01-05T10:00:00Z",
    ];
    for expires in FORMS {
        for now in READINGS {
            for ask in [Ask::Note, Ask::Request] {
                let swept = world(ask, expires);
                let sweep_says = sweep(&swept, now);
                let due = match sweep_says.as_str() {
                    "expired" => true,
                    "not-due" => false,
                    other => panic!("the sweep of {ask:?} {expires} at {now}: {other}"),
                };
                for name in [MARK_DELIVERED, MARK_READ] {
                    let backend = world(ask, expires);
                    let before = store(&backend);
                    let (outcome, events) = run(&backend, WORKER, now, name, &id_input());
                    if due {
                        assert_eq!(
                            outcome, "deadline-passed",
                            "{name} of a {ask:?} due at {expires}, read at {now}"
                        );
                        assert!(events.is_empty(), "{events:?}");
                        assert_eq!(store(&backend), before);
                    } else {
                        assert!(
                            ["delivered", "read", "handled"].contains(&outcome.as_str()),
                            "{name} of a {ask:?} due at {expires}, read at {now}: {outcome}"
                        );
                        assert_eq!(events.len(), 1, "{events:?}");
                    }
                }
            }
        }
    }
}

/// Every caller but the recipient, as an embedding of `inbox-core` may hand it in (both
/// attributes, one of them, none; the Sweeper), gets from each mark the same answer, member for
/// member, a second before the deadline and a second after it, and no event: no answer tells a
/// caller that is not the recipient that the message is due.
#[test]
fn no_caller_but_the_recipient_tells_a_due_message_from_a_live_one() {
    let principal = |who: &str| Some(Principal(who.to_owned()));
    let inbox = |who: &str| Some(Address(who.to_owned()));
    let callers: Vec<(&str, Actor, CallerAttributes)> = vec![
        (
            "the sender",
            Actor::Client,
            CallerAttributes::new(Principal(PLANNER.to_owned()), Address(PLANNER.to_owned())),
        ),
        (
            "a third principal",
            Actor::Client,
            CallerAttributes::new(Principal(THIRD.to_owned()), Address(THIRD.to_owned())),
        ),
        (
            "the sender without its inbox",
            Actor::Client,
            CallerAttributes {
                principal: principal(PLANNER),
                inbox: None,
            },
        ),
        (
            "the sender's inbox without a principal",
            Actor::Client,
            CallerAttributes {
                principal: None,
                inbox: inbox(PLANNER),
            },
        ),
        (
            "a third principal without its inbox",
            Actor::Client,
            CallerAttributes {
                principal: principal(THIRD),
                inbox: None,
            },
        ),
        (
            "the recipient's principal with another inbox",
            Actor::Client,
            CallerAttributes {
                principal: principal(WORKER),
                inbox: inbox(THIRD),
            },
        ),
        (
            "a client without attributes",
            Actor::Client,
            CallerAttributes::none(),
        ),
        ("the Sweeper", Actor::Sweeper, CallerAttributes::none()),
    ];
    for ask in [Ask::Note, Ask::Request] {
        for (who, actor, attributes) in &callers {
            for name in [MARK_DELIVERED, MARK_READ] {
                let answers: Vec<_> = [BEFORE, AFTER]
                    .into_iter()
                    .map(|now| {
                        let backend = world(ask, EXPIRES);
                        let before = store(&backend);
                        let (answer, events) =
                            run_as(&backend, *actor, attributes.clone(), now, name, &id_input());
                        assert!(events.is_empty(), "{who} {name} at {now}: {events:?}");
                        assert_eq!(store(&backend), before, "{who} {name} at {now}");
                        format!("{answer:?}")
                    })
                    .collect();
                assert_eq!(
                    answers[0], answers[1],
                    "{name} of a {ask:?} as {who}: live, then due"
                );
                assert!(
                    !answers[1].contains("deadline-passed") && !answers[1].contains("Expired"),
                    "{who} {name}: {}",
                    answers[1]
                );
            }
        }
    }
}
