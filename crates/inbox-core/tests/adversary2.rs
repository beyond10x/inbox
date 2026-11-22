//! Adversarial cases against the command-level section (pass 2): which handles share it, what a
//! panic inside it leaves, and a command whose context calls the entry again.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc;
use std::time::Duration;

use inbox::actor::{Actor, Caller};
use inbox::behaviour::{ExternalCommand, InboxStorage, MessageStorage, TryContext};
use inbox::messaging::{Address, DelegationId, MessageId, MessageState, PostMessage, Principal};
use inbox::obligation::UnmetObligation;
use inbox::server::entry::Refused;
use inbox::server::json::{self, Value};
use inbox_core::{
    CallerAttributes, Envelope, Exclusive, MemoryBackend, PostExternals, ServiceContext,
    SignatureVerifier,
};

const BOUND: Duration = Duration::from_secs(5);
const MESSAGE: &str = "01JGT0AE000000000000000001";

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

fn caller(who: &str) -> CallerAttributes {
    CallerAttributes::new(Principal(who.to_owned()), Address(who.to_owned()))
}

fn client<B, C>(backend: &B, context: C, name: &str, input: &str) -> Result<Value, Refused>
where
    B: Exclusive + InboxStorage + MessageStorage + Clone,
    C: TryContext + PostExternals,
{
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

fn as_client(backend: &MemoryBackend, who: &str, name: &str, input: &str) -> String {
    let context = ServiceContext::new(
        caller(who),
        Accepting,
        backend.clone(),
        crate::support::clock(),
    );
    let answer = client(backend, context, name, input)
        .unwrap_or_else(|refused| panic!("{name} as {who} refused: {refused}"));
    outcome(&answer)
}

fn outcome(answer: &Value) -> String {
    match answer.member("outcome") {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("no outcome in {answer:?}: {other:?}"),
    }
}

fn open_input(address: &str) -> String {
    format!("{{\"address\":\"{address}\",\"adapter\":\"terminal\"}}")
}

fn request(message_id: &str) -> String {
    format!(
        "{{\"message_id\":\"{message_id}\",\"from\":\"agent:ws/planner\",\
         \"on_behalf_of\":\"agent:ws/planner\",\"to\":\"agent:box/worker\",\"kind\":\"request\",\
         \"thread\":\"{message_id}\",\"reply_shape\":{{\"shape\":\"lines\",\"value\":3}},\
         \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
         \"dedup_key\":\"{message_id}\",\"body\":\"Status?\",\
         \"signature\":{{\"key_id\":\"key-1\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

fn reply(in_reply_to: &str, reply_id: &str) -> String {
    format!(
        "{{\"in_reply_to\":\"{in_reply_to}\",\"reply_id\":\"{reply_id}\",\
         \"from\":\"agent:box/worker\",\"on_behalf_of\":\"agent:box/worker\",\
         \"created\":\"2026-01-05T09:05:00Z\",\"expires\":\"2026-01-06T09:05:00Z\",\
         \"body\":\"One line.\",\
         \"signature\":{{\"key_id\":\"key-2\",\"value\":\"c2lnbmF0dXJl\"}}}}"
    )
}

/// Holds `backend`'s section on another thread until the answer sender is dropped or sends.
fn hold_section(backend: &MemoryBackend) -> mpsc::Sender<()> {
    let holder = backend.clone();
    let (held, is_held) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        holder.exclusive(|| {
            let _ = held.send(());
            let _ = released.recv_timeout(Duration::from_secs(10));
        });
    });
    is_held
        .recv_timeout(BOUND)
        .expect("the holder took the section");
    release
}

/// Opens `address` through the entry on another thread; the receiver gets its outcome.
fn open_elsewhere(backend: &MemoryBackend, address: &str) -> mpsc::Receiver<String> {
    let backend = backend.clone();
    let address = address.to_owned();
    let (sent, answered) = mpsc::channel();
    std::thread::spawn(move || {
        let answer = as_client(
            &backend,
            &address,
            "inbox.messaging.OpenInbox",
            &open_input(&address),
        );
        let _ = sent.send(answer);
    });
    answered
}

/// Clones share the section: a request through one clone waits while another clone holds it.
#[test]
fn a_clone_waits_while_another_clone_holds_the_section() {
    let backend = MemoryBackend::new();
    let release = hold_section(&backend.clone());
    let answered = open_elsewhere(&backend.clone(), "agent:box/one");
    assert!(
        answered.recv_timeout(Duration::from_millis(300)).is_err(),
        "a request through another clone ran while the section was held"
    );
    release.send(()).expect("the holder waits");
    assert_eq!(
        answered.recv_timeout(BOUND).expect("ran after the release"),
        "opened"
    );
}

/// Two stores are two sections: a request over a store of its own does not wait for another's.
#[test]
fn a_store_of_its_own_does_not_wait_for_another_stores_section() {
    let held = MemoryBackend::new();
    let release = hold_section(&held);
    let other = MemoryBackend::new();
    let answered = open_elsewhere(&other, "agent:box/one");
    assert_eq!(
        answered
            .recv_timeout(BOUND)
            .expect("a store of its own ran while another store's section was held"),
        "opened"
    );
    drop(release);
}

/// A context for the owner of `agent:box/worker` whose `shape-mismatch` answer panics: the
/// `Answer` command panics inside the section after every read and before its one write.
struct PanicsOnShape;

impl TryContext for PanicsOnShape {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        Ok(Some(Address("agent:box/worker".to_owned())))
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        Ok(Some(Principal("agent:box/worker".to_owned())))
    }

    fn try_external(
        &mut self,
        _command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        match outcome {
            "shape-mismatch" => panic!("the answer panics inside the section"),
            _ => Ok(false),
        }
    }
}

