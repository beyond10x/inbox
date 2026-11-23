//! The read of `Pending` with `undelivered=1` (`waited` in `src/service.rs`,
//! `docs/design/service.md`, the wait): its answer holds only the caller's rows in `Sent` that are
//! not due, so only such a row ends its wait. A row in `Delivered` sleeps as a due one does, a
//! `MarkDelivered` wakes it to read again and sleep on, and a post ends it. While the caller's
//! inbox is not open it is answered `409` at once, and a close ends a held one. The service reads a probe clock that logs which thread took
//! each reading, so a case knows when the wait sleeps: its first reading is the request's instant,
//! its second the one after its view read. Every key is created when the test runs. No case waits
//! for a bound: the waits ask for 30 s and a case fails after 10 s.

use crate::support;

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant as Monotonic};

use inbox::primitives::Timestamp;
use inbox_core::MemoryBackend;
use inbox_server::{Clock, ManualClock, Running};
use inbox_verify::Instant;
use support::{AGENT, OTHER, Reply, THIRD, World, encoded, ulid};

const POST: &str = "/messaging/commands/post";
const MARK_DELIVERED: &str = "/messaging/commands/mark-delivered";
const CLOSE: &str = "/messaging/commands/close-inbox";
/// A second after `LATER`, the `expires` of a note `send_note` posts: that note is due here.
const AFTER: &str = "2026-12-01T00:00:01Z";
/// A day after `AFTER`: a note that expires here is not due at `AFTER`.
const NEXT_DAY: &str = "2026-12-02T00:00:00Z";
/// How long a case waits for an answer before it calls the wait hung; the waits ask for 30 s.
const HANG: Duration = Duration::from_secs(10);

type Step = Box<dyn FnOnce() + Send>;

/// The world's [`ManualClock`], each reading logged with the thread that took it, and one step run
/// on the reading named by its number.
struct Probe {
    inner: Arc<ManualClock>,
    log: Mutex<Vec<ThreadId>>,
    step: Mutex<Option<(usize, Step)>>,
}

impl fmt::Debug for Probe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Probe")
    }
}

impl Probe {
    fn reads(&self) -> usize {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// The readings from `from` on taken by the thread that took reading `from`.
    fn reads_of_first_thread_since(&self, from: usize) -> usize {
        let log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(first) = log.get(from) else {
            return 0;
        };
        log[from..].iter().filter(|id| *id == first).count()
    }

    /// Runs `step` on the thread that takes reading number `at`, before it is answered.
    fn on_reading(&self, at: usize, step: Step) {
        *self.step.lock().unwrap_or_else(PoisonError::into_inner) = Some((at, step));
    }

    /// Blocks until the wait whose first reading is `from` has taken `count` readings.
    fn until(&self, from: usize, count: usize) {
        let started = Monotonic::now();
        while self.reads_of_first_thread_since(from) < count {
            assert!(
                started.elapsed() < HANG,
                "the wait took {} of {count} readings within {} s",
                self.reads_of_first_thread_since(from),
                HANG.as_secs()
            );
            thread::sleep(Duration::from_millis(2));
        }
    }
}

impl Clock for Probe {
    fn now(&self) -> Instant {
        let number = {
            let mut log = self.log.lock().unwrap_or_else(PoisonError::into_inner);
            log.push(thread::current().id());
            log.len() - 1
        };
        let step = {
            let mut held = self.step.lock().unwrap_or_else(PoisonError::into_inner);
            match held.as_ref() {
                Some((at, _)) if *at == number => held.take().map(|(_, step)| step),
                _ => None,
            }
        };
        if let Some(step) = step {
            step();
        }
        self.inner.now()
    }
}

/// A service over `world` that reads a probe clock, with a `max_wait` of 30 s, and OTHER's inbox
/// open.
fn start(world: &World) -> (Running<MemoryBackend>, Arc<Probe>) {
    let probe = Arc::new(Probe {
        inner: Arc::clone(&world.clock),
        log: Mutex::new(Vec::new()),
        step: Mutex::new(None),
    });
    let clock: Arc<dyn Clock> = probe.clone();
    let server = world.start(|config| {
        config.clock = clock;
        config.max_wait = Duration::from_secs(30);
    });
    world.open(server.local_addr(), OTHER);
    (server, probe)
}

/// Note `n` from AGENT to OTHER.
fn send_note(world: &World, address: SocketAddr, n: u32) {
    let post = world.post(AGENT, OTHER, n, "a note");
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&post));
    assert_eq!(reply.status, 202, "note {n}: {}", reply.body);
}

