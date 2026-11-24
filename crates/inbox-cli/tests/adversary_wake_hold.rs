//! Adversary cases of story:a-wake-that-ended-writes-no-second-line-when-its-reader-goes: the
//! point `inbox wake --before-exit <path>` of a debug build on the ends of a wake that the
//! story's case does not take, and on the orders of events its case does not take.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use crate::support;

use support::{ALICE, BOB, Dir, LOCAL, REFUSED};

/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

/// Longer than two looks of the watch of the reader (one look each 500 ms).
const TWO_LOOKS: Duration = Duration::from_millis(1200);

fn bob_only(name: &str) -> Dir {
    let dir = Dir::new(name);
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    dir
}

fn wake(dir: &Dir, url: &str, args: &[&str]) -> Command {
    let mut command = support::command(&dir.path);
    command
        .arg("wake")
        .args(args)
        .env("INBOX_URL", url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

/// The first line of a request, read in full (its head and its `Content-Length` bytes).
fn request_line(stream: &mut TcpStream) -> String {
    let _ = stream.set_read_timeout(Some(HANG));
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected: Option<usize> = None;
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

/// A stand-in on 127.0.0.1 that answers the `n`th request line as `answering` says.
struct StandIn {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<Vec<String>>>,
}

impl StandIn {
    fn start(answering: impl Fn(usize, &str) -> (u16, String) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let thread = std::thread::spawn(move || {
            let mut lines = Vec::new();
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < HANG {
                let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                let line = request_line(&mut stream);
                let (status, body) = answering(lines.len(), &line);
                lines.push(line);
                let text = format!(
                    "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(text.as_bytes());
            }
            lines
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }

    fn lines(mut self) -> Vec<String> {
        self.stop.store(true, Ordering::SeqCst);
        self.thread
            .take()
            .expect("not joined yet")
            .join()
            .expect("the stand-in ends")
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

const REFUSAL: &str = r#"{"refused":"unauthenticated"}"#;

/// A process of the binary, killed when dropped unfinished.
struct Started(Option<Child>);

impl Started {
    fn child(&mut self) -> &mut Child {
        self.0.as_mut().expect("not finished")
    }

    fn ended(&mut self) -> bool {
        self.child().try_wait().expect("asked").is_some()
    }

    /// Waits until the file `point` exists; panics with what the process wrote when it ended
    /// first.
    fn reach(&mut self, point: &Path) {
        let started = Instant::now();
        while !point.exists() {
            if self.ended() {
                let (status, stderr) = self.finish();
                panic!("the wake ended with {status} before the point: {stderr}");
            }
            assert!(started.elapsed() < HANG, "no point within {HANG:?}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Its exit status and standard error; a process that has not ended within [`HANG`] is
    /// killed and the case fails.
    fn finish(&mut self) -> (i32, String) {
        let started = Instant::now();
        while !self.ended() {
            if started.elapsed() > HANG {
                let mut child = self.0.take().expect("not finished");
                let _ = child.kill();
                let _ = child.wait();
                let mut stderr = String::new();
                if let Some(pipe) = child.stderr.as_mut() {
                    let _ = pipe.read_to_string(&mut stderr);
                }
                panic!("inbox wake hangs at the point, not ended within {HANG:?}: {stderr:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut child = self.0.take().expect("not finished");
        let status = child.wait().expect("waited");
        let mut stderr = String::new();
        if let Some(pipe) = child.stderr.as_mut() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        (status.code().unwrap_or(-1), stderr)
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

/// The reader goes after the wake ended and more than one look of the watch before the case
/// removes the file of the point: the watch has seen the pipe without a reader and the wake
/// ended, and has returned. The point must still let the process exit, with 1 and one line.
#[test]
fn a_reader_gone_two_looks_before_the_file_is_removed_does_not_hang_the_point() {
    let dir = bob_only("adv-hold-gone-early");
    let point = dir.join("before-exit");
    let stand_in = StandIn::start(|_, _| (403, REFUSAL.to_owned()));
    let mut started = Started(Some(
        wake(
            &dir,
            &stand_in.url,
            &["--before-exit", point.to_str().expect("UTF-8")],
        )
        .spawn()
        .expect("the binary starts"),
    ));
    started.reach(&point);
    drop(started.child().stdout.take());
    std::thread::sleep(TWO_LOOKS);
    fs::remove_file(&point).expect("the file of the point is removed");
    let (status, stderr) = started.finish();
    assert_eq!(status, REFUSED, "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
}

/// A wake whose standard output is a pipe without a reader from its start ends with 4 before
/// the watch is started. With `--before-exit` it must still exit once the file is removed.
#[test]
fn a_wake_without_a_reader_at_its_start_does_not_hang_at_the_point() {
    let dir = bob_only("adv-hold-no-reader");
    let point = dir.join("before-exit");
    let (reader, writer) = std::io::pipe().expect("a pipe");
    drop(reader);
    let mut command = wake(
        &dir,
        "http://127.0.0.1:9",
        &["--before-exit", point.to_str().expect("UTF-8")],
    );
    command.stdout(Stdio::from(writer));
    let mut started = Started(Some(command.spawn().expect("the binary starts")));
    started.reach(&point);
    fs::remove_file(&point).expect("the file of the point is removed");
    let (status, stderr) = started.finish();
    assert_eq!(status, LOCAL, "{stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
}

/// A row of `Pending` for bob, from alice, in `Sent`.
fn row(id: &str) -> String {
    format!(
        r#"{{"message_id":"{id}","from":"{ALICE}","on_behalf_of":"{ALICE}","to":"{BOB}","kind":"note","thread":"{id}","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"{id}","body":"hello","state":"Sent"}}"#
    )
}

/// `inbox wake --follow` gets a row, marks it, and its next read is refused: the wake ends
/// with 1 and one line after a full round, and its reader goes before the exit. The end of a
/// round of `--follow` sets no hold of its own, so only the end of the wake holds the watch.
#[test]
fn a_follow_wake_refused_after_a_full_round_writes_one_line_when_its_reader_goes() {
    let dir = bob_only("adv-hold-follow");
    let point = dir.join("before-exit");
    let id = format!("01J{:023}", 1);
    let reads = AtomicUsize::new(0);
    let answer = format!(r#"{{"rows":[{}]}}"#, row(&id));
    let stand_in = StandIn::start(move |_, line| {
        if line.starts_with("GET ") && line.contains("/pending?") {
            if reads.fetch_add(1, Ordering::SeqCst) == 0 {
                (200, answer.clone())
            } else {
                (403, REFUSAL.to_owned())
            }
        } else {
            (200, r#"{"outcome":"delivered","published":[]}"#.to_owned())
        }
    });
    let mut started = Started(Some(
        wake(
            &dir,
            &stand_in.url,
            &["--follow", "--before-exit", point.to_str().expect("UTF-8")],
        )
        .spawn()
        .expect("the binary starts"),
    ));
    started.reach(&point);
    drop(started.child().stdout.take());
    fs::remove_file(&point).expect("the file of the point is removed");
    let (status, stderr) = started.finish();
    let lines = stand_in.lines();
    assert_eq!(status, REFUSED, "{stderr} {lines:#?}");
    assert_eq!(
        stderr.lines().collect::<Vec<_>>(),
        ["inbox wake: refused: unauthenticated (403)"],
        "{lines:#?}"
    );
    assert_eq!(lines.len(), 3, "{lines:#?}");
}
