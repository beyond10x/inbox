//! Adversary cases, pass 2, for story:http-service: the request timeout, the cut at stop and the
//! linger after a refusal. Every key is created when the test runs.

mod support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant as Monotonic};

use inbox_server::request_headers;
use support::{AGENT, Reply, World, read_reply};

const INBOXES: &str = "/messaging/views/inboxes";

/// Sends `first`, then one byte every `every` until told to stop or the socket refuses; reads
/// nothing.
fn drip(
    address: SocketAddr,
    first: &'static [u8],
    every: Duration,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let Ok(mut stream) = TcpStream::connect(address) else {
            return;
        };
        let _ = stream.write_all(first);
        while !stop.load(Ordering::SeqCst) {
            if stream.write_all(b"a").is_err() {
                break;
            }
            thread::sleep(every);
        }
    })
}

/// A signed GET of `target` sent on another thread; the reply and how long it took arrive on the
/// channel.
fn signed_get_later(
    world: &Arc<World>,
    address: SocketAddr,
    target: &'static str,
) -> mpsc::Receiver<(Reply, Duration)> {
    let (sent, answered) = mpsc::channel();
    let world = Arc::clone(world);
    thread::spawn(move || {
        let asked = Monotonic::now();
        let reply = world.send(address, AGENT, "GET", target, b"");
        let _ = sent.send((reply, Monotonic::now() - asked));
    });
    answered
}

/// The bytes of a signed GET of `target` as a client writes them.
fn signed_get_bytes(world: &World, address: SocketAddr, target: &str) -> Vec<u8> {
    let member = world.member(AGENT);
    let headers = request_headers(
        "GET",
        target,
        b"",
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut head = format!("GET {target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: 0\r\n");
    for (name, value) in &headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    head.into_bytes()
}

// ---- the linger after a refusal ---------------------------------------------------------------

/// A request the reader refuses at once (no `METHOD TARGET HTTP/1.1`), after which the caller
/// keeps sending a byte every 300 ms: the linger reads each byte, so only the watch ends it.
const REFUSED_AT_ONCE: &[u8] = b"BAD\r\n";

#[test]
fn adv2_stop_returns_while_a_refused_connection_keeps_sending() {
    let world = World::new();
    let server = world.start(|config| config.request_timeout = Duration::from_secs(3600));
    let address = server.local_addr();
    let stop = Arc::new(AtomicBool::new(false));
    let refused = drip(
        address,
        REFUSED_AT_ONCE,
        Duration::from_millis(300),
        Arc::clone(&stop),
    );
    thread::sleep(Duration::from_millis(500));

    let (sent, stopped) = mpsc::channel();
    thread::spawn(move || {
        let _ = sent.send(server.stop());
    });
    let returned = stopped.recv_timeout(Duration::from_secs(3));
    stop.store(true, Ordering::SeqCst);
    let _ = refused.join();
    returned
        .expect(
            "Running::stop returns within 3 s while a connection whose request was refused keeps \
             sending a byte every 300 ms",
        )
        .expect("the service stops");
}

#[test]
fn adv2_a_refused_connection_that_keeps_sending_gives_its_slot_back_at_the_request_timeout() {
    let world = Arc::new(World::new());
    let server = world.start(|config| {
        config.max_connections = 1;
        config.request_timeout = Duration::from_millis(1500);
    });
    let address = server.local_addr();
    let stop = Arc::new(AtomicBool::new(false));
    let refused = drip(
        address,
        REFUSED_AT_ONCE,
        Duration::from_millis(300),
        Arc::clone(&stop),
    );
    thread::sleep(Duration::from_millis(300));

    let answered = signed_get_later(&world, address, INBOXES);
    let served = answered.recv_timeout(Duration::from_secs(5));
    stop.store(true, Ordering::SeqCst);
    let _ = refused.join();
    let (reply, took) = served.expect(
        "a signed request is served within 5 s while the one slot is held by a refused connection \
         that keeps sending a byte every 300 ms (request_timeout 1.5 s)",
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(took < Duration::from_secs(4), "served after {took:?}");
}

// ---- the request timeout ----------------------------------------------------------------------

#[test]
fn adv2_the_first_connection_after_an_idle_time_is_timed_out() {
    let world = World::new();
    let server = world.start(|config| config.request_timeout = Duration::from_millis(400));
    let address = server.local_addr();
    // The reaper has long found nothing to watch and waits without a deadline.
    thread::sleep(Duration::from_millis(1500));
    let started = Monotonic::now();
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    stream
        .write_all(b"GET /messaging/views/inboxes HTTP/1.1\r\nHost: ")
        .expect("the start of the head is sent");
    let reply = read_reply(stream);
    assert_eq!(reply.status, 408, "{}", reply.body);
    assert!(Monotonic::now() - started < Duration::from_millis(950));
}

#[test]
fn adv2_the_408_comes_at_the_request_timeout_and_not_before() {
    let world = World::new();
    // 700 ms: a bound read as twice as long meets the reader's own 1 s gap first (400).
    let server = world.start(|config| config.request_timeout = Duration::from_millis(700));
    let address = server.local_addr();
    let started = Monotonic::now();
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    stream
        .write_all(b"GET /messaging/views/inboxes HTTP/1.1\r\nHost: ")
        .expect("the start of the head is sent");
    let reply = read_reply(stream);
    let took = Monotonic::now() - started;
    assert_eq!(reply.status, 408, "after {took:?}: {}", reply.body);
    assert!(took >= Duration::from_millis(650), "cut after {took:?}");
}

#[test]
fn adv2_a_request_cut_while_its_body_is_read_is_one_408() {
    let world = World::new();
    let server = world.start(|config| config.request_timeout = Duration::from_millis(1200));
    let address = server.local_addr();
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("a read timeout");
    stream
        .write_all(
            b"POST /messaging/commands/post HTTP/1.1\r\nHost: x\r\nContent-Length: 100\r\n\r\n{",
        )
        .expect("the head and the start of the body are sent");
    // One body byte every 400 ms: inside the reader's 1 s gap bound, past the 1.2 s bound.
    let mut writer = stream.try_clone().expect("a second handle");
    let stop = Arc::new(AtomicBool::new(false));
    let dripping = {
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            for _ in 0..8 {
                if stop.load(Ordering::SeqCst) || writer.write_all(b" ").is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(400));
            }
        })
    };
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    stop.store(true, Ordering::SeqCst);
    let _ = dripping.join();
    let answer = String::from_utf8_lossy(&answer).into_owned();
    assert_eq!(
        answer.matches("HTTP/1.1 ").count(),
        1,
        "one answer, not two: {answer}"
    );
    assert!(
        answer.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "{answer}"
    );
    assert!(
        answer.ends_with("\r\n\r\n{\"refused\":\"request timeout\"}"),
        "{answer}"
    );
}

