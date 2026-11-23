//! Adversarial cases against the obligations and `MemoryBackend` (pass 1).

use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{
    Address, DelegationId, InboxSnapshot, MessageId, MessageSnapshot, Principal, ReplyShape,
};
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{
    CallerAttributes, Envelope, Exclusive, MemoryBackend, ServiceContext, SignatureVerifier,
};

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
    let context = ServiceContext::new(caller, Accepting, backend.clone(), crate::support::clock());
    inbox_core::handle(
        backend,
        context,
        Some(&Caller { actor }),
        name,
        json::parse(input).expect("the case writes JSON"),
    )
}

fn client<B>(backend: &B, who: &str, name: &str, input: &str) -> Value
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
{
    run(backend, Some(who), Actor::Client, name, input)
        .unwrap_or_else(|refused| panic!("{name} as {who} refused: {refused}"))
}

fn outcome(answer: &Value) -> String {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn open<B>(backend: &B, address: &str)
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
{
    let answer = client(
        backend,
        address,
        "inbox.messaging.OpenInbox",
        &format!("{{\"address\":\"{address}\",\"adapter\":\"claude_code\"}}"),
    );
    assert_eq!(outcome(&answer), "opened");
}

fn close<B>(backend: &B, address: &str)
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
{
    let answer = client(
        backend,
        address,
        "inbox.messaging.CloseInbox",
        &format!("{{\"address\":\"{address}\"}}"),
    );
    assert_eq!(outcome(&answer), "closed");
}

fn note(message_id: &str, to: &str, dedup_key: &str) -> String {
    format!(
        "{{\"message_id\":\"{message_id}\",\"from\":\"agent:ws/planner\",\
         \"on_behalf_of\":\"agent:ws/planner\",\"to\":\"{to}\",\"kind\":\"note\",\
         \"thread\":\"{message_id}\",\"created\":\"2026-01-05T09:00:00Z\",\
         \"expires\":\"2026-01-06T09:00:00Z\",\"dedup_key\":\"{dedup_key}\",\
         \"body\":\"Build 512 is green.\",\
         \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

fn post<B>(backend: &B, input: &str) -> String
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
{
    outcome(&client(
        backend,
        "agent:ws/planner",
        "inbox.messaging.PostMessage",
        input,
    ))
}

const FIRST: &str = "01JGT0AD000000000000000001";
const SECOND: &str = "01JGT0AD000000000000000002";

// ---------------------------------------------------------------------------------------------
// Concurrency: the read-check-write of `PostMessage` over clones of one `MemoryBackend`.
// ---------------------------------------------------------------------------------------------

/// A barrier that gives up after a while: where nothing serializes the commands it makes both
/// threads read the rows before either writes; where a command section holds the second thread
/// back, the first goes on after `PATIENCE` instead of waiting for it forever.
struct TimedBarrier {
    arrived: Mutex<usize>,
    all: Condvar,
    count: usize,
}

const PATIENCE: Duration = Duration::from_secs(2);

impl TimedBarrier {
    fn new(count: usize) -> Self {
        Self {
            arrived: Mutex::new(0),
            all: Condvar::new(),
            count,
        }
    }

    fn wait(&self) {
        let mut arrived = self.arrived.lock().expect("no panic holds it");
        *arrived += 1;
        self.all.notify_all();
        let _ = self
            .all
            .wait_timeout_while(arrived, PATIENCE, |arrived| *arrived < self.count)
            .expect("no panic holds it");
    }
}

/// `MemoryBackend` with a rendezvous of two threads inside `MessageStorage::list`: both posts have
/// read the store's rows before either writes. Every call is answered by the shared
/// `MemoryBackend`; the barrier only fixes one schedule the OS scheduler is free to pick.
#[derive(Clone)]
struct Rendezvous {
    inner: MemoryBackend,
    barrier: Arc<TimedBarrier>,
}

impl Exclusive for Rendezvous {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        self.inner.exclusive(command)
    }
}

impl InboxStorage for Rendezvous {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        InboxStorage::get(&self.inner, identity)
    }
    fn put(&mut self, snapshot: InboxSnapshot) {
        InboxStorage::put(&mut self.inner, snapshot);
    }
    fn delete(&mut self, identity: &Address) {
        InboxStorage::delete(&mut self.inner, identity);
    }
    fn list(&self) -> Vec<InboxSnapshot> {
        InboxStorage::list(&self.inner)
    }
}

impl MessageStorage for Rendezvous {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        MessageStorage::get(&self.inner, identity)
    }
    fn put(&mut self, snapshot: MessageSnapshot) {
        MessageStorage::put(&mut self.inner, snapshot);
    }
    fn delete(&mut self, identity: &MessageId) {
        MessageStorage::delete(&mut self.inner, identity);
    }
    fn list(&self) -> Vec<MessageSnapshot> {
        let rows = MessageStorage::list(&self.inner);
        self.barrier.wait();
        rows
    }
}

