//! Adversary cases, pass 1, for story:the-service-speaks-tls-with-a-pinned-certificate: the bounds
//! of `docs/design/service.md` over TLS, and the threads and descriptors of the ending of TLS.
//! Every certificate is made at run time; no certificate, key or pin is written into this file.

use crate::support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant as Monotonic};

use inbox_core::MemoryBackend;
use inbox_server::tls::{self, ClientStream, Identity};
use inbox_server::{Config, Running, request_headers};
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection};
use support::{AGENT, ANSWER_WAIT, OTHER, Reply, World, encoded};

const POST: &str = "/messaging/commands/post";
const INBOXES: &str = "/messaging/views/inboxes";

fn start_tls(
    world: &World,
    tweak: impl FnOnce(&mut Config),
) -> (Running<MemoryBackend>, Arc<ClientConfig>) {
    let identity = Identity::generate().expect("a certificate is made");
    let client = tls::client_config(identity.pin().clone()).expect("a client configuration");
    let server = world.start(|config| {
        config.tls = Some(Arc::new(identity));
        tweak(config);
    });
    (server, client)
}

fn tcp(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect_timeout(&address, ANSWER_WAIT).expect("the service accepts");
    stream
        .set_read_timeout(Some(ANSWER_WAIT))
        .expect("a read timeout");
    stream
        .set_write_timeout(Some(ANSWER_WAIT))
        .expect("a write timeout");
    stream
}

fn connect(address: SocketAddr, config: &Arc<ClientConfig>) -> ClientStream {
    tls::connect(config, tcp(address)).expect("the handshake completes")
}

fn parse(answer: &[u8]) -> Reply {
    let answer = String::from_utf8_lossy(answer).into_owned();
    let (head, body) = answer
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("a head and a body in {answer:?}"));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or_else(|| panic!("a status line in {head}"));
    Reply {
        status,
        body: body.to_owned(),
    }
}

