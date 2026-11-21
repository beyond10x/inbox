//! Adversary cases for story:dedup-key-scope (stage 2, wave 7), against the core.
//!
//! "The sender" of decision 6 of `docs/design/inbox.md` is the post's `from`, which `not-sender`
//! holds to the caller. These cases state who shares a key space when a post names another
//! principal in `on_behalf_of`: a delegate and the principal it speaks for do not, two delegates
//! of one principal do not, and one delegate does whoever it speaks for. A key scoped by
//! `on_behalf_of` (the option the host operator declined) answers each of them differently.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const THIRD: &str = "agent:box/other";
const OPERATOR: &str = "human:operator";
const OWNER: &str = "human:owner";
const RELAY: &str = "agent:ws/host-operator";
const RELAY_TWO: &str = "agent:ws/relay";
const KEY: &str = "deploy-2026-11-21";

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
    .unwrap_or_else(|refused| panic!("{name}: {refused}"))
}

fn outcome(answer: &Value) -> String {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn world() -> MemoryBackend {
    let backend = MemoryBackend::new();
    for who in [WORKER, THIRD] {
        let opened = as_client(
            &backend,
            who,
            "inbox.messaging.OpenInbox",
            &format!("{{\"address\":\"{who}\",\"adapter\":\"claude_code\"}}"),
        );
        assert_eq!(outcome(&opened), "opened", "{who}");
    }
    backend
}

fn id(n: u32) -> String {
    format!("01JGT0AW{n:018}")
}

/// A note from `from` (the caller) on behalf of `on_behalf_of` to `to` under `key`; a delegation
/// id is named when the two principals differ.
fn post(
    backend: &MemoryBackend,
    n: u32,
    from: &str,
    on_behalf_of: &str,
    to: &str,
    key: &str,
) -> Value {
    let id = id(n);
    let authority = if from == on_behalf_of {
        String::new()
    } else {
        "\"authority\":\"delegation-7\",".to_owned()
    };
    as_client(
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
    )
}

fn stored(backend: &MemoryBackend) -> usize {
    MessageStorage::list(backend).len()
}

#[test]
fn adv_w7_a_delegates_key_refuses_no_post_of_the_principal_it_speaks_for() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, RELAY, OPERATOR, WORKER, KEY)),
        "posted"
    );
    // The operator posts as itself under the key its delegate used for it.
    let answer = post(&backend, 2, OPERATOR, OPERATOR, WORKER, KEY);
    assert_eq!(outcome(&answer), "posted", "{answer:?}");
    assert_eq!(stored(&backend), 2);
}

#[test]
fn adv_w7_the_principals_key_refuses_no_post_of_its_delegate() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, OPERATOR, OPERATOR, THIRD, KEY)),
        "posted"
    );
    // The delegate learns nothing of the operator's key and is not blocked by it.
    let answer = post(&backend, 2, RELAY, OPERATOR, WORKER, KEY);
    assert_eq!(outcome(&answer), "posted", "{answer:?}");
    assert_eq!(stored(&backend), 2);
}

#[test]
fn adv_w7_two_delegates_of_one_principal_do_not_share_a_key() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, RELAY, OPERATOR, WORKER, KEY)),
        "posted"
    );
    let answer = post(&backend, 2, RELAY_TWO, OPERATOR, WORKER, KEY);
    assert_eq!(outcome(&answer), "posted", "{answer:?}");
    assert_eq!(stored(&backend), 2);
}

/// The scope is `from`: one delegate is refused its own key a second time, whoever it speaks for.
#[test]
fn adv_w7_a_delegate_is_refused_its_own_key_for_another_principal() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, RELAY, OPERATOR, WORKER, KEY)),
        "posted"
    );
    let answer = post(&backend, 2, RELAY, OWNER, WORKER, KEY);
    assert_eq!(outcome(&answer), "duplicate-key", "{answer:?}");
    assert_eq!(stored(&backend), 1);
}

/// What a caller learns: the whole answer of a post into its own inbox under a key another sender
/// holds is the answer under a key nobody holds, its own message id aside.
#[test]
fn adv_w7_a_key_another_sender_holds_answers_as_a_free_key() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, WORKER, WORKER, THIRD, KEY)),
        "posted"
    );
    let held = post(&backend, 2, THIRD, THIRD, THIRD, KEY);
    let free = post(&backend, 3, THIRD, THIRD, THIRD, "nobody-holds-this");
    let held = format!("{held:?}").replace(&id(2), "ID");
    let free = format!("{free:?}").replace(&id(3), "ID");
    assert_eq!(held, free);
}

