//! `MemoryBackend` against the generated storage ports: clones share one store, it is `Send`,
//! and `get`, `put`, `delete` and `list` keep the port's contract.

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{
    Adapter, Address, AnyInbox, AnyMessage, Body, DedupKey, Inbox, InboxData, Kind, Message,
    MessageData, MessageId, MessageSnapshot, Principal, Signature, ThreadId,
};
use inbox::primitives::Timestamp;
use inbox_core::MemoryBackend;

const _: fn() = || {
    fn send<T: Send + Sync>() {}
    send::<MemoryBackend>();
};

fn message(id: &str, dedup: &str) -> MessageSnapshot {
    AnyMessage::Sent(Message::new(MessageData {
        message_id: MessageId(id.to_owned()),
        from: Principal("agent:ws/planner".to_owned()),
        on_behalf_of: Principal("agent:ws/planner".to_owned()),
        authority: None,
        to: Address("agent:box/worker".to_owned()),
        kind: Kind::Note,
        thread: ThreadId(id.to_owned()),
        reply_shape: None,
        decision: None,
        default_on_silence: None,
        created: Timestamp("2026-01-05T09:00:00Z".to_owned()),
        expires: Timestamp("2026-01-06T09:00:00Z".to_owned()),
        dedup_key: DedupKey(dedup.to_owned()),
        body: Body("Build 512 is green.".to_owned()),
        signature: Signature {
            key_id: "key-1".to_owned(),
            value: "c2lnbmF0dXJl".to_owned(),
        },
        answered_by: None,
        decided_option: None,
        reply: None,
        refusal: None,
    }))
    .snapshot()
}

fn ids(backend: &MemoryBackend) -> Vec<String> {
    MessageStorage::list(backend)
        .into_iter()
        .map(|row| row.data.message_id.0)
        .collect()
}

#[test]
fn clones_share_one_store() {
    let mut writer = MemoryBackend::new();
    let reader = writer.clone();
    MessageStorage::put(&mut writer, message("01JGT0BK000000000000000001", "a"));
    let inbox = AnyInbox::Open(Inbox::new(InboxData {
        address: Address("agent:box/worker".to_owned()),
        owner: Principal("agent:box/worker".to_owned()),
        adapter: Adapter::Terminal,
    }))
    .snapshot();
    InboxStorage::put(&mut writer, inbox.clone());

    let id = MessageId("01JGT0BK000000000000000001".to_owned());
    assert!(MessageStorage::get(&reader, &id).is_some());
    assert_eq!(
        InboxStorage::get(&reader, &Address("agent:box/worker".to_owned())),
        Some(inbox)
    );

    let mut deleter = reader.clone();
    MessageStorage::delete(&mut deleter, &id);
    assert!(MessageStorage::get(&writer, &id).is_none());
}

#[test]
fn a_clone_moved_to_another_thread_writes_the_same_store() {
    let backend = MemoryBackend::new();
    let mut moved = backend.clone();
    std::thread::spawn(move || {
        MessageStorage::put(&mut moved, message("01JGT0BK000000000000000002", "b"));
    })
    .join()
    .expect("the thread ran");
    assert_eq!(ids(&backend), vec!["01JGT0BK000000000000000002".to_owned()]);
}

#[test]
fn put_replaces_delete_removes_and_list_answers_every_row() {
    let mut backend = MemoryBackend::new();
    MessageStorage::put(&mut backend, message("01JGT0BK000000000000000003", "c"));
    MessageStorage::put(&mut backend, message("01JGT0BK000000000000000001", "a"));
    MessageStorage::put(&mut backend, message("01JGT0BK000000000000000002", "b"));
    MessageStorage::put(
        &mut backend,
        message("01JGT0BK000000000000000002", "replaced"),
    );
    assert_eq!(
        ids(&backend),
        vec![
            "01JGT0BK000000000000000001".to_owned(),
            "01JGT0BK000000000000000002".to_owned(),
            "01JGT0BK000000000000000003".to_owned(),
        ]
    );
    let replaced = MessageStorage::get(
        &backend,
        &MessageId("01JGT0BK000000000000000002".to_owned()),
    )
    .expect("stored");
    assert_eq!(replaced.data.dedup_key, DedupKey("replaced".to_owned()));

    MessageStorage::delete(
        &mut backend,
        &MessageId("01JGT0BK000000000000000001".to_owned()),
    );
    MessageStorage::delete(
        &mut backend,
        &MessageId("01JGT0BK000000000000000009".to_owned()),
    );
    assert_eq!(
        ids(&backend),
        vec![
            "01JGT0BK000000000000000002".to_owned(),
            "01JGT0BK000000000000000003".to_owned(),
        ]
    );
}
