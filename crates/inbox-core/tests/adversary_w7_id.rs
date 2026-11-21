//! Adversary cases of stage 2, wave 7, story:post-does-not-tell-a-message-id-exists, against the
//! core. Each case names what it holds and the change of `fn post_message` it fails on.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
/// The delegate: it sends, as `from`, the word of [`SPOKEN_FOR`].
const DELEGATE: &str = "agent:ws/planner";
/// The principal the delegate speaks for (`on_behalf_of`).
const SPOKEN_FOR: &str = "agent:box/principal";
const HELD: &str = "01JGT0AD000000000000000001";

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
    let context = ServiceContext::new(caller, Accepting, backend.clone());
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

fn stored(backend: &MemoryBackend) -> Vec<String> {
    let mut held: Vec<String> = MessageStorage::list(backend)
        .iter()
        .map(|row| format!("{row:?}"))
        .collect();
    held.sort();
    held
}

fn open(backend: &MemoryBackend, who: &str) {
    let input = format!("{{\"address\":\"{who}\",\"adapter\":\"claude_code\"}}");
    let answer = as_client(backend, who, "inbox.messaging.OpenInbox", &input);
    assert_eq!(
        text(&answer, "outcome").as_deref(),
        Some("opened"),
        "{answer:?}"
    );
}

/// A note `id` from `from`, spoken for `on_behalf_of` under `authority` when given, to `to`
/// under the dedup key `key`; sent as `from`. `(outcome, error)`.
fn post(
    backend: &MemoryBackend,
    id: &str,
    from: &str,
    on_behalf_of: &str,
    authority: Option<&str>,
    to: &str,
    key: &str,
) -> (String, Option<String>) {
    let authority = authority
        .map(|authority| format!("\"authority\":\"{authority}\","))
        .unwrap_or_default();
    let answer = as_client(
        backend,
        from,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{id}\",\"from\":\"{from}\",\"on_behalf_of\":\"{on_behalf_of}\",\
             {authority}\"to\":\"{to}\",\"kind\":\"note\",\"thread\":\"{id}\",\
             \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
             \"dedup_key\":\"{key}\",\"body\":\"b\",\
             \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    );
    (
        text(&answer, "outcome").expect("an outcome"),
        text(&answer, "error"),
    )
}

fn refusal(outcome: &str, error: &str) -> (String, Option<String>) {
    (outcome.to_owned(), Some(format!("inbox.messaging.{error}")))
}

/// A delegate's message belongs to its sender, the `from` (`docs/design/inbox.md`, decision 6:
/// "`duplicate` answers only the sender's own stored message"; the model's `duplicate` selects
/// `from == input.from`). The principal it spoke for, posting under that id as itself, is another
/// sender: `id-taken`. The delegate's repost is `duplicate`, spoken for the same principal or for
/// itself. No case of the suite posts a delegated message twice, so a `duplicate` decided by
/// `on_behalf_of` instead of `from` stays green everywhere else.
#[test]
fn adv_w7_a_delegated_message_id_is_its_senders_not_the_spoken_for_principals() {
    let backend = MemoryBackend::new();
    open(&backend, WORKER);
    let sent = post(
        &backend,
        HELD,
        DELEGATE,
        SPOKEN_FOR,
        Some("delegation-7"),
        WORKER,
        "delegate-key",
    );
    assert_eq!(sent, ("posted".to_owned(), None));
    let before = stored(&backend);

    let spoken_for = post(
        &backend, HELD, SPOKEN_FOR, SPOKEN_FOR, None, WORKER, "own-key",
    );
    let repost = post(
        &backend,
        HELD,
        DELEGATE,
        SPOKEN_FOR,
        Some("delegation-7"),
        WORKER,
        "delegate-key",
    );
    let for_itself = post(
        &backend,
        HELD,
        DELEGATE,
        DELEGATE,
        None,
        WORKER,
        "another-key",
    );
    assert_eq!(stored(&backend), before, "a refused post changed the store");
    assert_eq!(
        (spoken_for, repost, for_itself),
        (
            refusal("id-taken", "MessageIdTaken"),
            refusal("duplicate", "Duplicate"),
            refusal("duplicate", "Duplicate"),
        ),
        "(the spoken-for principal as itself, the delegate's repost, the delegate for itself)"
    );
}