#[test]
fn adv2_a_signed_request_that_arrives_slowly_inside_the_request_timeout_is_served() {
    let world = World::new();
    let server = world.start(|config| config.request_timeout = Duration::from_secs(3));
    let address = server.local_addr();
    let bytes = signed_get_bytes(&world, address, INBOXES);
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    // The request in five pieces, 400 ms apart: 1.6 s in all, inside the 3 s bound.
    let piece = bytes.len().div_ceil(5);
    for chunk in bytes.chunks(piece) {
        stream.write_all(chunk).expect("a piece is sent");
        thread::sleep(Duration::from_millis(400));
    }
    let reply = read_reply(stream);
    assert_eq!(reply.status, 200, "{}", reply.body);
}

#[test]
fn adv2_slow_clients_that_reconnect_hold_a_signed_request_at_most_about_one_request_timeout() {
    let world = Arc::new(World::new());
    let server = world.start(|config| {
        config.max_connections = 2;
        config.request_timeout = Duration::from_millis(1500);
    });
    let address = server.local_addr();
    let stop = Arc::new(AtomicBool::new(false));
    // Two clients that each fill one slot with a head sent a byte every 900 ms, and connect again
    // as soon as they are cut off.
    let slow: Vec<JoinHandle<()>> = (0..2)
        .map(|_| {
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let one = drip(
                        address,
                        b"GET /messaging/views/inboxes HTTP/1.1\r\nX-Slow: ",
                        Duration::from_millis(900),
                        Arc::clone(&stop),
                    );
                    let _ = one.join();
                }
            })
        })
        .collect();
    thread::sleep(Duration::from_millis(400));
    let answered = signed_get_later(&world, address, INBOXES);
    let served = answered.recv_timeout(Duration::from_secs(6));
    stop.store(true, Ordering::SeqCst);
    let _ = server.stop();
    for one in slow {
        let _ = one.join();
    }
    let (reply, took) = served.expect("a signed request is served within 6 s");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(took < Duration::from_secs(3), "served after {took:?}");
}
