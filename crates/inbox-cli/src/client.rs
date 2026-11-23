//! The client side of the wire: the service's address from `INBOX_URL`, one signed request per
//! connection over a `TcpStream` (HTTP/1.1, no keep-alive, as the service answers), over TLS
//! whose certificate is checked against the pin `INBOX_TLS_PIN` names before a byte of the
//! request is sent (`https://`), or plain (`http://`), and what its answer means.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use inbox::server::inbox_service::ROUTES;
use inbox::server::json::{self, Value};
use inbox_server::tls::{self, Pin};
use inbox_server::{Clock, EPOCH_HEADER, SystemClock, format_instant, request_headers};

use crate::failure::Failure;
use crate::keys::Caller;
use crate::output;

/// How long a connection may take to open.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How long an answer may take, beyond the `wait` a request asks for.
const ANSWER_TIMEOUT: Duration = Duration::from_secs(30);

/// The path of a command (`POST`) or a view (`GET`) by its wire name, read from the generated
/// route table.
///
/// # Panics
///
/// When the table holds no such route: the name is the client's mistake, and a test holds every
/// name the client uses to the table.
pub fn route(method: &str, wire: &str) -> &'static str {
    ROUTES
        .iter()
        .find(|(held, path)| *held == method && path.rsplit('/').next() == Some(wire))
        .map(|(_, path)| *path)
        .unwrap_or_else(|| panic!("the generated route table holds no `{method}` route `{wire}`"))
}

/// Anything a request travels over: a plain socket or a TLS connection whose handshake is done.
trait Wire: Read + Write {}

impl<T: Read + Write> Wire for T {}

/// The service a client talks to: `INBOX_URL`, `http://<host>:<port>` or `https://<host>:<port>`,
/// on a loopback address, and over TLS the pin of its certificate.
#[derive(Debug, Clone)]
pub struct Service {
    addresses: Vec<SocketAddr>,
    host: String,
    pin: Option<Pin>,
}

impl Service {
    /// Reads `http://<host>:<port>` or `https://<host>:<port>` with an optional trailing `/`, and
    /// the pin (`INBOX_TLS_PIN`). An `https://` URL needs a pin, and a pin needs an `https://`
    /// URL: a client given a pin never falls back to plain HTTP. The service binds a loopback
    /// address only in this stage, so the client talks to a loopback address only: every address
    /// the host resolves to must be one.
    ///
    /// # Errors
    ///
    /// [`Failure::Usage`] when no URL is given, it is not of that form, its host does not
    /// resolve, or it resolves to an address that is not loopback; when the pin is not one, an
    /// `https://` URL comes without a pin, or a pin with an `http://` URL.
    pub fn from_url(url: Option<&str>, pin: Option<&str>) -> Result<Self, Failure> {
        let url = url.ok_or_else(|| {
            Failure::Usage("the service: pass --url <url> or set INBOX_URL".to_owned())
        })?;
        let pin = pin
            .map(|text| {
                Pin::parse(text).map_err(|why| Failure::Usage(format!("INBOX_TLS_PIN: {why}")))
            })
            .transpose()?;
        let refused = || {
            Failure::Usage(format!(
                "INBOX_URL `{url}` is not http://<host>:<port> or https://<host>:<port> (no path \
                 in this stage)"
            ))
        };
        let (host, tls) = match (url.strip_prefix("https://"), url.strip_prefix("http://")) {
            (Some(host), _) => (host, true),
            (None, Some(host)) => (host, false),
            (None, None) => return Err(refused()),
        };
        match (tls, &pin) {
            (true, None) => {
                return Err(Failure::Usage(format!(
                    "INBOX_URL `{url}` is https: pass the pin of the service's certificate with \
                     --tls-pin or INBOX_TLS_PIN"
                )));
            }
            (false, Some(_)) => {
                return Err(Failure::Usage(format!(
                    "INBOX_URL `{url}` is http, and a pin is given: a client given a pin talks \
                     TLS only (https://), never plain HTTP"
                )));
            }
            _ => {}
        }
        let host = host.strip_suffix('/').unwrap_or(host);
        if host.is_empty() || host.contains(['/', '?', '#', '@']) {
            return Err(refused());
        }
        let addresses: Vec<SocketAddr> = host.to_socket_addrs().map_err(|_| refused())?.collect();
        if addresses.is_empty() {
            return Err(refused());
        }
        if let Some(outside) = addresses.iter().find(|address| !address.ip().is_loopback()) {
            return Err(Failure::Usage(format!(
                "INBOX_URL `{url}`: {outside} is not a loopback address; the service serves on a \
                 loopback address only in this stage, and the client sends nowhere else"
            )));
        }
        Ok(Self {
            addresses,
            host: host.to_owned(),
            pin,
        })
    }