/// The empty string is a key like any other: one sender's refuses no other sender's, and refuses
/// its own sender a second time.
#[test]
fn adv_w7_the_empty_key_is_the_senders_too() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, THIRD, THIRD, THIRD, "")),
        "posted"
    );
    assert_eq!(
        outcome(&post(&backend, 2, WORKER, WORKER, THIRD, "")),
        "posted"
    );
    assert_eq!(
        outcome(&post(&backend, 3, WORKER, WORKER, WORKER, "")),
        "duplicate-key"
    );
    assert_eq!(stored(&backend), 2);
}

// ---- Second adversary agent: the life of a key, the key value, the order. ----

/// A command as `actor` with `caller`, answered or refused as `handle` answers it.
fn try_run(
    backend: &MemoryBackend,
    actor: Actor,
    caller: CallerAttributes,
    name: &str,
    input: &str,
) -> Result<Value, String> {
    let context = ServiceContext::new(caller, Accepting, backend.clone());
    inbox_core::handle(
        backend,
        context,
        Some(&Caller { actor }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
    .map_err(|refused| refused.to_string())
}

fn client(who: &str) -> CallerAttributes {
    CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()))
}

fn sweep(backend: &MemoryBackend, n: u32, now: &str, keep_until: &str) -> Value {
    try_run(
        backend,
        Actor::Sweeper,
        CallerAttributes::none(),
        "inbox.messaging.Expire",
        &format!(
            "{{\"message_id\":\"{}\",\"now\":\"{now}\",\"keep_until\":\"{keep_until}\"}}",
            id(n)
        ),
    )
    .expect("the sweeper's Expire is answered")
}

/// A post as WORKER of `kind` (`note` or `decision`) to `to` under `key`; `key` is written as
/// JSON text, so a case can send any spelling.
fn post_kind(backend: &MemoryBackend, n: u32, kind: &str, to: &str, key: &str) -> Value {
    let id = id(n);
    let decision = if kind == "decision" {
        "\"decision\":{\"options\":[{\"letter\":\"A\",\"text\":\"a\"},{\"letter\":\"B\",\"text\":\"b\"}]},\
         \"default_on_silence\":\"A\","
    } else {
        ""
    };
    as_client(
        backend,
        WORKER,
        "inbox.messaging.PostMessage",
        &format!(
            "{{\"message_id\":\"{id}\",\"from\":\"{WORKER}\",\"on_behalf_of\":\"{WORKER}\",\
             \"to\":\"{to}\",\"kind\":\"{kind}\",\"thread\":\"{id}\",{decision}\
             \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
             \"dedup_key\":{key},\"body\":\"b\",\
             \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
        ),
    )
}

/// A refused message lives until its sender collects the reason, and holds its key until then.
#[test]
fn adv_w7_a_refused_message_holds_its_key_until_the_sender_collects_it() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, WORKER, WORKER, THIRD, KEY)),
        "posted"
    );
    let refused = as_client(
        &backend,
        THIRD,
        "inbox.messaging.Refuse",
        &format!("{{\"message_id\":\"{}\",\"reason\":\"no\"}}", id(1)),
    );
    assert_eq!(outcome(&refused), "refused", "{refused:?}");
    assert_eq!(
        outcome(&post(&backend, 2, WORKER, WORKER, THIRD, KEY)),
        "duplicate-key"
    );
    // The recipient's own key space is untouched by the message it refused.
    assert_eq!(
        outcome(&post(&backend, 3, THIRD, THIRD, WORKER, KEY)),
        "posted"
    );
    let collected = as_client(
        &backend,
        WORKER,
        "inbox.messaging.Collect",
        &format!("{{\"message_id\":\"{}\"}}", id(1)),
    );
    assert_eq!(outcome(&collected), "collected", "{collected:?}");
    assert_eq!(
        outcome(&post(&backend, 4, WORKER, WORKER, THIRD, KEY)),
        "posted"
    );
}

/// A decision taken on silence is kept until `keep_until` and holds its key; the sweep after
/// that removes it and frees the key.
#[test]
fn adv_w7_a_decision_taken_on_silence_holds_its_key_until_it_is_swept() {
    let backend = world();
    let quoted = format!("\"{KEY}\"");
    assert_eq!(
        outcome(&post_kind(&backend, 1, "decision", THIRD, &quoted)),
        "posted"
    );
    let decided = sweep(&backend, 1, "2026-01-07T09:00:00Z", "2026-01-09T09:00:00Z");
    assert_eq!(outcome(&decided), "decided-on-silence", "{decided:?}");
    assert_eq!(
        outcome(&post_kind(&backend, 2, "note", WORKER, &quoted)),
        "duplicate-key"
    );
    let early = sweep(&backend, 1, "2026-01-08T09:00:00Z", "2026-01-10T09:00:00Z");
    assert_eq!(outcome(&early), "not-due", "{early:?}");
    let swept = sweep(&backend, 1, "2026-01-09T09:00:00Z", "2026-01-11T09:00:00Z");
    assert_eq!(outcome(&swept), "expired", "{swept:?}");
    assert_eq!(
        outcome(&post_kind(&backend, 3, "note", WORKER, &quoted)),
        "posted"
    );
}

