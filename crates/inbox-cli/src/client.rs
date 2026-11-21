//! The client side of the wire: the service's address from `INBOX_URL`, one signed request per
//! connection over a `TcpStream` (HTTP/1.1, no TLS, no keep-alive, as the service answers), and
//! what its answer means.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use inbox::server::inbox_service::ROUTES;
use inbox::server::json::{self, Value};
use inbox_server::{Clock, SystemClock, format_instant, request_headers};

use crate::failure::Failure;
use crate::keys::Caller;

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

/// The service a client talks to: `INBOX_URL`, `http://<host>:<port>`.
#[derive(Debug, Clone)]
pub struct Service {
    address: SocketAddr,
    host: String,
}

impl Service {
    /// Reads `http://<host>:<port>` with an optional trailing `/`.
    ///
    /// # Errors
    ///
    /// [`Failure::Usage`] when no URL is given, it is not of that form, or its host does not
    /// resolve.
    pub fn from_url(url: Option<&str>) -> Result<Self, Failure> {
        let url = url.ok_or_else(|| {
            Failure::Usage("the service: pass --url <url> or set INBOX_URL".to_owned())
        })?;
        let refused = || {
            Failure::Usage(format!(
                "INBOX_URL `{url}` is not http://<host>:<port> (no TLS, no path in this stage)"
            ))
        };
        let host = url.strip_prefix("http://").ok_or_else(refused)?;
        let host = host.strip_suffix('/').unwrap_or(host);
        if host.is_empty() || host.contains(['/', '?', '#', '@']) {
            return Err(refused());
        }
        let address = host
            .to_socket_addrs()
            .map_err(|_| refused())?
            .next()
            .ok_or_else(refused)?;
        Ok(Self {
            address,
            host: host.to_owned(),
        })
    }

    /// Sends one request signed by `caller` at the clock reading and reads the whole answer.
    ///
    /// # Errors
    ///
    /// [`Failure::Transport`] when the service cannot be reached or the answer is not HTTP.
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
        let transport = |error: std::io::Error| {
            Failure::Transport(format!("{} ({}): {error}", self.host, self.address))
        };
        let mut stream =
            TcpStream::connect_timeout(&self.address, CONNECT_TIMEOUT).map_err(transport)?;
        stream
            .set_read_timeout(Some(wait.saturating_add(ANSWER_TIMEOUT)))
            .map_err(transport)?;
        stream
            .set_write_timeout(Some(ANSWER_TIMEOUT))
            .map_err(transport)?;
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
            .map_err(|why| Failure::Transport(format!("{} ({}): {why}", self.host, self.address)))
    }
}

/// One answer of the service.
#[derive(Debug)]
pub struct Reply {
    /// The HTTP status.
    pub status: u16,
    /// The body, read as JSON.
    pub body: Value,
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
        Ok(Self { status, body })
    }

    /// The answer, or the refusal it is: a body that carries `outcome` and `error` is a declared
    /// refusal of the model, named by its `error`; any other status from 400 on is the
    /// service's own, named by its `refused` text.
    ///
    /// # Errors
    ///
    /// [`Failure::Refused`] for either refusal.
    pub fn accepted(self) -> Result<Value, Failure> {
        if let Some(name) = declared_error(&self.body) {
            return Err(Failure::Refused {
                name: name.to_owned(),
                status: self.status,
                answer: self.body,
            });
        }
        if self.status >= 400 {
            let name = match self.body.member("refused") {
                Some(Value::Text(text)) => text.clone(),
                _ => format!("status {}", self.status),
            };
            return Err(Failure::Refused {
                name,
                status: self.status,
                answer: self.body,
            });
        }
        Ok(self.body)
    }
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
    use super::{Service, query_value, route};

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
        assert!(Service::from_url(Some("http://127.0.0.1:7700")).is_ok());
        assert!(Service::from_url(Some("http://127.0.0.1:7700/")).is_ok());
        for refused in [
            "https://127.0.0.1:7700",
            "127.0.0.1:7700",
            "http://127.0.0.1:7700/inbox",
            "http://",
        ] {
            assert!(Service::from_url(Some(refused)).is_err(), "{refused}");
        }
        assert!(Service::from_url(None).is_err());
    }
}
