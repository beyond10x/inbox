//! Adversary cases, pass 2: backends that break the contract in ways the corrected checks do not
//! reach, and claims of the kit's documentation that its code does not keep.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{
    Adapter, Address, AnyInbox, Inbox, InboxData, InboxSnapshot, MessageId, MessageSnapshot,
    Principal,
};
use inbox_backend_testkit::{check_exclusive, check_inbox_storage, check_message_storage};
use inbox_core::{Exclusive, MemoryBackend};

/// The line the kit panicked with; panics itself when the kit passed.
fn failure(what: &str, check: impl FnOnce()) -> String {
    let payload = match catch_unwind(AssertUnwindSafe(check)) {
        Ok(()) => panic!("the kit passed {what}"),
        Err(payload) => payload,
    };
    if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_owned()
    } else {
        panic!("the kit panicked with a payload that is not text")
    }
}

fn assert_rule(line: &str, part: &str, rule: &str) {
    let expected = format!("backend contract broken: {part}: {rule}");
    assert!(
        line.starts_with(&expected),
        "expected a line starting {expected:?}, got {line:?}"
    );
}

fn inbox(n: u8, adapter: Adapter) -> InboxSnapshot {
    AnyInbox::Open(Inbox::new(InboxData {
        address: Address(format!("agent:adversary/inbox-{n}")),
        owner: Principal(format!("agent:adversary/owner-{n}")),
        adapter,
    }))
    .snapshot()
}

// ---------------------------------------------------------------------------------------------
// A handle that remembers what it wrote, and answers that before the store.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default, Clone)]
struct Wrote {
    inboxes: BTreeMap<String, Option<InboxSnapshot>>,
    messages: BTreeMap<String, Option<MessageSnapshot>>,
}

/// A shared store behind a per-handle write-through cache: a handle remembers each row it put or
/// deleted and answers its own last write before it asks the store. A clone copies the cache, as
/// `#[derive(Clone)]` does. A write through another handle leaves what this handle wrote in place.
#[derive(Debug, Default)]
struct Remembers {
    rows: MemoryBackend,
    wrote: Mutex<Wrote>,
}

impl Remembers {
    fn wrote(&self) -> MutexGuard<'_, Wrote> {
        self.wrote.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Clone for Remembers {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            wrote: Mutex::new(self.wrote().clone()),
        }
    }
}

fn overlay<T: Clone>(
    listed: Vec<T>,
    id: impl Fn(&T) -> String,
    wrote: &BTreeMap<String, Option<T>>,
) -> Vec<T> {
    let mut rows: BTreeMap<String, T> = listed.into_iter().map(|row| (id(&row), row)).collect();
    for (key, row) in wrote {
        match row {
            Some(row) => {
                rows.insert(key.clone(), row.clone());
            }
            None => {
                rows.remove(key);
            }
        }
    }
    rows.into_values().collect()
}

impl InboxStorage for Remembers {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        if let Some(row) = self.wrote().inboxes.get(&identity.0) {
            return row.clone();
        }
        InboxStorage::get(&self.rows, identity)
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        self.wrote()
            .inboxes
            .insert(snapshot.data.address.0.clone(), Some(snapshot.clone()));
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        self.wrote().inboxes.insert(identity.0.clone(), None);
        InboxStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        overlay(
            InboxStorage::list(&self.rows),
            |row: &InboxSnapshot| row.data.address.0.clone(),
            &self.wrote().inboxes,
        )
    }
}

impl MessageStorage for Remembers {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        if let Some(row) = self.wrote().messages.get(&identity.0) {
            return row.clone();
        }
        MessageStorage::get(&self.rows, identity)
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        self.wrote()
            .messages
            .insert(snapshot.data.message_id.0.clone(), Some(snapshot.clone()));
        MessageStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        self.wrote().messages.insert(identity.0.clone(), None);
        MessageStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        overlay(
            MessageStorage::list(&self.rows),
            |row: &MessageSnapshot| row.data.message_id.0.clone(),
            &self.wrote().messages,
        )
    }
}

