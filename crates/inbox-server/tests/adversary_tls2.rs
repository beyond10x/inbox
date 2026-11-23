//! Adversary cases, pass 2, for story:the-service-speaks-tls-with-a-pinned-certificate: the order
//! in which a TLS connection ends, held to `docs/design/service.md` (TLS). Every certificate is
//! made at run time; no certificate, key or pin is written into this file.

use crate::support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use inbox_core::MemoryBackend;
use inbox_server::tls::{self, ClientStream, Identity};
use inbox_server::{Config, Running, request_headers};
use rustls::ClientConfig;
use support::{AGENT, World};

/// The wait that only ends a hang.
const HANG: Duration = Duration::from_secs(20);

/// The length a request head announces: under the reader's bound, so the reader reads the body.
const ANNOUNCED: usize = 900_000;

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

fn connect(address: SocketAddr, config: &Arc<ClientConfig>) -> ClientStream {
    let stream = TcpStream::connect_timeout(&address, HANG).expect("the service accepts");
    stream.set_read_timeout(Some(HANG)).expect("a read timeout");
    stream
        .set_write_timeout(Some(HANG))
        .expect("a write timeout");
    tls::connect(config, stream).expect("the handshake completes")
}

/// How one exchange ended for its caller.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// A write of the request failed: the service cut the connection.
    WriteFailed,
    /// The request went out whole and the answer could not be read to its end.
    ReadFailed,
    /// The whole answer, with this status.
    Status(u16),
    /// The stream ended cleanly on an answer that is empty, has no status or is shorter than its
    /// `Content-Length`.
    ShortOrEmpty,
    /// A write or the read waited out its [`HANG`] bound.
    Hang,
}

fn waited(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// A plain connection with the same bounds as [`connect`].
fn connect_plain(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect_timeout(&address, HANG).expect("the service accepts");
    stream.set_read_timeout(Some(HANG)).expect("a read timeout");
    stream
        .set_write_timeout(Some(HANG))
        .expect("a write timeout");
    stream
}

/// Reads the answer on `stream` to its end and says how it ended.
fn outcome_of_the_answer(mut stream: impl Read) -> Outcome {
    let mut answer = Vec::new();
    match stream.read_to_end(&mut answer) {
        Err(error) if waited(&error) => return Outcome::Hang,
        Err(_) => return Outcome::ReadFailed,
        Ok(_) => {}
    }
    let answer = String::from_utf8_lossy(&answer).into_owned();
    let Some((head, body)) = answer.split_once("\r\n\r\n") else {
        return Outcome::ShortOrEmpty;
    };
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok());
    let declared: Option<usize> = head
        .split("\r\n")
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|value| value.trim().parse().ok());
    match (status, declared) {
        (Some(status), Some(declared)) if declared == body.len() => Outcome::Status(status),
        _ => Outcome::ShortOrEmpty,
    }
}

