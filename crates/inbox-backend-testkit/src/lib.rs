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
//! and makes its clones itself. The two storage checks refuse, with a panic naming the factory,
//! any backend the factory answers that holds a snapshot already. A check that fails panics with
//! one line, `backend contract broken: <part>: <rule> (<what was seen>)`.
//!
//! A check that waits fails after a bound of 2 s instead of hanging. The section check holds a
//! command for 500 ms while others try to get in, so a section that lets a second command in
//! only after a longer hold is not seen by the kit; a passing `check_exclusive` takes about 1 s.
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
use std::time::{Duration, Instant};

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{
    Adapter, Address, AnyInbox, AnyMessage, Body, DedupKey, Inbox, InboxData, InboxSnapshot, Kind,
    Message, MessageData, MessageId, MessageSnapshot, Principal, Signature, ThreadId,
};
use inbox::primitives::Timestamp;
use inbox_core::Exclusive;

/// How long a check waits for a command that the contract lets run before it fails.
const BOUND: Duration = Duration::from_secs(2);

/// How long the kit holds a command inside the section while other commands try to get in. A
/// section that admits a second command only after a longer hold is not seen by the kit.
const HOLD: Duration = Duration::from_millis(500);

/// Checks [`InboxStorage`] on backends `fresh` answers.
///
/// A snapshot stored with `put` is answered equal by `get`; a second `put` under one identity
/// replaces the first; `delete` removes the snapshot from `get` and from `list`; `list` answers
/// every stored snapshot once, in the same order on two calls with no write between them; two
/// clones of one backend read one store. Each handle reads before the other handle writes and
/// again after its `put`, its replacing `put` and its `delete`, with `get` and with `list`, in
/// both directions (original to clone, clone to original), so a handle that answers what it read
/// earlier fails.
///
/// # Panics
///
/// With a line naming the rule the backend broke, or when a backend `fresh` answers, on any call,
/// holds a snapshot already.
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
/// With a line naming the rule the backend broke, or when a backend `fresh` answers, on any call,
/// holds a snapshot already.
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
/// section; the section is free again after a command that panics, and a second command still
/// waits while one runs after it. The kit holds a command inside the section for 500 ms while a
/// command over the same backend and one over a clone try to get in; a section that admits a
/// second command only after a hold longer than 500 ms is not seen by the kit. Every other wait,
/// the command that panics included, fails after a bound of 2 s instead of hanging; a passing
/// call takes about 1 s. A thread left waiting on a broken section is not joined.
///
/// The factory's backends have no storage port here, so the kit cannot see whether one holds
/// rows; what it needs of each is that its section is free, and a section that does not let the
/// first command in within 2 s fails `a second command waits while one runs`.
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
    let clone = Arc::new(one.as_ref().clone());
    excludes(
        Arc::clone(&one),
        vec![(one, "the same backend", WAITS), (clone, "a clone", SHARED)],
        "",
    );

    let panicking = Arc::new(fresh());
    let (left, unwound) = mpsc::channel();
    let inside = Arc::clone(&panicking);
    std::thread::spawn(move || {
        let answer: std::thread::Result<()> = catch_unwind(AssertUnwindSafe(|| {
            inside.exclusive(|| {
                resume_unwind(Box::new(
                    "inbox-backend-testkit: a command panics inside the section, on purpose",
                ))
            })
        }));
        let _ = left.send(answer.is_err());
    });
    match unwound.recv_timeout(BOUND) {
        Ok(true) => {}
        Ok(false) => broken(
            "Exclusive",
            FREED,
            "the command panicked and exclusive answered as if it had not",
        ),
        Err(_) => broken(
            "Exclusive",
            FREED,
            format_args!(
                "exclusive did not return within {BOUND:?} from a command that panicked inside \
                 the section"
            ),
        ),
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
    let clone = Arc::new(panicking.as_ref().clone());
    excludes(
        Arc::clone(&panicking),
        vec![
            (panicking, "the same backend", WAITS),
            (clone, "a clone", WAITS),
        ],
        " after a command panicked inside the section",
    );
}

