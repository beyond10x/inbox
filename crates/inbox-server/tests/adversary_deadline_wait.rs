//! Adversary pass 2 on story:a-due-message-takes-no-mark: the wait on `Pending` that a due row
//! does not end (`waited` in `src/service.rs`). The service reads a probe clock: the test's
//! [`ManualClock`] behind a log of which thread read it and when, so a case can tell where a wait
//! is (its first reading is the request's instant, its second the reading after its view read,
//! taken before it sleeps) and can run a step at that point. Every key is created when the test
//! runs. No case waits on real time longer than its bound: 300 ms, or the request's own 1 s.

use crate::support;

use std::fmt;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant as Monotonic};

use inbox::primitives::Timestamp;
use inbox_core::MemoryBackend;
use inbox_server::{Clock, ManualClock, Running, format_reading};
use inbox_verify::Instant;
use support::{AGENT, LATER, OTHER, THIRD, World, encoded, instant, ulid};

const POST: &str = "/messaging/commands/post";
/// A second after `LATER`, the `expires` of a note `send_note` posts by default.
const AFTER: &str = "2026-12-01T00:00:01Z";
/// A day after `AFTER`: a note that expires here is not due at `AFTER`.
const NEXT_DAY: &str = "2026-12-02T00:00:00Z";
/// The service's bound of a wait in the cases that wait for it.
const BOUND: Duration = Duration::from_millis(300);

type Step = Box<dyn FnOnce() + Send>;

/// The service's clock in these cases: the world's [`ManualClock`], each reading logged with the
/// thread that took it, and one step run on the reading named by its number.
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
    fn new(inner: Arc<ManualClock>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            log: Mutex::new(Vec::new()),
            step: Mutex::new(None),
        })
    }

    /// The count of readings so far.
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

    /// Runs `step` on the thread that takes reading number `at` (0-based), before that reading
    /// is answered.
    fn on_reading(&self, at: usize, step: Step) {
        *self.step.lock().unwrap_or_else(PoisonError::into_inner) = Some((at, step));
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

/// A started service over `world` that reads a probe clock, with `max_wait`, and OTHER's inbox
/// open.
fn start(world: &World, max_wait: Duration) -> (Running<MemoryBackend>, Arc<Probe>) {
    let probe = Probe::new(Arc::clone(&world.clock));
    let clock: Arc<dyn Clock> = probe.clone();
    let server = world.start(|config| {
        config.clock = clock;
        config.max_wait = max_wait;
    });
    world.open(server.local_addr(), OTHER);
    (server, probe)
}

/// Note `n` from `from` to OTHER that expires at `expires`.
fn send_note(world: &World, address: SocketAddr, from: &str, n: u32, expires: &str) {
    let mut post = world.post(from, OTHER, n, "a note");
    post.expires = Timestamp(expires.to_owned());
    world.sign(&mut post);
    let reply = world.send(address, from, "POST", POST, &encoded(&post));
    assert_eq!(reply.status, 202, "note {n}: {}", reply.body);
}

/// `principal`'s wait on OTHER's `Pending` for `wait` seconds, on a thread; the answer and the
/// instant it arrived come on the channel.
fn wait_on(
    world: &Arc<World>,
    address: SocketAddr,
    principal: &'static str,
    wait: &str,
) -> mpsc::Receiver<(support::Reply, Monotonic)> {
    let (sent, answered) = mpsc::channel();
    let world = Arc::clone(world);
    let target = format!("/messaging/views/pending?inbox={OTHER}&wait={wait}");
    thread::spawn(move || {
        let reply = world.send(address, principal, "GET", &target, b"");
        let _ = sent.send((reply, Monotonic::now()));
    });
    answered
}

/// Blocks until the wait whose first reading is number `from` has taken `count` readings, 5 s at
/// most.
fn until_the_wait_read(probe: &Probe, from: usize, count: usize) {
    let started = Monotonic::now();
    while probe.reads_of_first_thread_since(from) < count {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "the wait took {} of {count} readings within 5 s",
            probe.reads_of_first_thread_since(from)
        );
        thread::sleep(Duration::from_millis(2));
    }
}

/// The answer a wait ends with at its bound is the view of the last read before it slept. A sweep
/// while it sleeps sends no command signal (`sweep` in `src/expiry.rs` does not call
/// `command_answered`), so a wait that slept on due rows answers rows the sweep removed, which
/// `pending` no longer lists: the design document says the wait is "answered as without `wait`:
/// every row".
#[test]
fn a_wait_that_slept_on_due_rows_answers_what_pending_lists_after_the_sweep() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world, BOUND);
    let address = server.local_addr();
    send_note(&world, address, AGENT, 1, LATER);
    world.clock.set(instant(AFTER));
    let from = probe.reads();
    let answered = wait_on(&world, address, OTHER, "1");
    // The wait has read its view, found only the due note and read the clock after it.
    until_the_wait_read(&probe, from, 2);
    assert_eq!(server.sweep_now(), 1, "the sweep expires the due note");
    let (reply, _) = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait ends at its bound");
    let pending = world.send(
        address,
        OTHER,
        "GET",
        &format!("/messaging/views/pending?inbox={OTHER}"),
        b"",
    );
    assert_eq!(pending.ids(), Vec::<String>::new(), "{}", pending.body);
    assert_eq!(
        reply.ids(),
        pending.ids(),
        "the wait answered a row the sweep removed before the wait ended: {}",
        reply.body
    );
}

