//! Adversary cases for story:external-refusal-order-cases against the core: each external fault
//! alone gives its own refusal with the envelopes the unit builds, the order holds when the faults
//! are made in other ways than the unit makes them, and the verifier is asked one question per
//! branch in order, none after the branch that is taken.

use std::sync::{Arc, Mutex};

use inbox::actor::{Actor, Caller};
use inbox::behaviour::MessageStorage;
use inbox::messaging::{Address, DelegationId, Principal};
use inbox::server::json;
use inbox_core::{CallerAttributes, Envelope, MemoryBackend, ServiceContext, SignatureVerifier};

use crate::support::{KeyIdVerifier, Post, World, open_worker, outcome, post};

const SECOND: &str = "01JGT0W8000000000000000002";

fn verifier() -> KeyIdVerifier {
    KeyIdVerifier::accepting()
        .refusing_key("key-refused")
        .refusing_delegation("delegation-lapsed")
}

fn unknown_option_only() -> Post {
    Post::decision(SECOND, Some("C"))
}

fn signature_invalid_only() -> Post {
    let mut envelope = Post::note(SECOND);
    envelope.key_id = "key-refused".to_owned();
    envelope
}

fn authority_invalid_only() -> Post {
    let mut envelope = Post::note(SECOND);
    envelope.on_behalf_of = "human:operator".to_owned();
    envelope.authority = Some("delegation-lapsed".to_owned());
    envelope
}

fn both(mut first: Post, second: &Post) -> Post {
    if second.kind == "decision" {
        first.kind = second.kind.clone();
        first.decision = second.decision.clone();
        first.default_on_silence = second.default_on_silence.clone();
    }
    if second.key_id == "key-refused" {
        first.key_id = second.key_id.clone();
    }
    if second.authority.is_some() {
        first.on_behalf_of = second.on_behalf_of.clone();
        first.authority = second.authority.clone();
    }
    first
}

fn answer_with(verifier: KeyIdVerifier, envelope: &Post) -> String {
    let world = World::new(verifier);
    open_worker(&world);
    let answer = post(&world, envelope);
    assert!(MessageStorage::list(&world.backend).is_empty());
    outcome(&answer).to_owned()
}

#[test]
fn each_external_fault_alone_answers_its_own_refusal() {
    assert_eq!(
        answer_with(verifier(), &unknown_option_only()),
        "unknown-option"
    );
    assert_eq!(
        answer_with(verifier(), &signature_invalid_only()),
        "signature-invalid"
    );
    assert_eq!(
        answer_with(verifier(), &authority_invalid_only()),
        "authority-invalid"
    );
}

#[test]
fn no_fault_at_all_posts_with_the_unit_envelopes_base() {
    let world = World::new(verifier());
    open_worker(&world);
    assert_eq!(outcome(&post(&world, &Post::note(SECOND))), "posted");
}

#[test]
fn a_recommended_option_no_option_has_answers_unknown_option_before_the_others() {
    let mut envelope = both(
        both(Post::decision(SECOND, None), &signature_invalid_only()),
        &authority_invalid_only(),
    );
    envelope.decision = Some(
        "{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"}],\"recommended_option\":\"Z\"}"
            .to_owned(),
    );
    assert_eq!(answer_with(verifier(), &envelope), "unknown-option");
}

#[test]
fn two_options_with_one_letter_answer_unknown_option_before_the_others() {
    let mut envelope = both(
        both(Post::decision(SECOND, None), &signature_invalid_only()),
        &authority_invalid_only(),
    );
    envelope.decision = Some(
        "{\"options\":[{\"letter\":\"A\",\"text\":\"Ship.\"},{\"letter\":\"A\",\"text\":\"Hold.\"}],\
         \"recommended_option\":\"A\"}"
            .to_owned(),
    );
    assert_eq!(answer_with(verifier(), &envelope), "unknown-option");
}

#[test]
fn a_silence_option_on_a_request_answers_unknown_option_before_the_others() {
    let mut envelope = both(
        both(Post::request(SECOND, 1), &signature_invalid_only()),
        &authority_invalid_only(),
    );
    envelope.default_on_silence = Some("A".to_owned());
    assert_eq!(answer_with(verifier(), &envelope), "unknown-option");
}