/// Holds the section through `holder` on one thread for [`HOLD`], while each of `waiters` tries
/// to get in on a thread of its own, then releases it; each waiter must stay out while it is held
/// and run within [`BOUND`] after.
///
/// A waiter that got in, or did not run after, fails with its own rule; when several did, the
/// first in `waiters` names the rule. A holder that does not get in fails with the first waiter's
/// rule. `when` ends every line it prints.
fn excludes<B>(holder: Arc<B>, waiters: Vec<(Arc<B>, &'static str, &'static str)>, when: &str)
where
    B: Exclusive + Send + Sync + 'static,
{
    let first_rule = waiters.first().map_or("", |waiter| waiter.2);
    let (entered, holder_in) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    std::thread::spawn(move || {
        holder.exclusive(|| {
            let _ = entered.send(());
            let _ = released.recv();
        });
    });
    if holder_in.recv_timeout(BOUND).is_err() {
        broken(
            "Exclusive",
            first_rule,
            format_args!("a first command did not run within {BOUND:?}{when}"),
        );
    }

    let (calling, all_calling) = mpsc::channel();
    let (ran, waiter_in) = mpsc::channel();
    for (index, (backend, _, _)) in waiters.iter().enumerate() {
        let backend = Arc::clone(backend);
        let calling = calling.clone();
        let ran = ran.clone();
        std::thread::spawn(move || {
            let _ = calling.send(());
            backend.exclusive(|| {
                let _ = ran.send(index);
            });
        });
    }
    for _ in &waiters {
        if all_calling.recv_timeout(BOUND).is_err() {
            broken(
                "Exclusive",
                first_rule,
                format_args!("the thread of a second command did not start within {BOUND:?}"),
            );
        }
    }

    let deadline = Instant::now() + HOLD;
    let mut inside = vec![false; waiters.len()];
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match waiter_in.recv_timeout(left) {
            Ok(index) => inside[index] = true,
            Err(_) => break,
        }
    }
    if let Some(index) = inside.iter().position(|got_in| *got_in) {
        let (_, which, rule) = waiters[index];
        broken(
            "Exclusive",
            rule,
            format_args!(
                "a second command over {which} ran while a first held the section for \
                 {HOLD:?}{when}"
            ),
        );
    }

    drop(release);
    let deadline = Instant::now() + BOUND;
    while inside.contains(&false) {
        let Some(left) = deadline.checked_duration_since(Instant::now()) else {
            break;
        };
        match waiter_in.recv_timeout(left) {
            Ok(index) => inside[index] = true,
            Err(_) => break,
        }
    }
    if let Some(index) = inside.iter().position(|got_in| !*got_in) {
        let (_, which, rule) = waiters[index];
        broken(
            "Exclusive",
            rule,
            format_args!(
                "a second command over {which} did not run within {BOUND:?} after the first left \
                 the section{when}"
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

    let mut calls = 0;
    let mut fresh = || {
        calls += 1;
        let backend = fresh();
        let found = P::list(&backend).len();
        assert!(
            found == 0,
            "inbox-backend-testkit: the factory must answer a backend over a new, empty store; \
             on call {calls} its {} list answered {found} snapshots",
            P::PART
        );
        backend
    };

    let mut backend = fresh();

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

    let mut original = fresh();
    let mut clone = original.clone();
    one_store::<P, B>(&mut original, &mut clone, 4, "the original", "its clone");
    one_store::<P, B>(&mut clone, &mut original, 5, "a clone", "the original");
}

/// Writes row `n` through `writer` and reads it through `reader` before the first write and
/// after each: its `put`, a replacing `put`, its `delete`, with `get` and with `list`. `w` and `r`
/// name the two handles in the line a failure prints.
fn one_store<P: Port<B>, B>(writer: &mut B, reader: &mut B, n: u8, w: &str, r: &str) {
    let row = P::row(n);
    let id = P::id(&row);
    let _ = P::get(reader, &id);
    let _ = P::list(reader);

    P::put(writer, row.clone());
    reads::<P, B>(reader, &id, Some(&row), r, &format!("after {w} put it"));

    let replacement = P::replacement(n);
    P::put(writer, replacement.clone());
    reads::<P, B>(
        reader,
        &id,
        Some(&replacement),
        r,
        &format!("after {w} replaced it"),
    );

    P::delete(writer, &id);
    reads::<P, B>(reader, &id, None, r, &format!("after {w} deleted it"));
}

/// `reader` answers `expected` under `id` through `get`, and through `list` exactly that row, or
/// none under `id` when `expected` is none.
fn reads<P: Port<B>, B>(reader: &B, id: &P::Id, expected: Option<&P::Row>, r: &str, step: &str) {
    const CLONES: &str = "two clones of one backend read one store";
    let got = P::get(reader, id);
    if got.as_ref() != expected {
        broken(
            P::PART,
            CLONES,
            format_args!("{r}'s get answered {got:?} for {id} {step}"),
        );
    }
    let key = id.to_string();
    let listed: Vec<P::Row> = P::list(reader)
        .into_iter()
        .filter(|row| P::id(row).to_string() == key)
        .collect();
    if listed.iter().collect::<Vec<_>>() != expected.into_iter().collect::<Vec<_>>() {
        broken(
            P::PART,
            CLONES,
            format_args!("{r}'s list answered {listed:?} under {id} {step}"),
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
