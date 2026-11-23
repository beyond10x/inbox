//! The cases of story:the-service-speaks-tls-with-a-pinned-certificate against the library: a
//! request over TLS with the pin, a client given another pin that sends no byte of its request,
//! a plain request to the TLS listener that is not served, and each bound of
//! `docs/design/service.md` the ending of TLS touches (the request timeout, the stop, the body
//! limits, the connection limit). Every certificate is made in process at run time
//! (`Identity::generate`, well under a millisecond); no certificate, key or pin is written into
//! this file.

use crate::support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant as Monotonic};

use inbox_core::MemoryBackend;
use inbox_server::tls::{self, ClientStream, Identity};
use inbox_server::{Config, Running, request_headers};
use rustls::{ClientConfig, ServerConnection, StreamOwned};
use support::{AGENT, ANSWER_WAIT, OTHER, Reply, World, encoded, ulid};

const POST: &str = "/messaging/commands/post";

fn pending(principal: &str) -> String {
    format!("/messaging/views/pending?inbox={principal}")
}

/// A service of `world` over TLS with a fresh certificate, and a client configuration that pins
/// it.
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

/// A TLS connection to `address` whose handshake with `config` is complete.
fn connect(address: SocketAddr, config: &Arc<ClientConfig>) -> ClientStream {
    let stream = TcpStream::connect_timeout(&address, ANSWER_WAIT).expect("the service accepts");
    stream
        .set_read_timeout(Some(ANSWER_WAIT))
        .expect("a read timeout");
    stream
        .set_write_timeout(Some(ANSWER_WAIT))
        .expect("a write timeout");
    tls::connect(config, stream).expect("the handshake completes")
}

/// Reads the whole answer from a TLS connection.
fn read_reply(mut stream: ClientStream) -> Reply {
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .unwrap_or_else(|error| panic!("the service sent no whole answer over TLS: {error}"));
    let answer = String::from_utf8(answer).expect("the answer is UTF-8");
    let (head, body) = answer.split_once("\r\n\r\n").expect("a head and a body");
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

/// Sends a request signed by `principal` over TLS and reads the answer.
fn send(
    world: &World,
    address: SocketAddr,
    config: &Arc<ClientConfig>,
    principal: &str,
    method: &str,
    target: &str,
    body: &[u8],
) -> Reply {
    let member = world.member(principal);
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
    let mut stream = connect(address, config);
    stream.write_all(head.as_bytes()).expect("the head is sent");
    stream.write_all(body).expect("the body is sent");
    stream.flush().expect("flushed");
    read_reply(stream)
}

fn open(world: &World, address: SocketAddr, config: &Arc<ClientConfig>, principal: &str) {
    let body = format!(r#"{{"address":"{principal}","adapter":"terminal"}}"#);
    let reply = send(
        world,
        address,
        config,
        principal,
        "POST",
        "/messaging/commands/open-inbox",
        body.as_bytes(),
    );
    assert_eq!(reply.status, 202, "{}", reply.body);
}

#[test]
fn a_post_over_tls_with_the_pin_is_posted_and_read_back() {
    let world = World::new();
    let (server, config) = start_tls(&world, |_| {});
    let address = server.local_addr();
    open(&world, address, &config, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "over TLS");
    let reply = send(
        &world,
        address,
        &config,
        AGENT,
        "POST",
        POST,
        &encoded(&envelope),
    );
    assert_eq!(reply.status, 202, "{}", reply.body);
    assert_eq!(reply.text("outcome"), "posted");
    let pending = send(&world, address, &config, OTHER, "GET", &pending(OTHER), b"");
    assert_eq!(pending.ids(), vec![ulid(1)]);
}

#[test]
fn a_client_given_another_pin_ends_the_handshake_and_posts_nothing() {
    let world = World::new();
    let (server, config) = start_tls(&world, |_| {});
    let address = server.local_addr();
    open(&world, address, &config, OTHER);
    let other = Identity::generate().expect("a certificate is made");
    let wrong = tls::client_config(other.pin().clone()).expect("a client configuration");
    let stream = TcpStream::connect(address).expect("the service accepts");
    let refused = tls::connect(&wrong, stream).expect_err("another pin is refused");
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidData, "{refused}");
    assert!(refused.to_string().contains("pin"), "{refused}");
    let pending = send(&world, address, &config, OTHER, "GET", &pending(OTHER), b"");
    assert!(pending.ids().is_empty(), "{}", pending.body);
}

/// The next connection on `listener` within `wait`.
fn accept_within(listener: &TcpListener, wait: Duration) -> TcpStream {
    listener.set_nonblocking(true).expect("non-blocking");
    let deadline = Monotonic::now() + wait;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).expect("blocking");
                return stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Monotonic::now() < deadline, "no connection within {wait:?}");
                thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("the stand-in's listener failed: {error}"),
        }
    }
}

