//! Adversary cases: backends that break the contract in ways the kit's checks do not reach, and
//! inputs the kit's own documentation says it refuses.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

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

// ---------------------------------------------------------------------------------------------
// A handle that caches what it read, and forgets it only on its own writes.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Cache {
    inboxes: BTreeMap<String, Option<InboxSnapshot>>,
    messages: BTreeMap<String, Option<MessageSnapshot>>,
    inbox_list: Option<Vec<InboxSnapshot>>,
    message_list: Option<Vec<MessageSnapshot>>,
}

/// A shared store behind a per-handle read cache: each clone starts with an empty cache, and a
/// handle clears its cache on its own `put` and `delete` only. A write through another handle
/// leaves what this handle read in place.
#[derive(Debug)]
struct Cached {
    rows: MemoryBackend,
    cache: Mutex<Cache>,
}

impl Cached {
    fn new() -> Self {
        Self {
            rows: MemoryBackend::new(),
            cache: Mutex::default(),
        }
    }

    fn cache(&self) -> std::sync::MutexGuard<'_, Cache> {
        self.cache.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Clone for Cached {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            cache: Mutex::default(),
        }
    }
}

impl InboxStorage for Cached {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        let mut cache = self.cache();
        cache
            .inboxes
            .entry(identity.0.clone())
            .or_insert_with(|| InboxStorage::get(&self.rows, identity))
            .clone()
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        *self.cache() = Cache::default();
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        *self.cache() = Cache::default();
        InboxStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        let mut cache = self.cache();
        cache
            .inbox_list
            .get_or_insert_with(|| InboxStorage::list(&self.rows))
            .clone()
    }
}

impl MessageStorage for Cached {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        let mut cache = self.cache();
        cache
            .messages
            .entry(identity.0.clone())
            .or_insert_with(|| MessageStorage::get(&self.rows, identity))
            .clone()
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        *self.cache() = Cache::default();
        MessageStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        *self.cache() = Cache::default();
        MessageStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        let mut cache = self.cache();
        cache
            .message_list
            .get_or_insert_with(|| MessageStorage::list(&self.rows))
            .clone()
    }
}

/// The cached backend does break "two clones of one backend read one store": a row one clone
/// deleted is still answered by the other clone that had read it. The kit must fail it.
#[test]
fn clones_that_read_a_stale_cache_fail() {
    let probe = Cached::new();
    let mut other = probe.clone();
    let row = AnyInbox::Open(Inbox::new(InboxData {
        address: Address("agent:adversary/inbox-1".to_owned()),
        owner: Principal("agent:adversary/owner-1".to_owned()),
        adapter: Adapter::Terminal,
    }))
    .snapshot();
    InboxStorage::put(&mut other, row.clone());
    assert!(InboxStorage::get(&probe, &row.data.address).is_some());
    InboxStorage::delete(&mut other, &row.data.address);
    assert!(
        InboxStorage::get(&probe, &row.data.address).is_some(),
        "the planted cache is not stale, the case proves nothing"
    );

    let line = failure(
        "a backend whose clones read a stale per-handle cache",
        || {
            check_inbox_storage(Cached::new);
        },
    );
    assert_rule(
        &line,
        "InboxStorage",
        "two clones of one backend read one store",
    );
    let line = failure(
        "a backend whose clones read a stale per-handle cache",
        || {
            check_message_storage(Cached::new);
        },
    );
    assert_rule(
        &line,
        "MessageStorage",
        "two clones of one backend read one store",
    );
}

// ---------------------------------------------------------------------------------------------
// A section with a lease: a second command gets in once the first has held it 300 ms.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct Leased {
    since: Arc<Mutex<Option<(u64, Instant)>>>,
    next: Arc<Mutex<u64>>,
}

const LEASE: Duration = Duration::from_millis(300);

impl Exclusive for Leased {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        let token = {
            let mut next = self.next.lock().unwrap_or_else(PoisonError::into_inner);
            *next += 1;
            *next
        };
        loop {
            {
                let mut since = self.since.lock().unwrap_or_else(PoisonError::into_inner);
                let free = match *since {
                    None => true,
                    Some((_, at)) => at.elapsed() >= LEASE,
                };
                if free {
                    *since = Some((token, Instant::now()));
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        struct Release<'a>(&'a Leased, u64);
        impl Drop for Release<'_> {
            fn drop(&mut self) {
                let mut since = self.0.since.lock().unwrap_or_else(PoisonError::into_inner);
                if matches!(*since, Some((holder, _)) if holder == self.1) {
                    *since = None;
                }
            }
        }
        let _release = Release(self, token);
        command()
    }
}

