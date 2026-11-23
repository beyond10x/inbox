//! The cases of story:inbox-wake-returns-on-a-post: `inbox wake` holds the `Pending` view of the
//! caller's own inbox open past the end of a wait and past a restart of the service, prints the
//! rows in `Sent` when one arrives, then sends `MarkDelivered` for each, and exits 0; a row in
//! `Delivered` neither ends it nor is printed. One wake per inbox and outbox file runs on a host.
//! No case waits for `max_wait` or for a long pause, and none holds an upper bound of time: a
//! bounded wait only ends a hang. Every key and certificate is made at run time with the binary
//! itself.

use crate::support;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::fs::MetadataExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use inbox::server::json;

use support::{
    ALICE, BOB, Dir, LOCAL, REFUSED, Run, Service, TRANSPORT, USAGE, ids, keys_only, run,
};

/// The exit status of a second `inbox wake` of the same inbox on the same host.
const ANOTHER_WAKE: i32 = 5;

/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

/// Waits until `fact` holds; [`HANG`] only ends a hang.
fn wait_for(what: &str, mut fact: impl FnMut() -> bool) {
    let started = Instant::now();
    while !fact() {
        assert!(
            started.elapsed() < HANG,
            "{what}: not within {} s",
            HANG.as_secs()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A process of the binary this case started; killed and waited for when dropped unfinished.
struct Started(Option<Child>);

impl Started {
    fn spawn(command: &mut Command) -> Self {
        Self(Some(command.spawn().expect("the binary starts")))
    }

    fn child(&mut self) -> &mut Child {
        self.0.as_mut().expect("not finished yet")
    }

    fn pid(&self) -> u32 {
        self.0.as_ref().expect("not finished yet").id()
    }

    fn signal(&self, name: &str) {
        let sent = Command::new("kill")
            .args([name, &self.pid().to_string()])
            .status()
            .expect("kill runs");
        assert!(sent.success(), "kill {name} failed");
    }

    /// Waits for its end ([`HANG`] only ends a hang) and answers its exit status and what it
    /// wrote to the pipes the case gave it.
    fn finish(mut self) -> (ExitStatus, Run) {
        let started = Instant::now();
        while self
            .child()
            .try_wait()
            .expect("the process is asked")
            .is_none()
        {
            assert!(
                started.elapsed() < HANG,
                "inbox wake did not end within {} s",
                HANG.as_secs()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut child = self.0.take().expect("not finished yet");
        let status = child.wait().expect("the process is waited for");
        let read = |pipe: Option<&mut dyn Read>| {
            let mut text = String::new();
            if let Some(pipe) = pipe {
                pipe.read_to_string(&mut text).expect("the pipe is UTF-8");
            }
            text
        };
        let stdout = read(child.stdout.as_mut().map(|pipe| pipe as &mut dyn Read));
        let stderr = read(child.stderr.as_mut().map(|pipe| pipe as &mut dyn Read));
        let code = status.code().unwrap_or(-1);
        (
            status,
            Run {
                status: code,
                stdout,
                stderr,
            },
        )
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `inbox wake <args>` of `who` against `url`, with the outbox file `outbox`, not started; its
/// standard output and standard error are pipes.
fn wake(dir: &Path, url: &str, who: &str, outbox: &Path, args: &[&str]) -> Command {
    let mut command = support::command(dir);
    command
        .arg("wake")
        .args(args)
        .env("INBOX_URL", url)
        .env("INBOX_SECRET_KEY_FILE", format!("{who}.key"))
        .env("INBOX_OUTBOX", outbox)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// A client command of `who` against `url`, its outbox file in `dir`.
fn client(dir: &Path, url: &str, who: &str, args: &[&str]) -> Run {
    let outbox = dir.join(format!("{who}.outbox"));
    run(
        dir,
        args,
        &[
            ("INBOX_URL", url),
            ("INBOX_SECRET_KEY_FILE", &format!("{who}.key")),
            ("INBOX_OUTBOX", outbox.to_str().expect("a UTF-8 path")),
        ],
    )
}

/// A message id made from `n`: 26 characters of the model's alphabet.
fn id(n: u32) -> String {
    format!("01J{n:023}")
}

/// A row of `Pending` for bob, from alice, in `state`, with `body`.
fn row(n: u32, state: &str, body: &str) -> String {
    let id = id(n);
    let mut text = String::new();
    json::push_text(&mut text, body);
    format!(
        r#"{{"message_id":"{id}","from":"{ALICE}","on_behalf_of":"{ALICE}","to":"{BOB}","kind":"note","thread":"{id}","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"{id}","body":{text},"state":"{state}"}}"#
    )
}

/// The answer of `Pending` with `rows`.
fn pending(rows: &[String]) -> String {
    format!(r#"{{"rows":[{}]}}"#, rows.join(","))
}

const DELIVERED: &str = r#"{"outcome":"delivered","published":[]}"#;
const ALREADY_DELIVERED: &str = r#"{"outcome":"already-delivered","published":[]}"#;

/// One request a [`StandIn`] read: its request line, its body, what the file of the wake's
/// standard output held when the request arrived, and when it was read.
#[derive(Debug, Clone)]
struct Request {
    line: String,
    body: String,
    stdout: String,
    at: Instant,
}

impl Request {
    fn is_pending(&self) -> bool {
        self.line.starts_with("GET ") && self.line.contains("/pending?")
    }

    fn is_mark(&self) -> bool {
        self.line.starts_with("POST ") && self.line.contains("/mark-delivered ")
    }
}

/// Reads one request in full from `stream`: the head, then as many bytes as `Content-Length`
/// says, or what arrives until the stream ends or [`HANG`] passes without a byte.
fn read_request(stream: &mut TcpStream) -> (String, String) {
    let _ = stream.set_read_timeout(Some(HANG));
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
    (line, body)
}

/// How a [`StandIn`] answers the `n`th request (from 0): the status and the body, or `None` to
/// close the connection without an answer.
type Answering = dyn Fn(usize, &Request) -> Option<(u16, String)> + Send;

/// A stand-in for the service on 127.0.0.1:0 that answers every request, one per connection, as
/// `answering` says, and keeps each request with what the file `stdout` held when it arrived. It
/// ends when it is stopped, or when no connection comes for [`HANG`], which only ends a hang.
struct StandIn {
    url: String,
    stop: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<Vec<Request>>,
}

impl StandIn {
    fn start(stdout: Option<PathBuf>, answering: Box<Answering>) -> Self {
        Self::at_epoch(stdout, None, answering)
    }

    /// [`StandIn::start`], every answer naming `epoch` in `Inbox-Epoch` when it is given.
    fn at_epoch(stdout: Option<PathBuf>, epoch: Option<String>, answering: Box<Answering>) -> Self {
        let named = epoch.map_or_else(String::new, |epoch| format!("Inbox-Epoch: {epoch}\r\n"));
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut requests = Vec::new();
            let mut idle = Instant::now();
            while !stopped.load(Ordering::SeqCst) && idle.elapsed() < HANG {
                let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                idle = Instant::now();
                let (line, body) = read_request(&mut stream);
                let request = Request {
                    line,
                    body,
                    at: Instant::now(),
                    stdout: stdout
                        .as_ref()
                        .map(|path| fs::read_to_string(path).unwrap_or_default())
                        .unwrap_or_default(),
                };
                let answer = answering(requests.len(), &request);
                requests.push(request);
                if let Some((status, body)) = answer {
                    let text = format!(
                        "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\n{named}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
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

/// A directory with bob's client key, for a case against a stand-in.
fn bob_only(name: &str) -> Dir {
    let dir = Dir::new(name);
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    dir
}

/// The rows are written to standard output, and flushed, before the first `MarkDelivered`: the
/// stand-in reads the file of standard output as each mark arrives. The read asks with
/// `undelivered=1`. Only the rows in `Sent` are printed and each gets a mark; a row in `Delivered`
/// (delivered before, not yet read) is neither printed nor marked, and `already-delivered` is no
/// failure: exit 0, one JSON document.
#[test]
fn the_rows_are_written_before_the_first_mark_and_only_rows_in_sent_are_marked() {
    let dir = bob_only("wake-order");
    let stdout = dir.join("wake.out");
    let answer = pending(&[
        row(1, "Sent", "first"),
        row(2, "Delivered", "second"),
        row(3, "Sent", "third"),
    ]);
    let first = id(1);
    let stand_in = StandIn::start(
        Some(stdout.clone()),
        Box::new(move |_, request| {
            if request.is_pending() {
                Some((200, answer.clone()))
            } else if request.body.contains(&first) {
                Some((200, DELIVERED.to_owned()))
            } else {
                Some((200, ALREADY_DELIVERED.to_owned()))
            }
        }),
    );
    let (status, ran) = Started::spawn(
        wake(
            &dir.path,
            &stand_in.url,
            "bob",
            &dir.join("bob.outbox"),
            &["--json"],
        )
        .stdout(File::create(&stdout).expect("the stdout file is created")),
    )
    .finish();
    let requests = stand_in.requests();
    assert_eq!(status.code(), Some(0), "{ran:#?} {requests:#?}");
    assert_eq!(ran.stderr, "", "{ran:#?}");
    let printed = fs::read_to_string(&stdout).expect("the stdout file is read");
    let document = json::parse(printed.trim()).expect("standard output is one JSON document");
    assert_eq!(ids(&document), [id(1), id(3)], "{printed}");
    assert!(
        !printed.contains("second"),
        "a row in Delivered is not printed: {printed}"
    );
    assert_eq!(requests.len(), 3, "{requests:#?}");
    assert!(requests[0].is_pending(), "{requests:#?}");
    assert!(
        requests[0].line.contains("&undelivered=1 "),
        "{requests:#?}"
    );
    for (request, n) in requests[1..].iter().zip([1, 3]) {
        assert!(request.is_mark(), "{requests:#?}");
        assert!(request.body.contains(&id(n)), "{requests:#?}");
        assert_eq!(
            request.stdout, printed,
            "the rows were on standard output before this mark"
        );
    }
}

/// Without `--json` the wake prints the rows in the form of `inbox pending`, the control
/// characters of a body escaped; a refused `MarkDelivered` loses none of that output, is named in
/// one line on standard error, and the wake exits with the refusal's status, 1.
#[test]
fn the_text_is_that_of_inbox_pending_escaped_and_a_refused_mark_exits_1_after_it() {
    let dir = bob_only("wake-text");
    let answer = pending(&[row(1, "Sent", "ring \u{1b}[2J bell")]);
    let shown = answer.clone();
    let stand_in = StandIn::start(
        None,
        Box::new(move |_, request| {
            if request.is_pending() {
                Some((200, answer.clone()))
            } else {
                Some((
                    404,
                    r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#
                        .to_owned(),
                ))
            }
        }),
    );
    let (_, ran) = Started::spawn(&mut wake(
        &dir.path,
        &stand_in.url,
        "bob",
        &dir.join("bob.outbox"),
        &[],
    ))
    .finish();
    let requests = stand_in.requests();
    assert_eq!(ran.status, REFUSED, "{ran:#?} {requests:#?}");
    assert_eq!(requests.len(), 2, "{requests:#?}");
    assert!(requests[1].is_mark(), "{requests:#?}");
    assert!(ran.stdout.contains("\\u{1b}[2J"), "{ran:#?}");
    assert!(!ran.stdout.contains('\u{1b}'), "{ran:#?}");
    let lines: Vec<&str> = ran.stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{ran:#?}");
    assert!(lines[0].contains(&id(1)), "{ran:#?}");
    assert!(lines[0].contains("UnknownMessage"), "{ran:#?}");

    let same = StandIn::start(None, Box::new(move |_, _| Some((200, shown.clone()))));
    let listed = client(&dir.path, &same.url, "bob", &["pending"]).ok();
    same.requests();
    assert_eq!(ran.stdout, listed.stdout, "the form of inbox pending");
}

/// A service that answers a failure of its own (`503`, `408`) or drops the connection is asked
/// again, and the wake exits 0 on the row the fourth try gets; one line on standard error says
/// that it keeps trying. No case waits for a grown pause: the first ones are short.
#[test]
fn a_service_that_answers_503_408_or_drops_the_connection_is_asked_again() {
    let dir = bob_only("wake-retry");
    let answer = pending(&[row(1, "Sent", "after three failures")]);
    let stand_in = StandIn::start(
        None,
        Box::new(move |n, _| match n {
            0 => Some((503, r#"{"refused":"the service is stopping"}"#.to_owned())),
            1 => Some((408, r#"{"refused":"the request took too long"}"#.to_owned())),
            2 => None,
            3 => Some((200, answer.clone())),
            _ => Some((200, DELIVERED.to_owned())),
        }),
    );
    let (_, ran) = Started::spawn(&mut wake(
        &dir.path,
        &stand_in.url,
        "bob",
        &dir.join("bob.outbox"),
        &["--json"],
    ))
    .finish();
    let requests = stand_in.requests();
    assert_eq!(ran.status, 0, "{ran:#?} {requests:#?}");
    assert_eq!(ids(&ran.json()), [id(1)], "{ran:#?}");
    assert_eq!(requests.len(), 5, "{requests:#?}");
    assert!(
        requests[..4].iter().all(Request::is_pending),
        "{requests:#?}"
    );
    assert!(requests[4].is_mark(), "{requests:#?}");
    let lines: Vec<&str> = ran.stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{ran:#?}");
    assert!(lines[0].contains("trying again"), "{ran:#?}");
}

/// A service that answers every read at once with one row in `Delivered` (one that does not know
/// `undelivered=1`) does not make the wake spin: after each held read that came back at once
/// without a row in `Sent` it pauses as after a failure, the pause growing (100 ms, 200 ms,
/// 400 ms before the next round). It prints nothing and marks nothing; SIGTERM ends it. Only
/// lower bounds of time are held, which a slow host cannot break.
#[test]
fn an_answer_at_once_with_only_a_delivered_row_is_followed_by_a_growing_pause() {
    let dir = bob_only("wake-quick");
    let answer = pending(&[row(1, "Delivered", "read me later")]);
    let seen = Arc::new(AtomicUsize::new(0));
    let count = Arc::clone(&seen);
    let stand_in = StandIn::start(
        None,
        Box::new(move |n, _| {
            count.store(n + 1, Ordering::SeqCst);
            Some((200, answer.clone()))
        }),
    );
    let started = Started::spawn(&mut wake(
        &dir.path,
        &stand_in.url,
        "bob",
        &dir.join("bob.outbox"),
        &[],
    ));
    wait_for("8 reads of Pending", || seen.load(Ordering::SeqCst) >= 8);
    started.signal("-TERM");
    let (status, ran) = started.finish();
    let requests = stand_in.requests();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{ran:#?}");
    assert_eq!(ran.stdout, "", "{ran:#?}");
    assert!(
        requests
            .iter()
            .all(|request| request.is_pending() && request.line.contains("&undelivered=1 ")),
        "{requests:#?}"
    );
    for (round, least) in [(1, 100), (2, 200), (3, 400)] {
        let (held, next) = (&requests[2 * round - 1], &requests[2 * round]);
        assert!(held.line.contains("&wait=60&"), "{requests:#?}");
        assert!(next.line.contains("&wait=0&"), "{requests:#?}");
        let paused = next.at - held.at;
        assert!(
            paused >= Duration::from_millis(least),
            "round {round}: the next read came {paused:?} after the held one"
        );
    }
}

/// `inbox wait` and `inbox pending` do not ask with `undelivered`: a row in `Delivered` ends
/// their wait as before.
#[test]
fn inbox_wait_and_inbox_pending_ask_without_undelivered() {
    let dir = bob_only("wake-wait-plain");
    let answer = pending(&[row(1, "Delivered", "listed")]);
    let stand_in = StandIn::start(None, Box::new(move |_, _| Some((200, answer.clone()))));
    let waited = client(&dir.path, &stand_in.url, "bob", &["wait", "--seconds", "5"]).ok();
    let listed = client(&dir.path, &stand_in.url, "bob", &["pending"]).ok();
    let requests = stand_in.requests();
    assert_eq!(requests.len(), 2, "{requests:#?}");
    assert!(requests[0].line.contains("&wait=5 "), "{requests:#?}");
    assert!(
        requests
            .iter()
            .all(|request| !request.line.contains("undelivered")),
        "{requests:#?}"
    );
    assert_eq!(waited.stdout, listed.stdout);
    assert!(waited.stdout.contains(&id(1)), "{waited:#?}");
}

/// A closed standard output (the harness is gone) ends the wake when it writes the rows: no
/// `MarkDelivered` is sent, and it exits 4, a local file that cannot be written.
#[test]
fn a_closed_standard_output_ends_the_wake_without_a_mark() {
    let dir = bob_only("wake-closed");
    let answer = pending(&[row(1, "Sent", "nobody reads this")]);
    let closed = Arc::new(AtomicBool::new(false));
    let gate = Arc::clone(&closed);
    let stand_in = StandIn::start(
        None,
        Box::new(move |_, request| {
            if request.is_pending() {
                wait_for("the case closed the pipe", || gate.load(Ordering::SeqCst));
                Some((200, answer.clone()))
            } else {
                Some((200, DELIVERED.to_owned()))
            }
        }),
    );
    let mut started = Started::spawn(&mut wake(
        &dir.path,
        &stand_in.url,
        "bob",
        &dir.join("bob.outbox"),
        &[],
    ));
    drop(started.child().stdout.take());
    closed.store(true, Ordering::SeqCst);
    let (_, ran) = started.finish();
    let requests = stand_in.requests();
    assert_eq!(ran.status, LOCAL, "{ran:#?} {requests:#?}");
    assert_eq!(requests.len(), 1, "no mark: {requests:#?}");
    assert_eq!(ran.stderr.lines().count(), 1, "{ran:#?}");
}

/// Wrong usage ends the wake at once with 2, before a lock or a request.
#[test]
fn wrong_usage_ends_the_wake_at_once() {
    let dir = bob_only("wake-usage");
    let (_, ran) = Started::spawn(&mut wake(
        &dir.path,
        "http://192.0.2.1:7700",
        "bob",
        &dir.join("bob.outbox"),
        &[],
    ))
    .finish();
    assert_eq!(ran.status, USAGE, "{ran:#?}");
    assert_eq!(ran.stdout, "", "{ran:#?}");
    assert!(
        lock_files(&dir.path, "bob.outbox.wake").is_empty(),
        "no lock was taken"
    );
}

/// The name of the lock file of the wake of `address` beside the outbox file `bob.outbox`: the
/// 64-bit FNV-1a hash of the address in 16 hex digits, as `docs/design/wake.md` states it.
fn lock_name(address: &str) -> String {
    let hash = address
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    format!("bob.outbox.wake.{hash:016x}")
}

/// The lock file is opened as the files beside the outbox file are: a FIFO in its place (no
/// reader) does not hold the open, and a link to a device is no regular file. Each ends the wake
/// at once with 4, one line on standard error, and no request; the stand-in gets none.
#[test]
fn a_fifo_or_a_link_to_a_device_in_place_of_the_lock_file_ends_the_wake_with_4() {
    let dir = bob_only("wake-lock-file");
    let place = dir.join(&lock_name(BOB));
    for make in ["fifo", "link"] {
        let _ = fs::remove_file(&place);
        if make == "fifo" {
            let made = Command::new("mkfifo")
                .arg(&place)
                .status()
                .expect("mkfifo runs");
            assert!(made.success(), "mkfifo failed");
        } else {
            std::os::unix::fs::symlink("/dev/null", &place).expect("the link is made");
        }
        let stand_in = StandIn::start(None, Box::new(|_, _| Some((200, pending(&[])))));
        let (_, ran) = Started::spawn(&mut wake(
            &dir.path,
            &stand_in.url,
            "bob",
            &dir.join("bob.outbox"),
            &[],
        ))
        .finish();
        let requests = stand_in.requests();
        assert_eq!(ran.status, LOCAL, "{make}: {ran:#?}");
        assert_eq!(ran.stdout, "", "{make}: {ran:#?}");
        let lines: Vec<&str> = ran.stderr.lines().collect();
        assert_eq!(lines.len(), 1, "{make}: {ran:#?}");
        assert!(lines[0].contains(&lock_name(BOB)), "{make}: {ran:#?}");
        assert!(requests.is_empty(), "{make}: {requests:#?}");
    }
}

/// A TCP forwarder on 127.0.0.1:0 to the address `target` names (changed with
/// [`Forwarder::to`]): each connection is carried to a new connection to the target, and the
/// bytes each client sent are kept. A connection whose target cannot be reached is closed.
struct Forwarder {
    address: String,
    target: Arc<Mutex<String>>,
    sent: Arc<Mutex<Vec<Vec<u8>>>>,
    stop: Arc<AtomicBool>,
}

impl Forwarder {
    fn start(target: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the forwarder binds");
        let address = listener.local_addr().expect("an address").to_string();
        let target = Arc::new(Mutex::new(target.to_owned()));
        let sent = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (to, kept, stopped) = (Arc::clone(&target), Arc::clone(&sent), Arc::clone(&stop));
        std::thread::spawn(move || {
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < 3 * HANG {
                let Some(client) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                // The target is taken before the connection is kept: a case that sees it kept
                // knows where it goes.
                let target = to.lock().expect("the target").clone();
                let at = {
                    let mut kept = kept.lock().expect("the record");
                    kept.push(Vec::new());
                    kept.len() - 1
                };
                let kept = Arc::clone(&kept);
                std::thread::spawn(move || carry(client, &target, &kept, at));
            }
        });
        Self {
            address,
            target,
            sent,
            stop,
        }
    }

    fn to(&self, target: &str) {
        *self.target.lock().expect("the target") = target.to_owned();
    }

    /// The bytes each connection's client sent so far.
    fn sent(&self) -> Vec<Vec<u8>> {
        self.sent.lock().expect("the record").clone()
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Carries one connection of a [`Forwarder`] to `target`, keeping what the client sent as
/// connection `at`.
fn carry(client: TcpStream, target: &str, kept: &Arc<Mutex<Vec<Vec<u8>>>>, at: usize) {
    let Ok(upstream) = TcpStream::connect(target) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    let (Ok(mut from_client), Ok(mut to_upstream)) = (client.try_clone(), upstream.try_clone())
    else {
        return;
    };
    let kept = Arc::clone(kept);
    let up = std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match from_client.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    kept.lock().expect("the record")[at].extend_from_slice(&buffer[..read]);
                    if to_upstream.write_all(&buffer[..read]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = to_upstream.shutdown(Shutdown::Write);
    });
    let (mut from_upstream, mut to_client) = (upstream, client);
    let _ = std::io::copy(&mut from_upstream, &mut to_client);
    let _ = to_client.shutdown(Shutdown::Both);
    let _ = from_upstream.shutdown(Shutdown::Both);
    let _ = up.join();
}

/// `inbox serve` over `keys.json` in `dir` on an ephemeral loopback port, with `extra` flags.
fn serve(dir: &Path, operator: &str, extra: &[&str]) -> Service {
    let mut args = vec![
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator,
    ];
    args.extend_from_slice(extra);
    Service::start(dir, &args, &[])
}

/// The `host:port` of a service's URL.
fn address(url: &str) -> &str {
    url.split_once("://").map_or(url, |(_, rest)| rest)
}

/// The acceptance of the story: a wake started on an empty inbox keeps running across the end
/// of a wait (`--max-wait 1`) and across a restart of the service (the reconnect inside it opens
/// the inbox again at the new start), and exits 0 with the message another key then posts in its
/// output; the message is in `Delivered`. The new start's `--max-wait` is longer than the case,
/// so only the post ends its wait.
#[test]
fn a_wake_on_an_empty_inbox_returns_on_a_post_after_the_end_of_a_wait_and_a_restart() {
    let (dir, operator) = keys_only("wake-acceptance");
    let first = serve(&dir.path, &operator, &["--max-wait", "1"]);
    let forwarder = Forwarder::start(address(&first.url));
    let url = format!("http://{}", forwarder.address);
    client(&dir.path, &url, "bob", &["open"]).ok();
    let outbox = dir.join("bob.outbox");
    let record = || fs::read_to_string(beside(&outbox, ".opened")).unwrap_or_default();
    let opened_first = record();
    assert!(opened_first.contains("\"epoch\""), "{opened_first}");

    let woken = Started::spawn(&mut wake(&dir.path, &url, "bob", &outbox, &["--json"]));
    // A wait of the first start ended without a row, and the wake asked again.
    wait_for("a third read of Pending", || {
        forwarder
            .sent()
            .iter()
            .filter(|bytes| bytes.starts_with(b"GET ") && contains(bytes, b"/pending?"))
            .count()
            >= 3
    });

    let second = serve(&dir.path, &operator, &["--max-wait", "60"]);
    let ended = first.terminate();
    assert!(ended.success(), "{ended:?}");
    forwarder.to(address(&second.url));
    wait_for("the wake opened the inbox again at the new start", || {
        let now = record();
        !now.is_empty() && now != opened_first
    });

    let posted = client(
        &dir.path,
        &url,
        "alice",
        &["post", "--to", BOB, "--body", "wake up"],
    )
    .ok();
    let message = posted.stdout.trim().to_owned();
    let (_, ran) = woken.finish();
    assert_eq!(ran.status, 0, "{ran:#?}");
    let document = ran.json();
    assert_eq!(ids(&document), std::slice::from_ref(&message), "{ran:#?}");
    assert!(
        ran.stderr.contains("reconnect at the start"),
        "the reconnect is said: {ran:#?}"
    );

    let listed = client(&dir.path, &second.url, "bob", &["pending", "--json"]).ok();
    let rows = support::rows(&listed.json());
    assert_eq!(rows.len(), 1, "{listed:#?}");
    assert_eq!(support::text(&rows[0], "message_id"), message);
    assert_eq!(support::text(&rows[0], "state"), "Delivered", "{listed:#?}");
}

/// A wake started while a message is in `Delivered` and not yet read (a first wake printed it)
/// does not return on it: it holds its read open, and returns only on the next post, with that
/// message alone in its output. Both are then in `Delivered`.
#[test]
fn a_wake_started_over_a_delivered_unread_message_returns_only_on_the_next_post() {
    let (dir, operator) = keys_only("wake-delivered");
    let service = serve(&dir.path, &operator, &["--max-wait", "60"]);
    let forwarder = Forwarder::start(address(&service.url));
    let url = format!("http://{}", forwarder.address);
    client(&dir.path, &url, "bob", &["open"]).ok();
    let outbox = dir.join("bob.outbox");
    let post = |body: &str| {
        let posted = client(
            &dir.path,
            &url,
            "alice",
            &["post", "--to", BOB, "--body", body],
        );
        posted.ok().stdout.trim().to_owned()
    };
    let first = post("first");
    let (_, ran) = Started::spawn(&mut wake(&dir.path, &url, "bob", &outbox, &["--json"])).finish();
    assert_eq!(ran.status, 0, "{ran:#?}");
    assert_eq!(ids(&ran.json()), std::slice::from_ref(&first), "{ran:#?}");

    let held_reads = || {
        forwarder
            .sent()
            .iter()
            .filter(|bytes| contains(bytes, b"/pending?") && contains(bytes, b"&wait=60&"))
            .count()
    };
    let before = held_reads();
    let mut woken = Started::spawn(&mut wake(&dir.path, &url, "bob", &outbox, &["--json"]));
    wait_for(
        "the second wake holds a read open over the delivered message",
        || held_reads() > before,
    );
    assert!(
        woken
            .child()
            .try_wait()
            .expect("the wake is asked")
            .is_none(),
        "the wake ended on a message in Delivered"
    );
    let second = post("second");
    let (_, ran) = woken.finish();
    assert_eq!(ran.status, 0, "{ran:#?}");
    assert_eq!(ids(&ran.json()), std::slice::from_ref(&second), "{ran:#?}");

    let listed = client(&dir.path, &service.url, "bob", &["pending", "--json"]).ok();
    let rows = support::rows(&listed.json());
    let states: Vec<(String, String)> = rows
        .iter()
        .map(|row| {
            (
                support::text(row, "message_id"),
                support::text(row, "state"),
            )
        })
        .collect();
    assert_eq!(
        states,
        [
            (first, "Delivered".to_owned()),
            (second, "Delivered".to_owned())
        ],
        "{listed:#?}"
    );
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

/// The file beside the outbox file `outbox` whose name ends in `suffix`.
fn beside(outbox: &Path, suffix: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// The files of `dir` whose name starts with `prefix`, by inode.
fn lock_files(dir: &Path, prefix: &str) -> Vec<(PathBuf, u64)> {
    let mut found: Vec<(PathBuf, u64)> = fs::read_dir(dir)
        .expect("the directory is read")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .map(|entry| {
            let inode = entry.metadata().expect("the file is there").ino();
            (entry.path(), inode)
        })
        .collect();
    found.sort();
    found
}

/// The processes that hold a `flock` of the file with the inode `inode`, from `/proc/locks`.
fn holders(inode: u64) -> Vec<u32> {
    fs::read_to_string("/proc/locks")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.get(1) != Some(&"FLOCK") {
                return None;
            }
            let pid = fields.get(4)?.parse().ok()?;
            let node: u64 = fields.get(5)?.rsplit(':').next()?.parse().ok()?;
            (node == inode).then_some(pid)
        })
        .collect()
}

/// The lock file of `who`'s wake among those of `prefix` in `dir`: the one `pid` holds.
fn held_by(dir: &Path, prefix: &str, pid: u32) -> Option<u64> {
    lock_files(dir, prefix)
        .into_iter()
        .map(|(_, inode)| inode)
        .find(|inode| holders(*inode).contains(&pid))
}

/// One wake per inbox and outbox file on a host: a second wake of bob exits 5 with the one line
/// the story names and sends nothing, while alice, who shares bob's outbox file, waits beside
/// him. SIGTERM and SIGINT end a wake at once with nothing on standard output and release its
/// lock; the lock file it leaves blocks the next wake of bob nothing.
#[test]
fn a_second_wake_of_one_inbox_exits_5_and_a_signal_ends_a_wake_and_frees_its_lock() {
    let world = support::World::new("wake-lock");
    let dir = &world.dir.path;
    let shared = world.outbox("bob");
    let prefix = "bob.outbox.wake";
    // Both inboxes are open: a wake of an inbox that is not open ends at once.
    let shared_text = shared.to_str().expect("a UTF-8 path");
    for who in ["bob", "alice"] {
        world
            .client_env(who, &["open"], &[("INBOX_OUTBOX", shared_text)])
            .ok();
    }

    let bob = Started::spawn(&mut wake(dir, &world.service.url, "bob", &shared, &[]));
    let bob_pid = bob.pid();
    wait_for("bob's wake holds its lock", || {
        held_by(dir, prefix, bob_pid).is_some()
    });
    let bob_lock = held_by(dir, prefix, bob_pid).expect("bob's lock");

    let silent = TcpListener::bind("127.0.0.1:0").expect("a port");
    let silent_url = format!("http://{}", silent.local_addr().expect("an address"));
    let (_, second) = Started::spawn(&mut wake(dir, &silent_url, "bob", &shared, &[])).finish();
    assert_eq!(second.status, ANOTHER_WAKE, "{second:#?}");
    assert_eq!(second.stdout, "", "{second:#?}");
    assert_eq!(
        second.stderr,
        format!("inbox wake: another wake of {BOB} runs on this host\n")
    );
    silent.set_nonblocking(true).expect("no blocking");
    assert!(
        matches!(silent.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "the second wake sent nothing"
    );

    let alice = Started::spawn(&mut wake(dir, &world.service.url, "alice", &shared, &[]));
    let alice_pid = alice.pid();
    wait_for("alice's wake holds a lock of her own", || {
        held_by(dir, prefix, alice_pid).is_some_and(|inode| inode != bob_lock)
    });

    bob.signal("-TERM");
    let (status, ended) = bob.finish();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{ended:#?}");
    assert_eq!(ended.stdout, "", "{ended:#?}");
    assert!(holders(bob_lock).is_empty(), "the lock is released");

    let again = Started::spawn(&mut wake(dir, &world.service.url, "bob", &shared, &[]));
    let again_pid = again.pid();
    wait_for(
        "the next wake of bob takes the lock file left behind",
        || holders(bob_lock).contains(&again_pid),
    );
    again.signal("-INT");
    let (status, ended) = again.finish();
    assert_eq!(status.signal(), Some(libc::SIGINT), "{ended:#?}");
    assert_eq!(ended.stdout, "", "{ended:#?}");

    alice.signal("-TERM");
    let (status, ended) = alice.finish();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{ended:#?}");
    assert_eq!(ended.stdout, "", "{ended:#?}");
}

/// A service whose TLS certificate is not the pinned one ends the wake at once with 3: one
/// connection, no second try.
#[test]
fn a_certificate_that_is_not_the_pinned_one_ends_the_wake_at_once_with_3() {
    let (dir, operator) = keys_only("wake-pin");
    run(&dir.path, &["tls", "new", "--out", "tls.json"], &[]).ok();
    let other = run(&dir.path, &["tls", "new", "--out", "other.json"], &[])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let service = serve(&dir.path, &operator, &["--tls-file", "tls.json"]);
    let forwarder = Forwarder::start(address(&service.url));
    let (_, ran) = Started::spawn(
        wake(
            &dir.path,
            &format!("https://{}", forwarder.address),
            "bob",
            &dir.join("bob.outbox"),
            &[],
        )
        .env("INBOX_TLS_PIN", &other),
    )
    .finish();
    assert_eq!(ran.status, TRANSPORT, "{ran:#?}");
    assert_eq!(ran.stdout, "", "{ran:#?}");
    assert_eq!(ran.stderr.lines().count(), 1, "{ran:#?}");
    assert_eq!(forwarder.sent().len(), 1, "one connection, no second try");
}

/// The reads of `Pending` held open through `forwarder` so far.
fn held_reads(forwarder: &Forwarder) -> usize {
    forwarder
        .sent()
        .iter()
        .filter(|bytes| contains(bytes, b"/pending?") && contains(bytes, b"&wait=60&"))
        .count()
}

/// A wake of an inbox this host never opened ends at once with 1 and one line that names
/// `inbox open`, without a held read: the service answers its read "not open". A wake of an
/// opened inbox that holds its read is ended by `inbox close` the same way, and does not open
/// the inbox again: a post to it is still refused `UnknownInbox`.
#[test]
fn a_wake_of_an_inbox_that_is_not_open_ends_with_1_and_names_inbox_open_also_after_a_close() {
    let (dir, operator) = keys_only("wake-not-open");
    let service = serve(&dir.path, &operator, &["--max-wait", "60"]);
    let forwarder = Forwarder::start(address(&service.url));
    let url = format!("http://{}", forwarder.address);
    let outbox = dir.join("bob.outbox");
    let (_, never) = Started::spawn(&mut wake(&dir.path, &url, "bob", &outbox, &[])).finish();
    assert_eq!(never.status, REFUSED, "{never:#?}");
    assert_eq!(never.stdout, "", "{never:#?}");
    let lines: Vec<&str> = never.stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{never:#?}");
    assert!(lines[0].contains("`inbox open`"), "{never:#?}");
    assert!(
        lines[0].contains("no record of an `inbox open`"),
        "{never:#?}"
    );
    assert_eq!(held_reads(&forwarder), 0, "no read was held");

    client(&dir.path, &url, "bob", &["open"]).ok();
    let woken = Started::spawn(&mut wake(&dir.path, &url, "bob", &outbox, &[]));
    wait_for("the wake holds its read", || held_reads(&forwarder) >= 1);
    client(&dir.path, &url, "bob", &["close"]).ok();
    let (_, closed) = woken.finish();
    assert_eq!(closed.status, REFUSED, "{closed:#?}");
    assert!(
        closed.stderr.contains("no record of an `inbox open`"),
        "{closed:#?}"
    );
    let refused = client(
        &dir.path,
        &service.url,
        "alice",
        &["post", "--to", BOB, "--body", "to a closed inbox"],
    );
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert!(refused.stderr.contains("UnknownInbox"), "{refused:#?}");
}

/// The epoch of bob's record of an `inbox open` beside `outbox`.
fn recorded_epoch(outbox: &Path) -> String {
    let record = fs::read_to_string(beside(outbox, ".opened")).expect("the record is written");
    let value = json::parse(record.trim()).expect("the record is JSON");
    support::text(&value, "epoch").to_owned()
}

/// What "inbox not open" ends a wake with when this host holds a record of an `inbox open`: an
/// open of the reconnect that the service refuses ends it with 1 and a line that says so; "not
/// open" again right after an open that succeeded ends it with 1 and a line of its own. A record
/// of the start that answers "not open" (an `inbox open` of the owner after the read began) has
/// nothing to open again: the wake reads once more at once, and returns on the row it gets.
#[test]
fn a_reopen_refused_or_still_not_open_ends_with_1_in_its_own_words_and_a_record_of_this_start_reads_again()
 {
    const NOT_OPEN: &str = r#"{"refused":"inbox not open"}"#;
    let (dir, operator) = keys_only("wake-reopen-words");
    let outbox = dir.join("bob.outbox");
    {
        let service = serve(&dir.path, &operator, &["--max-wait", "60"]);
        client(&dir.path, &service.url, "bob", &["open"]).ok();
    }
    let other = "7".repeat(32);
    let last_line = |ran: &Run| ran.stderr.lines().last().unwrap_or_default().to_owned();

    let refusing = StandIn::at_epoch(
        None,
        Some(other.clone()),
        Box::new(|_, request| {
            if request.is_pending() {
                Some((409, NOT_OPEN.to_owned()))
            } else {
                Some((403, r#"{"refused":"unauthenticated"}"#.to_owned()))
            }
        }),
    );
    let (_, ran) =
        Started::spawn(&mut wake(&dir.path, &refusing.url, "bob", &outbox, &[])).finish();
    let requests = refusing.requests();
    assert_eq!(ran.status, REFUSED, "{ran:#?} {requests:#?}");
    let said = last_line(&ran);
    assert!(
        said.contains("its open again") && said.contains("refused: unauthenticated (403)"),
        "{ran:#?}"
    );
    assert!(said.contains("`inbox open`"), "{ran:#?}");

    let at_this_start = recorded_epoch(&outbox);
    let answer = pending(&[row(1, "Sent", "after an open of this start")]);
    let reading = StandIn::at_epoch(
        None,
        Some(at_this_start),
        Box::new(move |n, _| match n {
            0 => Some((409, NOT_OPEN.to_owned())),
            1 => Some((200, answer.clone())),
            _ => Some((200, DELIVERED.to_owned())),
        }),
    );
    let (_, ran) = Started::spawn(&mut wake(&dir.path, &reading.url, "bob", &outbox, &[])).finish();
    let requests = reading.requests();
    assert_eq!(ran.status, 0, "{ran:#?} {requests:#?}");
    assert!(
        ran.stdout.contains("after an open of this start"),
        "{ran:#?}"
    );
    let lines: Vec<&str> = requests
        .iter()
        .map(|request| request.line.as_str())
        .collect();
    assert_eq!(
        requests.len(),
        3,
        "a read, a read again, a mark: {lines:#?}"
    );
    assert!(
        requests[1].is_pending() && requests[2].is_mark(),
        "{lines:#?}"
    );

    let opening = StandIn::at_epoch(
        None,
        Some(other),
        Box::new(|_, request| {
            if request.is_pending() {
                Some((409, NOT_OPEN.to_owned()))
            } else {
                Some((202, r#"{"outcome":"opened","published":[]}"#.to_owned()))
            }
        }),
    );
    let (_, ran) = Started::spawn(&mut wake(&dir.path, &opening.url, "bob", &outbox, &[])).finish();
    let requests = opening.requests();
    assert_eq!(ran.status, REFUSED, "{ran:#?} {requests:#?}");
    let said = last_line(&ran);
    assert!(
        said.contains("still not open") && said.contains("`inbox open`"),
        "{ran:#?}"
    );
    assert_eq!(requests.len(), 3, "a read, the open, a read: {requests:#?}");
}

/// Which answers are tried again: a `502` of a proxy without a refusal (its body not JSON) is,
/// and the wake exits 0 on the row the next read gets. A `302`, and a `200` whose body is not the
/// view, end the wake with 3 after one read.
#[test]
fn a_502_of_a_proxy_is_tried_again_and_a_3xx_or_a_2xx_that_is_not_the_view_ends_with_3() {
    let dir = bob_only("wake-statuses");
    let answer = pending(&[row(1, "Sent", "after a bad gateway")]);
    let proxy = StandIn::start(
        None,
        Box::new(move |n, _| match n {
            0 => Some((502, "<html>bad gateway</html>".to_owned())),
            1 => Some((200, answer.clone())),
            _ => Some((200, DELIVERED.to_owned())),
        }),
    );
    let (_, ran) = Started::spawn(&mut wake(
        &dir.path,
        &proxy.url,
        "bob",
        &dir.join("bob.outbox"),
        &[],
    ))
    .finish();
    let requests = proxy.requests();
    assert_eq!(ran.status, 0, "{ran:#?} {requests:#?}");
    assert_eq!(requests.len(), 3, "{requests:#?}");
    assert!(ran.stderr.contains("trying again"), "{ran:#?}");
    for (status, body) in [(302, "{}"), (200, r#"{"outcome":"opened"}"#)] {
        let other = StandIn::start(None, Box::new(move |_, _| Some((status, body.to_owned()))));
        let (_, ran) = Started::spawn(&mut wake(
            &dir.path,
            &other.url,
            "bob",
            &dir.join("bob.outbox"),
            &[],
        ))
        .finish();
        let requests = other.requests();
        assert_eq!(ran.status, TRANSPORT, "{status}: {ran:#?}");
        assert_eq!(requests.len(), 1, "{status}: {requests:#?}");
        assert_eq!(ran.stderr.lines().count(), 1, "{status}: {ran:#?}");
    }
}

/// A standard output that is `/dev/null` takes the rows to no reader: the wake sends no request,
/// neither a read nor a `MarkDelivered`, and exits 4 with one line, so the row stays in `Sent`.
#[test]
fn a_wake_whose_standard_output_is_dev_null_marks_nothing_and_exits_4() {
    let dir = bob_only("wake-dev-null");
    let answer = pending(&[row(1, "Sent", "to nobody")]);
    let stand_in = StandIn::start(None, Box::new(move |_, _| Some((200, answer.clone()))));
    let (_, ran) = Started::spawn(
        wake(
            &dir.path,
            &stand_in.url,
            "bob",
            &dir.join("bob.outbox"),
            &[],
        )
        .stdout(Stdio::null()),
    )
    .finish();
    let requests = stand_in.requests();
    assert_eq!(ran.status, LOCAL, "{ran:#?}");
    assert_eq!(ran.stderr.lines().count(), 1, "{ran:#?}");
    assert!(ran.stderr.contains("standard output"), "{ran:#?}");
    assert_eq!(requests.len(), 0, "no read and no mark: {requests:#?}");
}
