// generated from inbox v1
// contract digest 5d0a606ec95b4f3d1fc798803ab374eda0253182924a6940cb5d87c35b449778
// do not edit: regenerate with `ess synthesize --layout crate`

//! Ephemeral stores for the generated network entry points; no durability.

/// Ephemeral storage of `inbox.messaging.Inbox`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryInboxStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::messaging::InboxSnapshot>>>);
impl crate::behaviour::InboxStorage for MemoryInboxStorage {
fn get(&self, identity: &crate::messaging::Address) -> Option<crate::messaging::InboxSnapshot> { let key = memory_key_1(identity); self.0.borrow().iter().find(|row| memory_key_1(&row.data.address) == key).cloned() }
fn put(&mut self, snapshot: crate::messaging::InboxSnapshot) { self.delete(&snapshot.data.address); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_1(&row.data.address).cmp(&memory_key_1(&other.data.address))); }
fn delete(&mut self, identity: &crate::messaging::Address) { let key = memory_key_1(identity); self.0.borrow_mut().retain(|row| memory_key_1(&row.data.address) != key); }
fn list(&self) -> Vec<crate::messaging::InboxSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::InboxStorage for MemoryPorts {
fn get(&self, identity: &crate::messaging::Address) -> Option<crate::messaging::InboxSnapshot> { crate::behaviour::InboxStorage::get(&self.inbox_storage, identity) }
fn put(&mut self, snapshot: crate::messaging::InboxSnapshot) { crate::behaviour::InboxStorage::put(&mut self.inbox_storage, snapshot); }
fn delete(&mut self, identity: &crate::messaging::Address) { crate::behaviour::InboxStorage::delete(&mut self.inbox_storage, identity); }
fn list(&self) -> Vec<crate::messaging::InboxSnapshot> { crate::behaviour::InboxStorage::list(&self.inbox_storage) }
}

/// Ephemeral storage of `inbox.messaging.Message`, shared by clones.
#[derive(Clone, Default)]
pub struct MemoryMessageStorage(std::rc::Rc<std::cell::RefCell<Vec<crate::messaging::MessageSnapshot>>>);
impl crate::behaviour::MessageStorage for MemoryMessageStorage {
fn get(&self, identity: &crate::messaging::MessageId) -> Option<crate::messaging::MessageSnapshot> { let key = memory_key_15(identity); self.0.borrow().iter().find(|row| memory_key_15(&row.data.message_id) == key).cloned() }
fn put(&mut self, snapshot: crate::messaging::MessageSnapshot) { self.delete(&snapshot.data.message_id); let mut rows = self.0.borrow_mut(); rows.push(snapshot); rows.sort_by(|row, other| memory_key_15(&row.data.message_id).cmp(&memory_key_15(&other.data.message_id))); }
fn delete(&mut self, identity: &crate::messaging::MessageId) { let key = memory_key_15(identity); self.0.borrow_mut().retain(|row| memory_key_15(&row.data.message_id) != key); }
fn list(&self) -> Vec<crate::messaging::MessageSnapshot> { self.0.borrow().clone() }
}
impl crate::behaviour::MessageStorage for MemoryPorts {
fn get(&self, identity: &crate::messaging::MessageId) -> Option<crate::messaging::MessageSnapshot> { crate::behaviour::MessageStorage::get(&self.message_storage, identity) }
fn put(&mut self, snapshot: crate::messaging::MessageSnapshot) { crate::behaviour::MessageStorage::put(&mut self.message_storage, snapshot); }
fn delete(&mut self, identity: &crate::messaging::MessageId) { crate::behaviour::MessageStorage::delete(&mut self.message_storage, identity); }
fn list(&self) -> Vec<crate::messaging::MessageSnapshot> { crate::behaviour::MessageStorage::list(&self.message_storage) }
}

/// Ports shared by generated components. Values disappear when the process exits.
#[derive(Clone, Default)]
pub struct MemoryPorts {
/// Storage of `inbox.messaging.Inbox`.
pub inbox_storage: MemoryInboxStorage,
/// Storage of `inbox.messaging.Message`.
pub message_storage: MemoryMessageStorage,
}

impl crate::behaviour::TryContext for MemoryPorts {
fn try_caller_inbox(&self) -> Result<Option<crate::messaging::Address>, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("caller attribute: inbox")) }
fn try_caller_principal(&self) -> Result<Option<crate::messaging::Principal>, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("caller attribute: principal")) }
fn try_external(&mut self, _command: crate::behaviour::ExternalCommand<'_>, _outcome: &'static str) -> Result<bool, crate::obligation::UnmetObligation> { Err(crate::behaviour::unmet_context("external branch answer")) }
}

fn memory_key_1(value: &crate::messaging::Address) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

fn memory_key_15(value: &crate::messaging::MessageId) -> MemoryKey { let _ = value; MemoryKey::Text((&value.0).clone()) }

// A common structural order; individual specifications use only a subset of the variants.
#[allow(dead_code)]
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum MemoryKey {
    Null,
    Boolean(bool),
    Integer(i64),
    Number(String),
    Text(String),
    Bytes(Vec<u8>),
    List(Vec<MemoryKey>),
    Map(Vec<(MemoryKey, MemoryKey)>),
    Optional(Option<Box<MemoryKey>>),
}

impl crate::messaging::obligations::ExpireBehavior for MemoryPorts {
fn expire(&mut self, input: crate::messaging::Expire) -> Result<crate::messaging::ExpireOutcome, crate::obligation::UnmetObligation> { crate::messaging::obligations::ExpireBehavior::expire(&mut crate::messaging::obligations::Unimplemented, input) }
}
impl crate::messaging::obligations::PostMessageBehavior for MemoryPorts {
fn post_message(&mut self, input: crate::messaging::PostMessage) -> Result<crate::messaging::PostMessageOutcome, crate::obligation::UnmetObligation> { crate::messaging::obligations::PostMessageBehavior::post_message(&mut crate::messaging::obligations::Unimplemented, input) }
}
