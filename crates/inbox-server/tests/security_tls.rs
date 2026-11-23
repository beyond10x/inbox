//! Conformance cases of the security review of
//! story:the-service-speaks-tls-with-a-pinned-certificate: each case is named for the invariant
//! of the review it measures (the pin decides; one text per pin; validity times are not
//! consulted; TLS 1.3 only, no resumption, no early data; hostile handshake input stays inside
//! the bounds of `docs/design/service.md`). Every certificate, key and pin is made at run time;
//! none is written into this file.

use crate::support;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant as Monotonic};

use inbox_server::request_headers;
use inbox_server::tls::{self, Identity, Pin};
use ring::signature::{Ed25519KeyPair, KeyPair};
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
use rustls::server::{ClientHello, ResolvesServerCert};
use rustls::sign::CertifiedKey;
use rustls::{
    ClientConfig, ClientConnection, HandshakeKind, ProtocolVersion, ServerConfig, ServerConnection,
    StreamOwned,
};
use support::{AGENT, ANSWER_WAIT, World};
use zeroize::Zeroizing;

/// The bytes of a PKCS#8 (version 1) Ed25519 private key before its 32-byte seed (RFC 8410).
const PKCS8_ED25519: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// A server certificate resolver that answers with one fixed chain and signing key, whatever
/// they are: it lets a stand-in present a certificate whose key it does not hold.
#[derive(Debug)]
struct Fixed(Arc<CertifiedKey>);

impl ResolvesServerCert for Fixed {
    fn resolve(&self, _hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        Some(Arc::clone(&self.0))
    }
}

/// A TLS 1.3 server configuration over `chain` that signs with the key of `seed`.
fn stand_in_config(chain: Vec<CertificateDer<'static>>, seed: &[u8; 32]) -> Arc<ServerConfig> {
    let mut pkcs8 = Vec::with_capacity(48);
    pkcs8.extend_from_slice(&PKCS8_ED25519);
    pkcs8.extend_from_slice(seed);
    let pkcs8 = Zeroizing::new(pkcs8);
    let key =
        rustls::crypto::ring::sign::any_eddsa_type(&PrivatePkcs8KeyDer::from(pkcs8.as_slice()))
            .expect("an Ed25519 signing key");
    let resolver = Fixed(Arc::new(CertifiedKey::new(chain, key)));
    let config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .expect("TLS 1.3")
            .with_no_client_auth()
            .with_cert_resolver(Arc::new(resolver));
    Arc::new(config)
}

/// A stand-in on 127.0.0.1:0 with `config`: it answers one connection and hands back every
/// plain byte the client sent over it.
fn recording_stand_in(config: Arc<ServerConfig>) -> (SocketAddr, JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let address = listener.local_addr().expect("an address");
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("the client connects");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
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

fn certificate_of(identity: &Identity) -> CertificateDer<'static> {
    CertificateDer::from(identity.certificate().to_vec())
}

/// The client of `config` against a stand-in of `stand_in`: whether the handshake completed,
/// and the plain bytes the stand-in received.
fn try_stand_in(config: &Arc<ClientConfig>, stand_in: Arc<ServerConfig>) -> (bool, Vec<u8>) {
    let (address, recorded) = recording_stand_in(stand_in);
    let stream = TcpStream::connect(address).expect("the stand-in accepts");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    let completed = match tls::connect(config, stream) {
        Ok(mut stream) => {
            stream.write_all(b"GET / HTTP/1.1\r\n\r\n").expect("sent");
            stream.flush().expect("flushed");
            true
        }
        Err(_) => false,
    };
    (completed, recorded.join().expect("the stand-in ends"))
}

#[test]
fn invariant_1_a_server_with_the_pinned_certificate_but_another_key_fails_the_handshake() {
    let pinned = Identity::generate().expect("a certificate is made");
    let other = Identity::generate().expect("a certificate is made");
    let client = tls::client_config(pinned.pin().clone()).expect("a client configuration");

    // The control: the pinned certificate with its own key completes, and the bytes arrive.
    let (completed, plain) = try_stand_in(
        &client,
        stand_in_config(vec![certificate_of(&pinned)], pinned.seed()),
    );
    assert!(
        completed,
        "the stand-in with the pinned certificate and its key"
    );
    assert_eq!(plain, b"GET / HTTP/1.1\r\n\r\n");

    // The pinned certificate signed for with another key: the handshake signature fails.
    let (completed, plain) = try_stand_in(
        &client,
        stand_in_config(vec![certificate_of(&pinned)], other.seed()),
    );
    assert!(
        !completed,
        "a server that shows the pinned certificate without its key was accepted"
    );
    assert!(plain.is_empty(), "the stand-in received {plain:?}");
}

