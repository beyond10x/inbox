//! Planted backends, each breaking one rule of the backend contract, fail the kit with a line
//! naming that rule; the same backend without a fault passes it.

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use inbox::behaviour::{InboxStorage, MessageStorage};
use inbox::messaging::{Adapter, Address, InboxSnapshot, MessageId, MessageSnapshot};
use inbox_backend_testkit::{check_exclusive, check_inbox_storage, check_message_storage};
use inbox_core::{Exclusive, MemoryBackend};

/// The one rule a planted backend breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    GetAnswersOther,
    PutKeepsFirst,
    DeleteIsLost,
    DeleteLeavesList,
    ListRepeats,
    ListDropsOne,
    ListOrderShifts,
    ClonesCopyStore,
    ClonesWriteAside,
    NoSection,
    ClonesOwnSection,
    LockedAfterPanic,
}

/// `MemoryBackend` behind a wrapper that breaks one rule, or none.
#[derive(Debug)]
struct Planted {
    fault: Fault,
    rows: MemoryBackend,
    aside: Option<MemoryBackend>,
    buried: Arc<Mutex<BTreeSet<String>>>,
    lists: Arc<AtomicUsize>,
    section: Arc<Mutex<()>>,
    busy: Arc<AtomicBool>,
}

impl Planted {
    fn new(fault: Fault) -> Self {
        Self {
            fault,
            rows: MemoryBackend::new(),
            aside: None,
            buried: Arc::default(),
            lists: Arc::default(),
            section: Arc::default(),
            busy: Arc::default(),
        }
    }

    fn buried(&self) -> MutexGuard<'_, BTreeSet<String>> {
        self.buried.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn shape<T: Clone>(&self, mut rows: Vec<T>) -> Vec<T> {
        match self.fault {
            Fault::ListRepeats => {
                if let Some(first) = rows.first().cloned() {
                    rows.push(first);
                }
            }
            Fault::ListDropsOne => {
                rows.pop();
            }
            Fault::ListOrderShifts => {
                let call = self.lists.fetch_add(1, Ordering::SeqCst);
                if !rows.is_empty() {
                    let len = rows.len();
                    rows.rotate_left(call % len);
                }
            }
            _ => {}
        }
        rows
    }
}

impl Clone for Planted {
    fn clone(&self) -> Self {
        let rows = if self.fault == Fault::ClonesCopyStore {
            let mut copy = MemoryBackend::new();
            for row in InboxStorage::list(&self.rows) {
                InboxStorage::put(&mut copy, row);
            }
            for row in MessageStorage::list(&self.rows) {
                MessageStorage::put(&mut copy, row);
            }
            copy
        } else {
            self.rows.clone()
        };
        let section = if self.fault == Fault::ClonesOwnSection {
            Arc::default()
        } else {
            Arc::clone(&self.section)
        };
        let aside = if self.fault == Fault::ClonesWriteAside {
            Some(MemoryBackend::new())
        } else {
            self.aside.clone()
        };
        Self {
            fault: self.fault,
            rows,
            aside,
            buried: Arc::clone(&self.buried),
            lists: Arc::clone(&self.lists),
            section,
            busy: Arc::clone(&self.busy),
        }
    }
}

impl Exclusive for Planted {
    fn exclusive<R>(&self, command: impl FnOnce() -> R) -> R {
        match self.fault {
            Fault::NoSection => command(),
            Fault::LockedAfterPanic => {
                while self
                    .busy
                    .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                    .is_err()
                {
                    std::thread::sleep(Duration::from_millis(1));
                }
                let answer = command();
                self.busy.store(false, Ordering::SeqCst);
                answer
            }
            _ => {
                let _section = self.section.lock().unwrap_or_else(PoisonError::into_inner);
                command()
            }
        }
    }
}

impl InboxStorage for Planted {
    fn get(&self, identity: &Address) -> Option<InboxSnapshot> {
        if self.buried().contains(&format!("inbox {}", identity.0)) {
            return None;
        }
        let mut row = InboxStorage::get(&self.rows, identity)?;
        if self.fault == Fault::GetAnswersOther {
            row.data.adapter = if row.data.adapter == Adapter::Terminal {
                Adapter::Slack
            } else {
                Adapter::Terminal
            };
        }
        Some(row)
    }

    fn put(&mut self, snapshot: InboxSnapshot) {
        if let Some(aside) = self.aside.as_mut() {
            InboxStorage::put(aside, snapshot);
            return;
        }
        let key = format!("inbox {}", snapshot.data.address.0);
        if self.fault == Fault::PutKeepsFirst
            && InboxStorage::get(&self.rows, &snapshot.data.address).is_some()
        {
            return;
        }
        self.buried().remove(&key);
        InboxStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &Address) {
        if let Some(aside) = self.aside.as_mut() {
            InboxStorage::delete(aside, identity);
            return;
        }
        match self.fault {
            Fault::DeleteIsLost => {}
            Fault::DeleteLeavesList => {
                self.buried().insert(format!("inbox {}", identity.0));
            }
            _ => InboxStorage::delete(&mut self.rows, identity),
        }
    }

    fn list(&self) -> Vec<InboxSnapshot> {
        self.shape(InboxStorage::list(&self.rows))
    }
}

