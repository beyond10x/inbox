//! Adversary pass 1 on story:the-case-of-the-early-bridge-counts-its-lines-after-a-second-try:
//! what the tries of a bridge that waits for the service are, and the pause between them. No case
//! reaches a network beyond loopback or a model, and the stand-in `connectors` is never run.
//! Every key is made at run time.

use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use support::Dir;

const PERSON: &str = "human:person";
const HANG: Duration = support::STAND_IN_WAIT;

/// A listener on a loopback port the kernel picks that keeps the request line of each connection,
/// reads what arrives of the request and closes it without an answer. Its thread ends when it is
/// dropped, or after [`HANG`].
struct Tries {
    url: String,
    lines: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
}

impl Tries {
    fn start() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the listener binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let lines = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (kept, stopped) = (Arc::clone(&lines), Arc::clone(&stop));
        std::thread::spawn(move || {
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < HANG {
                let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                // What arrives of the request is read, so the close is no reset of a write.
                let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
                let mut received = Vec::new();
                let mut buffer = [0_u8; 4096];
                while let Ok(read @ 1..) = std::io::Read::read(&mut stream, &mut buffer) {
                    received.extend_from_slice(&buffer[..read]);
                }
                let line = String::from_utf8_lossy(&received)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned();
                kept.lock().expect("the lines").push(line);
                let _ = stream.shutdown(std::net::Shutdown::Both);
            }
        });
        Self { url, lines, stop }
    }

    fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("the lines").clone()
    }
}

impl Drop for Tries {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// `inbox bridge` of the person against `url`, with a stand-in `connectors` that is never run.
fn bridge(dir: &Dir, url: &str) -> Child {
    support::key_new(&dir.path, "person.key", Some(("person-1", PERSON)));
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).expect("the directory of the stand-in is made");
    let connectors = bin.join("connectors");
    fs::write(&connectors, "#!/bin/sh\nexit 1\n").expect("the stand-in is written");
    fs::set_permissions(&connectors, fs::Permissions::from_mode(0o755)).expect("the stand-in runs");
    support::command(&dir.path)
        .args([
            "bridge",
            "--chat-channel",
            "C0000000001",
            "--allowed-chat-user",
            "U0000000001",
            "--chat-connector",
            "slack",
            "--chat-poll-interval",
            "3600",
        ])
        .env("PATH", &bin)
        .env("INBOX_URL", url)
        .env("INBOX_SECRET_KEY_FILE", "person.key")
        .env("INBOX_OUTBOX", dir.join("person.outbox"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the binary starts")
}

/// Waits until `tries` kept `count` request lines or the bridge ended ([`HANG`] only ends a hang);
/// answers the lines, and how the bridge ended if it did. The bridge is ended.
fn tried(mut child: Child, tries: &Tries, count: usize) -> (Vec<String>, Option<String>) {
    let started = Instant::now();
    let mut ended = None;
    while tries.lines().len() < count && started.elapsed() < HANG {
        if let Some(status) = child.try_wait().expect("the process is asked") {
            ended = Some(format!("{status:?}"));
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    (tries.lines(), ended)
}

/// The mutant `std::thread::sleep(pause(*failures))` removed from `wake::say_once` (a bridge, or a
/// wake, that connects again at once, in a loop, to a service that does not answer) leaves the
/// whole `inbox-cli` suite green. The pauses after the first 3 failures are 100, 200 and 400 ms,
/// so the 4th try comes at least 700 ms after the bridge was started. Only that lower bound is
/// held, which a slow host cannot break.
#[test]
fn a_bridge_that_waits_for_the_service_pauses_between_its_tries() {
    let dir = Dir::new("adv-early-bridge-pause");
    let tries = Tries::start();
    let started = Instant::now();
    let (lines, ended) = tried(bridge(&dir, &tries.url), &tries, 4);
    let took = started.elapsed();
    assert_eq!(ended, None, "the bridge ended: {lines:?}");
    assert!(lines.len() >= 4, "{lines:?} within {} s", HANG.as_secs());
    assert!(
        took >= Duration::from_millis(700),
        "{} tries within {took:?} of the bridge's start: no pause of 100, 200 and 400 ms between \
         them",
        lines.len()
    );
}

/// The mutant "a failed open is the start done" (`Err(Failure::Transport(_)) => break,` before the
/// arm of `say_once` in the loop of the start in `wake::rounds`: the bridge goes on to read
/// `Pending` of an inbox it never opened) leaves the whole `inbox-cli` suite green, and the case
/// `bridge::a_bridge_started_before_the_service_answers_says_so_and_keeps_trying` with it: it
/// counts connections and does not look at what they ask. Each of the first 3 tries is the open.
#[test]
fn each_try_of_a_bridge_that_waits_for_the_service_is_its_open() {
    let dir = Dir::new("adv-early-bridge-opens");
    let tries = Tries::start();
    let (lines, ended) = tried(bridge(&dir, &tries.url), &tries, 3);
    assert_eq!(ended, None, "the bridge ended: {lines:?}");
    assert!(lines.len() >= 3, "{lines:?} within {} s", HANG.as_secs());
    for line in &lines[..3] {
        let mut parts = line.split(' ');
        let (method, target) = (parts.next(), parts.next().unwrap_or_default());
        assert!(
            method == Some("POST") && target.ends_with("/open-inbox"),
            "a try that is not the open of the start: {lines:?}"
        );
    }
}