/// The lost wake-up: a live note posted after the wait read its view and before it sleeps. The
/// post runs on the wait's own thread inside its reading after the view (reading 2 of the wait),
/// so it lands in that window every round; the wait must see the command and end with the note.
/// 5 rounds in one service: each round's live note is due by the next round's clock.
#[test]
fn a_live_post_between_the_view_read_and_the_sleep_ends_the_wait_every_round() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world, Duration::from_secs(10));
    let address = server.local_addr();
    send_note(&world, address, AGENT, 1, LATER);
    let mut clock = instant(AFTER);
    world.clock.set(clock);
    for round in 0..5u32 {
        let n = round + 2;
        let expires = format_reading(clock.plus_seconds(1));
        let from = probe.reads();
        let poster = Arc::clone(&world);
        probe.on_reading(
            from + 1,
            Box::new(move || send_note(&poster, address, THIRD, n, &expires)),
        );
        let started = Monotonic::now();
        let (reply, at) = wait_on(&world, address, OTHER, "10")
            .recv_timeout(Duration::from_secs(5))
            .unwrap_or_else(|_| panic!("round {round}: the post in the window ended no wait"));
        assert_eq!(reply.status, 200, "round {round}: {}", reply.body);
        assert!(
            reply.ids().contains(&ulid(n)),
            "round {round}: the live note is listed: {}",
            reply.body
        );
        assert!(
            at - started < Duration::from_secs(5),
            "round {round}: {:?}",
            at - started
        );
        // The wait read its view, took the reading the post ran in, and read again once.
        assert_eq!(
            probe.reads_of_first_thread_since(from),
            3,
            "round {round}: the post did not land between the view read and the sleep"
        );
        clock = clock.plus_seconds(2);
        world.clock.set(clock);
    }
}

/// A wait of the request's own second with a note due exactly at the clock's reading (`expires`
/// equal to it): it holds the second, not `max_wait` (5 s), lists the note, and reads its view
/// once: 2 readings on its thread, the request's instant and the one after the view. More would
/// be a loop that reads the view again without a command.
#[test]
fn one_wait_of_a_second_on_a_note_due_at_the_reading_reads_its_view_once() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world, Duration::from_secs(5));
    let address = server.local_addr();
    send_note(&world, address, AGENT, 1, LATER);
    world.clock.set(instant(LATER));
    let from = probe.reads();
    let started = Monotonic::now();
    let (reply, at) = wait_on(&world, address, OTHER, "1")
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait ends at its own second");
    let held = at - started;
    assert_eq!(reply.ids(), [ulid(1)], "{}", reply.body);
    assert!(held >= Duration::from_secs(1), "held {held:?}");
    assert!(held < Duration::from_secs(3), "held {held:?}");
    assert_eq!(probe.reads_of_first_thread_since(from), 2);
}

/// The sender's wait on the recipient's `Pending` with only its own due note: a live note of
/// another sender to the same inbox wakes the wait, which the read rule then shows nothing new,
/// so it neither ends early nor lists the other sender's note.
#[test]
fn a_live_note_of_another_sender_neither_ends_nor_shows_in_a_senders_wait() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world, BOUND);
    let address = server.local_addr();
    send_note(&world, address, AGENT, 1, LATER);
    world.clock.set(instant(AFTER));
    let from = probe.reads();
    let started = Monotonic::now();
    let answered = wait_on(&world, address, AGENT, "1");
    until_the_wait_read(&probe, from, 2);
    send_note(&world, address, THIRD, 2, NEXT_DAY);
    let (reply, at) = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait ends at its bound");
    assert_eq!(reply.ids(), [ulid(1)], "{}", reply.body);
    assert!(at - started >= BOUND, "ended after {:?}", at - started);
}

/// The stop ends a wait that sleeps on due rows, as it ends one with none.
#[test]
fn stopping_the_service_ends_a_wait_that_holds_only_a_due_note() {
    let world = Arc::new(World::new());
    let (server, probe) = start(&world, Duration::from_secs(30));
    let address = server.local_addr();
    send_note(&world, address, AGENT, 1, LATER);
    world.clock.set(instant(AFTER));
    let from = probe.reads();
    let answered = wait_on(&world, address, OTHER, "30");
    until_the_wait_read(&probe, from, 2);
    let stopped = Monotonic::now();
    server.stop().expect("the service stops");
    let (reply, at) = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the stop ends the wait");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(at - stopped < Duration::from_secs(5));
}
