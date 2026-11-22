//! Adversary cases for story:post-from-is-the-caller against the core: the claims of the unit
//! that no case of `cargo test` held before: a `Client` without a `principal` is an unmet
//! obligation, the generated `not-sender` of `Answer` in every state of the message, and a
//! `from` that differs from the caller only in its spelling.

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

const WORKER: &str = "agent:box/worker";
const PLANNER: &str = "agent:ws/planner";
const OTHER: &str = "agent:box/other";
const REQUEST: &str = "01JGT0W6000000000000000001";
const NOTE: &str = "01JGT0W6000000000000000002";
const MISSING: &str = "01JGT0W6000000000000000099";

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

fn run(
    backend: &MemoryBackend,
    caller: CallerAttributes,
    name: &str,
    input: &str,
) -> Result<Value, Refused> {
    let context = ServiceContext::new(caller, Accepting, backend.clone(), crate::support::clock());
    inbox_core::handle(
        backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
}

fn as_client(backend: &MemoryBackend, who: &str, name: &str, input: &str) -> Value {
    let caller = CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()));
    run(backend, caller, name, input).unwrap_or_else(|refused| panic!("{name} as {who}: {refused}"))
}

fn outcome(answer: &Value) -> String {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn published(answer: &Value) -> usize {
    match answer.member("published") {
        Some(Value::Array(events)) => events.len(),
        other => panic!("no published list in {answer:?}: {other:?}"),
    }
}

fn quoted(text: &str) -> String {
    let mut out = String::new();
    json::push_text(&mut out, text);
    out
}

fn post_json(id: &str, from: &str, kind: &str) -> String {
    let shape = if kind == "request" {
        r#","reply_shape":{"shape":"lines","value":3}"#
    } else {
        ""
    };
    format!(
        "{{\"message_id\":\"{id}\",\"from\":{f},\"on_behalf_of\":{f},\"to\":\"{WORKER}\",\
         \"kind\":\"{kind}\",\"thread\":\"{id}\"{shape},\"created\":\"2026-01-05T09:00:00Z\",\
         \"expires\":\"2026-01-06T09:00:00Z\",\"dedup_key\":\"{id}\",\"body\":\"b\",\
         \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}",
        f = quoted(from)
    )
}

fn reply_json(in_reply_to: &str, reply_id: &str, from: &str) -> String {
    format!(
        "{{\"in_reply_to\":\"{in_reply_to}\",\"reply_id\":\"{reply_id}\",\"from\":{f},\
         \"on_behalf_of\":{f},\"created\":\"2026-01-05T09:05:00Z\",\
         \"expires\":\"2026-01-06T09:05:00Z\",\"body\":\"One line.\",\
         \"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}",
        f = quoted(from)
    )
}

/// The worker's inbox open, a request and a note from the planner in it.
fn world() -> MemoryBackend {
    let backend = MemoryBackend::new();
    let opened = as_client(
        &backend,
        WORKER,
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{WORKER}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(outcome(&opened), "opened");
    for (id, kind) in [(REQUEST, "request"), (NOTE, "note")] {
        let posted = as_client(
            &backend,
            PLANNER,
            "inbox.messaging.PostMessage",
            &post_json(id, PLANNER, kind),
        );
        assert_eq!(outcome(&posted), "posted", "{kind}");
    }
    backend
}

#[test]
fn adv_w6_a_client_without_a_principal_posting_is_an_unmet_obligation() {
    let backend = world();
    let before = MessageStorage::list(&backend);
    let caller = CallerAttributes {
        principal: None,
        inbox: Some(Address(PLANNER.to_owned())),
    };
    let answered = run(
        &backend,
        caller,
        "inbox.messaging.PostMessage",
        &post_json("01JGT0W6000000000000000003", PLANNER, "note"),
    );
    assert!(
        answered.is_err(),
        "a Client without a principal was answered {answered:?}"
    );
    assert_eq!(MessageStorage::list(&backend), before);
}

#[test]
fn adv_w6_an_answer_whose_from_is_not_the_caller_is_not_sender_in_every_state() {
    let backend = world();
    let mut wrong = Vec::new();
    let mut check = |who: &str, in_reply_to: &str, reply_id: &str, from: &str, state: &str| {
        let before = MessageStorage::list(&backend);
        let answer = as_client(
            &backend,
            who,
            "inbox.messaging.Answer",
            &reply_json(in_reply_to, reply_id, from),
        );
        let after = MessageStorage::list(&backend);
        let answered = outcome(&answer);
        if answered != "not-sender" || published(&answer) != 0 || after != before {
            wrong.push(format!(
                "{who} as {from} on the {state}: answered {answered}"
            ));
        }
    };
    check(
        OTHER,
        REQUEST,
        "01JGT0W6000000000000000010",
        WORKER,
        "open request",
    );
    check(
        WORKER,
        REQUEST,
        "01JGT0W6000000000000000011",
        OTHER,
        "open request",
    );
    check(WORKER, NOTE, "01JGT0W6000000000000000012", OTHER, "note");
    check(
        OTHER,
        MISSING,
        "01JGT0W6000000000000000013",
        WORKER,
        "missing message",
    );
    check(
        WORKER,
        MISSING,
        "01JGT0W6000000000000000014",
        OTHER,
        "missing message",
    );
    let answered = as_client(
        &backend,
        WORKER,
        "inbox.messaging.Answer",
        &reply_json(REQUEST, "01JGT0W6000000000000000015", WORKER),
    );
    assert_eq!(outcome(&answered), "answered");
    check(
        OTHER,
        REQUEST,
        "01JGT0W6000000000000000016",
        WORKER,
        "answered request",
    );
    check(
        WORKER,
        REQUEST,
        "01JGT0W6000000000000000017",
        OTHER,
        "answered request",
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn adv_w6_a_from_that_differs_from_the_caller_only_in_spelling_is_not_sender() {
    let backend = world();
    let before = MessageStorage::list(&backend);
    let lookalike = PLANNER.replacen('a', "\u{0430}", 1);
    let upper = PLANNER.to_uppercase();
    let spaced = format!("{PLANNER} ");
    let slashed = format!("{PLANNER}/");
    let variants = [
        upper.as_str(),
        spaced.as_str(),
        slashed.as_str(),
        lookalike.as_str(),
        "",
    ];
    let mut wrong = Vec::new();
    for (n, from) in (20..).zip(variants) {
        let id = format!("01JGT0W60000000000000000{n}");
        // The caller is the planner and `from` a spelling of it; then the reverse.
        let answer = as_client(
            &backend,
            PLANNER,
            "inbox.messaging.PostMessage",
            &post_json(&id, from, "note"),
        );
        if outcome(&answer) != "not-sender" {
            wrong.push(format!(
                "caller {PLANNER:?}, from {from:?}: {}",
                outcome(&answer)
            ));
        }
        if !from.is_empty() {
            let answer = as_client(
                &backend,
                from,
                "inbox.messaging.PostMessage",
                &post_json(&id, PLANNER, "note"),
            );
            if outcome(&answer) != "not-sender" {
                wrong.push(format!(
                    "caller {from:?}, from {PLANNER:?}: {}",
                    outcome(&answer)
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    assert_eq!(MessageStorage::list(&backend), before);
}