#[test]
fn invariant_1_the_pin_is_held_to_the_end_entity_certificate_not_to_a_chain_member() {
    let pinned = Identity::generate().expect("a certificate is made");
    let other = Identity::generate().expect("a certificate is made");
    let client = tls::client_config(pinned.pin().clone()).expect("a client configuration");
    // Another end entity with its own key, the pinned certificate behind it in the chain.
    let (completed, plain) = try_stand_in(
        &client,
        stand_in_config(
            vec![certificate_of(&other), certificate_of(&pinned)],
            other.seed(),
        ),
    );
    assert!(
        !completed,
        "a chain that only holds the pinned certificate behind another was accepted"
    );
    assert!(plain.is_empty(), "the stand-in received {plain:?}");
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[test]
fn invariant_2_a_pin_has_one_text_only() {
    // A pin made at run time whose text holds `+` or `/`, so the URL alphabet differs from it.
    let pin = (0..256)
        .map(|_| {
            Identity::generate()
                .expect("a certificate is made")
                .pin()
                .to_base64()
        })
        .find(|pin| pin.contains(['+', '/']))
        .expect("a pin with `+` or `/` within 256 tries");
    let parsed = Pin::parse(&pin).expect("the canonical text reads");
    assert_eq!(parsed.to_base64(), pin);

    // The two low bits of the 43rd symbol are past the 256 bits of the digest: zero in the
    // canonical text, and any other value is another text for the same digest.
    let last = BASE64
        .iter()
        .position(|symbol| *symbol == pin.as_bytes()[42])
        .expect("a base64 symbol");
    assert_eq!(last & 3, 0, "the canonical text has zero trailing bits");
    let mut trailing = pin.clone().into_bytes();
    trailing[42] = BASE64[last | 1];
    let trailing = String::from_utf8(trailing).expect("ASCII");

    let url = pin.replace('+', "-").replace('/', "_");
    let middle = format!("{} {}", &pin[..20], &pin[20..]);
    for (what, text) in [
        ("the URL alphabet", url.as_str()),
        ("no padding", &pin[..43]),
        ("two padding characters", &format!("{pin}=")),
        ("a leading space", &format!(" {pin}")),
        ("a trailing space", &format!("{pin} ")),
        ("a trailing newline", &format!("{pin}\n")),
        ("a space inside", middle.as_str()),
        ("non-zero trailing bits", trailing.as_str()),
        (
            "a padding character inside",
            &format!("{}={}", &pin[..21], &pin[22..]),
        ),
    ] {
        assert!(
            Pin::parse(text).is_err(),
            "{what}: a second text of a pin was read"
        );
    }
}

/// One DER element: `tag`, the length of `content`, `content`.
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let length = content.len();
    if length < 0x80 {
        out.push(u8::try_from(length).expect("short"));
    } else if length < 0x100 {
        out.extend_from_slice(&[0x81, u8::try_from(length).expect("one byte")]);
    } else {
        let length = u16::try_from(length).expect("two bytes");
        out.push(0x82);
        out.extend_from_slice(&length.to_be_bytes());
    }
    out.extend_from_slice(content);
    out
}

/// A self-signed X.509 v3 certificate over the key of `seed`, valid from 2020 to 2021 only.
fn expired_certificate(seed: &[u8; 32]) -> Vec<u8> {
    let pair = Ed25519KeyPair::from_seed_unchecked(seed).expect("an Ed25519 key");
    let algorithm = der(0x30, &[0x06, 0x03, 0x2b, 0x65, 0x70]);
    let name = der(
        0x30,
        &der(
            0x31,
            &der(
                0x30,
                &[&[0x06, 0x03, 0x55, 0x04, 0x03][..], &der(0x0c, b"expired")].concat(),
            ),
        ),
    );
    let validity = der(
        0x30,
        &[der(0x17, b"200101000000Z"), der(0x17, b"210101000000Z")].concat(),
    );
    let spki = der(
        0x30,
        &[
            algorithm.clone(),
            der(0x03, &[&[0][..], pair.public_key().as_ref()].concat()),
        ]
        .concat(),
    );
    let tbs = der(
        0x30,
        &[
            der(0xa0, &der(0x02, &[2])),
            der(0x02, &[0x41, 0x02, 0x03, 0x04]),
            algorithm.clone(),
            name.clone(),
            validity,
            name,
            spki,
        ]
        .concat(),
    );
    let signature = pair.sign(&tbs);
    der(
        0x30,
        &[
            tbs,
            algorithm,
            der(0x03, &[&[0][..], signature.as_ref()].concat()),
        ]
        .concat(),
    )
}

