//! The in-memory backend: both generated storage ports over one shared store.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{Address, InboxSnapshot, MessageId, MessageSnapshot};

/// Every inbox and every message, in memory, behind the generated storage ports.
///
/// Clones share one store, and the store is `Send` and `Sync`, so a listener moves a clone to
/// another thread. `list` answers the rows in the order of their identity's text. Nothing
/// survives the process. It decides no rule: dedup and expiry are the obligations of
/// [`crate::Core`], which read and write through these ports.
#[derive(Debug, Clone, Default)]
pub struct MemoryBackend {
    store: Arc<Mutex<Store>>,
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
