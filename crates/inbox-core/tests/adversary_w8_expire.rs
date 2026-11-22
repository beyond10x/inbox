//! Adversary cases of story:expire-publishes-once (wave 8, pass 1): the states the 9 cases of
//! `expire_once.rs` never build. There, a message's thread is always its own id, its sender is
//! always the principal it speaks for, `now` is always exactly `expires`, and the store holds one
//! message. A core that is wrong only off those points passes them.

use inbox::actor::{Actor, Caller};
use inbox::messaging::{
    Address, DecisionTakenOnSilence, MessageExpired, MessageId, Principal, ThreadId,
};
use inbox::server::json::{self, Value};
use inbox::system::SystemEvent;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const OPERATOR: &str = "human:operator";
const ID: &str = "01JGT0AD000000000000000001";
const ID2: &str = "01JGT0AD000000000000000002";
const ID3: &str = "01JGT0AD000000000000000003";
const ID4: &str = "01JGT0AD000000000000000004";
const THREAD: &str = "01JGT0AD0000000000000000T1";
const EXPIRES: &str = "2026-01-05T10:00:00Z";
const LATE: &str = "2026-01-05T12:34:56Z";
const KEPT_UNTIL: &str = "2026-01-06T10:00:00Z";

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
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(outcome(&opened), "opened");
    backend
}

struct Sent<'a> {
    id: &'a str,
    thread: &'a str,
    on_behalf_of: &'a str,
    authority: &'a str,
    kind: &'a str,
    default_on_silence: bool,
}

impl<'a> Sent<'a> {
    fn decision(id: &'a str) -> Self {
        Sent {
            id,
            thread: id,
            on_behalf_of: PLANNER,
            authority: "",
            kind: "decision",
            default_on_silence: true,
        }
    }

    fn note(id: &'a str) -> Self {
        Sent {
            kind: "note",
            default_on_silence: false,
            ..Sent::decision(id)
        }
    }

    fn post(&self, backend: &MemoryBackend) {
        let extra = match self.kind {
            "decision" => format!(
                "\"decision\":{{\"options\":[{{\"letter\":\"A\",\"text\":\"Ship.\"}},\
                 {{\"letter\":\"B\",\"text\":\"Hold.\"}}],\"recommended_option\":\"B\"}},{}",
                if self.default_on_silence {
                    "\"default_on_silence\":\"B\","
                } else {
                    ""
                }
            ),
            _ => String::new(),
        };
        let authority = if self.authority.is_empty() {
            String::new()
        } else {
            format!("\"authority\":\"{}\",", self.authority)
        };
        let input = format!(
            "{{\"message_id\":\"{id}\",\"from\":\"{PLANNER}\",\"on_behalf_of\":\"{behalf}\",\
             {authority}\"to\":\"{WORKER}\",\"kind\":\"{kind}\",\"thread\":\"{thread}\",{extra}\
             \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"{EXPIRES}\",\
             \"dedup_key\":\"dedup-{id}\",\"body\":\"b\",\
             \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}",
            id = self.id,
            behalf = self.on_behalf_of,
            kind = self.kind,
            thread = self.thread,
        );
        let (posted, events) = run(
            backend,
            Actor::Client,
            PLANNER,
            "inbox.messaging.PostMessage",
            &input,
        );
        assert_eq!(outcome(&posted), "posted", "{posted:?}");
        assert_eq!(events.len(), 1, "{events:?}");
    }
}

fn expire(backend: &MemoryBackend, id: &str, now: &str) -> (String, Vec<SystemEvent>) {
    let (answer, events) = run(
        backend,
        Actor::Sweeper,
        "",
        "inbox.messaging.Expire",
        &format!("{{\"message_id\":\"{id}\",\"now\":\"{now}\",\"keep_until\":\"{KEPT_UNTIL}\"}}"),
    );
    (outcome(&answer).to_owned(), events)
}

fn expired(id: &str) -> Vec<SystemEvent> {
    expired_to(id, PLANNER, id)
}

fn expired_to(id: &str, to: &str, thread: &str) -> Vec<SystemEvent> {
    vec![SystemEvent::MessageExpired(MessageExpired {
        message_id: MessageId(id.to_owned()),
        to: Principal(to.to_owned()),
        thread: ThreadId(thread.to_owned()),
    })]
}

