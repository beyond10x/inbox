//! Adversary pass 2 on story:a-due-message-takes-no-answer, in the core: `expired-on-arrival` of
//! `Answer` (round 2) at its own boundary in every form an `expires` is written in, its place
//! among the other refusals of `Answer`, a delegate's reply, the sweep at the same readings, and
//! a repeated answer to a message already answered.

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
const OWNER: &str = "human:owner";
const ID: &str = "01JGT0AV000000000000000101";
const MISSING: &str = "01JGT0AV000000000000000199";
const REPLY: &str = "01JGT0AV000000000000000102";
const CREATED: &str = "2026-01-05T09:00:00Z";
const EXPIRES: &str = "2026-01-05T10:00:00Z";
const REPLY_CREATED: &str = "2026-01-05T09:10:00Z";
const PAST: &str = "2026-01-05T09:20:00Z";
const READING: &str = "2026-01-05T09:30:00Z";
const LIVE: &str = "2026-01-05T11:00:00Z";
const KEPT_UNTIL: &str = "2026-01-06T10:00:00Z";

/// A verifier that says `verdict` to every signature and every delegation.
#[derive(Debug, Clone, Copy)]
struct Verifier(bool);

impl SignatureVerifier for Verifier {
    fn verifies(&self, _envelope: Envelope<'_>) -> bool {
        self.0
    }

    fn authorizes(&self, _envelope: Envelope<'_>, _authority: &DelegationId) -> bool {
        self.0
    }
}

/// One command as `who` (the Sweeper for an empty `who`), the service's clock at `now`: the
/// outcome (or the refusal's text) and the events.
fn run_with(
    verifier: Verifier,
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
    let context = ServiceContext::new(caller, verifier, backend.clone(), Timestamp(now.to_owned()));
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

fn run(
    backend: &MemoryBackend,
    who: &str,
    now: &str,
    name: &str,
    input: &str,
) -> (String, Vec<SystemEvent>) {
    run_with(Verifier(true), backend, who, now, name, input)
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
    Request,
    Note,
}

fn envelope(ask: Ask, id: &str, expires: &str) -> String {
    let (kind, extra) = match ask {
        Ask::DecisionWithDefault => (
            "decision",
            "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"},\
             {\"letter\":\"B\",\"text\":\"Hold.\"}],\"recommended_option\":\"A\"},\
             \"default_on_silence\":\"B\",",
        ),
        Ask::Request => (
            "request",
            "\"reply_shape\":{\"shape\":\"lines\",\"value\":1},",
        ),
        Ask::Note => ("note", ""),
    };
    format!(
        "{{\"message_id\":\"{id}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{PLANNER}\",\
         \"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{id}\",{extra}\
         \"created\":\"{CREATED}\",\"expires\":\"{expires}\",\"dedup_key\":\"{id}\",\
         \"body\":\"b\",\"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

/// A reply envelope. `from` speaks for `on_behalf_of`, under `authority` when it names one.
struct Reply<'a> {
    in_reply_to: &'a str,
    from: &'a str,
    on_behalf_of: &'a str,
    authority: Option<&'a str>,
    expires: &'a str,
    option: Option<&'a str>,
    body: &'a str,
}

impl<'a> Reply<'a> {
    /// The worker's own reply to `ID`, created at `REPLY_CREATED`, expiring at `expires`.
    fn new(expires: &'a str) -> Self {
        Self {
            in_reply_to: ID,
            from: WORKER,
            on_behalf_of: WORKER,
            authority: None,
            expires,
            option: None,
            body: "A.",
        }
    }

    fn json(&self) -> String {
        let authority = self
            .authority
            .map(|id| format!("\"authority\":\"{id}\","))
            .unwrap_or_default();
        let option = self
            .option
            .map(|letter| format!("\"option\":\"{letter}\","))
            .unwrap_or_default();
        format!(
            "{{\"in_reply_to\":\"{}\",\"reply_id\":\"{REPLY}\",\"from\":\"{}\",\
             \"on_behalf_of\":\"{}\",{authority}\"created\":\"{REPLY_CREATED}\",\
             \"expires\":\"{}\",{option}\"body\":\"{}\",\
             \"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}",
            self.in_reply_to, self.from, self.on_behalf_of, self.expires, self.body
        )
    }
}

/// The worker's open inbox and the planner's message `ask` to it, due at `expires`, moved to
/// `state` at `CREATED`.
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
        &envelope(ask, ID, expires),
    );
    assert_eq!(posted, "posted", "{ask:?} {expires}");
    let id = format!("{{\"message_id\":\"{ID}\"}}");
    if matches!(state, MessageState::Delivered | MessageState::Read) {
        let (delivered, _) = run(
            &backend,
            WORKER,
            CREATED,
            "inbox.messaging.MarkDelivered",
            &id,
        );
        assert_eq!(delivered, "delivered");
    }
    if state == MessageState::Read {
        let (read, _) = run(&backend, WORKER, CREATED, "inbox.messaging.MarkRead", &id);
        assert_eq!(read, "read");
    }
    backend
}

