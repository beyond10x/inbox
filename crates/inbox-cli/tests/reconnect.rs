//! The cases of story:the-client-reconnects-after-a-restart: a client command whose answer names
//! an `Inbox-Epoch` other than the one the client recorded opens the caller's own inbox again
//! with the adapter it last opened it with (when it ran `inbox open` and no `inbox close` since)
//! and posts its kept envelopes again, after the command's own request, with no typed `inbox
//! open` or `inbox repost`. Every key and certificate is made at run time with the binary itself.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use inbox::server::json::{self, Value};

use support::{BOB, LOCAL, REFUSED, Run, Service, TRANSPORT, World, ids, keys_only, run, text};

/// The file beside the outbox file at `outbox` whose name ends in `suffix`.
fn beside(outbox: &Path, suffix: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// The lines of a file, as they are; none when it does not exist.
fn lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The entries of the epochs file beside `outbox`: each line split at its tab.
fn epochs(outbox: &Path) -> Vec<(String, String)> {
    lines(&beside(outbox, ".epochs"))
        .iter()
        .map(|line| {
            let (id, epoch) = line.split_once('\t').expect("a tab in an epochs line");
            (id.to_owned(), epoch.to_owned())
        })
        .collect()
}

/// The records of the file of the opened inbox beside `outbox`: one JSON object per line.
fn opened(outbox: &Path) -> Vec<Value> {
    lines(&beside(outbox, ".opened"))
        .iter()
        .map(|line| json::parse(line).expect("a line of the opened file is JSON"))
        .collect()
}

fn is_epoch(text: &str) -> bool {
    text.len() == 32
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The `reconnected` member of a `--json` document; `None` when it has none.
fn reconnected(document: &Value) -> Option<&Value> {
    document.member("reconnected")
}

/// The `reposted` entries of the repost step of a reconnect.
fn reposted(reconnect: &Value) -> Vec<Value> {
    match reconnect
        .member("repost")
        .and_then(|repost| repost.member("reposted"))
    {
        Some(Value::Array(entries)) => entries.clone(),
        other => panic!("the repost step lists {other:?} in {reconnect:?}"),
    }
}

/// The document without its `reconnected` member: what the command printed of its own answer.
fn own_members(document: &Value) -> Value {
    match document {
        Value::Object(members) => Value::Object(
            members
                .iter()
                .filter(|(name, _)| name != "reconnected")
                .cloned()
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The lines a run wrote to standard error.
fn stderr_lines(run: &Run) -> Vec<&str> {
    run.stderr.lines().collect()
}

/// Posts a request from `from` to `to` and answers its message id.
fn post_request(world: &World, from: &str, to: &str, body: &str) -> String {
    world
        .client(
            from,
            &[
                "post",
                "--to",
                to,
                "--kind",
                "request",
                "--reply-shape",
                "free",
                "--body",
                body,
            ],
        )
        .ok()
        .stdout
        .trim()
        .to_owned()
}

/// The acceptance of the story: after a restart of the service the owner's first command opens
/// its inbox again with the adapter it last opened it with, and the sender's next command puts
/// its kept unanswered request into the owner's `Pending`, once, also when a second command of
/// the sender follows; neither `inbox open` nor `inbox repost` is typed after the restart.
#[test]
fn after_a_restart_the_next_commands_open_the_inbox_again_and_deliver_the_kept_request_once() {
    let mut world = World::new("reconnect-acceptance");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let message = post_request(&world, "alice", BOB, "Answer me after a restart");
    let before = epochs(&world.outbox("alice"));
    assert_eq!(before.len(), 1, "{before:?}");

    world.restart();

    // The owner's first command: its own answer is the new start's, empty; then the reconnect
    // opens the inbox again.
    let owner = world.client("bob", &["pending", "--json"]).ok();
    let document = owner.json();
    assert!(ids(&document).is_empty(), "{owner:#?}");
    let reconnect = reconnected(&document).expect("the owner's command reconnected");
    let open = reconnect.member("open").expect("an open step");
    assert_eq!(text(open, "outcome"), "opened", "{owner:#?}");
    assert!(reconnect.member("repost").is_none(), "{owner:#?}");
    assert_eq!(stderr_lines(&owner).len(), 1, "{owner:#?}");
    let record = opened(&world.outbox("bob"));
    assert_eq!(record.len(), 1, "{record:?}");
    assert_eq!(text(&record[0], "inbox"), BOB);
    assert_eq!(text(&record[0], "adapter"), "slack");
    let epoch = text(reconnect, "epoch");
    assert_eq!(
        text(&record[0], "epoch"),
        epoch,
        "the new start is recorded"
    );

    // The sender's next command reposts the kept request; the one after it posts nothing.
    let sender = world.client("alice", &["outbox", "--json"]).ok();
    let document = sender.json();
    let reconnect = reconnected(&document).expect("the sender's command reconnected");
    assert!(reconnect.member("open").is_none(), "{sender:#?}");
    let entries = reposted(reconnect);
    assert_eq!(entries.len(), 1, "{sender:#?}");
    assert_eq!(text(&entries[0], "message_id"), message);
    assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(true)));
    assert_eq!(stderr_lines(&sender).len(), 1, "{sender:#?}");
    let after = epochs(&world.outbox("alice"));
    assert_eq!(after, [(message.clone(), epoch.clone())], "{sender:#?}");

    let again = world.client("alice", &["outbox", "--json"]).ok();
    assert!(reconnected(&again.json()).is_none(), "{again:#?}");
    assert_eq!(again.stderr, "", "{again:#?}");

    let pending = world.client("bob", &["pending", "--json"]).ok();
    assert_eq!(
        ids(&pending.json()),
        std::slice::from_ref(&message),
        "the request is in Pending once: {pending:#?}"
    );
    assert_eq!(pending.stderr, "", "no second reconnect: {pending:#?}");
}

/// One request a [`Recorder`] read: its request line and its body.
#[derive(Debug, Clone)]
struct Request {
    line: String,
    body: String,
}

/// Reads one request in full from `stream`: the head, then as many bytes as `Content-Length`
/// says, or what arrives until the stream ends or 10 s pass without a byte.
fn read_request(stream: &mut TcpStream) -> Request {
    let _ = stream.set_read_timeout(Some(support::STAND_IN_WAIT));
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected = None;
    let mut head_end = None;
    loop {
        if expected.is_some_and(|total| received.len() >= total) {
            break;
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => received.extend_from_slice(&buffer[..read]),
        }
        if head_end.is_none() {
            head_end = received.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(end) = head_end {
                let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                expected = Some(end + 4 + length);
            }
        }
    }
    let text = String::from_utf8_lossy(&received).into_owned();
    let line = text.lines().next().unwrap_or_default().to_owned();
    let body = head_end.map_or_else(String::new, |end| text[end + 4..].to_owned());
    Request { line, body }
}

/// How a [`Recorder`] answers the `n`th request (from 0) whose request line is given: the status,
/// the `Inbox-Epoch` and the body, or `None` to close the connection without an answer.
type Answering = dyn Fn(usize, &str) -> Option<(u16, String, &'static str)> + Send;

/// A stand-in for the service on 127.0.0.1:0 that answers every request, one per connection, as
/// `answering` says, and keeps each request until it is stopped. It ends when it is stopped, or
/// when no connection comes for [`support::STAND_IN_WAIT`], which only ends a hang.
struct Recorder {
    url: String,
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<Vec<Request>>,
}

impl Recorder {
    fn start(answering: Box<Answering>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            let mut idle = std::time::Instant::now();
            while !stopped.load(Ordering::SeqCst) && idle.elapsed() < support::STAND_IN_WAIT {
                let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(50))
                else {
                    continue;
                };
                idle = std::time::Instant::now();
                let request = read_request(&mut stream);
                let answer = answering(requests.len(), &request.line);
                requests.push(request);
                if let Some((status, epoch, body)) = answer {
                    let text = format!(
                        "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nInbox-Epoch: {epoch}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = stream.write_all(text.as_bytes());
                }
            }
            requests
        });
        Self { url, stop, thread }
    }

    /// Stops it, after the client it served has ended, and answers every request it read.
    fn requests(self) -> Vec<Request> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread.join().expect("the stand-in ends")
    }
}

/// An epoch of a start no real service had: 32 hex digits made from `n`.
fn epoch_of(n: usize) -> String {
    format!("{:032x}", 0x5eed_0000_usize + n)
}

/// The answer a service gives each request, by its request line: a view's empty rows, `opened`
/// for an open, `posted` for a post; each answer of another start than the one before it.
fn every_answer_a_new_start(n: usize, line: &str) -> Option<(u16, String, &'static str)> {
    let body = if line.starts_with("GET ") {
        r#"{"rows":[]}"#
    } else if line.contains("/open-inbox ") {
        r#"{"outcome":"opened","published":[]}"#
    } else {
        r#"{"outcome":"posted","published":[]}"#
    };
    let status = if line.contains("/post ") { 202 } else { 200 };
    Some((status, epoch_of(n), body))
}

/// The owner has run `inbox open` twice, with `terminal` and then `slack`, and keeps a note it
/// posted to its own inbox. One command against a service whose every answer names a new start:
/// its own request, then one open with `slack` and one post of the kept note, and nothing else,
/// although the answers of the open and of the post name starts the client did not record
/// either. Each answer's epoch is recorded where it belongs.
#[test]
fn the_reconnect_opens_with_the_last_adapter_and_starts_no_reconnect_of_its_own() {
    let world = World::new("reconnect-count");
    world.client("bob", &["open"]).ok();
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let note = world
        .client("bob", &["post", "--to", BOB, "--body", "to myself"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let outbox = world.outbox("bob");
    let kept = lines(&outbox);
    assert_eq!(kept.len(), 1);

    let recorder = Recorder::start(Box::new(every_answer_a_new_start));
    let done = world.client_env(
        "bob",
        &["pending", "--json"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    let done = done.ok();
    let seen: Vec<&str> = requests
        .iter()
        .map(|request| request.line.as_str())
        .collect();
    assert_eq!(requests.len(), 3, "{seen:?}");
    assert!(
        seen[0].starts_with("GET /messaging/views/pending?"),
        "{seen:?}"
    );
    assert!(
        seen[1].starts_with("POST /messaging/commands/open-inbox "),
        "{seen:?}"
    );
    assert!(
        seen[2].starts_with("POST /messaging/commands/post "),
        "{seen:?}"
    );
    let open = json::parse(&requests[1].body).expect("the open's body is JSON");
    assert_eq!(text(&open, "address"), BOB);
    assert_eq!(
        text(&open, "adapter"),
        "slack",
        "the adapter last opened with"
    );
    assert_eq!(requests[2].body, kept[0], "the kept envelope, unchanged");

    let document = done.json();
    assert!(ids(&document).is_empty(), "{done:#?}");
    let reconnect = reconnected(&document).expect("reconnected");
    assert_eq!(text(reconnect, "epoch"), epoch_of(0));
    assert_eq!(reposted(reconnect).len(), 1, "{done:#?}");
    assert_eq!(stderr_lines(&done).len(), 1, "{done:#?}");
    assert_eq!(text(&opened(&outbox)[0], "epoch"), epoch_of(1));
    assert_eq!(epochs(&outbox), [(note, epoch_of(2))]);
}

/// A client that never ran `inbox open` on this host opens nothing: it only posts its kept
/// envelope again.
#[test]
fn a_client_that_never_opened_its_inbox_only_reposts() {
    let world = World::new("reconnect-never-opened");
    world.client("bob", &["open"]).ok();
    let message = post_request(&world, "alice", BOB, "kept");
    let recorder = Recorder::start(Box::new(every_answer_a_new_start));
    let done = world.client_env(
        "alice",
        &["outbox", "--json"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    let done = done.ok();
    let seen: Vec<&str> = requests
        .iter()
        .map(|request| request.line.as_str())
        .collect();
    assert_eq!(requests.len(), 2, "{seen:?}");
    assert!(
        seen[0].starts_with("GET /messaging/views/outbox?"),
        "{seen:?}"
    );
    assert!(
        seen[1].starts_with("POST /messaging/commands/post "),
        "{seen:?}"
    );
    let reconnect = reconnected(&done.json()).cloned().expect("reconnected");
    assert!(reconnect.member("open").is_none(), "{done:#?}");
    assert_eq!(text(&reposted(&reconnect)[0], "message_id"), message);
    assert!(opened(&world.outbox("alice")).is_empty());
}

/// `inbox close` answered done removes the record of the opened inbox, and so does a close the
/// new start answers `UnknownInbox`: a later command opens nothing, and a post to the inbox is
/// refused `UnknownInbox`.
#[test]
fn a_closed_inbox_is_not_opened_again() {
    let mut world = World::new("reconnect-closed");
    world.client("bob", &["open"]).ok();
    assert_eq!(opened(&world.outbox("bob")).len(), 1);
    world.client("bob", &["close"]).ok();
    assert!(opened(&world.outbox("bob")).is_empty(), "close removed it");
    world.restart();
    let later = world.client("bob", &["pending", "--json"]).ok();
    assert!(reconnected(&later.json()).is_none(), "{later:#?}");
    assert_eq!(later.stderr, "", "{later:#?}");
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "closed"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert!(refused.stderr.contains("UnknownInbox"), "{refused:#?}");

    // Opened, then closed after a restart: the new start holds no such inbox.
    world.client("bob", &["open"]).ok();
    world.restart();
    let closed = world.client("bob", &["close", "--json"]);
    assert_eq!(closed.status, REFUSED, "{closed:#?}");
    assert!(reconnected(&closed.json()).is_none(), "{closed:#?}");
    assert!(opened(&world.outbox("bob")).is_empty(), "{closed:#?}");
    let later = world.client("bob", &["pending"]).ok();
    assert_eq!(later.stderr, "", "{later:#?}");
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "closed again"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
}

/// The repost of a reconnect refused `UnknownInbox` (the owner has not opened again yet) leaves
/// the typed command's standard output and exit status as they are, says so in one line on
/// standard error, keeps the envelope, records no epoch for it, and the next command tries
/// again: once the owner opened, it delivers the request, once.
#[test]
fn a_refused_repost_changes_nothing_of_the_command_and_the_next_command_tries_again() {
    let mut world = World::new("reconnect-refused");
    world.client("bob", &["open"]).ok();
    let message = post_request(&world, "alice", BOB, "wait for bob");
    let outbox = world.outbox("alice");
    let kept = lines(&outbox);
    let recorded = epochs(&outbox);
    world.restart();

    let refused = world.client("alice", &["outbox"]);
    assert_eq!(refused.status, 0, "{refused:#?}");
    assert_eq!(refused.stdout.trim(), "no messages", "{refused:#?}");
    let said = stderr_lines(&refused);
    assert_eq!(said.len(), 1, "{refused:#?}");
    assert!(said[0].contains("UnknownInbox"), "{refused:#?}");
    assert_eq!(lines(&outbox), kept, "the envelope stays kept");
    assert_eq!(epochs(&outbox), recorded, "no epoch is recorded for it");

    let json = world.client("alice", &["outbox", "--json"]);
    assert_eq!(json.status, 0, "{json:#?}");
    let document = json.json();
    let own = own_members(&document);
    assert!(ids(&own).is_empty(), "{json:#?}");
    assert_eq!(
        own.member("unreadable"),
        Some(&Value::Array(Vec::new())),
        "{json:#?}"
    );
    let reconnect = reconnected(&document).expect("the next command tried again");
    assert_eq!(
        text(reconnect.member("repost").expect("a repost step"), "error"),
        "inbox.messaging.UnknownInbox",
        "{json:#?}"
    );
    assert_eq!(epochs(&outbox), recorded);

    world.client("bob", &["pending"]).ok();
    let delivered = world.client("alice", &["outbox", "--json"]).ok();
    let reconnect = reconnected(&delivered.json())
        .cloned()
        .expect("reconnected");
    assert_eq!(
        reposted(&reconnect)[0].member("arrived"),
        Some(&Value::Bool(true))
    );
    world.client("alice", &["outbox"]).ok();
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), [message]);
}

/// The service gone again after the command's own answer: the open of the reconnect fails in
/// transport, the command exits 0 with its own output, one line on standard error says so, the
/// record keeps the start it had, no post is tried, and the next command tries again.
#[test]
fn a_service_gone_again_fails_the_reconnect_and_the_next_command_tries_again() {
    let world = World::new("reconnect-gone");
    world.client("bob", &["open"]).ok();
    world
        .client("bob", &["post", "--to", BOB, "--body", "to myself"])
        .ok();
    let outbox = world.outbox("bob");
    let record = lines(&beside(&outbox, ".opened"));
    let recorded = epochs(&outbox);

    let recorder = Recorder::start(Box::new(|n, line| {
        (n == 0)
            .then(|| every_answer_a_new_start(n, line))
            .flatten()
    }));
    let gone = world.client_env("bob", &["pending"], &[("INBOX_URL", recorder.url.as_str())]);
    let requests = recorder.requests();
    assert_eq!(gone.status, 0, "{gone:#?}");
    assert_eq!(gone.stdout.trim(), "no messages", "{gone:#?}");
    let said = stderr_lines(&gone);
    assert_eq!(said.len(), 1, "{gone:#?}");
    assert!(said[0].contains("transport"), "{gone:#?}");
    assert_eq!(requests.len(), 2, "the command and the open: {requests:?}");
    assert_eq!(lines(&beside(&outbox, ".opened")), record);
    assert_eq!(epochs(&outbox), recorded);

    let recorder = Recorder::start(Box::new(every_answer_a_new_start));
    world
        .client_env("bob", &["pending"], &[("INBOX_URL", recorder.url.as_str())])
        .ok();
    let requests = recorder.requests();
    assert_eq!(
        requests.len(),
        3,
        "the next command tried again: {requests:?}"
    );
}

/// A TCP proxy on 127.0.0.1:0 that carries the bytes of its first connection to `first` and of
/// every later one to `later`, both ways, and counts its connections.
struct Proxy {
    url: String,
    connections: Arc<AtomicUsize>,
}

impl Proxy {
    fn start(first: String, later: String) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the proxy binds");
        let url = format!("https://{}", listener.local_addr().expect("an address"));
        let connections = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&connections);
        std::thread::spawn(move || {
            while let Some(client) = support::accept_within(&listener, support::STAND_IN_WAIT) {
                let n = counted.fetch_add(1, Ordering::SeqCst);
                let target = if n == 0 { &first } else { &later };
                let Ok(service) = TcpStream::connect(target) else {
                    continue;
                };
                let (mut from_client, mut to_service) = (
                    client.try_clone().expect("a clone"),
                    service.try_clone().expect("a clone"),
                );
                let upstream = std::thread::spawn(move || {
                    let mut buffer = [0_u8; 4096];
                    while let Ok(read @ 1..) = from_client.read(&mut buffer) {
                        if to_service.write_all(&buffer[..read]).is_err() {
                            break;
                        }
                    }
                    let _ = to_service.shutdown(std::net::Shutdown::Write);
                });
                let (mut from_service, mut to_client) = (service, client);
                let mut buffer = [0_u8; 4096];
                while let Ok(read @ 1..) = from_service.read(&mut buffer) {
                    if to_client.write_all(&buffer[..read]).is_err() {
                        break;
                    }
                }
                let _ = to_client.shutdown(std::net::Shutdown::Both);
                let _ = upstream.join();
            }
        });
        Self { url, connections }
    }
}

/// `inbox serve --tls-file <file>` over `keys.json` in `dir`.
fn serve_tls(dir: &Path, operator: &str, tls_file: &str) -> Service {
    Service::start(
        dir,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            operator,
            "--tls-file",
            tls_file,
        ],
        &[],
    )
}

/// The client command of `who` at `url` with the pin `pin`, its outbox in `dir`.
fn tls_client(dir: &Path, url: &str, pin: &str, who: &str, args: &[&str]) -> Run {
    let key = format!("{who}.key");
    let outbox = dir.join(format!("{who}.outbox"));
    let outbox = outbox.to_str().expect("a UTF-8 path");
    run(
        dir,
        args,
        &[
            ("INBOX_URL", url),
            ("INBOX_TLS_PIN", pin),
            ("INBOX_SECRET_KEY_FILE", key.as_str()),
            ("INBOX_OUTBOX", outbox),
        ],
    )
}

/// The service comes back with another certificate after the command's own answer: the open of
/// the reconnect ends at the pin check, so that service handles no request of it; the command
/// exits 0 with its own output, one line on standard error names the failure, and the record
/// keeps the start it had.
#[test]
fn a_service_back_with_another_certificate_gets_no_byte_of_a_reconnect() {
    let (dir, operator) = keys_only("reconnect-tls");
    let pin = run(&dir.path, &["tls", "new", "--out", "tls.json"], &[])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let other = run(&dir.path, &["tls", "new", "--out", "other.json"], &[])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let first = serve_tls(&dir.path, &operator, "tls.json");
    tls_client(&dir.path, &first.url, &pin, "bob", &["open"]).ok();
    let outbox = dir.join("bob.outbox");
    let record = lines(&beside(&outbox, ".opened"));
    assert_eq!(record.len(), 1);
    drop(first);

    let again = serve_tls(&dir.path, &operator, "tls.json");
    let changed = serve_tls(&dir.path, &operator, "other.json");
    let address = |service: &Service| {
        service
            .url
            .strip_prefix("https://")
            .expect("an https URL")
            .to_owned()
    };
    let proxy = Proxy::start(address(&again), address(&changed));
    let done = tls_client(&dir.path, &proxy.url, &pin, "bob", &["pending"]);
    assert_eq!(done.status, 0, "{done:#?}");
    assert_eq!(done.stdout.trim(), "no messages", "{done:#?}");
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(said[0].contains("pin"), "{done:#?}");
    assert_eq!(lines(&beside(&outbox, ".opened")), record);
    assert_eq!(
        proxy.connections.load(Ordering::SeqCst),
        2,
        "the command's own request and the open of the reconnect"
    );

    // That service holds no inbox of bob: the open never reached it.
    let refused = tls_client(
        &dir.path,
        &changed.url,
        &other,
        "alice",
        &["post", "--to", BOB, "--body", "is bob there?"],
    );
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert!(refused.stderr.contains("UnknownInbox"), "{refused:#?}");
    let unreachable = tls_client(&dir.path, &changed.url, &pin, "bob", &["pending"]);
    assert_eq!(unreachable.status, TRANSPORT, "{unreachable:#?}");
}

/// Two commands of one client that meet the new start at once, three times for the owner and
/// three for the sender: no file beside the outbox is torn, the owner's record names the new
/// start once, and the request is in `Pending` once: the service answers a post of a message id
/// it holds `duplicate`.
#[test]
fn commands_that_meet_the_new_start_at_once_break_no_file_and_deliver_the_request_once() {
    let mut world = World::new("reconnect-at-once");
    world.client("bob", &["open"]).ok();
    let message = post_request(&world, "alice", BOB, "once");
    let kept = lines(&world.outbox("alice"));
    world.restart();

    let owners: Vec<_> = (0..3).map(|_| world.spawn("bob", &["pending"])).collect();
    for owner in owners {
        owner.finish().ok();
    }
    let record = opened(&world.outbox("bob"));
    assert_eq!(record.len(), 1, "{record:?}");
    assert_eq!(text(&record[0], "inbox"), BOB);
    assert!(is_epoch(&text(&record[0], "epoch")), "{record:?}");

    let senders: Vec<_> = (0..3).map(|_| world.spawn("alice", &["outbox"])).collect();
    for sender in senders {
        sender.finish().ok();
    }
    let outbox = world.outbox("alice");
    assert_eq!(lines(&outbox), kept, "the outbox file is whole");
    let recorded = epochs(&outbox);
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0].0, message);
    assert_eq!(recorded[0].1, text(&record[0], "epoch"), "the new start");
    for (who, name) in [("bob", ".opened.new"), ("alice", ".epochs.new")] {
        let staged = beside(&world.outbox(who), name);
        assert!(!staged.exists(), "{} is left", staged.display());
    }
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), [message]);
}

