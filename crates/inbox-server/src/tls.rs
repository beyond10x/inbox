//! TLS (`docs/design/service.md`, TLS): the certificate the service answers with, the pin a client
//! checks it against, and the ending of TLS in front of the generated reader.
//!
//! The certificate is self-signed over an Ed25519 key. The pin is the SHA-256 of the certificate's
//! SubjectPublicKeyInfo in the base64 of `docs/design/signing.md` ("Encoding on the wire"): 44
//! characters. A certificate renewed over the same key keeps its pin. A client given a pin accepts
//! the certificate whose public key it names and no other, by no other check, and completes the
//! handshake before it sends a byte of its request ([`connect`]).
//!
//! The generated reader takes a `BufReader<TcpStream>`, so TLS cannot sit between the socket and
//! the reader. The service ends TLS in two threads of its own per connection ([`terminate`]) and
//! hands the plain bytes to the reader over a pair of connected loopback sockets: the reader, the
//! watch of the request's arrival and the limits stay as they are for a plain connection.

use std::fmt;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr, Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;

use inbox::server::http;
use inbox::server::json::{self, Value};
use inbox_verify::Instant;
use ring::signature::{Ed25519KeyPair, KeyPair};
use rustls::client::Resumption;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::{CryptoProvider, verify_tls13_signature};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::server::ParsedCertificate;
use rustls::{
    CertificateError, ClientConfig, ClientConnection, DigitallySignedStruct, OtherError,
    ServerConfig, ServerConnection, SignatureScheme, StreamOwned,
};
use zeroize::Zeroizing;

use crate::clock::{Clock, SystemClock, format_instant};

/// The bytes of a PKCS#8 (version 1) Ed25519 private key before its 32-byte seed (RFC 8410).
const PKCS8_ED25519: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];

/// The object identifier of Ed25519, 1.3.101.112, as DER.
const ED25519_OID: [u8; 5] = [0x06, 0x03, 0x2b, 0x65, 0x70];

/// The object identifier of the attribute `commonName`, 2.5.4.3, as DER.
const COMMON_NAME_OID: [u8; 5] = [0x06, 0x03, 0x55, 0x04, 0x03];

/// The subject and issuer of a certificate `Identity::generate` makes.
const COMMON_NAME: &[u8] = b"inbox serve";

/// The size of the buffers of the two threads that end TLS.
const CHUNK: usize = 16 * 1024;

/// The crypto provider of every TLS configuration here: rustls over `ring`.
fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// The pin of a certificate: the SHA-256 of its SubjectPublicKeyInfo.
#[derive(Clone, PartialEq, Eq)]
pub struct Pin([u8; 32]);

impl Pin {
    /// The pin of the DER certificate `certificate`.
    ///
    /// # Errors
    ///
    /// A text naming the cause when the bytes do not parse as an X.509 certificate.
    pub fn of_certificate(certificate: &[u8]) -> Result<Self, String> {
        let certificate = CertificateDer::from(certificate);
        let parsed = ParsedCertificate::try_from(&certificate)
            .map_err(|error| format!("not an X.509 certificate: {error}"))?;
        Ok(Self::of_spki(parsed.subject_public_key_info().as_ref()))
    }

    fn of_spki(spki: &[u8]) -> Self {
        let digest = ring::digest::digest(&ring::digest::SHA256, spki);
        let mut bytes = [0_u8; 32];
        bytes.copy_from_slice(digest.as_ref());
        Self(bytes)
    }

    /// The pin from its text: 44 characters of canonical base64 (standard alphabet, `=` padding)
    /// that decode to 32 bytes.
    ///
    /// # Errors
    ///
    /// A text naming the cause otherwise.
    pub fn parse(text: &str) -> Result<Self, String> {
        let refused = || {
            "a pin is the SHA-256 of a certificate's public key: 44 characters of canonical \
             base64"
                .to_owned()
        };
        let bytes =
            json::bytes_at(&Value::Text(text.to_owned()), "", "base64").map_err(|_| refused())?;
        let bytes: [u8; 32] = bytes.try_into().map_err(|_| refused())?;
        let pin = Self(bytes);
        if pin.to_base64() != text {
            return Err(refused());
        }
        Ok(pin)
    }