/// A verifier that does not know a key (not merely refuses it) and finds a delegation lapsed
/// because it names another principal than the one the envelope speaks for.
#[derive(Debug, Clone, Default)]
struct Realistic {
    calls: Arc<Mutex<Vec<&'static str>>>,
}

impl SignatureVerifier for Realistic {
    fn verifies(&self, envelope: Envelope<'_>) -> bool {
        self.calls.lock().unwrap().push("verifies");
        envelope.signature().key_id == "key-1"
    }

    fn authorizes(&self, envelope: Envelope<'_>, authority: &DelegationId) -> bool {
        self.calls.lock().unwrap().push("authorizes");
        authority.0 == "delegation-for-operator" && envelope.on_behalf_of().0 == "human:operator"
    }
}

fn run_realistic(envelope: &Post) -> (String, Vec<&'static str>) {
    let backend = MemoryBackend::new();
    let verifier = Realistic::default();
    let calls = verifier.calls.clone();
    let owner = CallerAttributes::new(
        Principal("agent:box/worker".to_owned()),
        Address("agent:box/worker".to_owned()),
    );
    let context = ServiceContext::new(owner, verifier.clone(), backend.clone());
    let opened = inbox_core::handle(
        &backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        "inbox.messaging.OpenInbox",
        json::parse(r#"{"address":"agent:box/worker","adapter":"claude_code"}"#).unwrap(),
    )
    .expect("open");
    assert_eq!(outcome(&opened), "opened");
    calls.lock().unwrap().clear();
    let sender = CallerAttributes::new(
        Principal(envelope.from.clone()),
        Address(envelope.from.clone()),
    );
    let context = ServiceContext::new(sender, verifier, backend.clone());
    let answered = inbox_core::handle(
        &backend,
        context,
        Some(&Caller {
            actor: Actor::Client,
        }),
        "inbox.messaging.PostMessage",
        json::parse(&envelope.json()).unwrap(),
    )
    .expect("post");
    assert!(MessageStorage::list(&backend).is_empty());
    let asked = calls.lock().unwrap().clone();
    (outcome(&answered).to_owned(), asked)
}

#[test]
fn an_unknown_key_answers_before_a_delegation_naming_another_principal() {
    let mut envelope = Post::note(SECOND);
    envelope.key_id = "key-unknown".to_owned();
    envelope.on_behalf_of = "human:other".to_owned();
    envelope.authority = Some("delegation-for-operator".to_owned());
    let (answered, _) = run_realistic(&envelope);
    assert_eq!(answered, "signature-invalid");
}

#[test]
fn a_delegation_naming_another_principal_alone_answers_authority_invalid() {
    let mut envelope = Post::note(SECOND);
    envelope.on_behalf_of = "human:other".to_owned();
    envelope.authority = Some("delegation-for-operator".to_owned());
    let (answered, asked) = run_realistic(&envelope);
    assert_eq!(answered, "authority-invalid");
    assert_eq!(asked, ["verifies", "authorizes"]);
}

#[test]
fn unknown_option_asks_the_verifier_nothing() {
    let envelope = both(
        both(unknown_option_only(), &signature_invalid_only()),
        &authority_invalid_only(),
    );
    let mut envelope = envelope;
    envelope.key_id = "key-unknown".to_owned();
    envelope.on_behalf_of = "human:other".to_owned();
    let (answered, asked) = run_realistic(&envelope);
    assert_eq!(answered, "unknown-option");
    assert_eq!(asked, Vec::<&str>::new());
}

#[test]
fn signature_invalid_asks_the_verifier_no_delegation_question() {
    let mut envelope = Post::note(SECOND);
    envelope.key_id = "key-unknown".to_owned();
    envelope.on_behalf_of = "human:other".to_owned();
    envelope.authority = Some("delegation-for-operator".to_owned());
    let (answered, asked) = run_realistic(&envelope);
    assert_eq!(answered, "signature-invalid");
    assert_eq!(asked, ["verifies"]);
}