/// Sends a signed `GET /messaging/views/inboxes` over a TLS connection of `config` and answers
/// the status of the reply.
fn status_over_tls(world: &World, address: SocketAddr, config: &Arc<ClientConfig>) -> u16 {
    let target = "/messaging/views/inboxes";
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
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let stream = TcpStream::connect_timeout(&address, ANSWER_WAIT).expect("the service accepts");
    stream
        .set_read_timeout(Some(ANSWER_WAIT))
        .expect("a read timeout");
    let mut stream = tls::connect(config, stream).expect("the handshake completes");
    stream.write_all(head.as_bytes()).expect("the head is sent");
    stream.flush().expect("flushed");
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).expect("a whole answer");
    let answer = String::from_utf8(answer).expect("UTF-8");
    answer
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or_else(|| panic!("a status line in {answer}"))
}

#[test]
fn invariant_3_a_pinned_client_accepts_a_certificate_past_its_not_after_over_the_same_key() {
    let world = World::new();
    let fresh = Identity::generate().expect("a certificate is made");
    let expired = Identity::from_parts(
        expired_certificate(fresh.seed()),
        Zeroizing::new(*fresh.seed()),
    )
    .expect("the expired certificate and its key read as an identity");
    // A certificate made again over the same key keeps the pin.
    assert_eq!(expired.pin(), fresh.pin());
    let client = tls::client_config(fresh.pin().clone()).expect("a client configuration");
    let server = world.start(|config| config.tls = Some(Arc::new(expired)));
    // No validity time is consulted: the documented behaviour.
    assert_eq!(status_over_tls(&world, server.local_addr(), &client), 200);
}

#[test]
fn invariant_5_tls_1_3_only_no_resumption_no_early_data() {
    // rustls is built without TLS 1.2: no configuration here can offer it.
    assert!(
        rustls::ALL_VERSIONS
            .iter()
            .all(|version| version.version == ProtocolVersion::TLSv1_3),
        "rustls offers a version other than TLS 1.3"
    );
    let world = World::new();
    let identity = Identity::generate().expect("a certificate is made");
    let server_config = identity.server_config().expect("a server configuration");
    assert_eq!(server_config.max_early_data_size, 0);
    assert_eq!(server_config.send_tls13_tickets, 0);
    let client = tls::client_config(identity.pin().clone()).expect("a client configuration");
    assert!(!client.enable_early_data, "the client offers early data");
    let server = world.start(|config| config.tls = Some(Arc::new(identity)));
    let address = server.local_addr();
    for _ in 0..2 {
        let mut stream = TcpStream::connect(address).expect("the service accepts");
        stream
            .set_read_timeout(Some(ANSWER_WAIT))
            .expect("a read timeout");
        let mut connection = ClientConnection::new(
            Arc::clone(&client),
            ServerName::IpAddress(address.ip().into()),
        )
        .expect("a client connection");
        while connection.is_handshaking() {
            connection.complete_io(&mut stream).expect("the handshake");
        }
        assert_eq!(
            connection.protocol_version(),
            Some(ProtocolVersion::TLSv1_3)
        );
        assert_eq!(connection.handshake_kind(), Some(HandshakeKind::Full));
    }
}

/// The wait that only ends a hang: a connection the service cut is closed long before it, and a
/// slow host does not reach it. The request timeout of a case that waits for it is far below
/// it or above it.
const HANG: Duration = Duration::from_secs(20);

/// Reads from `stream` until its end, an error or `wait`; answers how long it took.
fn until_closed(mut stream: TcpStream, wait: Duration) -> Duration {
    stream.set_read_timeout(Some(wait)).expect("a read timeout");
    let started = Monotonic::now();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return started.elapsed(),
            Ok(_) => {}
        }
    }
}