    /// The 44 characters of base64.
    #[must_use]
    pub fn to_base64(&self) -> String {
        let mut quoted = String::new();
        json::push_base64(&mut quoted, &self.0);
        quoted[1..quoted.len() - 1].to_owned()
    }
}

impl fmt::Debug for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pin({})", self.to_base64())
    }
}

impl fmt::Display for Pin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base64())
    }
}

/// The certificate the service answers with and its private key. Its `Debug` prints the pin
/// only; the seed is zeroed when it is dropped.
pub struct Identity {
    certificate: Vec<u8>,
    seed: Zeroizing<[u8; 32]>,
    pin: Pin,
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity")
            .field("pin", &self.pin)
            .finish_non_exhaustive()
    }
}

impl Identity {
    /// A fresh Ed25519 key from the OS random source and a self-signed certificate over it, valid
    /// from now on with no end (`99991231235959Z`, RFC 5280).
    ///
    /// # Errors
    ///
    /// When the OS random source does not answer.
    pub fn generate() -> io::Result<Self> {
        let random = |bytes: &mut [u8]| {
            getrandom::fill(bytes)
                .map_err(|error| io::Error::other(format!("the OS random source failed: {error}")))
        };
        let mut seed = Zeroizing::new([0_u8; 32]);
        random(seed.as_mut())?;
        let mut serial = [0_u8; 16];
        random(&mut serial)?;
        let pair = Ed25519KeyPair::from_seed_unchecked(seed.as_ref())
            .map_err(|error| io::Error::other(format!("the Ed25519 key: {error}")))?;
        let certificate = certificate(&pair, serial, SystemClock.now());
        Self::from_parts(certificate, seed).map_err(io::Error::other)
    }

    /// The identity of a DER certificate and the Ed25519 seed of its key, as `inbox tls new`
    /// stores them.
    ///
    /// # Errors
    ///
    /// A text naming the cause when the certificate does not parse, its public key is not the
    /// seed's, or rustls does not take the pair.
    pub fn from_parts(certificate: Vec<u8>, seed: Zeroizing<[u8; 32]>) -> Result<Self, String> {
        let pin = Pin::of_certificate(&certificate)?;
        let pair = Ed25519KeyPair::from_seed_unchecked(seed.as_ref())
            .map_err(|error| format!("the Ed25519 key: {error}"))?;
        if pin != Pin::of_spki(&spki(pair.public_key().as_ref())) {
            return Err("the certificate is not of this key".to_owned());
        }
        let identity = Self {
            certificate,
            seed,
            pin,
        };
        identity
            .server_config()
            .map_err(|error| format!("the certificate and its key: {error}"))?;
        Ok(identity)
    }

    /// The certificate, DER.
    #[must_use]
    pub fn certificate(&self) -> &[u8] {
        &self.certificate
    }

    /// The 32-byte Ed25519 seed of the private key, for `inbox tls new` to store.
    #[must_use]
    pub fn seed(&self) -> &[u8; 32] {
        &self.seed
    }

    /// The pin of the certificate.
    #[must_use]
    pub fn pin(&self) -> &Pin {
        &self.pin
    }

    /// A server configuration that answers with this certificate, TLS 1.3 only, and sends no
    /// session ticket: every connection shows the certificate.
    ///
    /// # Errors
    ///
    /// What rustls says of the certificate or the key.
    pub fn server_config(&self) -> Result<Arc<ServerConfig>, rustls::Error> {
        let mut pkcs8 = Vec::with_capacity(PKCS8_ED25519.len() + 32);
        pkcs8.extend_from_slice(&PKCS8_ED25519);
        pkcs8.extend_from_slice(self.seed.as_ref());
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pkcs8));
        let mut config = ServerConfig::builder_with_provider(provider())
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(vec![CertificateDer::from(self.certificate.clone())], key)?;
        config.send_tls13_tickets = 0;
        Ok(Arc::new(config))
    }
}