/// The answer of a post refused `UnknownInbox`, as the generated route writes it.
const UNKNOWN_INBOX: &str =
    r#"{"outcome":"unknown-inbox","published":[],"error":"inbox.messaging.UnknownInbox"}"#;

/// The answer of a post accepted.
const POSTED: &str = r#"{"outcome":"posted","published":[]}"#;

/// A typed `inbox post` that a new start refuses `UnknownInbox` keeps its envelope, and the
/// reconnect of the same command posts the other kept envelope and not the typed one: two
/// requests in all.
#[test]
fn the_reconnect_of_a_typed_post_does_not_post_its_own_envelope_again() {
    let world = World::new("reconnect-own-post");
    world.client("bob", &["open"]).ok();
    post_request(&world, "alice", BOB, "kept from before");
    let outbox = world.outbox("alice");
    let before = lines(&outbox);
    assert_eq!(before.len(), 1);

    let recorder = Recorder::start(Box::new(|n, _| {
        Some(if n == 0 {
            (404, epoch_of(n), UNKNOWN_INBOX)
        } else {
            (202, epoch_of(n), POSTED)
        })
    }));
    let refused = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "too early"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let kept = lines(&outbox);
    assert_eq!(kept.len(), 2, "the typed envelope is kept: {refused:#?}");
    assert_eq!(kept[0], before[0]);
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(requests.len(), 2, "{seen:?}");
    assert_eq!(requests[0].body, kept[1], "the typed post");
    assert_eq!(requests[1].body, kept[0], "the kept envelope, once");
    let said = stderr_lines(&refused);
    assert!(
        said.last()
            .is_some_and(|line| line.contains("reposted 1 kept envelope, 1 arrived")),
        "{refused:#?}"
    );
}