impl MessageStorage for Planted {
    fn get(&self, identity: &MessageId) -> Option<MessageSnapshot> {
        if self.buried().contains(&format!("message {}", identity.0)) {
            return None;
        }
        let mut row = MessageStorage::get(&self.rows, identity)?;
        if self.fault == Fault::GetAnswersOther {
            row.data.body.0.push_str(" (changed)");
        }
        Some(row)
    }

    fn put(&mut self, snapshot: MessageSnapshot) {
        if let Some(aside) = self.aside.as_mut() {
            MessageStorage::put(aside, snapshot);
            return;
        }
        let key = format!("message {}", snapshot.data.message_id.0);
        if self.fault == Fault::PutKeepsFirst
            && MessageStorage::get(&self.rows, &snapshot.data.message_id).is_some()
        {
            return;
        }
        self.buried().remove(&key);
        MessageStorage::put(&mut self.rows, snapshot);
    }

    fn delete(&mut self, identity: &MessageId) {
        if let Some(aside) = self.aside.as_mut() {
            MessageStorage::delete(aside, identity);
            return;
        }
        match self.fault {
            Fault::DeleteIsLost => {}
            Fault::DeleteLeavesList => {
                self.buried().insert(format!("message {}", identity.0));
            }
            _ => MessageStorage::delete(&mut self.rows, identity),
        }
    }

    fn list(&self) -> Vec<MessageSnapshot> {
        self.shape(MessageStorage::list(&self.rows))
    }
}

/// The message the kit panicked with, or the case fails because it passed.
fn failure(fault: Fault, check: fn(Fault)) -> String {
    let payload = match catch_unwind(AssertUnwindSafe(|| check(fault))) {
        Ok(()) => panic!("the kit passed a backend planted with {fault:?}"),
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

fn inboxes(fault: Fault) {
    check_inbox_storage(move || Planted::new(fault));
}

fn messages(fault: Fault) {
    check_message_storage(move || Planted::new(fault));
}

fn section(fault: Fault) {
    check_exclusive(move || Planted::new(fault));
}

/// Asserts that the kit fails `fault` on both storage ports, each with a line naming `rule`.
fn both_ports_fail(fault: Fault, rule: &str) {
    for (check, part) in [
        (inboxes as fn(Fault), "InboxStorage"),
        (messages as fn(Fault), "MessageStorage"),
    ] {
        let line = failure(fault, check);
        let expected = format!("backend contract broken: {part}: {rule}");
        assert!(
            line.starts_with(&expected),
            "{fault:?}: expected a line starting {expected:?}, got {line:?}"
        );
    }
}

fn section_fails(fault: Fault, rule: &str) {
    let line = failure(fault, section);
    let expected = format!("backend contract broken: Exclusive: {rule}");
    assert!(
        line.starts_with(&expected),
        "{fault:?}: expected a line starting {expected:?}, got {line:?}"
    );
}

#[test]
fn the_planted_backend_without_a_fault_passes() {
    inboxes(Fault::None);
    messages(Fault::None);
    section(Fault::None);
}

#[test]
fn a_delete_that_leaves_the_snapshot_in_list_fails() {
    both_ports_fail(
        Fault::DeleteLeavesList,
        "delete removes the snapshot from list",
    );
}

#[test]
fn a_get_that_answers_another_snapshot_fails() {
    both_ports_fail(
        Fault::GetAnswersOther,
        "a snapshot stored with put is answered equal by get",
    );
}

#[test]
fn a_put_that_keeps_the_first_snapshot_fails() {
    both_ports_fail(
        Fault::PutKeepsFirst,
        "a second put under one identity replaces the first",
    );
}

#[test]
fn a_delete_that_is_lost_fails() {
    both_ports_fail(Fault::DeleteIsLost, "delete removes the snapshot from get");
}

#[test]
fn a_list_that_repeats_a_snapshot_fails() {
    both_ports_fail(
        Fault::ListRepeats,
        "list answers every stored snapshot once",
    );
}

#[test]
fn a_list_that_drops_a_snapshot_fails() {
    both_ports_fail(
        Fault::ListDropsOne,
        "list answers every stored snapshot once",
    );
}

#[test]
fn a_list_whose_order_shifts_between_calls_fails() {
    both_ports_fail(
        Fault::ListOrderShifts,
        "list answers in the same order on two calls with no write between them",
    );
}

#[test]
fn clones_that_do_not_share_a_store_fail() {
    both_ports_fail(
        Fault::ClonesCopyStore,
        "two clones of one backend read one store",
    );
}

#[test]
fn clones_whose_writes_do_not_reach_the_original_fail() {
    both_ports_fail(
        Fault::ClonesWriteAside,
        "two clones of one backend read one store",
    );
    for check in [inboxes as fn(Fault), messages as fn(Fault)] {
        let line = failure(Fault::ClonesWriteAside, check);
        assert!(
            line.contains("the original's get answered None") && line.contains("after a clone put"),
            "the clone-to-original read did not catch it: {line:?}"
        );
    }
}

#[test]
fn a_section_that_lets_a_second_command_in_fails() {
    section_fails(Fault::NoSection, "a second command waits while one runs");
}

#[test]
fn clones_that_do_not_share_the_section_fail() {
    section_fails(
        Fault::ClonesOwnSection,
        "clones of one backend share the section",
    );
}

#[test]
fn a_section_left_locked_by_a_panic_fails() {
    section_fails(
        Fault::LockedAfterPanic,
        "the section is free again after a command that panics",
    );
}