/// One DER element: `tag`, the length of `content`, `content`.
fn der(tag: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![tag];
    let length = content.len();
    if length < 0x80 {
        out.push(u8::try_from(length).unwrap_or(0));
    } else {
        let bytes = length.to_be_bytes();
        let skip = bytes.iter().take_while(|byte| **byte == 0).count();
        out.push(0x80 | u8::try_from(bytes.len() - skip).unwrap_or(0));
        out.extend_from_slice(&bytes[skip..]);
    }
    out.extend_from_slice(content);
    out
}

fn concat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

/// The AlgorithmIdentifier of Ed25519: no parameters (RFC 8410).
fn ed25519_algorithm() -> Vec<u8> {
    der(0x30, &ED25519_OID)
}

/// The SubjectPublicKeyInfo of an Ed25519 public key.
fn spki(public_key: &[u8]) -> Vec<u8> {
    der(
        0x30,
        &concat(&[
            &ed25519_algorithm(),
            &der(0x03, &concat(&[&[0], public_key])),
        ]),
    )
}

/// The Name `CN=inbox serve`.
fn name() -> Vec<u8> {
    der(
        0x30,
        &der(
            0x31,
            &der(0x30, &concat(&[&COMMON_NAME_OID, &der(0x0c, COMMON_NAME)])),
        ),
    )
}

/// `instant` to the second as RFC 5280 writes a validity time: UTCTime for the years 1950 to
/// 2049, GeneralizedTime otherwise.
fn validity_time(instant: Instant) -> Vec<u8> {
    let digits: String = format_instant(instant)
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    let year: u32 = digits
        .get(..4)
        .and_then(|year| year.parse().ok())
        .unwrap_or(0);
    if (1950..2050).contains(&year) {
        der(0x17, format!("{}Z", &digits[2..]).as_bytes())
    } else {
        der(0x18, format!("{digits}Z").as_bytes())
    }
}

/// A self-signed X.509 version 3 certificate over `pair`, valid from `now` with no end.
fn certificate(pair: &Ed25519KeyPair, mut serial: [u8; 16], now: Instant) -> Vec<u8> {
    // Positive, and its first byte is not zero: the INTEGER is in its shortest form.
    serial[0] = (serial[0] & 0x7f) | 0x40;
    let validity = der(
        0x30,
        &concat(&[&validity_time(now), &der(0x18, b"99991231235959Z")]),
    );
    let tbs = der(
        0x30,
        &concat(&[
            &der(0xa0, &der(0x02, &[2])),
            &der(0x02, &serial),
            &ed25519_algorithm(),
            &name(),
            &validity,
            &name(),
            &spki(pair.public_key().as_ref()),
        ]),
    );
    let signature = pair.sign(&tbs);
    der(
        0x30,
        &concat(&[
            &tbs,
            &ed25519_algorithm(),
            &der(0x03, &concat(&[&[0], signature.as_ref()])),
        ]),
    )
}

/// The refusal of a certificate whose public key the pin does not name.
#[derive(Debug)]
struct NotPinned;

impl fmt::Display for NotPinned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the certificate's public key does not match the pin")
    }
}

impl std::error::Error for NotPinned {}

