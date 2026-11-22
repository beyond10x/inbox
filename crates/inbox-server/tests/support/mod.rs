//! Fixtures shared by the integration tests: three principals with keys created at run time, an
//! operator key that signs their key entries, a clock the test moves, and a small HTTP client.
//! No key material is written into this file.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use inbox::messaging::{
    Address, Body, DedupKey, KeyEntry, KeyFile, Kind, MessageId, PostMessage, Principal, Signature,
    ThreadId,
};
use inbox::primitives::Timestamp;
use inbox::server::json::{self, Value};
use inbox::server::wire;
use inbox_core::MemoryBackend;
use inbox_server::{Config, ManualClock, Running, Server, format_instant, request_headers};
use inbox_verify::{Instant, KeyRing, SecretKey, sign_envelope, sign_key_entry};

pub const AGENT: &str = "agent:host-a/session-one";
pub const OTHER: &str = "agent:host-b/session-two";
pub const THIRD: &str = "agent:host-c/session-three";
pub const NOW: &str = "2026-11-21T12:00:00Z";
pub const LATER: &str = "2026-12-01T00:00:00Z";
pub const KEYS_VALID_UNTIL: &str = "2027-06-01T00:00:00Z";

/// A ULID-shaped id: 26 characters of the Crockford alphabet, distinct per `n`.
pub fn ulid(n: u32) -> String {
    format!("01JD{n:022}")
}

/// One principal's key entry id and private key.
pub struct Member {
    pub principal: &'static str,
    pub key_id: String,
    pub key: SecretKey,
}

/// Everything a test needs: the members, the ring the service reads, a clock and a store.
pub struct World {
    pub members: Vec<Member>,
    pub ring: KeyRing,
    pub clock: Arc<ManualClock>,
    pub backend: MemoryBackend,
}

fn generate() -> SecretKey {
    SecretKey::generate().expect("the OS random source answers")
}

impl World {
    pub fn new() -> Self {
        let operator = generate();
        let members: Vec<Member> = [AGENT, OTHER, THIRD]
            .into_iter()
            .enumerate()
            .map(|(at, principal)| Member {
                principal,
                key_id: format!("key-{at}"),
                key: generate(),
            })
            .collect();
        let keys = members
            .iter()
            .map(|member| {
                let mut entry = KeyEntry {
                    key_id: member.key_id.clone(),
                    principal: Principal(member.principal.to_owned()),
                    public_key: member.key.public_key().to_base64(),
                    valid_until: Timestamp(KEYS_VALID_UNTIL.to_owned()),
                    operator_signature: String::new(),
                };
                sign_key_entry(&mut entry, &operator);
                entry
            })
            .collect();
        let file = KeyFile {
            keys,
            delegations: Vec::new(),
        };
        let mut text = String::new();
        wire::encode_inbox_messaging_key_file(&file, &mut text);
        let ring = KeyRing::parse(&text, &operator.public_key()).expect("the key file reads");
        Self {
            members,
            ring,
            clock: Arc::new(ManualClock::new(instant(NOW))),
            backend: MemoryBackend::new(),
        }
    }

    pub fn member(&self, principal: &str) -> &Member {
        self.members
            .iter()
            .find(|member| member.principal == principal)
            .expect("a known principal")
    }

    /// The configuration of a service on an ephemeral loopback port over this world's ring and
    /// clock, with short waits, a 2 s bound on a request's arrival, and no sweep in the background
    /// unless a test asks for one.
    pub fn config(&self) -> Config {
        let mut config = Config::new(
            "127.0.0.1:0".parse().expect("a loopback address"),
            self.ring.clone(),
        );
        config.clock = self.clock.clone();
        config.max_wait = Duration::from_secs(5);
        config.sweep_interval = Duration::from_secs(3600);
        config.request_timeout = Duration::from_secs(2);
        config
    }

    pub fn start(&self, tweak: impl FnOnce(&mut Config)) -> Running<MemoryBackend> {
        let mut config = self.config();
        tweak(&mut config);
        Server::bind(config, self.backend.clone())
            .expect("the loopback listener binds")
            .spawn()
    }

    /// A post from `from` to `to`, signed with `from`'s key.
    pub fn post(&self, from: &str, to: &str, n: u32, body: &str) -> PostMessage {
        let mut post = PostMessage {
            message_id: MessageId(ulid(n)),
            from: Principal(from.to_owned()),
            on_behalf_of: Principal(from.to_owned()),
            authority: None,
            to: Address(to.to_owned()),
            kind: Kind::Note,
            thread: ThreadId(ulid(n)),
            reply_shape: None,
            decision: None,
            default_on_silence: None,
            created: Timestamp(NOW.to_owned()),
            expires: Timestamp(LATER.to_owned()),
            dedup_key: DedupKey(ulid(n)),
            body: Body(body.to_owned()),
            signature: Signature {
                key_id: String::new(),
                value: String::new(),
            },
        };
        self.sign(&mut post);
        post
    }

    pub fn sign(&self, post: &mut PostMessage) {
        let member = self.member(&post.from.0);
        sign_envelope(post, &member.key_id, &member.key).expect("the envelope canonicalizes");
    }

    /// The request timestamp a caller would send now.
    pub fn timestamp(&self) -> String {
        format_instant(inbox_server::Clock::now(&*self.clock))
    }

    /// Sends a request signed by `principal` at the clock's reading.
    pub fn send(
        &self,
        address: SocketAddr,
        principal: &str,
        method: &str,
        target: &str,
        body: &[u8],
    ) -> Reply {
        self.send_at(address, principal, method, target, body, &self.timestamp())
    }