/// The write-through cache breaks "two clones of one backend read one store": a handle that put a
/// row still answers it after a clone replaced it. The kit reads each row only through the handle
/// that did not write it, so it must still fail this backend.
#[test]
fn clones_that_answer_their_own_earlier_write_fail() {
    let mut first = Remembers::default();
    let mut second = first.clone();
    let row = inbox(1, Adapter::Terminal);
    InboxStorage::put(&mut first, row.clone());
    let replacement = inbox(1, Adapter::Slack);
    InboxStorage::put(&mut second, replacement.clone());
    assert_eq!(
        InboxStorage::get(&first, &row.data.address),
        Some(row.clone()),
        "the planted cache is not stale, the case proves nothing"
    );
    assert_eq!(
        InboxStorage::get(&second, &row.data.address),
        Some(replacement)
    );

    let line = failure(
        "a backend whose handles answer their own earlier write after a clone replaced it",
        || check_inbox_storage(Remembers::default),
    );
    assert_rule(
        &line,
        "InboxStorage",
        "two clones of one backend read one store",
    );
    let line = failure(
        "a backend whose handles answer their own earlier write after a clone replaced it",
        || check_message_storage(Remembers::default),
    );
    assert_rule(
        &line,
        "MessageStorage",
        "two clones of one backend read one store",
    );
}

// ---------------------------------------------------------------------------------------------
// A handle whose `list` is a cache and whose `get` asks the store.
// ---------------------------------------------------------------------------------------------

/// A shared store whose `get` is fresh, behind a per-handle cache of `list`: each clone starts
/// with no cached list, and a handle drops its cached lists on its own `put` and `delete` only.
#[derive(Debug, Default)]
struct ListedOnce {
    rows: MemoryBackend,
    inboxes: Mutex<Option<Vec<InboxSnapshot>>>,
    messages: Mutex<Option<Vec<MessageSnapshot>>>,
}

impl ListedOnce {
    fn forget(&self) {
        *self.inboxes.lock().unwrap_or_else(PoisonError::into_inner) = None;
        *self.messages.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

impl Clone for ListedOnce {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            ..Self::default()
        }
    }
}

impl InboxStorage for ListedOnce {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        InboxStorage::get(&self.rows, identity)
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        self.forget();
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        self.forget();
        InboxStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        self.inboxes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert_with(|| InboxStorage::list(&self.rows))
            .clone()
    }
}

impl MessageStorage for ListedOnce {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        MessageStorage::get(&self.rows, identity)
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        self.forget();
        MessageStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        self.forget();
        MessageStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        self.messages
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_or_insert_with(|| MessageStorage::list(&self.rows))
            .clone()
    }
}

/// Green on efe92d7; red on the mutant that removes the `list` half of the clone check
/// (`broken(` in `reads`, the second one): only that check sees a clone whose `get` is fresh and
/// whose `list` is stale, and no other case of the suite builds one.
#[test]
fn clones_whose_list_alone_is_stale_fail() {
    let probe = ListedOnce::default();
    let mut other = probe.clone();
    assert!(InboxStorage::list(&probe).is_empty());
    let row = inbox(1, Adapter::Terminal);
    InboxStorage::put(&mut other, row.clone());
    assert_eq!(InboxStorage::get(&probe, &row.data.address), Some(row));
    assert!(
        InboxStorage::list(&probe).is_empty(),
        "the planted list is not stale, the case proves nothing"
    );

    let line = failure("a backend whose clones list from a stale cache", || {
        check_inbox_storage(ListedOnce::default)
    });
    assert_rule(
        &line,
        "InboxStorage",
        "two clones of one backend read one store",
    );
    let line = failure("a backend whose clones list from a stale cache", || {
        check_message_storage(ListedOnce::default)
    });
    assert_rule(
        &line,
        "MessageStorage",
        "two clones of one backend read one store",
    );
}

// ---------------------------------------------------------------------------------------------
// A section that each handle renews for itself after a panic.
// ---------------------------------------------------------------------------------------------