/// The one check of a server certificate a client given a pin makes: its SubjectPublicKeyInfo
/// hashes to the pin. No name, no validity time, no authority is consulted, and nothing else
/// accepts a certificate. The handshake signature is still verified with the certificate's key.
#[derive(Debug)]
struct Pinned {
    pin: Pin,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for Pinned {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        match Pin::of_certificate(end_entity.as_ref()) {
            Ok(pin) if pin == self.pin => Ok(ServerCertVerified::assertion()),
            Ok(_) => Err(rustls::Error::InvalidCertificate(CertificateError::Other(
                OtherError(Arc::new(NotPinned)),
            ))),
            Err(_) => Err(rustls::Error::InvalidCertificate(
                CertificateError::BadEncoding,
            )),
        }
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // Only TLS 1.3 is offered; rustls never asks this.
        Err(rustls::Error::General("TLS 1.2 is not offered".to_owned()))
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// A client configuration, TLS 1.3 only, that accepts the certificate `pin` names and no other.
/// It resumes no session: every handshake checks the certificate it is shown.
///
/// # Errors
///
/// What rustls says of the protocol versions (nothing with the `ring` provider).
pub fn client_config(pin: Pin) -> Result<Arc<ClientConfig>, rustls::Error> {
    let provider = provider();
    let verifier = Arc::new(Pinned {
        pin,
        provider: Arc::clone(&provider),
    });
    let mut config = ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])?
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    config.resumption = Resumption::disabled();
    Ok(Arc::new(config))
}

/// A TLS connection of a client whose handshake is complete.
#[derive(Debug)]
pub struct ClientStream(StreamOwned<ClientConnection, TcpStream>);

impl Read for ClientStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer)
    }
}

impl Write for ClientStream {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

/// Completes the TLS handshake on `stream` with `config` before anything else is written: a
/// certificate `config` does not accept ends the handshake with an error, and no byte of a
/// request has been sent then.
///
/// # Errors
///
/// What the handshake or the socket fails with; a certificate the pin does not name is an
/// [`io::ErrorKind::InvalidData`] error that says so.
pub fn connect(config: &Arc<ClientConfig>, mut stream: TcpStream) -> io::Result<ClientStream> {
    let server = ServerName::IpAddress(stream.peer_addr()?.ip().into());
    let mut connection =
        ClientConnection::new(Arc::clone(config), server).map_err(io::Error::other)?;
    while connection.is_handshaking() {
        let (read, written) = connection.complete_io(&mut stream).map_err(not_pinned)?;
        if read == 0 && written == 0 && connection.is_handshaking() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the connection ended during the TLS handshake",
            ));
        }
    }
    Ok(ClientStream(StreamOwned::new(connection, stream)))
}

/// The error of a handshake the pin ended, said in words (rustls writes the debug form of the
/// cause); every other error unchanged.
fn not_pinned(error: io::Error) -> io::Error {
    let pinned = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<rustls::Error>())
        .is_some_and(|inner| {
            matches!(
                inner,
                rustls::Error::InvalidCertificate(CertificateError::Other(_))
            )
        });
    if pinned {
        io::Error::new(io::ErrorKind::InvalidData, NotPinned)
    } else {
        error
    }
}

/// Two connected loopback sockets that no third party is one end of: the end this process
/// accepted is checked to come from the end it connected.
fn pair() -> io::Result<(TcpStream, TcpStream)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .or_else(|_| TcpListener::bind((Ipv6Addr::LOCALHOST, 0)))?;
    pair_on(&listener)
}

/// [`pair`] on a `listener` given: a connection anyone else made to it, before or after, is
/// dropped unread.
fn pair_on(listener: &TcpListener) -> io::Result<(TcpStream, TcpStream)> {
    let near = TcpStream::connect(listener.local_addr()?)?;
    let expected = near.local_addr()?;
    loop {
        let (far, from) = listener.accept()?;
        if from == expected {
            let _ = far.set_nodelay(true);
            let _ = near.set_nodelay(true);
            return Ok((far, near));
        }
    }
}

fn lock(connection: &Mutex<ServerConnection>) -> MutexGuard<'_, ServerConnection> {
    connection.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Writes what `connection` has to send to `outer`.