/// `inbox close` that fails in transport, or that the service refuses for another reason than
/// `UnknownInbox`, keeps the record of the opened inbox as it was, so a later command after a
/// restart opens the inbox again.
#[test]
fn a_close_that_failed_in_transport_or_was_refused_keeps_the_record() {
    let mut world = World::new("reconnect-close-failed");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let outbox = world.outbox("bob");
    let record = lines(&beside(&outbox, ".opened"));
    assert_eq!(record.len(), 1);

    let closed_port = support::ClosedPort::new();
    let down = closed_port.url();
    let gone = world.client_env("bob", &["close"], &[("INBOX_URL", down.as_str())]);
    assert_eq!(gone.status, TRANSPORT, "{gone:#?}");
    assert_eq!(lines(&beside(&outbox, ".opened")), record, "{gone:#?}");

    let recorder = Recorder::start(Box::new(|n, _| {
        Some((401, epoch_of(n), r#"{"refused":"unauthenticated"}"#))
    }));
    let refused = world.client_env("bob", &["close"], &[("INBOX_URL", recorder.url.as_str())]);
    let requests = recorder.requests();
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert!(
        requests[0]
            .line
            .starts_with("POST /messaging/commands/close-inbox "),
        "{requests:?}"
    );
    assert_eq!(lines(&beside(&outbox, ".opened")), record, "{refused:#?}");

    world.restart();
    let later = world.client("bob", &["pending", "--json"]).ok();
    let reconnect = reconnected(&later.json()).cloned().expect("reconnected");
    let open = reconnect.member("open").expect("an open step");
    assert_eq!(text(open, "outcome"), "opened", "{later:#?}");
}

/// A record of the opened inbox behind a symbolic link to `/dev/zero`, a device that never ends,
/// and then the outbox file behind one: a command after a restart ends at once with its own
/// output, opens nothing, and one line on standard error names the file.
#[test]
fn a_link_to_a_device_beside_the_outbox_holds_nothing_and_blocks_no_command() {
    let mut world = World::new("reconnect-device");
    world.client("bob", &["open"]).ok();
    let outbox = world.outbox("bob");
    let record = beside(&outbox, ".opened");
    fs::remove_file(&record).expect("the record is removed");
    std::os::unix::fs::symlink("/dev/zero", &record).expect("a link");
    world.restart();

    let done = world.client("bob", &["pending", "--json"]).ok();
    assert!(reconnected(&done.json()).is_none(), "{done:#?}");
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(said[0].contains(".opened"), "{done:#?}");
    assert!(said[0].contains("no regular file"), "{done:#?}");

    fs::remove_file(&record).expect("the link is removed");
    std::os::unix::fs::symlink("/dev/zero", &outbox).expect("a link");
    let done = world.client("bob", &["pending"]).ok();
    assert_eq!(done.stdout.trim(), "no messages", "{done:#?}");
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(said[0].contains("bob.outbox:"), "{done:#?}");
    assert!(said[0].contains("no regular file"), "{done:#?}");
}

/// A record of the opened inbox in a file over its bound (1 MiB): read as no record, so a
/// command after a restart opens nothing, and one line on standard error names the file.
#[test]
fn a_file_of_the_opened_inbox_over_its_bound_holds_nothing() {
    let mut world = World::new("reconnect-over-bound");
    world.client("bob", &["open"]).ok();
    let record = beside(&world.outbox("bob"), ".opened");
    let mut text = fs::read_to_string(&record).expect("the record");
    text.push_str(&"\n".repeat(1024 * 1024 + 1 - text.len()));
    fs::write(&record, text).expect("the record grows");
    world.restart();

    let done = world.client("bob", &["pending", "--json"]).ok();
    assert!(reconnected(&done.json()).is_none(), "{done:#?}");
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(said[0].contains(".opened"), "{done:#?}");
    assert!(said[0].contains("more than 1048576 bytes"), "{done:#?}");
}

/// The processes that hold, and those that wait for, a `flock` of the file with the inode
/// `inode`, read from `/proc/locks`.
fn flocks(inode: u64) -> (Vec<u32>, Vec<u32>) {
    let (mut holders, mut waiters) = (Vec::new(), Vec::new());
    for line in fs::read_to_string("/proc/locks")
        .unwrap_or_default()
        .lines()
    {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (waiting, rest) = match fields.get(1) {
            Some(&"->") => (true, &fields[2..]),
            _ => (false, fields.get(1..).unwrap_or_default()),
        };
        if rest.first() != Some(&"FLOCK") {
            continue;
        }
        let pid = rest.get(3).and_then(|pid| pid.parse().ok());
        let node = rest
            .get(4)
            .and_then(|field| field.rsplit(':').next())
            .and_then(|node| node.parse::<u64>().ok());
        if let (Some(pid), Some(node)) = (pid, node)
            && node == inode
        {
            if waiting {
                waiters.push(pid);
            } else {
                holders.push(pid);
            }
        }
    }
    (holders, waiters)
}

/// Waits until `fact` holds. [`support::STAND_IN_WAIT`] only ends a hang.
fn wait_for(what: &str, mut fact: impl FnMut() -> bool) {
    let started = std::time::Instant::now();
    while !fact() {
        assert!(
            started.elapsed() < support::STAND_IN_WAIT,
            "{what}: not within {} s",
            support::STAND_IN_WAIT.as_secs()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// `inbox open` writes the record of the opened inbox only while it holds `<outbox>.lock`: with
/// the lock held by this test it waits for it (as `/proc/locks` says) and the file stays
/// unwritten until the lock is released. A FIFO at the place of the staged file is left as it is
/// while the open waits for the lock, and is removed, not opened, once the open holds it. The
/// case no longer holds that the lock is held until the staged file is renamed over the record.
#[test]
fn the_record_of_the_opened_inbox_is_written_under_the_lock() {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    use std::process::Stdio;

    let world = World::new("reconnect-lock");
    let outbox = world.outbox("bob");
    let lock = beside(&outbox, ".lock");
    let held = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)
        .expect("the lock file");
    held.lock().expect("the lock");
    let inode = fs::metadata(&lock).expect("the lock file").ino();
    let child = world
        .client_command("bob", &["open"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let pid = child.id();
    wait_for("the open waits for the lock", || {
        flocks(inode).1.contains(&pid)
    });
    assert!(
        !beside(&outbox, ".opened").exists(),
        "the record was written while this test held the lock"
    );
    held.unlock().expect("the lock is released");
    let output = child.wait_with_output().expect("the open ends");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(opened(&outbox).len(), 1);

    // A FIFO without a reader at the place of the staged file: the open leaves it alone while it
    // waits for the lock, then removes it under the lock and writes its staged file anew; it is
    // not opened, so the open is not held up by it (story:an-unreadable-epochs-file-keeps-its-
    // other-lines, round 3: `replace` follows nothing at `<file>.new`).
    let staged = beside(&outbox, ".opened.new");
    let made = std::process::Command::new("mkfifo")
        .arg(&staged)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "{made:?}");
    held.lock().expect("the lock again");
    let child = world
        .client_command("bob", &["open", "--adapter", "slack"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary starts");
    let pid = child.id();
    wait_for("the second open waits for the lock", || {
        flocks(inode).1.contains(&pid)
    });
    let kept = fs::symlink_metadata(&staged).map(|meta| meta.file_type().is_fifo());
    held.unlock().expect("the lock is released");
    let output = child.wait_with_output().expect("the open ends");
    assert!(
        matches!(kept, Ok(true)),
        "the staged file's place changed before the open held the lock: {kept:?}"
    );
    assert!(output.status.success(), "{output:?}");
    assert!(
        fs::symlink_metadata(&staged).is_err(),
        "the FIFO at the staged file's place is left"
    );
    assert_eq!(opened(&outbox).len(), 1);
}

/// A typed `inbox repost` at a new start before the owner opened again: each kept envelope is
/// posted once (refused `UnknownInbox`), and the command's reconnect posts nothing more, so its
/// document has no member `reconnected`.
#[test]
fn a_typed_repost_at_a_new_start_posts_each_kept_envelope_once() {
    let world = World::new("reconnect-typed-repost");
    world.client("bob", &["open"]).ok();
    post_request(&world, "alice", BOB, "one");
    post_request(&world, "alice", BOB, "two");
    let outbox = world.outbox("alice");
    let kept = lines(&outbox);
    assert_eq!(kept.len(), 2);

    let recorder = Recorder::start(Box::new(|n, line| {
        Some(if line.starts_with("GET ") {
            (200, epoch_of(n), r#"{"rows":[]}"#)
        } else {
            (404, epoch_of(n), UNKNOWN_INBOX)
        })
    }));
    let done = world.client_env(
        "alice",
        &["repost", "--json"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(done.status, REFUSED, "{done:#?}");
    assert_eq!(requests.len(), 3, "the epoch and one post each: {seen:?}");
    assert!(
        seen[0].starts_with("GET /messaging/views/outbox?"),
        "{seen:?}"
    );
    assert_eq!(requests[1].body, kept[0]);
    assert_eq!(requests[2].body, kept[1]);
    let document = done.json();
    assert!(reconnected(&document).is_none(), "{done:#?}");
    assert_eq!(lines(&outbox), kept);
}

/// A marked envelope (its post failed in transport) that the reconnect's repost gets refused
/// `413`, a refusal of the service's own, loses its mark and gets no epoch, as after a typed
/// post: the next command posts nothing (round 3, F2).
#[test]
fn a_marked_envelope_refused_413_by_a_reconnect_is_not_posted_again() {
    let world = World::new("reconnect-413");
    world.client("bob", &["open"]).ok();
    let closed_port = support::ClosedPort::new();
    let down = closed_port.url();
    let failed = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "while the service was down"],
        &[("INBOX_URL", down.as_str())],
    );
    assert_eq!(failed.status, TRANSPORT, "{failed:#?}");
    let outbox = world.outbox("alice");
    let kept = lines(&outbox);
    assert_eq!(epochs(&outbox).len(), 1);
    assert_eq!(epochs(&outbox)[0].1, "none", "marked unsent");

    let recorder = Recorder::start(Box::new(|n, line| {
        Some(if line.starts_with("GET ") {
            (200, epoch_of(n), r#"{"rows":[]}"#)
        } else {
            (
                413,
                epoch_of(n),
                r#"{"refused":"body too large","limit":1024}"#,
            )
        })
    }));
    let url = recorder.url.clone();
    let first = world.client_env("alice", &["outbox"], &[("INBOX_URL", url.as_str())]);
    let second = world.client_env("alice", &["outbox"], &[("INBOX_URL", url.as_str())]);
    let requests = recorder.requests();
    let first = first.ok();
    let second = second.ok();
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(
        requests.len(),
        3,
        "the view, the repost, the view: {seen:?}"
    );
    assert!(
        seen[1].starts_with("POST /messaging/commands/post "),
        "{seen:?}"
    );
    assert!(stderr_lines(&first)[0].contains("refused"), "{first:#?}");
    assert_eq!(second.stderr, "", "{second:#?}");
    assert!(epochs(&outbox).is_empty(), "no mark, no epoch");
    assert_eq!(lines(&outbox), kept, "the envelope stays kept");
}

/// The reconnect's open reaches the service after a `close` of the owner took the record (here
/// the stand-in removes it as the open arrives): the reconnect sends `CloseInbox`, writes no
/// record, says on standard error that the inbox was closed meanwhile, and its `open` member is
/// `closed-meanwhile` with both answers (round 3, F4).
#[test]
fn a_reconnects_open_whose_record_a_close_took_closes_the_inbox_again() {
    let world = World::new("reconnect-closed-meanwhile");
    world.client("bob", &["open"]).ok();
    let record = beside(&world.outbox("bob"), ".opened");
    let taken = record.clone();
    let recorder = Recorder::start(Box::new(move |n, line| {
        let body = if line.starts_with("GET ") {
            r#"{"rows":[]}"#
        } else if line.contains("/open-inbox ") {
            let _ = fs::remove_file(&taken);
            r#"{"outcome":"opened","published":[]}"#
        } else {
            r#"{"outcome":"closed","published":[]}"#
        };
        Some((200, epoch_of(n), body))
    }));
    let done = world.client_env(
        "bob",
        &["pending", "--json"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    let done = done.ok();
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(requests.len(), 3, "{seen:?}");
    assert!(
        seen[1].starts_with("POST /messaging/commands/open-inbox "),
        "{seen:?}"
    );
    assert!(
        seen[2].starts_with("POST /messaging/commands/close-inbox "),
        "{seen:?}"
    );
    let document = done.json();
    let open = reconnected(&document)
        .and_then(|reconnect| reconnect.member("open"))
        .expect("an open step");
    assert_eq!(text(open, "outcome"), "closed-meanwhile", "{done:#?}");
    assert_eq!(
        text(
            open.member("closed").expect("the close's answer"),
            "outcome"
        ),
        "closed"
    );
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(said[0].contains("closed meanwhile"), "{done:#?}");
    assert!(!record.exists(), "no record is written");
}

/// The record bob's `inbox open --adapter terminal` wrote, with the adapter `slack`: what a
/// typed `inbox open --adapter slack` of bob at another start would write.
fn record_of_slack(world: &World) -> String {
    let record = lines(&beside(&world.outbox("bob"), ".opened")).join("\n");
    let slack = record.replace(r#""adapter":"terminal""#, r#""adapter":"slack""#);
    assert_ne!(
        slack, record,
        "the record names the adapter terminal: {record}"
    );
    slack
}

/// The reconnect's open finds another record than the one it started from (here the stand-in
/// writes bob's record of `slack` as the open arrives, as a typed close and open of bob would):
/// the record stays, no `CloseInbox` follows, and the step says the inbox was opened by another
/// command meanwhile (round 4, F2).
#[test]
fn a_reconnects_open_that_finds_another_record_leaves_it() {
    let world = World::new("reconnect-opened-meanwhile");
    world.client("bob", &["open", "--adapter", "terminal"]).ok();
    let record = beside(&world.outbox("bob"), ".opened");
    let slack = record_of_slack(&world);
    let written = record.clone();
    let recorder = Recorder::start(Box::new(move |n, line| {
        let body = if line.starts_with("GET ") {
            r#"{"rows":[]}"#
        } else {
            fs::write(&written, format!("{slack}\n")).expect("the record of slack");
            r#"{"outcome":"already-open","published":[]}"#
        };
        Some((200, epoch_of(n), body))
    }));
    let done = world.client_env(
        "bob",
        &["pending", "--json"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    let done = done.ok();
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(requests.len(), 2, "the view and the open: {seen:?}");
    let document = done.json();
    let open = reconnected(&document)
        .and_then(|reconnect| reconnect.member("open"))
        .expect("an open step");
    assert_eq!(text(open, "outcome"), "opened-meanwhile", "{done:#?}");
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(
        said[0].contains("opened by another command meanwhile"),
        "{done:#?}"
    );
    let records = opened(&world.outbox("bob"));
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(text(&records[0], "adapter"), "slack");
}

/// After the reconnect's `CloseInbox` of an inbox closed meanwhile, a record is there again (here
/// the stand-in writes it as the close arrives, as a typed open of bob would): the reconnect
/// opens the inbox once more with that record's adapter and leaves the record as it is (round 4,
/// F1).
#[test]
fn a_reconnects_close_after_closed_meanwhile_is_followed_by_an_open_when_a_record_came_back() {
    let world = World::new("reconnect-closed-then-opened");
    world.client("bob", &["open", "--adapter", "terminal"]).ok();
    let record = beside(&world.outbox("bob"), ".opened");
    let slack = record_of_slack(&world);
    let written = record.clone();
    let recorder = Recorder::start(Box::new(move |n, line| {
        let body = if line.starts_with("GET ") {
            r#"{"rows":[]}"#
        } else if line.contains("/close-inbox ") {
            fs::write(&written, format!("{slack}\n")).expect("the record of slack");
            r#"{"outcome":"closed","published":[]}"#
        } else {
            if n == 1 {
                let _ = fs::remove_file(&written);
            }
            r#"{"outcome":"opened","published":[]}"#
        };
        Some((200, epoch_of(n), body))
    }));
    let done = world.client_env(
        "bob",
        &["pending", "--json"],
        &[("INBOX_URL", recorder.url.as_str())],
    );
    let requests = recorder.requests();
    let done = done.ok();
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(
        requests.len(),
        4,
        "the view, the open, the close, the open: {seen:?}"
    );
    assert!(
        seen[3].starts_with("POST /messaging/commands/open-inbox "),
        "{seen:?}"
    );
    assert!(requests[3].body.contains("slack"), "{:?}", requests[3].body);
    let document = done.json();
    let open = reconnected(&document)
        .and_then(|reconnect| reconnect.member("open"))
        .expect("an open step");
    assert_eq!(text(open, "outcome"), "closed-meanwhile", "{done:#?}");
    assert_eq!(
        text(
            open.member("reopened").expect("the second open's answer"),
            "outcome"
        ),
        "opened"
    );
    let said = stderr_lines(&done);
    assert_eq!(said.len(), 1, "{done:#?}");
    assert!(said[0].contains("closed meanwhile"), "{done:#?}");
    let records = opened(&world.outbox("bob"));
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(text(&records[0], "adapter"), "slack");
    assert_ne!(
        text(&records[0], "epoch"),
        epoch_of(3),
        "the record is left as it is"
    );
}

/// A marked envelope that the reconnect's repost gets answered `503` (the service stopping)
/// keeps its mark, as after a typed post: the next command posts it again (round 4, F3).
#[test]
fn a_marked_envelope_answered_503_by_a_reconnect_is_posted_again_by_the_next_command() {
    let world = World::new("reconnect-503");
    world.client("bob", &["open"]).ok();
    let closed_port = support::ClosedPort::new();
    let down = closed_port.url();
    let failed = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "while the service was down"],
        &[("INBOX_URL", down.as_str())],
    );
    assert_eq!(failed.status, TRANSPORT, "{failed:#?}");
    let outbox = world.outbox("alice");
    let recorder = Recorder::start(Box::new(|n, line| {
        Some(if line.starts_with("GET ") {
            (200, epoch_of(n), r#"{"rows":[]}"#)
        } else {
            (503, epoch_of(n), r#"{"refused":"stopping"}"#)
        })
    }));
    let url = recorder.url.clone();
    let first = world.client_env("alice", &["outbox"], &[("INBOX_URL", url.as_str())]);
    let second = world.client_env("alice", &["outbox"], &[("INBOX_URL", url.as_str())]);
    let requests = recorder.requests();
    first.ok();
    second.ok();
    let seen: Vec<&str> = requests.iter().map(|r| r.line.as_str()).collect();
    assert_eq!(
        requests.len(),
        4,
        "the view and the repost, twice: {seen:?}"
    );
    assert!(
        seen[3].starts_with("POST /messaging/commands/post "),
        "{seen:?}"
    );
    assert_eq!(epochs(&outbox).len(), 1);
    assert_eq!(epochs(&outbox)[0].1, "none", "still marked unsent");
}

/// A post whose answer names no epoch (a service without the header `Inbox-Epoch`) arrived: its
/// mark goes and no epoch is recorded (round 4, F4).
#[test]
fn a_post_answered_without_an_epoch_loses_its_mark() {
    let world = World::new("reconnect-no-epoch");
    let (url, answered) = support::stand_in_answers(vec![(202, None, POSTED)]);
    let done = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "to a service without epochs"],
        &[("INBOX_URL", url.as_str())],
    );
    support::wait_until_finished(&answered, "the stand-in without epochs");
    let _ = answered.join();
    done.ok();
    let outbox = world.outbox("alice");
    assert_eq!(lines(&outbox).len(), 1, "the envelope is kept");
    assert!(epochs(&outbox).is_empty(), "no mark, no epoch");
}

/// The epochs file is replaced before the outbox file: when the outbox file cannot be written
/// (a directory where its staged file goes), `post` exits 4 before its request and leaves a mark
/// of an envelope the outbox file does not keep; nothing posts it, and the next write drops it
/// (round 4, F5).
#[test]
fn a_mark_whose_envelope_was_not_kept_is_dropped_by_the_next_write() {
    let world = World::new("reconnect-mark-first");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "first"])
        .ok();
    let outbox = world.outbox("alice");
    let kept = lines(&outbox);
    let staged = beside(&outbox, ".new");
    fs::create_dir(&staged).expect("a directory where the staged outbox file goes");
    let failed = world.client("alice", &["post", "--to", BOB, "--body", "second"]);
    assert_eq!(failed.status, LOCAL, "{failed:#?}");
    assert_eq!(lines(&outbox), kept, "the second envelope is not kept");
    let marks: Vec<String> = epochs(&outbox)
        .into_iter()
        .filter(|(_, epoch)| epoch == "none")
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        marks.len(),
        1,
        "the mark was written first: {:?}",
        epochs(&outbox)
    );
    fs::remove_dir(&staged).expect("the directory removed");

    world
        .client("alice", &["post", "--to", BOB, "--body", "third"])
        .ok();
    assert_eq!(lines(&outbox).len(), 2);
    let entries = epochs(&outbox);
    assert_eq!(entries.len(), 2, "{entries:?}");
    assert!(
        entries
            .iter()
            .all(|(id, epoch)| id != &marks[0] && is_epoch(epoch)),
        "{entries:?}"
    );
}