/// A note read by its recipient is handled and gone at once: its key is free at that moment.
#[test]
fn adv_w7_a_read_note_frees_its_key_at_once() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, WORKER, WORKER, THIRD, KEY)),
        "posted"
    );
    let read = as_client(
        &backend,
        THIRD,
        "inbox.messaging.MarkRead",
        &format!("{{\"message_id\":\"{}\"}}", id(1)),
    );
    assert_eq!(outcome(&read), "handled", "{read:?}");
    assert_eq!(
        outcome(&post(&backend, 2, WORKER, WORKER, THIRD, KEY)),
        "posted"
    );
}

/// The order for the sender's own key: an address no inbox has answers `unknown-inbox` first; a
/// closed inbox answers `duplicate-key` first (the order of step 3 of `fn post_message`). The
/// message in the closed inbox still holds the key.
#[test]
fn adv_w7_the_order_of_duplicate_key_for_the_senders_own_key() {
    let backend = world();
    assert_eq!(
        outcome(&post(&backend, 1, WORKER, WORKER, THIRD, KEY)),
        "posted"
    );
    let closed = as_client(
        &backend,
        THIRD,
        "inbox.messaging.CloseInbox",
        &format!("{{\"address\":\"{THIRD}\"}}"),
    );
    assert_eq!(outcome(&closed), "closed", "{closed:?}");
    assert_eq!(
        outcome(&post(&backend, 2, WORKER, WORKER, THIRD, KEY)),
        "duplicate-key"
    );
    assert_eq!(
        outcome(&post(&backend, 3, WORKER, WORKER, "agent:box/nobody", KEY)),
        "unknown-inbox"
    );
    assert_eq!(
        outcome(&post(&backend, 4, WORKER, WORKER, THIRD, "free")),
        "inbox-closed"
    );
}

/// A key is compared as its text: a key that differs in case, in a trailing space, in Unicode
/// normalisation or in a JSON escape of the same text is another key or the same key exactly as
/// its decoded text says. A very long key is a key like any other.
#[test]
fn adv_w7_a_key_is_compared_as_its_decoded_text() {
    let backend = world();
    let long = format!("\"{}\"", "k".repeat(60_000));
    for (n, key) in [
        "\"Deploy\"",
        "\"deploy\"",
        "\"deploy \"",
        "\"caf\u{e9}\"",
        "\"cafe\u{301}\"",
        long.as_str(),
    ]
    .into_iter()
    .enumerate()
    {
        let answer = post_kind(&backend, 10 + n as u32, "note", THIRD, key);
        assert_eq!(outcome(&answer), "posted", "{key:.40}: {answer:?}");
    }
    // `\u0064eploy` is the text `deploy`, which the sender already holds.
    let escaped = post_kind(&backend, 30, "note", THIRD, "\"\\u0064eploy\"");
    assert_eq!(outcome(&escaped), "duplicate-key", "{escaped:?}");
    assert_eq!(
        outcome(&post_kind(&backend, 31, "note", THIRD, &long)),
        "duplicate-key"
    );
}

/// There is no post without a key: an absent `dedup_key` and a `null` one are refused before
/// any rule, so two senders with no key share nothing and one sender's two keyless posts are not
/// compared.
#[test]
fn adv_w7_an_absent_or_null_key_is_no_post() {
    let backend = world();
    for (n, key) in [(1, None), (2, Some("null"))] {
        let id = id(n);
        let key = key
            .map(|text| format!("\"dedup_key\":{text},"))
            .unwrap_or_default();
        let answer = try_run(
            &backend,
            Actor::Client,
            client(WORKER),
            "inbox.messaging.PostMessage",
            &format!(
                "{{\"message_id\":\"{id}\",\"from\":\"{WORKER}\",\"on_behalf_of\":\"{WORKER}\",\
                 \"to\":\"{THIRD}\",\"kind\":\"note\",\"thread\":\"{id}\",\
                 \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
                 {key}\"body\":\"b\",\
                 \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
            ),
        );
        assert!(answer.is_err(), "post {n}: {answer:?}");
    }
    assert_eq!(stored(&backend), 0);
}
