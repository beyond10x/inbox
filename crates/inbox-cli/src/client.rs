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
use inbox_server::{Clock, EPOCH_HEADER, SystemClock, format_instant, request_headers_for};

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
    /// [`Failure::Usage`] when no URL is given, it is not of that form, its host or port holds a
    /// control character ([`host_holds_no_control`], before anything is resolved), its host does
    /// not resolve, or it resolves to an address that is not loopback; when the pin is not one, an
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
        // Before the resolver is handed anything: a control character in the host or its port.
        host_holds_no_control(host)?;
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
    /// request is written. With `on_behalf_of` (`--for`) the request carries the header
    /// `Inbox-On-Behalf-Of` with that address and signs it as the sixth member of the signed
    /// request object (`docs/design/signing.md`, "Request"); without it the request is the one
    /// of before the header, byte for byte.
    ///
    /// # Errors
    ///
    /// [`Failure::Transport`] when the service cannot be reached or the answer is not HTTP;
    /// [`Failure::Certificate`] when its certificate does not match the pin or it speaks no TLS;
    /// [`Failure::Usage`] when a part of the head holds a line break ([`request_head`]), before a
    /// connection is opened.
    pub fn send(
        &self,
        caller: &Caller,
        on_behalf_of: Option<&str>,
        method: &str,
        target: &str,
        body: &[u8],
        wait: Duration,
    ) -> Result<Reply, Failure> {
        let timestamp = format_instant(SystemClock.now());
        let headers = request_headers_for(
            method,
            target,
            body,
            &caller.key_id,
            &caller.key,
            &timestamp,
            on_behalf_of,
        );
        let head = request_head(method, target, &self.host, body.len(), &headers)?;
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
                // `InvalidData`: the handshake was refused here, the certificate not being the
                // pinned one or the peer speaking no TLS; a handshake the peer cut stays a
                // transport failure.
                Box::new(tls::connect(&config, socket).map_err(|error| {
                    if error.kind() == std::io::ErrorKind::InvalidData {
                        Failure::Certificate(format!("{} ({peer}): {error}", self.host))
                    } else {
                        transport(error)
                    }
                })?)
            }
        };
        stream.write_all(head.as_bytes()).map_err(transport)?;
        stream.write_all(body).map_err(transport)?;
        stream.flush().map_err(transport)?;
        let mut answer = Vec::new();
        stream.read_to_end(&mut answer).map_err(transport)?;
        Reply::read(&answer).map_err(|(status, why)| {
            let detail = format!("{} ({peer}): {why}", self.host);
            match status {
                Some(status) => Failure::Unexpected { status, detail },
                None => Failure::Transport(detail),
            }
        })
    }
}

/// `host`, the text of `INBOX_URL` after its scheme (the host, its port and a trailing `/`), is
/// fine when it holds no control character (`char::is_control`); one that holds one is wrong
/// usage, shown as [`output::quoted_cut`] shows a value: the host is the value of the header
/// `Host`, and the resolver is never handed such a name. It reads the text alone and resolves
/// nothing.
fn host_holds_no_control(host: &str) -> Result<(), Failure> {
    if host.contains(char::is_control) {
        return Err(Failure::Usage(format!(
            "INBOX_URL: its host {} holds a control character, which no request can carry; no \
             request is sent",
            output::quoted_cut(host)
        )));
    }
    Ok(())
}