    /// A connection to the first of the resolved addresses that accepts one.
    fn connect(&self) -> Result<TcpStream, Failure> {
        let mut failures = Vec::new();
        for address in &self.addresses {
            match TcpStream::connect_timeout(address, CONNECT_TIMEOUT) {
                Ok(stream) => return Ok(stream),
                Err(error) => failures.push(format!("{address}: {error}")),
            }
        }
        Err(Failure::Transport(format!(
            "{}: {}",
            self.host,
            failures.join("; ")
        )))
    }

    /// Sends one request signed by `caller` at the clock reading and reads the whole answer. Over
    /// TLS the handshake is complete, and the certificate held to the pin, before a byte of the
    /// request is written.
    ///
    /// # Errors
    ///
    /// [`Failure::Transport`] when the service cannot be reached, its certificate does not match
    /// the pin, or the answer is not HTTP.
    pub fn send(
        &self,
        caller: &Caller,
        method: &str,
        target: &str,
        body: &[u8],
        wait: Duration,
    ) -> Result<Reply, Failure> {
        let timestamp = format_instant(SystemClock.now());
        let headers = request_headers(
            method,
            target,
            body,
            &caller.key_id,
            &caller.key,
            &timestamp,
        );
        let socket = self.connect()?;
        let peer = socket
            .peer_addr()
            .map_or_else(|_| self.host.clone(), |peer| peer.to_string());
        let transport =
            |error: std::io::Error| Failure::Transport(format!("{} ({peer}): {error}", self.host));
        socket
            .set_read_timeout(Some(wait.saturating_add(ANSWER_TIMEOUT)))
            .map_err(transport)?;
        socket
            .set_write_timeout(Some(ANSWER_TIMEOUT))
            .map_err(transport)?;
        let mut stream: Box<dyn Wire> = match &self.pin {
            None => Box::new(socket),
            Some(pin) => {
                let config = tls::client_config(pin.clone())
                    .map_err(|error| transport(std::io::Error::other(error)))?;
                Box::new(tls::connect(&config, socket).map_err(transport)?)
            }
        };
        let mut head = format!(
            "{method} {target} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\nConnection: close\r\n",
            self.host,
            body.len()
        );
        for (name, value) in headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str("\r\n");
        stream.write_all(head.as_bytes()).map_err(transport)?;
        stream.write_all(body).map_err(transport)?;
        stream.flush().map_err(transport)?;
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).map_err(transport)?;
        Reply::read(&answer)
            .map_err(|why| Failure::Transport(format!("{} ({peer}): {why}", self.host)))
    }
}

/// The epoch of one start of the service, as its answers name it in [`EPOCH_HEADER`]: 32
/// lower-case hex digits (`docs/design/service.md`, the epoch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epoch(String);

