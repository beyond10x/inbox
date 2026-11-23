//! Adversary pass 2 on story:inbox-wake-returns-on-a-post: rounds 2 and 3 (the `409` of a read
//! with `undelivered=1`, the cut answer told from a whole one). Each case states a rule of
//! `docs/design/wake.md` that `inbox wake` breaks. A bounded wait only ends a hang. Every key is
//! made at run time.

use crate::support;

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use support::{ALICE, BOB, Dir, TRANSPORT};

/// How long a case waits for a fact: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

fn wait_for(what: &str, mut fact: impl FnMut() -> bool) {
    let started = Instant::now();
    while !fact() {
        assert!(started.elapsed() < HANG, "{what}: not within {HANG:?}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A child killed and waited for when dropped.
struct Started(Child);

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl Started {
    fn exited(&mut self) -> Option<i32> {
        self.0
            .try_wait()
            .expect("the process is asked")
            .map(|status| status.code().unwrap_or(-1))
    }

    fn stderr(mut self) -> String {
        let _ = self.0.kill();
        let _ = self.0.wait();
        let mut stderr = String::new();
        if let Some(pipe) = self.0.stderr.as_mut() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        stderr
    }
}

fn wake(dir: &Path, url: &str, who: &str) -> Started {
    let mut command = support::command(dir);
    command
        .arg("wake")
        .env("INBOX_URL", url)
        .env("INBOX_SECRET_KEY_FILE", format!("{who}.key"))
        .env("INBOX_OUTBOX", dir.join(format!("{who}.outbox")))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    Started(command.spawn().expect("the binary starts"))
}

/// Reads one request (its head, and its body as `Content-Length` says) and answers its first line.
fn read_request(stream: &mut TcpStream) -> String {
    let _ = stream.set_read_timeout(Some(HANG));
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected = None;
    loop {
        if expected.is_some_and(|total| received.len() >= total) {
            break;
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => received.extend_from_slice(&buffer[..read]),
        }
        if expected.is_none()
            && let Some(end) = received.windows(4).position(|window| window == b"\r\n\r\n")
        {
            let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
            let length: usize = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0);
            expected = Some(end + 4 + length);
        }
    }
    String::from_utf8_lossy(&received)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// A stand-in on 127.0.0.1:0: `answer` is given each request's first line and answers the bytes
/// to write, or `None` to close the connection without an answer. It counts the `GET`s.
struct StandIn {
    url: String,
    reads: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl StandIn {
    fn start(answer: impl Fn(&str) -> Option<String> + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let reads = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (counted, stopped) = (Arc::clone(&reads), Arc::clone(&stop));
        std::thread::spawn(move || {
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < 3 * HANG {
                let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                let line = read_request(&mut stream);
                if line.starts_with("GET ") {
                    counted.fetch_add(1, Ordering::SeqCst);
                }
                if let Some(bytes) = answer(&line) {
                    let _ = stream.write_all(bytes.as_bytes());
                }
                let _ = stream.shutdown(Shutdown::Both);
            }
        });
        Self { url, reads, stop }
    }

    fn reads(&self) -> usize {
        self.reads.load(Ordering::SeqCst)
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// `docs/design/wake.md`, "Failures and tries": "Any other answer that is neither a success nor a
/// refusal: ... a `4xx` without `refused`, a whole answer whose body is not JSON ... exits 3 with
/// one line". An answer without `Content-Length` that ends with its connection (`Connection:
/// close`, HTTP/1.1 section 6.3 of RFC 9112: the body is whole when the connection closes) is such
/// a whole answer. The client counts it as cut (`client.rs`, `Reply::read`: whole only with a
/// `Content-Length`), so a `404` page of another HTTP server on the URL is tried again forever.
#[test]
fn a_whole_404_page_without_content_length_ends_the_wake_with_3() {
    let dir = Dir::new("adv2-wake-no-length");
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    let stand_in = StandIn::start(|_| {
        Some(
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n\
             <html><body><h1>Not Found</h1></body></html>\n"
                .to_owned(),
        )
    });
    let mut woken = wake(&dir.path, &stand_in.url, "bob");
    wait_for("the wake ends or reads a third time", || {
        woken.exited().is_some() || stand_in.reads() >= 3
    });
    let asked = stand_in.reads();
    let exited = woken.exited();
    let said = woken.stderr();
    assert_eq!(
        exited,
        Some(TRANSPORT),
        "inbox wake still runs after {asked} reads answered by a whole 404 page without \
         Content-Length; it said: {said}"
    );
}

/// `docs/design/wake.md`: on `409 {"refused":"inbox not open"}` the wake runs the reconnect and
/// ends with exit 1 only "when it opened nothing (this host never opened the inbox, or `inbox
/// close` took the record)"; "No answer: the service cannot be reached ... tries again". Here this
/// host holds the record of an `inbox open` of bob, the read is answered `409` by a new start, and
/// the open of the reconnect gets no answer (the start went away again: a second restart in a
/// row). The wake ends with exit 1 and says that no `inbox open` of it on this host opened it
/// again and to run `inbox open` first: false on both counts, and a transient failure ends it.
#[test]
fn a_reopen_that_gets_no_answer_does_not_end_the_wake_as_never_opened() {
    let world = support::World::new("adv2-wake-reopen-cut");
    let opened = world.client("bob", &["open"]);
    assert_eq!(opened.status, 0, "{opened:#?}");
    // The service is only needed for the record: the stand-in is the next start.
    let stand_in = StandIn::start(|line| {
        if line.starts_with("GET ") {
            let body = r#"{"refused":"inbox not open"}"#;
            Some(format!(
                "HTTP/1.1 409 Conflict\r\nContent-Type: application/json\r\nInbox-Epoch: \
                 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                "5".repeat(32),
                body.len()
            ))
        } else {
            None
        }
    });
    let mut woken = wake(&world.dir.path, &stand_in.url, "bob");
    wait_for("the wake ends or reads a second time", || {
        woken.exited().is_some() || stand_in.reads() >= 2
    });
    let exited = woken.exited();
    let said = woken.stderr();
    assert_eq!(
        exited, None,
        "inbox wake ended when the open of its reconnect got no answer; it said: {said}"
    );
}

/// A stand-in on 127.0.0.1:0 that answers each `GET` with a view of one row of bob in `Sent` and
/// each `POST` with `delivered`, and counts the `POST`s: the marks.
fn one_row_in_sent() -> (StandIn, Arc<AtomicUsize>) {
    let marks = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&marks);
    let stand_in = StandIn::start(move |line| {
        let body = if line.starts_with("GET ") {
            format!(
                r#"{{"rows":[{{"message_id":"01J00000000000000000000001","from":"{ALICE}","on_behalf_of":"{ALICE}","to":"{BOB}","kind":"note","thread":"01J00000000000000000000001","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"01J00000000000000000000001","body":"to nobody","state":"Sent"}}]}}"#
            )
        } else {
            counted.fetch_add(1, Ordering::SeqCst);
            r#"{"outcome":"delivered","published":[]}"#.to_owned()
        };
        let status = if line.starts_with("GET ") { 200 } else { 202 };
        Some(format!(
            "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: \
             {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
    });
    (stand_in, marks)
}

/// `docs/design/wake.md`, the delivery: "A standard output that cannot be written ... ends the
/// wake without a mark, exit 4". Descriptor 1 open but read only (`inbox wake 1<file`) cannot be
/// written: each write fails `EBADF`, which the standard library's standard output counts as
/// written (the same rule pass 1 met for a closed descriptor). `output::reaches_a_reader` lets it
/// pass (the duplicate and the `fstat` succeed, it is no `/dev/null`), so the row is marked
/// delivered though no byte of it was written, and the wake exits 0.
#[test]
fn a_standard_output_open_only_for_reading_marks_nothing_and_exits_4() {
    let dir = Dir::new("adv2-wake-read-only-stdout");
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    let readable = dir.join("read-only");
    std::fs::write(&readable, b"").expect("the file is written");
    let (stand_in, marks) = one_row_in_sent();
    let mut command = support::command(&dir.path);
    command
        .arg("wake")
        .env("INBOX_URL", &stand_in.url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"))
        .stdout(std::fs::File::open(&readable).expect("the file opens for reading"))
        .stderr(Stdio::piped());
    let mut woken = Started(command.spawn().expect("the binary starts"));
    wait_for("the wake ends", || woken.exited().is_some());
    let exited = woken.exited();
    let said = woken.stderr();
    assert_eq!(
        (exited, marks.load(Ordering::SeqCst)),
        (Some(support::LOCAL), 0),
        "inbox wake with a standard output it cannot write marked the row (exit, marks); it \
         said: {said}"
    );
}

/// Round 3's decision 4 (`round3.md`): "before the lock and the first request the wake checks
/// that descriptor 1 is open ... and exits 4 with one line when it is not". The check runs only
/// when a row is to be written (`output::reaches_a_reader` in `try_print_*`): a wake started with
/// descriptor 1 closed (which the standard library's start turns into `/dev/null`, as here) takes
/// the lock and holds its reads on an empty inbox, so a second, working wake of that inbox exits 5
/// until a message comes, and that message is then not delivered.
#[test]
fn a_wake_whose_standard_output_no_one_reads_exits_4_before_its_first_request() {
    let dir = Dir::new("adv2-wake-null-first");
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    let stand_in = StandIn::start(|_| {
        let body = r#"{"rows":[]}"#;
        Some(format!(
            "HTTP/1.1 200 Stand-in\r\nContent-Type: application/json\r\nContent-Length: \
             {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
    });
    let mut command = support::command(&dir.path);
    command
        .arg("wake")
        .env("INBOX_URL", &stand_in.url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut woken = Started(command.spawn().expect("the binary starts"));
    let started = Instant::now();
    while woken.exited().is_none() && started.elapsed() < HANG {
        std::thread::sleep(Duration::from_millis(10));
    }
    let (exited, asked) = (woken.exited(), stand_in.reads());
    drop(woken);
    assert_eq!(
        (exited, asked),
        (Some(support::LOCAL), 0),
        "inbox wake with no reader of its standard output (exit, reads of Pending) after {:?}",
        started.elapsed()
    );
}

/// `docs/design/wake.md`, "Failures and tries": "`408`, or any status from `500` on, that is no
/// refusal of the model ... tries again", and a `4xx` without `refused` exits 3. The two ends of
/// that rule, `500` and `499`: no case of `tests/wake.rs` holds either (the mutant `status >= 501`
/// of `wake::passing` survives the suite).
#[test]
fn a_500_is_tried_again_and_a_499_ends_the_wake_with_3() {
    for (status, tried_again) in [(500_u16, true), (499, false)] {
        let dir = Dir::new(&format!("adv2-wake-status-{status}"));
        support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
        let stand_in = StandIn::start(move |_| {
            let body = r#"{"detail":"a proxy on loopback"}"#;
            Some(format!(
                "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: \
                 {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ))
        });
        let mut woken = wake(&dir.path, &stand_in.url, "bob");
        wait_for("the wake ends or reads a second time", || {
            woken.exited().is_some() || stand_in.reads() >= 2
        });
        let exited = woken.exited();
        let said = woken.stderr();
        let expected = if tried_again { None } else { Some(TRANSPORT) };
        assert_eq!(exited, expected, "{status}: {said}");
    }
}

/// `docs/design/wake.md`: "or the inbox is still not open right after it was opened again, the
/// wake ends with exit 1". A stand-in answers every read `409 inbox not open` from a start of its
/// own (a new `Inbox-Epoch` each time) and takes every open: without the guard the wake opens and
/// reads again forever. The case counts the reads of a wake that must end.
#[test]
fn a_service_that_is_never_open_after_an_open_ends_the_wake_with_1() {
    let world = support::World::new("adv2-wake-never-open");
    let opened = world.client("bob", &["open"]);
    assert_eq!(opened.status, 0, "{opened:#?}");
    let started = Arc::new(AtomicUsize::new(0));
    let starts = Arc::clone(&started);
    let stand_in = StandIn::start(move |line| {
        let (status, body, epoch) = if line.starts_with("GET ") {
            let n = starts.fetch_add(1, Ordering::SeqCst) + 1;
            (409, r#"{"refused":"inbox not open"}"#, n)
        } else {
            (
                202,
                r#"{"outcome":"opened","published":[]}"#,
                starts.load(Ordering::SeqCst),
            )
        };
        Some(format!(
            "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nInbox-Epoch: \
             {epoch:032x}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        ))
    });
    let mut woken = wake(&world.dir.path, &stand_in.url, "bob");
    wait_for("the wake ends or reads 20 times", || {
        woken.exited().is_some() || stand_in.reads() >= 20
    });
    let (exited, asked) = (woken.exited(), stand_in.reads());
    let said = woken.stderr();
    assert_eq!(exited, Some(1), "after {asked} reads: {said}");
}

/// `commands::unsent` and `AGENTS.md` (the epochs file): an envelope whose post "did not reach a
/// service that holds the recipient's inbox" is marked unsent, so a reconnect posts it again. A
/// whole answer that is neither a success nor a refusal (`Failure::Unexpected` since round 3:
/// here a `404` of another HTTP server on the URL) was a transport failure on `main`. A kept
/// envelope with no line in the epochs file (its post refused `413`, which takes the mark) that
/// `repost` meets with such a `404` gets the mark again, as on `main`. No case holds this: the
/// mutant that drops `Unexpected` from `unsent` survives the suite.
#[test]
fn a_repost_answered_404_by_another_server_marks_the_envelope_unsent() {
    let dir = Dir::new("adv2-repost-404");
    support::key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let outbox = dir.join("alice.outbox");
    let env = |url: &str| -> Vec<(String, String)> {
        vec![
            ("INBOX_URL".to_owned(), url.to_owned()),
            ("INBOX_SECRET_KEY_FILE".to_owned(), "alice.key".to_owned()),
            (
                "INBOX_OUTBOX".to_owned(),
                outbox.to_str().expect("UTF-8").to_owned(),
            ),
        ]
    };
    let run = |args: &[&str], url: &str| {
        let env = env(url);
        let pairs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        support::run(&dir.path, args, &pairs)
    };
    let mut epochs = outbox.as_os_str().to_owned();
    epochs.push(".epochs");
    let marks = || std::fs::read_to_string(&epochs).unwrap_or_default();

    let (url, served) = support::stand_in(413, r#"{"refused":"body too large","limit":1}"#);
    let posted = run(&["post", "--to", BOB, "--body", "x"], &url);
    served.join().expect("the stand-in ends");
    assert_eq!(posted.status, 1, "{posted:#?}");
    let kept = std::fs::read_to_string(&outbox).unwrap_or_default();
    assert_eq!(kept.lines().count(), 1, "the envelope is kept: {kept}");
    assert!(
        !marks().contains("\tnone"),
        "a 413 takes the mark: {:?}",
        marks()
    );

    let (url, served) = support::stand_in(404, r#"{"detail":"Not Found"}"#);
    let reposted = run(&["repost"], &url);
    served.join().expect("the stand-in ends");
    assert_eq!(reposted.status, TRANSPORT, "{reposted:#?}");
    assert!(
        marks().lines().any(|line| line.ends_with("\tnone")),
        "the envelope a 404 of another server met is marked unsent: {:?}",
        marks()
    );
}