/// The head of a request, up to and with its empty line: the request line, `Host`, the fixed
/// headers and `headers` in their order. A CR or LF in any part (`method`, `target`, `host`, a
/// name or a value of `headers`) would end a line on the wire where the signed text goes on, so
/// it is refused as [`Failure::Usage`] and no head is made, whatever the caller passes.
fn request_head(
    method: &str,
    target: &str,
    host: &str,
    length: usize,
    headers: &[(&str, String)],
) -> Result<String, Failure> {
    let parts = [
        ("the method", method),
        ("the target", target),
        ("the host", host),
    ];
    let named = headers
        .iter()
        .flat_map(|(name, value)| [("a header name", *name), ("a header value", value.as_str())]);
    if let Some((part, _)) = parts
        .into_iter()
        .chain(named)
        .find(|(_, text)| text.contains(['\r', '\n']))
    {
        return Err(Failure::Usage(format!(
            "{part} holds a line break; no request is sent"
        )));
    }
    let mut head = format!(
        "{method} {target} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
         Content-Length: {length}\r\nConnection: close\r\n"
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    Ok(head)
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
    /// The answer read from the bytes a connection carried until its end. A body that is not JSON
    /// is an answer cut before its end ([`Failure::Transport`]: the error carries no status) unless
    /// the answer is whole: it holds every byte its `Content-Length` names, or it has neither a
    /// `Content-Length` nor a chunked `Transfer-Encoding` and so ends with its connection. Then it
    /// is whole and not JSON ([`Failure::Unexpected`]: the error carries its status). A head that
    /// is not whole is cut, and so is a chunked answer whose body is not JSON.
    fn read(answer: &[u8]) -> Result<Self, (Option<u16>, String)> {
        let cut = |why: &str| (None, why.to_owned());
        let split = answer
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .ok_or_else(|| cut("the answer has no end of its head"))?;
        let head =
            std::str::from_utf8(&answer[..split]).map_err(|_| cut("the head is not UTF-8"))?;
        let body = &answer[split + 4..];
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|status| status.parse().ok())
            .ok_or_else(|| cut("the answer has no status line"))?;
        let whole = match content_length(head) {
            Some(length) => body.len() >= length,
            // Without a length the body ends with the connection (RFC 9112, 6.3), which this
            // client asks to close: what it read until the end is the whole answer. A chunked
            // body is not read as one: it stays cut, as before.
            None => !chunked(head),
        };
        let whole = |why: String| (whole.then_some(status), why);
        let body = std::str::from_utf8(body)
            .map_err(|_| whole(format!("the answer with the status {status} is not UTF-8")))?;
        let body = json::parse(body).map_err(|error| {
            whole(format!(
                "the answer with the status {status} is not JSON: {error}"
            ))
        })?;
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
    /// [`Failure::Refused`] for either refusal; [`Failure::Unexpected`] naming the status for any
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
        Err(Failure::Unexpected {
            status: self.status,
            detail: format!(
                "the answer has the status {}, which is neither a success nor a refusal of the \
                 service: {}",
                self.status,
                output::document(&self.body)
            ),
        })
    }
}

/// The value of the `Content-Length` header of an answer's head, when it has one that reads.
fn content_length(head: &str) -> Option<usize> {
    head.split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.trim().parse().ok())
}