fn answer(
    backend: &MemoryBackend,
    who: &str,
    now: &str,
    reply: &Reply<'_>,
) -> (String, Vec<SystemEvent>) {
    run(backend, who, now, "inbox.messaging.Answer", &reply.json())
}

fn sweep(backend: &MemoryBackend, id: &str, now: &str) -> String {
    run(
        backend,
        "",
        now,
        "inbox.messaging.Expire",
        &format!("{{\"message_id\":\"{id}\",\"now\":\"{now}\",\"keep_until\":\"{KEPT_UNTIL}\"}}"),
    )
    .0
}

/// The reply's own `expires`, written with a fraction of 1 to 9 digits, an offset or in lower
/// case, against a reading equal to it, one nanosecond before and after it. Each reading is 30
/// minutes before the message's own deadline, so the message is open.
const BOUNDARY: [(&str, &str, &str); 17] = [
    (
        "2026-01-05T09:30:00Z",
        "2026-01-05T09:30:00Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T09:30:00Z",
        "2026-01-05T09:29:59.999999999Z",
        "answered",
    ),
    (
        "2026-01-05T09:30:00Z",
        "2026-01-05T09:30:00.000000001Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T09:30:00.5Z",
        "2026-01-05T09:30:00.500000000Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T09:30:00.5Z",
        "2026-01-05T09:30:00.499999999Z",
        "answered",
    ),
    (
        "2026-01-05T09:30:00.50Z",
        "2026-01-05T09:30:00.5Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T09:30:00.123456789Z",
        "2026-01-05T09:30:00.123456789Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T09:30:00.123456789Z",
        "2026-01-05T09:30:00.123456788Z",
        "answered",
    ),
    (
        "2026-01-05T09:30:00.000000001Z",
        "2026-01-05T09:30:00Z",
        "answered",
    ),
    (
        "2026-01-05T09:30:00.000000001Z",
        "2026-01-05T09:30:00.000000001Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T11:30:00+02:00",
        "2026-01-05T09:30:00Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T11:30:00+02:00",
        "2026-01-05T09:29:59.9Z",
        "answered",
    ),
    (
        "2026-01-05T09:00:00-00:30",
        "2026-01-05T09:30:00Z",
        "expired-on-arrival",
    ),
    (
        "2026-01-05T09:00:00-00:30",
        "2026-01-05T09:29:59.999999999Z",
        "answered",
    ),
    (
        "2026-01-05t09:30:00z",
        "2026-01-05T09:30:00Z",
        "expired-on-arrival",
    ),
    ("2026-01-05t09:30:00z", "2026-01-05T09:29:59Z", "answered"),
    (
        "2026-01-05T09:30:00.7Z",
        "2026-01-05T11:30:00.7+02:00",
        "expired-on-arrival",
    ),
];

/// `Answer` refuses a reply at its own `expires` exactly, at the clock's full precision, and
/// takes it one nanosecond before; a refused reply changes no row and publishes nothing.
#[test]
fn a_reply_is_refused_at_its_own_expires_exactly_and_taken_a_nanosecond_before() {
    for (expires, now, expected) in BOUNDARY {
        let backend = world(Ask::Request, EXPIRES, MessageState::Read);
        let before = store(&backend);
        let (outcome, events) = answer(&backend, WORKER, now, &Reply::new(expires));
        assert_eq!(outcome, expected, "reply expires {expires}, reading {now}");
        if expected == "expired-on-arrival" {
            assert!(events.is_empty(), "{expires} at {now}: {events:?}");
            assert_eq!(store(&backend), before, "{expires} at {now}");
        }
    }
}