/// Note `n` from AGENT to OTHER that expires a day after `AFTER`: live after the clock moved there.
fn send_live_note(world: &World, address: SocketAddr, n: u32) {
    let mut post = world.post(AGENT, OTHER, n, "a live note");
    post.expires = Timestamp(NEXT_DAY.to_owned());
    world.sign(&mut post);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&post));
    assert_eq!(reply.status, 202, "note {n}: {}", reply.body);
}

/// OTHER's `MarkDelivered` of note `n`: answered, whatever its outcome.
fn mark(world: &World, address: SocketAddr, n: u32) -> Reply {
    let body = format!(r#"{{"message_id":"{}"}}"#, ulid(n));
    world.send(address, OTHER, "POST", MARK_DELIVERED, body.as_bytes())
}

/// OTHER's read of its `Pending` with `query` after `inbox=…`, on a thread.
fn pending_on(world: &Arc<World>, address: SocketAddr, query: &str) -> mpsc::Receiver<Reply> {
    let (sent, answered) = mpsc::channel();
    let world = Arc::clone(world);
    let target = format!("/messaging/views/pending?inbox={OTHER}{query}");
    thread::spawn(move || {
        let _ = sent.send(world.send(address, OTHER, "GET", &target, b""));
    });
    answered
}

fn state(row: &inbox::server::json::Value) -> String {
    match row.member("state") {
        Some(inbox::server::json::Value::Text(state)) => state.clone(),
        other => panic!("`state` is {other:?}"),
    }
}

/// Note 1 is in `Sent` when the wait arrives and is marked delivered at the request's instant,
/// before the view is read: the wait reads only a `Delivered` row and sleeps. A second
/// `MarkDelivered` wakes it; it reads again and sleeps on. Note 2's post ends it, and the answer
/// holds note 2 alone: the row in `Delivered` is left out.
#[test]
fn a_wait_with_undelivered_sleeps_over_a_delivered_row_and_a_mark_and_ends_on_a_post() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world);
    let address = server.local_addr();
    send_note(&world, address, 1);
    let from = probe.reads();
    let marker = Arc::clone(&world);
    probe.on_reading(
        from,
        Box::new(move || {
            let marked = mark(&marker, address, 1);
            assert_eq!(marked.status, 202, "{}", marked.body);
        }),
    );
    let answered = pending_on(&world, address, "&wait=30&undelivered=1");
    probe.until(from, 2);
    assert!(
        answered.try_recv().is_err(),
        "the wait ended on a row in Delivered"
    );
    let again = mark(&world, address, 1);
    assert_eq!(again.text("outcome"), "already-delivered", "{}", again.body);
    // The mark's answer woke the wait: it read its view again and took a third reading.
    probe.until(from, 3);
    send_note(&world, address, 2);
    let reply = answered.recv_timeout(HANG).expect("the post ends the wait");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(
        reply.ids(),
        [ulid(2)],
        "the row in Delivered is left out: {}",
        reply.body
    );
    let states: Vec<String> = reply.rows().iter().map(state).collect();
    assert_eq!(states, ["Sent"], "{}", reply.body);
}