/// A TLS stand-in with the certificate of `identity` on 127.0.0.1:0: it completes the handshake
/// where the client goes on, and hands back every plain byte the client sent over one
/// connection.
fn recording_stand_in(identity: &Identity) -> (SocketAddr, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let address = listener.local_addr().expect("an address");
    let config = identity.server_config().expect("a server configuration");
    let handle = thread::spawn(move || {
        let stream = accept_within(&listener, ANSWER_WAIT);
        stream
            .set_read_timeout(Some(ANSWER_WAIT))
            .expect("a read timeout");
        let connection = ServerConnection::new(config).expect("a TLS connection");
        let mut stream = StreamOwned::new(connection, stream);
        let mut plain = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => plain.extend_from_slice(&buffer[..read]),
            }
        }
        plain
    });
    (address, handle)
}

#[test]
fn a_client_given_another_pin_sends_no_byte_of_its_request() {
    let stand_in = Identity::generate().expect("a certificate is made");
    let pinned = Identity::generate().expect("a certificate is made");

    // The stand-in records: a client that pins its certificate gets its bytes through.
    let (address, recorded) = recording_stand_in(&stand_in);
    let right = tls::client_config(stand_in.pin().clone()).expect("a client configuration");
    let mut stream = tls::connect(&right, TcpStream::connect(address).expect("accepted"))
        .expect("the handshake completes");
    stream.write_all(b"GET / HTTP/1.1\r\n\r\n").expect("sent");
    stream.flush().expect("flushed");
    drop(stream);
    assert_eq!(
        recorded.join().expect("the stand-in ends"),
        b"GET / HTTP/1.1\r\n\r\n"
    );

    // A client given another pin ends the handshake: the stand-in gets no plain byte.
    let (address, recorded) = recording_stand_in(&stand_in);
    let wrong = tls::client_config(pinned.pin().clone()).expect("a client configuration");
    let refused = tls::connect(&wrong, TcpStream::connect(address).expect("accepted"))
        .expect_err("a certificate the pin does not name is refused");
    assert!(refused.to_string().contains("pin"), "{refused}");
    let plain = recorded.join().expect("the stand-in ends");
    assert!(plain.is_empty(), "the stand-in received {plain:?}");
}