/// A signed `GET /openapi.json` on `stream`, read to its end.
fn outcome_of_a_get(world: &World, mut stream: impl Read + Write) -> Outcome {
    let member = world.member(AGENT);
    let headers = request_headers(
        "GET",
        "/openapi.json",
        b"",
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut head =
        String::from("GET /openapi.json HTTP/1.1\r\nHost: inbox\r\nContent-Length: 0\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    match stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.flush())
    {
        Err(error) if waited(&error) => Outcome::Hang,
        Err(_) => Outcome::WriteFailed,
        Ok(()) => outcome_of_the_answer(stream),
    }
}

/// Sends a signed head announcing [`ANNOUNCED`] bytes, then the body in pieces of 8 KiB with a
/// pause after each, as a slow caller does, then reads the answer, and says how it ended.
fn outcome_of_a_slow_post(world: &World, mut stream: impl Read + Write) -> Outcome {
    let member = world.member(AGENT);
    let target = "/messaging/commands/post";
    let headers = request_headers(
        "POST",
        target,
        b"",
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut head =
        format!("POST {target} HTTP/1.1\r\nHost: inbox\r\nContent-Length: {ANNOUNCED}\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    if let Err(error) = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.flush())
    {
        return if waited(&error) {
            Outcome::Hang
        } else {
            Outcome::WriteFailed
        };
    }
    let piece = [b'x'; 8192];
    let mut sent = 0;
    while sent < ANNOUNCED {
        let take = piece.len().min(ANNOUNCED - sent);
        if let Err(error) = stream
            .write_all(&piece[..take])
            .and_then(|()| stream.flush())
        {
            return if waited(&error) {
                Outcome::Hang
            } else {
                Outcome::WriteFailed
            };
        }
        sent += take;
        thread::sleep(Duration::from_millis(10));
    }
    outcome_of_the_answer(stream)
}

/// How a caller learns of a cut answered `status`: `Some(false)` when a write or the read failed
/// (the connection was reset under it), `Some(true)` when it read the whole answer of the cut,
/// `None` for every other ending (a hang, a clean end on a short or empty answer, another status).
fn told_of_the_cut(outcome: &Outcome, status: u16) -> Option<bool> {
    match outcome {
        Outcome::WriteFailed | Outcome::ReadFailed => Some(false),
        Outcome::Status(answered) if *answered == status => Some(true),
        _ => None,
    }
}

/// Holds that a cut answered `status` ended over plain HTTP and over TLS in one of the ways of
/// [`told_of_the_cut`], and the same way over both.
fn assert_cut_alike(over_plain: &Outcome, over_tls: &Outcome, status: u16) {
    let plain = told_of_the_cut(over_plain, status);
    let tls = told_of_the_cut(over_tls, status);
    assert!(
        plain.is_some() && plain == tls,
        "a cut ({status}) ended over plain HTTP with {over_plain:?}, over TLS with {over_tls:?}"
    );
}

/// Round 3 rewrote this case: it asserted that the caller reads the `408` of a cut while its
/// body still arrives, which holds over neither plain HTTP nor TLS (the cut shuts the socket for
/// reading, and the caller's next bytes are reset). It now holds what is true: the exchange ends
/// over TLS as over plain HTTP (a failed write or read, or the whole `408`; never a hang, never a
/// clean end on a short or empty answer), and the one slot of `max_connections` is free again
/// after it.
#[test]
fn adv2_tls_a_408_cut_while_the_body_still_arrives_ends_as_over_plain_and_frees_the_slot() {
    let world = World::new();
    // The body takes more than a second to arrive at its slowest; the bound cuts it on the way.
    let tweak = |config: &mut Config| {
        config.max_connections = 1;
        config.request_timeout = Duration::from_millis(400);
    };
    let plain = world.start(tweak);
    let (over_tls, client) = start_tls(&world, tweak);

    let over_plain = outcome_of_a_slow_post(&world, connect_plain(plain.local_addr()));
    let tls = outcome_of_a_slow_post(&world, connect(over_tls.local_addr(), &client));
    assert_cut_alike(&over_plain, &tls, 408);

    // The one slot is free again: the next request is served whole.
    let next = outcome_of_a_get(&world, connect_plain(plain.local_addr()));
    assert_eq!(next, Outcome::Status(200), "over plain HTTP");
    let next = outcome_of_a_get(&world, connect(over_tls.local_addr(), &client));
    assert_eq!(next, Outcome::Status(200), "over TLS");
}

/// Round 3 rewrote this case for the reason the 408 case names: a body still arriving when the
/// service stops is cut (`503`), the exchange ends over TLS as over plain HTTP, and the stop
/// returns, which it does only once every slot of `max_connections` is free.
#[test]
fn adv2_tls_a_503_cut_while_the_body_still_arrives_ends_as_over_plain_and_the_stop_returns() {
    let world = World::new();
    let tweak = |config: &mut Config| {
        config.request_timeout = Duration::from_secs(3600);
    };
    let cut_by_a_stop = |server: Running<MemoryBackend>, stream: Box<dyn ReadWrite>| {
        let stopper = thread::spawn(move || {
            thread::sleep(Duration::from_millis(400));
            server.stop().expect("the service stops");
        });
        let outcome = outcome_of_a_slow_post(&world, stream);
        stopper.join().expect("the stop returns");
        outcome
    };
    let plain = world.start(tweak);
    let stream = connect_plain(plain.local_addr());
    let over_plain = cut_by_a_stop(plain, Box::new(stream));
    let (over_tls, client) = start_tls(&world, tweak);
    let stream = connect(over_tls.local_addr(), &client);
    let tls = cut_by_a_stop(over_tls, Box::new(stream));
    assert_cut_alike(&over_plain, &tls, 503);
}

/// A stream of either kind.
trait ReadWrite: Read + Write + Send {}

impl<T: Read + Write + Send> ReadWrite for T {}

/// What one exchange of [`an_answer_with_bytes_after_its_request`] ended with.
#[derive(Debug, PartialEq, Eq)]
enum Ending {
    /// The stream ended with `close_notify` and the body is as long as `Content-Length`.
    Whole,
    /// The stream ended with `close_notify`, and the body is shorter than `Content-Length`.
    CleanButShort { declared: usize, got: usize },
    /// The stream did not end cleanly (no `close_notify`): the caller knows it was cut.
    Failed,
}

/// A whole request for `/openapi.json` (about 240 KB of answer), followed by `extra` bytes the
/// service never reads, then the answer read to its end.
fn an_answer_with_bytes_after_its_request(
    world: &World,
    address: SocketAddr,
    config: &Arc<ClientConfig>,
    extra: usize,
) -> Ending {
    let member = world.member(AGENT);
    let headers = request_headers(
        "GET",
        "/openapi.json",
        b"",
        &member.key_id,
        &member.key,
        &world.timestamp(),
    );
    let mut bytes =
        String::from("GET /openapi.json HTTP/1.1\r\nHost: inbox\r\nContent-Length: 0\r\n");
    for (name, value) in headers {
        bytes.push_str(&format!("{name}: {value}\r\n"));
    }
    bytes.push_str("\r\n");
    let mut bytes = bytes.into_bytes();
    bytes.extend(std::iter::repeat_n(b'z', extra));
    let mut stream = connect(address, config);
    let _ = stream.write_all(&bytes).and_then(|()| stream.flush());
    let mut answer = Vec::new();
    if stream.read_to_end(&mut answer).is_err() {
        return Ending::Failed;
    }
    let answer = String::from_utf8_lossy(&answer).into_owned();
    let (head, body) = answer.split_once("\r\n\r\n").unwrap_or((&answer, ""));
    let declared = head
        .split("\r\n")
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(usize::MAX);
    if body.len() == declared {
        Ending::Whole
    } else {
        Ending::CleanButShort {
            declared,
            got: body.len(),
        }
    }
}

/// `close_notify` tells the caller that the stream it read is the whole stream. When the
/// service's end of the pair breaks off before the whole answer reached the caller (here: the
/// caller sent bytes after its request, and the service closed its end with them unread, as
/// over plain HTTP, where the caller then gets a reset), the answer is cut, and the caller must
/// not get a clean end of TLS behind it. Ten tries.
#[test]
fn adv2_tls_an_answer_cut_short_is_never_ended_with_close_notify() {
    let world = World::new();
    let (server, client) = start_tls(&world, |_| {});
    let endings: Vec<Ending> = (0..10)
        .map(|_| {
            an_answer_with_bytes_after_its_request(&world, server.local_addr(), &client, 200_000)
        })
        .collect();
    let clean_but_short: Vec<&Ending> = endings
        .iter()
        .filter(|ending| matches!(ending, Ending::CleanButShort { .. }))
        .collect();
    assert!(
        clean_but_short.is_empty(),
        "an answer cut short ended with close_notify: {endings:?}"
    );
}
