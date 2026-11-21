//! `MemoryBackend` passes the kit: one call per part of the backend contract.

use inbox_core::MemoryBackend;

#[test]
fn memory_backend_keeps_the_inbox_storage_contract() {
    inbox_backend_testkit::check_inbox_storage(MemoryBackend::new);
}

#[test]
fn memory_backend_keeps_the_message_storage_contract() {
    inbox_backend_testkit::check_message_storage(MemoryBackend::new);
}

#[test]
fn memory_backend_keeps_the_exclusive_contract() {
    inbox_backend_testkit::check_exclusive(MemoryBackend::new);
}