impl Epoch {
    /// The epoch `text` names, when it is 32 lower-case hex digits, as the service writes one.
    pub fn parse(text: &str) -> Option<Self> {
        (text.len() == 32
            && text
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        .then(|| Self(text.to_owned()))
    }

    /// The 32 hex digits.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One answer of the service.
#[derive(Debug)]
pub struct Reply {
    /// The HTTP status.
    pub status: u16,
    /// The body, read as JSON.
    pub body: Value,
    /// The epoch the answer names, whatever its status; `None` when its head carries no
    /// [`EPOCH_HEADER`] (an older service), carries two different ones, or one that is not an
    /// epoch.
    pub epoch: Option<Epoch>,
}

impl Reply {
    fn read(answer: &[u8]) -> Result<Self, String> {
        let answer = std::str::from_utf8(answer).map_err(|_| "the answer is not UTF-8")?;
        let (head, body) = answer
            .split_once("\r\n\r\n")
            .ok_or("the answer has no end of its head")?;
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|status| status.parse().ok())
            .ok_or("the answer has no status line")?;
        let body = json::parse(body).map_err(|error| format!("the answer is not JSON: {error}"))?;
        Ok(Self {
            status,
            body,
            epoch: epoch(head),
        })
    }

    /// The answer, or the refusal it is. A `2xx` answer is the only success. A body that carries
    /// `outcome` and `error` is a declared refusal of the model, named by its `error`; a status
    /// from 400 on whose body carries `refused` is the service's own refusal, named by that text.
    ///
    /// # Errors
    ///
    /// [`Failure::Refused`] for either refusal; [`Failure::Transport`] naming the status for any
    /// other answer (a `3xx`, or an error without a `refused` text), which no service of the
    /// inbox gives.
    pub fn accepted(self) -> Result<Value, Failure> {
        if let Some(name) = declared_error(&self.body) {
            return Err(Failure::Refused {
                name: name.to_owned(),
                status: self.status,
                answer: self.body,
            });
        }
        if let (400.., Some(Value::Text(name))) = (self.status, self.body.member("refused")) {
            return Err(Failure::Refused {
                name: name.clone(),
                status: self.status,
                answer: self.body,
            });
        }
        if (200..300).contains(&self.status) {
            return Ok(self.body);
        }
        Err(Failure::Transport(format!(
            "the answer has the status {}, which is neither a success nor a refusal of the \
             service: {}",
            self.status,
            output::document(&self.body)
        )))
    }
}

/// The epoch the head of an answer names: the value of its [`EPOCH_HEADER`] headers (the name
/// compared without regard to case) when they all carry one and it is an [`Epoch`].
fn epoch(head: &str) -> Option<Epoch> {
    let mut values = head
        .split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case(EPOCH_HEADER))
        .map(|(_, value)| value.trim());
    let first = values.next()?;
    if values.any(|value| value != first) {
        return None;
    }
    Epoch::parse(first)
}

/// The model's error name of a declared refusal: the `error` member the generated route writes
/// beside `outcome` (`{"outcome":"unknown-inbox","published":[],"error":"inbox.messaging.UnknownInbox"}`).
pub fn declared_error(body: &Value) -> Option<&str> {
    match (body.member("outcome"), body.member("error")) {
        (Some(Value::Text(_)), Some(Value::Text(name))) => Some(name),
        _ => None,
    }
}