fn signed_head(
    world: &World,
    address: SocketAddr,
    method: &str,
    target: &str,
    body: &[u8],
) -> String {
    let member = world.member(AGENT);
    let headers = request_headers(
        method,
        target,
        body,
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut head = format!(
        "{method} {target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    head
}

fn tls_get(world: &World, address: SocketAddr, config: &Arc<ClientConfig>, target: &str) -> Reply {
    let mut stream = connect(address, config);
    stream
        .write_all(signed_head(world, address, "GET", target, b"").as_bytes())
        .expect("the head is sent");
    stream.flush().expect("flushed");
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .unwrap_or_else(|error| panic!("no whole answer over TLS: {error}"));
    parse(&answer)
}

/// The first `cut` bytes of a real ClientHello of `config`.
fn part_of_a_client_hello(config: &Arc<ClientConfig>, cut: usize) -> Vec<u8> {
    let mut connection = ClientConnection::new(
        Arc::clone(config),
        ServerName::IpAddress(std::net::Ipv4Addr::LOCALHOST.into()),
    )
    .expect("a client connection");
    let mut hello = Vec::new();
    while connection.wants_write() {
        connection
            .write_tls(&mut hello)
            .expect("the hello is written");
    }
    hello.truncate(cut.min(hello.len().saturating_sub(1)));
    hello
}

/// The threads of this process whose name is one of the two that end TLS.
fn tls_threads() -> usize {
    std::fs::read_dir("/proc/self/task")
        .expect("/proc/self/task")
        .filter_map(Result::ok)
        .filter(|task| {
            std::fs::read_to_string(task.path().join("comm"))
                .is_ok_and(|name| name.trim().starts_with("inbox-tls-"))
        })
        .count()
}

/// The open descriptors of this process.
fn descriptors() -> usize {
    std::fs::read_dir("/proc/self/fd")
        .expect("/proc/self/fd")
        .count()
}

/// Polls until the TLS threads are gone and the descriptors are back at `baseline`, or `within`
/// has passed; answers both counts at the end.
fn settle(baseline: usize, within: Duration) -> (usize, usize) {
    let deadline = Monotonic::now() + within;
    loop {
        let counts = (tls_threads(), descriptors());
        if (counts.0 == 0 && counts.1 <= baseline) || Monotonic::now() >= deadline {
            return counts;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn adv_tls_threads_and_descriptors_end_with_every_kind_of_connection() {
    let world = World::new();
    let (server, config) = start_tls(&world, |config| {
        config.request_timeout = Duration::from_millis(400);
    });
    let address = server.local_addr();
    assert_eq!(tls_get(&world, address, &config, INBOXES).status, 200);
    let (threads, baseline) = settle(usize::MAX, Duration::from_secs(5));
    assert_eq!(threads, 0, "the threads of a served connection ended");
    let baseline = descriptors().min(baseline);

    let mut held = Vec::new();
    // TCP only, nothing sent.
    held.push(tcp(address));
    // Half a ClientHello, then nothing.
    let mut half = tcp(address);
    half.write_all(&part_of_a_client_hello(&config, 40))
        .expect("half a hello");
    held.push(half);
    // The handshake completed, nothing sent.
    let quiet = connect(address, &config);
    // A whole request whose answer is never read.
    let mut unread = connect(address, &config);
    unread
        .write_all(signed_head(&world, address, "GET", INBOXES, b"").as_bytes())
        .expect("sent");
    unread.flush().expect("flushed");
    // A whole request, closed at once.
    {
        let mut closed = connect(address, &config);
        closed
            .write_all(signed_head(&world, address, "GET", INBOXES, b"").as_bytes())
            .expect("sent");
        closed.flush().expect("flushed");
    }
    // Garbage instead of TLS.
    {
        let mut garbage = tcp(address);
        let _ = garbage.write_all(b"GET / HTTP/1.1\r\n\r\n");
    }
    let running = tls_threads();
    assert!(running >= 4, "the count sees the TLS threads: {running}");
    thread::sleep(Duration::from_millis(1500));
    // The request timeout has passed for each; the service is done with every one of them while
    // the callers still hold their end open.
    let (threads, open) = settle(baseline + held.len() + 2 + 1, Duration::from_secs(5));
    drop((quiet, unread, held));
    let (after_threads, after_open) = settle(baseline, Duration::from_secs(5));
    assert_eq!(
        (threads, after_threads),
        (0, 0),
        "TLS threads still running past the request timeout ({threads}) and after the callers \
         closed ({after_threads})"
    );
    assert!(
        open <= baseline + 4 && after_open <= baseline,
        "descriptors: {baseline} before, {open} past the request timeout, {after_open} after the \
         callers closed"
    );
}

#[test]
fn adv_tls_a_caller_stalled_in_the_handshake_gives_its_slot_back_at_the_request_timeout() {
    let world = Arc::new(World::new());
    let (server, config) = start_tls(&world, |config| {
        config.max_connections = 1;
        config.request_timeout = Duration::from_millis(600);
    });
    let address = server.local_addr();
    let mut stalled = tcp(address);
    stalled
        .write_all(&part_of_a_client_hello(&config, 40))
        .expect("half a hello");
    thread::sleep(Duration::from_millis(100));
    let (sent, served) = mpsc::channel();
    {
        let (world, config) = (Arc::clone(&world), Arc::clone(&config));
        thread::spawn(move || {
            let reply = tls_get(&world, address, &config, INBOXES);
            let _ = sent.send(reply);
        });
    }
    let reply = served
        .recv_timeout(Duration::from_secs(8))
        .expect("the second connection is served once the stalled one is cut off");
    assert_eq!(reply.status, 200, "{}", reply.body);
    // The stalled caller got no answer and its connection is closed.
    let mut rest = Vec::new();
    let _ = stalled.read_to_end(&mut rest);
    assert!(!rest.starts_with(b"HTTP/"), "answered in the clear");
}

#[test]
fn adv_tls_a_request_sent_a_byte_per_record_is_served() {
    let world = World::new();
    let (server, config) = start_tls(&world, |config| {
        config.request_timeout = Duration::from_secs(30);
    });
    let address = server.local_addr();
    let mut stream = connect(address, &config);
    for byte in signed_head(&world, address, "GET", INBOXES, b"").as_bytes() {
        stream.write_all(&[*byte]).expect("a byte");
        stream.flush().expect("flushed");
    }
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("the answer");
    assert_eq!(parse(&answer).status, 200);
}

#[test]
fn adv_tls_a_wait_over_tls_ends_on_a_post() {
    let world = Arc::new(World::new());
    let (server, config) = start_tls(&world, |config| {
        config.max_wait = Duration::from_secs(30);
    });
    let address = server.local_addr();
    world.open_tls(address, &config, OTHER);
    let target = format!("/messaging/views/pending?inbox={OTHER}&wait=20");
    let (sent, answered) = mpsc::channel();
    {
        let (world, config, target) = (Arc::clone(&world), Arc::clone(&config), target.clone());
        thread::spawn(move || {
            let member = world.member(OTHER);
            let headers = request_headers(
                "GET",
                &target,
                b"",
                &member.key_id,
                &member.key,
                &world.timestamp(),
            );
            let mut head =
                format!("GET {target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: 0\r\n");
            for (name, value) in headers {
                head.push_str(&format!("{name}: {value}\r\n"));
            }
            head.push_str("\r\n");
            let mut stream = connect(address, &config);
            stream.write_all(head.as_bytes()).expect("sent");
            stream.flush().expect("flushed");
            let mut answer = Vec::new();
            let started = Monotonic::now();
            let read = stream.read_to_end(&mut answer);
            let _ = sent.send((read.map(|_| parse(&answer)), started.elapsed()));
        });
    }
    thread::sleep(Duration::from_millis(300));
    let envelope = world.post(AGENT, OTHER, 1, "wakes the wait");
    let body = encoded(&envelope);
    let mut stream = connect(address, &config);
    stream
        .write_all(signed_head(&world, address, "POST", POST, &body).as_bytes())
        .expect("sent");
    stream.write_all(&body).expect("sent");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("the post's answer");
    assert_eq!(parse(&answer).status, 202);
    let (reply, took) = answered
        .recv_timeout(Duration::from_secs(10))
        .expect("the wait answers on the post");
    let reply = reply.expect("a whole answer");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(took < Duration::from_secs(10), "{took:?}");
    server.stop().expect("stops");
}

trait OpenTls {
    fn open_tls(&self, address: SocketAddr, config: &Arc<ClientConfig>, principal: &str);
}

impl OpenTls for World {
    fn open_tls(&self, address: SocketAddr, config: &Arc<ClientConfig>, principal: &str) {
        let body = format!(r#"{{"address":"{principal}","adapter":"terminal"}}"#);
        let target = "/messaging/commands/open-inbox";
        let member = self.member(principal);
        let headers = request_headers(
            "POST",
            target,
            body.as_bytes(),
            &member.key_id,
            &member.key,
            &self.timestamp(),
        );
        let mut head = format!(
            "POST {target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n",
            body.len()
        );
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        let mut stream = connect(address, config);
        stream.write_all(head.as_bytes()).expect("sent");
        stream.write_all(body.as_bytes()).expect("sent");
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).expect("an answer");
        assert_eq!(parse(&answer).status, 202);
    }
}
