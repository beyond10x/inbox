//! The checks every backend of the inbox passes, beside the conformance suite.
//!
//! A backend is three parts of one contract: the generated storage ports
//! [`InboxStorage`] and [`MessageStorage`], and the command section [`Exclusive`] of `inbox-core`.
//! A backend crate calls one function per part from its own tests:
//!
//! ```text
//! #[test]
//! fn inboxes() { inbox_backend_testkit::check_inbox_storage(MyBackend::fresh); }
//! #[test]
//! fn messages() { inbox_backend_testkit::check_message_storage(MyBackend::fresh); }
//! #[test]
//! fn section() { inbox_backend_testkit::check_exclusive(MyBackend::fresh); }
//! ```
//!
//! Each function takes a factory that answers a backend over a new, empty store on every call,
//! and makes its clones itself. A check that fails panics with one line,
//! `backend contract broken: <part>: <rule> (<what was seen>)`.
//!
//! The kit checks the ports and the section only. Dedup, expiry and every rule of a command are
//! the core's and the conformance suite's; the snapshots it stores are built through the generated
//! constructors and transitions, and no rule of the specification is restated here.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::fmt::{Debug, Display};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{
    Adapter, Address, AnyInbox, AnyMessage, Body, DedupKey, Inbox, InboxData, InboxSnapshot, Kind,
    Message, MessageData, MessageId, MessageSnapshot, Principal, Signature, ThreadId,
};
use inbox::primitives::Timestamp;
use inbox_core::Exclusive;

/// How long a check waits for a command that the contract lets run before it fails.
const BOUND: Duration = Duration::from_secs(2);

/// How long a second command has to get into a section another command holds, before the
/// check takes it as waiting.
const OVERLAP: Duration = Duration::from_millis(200);

/// Checks [`InboxStorage`] on backends `fresh` answers.
///
/// A snapshot stored with `put` is answered equal by `get`; a second `put` under one identity
/// replaces the first; `delete` removes the snapshot from `get` and from `list`; `list` answers
/// every stored snapshot once, in the same order on two calls with no write between them; two
/// clones of one backend read one store.
///
/// # Panics
///
/// With a line naming the rule the backend broke, or when `fresh` answers a store that is not
/// empty.
pub fn check_inbox_storage<B, F>(fresh: F)
where
    B: InboxStorage + Clone,
    F: FnMut() -> B,
{
    check_port::<Inboxes, B, F>(fresh);
}

/// Checks [`MessageStorage`] on backends `fresh` answers: the rules of
/// [`check_inbox_storage`], over messages.
///
/// # Panics
///
/// With a line naming the rule the backend broke, or when `fresh` answers a store that is not
/// empty.
pub fn check_message_storage<B, F>(fresh: F)
where
    B: MessageStorage + Clone,
    F: FnMut() -> B,
{
    check_port::<Messages, B, F>(fresh);
}

/// Checks [`Exclusive`] on backends `fresh` answers.
///
/// A second command over one backend waits while one runs; clones of one backend share the
/// section; the section is free again after a command that panics. A check that waits fails after
/// a bound of 2 s instead of hanging, so a passing call takes well under a second. A thread left
/// waiting on a broken section is not joined.
///
/// # Panics
///
/// With a line naming the rule the backend broke.
pub fn check_exclusive<B, F>(mut fresh: F)
where
    B: Exclusive + Clone + Send + Sync + 'static,
    F: FnMut() -> B,
{
    const WAITS: &str = "a second command waits while one runs";
    const SHARED: &str = "clones of one backend share the section";
    const FREED: &str = "the section is free again after a command that panics";

    let one = Arc::new(fresh());
    second_waits(Arc::clone(&one), Arc::clone(&one), WAITS);

    let first = Arc::new(fresh());
    let clone = Arc::new(first.as_ref().clone());
    second_waits(first, clone, SHARED);

    let panicking = Arc::new(fresh());
    let unwound: std::thread::Result<()> = catch_unwind(AssertUnwindSafe(|| {
        panicking.exclusive(|| {
            resume_unwind(Box::new(
                "inbox-backend-testkit: a command panics inside the section, on purpose",
            ))
        })
    }));
    if unwound.is_ok() {
        broken(
            "Exclusive",
            FREED,
            "the command panicked and exclusive answered as if it had not",
        );
    }
    let same = Arc::clone(&panicking);
    let clone = Arc::new(panicking.as_ref().clone());
    for (backend, which) in [(same, "the same backend"), (clone, "a clone")] {
        let (ran, done) = mpsc::channel();
        std::thread::spawn(move || {
            backend.exclusive(|| {
                let _ = ran.send(());
            });
        });
        if done.recv_timeout(BOUND).is_err() {
            broken(
                "Exclusive",
                FREED,
                format_args!(
                    "a command over {which} did not run within {BOUND:?} after a command \
                     panicked inside the section"
                ),
            );
        }
    }
}