    /// Sends a request signed by `principal` with the timestamp `timestamp`.
    pub fn send_at(
        &self,
        address: SocketAddr,
        principal: &str,
        method: &str,
        target: &str,
        body: &[u8],
        timestamp: &str,
    ) -> Reply {
        let member = self.member(principal);
        let headers = request_headers(method, target, body, &member.key_id, &member.key, timestamp);
        let headers: Vec<(&str, String)> = headers.into_iter().collect();
        send_raw(address, method, target, &headers, body)
    }

    /// [`World::send`], answering the head of the answer beside it.
    pub fn send_answer(
        &self,
        address: SocketAddr,
        principal: &str,
        method: &str,
        target: &str,
        body: &[u8],
    ) -> (String, Reply) {
        let member = self.member(principal);
        let headers = request_headers(
            method,
            target,
            body,
            &member.key_id,
            &member.key,
            &self.timestamp(),
        );
        let headers: Vec<(&str, String)> = headers.into_iter().collect();
        send_raw_answer(address, method, target, &headers, body)
    }

    /// Opens `principal`'s own inbox and asserts it was opened.
    pub fn open(&self, address: SocketAddr, principal: &str) {
        let body = format!(r#"{{"address":"{principal}","adapter":"terminal"}}"#);
        let reply = self.send(
            address,
            principal,
            "POST",
            "/messaging/commands/open-inbox",
            body.as_bytes(),
        );
        assert_eq!(reply.status, 202, "{}", reply.body);
    }
}

pub fn instant(text: &str) -> Instant {
    Instant::parse(text).expect("a fixed instant reads")
}

/// A post as its sender writes it on the wire.
pub fn encoded(post: &PostMessage) -> Vec<u8> {
    let mut out = String::new();
    wire::encode_command_inbox_messaging_post_message(post, &mut out);
    out.into_bytes()
}

/// One answer as the client read it.
#[derive(Debug)]
pub struct Reply {
    pub status: u16,
    pub body: String,
}

impl Reply {
    pub fn json(&self) -> Value {
        json::parse(&self.body).unwrap_or_else(|error| panic!("{error}: {}", self.body))
    }

    pub fn text(&self, member: &str) -> String {
        match self.json().member(member) {
            Some(Value::Text(text)) => text.clone(),
            other => panic!("`{member}` is {other:?} in {}", self.body),
        }
    }

    /// The rows of a view answer.
    pub fn rows(&self) -> Vec<Value> {
        match self.json().member("rows") {
            Some(Value::Array(rows)) => rows.clone(),
            other => panic!("`rows` is {other:?} in {}", self.body),
        }
    }

    /// The `message_id` of every row of a view answer.
    pub fn ids(&self) -> Vec<String> {
        self.rows()
            .iter()
            .map(|row| match row.member("message_id") {
                Some(Value::Text(id)) => id.clone(),
                other => panic!("`message_id` is {other:?}"),
            })
            .collect()
    }
}

/// Sends one request with exactly these extra headers and reads the whole answer.
pub fn send_raw(
    address: SocketAddr,
    method: &str,
    target: &str,
    headers: &[(&str, String)],
    body: &[u8],
) -> Reply {
    send_raw_answer(address, method, target, headers, body).1
}

/// [`send_raw`], answering the head of the answer beside it.
pub fn send_raw_answer(
    address: SocketAddr,
    method: &str,
    target: &str,
    headers: &[(&str, String)],
    body: &[u8],
) -> (String, Reply) {
    let mut stream = TcpStream::connect_timeout(&address, ANSWER_WAIT)
        .unwrap_or_else(|error| panic!("the service accepted no connection: {error}"));
    stream
        .set_read_timeout(Some(ANSWER_WAIT))
        .expect("a read timeout");
    stream
        .set_write_timeout(Some(ANSWER_WAIT))
        .expect("a write timeout");
    let mut head = format!(
        "{method} {target} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).expect("the head is sent");
    stream.write_all(body).expect("the body is sent");
    read_answer(stream)
}

pub fn read_reply(stream: TcpStream) -> Reply {
    read_answer(stream).1
}

/// The values of the header `name` in the head of an answer, the name compared without regard to
/// case, in the order they came.
pub fn header(head: &str, name: &str) -> Vec<String> {
    head.split("\r\n")
        .skip(1)
        .filter_map(|line| line.split_once(':'))
        .filter(|(held, _)| held.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim().to_owned())
        .collect()
}

/// How long the client of these tests waits for a connection, for a write and for each read of
/// an answer: a service that sends nothing fails the case within seconds, with a message.
pub const ANSWER_WAIT: Duration = Duration::from_secs(20);

/// Reads the whole answer: its head, and the answer as [`read_reply`] reads it. A stream with no
/// read timeout of its own gets [`ANSWER_WAIT`].
pub fn read_answer(mut stream: TcpStream) -> (String, Reply) {
    if stream.read_timeout().ok().flatten().is_none() {
        stream
            .set_read_timeout(Some(ANSWER_WAIT))
            .expect("a read timeout");
    }
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap_or_else(|error| {
        panic!(
            "the service sent no whole answer within {} s of a read: {error}",
            ANSWER_WAIT.as_secs()
        )
    });
    let answer = String::from_utf8(answer).expect("the answer is UTF-8");
    let (head, body) = answer.split_once("\r\n\r\n").expect("a head and a body");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|status| status.parse().ok())
        .unwrap_or_else(|| panic!("a status line in {head}"));
    (
        head.to_owned(),
        Reply {
            status,
            body: body.to_owned(),
        },
    )
}