/// The leased section does let a second command in while the first runs, once the first has
/// run 300 ms. The kit must fail it.
#[test]
fn a_section_whose_lease_expires_after_300_ms_fails() {
    let section = Leased::default();
    let inside = Arc::new(AtomicBool::new(false));
    let overlapped = Arc::new(AtomicBool::new(false));
    let holder = {
        let section = section.clone();
        let inside = Arc::clone(&inside);
        std::thread::spawn(move || {
            section.exclusive(|| {
                inside.store(true, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(800));
                inside.store(false, Ordering::SeqCst);
            });
        })
    };
    std::thread::sleep(Duration::from_millis(50));
    section.exclusive(|| {
        if inside.load(Ordering::SeqCst) {
            overlapped.store(true, Ordering::SeqCst);
        }
    });
    holder.join().expect("the holder thread");
    assert!(
        overlapped.load(Ordering::SeqCst),
        "the planted lease did not let a second command in, the case proves nothing"
    );

    let line = failure("a section whose lease expires after 300 ms", || {
        check_exclusive(Leased::default);
    });
    assert_rule(&line, "Exclusive", "a second command waits while one runs");
}

// ---------------------------------------------------------------------------------------------
// A section that stops excluding once a command panicked inside it.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct ForgetsAfterPanic {
    section: Arc<Mutex<()>>,
    lost: Arc<AtomicBool>,
}

impl Exclusive for ForgetsAfterPanic {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        if self.lost.load(Ordering::SeqCst) {
            return command();
        }
        struct Mark<'a>(&'a AtomicBool);
        impl Drop for Mark<'_> {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    self.0.store(true, Ordering::SeqCst);
                }
            }
        }
        let _section = self.section.lock().unwrap_or_else(PoisonError::into_inner);
        let _mark = Mark(&self.lost);
        command()
    }
}