/// The reply's own `expires` against the sweep at the same reading: a reply `Answer` takes is
/// not due for `Expire` at that reading, and a reply it refuses would be. A message stored with
/// the reply's `expires` stands in for the stored answer the refused reply would have made.
#[test]
fn answer_takes_a_reply_exactly_where_the_sweep_at_that_reading_keeps_it() {
    for (expires, now, _) in BOUNDARY {
        let answered = world(Ask::Request, EXPIRES, MessageState::Read);
        let (outcome, _) = answer(&answered, WORKER, now, &Reply::new(expires));
        let stand_in = MemoryBackend::new();
        run(
            &stand_in,
            WORKER,
            CREATED,
            "inbox.messaging.OpenInbox",
            &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
        );
        let (posted, _) = run(
            &stand_in,
            PLANNER,
            CREATED,
            "inbox.messaging.PostMessage",
            &envelope(Ask::Request, ID, expires),
        );
        assert_eq!(posted, "posted", "{expires}");
        let due = sweep(&stand_in, ID, now);
        match outcome.as_str() {
            "answered" => {
                assert_eq!(sweep(&answered, ID, now), "not-due", "{expires} at {now}");
                assert_eq!(due, "not-due", "{expires} at {now}");
            }
            "expired-on-arrival" => assert_eq!(due, "expired", "{expires} at {now}"),
            other => panic!("{expires} at {now}: {other}"),
        }
    }
}

/// Where a refusal that tells the caller about the message, or the caller's own envelope, holds
/// beside an expired reply, `Answer` answers that refusal. A third principal gets the very
/// answer an id no message carries gets. A message the clock has reached answers
/// `deadline-passed` (its own deadline first). `shape-mismatch` comes after.
#[test]
fn the_expired_reply_answers_after_the_party_checks_and_before_the_shape() {
    let expired = Reply::new(PAST);

    let backend = world(Ask::Request, EXPIRES, MessageState::Read);
    let third = Reply {
        from: THIRD,
        on_behalf_of: THIRD,
        ..Reply::new(PAST)
    };
    let missing = Reply {
        in_reply_to: MISSING,
        ..Reply::new(PAST)
    };
    let missing_third = Reply {
        in_reply_to: MISSING,
        ..third
    };
    let third_to_missing = Reply {
        from: THIRD,
        on_behalf_of: THIRD,
        ..missing_third
    };
    assert_eq!(
        answer(&backend, THIRD, READING, &third).0,
        "unknown-message"
    );
    assert_eq!(
        answer(&backend, THIRD, READING, &third_to_missing).0,
        "unknown-message"
    );
    assert_eq!(
        answer(&backend, WORKER, READING, &missing).0,
        "unknown-message"
    );
    let sender = Reply {
        from: PLANNER,
        on_behalf_of: PLANNER,
        ..Reply::new(PAST)
    };
    assert_eq!(
        answer(&backend, PLANNER, READING, &sender).0,
        "not-recipient"
    );
    assert_eq!(answer(&backend, WORKER, READING, &sender).0, "not-sender");
    assert_eq!(
        run_with(
            Verifier(false),
            &backend,
            WORKER,
            READING,
            "inbox.messaging.Answer",
            &expired.json()
        )
        .0,
        "signature-invalid"
    );
    let unbacked = Reply {
        on_behalf_of: OWNER,
        ..Reply::new(PAST)
    };
    assert_eq!(
        answer(&backend, WORKER, READING, &unbacked).0,
        "no-authority"
    );
    let backwards = Reply::new("2026-01-05T09:05:00Z");
    assert_eq!(
        answer(&backend, WORKER, READING, &backwards).0,
        "already-expired"
    );
    let two_lines = Reply {
        body: "A.\\nB.",
        ..Reply::new(PAST)
    };
    assert_eq!(
        answer(&backend, WORKER, READING, &two_lines).0,
        "expired-on-arrival"
    );
    let live_two_lines = Reply {
        body: "A.\\nB.",
        ..Reply::new(LIVE)
    };
    assert_eq!(
        answer(&backend, WORKER, READING, &live_two_lines).0,
        "shape-mismatch"
    );
    assert_eq!(
        answer(&backend, WORKER, EXPIRES, &expired).0,
        "deadline-passed"
    );

    let note = world(Ask::Note, EXPIRES, MessageState::Delivered);
    assert_eq!(answer(&note, WORKER, READING, &expired).0, "note");
}