/// `text` percent-encoded for a query value: every byte but the unreserved ones as `%XX`.
pub fn query_value(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Epoch, Reply, Service, query_value, route};

    const EPOCH: &str = "0123456789abcdef0123456789abcdef";

    fn answer(headers: &str, status: u16) -> Reply {
        let text = format!(
            "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n{headers}Connection: close\r\n\r\n{{}}"
        );
        Reply::read(text.as_bytes()).unwrap()
    }

    #[test]
    fn the_epoch_is_read_from_the_head_of_an_answer_of_any_status() {
        for status in [200, 202, 400, 401, 404, 503] {
            let reply = answer(&format!("Inbox-Epoch: {EPOCH}\r\n"), status);
            assert_eq!(reply.epoch, Epoch::parse(EPOCH), "{status}");
        }
        let reply = answer(&format!("inbox-EPOCH:{EPOCH} \r\n"), 200);
        assert_eq!(reply.epoch.as_ref().map(Epoch::as_str), Some(EPOCH));
    }

    #[test]
    fn no_header_two_values_or_a_value_that_is_no_epoch_is_no_epoch() {
        assert_eq!(answer("", 200).epoch, None);
        let other = "fedcba9876543210fedcba9876543210";
        let two = format!("Inbox-Epoch: {EPOCH}\r\nInbox-Epoch: {other}\r\n");
        assert_eq!(answer(&two, 200).epoch, None);
        let same = format!("Inbox-Epoch: {EPOCH}\r\nInbox-Epoch: {EPOCH}\r\n");
        assert_eq!(answer(&same, 200).epoch, Epoch::parse(EPOCH));
        for wrong in [
            "0123456789ABCDEF0123456789ABCDEF",
            "0123456789abcdef",
            "0123456789abcdef0123456789abcdefa",
            "0123456789abcdef 123456789abcdef",
            "",
        ] {
            let reply = answer(&format!("Inbox-Epoch: {wrong}\r\n"), 200);
            assert_eq!(reply.epoch, None, "{wrong:?}");
        }
    }

    /// Every wire name the client sends to, by method.
    pub const USED: &[(&str, &str)] = &[
        ("POST", "open-inbox"),
        ("POST", "close-inbox"),
        ("POST", "post"),
        ("POST", "read"),
        ("POST", "answer"),
        ("POST", "refuse"),
        ("POST", "collect"),
        ("GET", "pending"),
        ("GET", "outbox"),
        ("GET", "thread"),
        ("GET", "replies"),
    ];

    #[test]
    fn every_route_the_client_uses_is_in_the_generated_table() {
        for (method, wire) in USED {
            assert!(route(method, wire).ends_with(&format!("/{wire}")));
        }
    }

    #[test]
    #[should_panic(expected = "holds no `GET` route `post`")]
    fn a_route_not_in_the_table_is_the_clients_mistake() {
        route("GET", "post");
    }

    #[test]
    fn an_address_is_encoded_for_a_query() {
        assert_eq!(
            query_value("agent:host-a/one two"),
            "agent%3Ahost-a%2Fone%20two"
        );
    }

    #[test]
    fn the_url_is_http_host_and_port_only() {
        assert!(Service::from_url(Some("http://127.0.0.1:7700"), None).is_ok());
        assert!(Service::from_url(Some("http://127.0.0.1:7700/"), None).is_ok());
        for refused in [
            "https://127.0.0.1:7700",
            "127.0.0.1:7700",
            "http://127.0.0.1:7700/inbox",
            "http://",
        ] {
            assert!(Service::from_url(Some(refused), None).is_err(), "{refused}");
        }
        assert!(Service::from_url(None, None).is_err());
    }

    /// A pin made at run time: no pin is written into this file.
    fn a_pin() -> String {
        inbox_server::tls::Identity::generate()
            .unwrap()
            .pin()
            .to_base64()
    }

    #[test]
    fn an_https_url_needs_a_pin_and_a_pin_needs_an_https_url() {
        let pin = a_pin();
        assert!(Service::from_url(Some("https://127.0.0.1:7700"), Some(&pin)).is_ok());
        assert!(Service::from_url(Some("https://[::1]:7700/"), Some(&pin)).is_ok());
        for (url, pin) in [
            ("https://127.0.0.1:7700", None),
            ("http://127.0.0.1:7700", Some(pin.as_str())),
            ("https://127.0.0.1:7700/inbox", Some(pin.as_str())),
            ("https://192.0.2.1:7700", Some(pin.as_str())),
            ("https://127.0.0.1:7700", Some(&pin[..43])),
        ] {
            assert!(
                matches!(
                    Service::from_url(Some(url), pin),
                    Err(crate::failure::Failure::Usage(_))
                ),
                "{url} {pin:?}"
            );
        }
    }

    #[test]
    fn the_url_names_a_loopback_address_only() {
        assert!(Service::from_url(Some("http://[::1]:7700"), None).is_ok());
        assert!(Service::from_url(Some("http://localhost:7700"), None).is_ok());
        for outside in [
            "http://192.0.2.1:7700",
            "http://0.0.0.0:7700",
            "http://[2001:db8::1]:7700",
        ] {
            match Service::from_url(Some(outside), None) {
                Err(crate::failure::Failure::Usage(detail)) => {
                    assert!(detail.contains("not a loopback address"), "{detail}");
                }
                other => panic!("{outside}: {other:?}"),
            }
        }
    }
}