/// After one panic the section lets every command in at once: it is free again, and no longer a
/// section. The kit must fail it, on the rule that a second command waits while one runs.
#[test]
fn a_section_that_stops_excluding_after_a_panic_fails() {
    let line = failure("a section that stops excluding after a panic", || {
        check_exclusive(ForgetsAfterPanic::default);
    });
    assert!(
        line.starts_with("backend contract broken: Exclusive: "),
        "expected a line naming an Exclusive rule, got {line:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// A section whose way out after a panic blocks: the kit's own claim is that it does not hang.
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct BlocksAfterPanic {
    section: Arc<Mutex<()>>,
}

impl Exclusive for BlocksAfterPanic {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        let _section = self.section.lock().unwrap_or_else(PoisonError::into_inner);
        match catch_unwind(AssertUnwindSafe(command)) {
            Ok(answer) => answer,
            Err(_) => loop {
                std::thread::sleep(Duration::from_secs(3600));
            },
        }
    }
}

/// "A check that waits fails after a bound of 2 s instead of hanging": the kit must answer
/// (pass or fail) within 10 s for a section that never returns from a panicking command.
#[test]
fn the_kit_does_not_hang_on_a_section_that_blocks_after_a_panic() {
    let (done, answered) = mpsc::channel();
    std::thread::spawn(move || {
        let outcome = catch_unwind(|| check_exclusive(BlocksAfterPanic::default));
        let _ = done.send(outcome.is_err());
    });
    assert!(
        answered.recv_timeout(Duration::from_secs(10)).is_ok(),
        "check_exclusive did not answer within 10 s: it waits without a bound on the command \
         that panics"
    );
}

// ---------------------------------------------------------------------------------------------
// The factory guard.
// ---------------------------------------------------------------------------------------------

/// "Panics ... when fresh answers a store that is not empty": a factory answering clones of one
/// store answers a store that is not empty from its second call on.
#[test]
fn a_factory_that_answers_one_store_on_every_call_is_refused() {
    let one = MemoryBackend::new();
    let line = failure("a factory that answers one store on every call", || {
        check_inbox_storage(|| one.clone());
    });
    assert!(
        line.contains("the factory must answer a backend over a new, empty store"),
        "expected the factory guard, got {line:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Guards for checks no planted case of the unit reaches (green now; red on the mutant named).
// ---------------------------------------------------------------------------------------------

/// A list that also answers the replaced snapshot, as a replay of an append-only stream does.
/// Caught today only by the "not stored as that" branch of `every_once`.
#[derive(Debug, Clone, Default)]
struct KeepsReplaced {
    rows: MemoryBackend,
    history: Arc<Mutex<Vec<InboxSnapshot>>>,
}

impl InboxStorage for KeepsReplaced {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        InboxStorage::get(&self.rows, identity)
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        if let Some(old) = InboxStorage::get(&self.rows, &snapshot.data.address) {
            self.history
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(old);
        }
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        InboxStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        let mut rows = InboxStorage::list(&self.rows);
        rows.extend(
            self.history
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .iter()
                .cloned(),
        );
        rows
    }
}

#[test]
fn a_list_that_keeps_the_replaced_snapshot_fails() {
    let line = failure("a list that keeps the replaced snapshot", || {
        check_inbox_storage(KeepsReplaced::default);
    });
    assert_rule(
        &line,
        "InboxStorage",
        "list answers every stored snapshot once",
    );
}

/// A get that answers none for what was put: caught today only by the `None` branch of
/// `answered_equal`.
#[derive(Debug, Clone, Default)]
struct GetForgets {
    rows: MemoryBackend,
}

impl InboxStorage for GetForgets {
    fn get(&self, _identity: &Address) -> Option<InboxSnapshot> {
        None
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        InboxStorage::delete(&mut self.rows, identity);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        InboxStorage::list(&self.rows)
    }
}

#[test]
fn a_get_that_answers_none_fails() {
    let line = failure("a get that answers none", || {
        check_inbox_storage(GetForgets::default);
    });
    assert_rule(
        &line,
        "InboxStorage",
        "a snapshot stored with put is answered equal by get",
    );
}

/// A section that is released only when a command panics: after a command that returns, it
/// stays held. Caught on the rule it breaks only by the check after the first command leaves
/// (`second_waits`, "did not run ... after the first left the section").
#[derive(Debug, Clone, Default)]
struct NeverReleased {
    held: Arc<AtomicBool>,
}

impl Exclusive for NeverReleased {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        while self
            .held
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            std::thread::sleep(Duration::from_millis(1));
        }
        struct OnPanic<'a>(&'a AtomicBool);
        impl Drop for OnPanic<'_> {
            fn drop(&mut self) {
                if std::thread::panicking() {
                    self.0.store(false, Ordering::SeqCst);
                }
            }
        }
        let _on_panic = OnPanic(&self.held);
        command()
    }
}

#[test]
fn a_section_held_after_a_command_that_returns_fails() {
    let line = failure("a section held after a command that returns", || {
        check_exclusive(NeverReleased::default);
    });
    assert_rule(&line, "Exclusive", "a second command waits while one runs");
}

/// A delete that hides the row from the handle that deleted it only: a per-handle tombstone.
/// Caught today only by the last clone check ("the original's get still answers a row its clone
/// deleted").
#[derive(Debug, Default)]
struct LocalDelete {
    rows: MemoryBackend,
    hidden: Mutex<std::collections::BTreeSet<String>>,
}

impl Clone for LocalDelete {
    fn clone(&self) -> Self {
        Self {
            rows: self.rows.clone(),
            hidden: Mutex::default(),
        }
    }
}

impl LocalDelete {
    fn hidden(&self, identity: &str) -> bool {
        self.hidden
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains(identity)
    }
}

impl InboxStorage for LocalDelete {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        if self.hidden(&identity.0) {
            return None;
        }
        InboxStorage::get(&self.rows, identity)
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        self.hidden
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&snapshot.data.address.0);
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        self.hidden
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(identity.0.clone());
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        InboxStorage::list(&self.rows)
            .into_iter()
            .filter(|row| !self.hidden(&row.data.address.0))
            .collect()
    }
}

#[test]
fn a_delete_that_hides_the_row_from_its_own_handle_only_fails() {
    let line = failure(
        "a delete that hides the row from its own handle only",
        || {
            check_inbox_storage(LocalDelete::default);
        },
    );
    assert_rule(
        &line,
        "InboxStorage",
        "two clones of one backend read one store",
    );
}