/// Whether the head of an answer names `chunked` in a `Transfer-Encoding` header.
fn chunked(head: &str) -> bool {
    head.split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("transfer-encoding"))
        .any(|(_, value)| {
            value
                .split(',')
                .any(|coding| coding.trim().eq_ignore_ascii_case("chunked"))
        })
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
    use super::{Epoch, Reply, Service, host_holds_no_control, query_value, request_head, route};
    use crate::failure::Failure;

    /// A host with TAB, ESC, DEL, NEL (U+0085), CR or LF, anywhere in the host or its port, is
    /// wrong usage: one line that names `INBOX_URL` and shows the host escaped, no raw control
    /// character; a long one is cut to its first 64 characters. A host with U+2028 (no control
    /// character) and a plain one pass. The function reads the text only: nothing is resolved.
    #[test]
    fn a_host_with_a_control_character_is_wrong_usage() {
        for host in [
            "in\tbox:7700",
            "in\u{1b}box:7700",
            "in\u{7f}box:7700",
            "in\u{85}box:7700",
            "in\rbox:7700",
            "in\nbox:7700",
            "inbox:77\t00",
            "inbox:7700/\u{1b}",
        ] {
            match host_holds_no_control(host) {
                Err(Failure::Usage(line)) => {
                    assert!(line.starts_with("INBOX_URL"), "{host:?}: {line:?}");
                    assert!(line.contains(&format!("{host:?}")), "{host:?}: {line:?}");
                    assert!(!line.contains(char::is_control), "{host:?}: {line:?}");
                }
                other => panic!("{host:?}: {other:?}"),
            }
        }
        let long = format!("{}\t:7700", "h".repeat(1 << 20));
        match host_holds_no_control(&long) {
            Err(Failure::Usage(line)) => {
                assert!(line.len() < 1024, "{} bytes", line.len());
                assert!(
                    line.contains(&format!(
                        "{:?} (and 1048518 more characters)",
                        "h".repeat(64)
                    )),
                    "{line:?}"
                );
            }
            other => panic!("long: {other:?}"),
        }
        for host in [
            "in\u{2028}box:7700",
            "localhost:7700",
            "127.0.0.1:7700",
            "[::1]:7700",
        ] {
            assert!(host_holds_no_control(host).is_ok(), "{host:?}");
        }
    }

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

    /// An answer whose body is not JSON is whole, and keeps its status, when it fills its
    /// `Content-Length` or has none and is not chunked (it ended with its connection); it is cut,
    /// with no status, when it is chunked, falls short of its `Content-Length`, or has no whole
    /// head.
    #[test]
    fn a_body_that_is_not_json_is_whole_or_cut_by_its_head() {
        let page = "<html>Not Found</html>\n";
        let read = |headers: &str| {
            let text = format!("HTTP/1.1 404 Not Found\r\n{headers}\r\n{page}");
            Reply::read(text.as_bytes()).map(|reply| reply.status)
        };
        let whole = |result: Result<u16, (Option<u16>, String)>| result.unwrap_err().0;
        assert_eq!(whole(read("Connection: close\r\n")), Some(404));
        let filled = format!("Content-Length: {}\r\n", page.len());
        assert_eq!(whole(read(&filled)), Some(404));
        let short = format!("Content-Length: {}\r\n", page.len() + 1);
        assert_eq!(whole(read(&short)), None);
        assert_eq!(whole(read("Transfer-Encoding: gzip, Chunked\r\n")), None);
        let no_head = Reply::read(b"HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\n");
        assert!(matches!(no_head, Err((None, _))), "{no_head:?}");
    }

    /// A CR or LF in any part of the head, a value of `Inbox-On-Behalf-Of` among them, is an
    /// error and makes no head; without one the head is the lines as given.
    #[test]
    fn the_head_holds_no_line_break_a_caller_passes() {
        let header = |value: &str| vec![("Inbox-On-Behalf-Of", value.to_owned())];
        let head = request_head("GET", "/p", "127.0.0.1:1", 0, &header("agent:test/bob")).unwrap();
        assert_eq!(
            head,
            "GET /p HTTP/1.1\r\nHost: 127.0.0.1:1\r\nContent-Type: application/json\r\n\
             Content-Length: 0\r\nConnection: close\r\nInbox-On-Behalf-Of: agent:test/bob\r\n\r\n"
        );
        for broken in ["a\r\nX-Injected: 1", "a\nX", "a\rX", "\n"] {
            for (method, target, host, headers) in [
                (broken, "/p", "h", header("a")),
                ("GET", broken, "h", header("a")),
                ("GET", "/p", broken, header("a")),
                ("GET", "/p", "h", header(broken)),
                ("GET", "/p", "h", vec![("X\r\nY", "a".to_owned())]),
            ] {
                let made = request_head(method, target, host, 0, &headers);
                assert!(
                    matches!(made, Err(Failure::Usage(_))),
                    "{broken:?} {method:?} {target:?} {host:?}: {made:?}"
                );
            }
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
        ("POST", "mark-delivered"),
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