/// Holds the section through `first` on one thread, and checks that a command through `second`,
/// on another thread, waits until the first leaves and then runs.
fn second_waits<B>(first: Arc<B>, second: Arc<B>, rule: &str)
where
    B: Exclusive + Send + Sync + 'static,
{
    let (entered, first_in) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        first.exclusive(|| {
            let _ = entered.send(());
            let _ = released.recv();
        });
    });
    if first_in.recv_timeout(BOUND).is_err() {
        broken(
            "Exclusive",
            rule,
            format_args!("the first command over a new store did not run within {BOUND:?}"),
        );
    }

    let (calling, second_calling) = mpsc::channel();
    let (ran, second_in) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = calling.send(());
        second.exclusive(|| {
            let _ = ran.send(());
        });
    });
    if second_calling.recv_timeout(BOUND).is_err() {
        broken(
            "Exclusive",
            rule,
            format_args!("the thread of the second command did not start within {BOUND:?}"),
        );
    }
    if second_in.recv_timeout(OVERLAP).is_ok() {
        broken(
            "Exclusive",
            rule,
            "a second command ran while the first held the section",
        );
    }
    drop(release);
    if second_in.recv_timeout(BOUND).is_err() {
        broken(
            "Exclusive",
            rule,
            format_args!(
                "the second command did not run within {BOUND:?} after the first left the section"
            ),
        );
    }
}

/// Fails a check: one line naming the part of the contract and the rule the backend broke.
#[track_caller]
fn broken(part: &str, rule: &str, seen: impl Display) -> ! {
    panic!("backend contract broken: {part}: {rule} ({seen})")
}

/// One storage port, seen the same way for both entities.
trait Port<B> {
    /// The port's name, as a failing check prints it.
    const PART: &'static str;
    /// The snapshot the port stores.
    type Row: Clone + PartialEq + Debug;
    /// The identity a row is stored under.
    type Id: Display;

    fn id(row: &Self::Row) -> Self::Id;
    fn get(backend: &B, id: &Self::Id) -> Option<Self::Row>;
    fn put(backend: &mut B, row: Self::Row);
    fn delete(backend: &mut B, id: &Self::Id);
    fn list(backend: &B) -> Vec<Self::Row>;

    /// A row with identity `n`, in the state the lifecycle starts it in.
    fn row(n: u8) -> Self::Row;
    /// Another row with identity `n`: another state and other fields.
    fn replacement(n: u8) -> Self::Row;
}

