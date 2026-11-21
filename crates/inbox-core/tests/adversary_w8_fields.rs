//! Adversary cases for story:silence-leaves-the-other-fields-scenario (wave 8, pass 1): what the
//! authored scenario `silence-leaves-the-decision-as-it-was` leaves unclaimed. A decision with
//! every envelope field set is decided on silence from each of its three open states, and the
//! stored row is held to the row posted: every member `Expire` does not set stays as it was.
//! Run against a core whose `Expire` changes `on_behalf_of`, `to`, `kind`, `reply_shape`,
//! `dedup_key`, the signature, the options or the recommended option when it decides on silence,
//! or one of the 5 members the scenario claims only when the decision was Delivered or Read, each
//! case fails.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{Address, DelegationId, MessageData, MessageId, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{
    CallerAttributes, Envelope, Exclusive, MemoryBackend, ServiceContext, SignatureVerifier,
};

const WORKER: &str = "agent:box/worker";
const RELAY: &str = "agent:ws/relay";
const MESSAGE: &str = "01JGT0W8000000000000000001";
const THREAD: &str = "01JGT0W8000000000000000009";

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

fn run<B>(
    backend: &B,
    who: Option<&str>,
    actor: Actor,
    name: &str,
    input: &str,
) -> Result<Value, Refused>
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
{
    let caller = match who {
        Some(who) => CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned())),
        None => CallerAttributes::none(),
    };
    let context = ServiceContext::new(caller, Accepting, backend.clone());
    inbox_core::handle(
        backend,
        context,
        Some(&Caller { actor }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
}

fn outcome(answer: &Value) -> String {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn as_client(backend: &MemoryBackend, who: &str, name: &str, input: &str) -> String {
    outcome(
        &run(backend, Some(who), Actor::Client, name, input)
            .unwrap_or_else(|refused| panic!("{name} as {who} refused: {refused}")),
    )
}

/// A decision from a relay for the principal of the operator, every optional member set, the
/// silence option `A` and the recommended option `B` (so that neither can stand for the other).
fn decision() -> String {
    format!(
        "{{\"message_id\":\"{MESSAGE}\",\"from\":\"{RELAY}\",\
         \"on_behalf_of\":\"human:operator\",\"authority\":\"delegation-7\",\
         \"to\":\"{WORKER}\",\"kind\":\"decision\",\"thread\":\"{THREAD}\",\
         \"reply_shape\":{{\"shape\":\"lines\",\"value\":2}},\
         \"decision\":{{\"options\":[{{\"letter\":\"A\",\"text\":\"Ship.\"}},\
         {{\"letter\":\"B\",\"text\":\"Hold.\"}},{{\"letter\":\"C\",\"text\":\"Cancel.\"}}],\
         \"recommended_option\":\"B\",\"request_ref\":\"req-9\"}},\
         \"default_on_silence\":\"A\",\"created\":\"2026-01-05T08:59:00Z\",\
         \"expires\":\"2026-01-05T10:00:00Z\",\"dedup_key\":\"logical-1\",\
         \"body\":\"Ship  or hold? Line two.\\n \",\
         \"signature\":{{\"key_id\":\"key-relay\",\"value\":\"c2lnLXJlbGF5\"}}}}"
    )
}

/// Posts the decision, moves it into `steps` (`MarkDelivered`, `MarkRead`), decides it on silence
/// and returns the stored row before and after.
fn decided_from(steps: &[&str]) -> (MessageData, MessageData) {
    let backend = MemoryBackend::new();
    assert_eq!(
        as_client(
            &backend,
            WORKER,
            "inbox.messaging.OpenInbox",
            &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}")
        ),
        "opened"
    );
    assert_eq!(
        as_client(&backend, RELAY, "inbox.messaging.PostMessage", &decision()),
        "posted"
    );
    for step in steps {
        as_client(
            &backend,
            WORKER,
            &format!("inbox.messaging.{step}"),
            &format!("{{\"message_id\":\"{MESSAGE}\"}}"),
        );
    }
    let id = MessageId(MESSAGE.to_owned());
    let before = MessageStorage::get(&backend, &id).expect("stored").data;
    let answer = run(
        &backend,
        None,
        Actor::Sweeper,
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":\"{MESSAGE}\",\"now\":\"2026-01-05T10:00:00Z\",\
             \"keep_until\":\"2026-01-06T10:00:00Z\"}}"
        ),
    )
    .expect("the sweeper expires");
    assert_eq!(outcome(&answer), "decided-on-silence");
    let after = MessageStorage::get(&backend, &id).expect("kept").data;
    (before, after)
}

/// Every member `Expire` does not set is the member posted; the 3 it sets are what the contract
/// says (`expires` is `keep_until`, `answered_by` silence, `decided_option` the silence option).
fn holds_the_other_fields(before: MessageData, after: MessageData) {
    let mut expected = before.clone();
    expected.expires = inbox::primitives::Timestamp("2026-01-06T10:00:00Z".to_owned());
    expected.answered_by = Some(inbox::messaging::AnsweredBy::Silence);
    expected.decided_option = before.default_on_silence.clone();
    assert_eq!(after, expected);
    assert_eq!(
        after
            .decided_option
            .as_ref()
            .map(|letter| letter.0.as_str()),
        Some("A"),
        "silence takes the silence option, not the recommended one"
    );
}

#[test]
fn a_decision_decided_on_silence_from_sent_keeps_every_member_expire_does_not_set() {
    let (before, after) = decided_from(&[]);
    holds_the_other_fields(before, after);
}

#[test]
fn a_decision_decided_on_silence_from_delivered_keeps_every_member_expire_does_not_set() {
    let (before, after) = decided_from(&["MarkDelivered"]);
    holds_the_other_fields(before, after);
}

#[test]
fn a_decision_decided_on_silence_from_read_keeps_every_member_expire_does_not_set() {
    let (before, after) = decided_from(&["MarkDelivered", "MarkRead"]);
    holds_the_other_fields(before, after);
}