fn flush(connection: &mut ServerConnection, outer: &mut TcpStream) -> io::Result<()> {
    while connection.wants_write() {
        connection.write_tls(outer)?;
    }
    outer.flush()
}

/// Ends TLS on `outer` with `config` and runs `serve` on the plain end of a loopback socket pair,
/// on this thread. Two threads carry the bytes: one reads TLS records from `outer` and writes the
/// plain bytes they hold to the pair, one reads the answer from the pair and writes it to `outer`
/// as TLS records. Returns when `serve` has returned and both threads have ended: `serve` closes
/// its end, which ends the second thread (it closes TLS and shuts `outer` for writing), and then
/// `outer` is shut for reading too, which ends the first.
///
/// A connection whose pair or threads cannot be made is dropped without an answer, as a
/// connection whose thread cannot be started is.
pub(crate) fn terminate(
    config: &Arc<ServerConfig>,
    outer: TcpStream,
    serve: impl FnOnce(TcpStream),
) {
    let Ok(connection) = ServerConnection::new(Arc::clone(config)) else {
        return;
    };
    let Ok((plain, inner)) = pair() else {
        return;
    };
    let _ = outer.set_write_timeout(Some(http::WRITE_TIMEOUT));
    let (Ok(outer_in), Ok(outer_out), Ok(inner_out)) =
        (outer.try_clone(), outer.try_clone(), inner.try_clone())
    else {
        return;
    };
    let connection = Arc::new(Mutex::new(connection));
    let inbound = {
        let connection = Arc::clone(&connection);
        thread::Builder::new()
            .name("inbox-tls-in".to_owned())
            .spawn(move || inbound(&connection, outer_in, inner))
    };
    let Ok(inbound) = inbound else {
        return;
    };
    let outbound = {
        let connection = Arc::clone(&connection);
        thread::Builder::new()
            .name("inbox-tls-out".to_owned())
            .spawn(move || outbound(&connection, inner_out, outer_out))
    };
    let Ok(outbound) = outbound else {
        let _ = outer.shutdown(Shutdown::Both);
        drop(plain);
        let _ = inbound.join();
        return;
    };
    serve(plain);
    let _ = outbound.join();
    // Only now, with the service's end closed, is `outer` shut for reading: until then the first
    // thread keeps reading, so the rest of a request the service refused while it was still
    // arriving is read and dropped (`http::linger`) as over plain HTTP, not reset.
    let _ = outer.shutdown(Shutdown::Both);
    let _ = inbound.join();
}

/// Reads TLS records from `outer`, answers the handshake on it, and writes the plain bytes to
/// `inner`. At the end of `outer`, a close of TLS or a TLS error (its alert sent), shuts `inner`
/// for writing: the reader sees the end of the stream.
fn inbound(connection: &Mutex<ServerConnection>, mut outer: TcpStream, mut inner: TcpStream) {
    let mut buffer = vec![0_u8; CHUNK];
    let mut writer = match outer.try_clone() {
        Ok(writer) => writer,
        Err(_) => {
            let _ = inner.shutdown(Shutdown::Write);
            return;
        }
    };
    loop {
        let read = match outer.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => read,
        };
        let mut plain = Vec::new();
        let mut ended = false;
        {
            let mut connection = lock(connection);
            let mut rest = &buffer[..read];
            while !rest.is_empty() && !ended {
                if connection.read_tls(&mut rest).is_err() {
                    ended = true;
                    break;
                }
                match connection.process_new_packets() {
                    Ok(state) => {
                        let start = plain.len();
                        plain.resize(start + state.plaintext_bytes_to_read(), 0);
                        if connection.reader().read_exact(&mut plain[start..]).is_err() {
                            ended = true;
                        }
                        ended |= state.peer_has_closed();
                    }
                    Err(_) => ended = true,
                }
            }
            if flush(&mut connection, &mut writer).is_err() {
                ended = true;
            }
        }
        if !plain.is_empty() && inner.write_all(&plain).is_err() {
            break;
        }
        if ended {
            break;
        }
    }
    let _ = inner.shutdown(Shutdown::Write);
}