/// The checks the Outcome of the kit lists, over one port.
fn check_port<P, B, F>(mut fresh: F)
where
    P: Port<B>,
    B: Clone,
    F: FnMut() -> B,
{
    const GET: &str = "a snapshot stored with put is answered equal by get";
    const REPLACE: &str = "a second put under one identity replaces the first";
    const DELETE_GET: &str = "delete removes the snapshot from get";
    const DELETE_LIST: &str = "delete removes the snapshot from list";
    const ORDER: &str = "list answers in the same order on two calls with no write between them";
    const CLONES: &str = "two clones of one backend read one store";

    let mut backend = fresh();
    let found = P::list(&backend).len();
    assert!(
        found == 0,
        "inbox-backend-testkit: the factory must answer a backend over a new, empty store; \
         its {} list answered {found} snapshots",
        P::PART
    );

    let first = P::row(3);
    P::put(&mut backend, first.clone());
    answered_equal::<P, B>(&backend, &first, GET);

    let others = [P::row(1), P::row(2)];
    for row in &others {
        P::put(&mut backend, row.clone());
    }
    let replacement = P::replacement(3);
    P::put(&mut backend, replacement.clone());
    answered_equal::<P, B>(&backend, &replacement, REPLACE);

    let mut stored = vec![replacement.clone(), others[0].clone(), others[1].clone()];
    every_once::<P, B>(&backend, &stored);
    same_order::<P, B>(&backend, ORDER);

    let gone = others[0].clone();
    P::delete(&mut backend, &P::id(&gone));
    if let Some(row) = P::get(&backend, &P::id(&gone)) {
        broken(
            P::PART,
            DELETE_GET,
            format_args!("get still answers {row:?} after delete"),
        );
    }
    let gone_id = P::id(&gone).to_string();
    if P::list(&backend)
        .iter()
        .any(|row| P::id(row).to_string() == gone_id)
    {
        broken(
            P::PART,
            DELETE_LIST,
            format_args!("list still answers {} after delete", P::id(&gone)),
        );
    }
    stored.retain(|row| *row != gone);
    every_once::<P, B>(&backend, &stored);
    same_order::<P, B>(&backend, ORDER);

    let mut writer = fresh();
    let mut reader = writer.clone();
    let written = P::row(4);
    P::put(&mut writer, written.clone());
    match P::get(&reader, &P::id(&written)) {
        Some(row) if row == written => {}
        other => broken(
            P::PART,
            CLONES,
            format_args!("a clone's get answered {other:?} for a row the other clone put"),
        ),
    }
    if !P::list(&reader).contains(&written) {
        broken(
            P::PART,
            CLONES,
            "a clone's list does not answer a row the other clone put",
        );
    }
    let back = P::row(5);
    P::put(&mut reader, back.clone());
    if P::get(&writer, &P::id(&back)).as_ref() != Some(&back) {
        broken(
            P::PART,
            CLONES,
            "the original's get does not answer a row its clone put",
        );
    }
    P::delete(&mut reader, &P::id(&written));
    if P::get(&writer, &P::id(&written)).is_some() {
        broken(
            P::PART,
            CLONES,
            "the original's get still answers a row its clone deleted",
        );
    }
}

/// `get` answers `row` under its identity.
fn answered_equal<P: Port<B>, B>(backend: &B, row: &P::Row, rule: &str) {
    match P::get(backend, &P::id(row)) {
        Some(found) if found == *row => {}
        Some(found) => broken(
            P::PART,
            rule,
            format_args!("put {row:?}, get answered {found:?}"),
        ),
        None => broken(
            P::PART,
            rule,
            format_args!("put {}, get answered none", P::id(row)),
        ),
    }
}

/// `list` answers each row of `stored` once and nothing else.
fn every_once<P: Port<B>, B>(backend: &B, stored: &[P::Row]) {
    const RULE: &str = "list answers every stored snapshot once";
    let listed = P::list(backend);
    for row in stored {
        let times = listed.iter().filter(|found| *found == row).count();
        if times != 1 {
            broken(
                P::PART,
                RULE,
                format_args!("list answered {} {times} times", P::id(row)),
            );
        }
    }
    if let Some(extra) = listed.iter().find(|found| !stored.contains(found)) {
        broken(
            P::PART,
            RULE,
            format_args!(
                "list answered {}, which is not stored as that",
                P::id(extra)
            ),
        );
    }
}

/// Two calls of `list` with no write between them answer one order.
fn same_order<P: Port<B>, B>(backend: &B, rule: &str) {
    let once: Vec<String> = P::list(backend)
        .iter()
        .map(|r| P::id(r).to_string())
        .collect();
    let twice: Vec<String> = P::list(backend)
        .iter()
        .map(|r| P::id(r).to_string())
        .collect();
    if once != twice {
        broken(
            P::PART,
            rule,
            format_args!("first {once:?}, then {twice:?}"),
        );
    }
}