/// A value of `undelivered` other than `1` is refused, with a `wait` and without one. With `1` and
/// no `wait`, or `wait=0`, the view is answered at once, without the row in `Delivered`; without the
/// parameter that row is listed and ends a wait at once as before.
#[test]
fn undelivered_is_1_or_refused_and_without_a_held_wait_or_without_it_answers_at_once() {
    let world = Arc::new(World::new());
    let (server, _probe) = start(&world);
    let address = server.local_addr();
    for query in [
        "&undelivered=2",
        "&wait=30&undelivered=0",
        "&wait=30&undelivered=true",
        "&wait=30&undelivered=",
        "&wait=30&undelivered=01",
    ] {
        let reply = pending_on(&world, address, query)
            .recv_timeout(HANG)
            .expect("a refusal is answered at once");
        assert_eq!(
            (reply.status, reply.body.as_str()),
            (400, r#"{"refused":"undelivered is not 1"}"#),
            "{query}"
        );
    }
    let twice = pending_on(&world, address, "&wait=30&undelivered=1&undelivered=1")
        .recv_timeout(HANG)
        .expect("a refusal is answered at once");
    assert_eq!(twice.status, 400, "{}", twice.body);
    for query in ["&undelivered=1", "&wait=0&undelivered=1"] {
        let reply = pending_on(&world, address, query)
            .recv_timeout(HANG)
            .unwrap_or_else(|_| panic!("{query}: an empty inbox is answered at once"));
        assert_eq!((reply.status, reply.ids()), (200, vec![]), "{query}");
    }
    send_note(&world, address, 1);
    assert_eq!(mark(&world, address, 1).status, 202);
    for (query, ids) in [
        ("&undelivered=1", vec![]),
        ("&wait=0&undelivered=1", vec![]),
        ("&wait=0", vec![ulid(1)]),
        ("&wait=30", vec![ulid(1)]),
    ] {
        let reply = pending_on(&world, address, query)
            .recv_timeout(HANG)
            .unwrap_or_else(|_| panic!("{query}: answered at once"));
        assert_eq!((reply.status, reply.ids()), (200, ids), "{query}");
    }
}

/// The refusal of a read with `undelivered=1` while the caller's own inbox is not open.
const NOT_OPEN: &str = r#"{"refused":"inbox not open"}"#;

/// THIRD's own inbox was never opened: a read with `undelivered=1` is answered `409` at once, held
/// or not, and one without it as before. OTHER's held read is ended by OTHER's close of its inbox,
/// with the same `409`, and the inbox stays closed.
#[test]
fn a_read_with_undelivered_is_never_held_for_an_inbox_that_is_not_open() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world);
    let address = server.local_addr();
    for query in ["&wait=30&undelivered=1", "&undelivered=1"] {
        let target = format!("/messaging/views/pending?inbox={THIRD}{query}");
        let reply = world.send(address, THIRD, "GET", &target, b"");
        assert_eq!(
            (reply.status, reply.body.as_str()),
            (409, NOT_OPEN),
            "{query}"
        );
    }
    let target = format!("/messaging/views/pending?inbox={THIRD}&wait=0");
    let plain = world.send(address, THIRD, "GET", &target, b"");
    assert_ne!(
        plain.body, NOT_OPEN,
        "without the parameter: {}",
        plain.body
    );

    let from = probe.reads();
    let answered = pending_on(&world, address, "&wait=30&undelivered=1");
    probe.until(from, 2);
    let body = format!(r#"{{"address":"{OTHER}"}}"#);
    let closed = world.send(address, OTHER, "POST", CLOSE, body.as_bytes());
    assert_eq!(closed.status, 202, "{}", closed.body);
    let reply = answered
        .recv_timeout(HANG)
        .expect("the close ends the wait");
    assert_eq!((reply.status, reply.body.as_str()), (409, NOT_OPEN));
    let again = pending_on(&world, address, "&undelivered=1")
        .recv_timeout(HANG)
        .expect("answered at once");
    assert_eq!((again.status, again.body.as_str()), (409, NOT_OPEN));
}

/// Note 1 is due at the service's clock, note 2 is in `Delivered`, note 3 is live in `Sent`: the
/// answer with `undelivered=1` holds note 3 alone, without a wait as with one; without the
/// parameter every row. With only the due and the delivered note a held read sleeps, and a post
/// ends it with that note alone.
#[test]
fn the_answer_with_undelivered_holds_neither_a_due_row_nor_one_in_delivered() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world);
    let address = server.local_addr();
    send_note(&world, address, 1);
    send_live_note(&world, address, 2);
    assert_eq!(mark(&world, address, 2).status, 202);
    world.clock.set(support::instant(AFTER));
    send_live_note(&world, address, 3);
    for (query, ids) in [
        ("&wait=0&undelivered=1", vec![ulid(3)]),
        ("&wait=30&undelivered=1", vec![ulid(3)]),
        ("&wait=0", vec![ulid(1), ulid(2), ulid(3)]),
    ] {
        let reply = pending_on(&world, address, query)
            .recv_timeout(HANG)
            .unwrap_or_else(|_| panic!("{query}: answered at once"));
        assert_eq!((reply.status, reply.ids()), (200, ids), "{query}");
    }
    assert_eq!(mark(&world, address, 3).status, 202);
    let from = probe.reads();
    let answered = pending_on(&world, address, "&wait=30&undelivered=1");
    probe.until(from, 2);
    assert!(
        answered.try_recv().is_err(),
        "the wait ended on a due row or one in Delivered"
    );
    send_live_note(&world, address, 4);
    let reply = answered.recv_timeout(HANG).expect("the post ends the wait");
    assert_eq!(reply.ids(), [ulid(4)], "{}", reply.body);
}