/// Each handle holds its own slot pointing at the shared lock; a clone copies the pointer. A
/// handle that finds the lock poisoned by a panic puts a new lock in its own slot. Clones made
/// after that share the new lock; a clone made before the panic renews its slot to a lock of its
/// own, and from then on shares nothing with the original.
#[derive(Debug, Default)]
struct Renews {
    slot: Mutex<Arc<Mutex<()>>>,
}

impl Clone for Renews {
    fn clone(&self) -> Self {
        let lock = Arc::clone(&self.slot.lock().unwrap_or_else(PoisonError::into_inner));
        Self {
            slot: Mutex::new(lock),
        }
    }
}

impl Exclusive for Renews {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        let lock = {
            let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
            if slot.is_poisoned() {
                *slot = Arc::default();
            }
            Arc::clone(&slot)
        };
        let _held = lock.lock().unwrap_or_else(PoisonError::into_inner);
        command()
    }
}

fn panic_inside<B: Exclusive>(backend: &B) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        backend.exclusive(|| resume_unwind(Box::new("adversary: a command panics, on purpose")))
    }));
}

/// The renewing section breaks "clones of one backend share the section" after a panic: a clone
/// made before the panic runs while the original holds the section. A listener holds its clones
/// from before any command, so the kit must check those, and fail this section.
#[test]
fn clones_made_before_a_panic_still_share_the_section() {
    let original = Arc::new(Renews::default());
    let before = Arc::new(original.as_ref().clone());
    panic_inside(original.as_ref());

    let (entered, held) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let holder = Arc::clone(&original);
    let holding = std::thread::spawn(move || {
        holder.exclusive(|| {
            let _ = entered.send(());
            let _ = released.recv();
        });
    });
    held.recv_timeout(Duration::from_secs(2))
        .expect("the original did not enter");
    let overlapped = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&overlapped);
    let (ran, done) = mpsc::channel();
    let clone = Arc::clone(&before);
    std::thread::spawn(move || {
        clone.exclusive(|| seen.store(true, Ordering::SeqCst));
        let _ = ran.send(());
    });
    let _ = done.recv_timeout(Duration::from_millis(500));
    drop(release);
    holding.join().expect("the holder panicked");
    assert!(
        overlapped.load(Ordering::SeqCst),
        "the planted section still excludes the earlier clone, the case proves nothing"
    );

    let line = failure(
        "a section that clones made before a panic no longer share",
        || check_exclusive(Renews::default),
    );
    assert_rule(
        &line,
        "Exclusive",
        "clones of one backend share the section",
    );
}

// ---------------------------------------------------------------------------------------------
// The factory guard, against the sentence of the docs.
// ---------------------------------------------------------------------------------------------

/// A plain shared mutex: a correct section.
#[derive(Debug, Clone, Default)]
struct Locked {
    section: Arc<Mutex<()>>,
}

impl Exclusive for Locked {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        let _held = self.section.lock().unwrap_or_else(PoisonError::into_inner);
        command()
    }
}

/// `check_exclusive` documents what it does in place of the empty-store guard: "a section that
/// does not let the first command in within 2 s fails `a second command waits while one runs`".
/// On the factory's second call the kit says instead that a command panicked inside it.
#[test]
fn a_section_not_free_on_the_second_factory_call_fails_on_the_documented_rule() {
    let busy = Locked::default();
    let (entered, held) = mpsc::channel();
    let holder = busy.clone();
    std::thread::spawn(move || {
        holder.exclusive(|| {
            let _ = entered.send(());
            std::thread::sleep(Duration::from_secs(30));
        });
    });
    held.recv_timeout(Duration::from_secs(2))
        .expect("the holder did not enter");
    let mut calls = 0;
    let line = failure("a factory whose second backend's section is held", || {
        check_exclusive(move || {
            calls += 1;
            if calls == 1 {
                Locked::default()
            } else {
                busy.clone()
            }
        });
    });
    assert_rule(&line, "Exclusive", "a second command waits while one runs");
}