impl PostExternals for PanicsOnShape {
    fn expired_on_arrival(&mut self, _input: &PostMessage) -> Result<bool, UnmetObligation> {
        Ok(false)
    }

    fn signature_invalid(&mut self, _input: &PostMessage) -> bool {
        false
    }

    fn authority_invalid(&mut self, _input: &PostMessage) -> bool {
        false
    }
}

/// An `Answer` that panics inside the section leaves the message as it was, and the next answer
/// through the entry settles it.
#[test]
fn an_answer_that_panics_leaves_the_message_open_and_the_next_answer_settles_it() {
    let backend = MemoryBackend::new();
    let worker = "agent:box/worker";
    assert_eq!(
        as_client(
            &backend,
            worker,
            "inbox.messaging.OpenInbox",
            &open_input(worker)
        ),
        "opened"
    );
    assert_eq!(
        as_client(
            &backend,
            "agent:ws/planner",
            "inbox.messaging.PostMessage",
            &request(MESSAGE)
        ),
        "posted"
    );
    let panicked = catch_unwind(AssertUnwindSafe(|| {
        client(
            &backend,
            PanicsOnShape,
            "inbox.messaging.Answer",
            &reply(MESSAGE, "01JGT0AE000000000000000101"),
        )
    }));
    assert!(panicked.is_err(), "the answer panicked");
    let row = MessageStorage::get(&backend, &MessageId(MESSAGE.to_owned())).expect("still stored");
    assert_eq!(row.state, MessageState::Sent);
    assert!(row.data.reply.is_none() && row.data.answered_by.is_none());

    let next = backend.clone();
    let (sent, answered) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(as_client(
            &next,
            "agent:box/worker",
            "inbox.messaging.Answer",
            &reply(MESSAGE, "01JGT0AE000000000000000102"),
        ));
    });
    assert_eq!(
        answered
            .recv_timeout(BOUND)
            .expect("the next answer ran: the section was released"),
        "answered"
    );
}

/// A context whose `signature-invalid` question reads a view through the entry over the same
/// store: a verifier or harness that consults the service while a post runs.
struct AsksTheEntry {
    inner: ServiceContext<Accepting, MemoryBackend, inbox::primitives::Timestamp>,
    backend: MemoryBackend,
}

impl TryContext for AsksTheEntry {
    fn try_caller_inbox(&self) -> Result<Option<Address>, UnmetObligation> {
        self.inner.try_caller_inbox()
    }

    fn try_caller_principal(&self) -> Result<Option<Principal>, UnmetObligation> {
        self.inner.try_caller_principal()
    }

    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        self.inner.try_external(command, outcome)
    }
}

impl PostExternals for AsksTheEntry {
    fn expired_on_arrival(&mut self, input: &PostMessage) -> Result<bool, UnmetObligation> {
        self.inner.expired_on_arrival(input)
    }

    fn signature_invalid(&mut self, input: &PostMessage) -> bool {
        let context = ServiceContext::new(
            caller("agent:ws/reader"),
            Accepting,
            self.backend.clone(),
            crate::support::clock(),
        );
        let _ = client(&self.backend, context, "inbox.messaging.Inboxes", "{}");
        self.inner.signature_invalid(input)
    }

    fn authority_invalid(&mut self, input: &PostMessage) -> bool {
        self.inner.authority_invalid(input)
    }
}

/// A command whose context calls the entry again over the same store fails at once: the second
/// entry on the thread that holds the section panics with the rule, nothing is stored, and the
/// next command through the entry over the same store is answered.
#[test]
fn a_context_that_calls_the_entry_again_fails_at_once_and_frees_the_section() {
    let backend = MemoryBackend::new();
    let worker = "agent:box/worker";
    assert_eq!(
        as_client(
            &backend,
            worker,
            "inbox.messaging.OpenInbox",
            &open_input(worker)
        ),
        "opened"
    );
    let inner = backend.clone();
    let (sent, answered) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let context = AsksTheEntry {
            inner: ServiceContext::new(
                caller("agent:ws/planner"),
                Accepting,
                inner.clone(),
                crate::support::clock(),
            ),
            backend: inner.clone(),
        };
        let answer = client(
            &inner,
            context,
            "inbox.messaging.PostMessage",
            &request(MESSAGE),
        );
        let _ = sent.send(answer.map(|answer| outcome(&answer)));
    });
    let answer = answered.recv_timeout(BOUND);
    assert!(
        matches!(answer, Err(mpsc::RecvTimeoutError::Disconnected)),
        "a post whose context calls the entry again did not end within {BOUND:?} with a panic: {answer:?}"
    );
    let payload = thread.join().expect_err("the thread ended with the panic");
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| {
            payload
                .downcast_ref::<&str>()
                .map(|text| (*text).to_owned())
        })
        .unwrap_or_default();
    assert!(
        message.contains("must not call the entry over the same store"),
        "the panic names the rule: {message:?}"
    );
    assert!(
        MessageStorage::get(&backend, &MessageId(MESSAGE.to_owned())).is_none(),
        "the outer post stored nothing"
    );
    assert_eq!(
        open_elsewhere(&backend, "agent:box/one")
            .recv_timeout(BOUND)
            .expect("the next command through the entry ran"),
        "opened"
    );
}