/// A delegate's reply is held to its own `expires` as the principal's own is: refused at it,
/// taken before it, and the one taken is stored under the delegation.
#[test]
fn a_delegates_reply_is_held_to_its_own_expires() {
    let delegated = |expires| Reply {
        on_behalf_of: OWNER,
        authority: Some("delegation-1"),
        ..Reply::new(expires)
    };
    let backend = world(Ask::Request, EXPIRES, MessageState::Read);
    let before = store(&backend);
    let (late, events) = answer(&backend, WORKER, READING, &delegated(READING));
    assert_eq!(late, "expired-on-arrival");
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(store(&backend), before);
    let (taken, _) = answer(&backend, WORKER, READING, &delegated(LIVE));
    assert_eq!(taken, "answered");
    let held = MessageStorage::list(&backend);
    assert_eq!(held[0].data.expires.0, LIVE);
    let reply = held[0].data.reply.as_ref().expect("the reply is stored");
    assert_eq!(reply.on_behalf_of.0, OWNER);
    assert_eq!(
        reply.authority.as_ref().map(|id| id.0.as_str()),
        Some("delegation-1")
    );
}

/// A decision whose silence takes B: a reply already expired is refused before the deadline,
/// the sweep at the deadline decides it on silence, and the sender collects B, taken on silence.
#[test]
fn an_expired_reply_to_a_decision_leaves_it_to_silence_at_the_deadline() {
    let backend = world(Ask::DecisionWithDefault, EXPIRES, MessageState::Read);
    let reply = Reply {
        option: Some("A"),
        ..Reply::new(PAST)
    };
    assert_eq!(
        answer(&backend, WORKER, READING, &reply).0,
        "expired-on-arrival"
    );
    assert_eq!(sweep(&backend, ID, READING), "not-due");
    assert_eq!(sweep(&backend, ID, EXPIRES), "decided-on-silence");
    let held = MessageStorage::list(&backend);
    assert_eq!(held.len(), 1);
    assert_eq!(format!("{:?}", held[0].data.answered_by), "Some(Silence)");
    assert_eq!(
        held[0]
            .data
            .decided_option
            .as_ref()
            .map(|letter| letter.0.as_str()),
        Some("B")
    );
    assert!(held[0].data.reply.is_none(), "{held:?}");
    let (collected, _) = run(
        &backend,
        PLANNER,
        EXPIRES,
        "inbox.messaging.Collect",
        &format!("{{\"message_id\":\"{ID}\"}}"),
    );
    assert_eq!(collected, "collected");
}

/// `docs/design/service.md` ("a repeated `Answer` is `already-settled`") and
/// `website/docs/limits.md`: the service keeps no record of requests, and a client that sends
/// its answer again learns from `already-settled` that the first one was taken. The same answer
/// sent again after its own `expires` and before the sweep removes it is answered
/// `expired-on-arrival` (`AlreadyExpired`, 502 over HTTP), which says the answer was not stored,
/// although it is. A message in `Answered` or `Refused` is not open, as `deadline-passed` reads it.
#[test]
fn a_repeated_answer_to_an_answered_message_is_already_settled_after_its_own_expires() {
    let backend = world(Ask::Request, EXPIRES, MessageState::Read);
    let reply = Reply::new("2026-01-05T09:40:00Z");
    assert_eq!(answer(&backend, WORKER, READING, &reply).0, "answered");
    let again = "2026-01-05T09:40:00.5Z";
    let (repeated, events) = answer(&backend, WORKER, again, &reply);
    assert!(events.is_empty(), "{events:?}");
    assert_eq!(
        repeated,
        "already-settled",
        "the stored answer is still there until the sweep: {}",
        store(&backend)
    );
}