/// Two posts of one logical message (one dedup key, two ids) served at once by two systems over
/// clones of one `MemoryBackend`, the embedding `inbox_core::system` documents: the store must end
/// with one message carrying the key, and one post must answer `duplicate-key`.
#[test]
fn concurrent_posts_with_one_dedup_key_store_one_message() {
    let backend = MemoryBackend::new();
    open(&backend, "agent:box/worker");
    let barrier = Arc::new(TimedBarrier::new(2));
    let threads: Vec<_> = [FIRST, SECOND]
        .into_iter()
        .map(|id| {
            let shared = Rendezvous {
                inner: backend.clone(),
                barrier: Arc::clone(&barrier),
            };
            std::thread::spawn(move || post(&shared, &note(id, "agent:box/worker", "deploy-42")))
        })
        .collect();
    let mut answers: Vec<String> = threads
        .into_iter()
        .map(|thread| thread.join().expect("the thread ran"))
        .collect();
    answers.sort();
    let keyed = MessageStorage::list(&backend)
        .into_iter()
        .filter(|row| row.data.dedup_key.0 == "deploy-42")
        .count();
    assert_eq!(
        (keyed, answers.clone()),
        (1, vec!["duplicate-key".to_owned(), "posted".to_owned()]),
        "rows with the key: {keyed}, answers: {answers:?}"
    );
}

