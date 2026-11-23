//! Adversary pass 1 on story:inbox-wake-returns-on-a-post. Each case states a rule of
//! `docs/design/wake.md` or `docs/design/service.md` that `inbox wake` breaks. No case waits for
//! `max_wait` or a pause; a bounded wait only ends a hang. Every key is made at run time.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use inbox_server::format_instant;
use inbox_verify::Instant as Reading;
use support::{BOB, Dir, LOCAL, Service, TRANSPORT, keys_only};

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

    fn output(mut self) -> (i32, String, String) {
        wait_for("inbox wake ends", || self.exited().is_some());
        let code = self.exited().unwrap_or(-1);
        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Some(pipe) = self.0.stdout.as_mut() {
            let _ = pipe.read_to_string(&mut stdout);
        }
        if let Some(pipe) = self.0.stderr.as_mut() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        (code, stdout, stderr)
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

fn bob_only(name: &str) -> Dir {
    let dir = Dir::new(name);
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    dir
}

/// Reads the head of one request, and its body as `Content-Length` says.
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

/// A stand-in on 127.0.0.1:0 that answers every request at once with `status` and `body`, and
/// counts the requests.
struct StandIn {
    url: String,
    count: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl StandIn {
    fn start(status: u16, body: impl Into<String>) -> Self {
        Self::answering(vec![(status, body.into())])
    }

    /// The `n`th request (from 0) is answered with `answers[n]`, every one after the last with the
    /// last.
    fn answering(answers: Vec<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let count = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (counted, stopped) = (Arc::clone(&count), Arc::clone(&stop));
        std::thread::spawn(move || {
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < 3 * HANG {
                let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                read_request(&mut stream);
                let n = counted.fetch_add(1, Ordering::SeqCst);
                let (status, body) = &answers[n.min(answers.len() - 1)];
                let text = format!(
                    "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(text.as_bytes());
            }
        });
        Self { url, count, stop }
    }

    fn count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// `docs/design/wake.md`, "Failures and tries": a transport failure is tried again after a pause,
/// and "a read that ends without a row starts the next round at once". An answer without a row
/// that comes at once to both reads (what `inbox serve --max-wait 0` answers, the flag is
/// accepted) makes the wake ask again and again with no pause: hundreds of connections a second
/// for as long as it runs. A slow host only lowers the count.
#[test]
fn an_empty_answer_that_comes_at_once_is_not_asked_again_without_a_pause() {
    let dir = bob_only("adv-wake-spin");
    let stand_in = StandIn::start(200, r#"{"rows":[]}"#);
    let woken = wake(&dir.path, &stand_in.url, "bob");
    std::thread::sleep(Duration::from_secs(2));
    let asked = stand_in.count();
    drop(woken);
    assert!(
        asked <= 50,
        "inbox wake read Pending {asked} times in 2 s against a service whose wait ends at once"
    );
}

/// `docs/design/wake.md`, "Failures and tries": only "the service cannot be reached, or cuts the
/// answer" and the service's own `408` and `503` are tried again; every other failure ends the
/// wake with its status, and the reference names 3 for "its answer was neither a success nor a
/// refusal". A `404` that is not the service's refusal (another HTTP server on the URL) is tried
/// again forever instead.
#[test]
fn an_answer_that_is_neither_a_success_nor_a_refusal_ends_the_wake_with_3() {
    let dir = bob_only("adv-wake-not-the-service");
    let stand_in = StandIn::start(404, r#"{"detail":"Not Found"}"#);
    let mut woken = wake(&dir.path, &stand_in.url, "bob");
    wait_for("the wake ends or asks a third time", || {
        woken.exited().is_some() || stand_in.count() >= 3
    });
    let asked = stand_in.count();
    assert_eq!(
        woken.exited(),
        Some(TRANSPORT),
        "inbox wake still runs after {asked} reads answered 404 by a server that is not the service"
    );
    assert_eq!(asked, 1);
}

/// `docs/design/wake.md`: the lock file "must be a regular file: exit 4 otherwise", opened as the
/// files beside the outbox file. A dangling symbolic link in its place is followed and the open
/// creates the file it names, elsewhere, then the wake runs.
#[test]
fn a_dangling_link_in_place_of_the_lock_file_creates_nothing_through_it() {
    let dir = bob_only("adv-wake-dangling");
    let hash = BOB.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    let place = dir.join(&format!("bob.outbox.wake.{hash:016x}"));
    fs::create_dir(dir.join("elsewhere")).expect("the directory is made");
    let target = dir.join("elsewhere").join("made-through-the-link");
    std::os::unix::fs::symlink(&target, &place).expect("the link is made");
    let stand_in = StandIn::start(200, r#"{"rows":[]}"#);
    let mut woken = wake(&dir.path, &stand_in.url, "bob");
    wait_for("the wake ends or asks", || {
        woken.exited().is_some() || stand_in.count() >= 1
    });
    let made = target.exists();
    let asked = stand_in.count();
    let ended = woken.exited();
    drop(woken);
    assert!(
        !made,
        "the open of the lock file created {} through a dangling link ({asked} requests sent)",
        target.display()
    );
    assert_eq!(ended, Some(LOCAL), "{asked} requests sent");
}

/// A TCP forwarder on 127.0.0.1:0 to `target` that counts the requests to `Pending` it carried;
/// with `then`, every connection after the first goes to `then`.
struct Forwarder {
    address: String,
    pending: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
}

impl Forwarder {
    fn start(target: &str) -> Self {
        Self::switching(target, None)
    }

    fn switching(target: &str, then: Option<&str>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the forwarder binds");
        let address = listener.local_addr().expect("an address").to_string();
        let pending = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (counted, stopped, target) = (
            Arc::clone(&pending),
            Arc::clone(&stop),
            Arc::new(Mutex::new(target.to_owned())),
        );
        let then = then.map(str::to_owned);
        std::thread::spawn(move || {
            let started = Instant::now();
            let mut connections = 0_usize;
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < 3 * HANG {
                let Some(client) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                connections += 1;
                let to = match &then {
                    Some(then) if connections > 1 => then.clone(),
                    _ => target.lock().expect("the target").clone(),
                };
                let counted = Arc::clone(&counted);
                std::thread::spawn(move || carry(client, &to, &counted));
            }
        });
        Self {
            address,
            pending,
            stop,
        }
    }

    fn pending(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn carry(client: TcpStream, target: &str, pending: &Arc<AtomicUsize>) {
    let Ok(upstream) = TcpStream::connect(target) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    let (Ok(mut from_client), Ok(mut to_upstream)) = (client.try_clone(), upstream.try_clone())
    else {
        return;
    };
    let pending = Arc::clone(pending);
    let up = std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let mut first = true;
        loop {
            match from_client.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    if first {
                        first = false;
                        let head = String::from_utf8_lossy(&buffer[..read]);
                        if head.starts_with("GET ") && head.contains("/pending?") {
                            pending.fetch_add(1, Ordering::SeqCst);
                        }
                    }
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
    let _ = up.join();
}

fn this_second() -> Reading {
    Reading::from_unix(
        Reading::from_system_time(std::time::SystemTime::now()).unix_seconds(),
        0,
    )
}

fn set_clock(dir: &Path, at: Reading) {
    fs::write(dir.join("clock.new"), inbox_server::format_reading(at)).expect("written");
    fs::rename(dir.join("clock.new"), dir.join("clock")).expect("replaced");
}

/// `docs/design/service.md`, the expiry: "A due row does not end a wait on `Pending`, so a wake
/// loop of `wait` and `read` does not get the row it cannot read back at once, again and again,
/// until the sweep." `inbox wake` reads `Pending` with `wait=0` first, which answers every row,
/// due ones included, and returns on a note it can neither read nor mark: its `MarkDelivered` is
/// refused `Expired`, exit 1, and the next wake returns on it again at once. A real `inbox serve`,
/// its clock a file the case moves past the note's `expires`, its sweep an hour apart.
#[test]
fn a_due_note_alone_does_not_end_the_wake() {
    let (dir, operator) = keys_only("adv-wake-due");
    set_clock(&dir.path, this_second());
    let service = Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--max-wait",
            "60",
            "--sweep-interval",
            "3600",
            "--clock-file",
            "clock",
        ],
        &[],
    );
    let client = |who: &str, url: &str, args: &[&str]| {
        let outbox = dir.join(&format!("{who}.outbox"));
        support::run(
            &dir.path,
            args,
            &[
                ("INBOX_URL", url),
                ("INBOX_SECRET_KEY_FILE", &format!("{who}.key")),
                ("INBOX_OUTBOX", outbox.to_str().expect("UTF-8")),
            ],
        )
    };
    client("bob", &service.url, &["open"]).ok();
    let expires_at = this_second().plus_seconds(30);
    let due = client(
        "alice",
        &service.url,
        &[
            "post",
            "--to",
            BOB,
            "--body",
            "soon due",
            "--expires",
            &format_instant(expires_at),
        ],
    )
    .ok()
    .stdout
    .trim()
    .to_owned();
    set_clock(&dir.path, expires_at.plus_seconds(1));

    let forwarder = Forwarder::start(service.url.trim_start_matches("http://"));
    let url = format!("http://{}", forwarder.address);
    let mut woken = wake(&dir.path, &url, "bob");
    wait_for("the wake holds its second read or ends", || {
        woken.exited().is_some() || forwarder.pending() >= 2
    });
    let ended = woken.exited();
    let live = client(
        "alice",
        &service.url,
        &["post", "--to", BOB, "--body", "a live note"],
    )
    .ok()
    .stdout
    .trim()
    .to_owned();
    let (code, stdout, stderr) = woken.output();
    assert_eq!(
        ended, None,
        "inbox wake ended on the due note {due} alone (exit {code}): {stdout} {stderr}"
    );
    assert!(stdout.contains(&live), "{stdout} {stderr}");
}

/// The rule of the story: "A closed standard output (the harness gone) ends the wake without a
/// `MarkDelivered`." Started with descriptor 1 closed (`inbox wake >&-`), the wake's write goes
/// nowhere and is answered as written (the standard library takes `EBADF` on standard output for
/// a success), so it marks the row delivered that no reader got, and exits 0. The stand-in
/// answers every request with one row in `Sent`; a mark is the second request.
#[test]
fn a_wake_started_with_standard_output_closed_marks_nothing() {
    let dir = bob_only("adv-wake-fd-closed");
    let id = format!("01J{:023}", 7);
    let row = format!(
        r#"{{"rows":[{{"message_id":"{id}","from":"agent:test/alice","on_behalf_of":"agent:test/alice","to":"{BOB}","kind":"note","thread":"{id}","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"{id}","body":"never seen","state":"Sent"}}]}}"#
    );
    let stand_in = StandIn::start(200, row);
    let mut command = Command::new("sh");
    command
        .args(["-c", "exec \"$0\" wake >&-", support::BIN])
        .current_dir(&dir.path)
        .env("HOME", &dir.path)
        .env("XDG_STATE_HOME", dir.join("state"))
        .env_remove("INBOX_TLS_PIN")
        .env("INBOX_URL", &stand_in.url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let (code, _, stderr) = Started(command.spawn().expect("sh starts")).output();
    let asked = stand_in.count();
    assert_eq!(
        asked, 0,
        "inbox wake with no standard output sent {asked} requests (exit {code}): {stderr}"
    );
    assert_eq!(code, LOCAL, "{stderr}");
}

/// `docs/design/wake.md`: "One line on standard error names the first failure of a run of
/// them", and "an answer resets" the pause. A run of failures, an answer, and a second run: two
/// lines. Green on the unit; red when the reset after an answer is dropped (the mutant
/// `failures = 0;` removed from `wake::held`, which the cases of `tests/wake.rs` let live).
#[test]
fn each_run_of_failures_after_an_answer_is_said_again() {
    let dir = bob_only("adv-wake-runs");
    let id = format!("01J{:023}", 8);
    let row = format!(
        r#"{{"rows":[{{"message_id":"{id}","from":"agent:test/alice","on_behalf_of":"agent:test/alice","to":"{BOB}","kind":"note","thread":"{id}","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"{id}","body":"after two runs","state":"Sent"}}]}}"#
    );
    let busy = (503, r#"{"refused":"the service is stopping"}"#.to_owned());
    let stand_in = StandIn::answering(vec![
        busy.clone(),
        (200, r#"{"rows":[]}"#.to_owned()),
        busy,
        (200, row),
        (200, r#"{"outcome":"delivered","published":[]}"#.to_owned()),
    ]);
    let (code, stdout, stderr) = wake(&dir.path, &stand_in.url, "bob").output();
    assert_eq!(code, 0, "{stdout} {stderr}");
    assert_eq!(stand_in.count(), 5, "{stderr}");
    assert_eq!(
        stderr.matches("trying again").count(),
        2,
        "one line per run of failures: {stderr}"
    );
}

/// `docs/design/wake.md`: SIGINT and SIGTERM end the wake "also when it was started with them
/// ignored". Started with both ignored (`trap "" INT TERM` before the `exec`), held in its first
/// read by a listener that never answers, SIGTERM ends it by the signal. Green on the unit; red
/// when `end_on_signal()` is not called (a mutant the cases of `tests/wake.rs` let live).
#[test]
fn a_wake_started_with_sigterm_ignored_still_ends_on_sigterm() {
    let dir = bob_only("adv-wake-ignored");
    let silent = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}", silent.local_addr().expect("an address"));
    let hash = BOB.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    });
    let place = dir.join(&format!("bob.outbox.wake.{hash:016x}"));
    let mut command = Command::new("sh");
    command
        .args(["-c", "trap '' INT TERM; exec \"$0\" wake", support::BIN])
        .current_dir(&dir.path)
        .env("HOME", &dir.path)
        .env("XDG_STATE_HOME", dir.join("state"))
        .env_remove("INBOX_TLS_PIN")
        .env("INBOX_URL", &url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut woken = Started(command.spawn().expect("sh starts"));
    // The handlers are set before the lock file is made.
    wait_for("the wake made its lock file", || place.exists());
    let pid = woken.0.id().to_string();
    assert!(
        Command::new("kill")
            .args(["-TERM", &pid])
            .status()
            .expect("kill runs")
            .success()
    );
    wait_for("the wake ends on SIGTERM", || woken.exited().is_some());
    let status = woken.0.wait().expect("waited for");
    assert_eq!(
        std::os::unix::process::ExitStatusExt::signal(&status),
        Some(libc::SIGTERM),
        "{status:?}"
    );
    drop(silent);
}

/// `docs/design/wake.md`: "The read without a wait names the epoch of the start that answers it
/// before the wake waits on that start ... So a wake that meets a restart opens the inbox again
/// before it waits on the new start." When the start changes between the two reads of a round
/// (the old start answers the read without a wait, the new one gets the held read), the wake
/// waits on the new start, where the inbox is not open, until that wait ends (up to 60 s): a post
/// meanwhile is refused `UnknownInbox`. The forwarder carries the wake's first connection to the
/// first start and every later one to the second. This window is the likely cause of the
/// acceptance case of `tests/wake.rs` failing under load ("the wake opened the inbox again at the
/// new start: not within 10 s", 2 of 5 runs of the package here).
#[test]
fn a_restart_between_the_two_reads_of_a_round_still_opens_the_inbox_again_first() {
    let (dir, operator) = keys_only("adv-wake-window");
    let start = || {
        Service::start(
            &dir.path,
            &[
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                &operator,
                "--max-wait",
                "60",
            ],
            &[],
        )
    };
    let first = start();
    let second = start();
    let outbox = dir.join("bob.outbox");
    let opened = support::run(
        &dir.path,
        &["open"],
        &[
            ("INBOX_URL", &first.url),
            ("INBOX_SECRET_KEY_FILE", "bob.key"),
            ("INBOX_OUTBOX", outbox.to_str().expect("UTF-8")),
        ],
    );
    assert_eq!(opened.status, 0, "{opened:#?}");
    let mut record_path = outbox.as_os_str().to_owned();
    record_path.push(".opened");
    let record = || fs::read_to_string(&record_path).unwrap_or_default();
    let opened_first = record();
    assert!(opened_first.contains("\"epoch\""), "{opened_first}");

    let forwarder = Forwarder::switching(
        first.url.trim_start_matches("http://"),
        Some(second.url.trim_start_matches("http://")),
    );
    let mut woken = wake(&dir.path, &format!("http://{}", forwarder.address), "bob");
    wait_for("the wake sent its held read", || forwarder.pending() >= 2);
    let started = Instant::now();
    while record() == opened_first && woken.exited().is_none() && started.elapsed() < HANG {
        std::thread::sleep(Duration::from_millis(10));
    }
    let now = record();
    drop(woken);
    assert_ne!(
        now,
        opened_first,
        "the wake held its read on the new start for {:?} without opening the inbox there",
        started.elapsed()
    );
}
