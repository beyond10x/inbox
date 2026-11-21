//! The selection order of `PostMessage` where two of its refusals hold at once, as the ESS 0.96.0
//! interpreter and synthesizer answer it: the input-guarded refusals first, then the 3 externals
//! before any row is read, then the row refusals.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{Address, InboxSnapshot, MessageId, MessageSnapshot, Principal};
use inbox::server::inbox_service::handle;
use inbox::server::json;
use inbox_core::{CallerAttributes, MemoryBackend, ServiceContext};

use crate::support::{KeyIdVerifier, Post, World, open_worker, outcome, post, quoted};

const FIRST: &str = "01JGT0PR000000000000000001";
const SECOND: &str = "01JGT0PR000000000000000002";

/// A row refusal the second post meets.
#[derive(Debug, Clone, Copy)]
enum Row {
    UnknownInbox,
    Duplicate,
    DuplicateKey,
    InboxClosed,
}

impl Row {
    const ALL: [Row; 4] = [
        Row::UnknownInbox,
        Row::Duplicate,
        Row::DuplicateKey,
        Row::InboxClosed,
    ];
}

/// An envelope refusal the second post carries.
#[derive(Debug, Clone, Copy)]
enum Fault {
    NoAuthority,
    AlreadyExpired,
    RequestWithoutShape,
    DecisionWithoutOptions,
    UnknownOption,
    SignatureInvalid,
    AuthorityInvalid,
}

impl Fault {
    fn outcome(self) -> &'static str {
        match self {
            Fault::NoAuthority => "no-authority",
            Fault::AlreadyExpired => "already-expired",
            Fault::RequestWithoutShape => "request-without-shape",
            Fault::DecisionWithoutOptions => "decision-without-options",
            Fault::UnknownOption => "unknown-option",
            Fault::SignatureInvalid => "signature-invalid",
            Fault::AuthorityInvalid => "authority-invalid",
        }
    }

    /// The envelope with this fault and no other.
    fn envelope(self, message_id: &str) -> Post {
        let mut envelope = Post::note(message_id);
        match self {
            Fault::NoAuthority => envelope.on_behalf_of = "human:operator".to_owned(),
            Fault::AlreadyExpired => envelope.expires = envelope.created.clone(),
            Fault::RequestWithoutShape => {
                envelope = Post::request(message_id, 1);
                envelope.reply_shape = None;
            }
            Fault::DecisionWithoutOptions => {
                envelope = Post::decision(message_id, None);
                envelope.decision = None;
            }
            Fault::UnknownOption => envelope = Post::decision(message_id, Some("C")),
            Fault::SignatureInvalid => envelope.key_id = "key-refused".to_owned(),
            Fault::AuthorityInvalid => {
                envelope.on_behalf_of = "human:operator".to_owned();
                envelope.authority = Some("delegation-lapsed".to_owned());
            }
        }
        envelope
    }
}

fn verifier() -> KeyIdVerifier {
    KeyIdVerifier::accepting()
        .refusing_key("key-refused")
        .refusing_delegation("delegation-lapsed")
}

/// Posts an envelope carrying `fault` into a world where `row` also holds, and answers the
/// outcome.
fn pair(fault: Fault, row: Row) -> String {
    let world = World::new(verifier());
    open_worker(&world);
    let mut envelope = fault.envelope(SECOND);
    match row {
        Row::UnknownInbox => envelope.to = "agent:box/nobody".to_owned(),
        Row::Duplicate => {
            let mut first = Post::note(SECOND);
            first.dedup_key = "held-by-the-first".to_owned();
            assert_eq!(outcome(&post(&world, &first)), "posted");
        }
        Row::DuplicateKey => {
            let mut first = Post::note(FIRST);
            first.dedup_key = envelope.dedup_key.clone();
            assert_eq!(outcome(&post(&world, &first)), "posted");
        }
        Row::InboxClosed => {
            let closed = world.client(
                "agent:box/worker",
                "inbox.messaging.CloseInbox",
                r#"{"address":"agent:box/worker"}"#,
            );
            assert_eq!(outcome(&closed), "closed");
        }
    }
    outcome(&post(&world, &envelope)).to_owned()
}