/// Reads the answer from `inner` and writes it to `outer` as TLS records. At the end of `inner`
/// (the service shut its end for writing), closes TLS and shuts `outer` for writing; `inner` stays
/// open for the first thread, whose bytes the service may still read and drop. When `outer` or
/// `inner` fails, the answer is cut: `outer` is shut without closing TLS, so the caller's read
/// fails rather than ending cleanly on a short answer, and `inner` is shut too, so the service's
/// next write fails instead of waiting.
fn outbound(connection: &Mutex<ServerConnection>, mut inner: TcpStream, mut outer: TcpStream) {
    let mut buffer = vec![0_u8; CHUNK];
    let failed = loop {
        let read = match inner.read(&mut buffer) {
            // The end of a stream the service closed, or of one reset under the rest of its
            // answer: the reset's error goes to whichever thread meets it first, and then a read
            // here sees only the end. A reset pair is no longer connected; one the service shut
            // for writing still is.
            Ok(0) => break inner.peer_addr().is_err(),
            Err(_) => break true,
            Ok(read) => read,
        };
        let mut connection = lock(connection);
        if connection.writer().write_all(&buffer[..read]).is_err()
            || flush(&mut connection, &mut outer).is_err()
        {
            break true;
        }
    };
    if !failed {
        let mut connection = lock(connection);
        connection.send_close_notify();
        let _ = flush(&mut connection, &mut outer);
    }
    let _ = outer.shutdown(Shutdown::Write);
    if failed {
        let _ = inner.shutdown(Shutdown::Both);
    }
}

#[cfg(test)]
mod tests {
    use super::{Identity, Pin, client_config, connect, der, pair_on, terminate};
    use std::io::{ErrorKind, Read, Write};
    use std::net::{Ipv4Addr, Shutdown, TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, mpsc};
    use std::thread;
    use std::time::Duration;