fn silence(id: &str, to: &str, thread: &str) -> Vec<SystemEvent> {
    vec![SystemEvent::DecisionTakenOnSilence(
        DecisionTakenOnSilence {
            message_id: MessageId(id.to_owned()),
            to: Principal(to.to_owned()),
            thread: ThreadId(thread.to_owned()),
        },
    )]
}

#[test]
fn decision_taken_on_silence_names_the_thread_of_the_message_not_its_id() {
    let backend = world();
    Sent {
        thread: THREAD,
        ..Sent::decision(ID)
    }
    .post(&backend);
    let (taken, events) = expire(&backend, ID, EXPIRES);
    assert_eq!(taken, "decided-on-silence");
    assert_eq!(events, silence(ID, PLANNER, THREAD));
}

#[test]
fn decision_taken_on_silence_goes_to_the_sender_not_the_principal_it_spoke_for() {
    let backend = world();
    Sent {
        on_behalf_of: OPERATOR,
        authority: "delegation-7",
        ..Sent::decision(ID)
    }
    .post(&backend);
    let (taken, events) = expire(&backend, ID, EXPIRES);
    assert_eq!(taken, "decided-on-silence");
    assert_eq!(events, silence(ID, PLANNER, ID));
}

#[test]
fn message_expired_names_the_thread_of_the_message_not_its_id() {
    let backend = world();
    Sent {
        thread: THREAD,
        ..Sent::note(ID)
    }
    .post(&backend);
    let (taken, events) = expire(&backend, ID, EXPIRES);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired_to(ID, PLANNER, THREAD));
}

#[test]
fn message_expired_goes_to_the_sender_not_the_principal_it_spoke_for_nor_the_recipient() {
    let backend = world();
    Sent {
        on_behalf_of: OPERATOR,
        authority: "delegation-7",
        ..Sent::note(ID)
    }
    .post(&backend);
    let (taken, events) = expire(&backend, ID, EXPIRES);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired_to(ID, PLANNER, ID));
}

#[test]
fn a_late_sweep_publishes_message_expired_once() {
    let backend = world();
    Sent::note(ID).post(&backend);
    let (taken, events) = expire(&backend, ID, LATE);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired(ID));
}

#[test]
fn a_late_sweep_publishes_decision_taken_on_silence_once() {
    let backend = world();
    Sent::decision(ID).post(&backend);
    let (taken, events) = expire(&backend, ID, LATE);
    assert_eq!(taken, "decided-on-silence");
    assert_eq!(events, silence(ID, PLANNER, ID));
}

#[test]
fn expiring_one_of_four_messages_publishes_only_its_own_event_once() {
    let backend = world();
    Sent::note(ID).post(&backend);
    Sent::decision(ID2).post(&backend);
    Sent::note(ID3).post(&backend);
    Sent::decision(ID4).post(&backend);

    let (taken, events) = expire(&backend, ID3, EXPIRES);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired(ID3));

    let (taken, events) = expire(&backend, ID2, EXPIRES);
    assert_eq!(taken, "decided-on-silence");
    assert_eq!(events, silence(ID2, PLANNER, ID2));

    let (taken, events) = expire(&backend, ID, LATE);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired(ID));

    let (taken, events) = expire(&backend, ID4, LATE);
    assert_eq!(taken, "decided-on-silence");
    assert_eq!(events, silence(ID4, PLANNER, ID4));

    let (taken, events) = expire(&backend, ID3, LATE);
    assert_eq!(taken, "unknown-message");
    assert_eq!(events, Vec::<SystemEvent>::new());
}

#[test]
fn a_refused_decision_with_a_silence_default_publishes_message_expired_once() {
    let backend = world();
    Sent::decision(ID).post(&backend);
    let (refused, _) = run(
        &backend,
        Actor::Client,
        WORKER,
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":\"{ID}\",\"reason\":\"no\"}}"),
    );
    assert_eq!(outcome(&refused), "refused", "{refused:?}");
    let (taken, events) = expire(&backend, ID, EXPIRES);
    assert_eq!(taken, "expired");
    assert_eq!(events, expired(ID));
}