/// The same race with the real scheduler and no rendezvous: 8 threads per round, each a system over
/// a clone of one `MemoryBackend`, each posting its own id with one dedup key.
#[test]
fn concurrent_posts_with_one_dedup_key_store_one_message_under_the_scheduler() {
    const THREADS: usize = 8;
    for round in 0..300 {
        let backend = MemoryBackend::new();
        open(&backend, "agent:box/worker");
        let barrier = Arc::new(Barrier::new(THREADS));
        let threads: Vec<_> = (0..THREADS)
            .map(|n| {
                let shared = backend.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    let id = format!("01JGT0AD0000000000000001{n:02}");
                    let input = note(&id, "agent:box/worker", "deploy-42");
                    barrier.wait();
                    post(&shared, &input)
                })
            })
            .collect();
        let posted = threads
            .into_iter()
            .map(|thread| thread.join().expect("the thread ran"))
            .filter(|answer| answer == "posted")
            .count();
        let keyed = MessageStorage::list(&backend).len();
        assert!(
            posted == 1 && keyed == 1,
            "round {round}: {posted} posts answered posted, {keyed} rows carry the one dedup key"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The order among the row refusals, as the ESS 0.97.1 interpreted target answered each pair
// (probe scenarios in the adversary's scratch directory; each pair run once per candidate) and
// the ESS 0.98.0 one answers the authored scenario `row-refusals-of-a-post-answer-in-one-order`.
// The pairs with `id-taken` are in `tests/post_message_id.rs`.
// ---------------------------------------------------------------------------------------------

#[test]
fn unknown_inbox_answers_before_duplicate_key() {
    let backend = MemoryBackend::new();
    open(&backend, "agent:box/worker");
    assert_eq!(
        post(&backend, &note(FIRST, "agent:box/worker", "deploy-42")),
        "posted"
    );
    let answered = post(&backend, &note(SECOND, "agent:box/nobody", "deploy-42"));
    assert_eq!(answered, "unknown-inbox");
}

/// The sender's own stored message id and the same id from another sender, each posted to an
/// address no inbox has: both answer `unknown-inbox`, as a free id does. The model declares
/// `duplicate` first, and the interpreted target still answers `exists: false` before it and
/// before `id-taken`.
#[test]
fn unknown_inbox_answers_before_duplicate_and_id_taken() {
    let backend = MemoryBackend::new();
    open(&backend, "agent:box/worker");
    assert_eq!(
        post(&backend, &note(FIRST, "agent:box/worker", "key-a")),
        "posted"
    );
    let own = post(&backend, &note(FIRST, "agent:box/nobody", "key-b"));
    let foreign = outcome(&client(
        &backend,
        "agent:box/other",
        "inbox.messaging.PostMessage",
        &note(FIRST, "agent:box/nobody", "key-c").replace("agent:ws/planner", "agent:box/other"),
    ));
    let free = post(&backend, &note(SECOND, "agent:box/nobody", "key-d"));
    assert_eq!(
        (own.as_str(), foreign.as_str(), free.as_str()),
        ("unknown-inbox", "unknown-inbox", "unknown-inbox")
    );
}

#[test]
fn duplicate_key_answers_before_inbox_closed() {
    let backend = MemoryBackend::new();
    open(&backend, "agent:box/worker");
    assert_eq!(
        post(&backend, &note(FIRST, "agent:box/worker", "deploy-42")),
        "posted"
    );
    close(&backend, "agent:box/worker");
    let answered = post(&backend, &note(SECOND, "agent:box/worker", "deploy-42"));
    assert_eq!(answered, "duplicate-key");
}

#[test]
fn duplicate_answers_before_inbox_closed() {
    let backend = MemoryBackend::new();
    open(&backend, "agent:box/worker");
    assert_eq!(
        post(&backend, &note(FIRST, "agent:box/worker", "key-a")),
        "posted"
    );
    close(&backend, "agent:box/worker");
    let answered = post(&backend, &note(FIRST, "agent:box/worker", "key-b"));
    assert_eq!(answered, "duplicate");
}

// ---------------------------------------------------------------------------------------------
// The `posted` row: every envelope field the contract's `sets:` names, read back from the store.
// ---------------------------------------------------------------------------------------------

#[test]
fn posted_stores_every_envelope_field_as_sent() {
    let backend = MemoryBackend::new();
    open(&backend, "agent:box/worker");
    let input = format!(
        "{{\"message_id\":\"{FIRST}\",\"from\":\"agent:ws/relay\",\
         \"on_behalf_of\":\"human:operator\",\"authority\":\"delegation-7\",\
         \"to\":\"agent:box/worker\",\"kind\":\"decision\",\"thread\":\"{SECOND}\",\
         \"reply_shape\":{{\"shape\":\"lines\",\"value\":2}},\
         \"decision\":{{\"options\":[{{\"letter\":\"A\",\"text\":\"Ship.\"}},\
         {{\"letter\":\"B\",\"text\":\"Hold.\"}}],\"recommended_option\":\"B\",\
         \"request_ref\":\"req-9\"}},\
         \"default_on_silence\":\"A\",\"created\":\"2026-01-05T08:59:00Z\",\
         \"expires\":\"2026-01-06T09:00:00Z\",\"dedup_key\":\"logical-1\",\
         \"body\":\"Ship or hold?\",\
         \"signature\":{{\"key_id\":\"key-relay\",\"value\":\"c2lnLXJlbGF5\"}}}}"
    );
    let answer = client(
        &backend,
        "agent:ws/relay",
        "inbox.messaging.PostMessage",
        &input,
    );
    assert_eq!(outcome(&answer), "posted");
    let row = MessageStorage::get(&backend, &MessageId(FIRST.to_owned())).expect("stored");
    let data = row.data;
    assert_eq!(data.from.0, "agent:ws/relay");
    assert_eq!(data.on_behalf_of.0, "human:operator");
    assert_eq!(data.authority.map(|a| a.0).as_deref(), Some("delegation-7"));
    assert_eq!(data.to.0, "agent:box/worker");
    assert_eq!(data.thread.0, SECOND);
    assert_eq!(data.reply_shape, Some(ReplyShape::Lines(2)));
    let decision = data.decision.expect("decision stored");
    assert_eq!(decision.options.len(), 2);
    assert_eq!(
        decision.recommended_option.map(|l| l.0).as_deref(),
        Some("B")
    );
    assert_eq!(decision.request_ref.as_deref(), Some("req-9"));
    assert_eq!(data.default_on_silence.map(|l| l.0).as_deref(), Some("A"));
    assert_eq!(data.created.0, "2026-01-05T08:59:00Z");
    assert_eq!(data.expires.0, "2026-01-06T09:00:00Z");
    assert_eq!(data.dedup_key.0, "logical-1");
    assert_eq!(data.body.0, "Ship or hold?");
    assert_eq!(data.signature.key_id, "key-relay");
    assert_eq!(data.signature.value, "c2lnLXJlbGF5");
    assert!(data.answered_by.is_none() && data.decided_option.is_none());
    assert!(data.reply.is_none() && data.refusal.is_none());
}