#[test]
fn a_plain_request_to_the_tls_listener_is_not_served() {
    let world = World::new();
    let (server, config) = start_tls(&world, |_| {});
    let address = server.local_addr();
    open(&world, address, &config, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "in the clear");
    let body = encoded(&envelope);
    let member = world.member(AGENT);
    let headers = request_headers(
        "POST",
        POST,
        &body,
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut head = format!(
        "POST {POST} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let mut stream = TcpStream::connect(address).expect("the service accepts");
    stream
        .set_read_timeout(Some(ANSWER_WAIT))
        .expect("a read timeout");
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&body);
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    assert!(
        !answer.starts_with(b"HTTP/"),
        "answered in the clear: {:?}",
        String::from_utf8_lossy(&answer)
    );
    let pending = send(&world, address, &config, OTHER, "GET", &pending(OTHER), b"");
    assert!(pending.ids().is_empty(), "{}", pending.body);
}

/// Completes the handshake and sends the start of a request head, then nothing more.
fn half_a_request(address: SocketAddr, config: &Arc<ClientConfig>) -> ClientStream {
    let mut stream = connect(address, config);
    stream
        .write_all(b"GET /messaging/views/inboxes HTTP/1.1\r\nHost: ")
        .expect("the start of the head is sent");
    stream.flush().expect("flushed");
    stream
}

#[test]
fn a_tls_request_not_arrived_within_the_request_timeout_is_408() {
    let world = World::new();
    // Shorter than the reader's own 1 s gap bound, so the answer is this service's.
    let (server, config) = start_tls(&world, |config| {
        config.request_timeout = Duration::from_millis(300);
    });
    // The 408 is the cut of the request timeout: no clock is read here, so a slow host passes.
    let reply = read_reply(half_a_request(server.local_addr(), &config));
    assert_eq!(reply.status, 408, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"request timeout"}"#);
}

#[test]
fn stop_answers_a_tls_connection_still_sending_its_request_with_503() {
    let world = World::new();
    let (server, config) = start_tls(&world, |config| {
        config.request_timeout = Duration::from_secs(3600);
    });
    let stream = half_a_request(server.local_addr(), &config);
    thread::sleep(Duration::from_millis(100));
    // A stop that waited for the hour of the request timeout would be ended by the runner's
    // bound; the 503 shows the stop cut the request.
    server.stop().expect("the service stops");
    let reply = read_reply(stream);
    assert_eq!(reply.status, 503, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"stopping"}"#);
}

#[test]
fn a_body_over_the_limits_over_tls_is_413() {
    let world = World::new();
    let (server, config) = start_tls(&world, |config| config.max_body = 64);
    let address = server.local_addr();
    let envelope = world.post(
        AGENT,
        OTHER,
        1,
        "a body that takes the post past sixty-four bytes",
    );
    let reply = send(
        &world,
        address,
        &config,
        AGENT,
        "POST",
        POST,
        &encoded(&envelope),
    );
    assert_eq!(reply.status, 413, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"body too large","limit":64}"#);

    // Past the generated reader's own bound, the reader refuses before the body is read.
    let mut stream = connect(address, &config);
    stream
        .write_all(format!("POST {POST} HTTP/1.1\r\nContent-Length: 2000000\r\n\r\n").as_bytes())
        .expect("the head is sent");
    stream.flush().expect("flushed");
    let reply = read_reply(stream);
    assert_eq!(reply.status, 413, "{}", reply.body);
}

#[test]
fn a_tls_connection_beyond_the_limit_waits_until_one_ends() {
    let world = Arc::new(World::new());
    let (server, config) = start_tls(&world, |config| {
        config.max_connections = 1;
        config.request_timeout = Duration::from_secs(3600);
    });
    let address = server.local_addr();
    // The first holds the one slot for as long as this case keeps it open: no bound of the
    // service ends it, so a slow host cannot free the slot early.
    let first = half_a_request(address, &config);
    let (sent, second) = mpsc::channel();
    {
        let (world, config) = (Arc::clone(&world), Arc::clone(&config));
        thread::spawn(move || {
            let reply = send(
                &world,
                address,
                &config,
                AGENT,
                "GET",
                "/messaging/views/inboxes",
                b"",
            );
            let _ = sent.send(reply);
        });
    }
    thread::sleep(Duration::from_millis(300));
    assert!(
        second.try_recv().is_err(),
        "the second connection was served beside the first"
    );
    drop(first);
    let reply = second
        .recv_timeout(ANSWER_WAIT)
        .expect("the second request is served once the first ended");
    assert_eq!(reply.status, 200, "{}", reply.body);
}