#[test]
fn invariant_7_a_handshake_that_never_ends_is_cut_at_the_request_timeout_and_frees_its_slot() {
    let world = World::new();
    let identity = Identity::generate().expect("a certificate is made");
    let client = tls::client_config(identity.pin().clone()).expect("a client configuration");
    let server = world.start(|config| {
        config.tls = Some(Arc::new(identity));
        config.max_connections = 1;
        config.request_timeout = Duration::from_millis(300);
    });
    let address = server.local_addr();
    // The head of a handshake record that announces 512 bytes, and ten of them: then nothing.
    let mut stalled = TcpStream::connect(address).expect("the service accepts");
    stalled
        .write_all(&[
            0x16, 0x03, 0x01, 0x02, 0x00, 0x01, 0x00, 0x01, 0xfc, 0x03, 0x03, 0, 0, 0, 0,
        ])
        .expect("the start of a ClientHello is sent");
    let took = until_closed(stalled, HANG);
    assert!(
        took < HANG,
        "a stalled handshake held its connection {took:?} past a 300 ms request timeout"
    );
    // The one slot is free again: a whole request is served.
    assert_eq!(status_over_tls(&world, address, &client), 200);
}

#[test]
fn invariant_7_a_handshake_message_over_its_bound_ends_the_connection_at_once() {
    let world = World::new();
    let identity = Identity::generate().expect("a certificate is made");
    let server = world.start(|config| {
        config.tls = Some(Arc::new(identity));
        config.request_timeout = Duration::from_secs(30);
    });
    // A handshake record of 16384 bytes whose ClientHello announces 16 MiB.
    let mut record = vec![0x16, 0x03, 0x01, 0x40, 0x00, 0x01, 0xff, 0xff, 0xff];
    record.resize(5 + 0x4000, 0);
    let mut huge = TcpStream::connect(server.local_addr()).expect("the service accepts");
    let _ = huge.write_all(&record);
    let took = until_closed(huge, HANG);
    assert!(
        took < HANG,
        "a ClientHello over the bound held its connection {took:?}"
    );
}

/// The size of a body past the reader's bound: 2 MiB.
const OVERSIZED: usize = 2 * 1024 * 1024;

/// Writes the head of a request that announces a body past the reader's bound, pauses 200 ms
/// (less than the 500 ms gap `http::linger` waits for the rest), writes that body whole, then
/// reads the answer: the status, or what failed.
fn status_after_whole_body(mut stream: impl Read + Write) -> Result<u16, String> {
    let head = format!(
        "POST /messaging/commands/post HTTP/1.1\r\nHost: inbox\r\nContent-Length: {OVERSIZED}\r\n\r\n"
    );
    stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|error| format!("writing the head: {error}"))?;
    thread::sleep(Duration::from_millis(200));
    stream
        .write_all(&vec![b'x'; OVERSIZED])
        .map_err(|error| format!("writing the body: {error}"))?;
    stream
        .flush()
        .map_err(|error| format!("flushing: {error}"))?;
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .map_err(|error| format!("reading the answer: {error}"))?;
    let answer = String::from_utf8_lossy(&answer).into_owned();
    answer
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok())
        .ok_or_else(|| format!("no status line in {answer:?}"))
}

#[test]
fn invariant_7_a_body_past_the_bound_still_arriving_over_tls_is_answered_413_as_over_plain() {
    // The control: over plain HTTP the service answers 413 and drains the rest of the body
    // (`http::linger`), so the caller that writes its whole body still reads the answer.
    let world = World::new();
    let plain = world.start(|_| {});
    let stream = TcpStream::connect(plain.local_addr()).expect("the service accepts");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("a write timeout");
    assert_eq!(status_after_whole_body(stream), Ok(413), "over plain HTTP");

    // Over TLS the same request is to be answered the same: `docs/design/service.md`, TLS, "the
    // body limits answer `413`".
    let identity = Identity::generate().expect("a certificate is made");
    let client = tls::client_config(identity.pin().clone()).expect("a client configuration");
    let over_tls = world.start(|config| config.tls = Some(Arc::new(identity)));
    let stream = TcpStream::connect(over_tls.local_addr()).expect("the service accepts");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("a write timeout");
    let stream = tls::connect(&client, stream).expect("the handshake completes");
    assert_eq!(status_after_whole_body(stream), Ok(413), "over TLS");
}