fn assert_pairs(cases: &[(Fault, Row)], expected: impl Fn(Fault, Row) -> &'static str) {
    let wrong: Vec<String> = cases
        .iter()
        .filter_map(|&(fault, row)| {
            let answered = pair(fault, row);
            let expected = expected(fault, row);
            (answered != expected).then(|| {
                format!("{fault:?} with {row:?}: answered {answered}, expected {expected}")
            })
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn an_external_refusal_answers_before_every_row_refusal() {
    let cases: Vec<(Fault, Row)> = [
        Fault::UnknownOption,
        Fault::SignatureInvalid,
        Fault::AuthorityInvalid,
    ]
    .into_iter()
    .flat_map(|fault| Row::ALL.map(|row| (fault, row)))
    .collect();
    assert_eq!(cases.len(), 12);
    assert_pairs(&cases, |fault, _| fault.outcome());
}

#[test]
fn request_without_shape_answers_before_every_row_refusal() {
    let cases = Row::ALL.map(|row| (Fault::RequestWithoutShape, row));
    assert_pairs(&cases, |fault, _| fault.outcome());
}

#[test]
fn every_other_input_refusal_answers_before_unknown_inbox() {
    let cases = [
        Fault::NoAuthority,
        Fault::AlreadyExpired,
        Fault::DecisionWithoutOptions,
    ]
    .map(|fault| (fault, Row::UnknownInbox));
    assert_pairs(&cases, |fault, _| fault.outcome());
}

#[test]
fn an_input_refusal_answers_before_an_external_refusal() {
    let cases = [
        (Fault::NoAuthority, "no-authority"),
        (Fault::AlreadyExpired, "already-expired"),
        (Fault::RequestWithoutShape, "request-without-shape"),
    ];
    let world = World::new(verifier());
    open_worker(&world);
    for (fault, expected) in cases {
        let mut envelope = fault.envelope(SECOND);
        envelope.key_id = "key-refused".to_owned();
        assert_eq!(outcome(&post(&world, &envelope)), expected, "{fault:?}");
    }
}

/// `MemoryBackend`, counting every read.
#[derive(Clone, Default)]
struct Counting {
    inner: MemoryBackend,
    reads: Arc<AtomicUsize>,
}

impl Counting {
    fn read(&self) {
        self.reads.fetch_add(1, Ordering::SeqCst);
    }
}

impl InboxStorage for Counting {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        self.read();
        InboxStorage::get(&self.inner, identity)
    }
    fn put(&mut self, snapshot: InboxSnapshot) {
        InboxStorage::put(&mut self.inner, snapshot);
    }
    fn delete(&mut self, identity: &Address) {
        InboxStorage::delete(&mut self.inner, identity);
    }
    fn list(&self) -> Vec<InboxSnapshot> {
        self.read();
        InboxStorage::list(&self.inner)
    }
}

impl MessageStorage for Counting {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        self.read();
        MessageStorage::get(&self.inner, identity)
    }
    fn put(&mut self, snapshot: MessageSnapshot) {
        MessageStorage::put(&mut self.inner, snapshot);
    }
    fn delete(&mut self, identity: &MessageId) {
        MessageStorage::delete(&mut self.inner, identity);
    }
    fn list(&self) -> Vec<MessageSnapshot> {
        self.read();
        MessageStorage::list(&self.inner)
    }
}

/// Runs one `Client` command over `backend` and answers its outcome.
fn run(backend: &Counting, who: &str, name: &str, input: &str) -> String {
    let caller = CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()));
    let context = ServiceContext::new(caller, verifier(), backend.clone());
    let mut system = inbox_core::system(backend.clone(), context);
    let input = json::parse(input).expect("the case writes JSON");
    let answer = handle(
        &mut system,
        Some(&Caller {
            actor: Actor::Client,
        }),
        name,
        input,
    )
    .unwrap_or_else(|refused| panic!("{name} refused: {refused}"));
    outcome(&answer).to_owned()
}

/// The reads one post makes, in a world holding an open inbox.
fn reads_of(envelope: &Post) -> (String, usize) {
    let backend = Counting::default();
    let opened = run(
        &backend,
        "agent:box/worker",
        "inbox.messaging.OpenInbox",
        &format!(
            "{{\"address\":{},\"adapter\":\"claude_code\"}}",
            quoted("agent:box/worker")
        ),
    );
    assert_eq!(opened, "opened");
    backend.reads.store(0, Ordering::SeqCst);
    let answered = run(
        &backend,
        &envelope.from,
        "inbox.messaging.PostMessage",
        &envelope.json(),
    );
    (answered, backend.reads.load(Ordering::SeqCst))
}

#[test]
fn a_post_an_external_branch_refuses_reads_no_row() {
    for fault in [
        Fault::UnknownOption,
        Fault::SignatureInvalid,
        Fault::AuthorityInvalid,
    ] {
        let (answered, reads) = reads_of(&fault.envelope(SECOND));
        assert_eq!(answered, fault.outcome());
        assert_eq!(reads, 0, "{fault:?} read {reads} rows");
    }
    let (answered, reads) = reads_of(&Post::note(SECOND));
    assert_eq!(answered, "posted");
    assert!(
        reads > 0,
        "a stored post reads the rows it is checked against"
    );
}