    /// The listener holds a connection's slot of `max_connections` while `terminate` runs, so
    /// this is the slot held until the thread that writes the answer has ended, after `serve`
    /// returned.
    #[test]
    fn terminate_returns_only_when_the_thread_that_writes_the_answer_has_ended() {
        let identity = Identity::generate().unwrap();
        let server = identity.server_config().unwrap();
        let client = client_config(identity.pin().clone()).unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let caller = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        caller
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let (outer, _) = listener.accept().unwrap();
        let (release, released) = mpsc::channel::<()>();
        let (served, serve_returned) = mpsc::channel();
        let (ended, terminate_returned) = mpsc::channel();
        thread::spawn(move || {
            terminate(&server, outer, move |mut plain| {
                // The answer is still on its way when `serve` returns: the service's end is open.
                thread::spawn(move || {
                    let _ = released.recv();
                    let _ = plain.write_all(b"answer");
                });
                let _ = served.send(());
            });
            let _ = ended.send(());
        });
        let mut stream = connect(&client, caller).unwrap();
        serve_returned
            .recv_timeout(Duration::from_secs(10))
            .expect("serve returns");
        assert!(
            terminate_returned
                .recv_timeout(Duration::from_millis(300))
                .is_err(),
            "terminate returned while its answer was still being written"
        );
        release.send(()).unwrap();
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).unwrap();
        assert_eq!(answer, b"answer");
        terminate_returned
            .recv_timeout(Duration::from_secs(10))
            .expect("terminate returns once both threads have ended");
    }

    /// The wait that only ends a hang.
    const HANG: Duration = Duration::from_secs(20);

    /// A caller over TLS of `terminate` running `serve` on a thread of its own, its handshake
    /// done; beside it a clone of its socket, and what says `terminate` has returned.
    fn caller_of(
        serve: impl FnOnce(TcpStream) + Send + 'static,
    ) -> (super::ClientStream, TcpStream, mpsc::Receiver<()>) {
        let identity = Identity::generate().unwrap();
        let server = identity.server_config().unwrap();
        let client = client_config(identity.pin().clone()).unwrap();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let caller = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        caller.set_read_timeout(Some(HANG)).unwrap();
        caller.set_write_timeout(Some(HANG)).unwrap();
        let raw = caller.try_clone().unwrap();
        let (outer, _) = listener.accept().unwrap();
        let (ended, terminate_returned) = mpsc::channel();
        thread::spawn(move || {
            terminate(&server, outer, serve);
            let _ = ended.send(());
        });
        (connect(&client, caller).unwrap(), raw, terminate_returned)
    }

    /// Writes to `stream` on a thread of its own until a write fails, and returns once the
    /// writes have stopped moving for 500 ms: every buffer on the way into the service is full,
    /// and the thread that reads the caller is held writing into the pair, which the service
    /// does not read. The thread ends when the caller's socket is shut or fails.
    fn fill_the_way_in(mut stream: super::ClientStream) -> thread::JoinHandle<()> {
        let sent = Arc::new(AtomicUsize::new(0));
        let writer = {
            let sent = Arc::clone(&sent);
            thread::spawn(move || {
                let chunk = vec![b'f'; 65_536];
                while stream.write_all(&chunk).is_ok() {
                    sent.fetch_add(chunk.len(), Ordering::SeqCst);
                }
            })
        };
        let (mut last, mut still, mut polls) = (0, 0, 0);
        while still < 10 {
            polls += 1;
            assert!(polls < 400, "the way in never filled");
            thread::sleep(Duration::from_millis(50));
            let now = sent.load(Ordering::SeqCst);
            if now == last && now > 0 {
                still += 1;
            } else {
                (last, still) = (now, 0);
            }
        }
        writer
    }

    /// The thread that reads the caller can outlive `serve`: held writing into the pair, it ends
    /// only when the service's end is closed. `terminate`, and with it the connection's slot of
    /// `max_connections`, waits for it.
    #[test]
    fn terminate_returns_only_when_the_thread_that_reads_the_caller_has_ended() {
        let (full, filled) = mpsc::channel::<()>();
        let (handed, held) = mpsc::channel();
        let (stream, raw, terminate_returned) = caller_of(move |plain| {
            let _ = filled.recv_timeout(HANG);
            // The answer is whole; the service's end stays open, unread, beyond `serve`.
            let _ = plain.shutdown(Shutdown::Write);
            let _ = handed.send(plain);
        });
        let writer = fill_the_way_in(stream);
        full.send(()).unwrap();
        let plain: TcpStream = held.recv_timeout(HANG).expect("serve returns");
        assert!(
            terminate_returned
                .recv_timeout(Duration::from_millis(300))
                .is_err(),
            "terminate returned while the thread that reads the caller was still writing"
        );
        drop(plain);
        terminate_returned
            .recv_timeout(HANG)
            .expect("terminate returns once the service's end is closed");
        let _ = raw.shutdown(Shutdown::Both);
        writer.join().unwrap();
    }

    /// When the caller is gone in the middle of an answer, the service's next write to its end of
    /// the pair fails at once, also while the thread that reads the caller is held writing into
    /// the pair and keeps the pair open.
    #[test]
    fn a_caller_gone_in_the_middle_of_an_answer_fails_the_services_next_write() {
        let (failed_with, failure) = mpsc::channel();
        let (full, filled) = mpsc::channel::<()>();
        let (stream, raw, terminate_returned) = caller_of(move |mut plain| {
            // Longer than the test waits for the failure: a write that waits it out is red.
            plain.set_write_timeout(Some(HANG + HANG)).unwrap();
            // The answer starts after the first byte of the request, as a service's does, and
            // once the way in is full: the thread that reads the caller is then held writing
            // into the pair, not waiting for the lock the thread that writes the answer holds.
            let mut first = [0_u8; 1];
            let _ = plain.read_exact(&mut first);
            let _ = filled.recv_timeout(HANG);
            let chunk = vec![b'a'; 65_536];
            let kind = loop {
                if let Err(error) = plain.write_all(&chunk) {
                    break error.kind();
                }
            };
            let _ = failed_with.send(kind);
        });
        let writer = fill_the_way_in(stream);
        full.send(()).unwrap();
        // The caller goes, its answer unread: closing its socket resets the connection.
        raw.shutdown(Shutdown::Both).unwrap();
        writer.join().unwrap();
        drop(raw);
        let kind = failure
            .recv_timeout(HANG)
            .expect("the service's write fails");
        assert!(
            matches!(kind, ErrorKind::BrokenPipe | ErrorKind::ConnectionReset),
            "the service's write to a caller that is gone ended with {kind:?}"
        );
        terminate_returned
            .recv_timeout(HANG)
            .expect("terminate returns");
    }

    #[test]
    fn an_intruder_that_connects_to_the_pair_first_is_dropped_and_gets_no_byte() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let mut intruder = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut far, mut near) = pair_on(&listener).unwrap();
        assert_eq!(far.peer_addr().unwrap(), near.local_addr().unwrap());
        assert_ne!(far.peer_addr().unwrap(), intruder.local_addr().unwrap());
        // The pair carries bytes both ways.
        near.write_all(b"to far").unwrap();
        far.write_all(b"to near").unwrap();
        let mut buffer = [0_u8; 7];
        far.read_exact(&mut buffer[..6]).unwrap();
        assert_eq!(&buffer[..6], b"to far");
        near.read_exact(&mut buffer).unwrap();
        assert_eq!(&buffer, b"to near");
        // The intruder's connection was closed without a byte. The bound only ends a hang.
        intruder
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut got = Vec::new();
        match intruder.read_to_end(&mut got) {
            Ok(_) => assert!(got.is_empty(), "the intruder got {} bytes", got.len()),
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset),
        }
    }

    #[test]
    fn a_generated_certificate_parses_and_its_pin_is_44_characters_of_base64() {
        let identity = Identity::generate().unwrap();
        let pin = identity.pin().to_base64();
        assert_eq!(pin.len(), 44);
        assert_eq!(Pin::parse(&pin).unwrap(), *identity.pin());
        assert_eq!(
            Pin::of_certificate(identity.certificate()).unwrap(),
            *identity.pin()
        );
        assert!(!format!("{identity:?}").contains(&format!("{:?}", identity.seed())));
    }

    #[test]
    fn two_identities_have_two_pins_and_a_certificate_of_another_key_is_refused() {
        let one = Identity::generate().unwrap();
        let two = Identity::generate().unwrap();
        assert_ne!(one.pin(), two.pin());
        let seed = zeroize::Zeroizing::new(*two.seed());
        let refused = Identity::from_parts(one.certificate().to_vec(), seed).unwrap_err();
        assert!(refused.contains("not of this key"), "{refused}");
    }

    #[test]
    fn a_pin_that_is_not_canonical_base64_of_32_bytes_is_refused() {
        let pin = Identity::generate().unwrap().pin().to_base64();
        for wrong in [
            &pin[..43],
            &format!("{pin}="),
            "",
            &pin.replace('=', "A"),
            &format!("{}=", &pin[..42]),
        ] {
            assert!(Pin::parse(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn a_long_der_length_is_written_in_its_long_form() {
        assert_eq!(der(0x04, &[7; 3])[..2], [0x04, 3]);
        assert_eq!(der(0x04, &[7; 200])[..3], [0x04, 0x81, 200]);
        assert_eq!(der(0x04, &[7; 300])[..4], [0x04, 0x82, 0x01, 0x2c]);
    }
}
