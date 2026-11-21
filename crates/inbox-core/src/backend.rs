//! The in-memory backend: both generated storage ports over one shared store.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::ThreadId;

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{Address, InboxSnapshot, MessageId, MessageSnapshot};

/// The command-level section of a backend: one command at a time per shared store.
///
/// Every command reads, decides and then writes through separate storage calls, so two commands
/// over one store at once can both decide on what neither has written yet: two posts with one
/// dedup key both stored, two answers to one message both answered. [`crate::handle`] runs every
/// command and view inside `exclusive`, from before its first read to after its last write.
///
/// The section is not the lock that guards the rows: a command's own storage calls run inside it
/// and must not block on it. An implementation lets one `command` run at a time across every
/// clone or handle that shares its store, and must not stay locked when `command` panics.
/// Generic, so not object-safe; a backend is a type parameter of [`crate::Core`].
///
/// A command's context, verifier or `PostExternals` must not call the entry over the same store:
/// that is `exclusive` again on the thread that holds the section, and an implementation either
/// refuses it at once or documents what it does instead.
pub trait Exclusive {
    /// Runs `command` while no other command runs over this store, and answers what it answers.
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R;
}

/// What the section's second entry on the thread that holds it panics with.
const REENTERED: &str = "inbox-core: a command's context, verifier or PostExternals \
    must not call the entry over the same store (the section of the store is already held by \
    this thread)";

/// Every inbox and every message, in memory, behind the generated storage ports.
///
/// Clones share one store, and the store is `Send` and `Sync`, so a listener moves a clone to
/// another thread. `list` answers the rows in the order of their identity's text. Nothing
/// survives the process. It decides no rule: dedup and expiry are the obligations of
/// [`crate::Core`], which read and write through these ports.
///
/// Its clones share one [`Exclusive`] section too, a lock apart from the one that guards the
/// rows.
#[derive(Debug, Clone, Default)]
pub struct MemoryBackend {
    store: Arc<Mutex<Store>>,
    section: Arc<Mutex<()>>,
    holder: Arc<Mutex<Option<ThreadId>>>,
}

/// The section is a mutex over nothing, shared by every clone, and beside it the thread that
/// holds it.
///
/// Another thread waits for the section. The thread that holds it, calling `exclusive` again over
/// the same store (a context, verifier or `PostExternals` that calls the entry from inside a
/// command), panics at once with a message naming that rule: it neither waits on itself nor runs
/// the inner command. The panic unwinds through the outer command before its write, so the store
/// is unchanged, and the section is free again.
///
/// A command that panics inside the section poisons it, and the next command takes it anyway:
/// nothing is guarded by it but the order of commands. The holder is cleared on the way out, by a
/// guard, a panic included. The rows are then as that command left them. Each command writes at
/// most one row (one `put` or one `delete`), so the store holds either all of its effect or none of
/// it, and its caller has no answer.
impl Exclusive for MemoryBackend {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        let me = std::thread::current().id();
        if *self.holder() == Some(me) {
            panic!("{REENTERED}");
        }
        let _section = self.section.lock().unwrap_or_else(PoisonError::into_inner);
        *self.holder() = Some(me);
        let _holding = Holding(self);
        command()
    }
}

/// Clears the holder of the section when the command leaves it, also by a panic.
struct Holding<'a>(&'a MemoryBackend);

impl Drop for Holding<'_> {
    fn drop(&mut self) {
        *self.0.holder() = None;
    }
}

#[derive(Debug, Default)]
struct Store {
    inboxes: BTreeMap<String, InboxSnapshot>,
    messages: BTreeMap<String, MessageSnapshot>,
}

impl MemoryBackend {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// The store, also after a thread panicked while holding it: every write replaces whole
    /// rows, so no half-written row is left behind.
    fn store(&self) -> MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The thread that holds the section, if one does.
    fn holder(&self) -> MutexGuard<'_, Option<ThreadId>> {
        self.holder.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl InboxStorage for MemoryBackend {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        self.store().inboxes.get(&identity.0).cloned()
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        self.store()
            .inboxes
            .insert(snapshot.data.address.0.clone(), snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        self.store().inboxes.remove(&identity.0);
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        self.store().inboxes.values().cloned().collect()
    }
}

impl MessageStorage for MemoryBackend {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        self.store().messages.get(&identity.0).cloned()
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        self.store()
            .messages
            .insert(snapshot.data.message_id.0.clone(), snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        self.store().messages.remove(&identity.0);
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        self.store().messages.values().cloned().collect()
    }
}