/// [`InboxStorage`] as a [`Port`].
struct Inboxes;

/// An inbox address, printed as its text.
struct AddressText(Address);

impl Display for AddressText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.0)
    }
}

fn inbox_data(n: u8, owner: &str, adapter: Adapter) -> InboxData {
    InboxData {
        address: Address(format!("agent:testkit/inbox-{n}")),
        owner: Principal(format!("agent:testkit/{owner}-{n}")),
        adapter,
    }
}

impl<B: InboxStorage> Port<B> for Inboxes {
    const PART: &'static str = "InboxStorage";
    type Row = InboxSnapshot;
    type Id = AddressText;

    fn id(row: &InboxSnapshot) -> AddressText {
        AddressText(row.data.address.clone())
    }

    fn get(backend: &B, id: &AddressText) -> Option<InboxSnapshot> {
        InboxStorage::get(backend, &id.0)
    }

    fn put(backend: &mut B, row: InboxSnapshot) {
        InboxStorage::put(backend, row);
    }

    fn delete(backend: &mut B, id: &AddressText) {
        InboxStorage::delete(backend, &id.0);
    }

    fn list(backend: &B) -> Vec<InboxSnapshot> {
        InboxStorage::list(backend)
    }

    fn row(n: u8) -> InboxSnapshot {
        AnyInbox::Open(Inbox::new(inbox_data(n, "owner", Adapter::Terminal))).snapshot()
    }

    fn replacement(n: u8) -> InboxSnapshot {
        AnyInbox::Closed(Inbox::new(inbox_data(n, "successor", Adapter::Slack)).close()).snapshot()
    }
}

/// [`MessageStorage`] as a [`Port`].
struct Messages;

/// A message id, printed as its text.
struct MessageIdText(MessageId);

impl Display for MessageIdText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.0)
    }
}

fn message_data(n: u8, body: &str) -> MessageData {
    let id = format!("01JKTM{n:020}");
    MessageData {
        message_id: MessageId(id.clone()),
        from: Principal("agent:testkit/sender".to_owned()),
        on_behalf_of: Principal("agent:testkit/sender".to_owned()),
        authority: None,
        to: Address("agent:testkit/inbox-1".to_owned()),
        kind: Kind::Note,
        thread: ThreadId(id),
        reply_shape: None,
        decision: None,
        default_on_silence: None,
        created: Timestamp("2026-01-05T09:00:00Z".to_owned()),
        expires: Timestamp("2026-01-06T09:00:00Z".to_owned()),
        dedup_key: DedupKey(format!("{body}-{n}")),
        body: Body(format!("{body} {n}.")),
        signature: Signature {
            key_id: "testkit-key".to_owned(),
            value: "not-a-signature".to_owned(),
        },
        answered_by: None,
        decided_option: None,
        reply: None,
        refusal: None,
    }
}

impl<B: MessageStorage> Port<B> for Messages {
    const PART: &'static str = "MessageStorage";
    type Row = MessageSnapshot;
    type Id = MessageIdText;

    fn id(row: &MessageSnapshot) -> MessageIdText {
        MessageIdText(row.data.message_id.clone())
    }

    fn get(backend: &B, id: &MessageIdText) -> Option<MessageSnapshot> {
        MessageStorage::get(backend, &id.0)
    }

    fn put(backend: &mut B, row: MessageSnapshot) {
        MessageStorage::put(backend, row);
    }

    fn delete(backend: &mut B, id: &MessageIdText) {
        MessageStorage::delete(backend, &id.0);
    }

    fn list(backend: &B) -> Vec<MessageSnapshot> {
        MessageStorage::list(backend)
    }

    fn row(n: u8) -> MessageSnapshot {
        AnyMessage::Sent(Message::new(message_data(n, "Message"))).snapshot()
    }

    fn replacement(n: u8) -> MessageSnapshot {
        AnyMessage::Delivered(Message::new(message_data(n, "Replacement")).deliver()).snapshot()
    }
}
