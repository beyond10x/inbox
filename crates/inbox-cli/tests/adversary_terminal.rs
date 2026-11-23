//! Adversary cases of story:the-terminal-adapter-shows-a-post (`inbox wake --follow`, the
//! adapter `terminal`), round 1. Some are red against the round: each names what it shows. The
//! others are green and each kills a mutant the cases of `terminal_adapter.rs` let live. No case
//! holds an upper bound of time: a bounded wait only ends a hang.

use crate::support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::process::ExitStatusExt;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use support::{ALICE, BOB, Dir, REFUSED, World};

/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

/// The terminal's bell.
const BEL: u8 = 0x07;

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

/// A started process whose standard output and standard error are read by threads; killed when
/// dropped unfinished.
struct Running {
    child: Option<Child>,
    stdout: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    readers: Vec<std::thread::JoinHandle<()>>,
}

impl Running {
    fn spawn(command: &mut Command) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| {
                panic!(
                    "{:?} does not start (a case on a terminal needs `script` of util-linux on \
                     PATH): {error}",
                    command.get_program()
                )
            });
        let stdout = Arc::new(Mutex::new(Vec::new()));
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let mut readers = Vec::new();
        if let Some(pipe) = child.stdout.take() {
            readers.push(read_into(pipe, Arc::clone(&stdout)));
        }
        if let Some(pipe) = child.stderr.take() {
            readers.push(read_into(pipe, Arc::clone(&stderr)));
        }
        Self {
            child: Some(child),
            stdout,
            stderr,
            readers,
        }
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("not finished yet").id()
    }

    fn stdout(&self) -> Vec<u8> {
        self.stdout.lock().expect("the bytes").clone()
    }

    fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout()).into_owned()
    }

    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr.lock().expect("the bytes")).into_owned()
    }

    fn running(&mut self) -> bool {
        matches!(self.child.as_mut().map(Child::try_wait), Some(Ok(None)))
    }

    fn end(mut self) -> (ExitStatus, Vec<u8>, String) {
        let started = Instant::now();
        let status = loop {
            let child = self.child.as_mut().expect("not finished yet");
            if let Some(status) = child.try_wait().expect("the process is asked") {
                break status;
            }
            assert!(
                started.elapsed() < HANG,
                "the process did not end within {} s: {}",
                HANG.as_secs(),
                self.stderr_text()
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        self.child = None;
        for reader in std::mem::take(&mut self.readers) {
            support::wait_until_finished(&reader, "a reader of a pipe");
            reader.join().expect("the reader ends");
        }
        let stderr = self.stderr_text();
        (status, self.stdout(), stderr)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn read_into(
    mut pipe: impl Read + Send + 'static,
    into: Arc<Mutex<Vec<u8>>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        while let Ok(read @ 1..) = pipe.read(&mut buffer) {
            into.lock()
                .expect("the bytes")
                .extend_from_slice(&buffer[..read]);
        }
    })
}

fn signal(pid: u32, name: &str) {
    let sent = Command::new("kill")
        .args([name, &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success(), "kill {name} {pid} failed");
}

fn post(world: &World, body: &str) -> String {
    world
        .client(
            "alice",
            &["post", "--to", BOB, "--kind", "note", "--body", body],
        )
        .ok()
        .stdout
        .trim()
        .to_owned()
}

fn state(world: &World, message_id: &str) -> String {
    let listed = world.client("bob", &["pending", "--json"]).ok();
    support::rows(&listed.json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

fn bells(bytes: &[u8]) -> usize {
    bytes.iter().filter(|byte| **byte == BEL).count()
}

/// `script -q -e -c "exec inbox <args>" /dev/null` with the environment and directory of bob's
/// client in `world`: the binary on a pseudo terminal (as `terminal_adapter.rs` runs it).
fn on_a_terminal(world: &World, args: &str) -> Command {
    let inner = world.client_command("bob", &[]);
    let mut script = Command::new("script");
    script
        .args(["-q", "-e", "-c"])
        .arg(format!("exec '{}' {args}", support::BIN))
        .arg("/dev/null")
        .env("SHELL", "/bin/sh")
        .stdin(Stdio::null());
    if let Some(dir) = inner.get_current_dir() {
        script.current_dir(dir);
    }
    for (name, value) in inner.get_envs() {
        match value {
            Some(value) => script.env(name, value),
            None => script.env_remove(name),
        };
    }
    script
}

/// The pid of the child of `parent` that runs the binary.
fn child_running_inbox(parent: u32) -> u32 {
    let mut found = None;
    wait_for("the binary runs under script", || {
        let children = std::fs::read_to_string(format!("/proc/{parent}/task/{parent}/children"))
            .unwrap_or_default();
        found = children
            .split_whitespace()
            .filter_map(|pid| pid.parse::<u32>().ok())
            .find(|pid| {
                std::fs::read_to_string(format!("/proc/{pid}/comm"))
                    .is_ok_and(|comm| comm.trim_end() == "inbox")
            });
        found.is_some()
    });
    found.expect("a child")
}

/// A round of 2 rows rings one bell, not one per row: both notes are posted before the adapter
/// starts, so its first read holds both. Kills the mutant "the BEL once per row", which the bell
/// case of `terminal_adapter.rs` (one row per round) lets live.
#[test]
fn a_round_of_two_rows_on_a_terminal_rings_one_bell() {
    let world = World::new("adv-terminal-two-rows");
    world.client("bob", &["open"]).ok();
    let first = post(&world, "one");
    let second = post(&world, "two");
    let terminal = Running::spawn(&mut on_a_terminal(&world, "wake --follow"));
    let wake = child_running_inbox(terminal.pid());
    wait_for("both are delivered", || {
        state(&world, &first) == "Delivered" && state(&world, &second) == "Delivered"
    });
    wait_for("the round rings", || bells(&terminal.stdout()) >= 1);
    let shown = terminal.stdout_text();
    assert!(
        shown.contains(&first) && shown.contains(&second),
        "{shown:?}"
    );
    signal(wake, "-TERM");
    let (_, stdout, stderr) = terminal.end();
    assert_eq!(
        bells(&stdout),
        1,
        "one round, one bell: {stdout:?} {stderr}"
    );
}

/// `inbox wake --follow --json` on a terminal writes JSON lines and no bell: the design says no
/// bell with `--json`. Kills the mutant "the BEL in --json", which no case of
/// `terminal_adapter.rs` sees (its `--json` case writes to a pipe).
#[test]
fn json_follow_on_a_terminal_rings_no_bell() {
    let world = World::new("adv-terminal-json-bell");
    world.client("bob", &["open"]).ok();
    let terminal = Running::spawn(&mut on_a_terminal(&world, "--json wake --follow"));
    let wake = child_running_inbox(terminal.pid());
    let message = post(&world, "in json");
    wait_for("the document is shown", || {
        terminal.stdout_text().contains(&message)
    });
    wait_for("it is delivered", || state(&world, &message) == "Delivered");
    signal(wake, "-TERM");
    let (_, stdout, stderr) = terminal.end();
    assert!(String::from_utf8_lossy(&stdout).contains(&message));
    assert_eq!(
        bells(&stdout),
        0,
        "no bell with --json: {stdout:?} {stderr}"
    );
}

/// `inbox wake` without the flag on a terminal rings no bell: it is unchanged. Kills the mutant
/// "the bell in `try_print_text`", which every case writing to a pipe or a file lets live.
#[test]
fn the_plain_wake_on_a_terminal_rings_no_bell() {
    let world = World::new("adv-terminal-plain-bell");
    world.client("bob", &["open"]).ok();
    let message = post(&world, "for the plain wake");
    let (status, stdout, stderr) = Running::spawn(&mut on_a_terminal(&world, "wake")).end();
    assert_eq!(status.code(), Some(0), "{stdout:?} {stderr}");
    assert!(String::from_utf8_lossy(&stdout).contains(&message));
    assert_eq!(bells(&stdout), 0, "{stdout:?}");
}

/// A sender's bidirectional override (U+202E) and isolate (U+2066) reach the terminal raw: they
/// are format characters, not control characters, so `visible` keeps them, and they reorder
/// what the owner sees in the line. The brief names a bidirectional override among the texts a
/// sender must not put on the owner's terminal. Red against the round (and against `inbox
/// wake`, `inbox pending`: the printer is theirs).
#[test]
fn a_senders_bidirectional_override_does_not_reach_the_terminal_raw() {
    let world = World::new("adv-terminal-bidi");
    world.client("bob", &["open"]).ok();
    let mut adapter = Running::spawn(&mut world.client_command("bob", &["wake", "--follow"]));
    let message = post(&world, "pay \u{202e}0001\u{202c} now \u{2066}x\u{2069} end");
    wait_for("the note is shown", || {
        adapter.stdout_text().contains(" end")
    });
    assert!(adapter.running(), "{}", adapter.stderr_text());
    signal(adapter.pid(), "-TERM");
    let (_, stdout, _) = adapter.end();
    let text = String::from_utf8_lossy(&stdout).into_owned();
    assert!(text.contains(&message), "{text}");
    for raw in ['\u{202e}', '\u{202c}', '\u{2066}', '\u{2069}'] {
        assert!(
            !text.contains(raw),
            "U+{:04X} reached the terminal raw: {text:?}",
            u32::from(raw)
        );
    }
}

/// The record of this host changes to `claude_code` while the adapter runs (the owner follows the
/// first-use page for Claude Code: `inbox open --adapter claude_code`). The design says the check
/// of the record exists so that "a terminal does not take the lock of a Claude Code session's
/// plugin, whose `inbox wake` would then exit 5 and never wake the session". The adapter checks
/// the record once, at its start: it keeps the lock, shows and marks delivered the next post,
/// and the plugin's `inbox wake` exits 5, which the plugin takes silently. Red against the round:
/// the adapter goes on.
#[test]
fn the_adapter_does_not_keep_the_lock_once_the_inbox_is_opened_with_another_adapter() {
    let world = World::new("adv-terminal-reopened");
    world.client("bob", &["open"]).ok();
    let adapter = Running::spawn(&mut world.client_command("bob", &["wake", "--follow"]));
    let first = post(&world, "before the switch");
    wait_for("the first note is shown", || {
        adapter.stdout_text().contains(&first)
    });
    wait_for("the first note is delivered", || {
        state(&world, &first) == "Delivered"
    });
    world
        .client("bob", &["open", "--adapter", "claude_code"])
        .ok();
    let second = post(&world, "for the Claude Code session");
    // The adapter ends first, by itself: nothing else orders its end before the plain wake
    // reaches the lock (adversary pass 2, finding 1). An adapter that goes on fails here, at the
    // bound of `end`.
    let (status, shown, stderr) = adapter.end();
    assert_eq!(status.code(), Some(REFUSED), "{stderr}");
    assert!(
        !String::from_utf8_lossy(&shown).contains(&second),
        "the terminal adapter showed the post of the Claude Code session"
    );
    let (plain, rows, plain_stderr) =
        Running::spawn(&mut world.client_command("bob", &["wake"])).end();
    assert_eq!(
        plain.code(),
        Some(0),
        "the plugin's wake after the terminal adapter ended: {plain_stderr}"
    );
    assert!(
        String::from_utf8_lossy(&rows).contains(&second),
        "the plugin's wake shows the post: {plain_stderr}"
    );
}

/// A request a stand-in read, with the instant it arrived.
#[derive(Debug, Clone)]
struct Seen {
    line: String,
    at: Instant,
}

fn read_line(stream: &mut TcpStream) -> String {
    let _ = stream.set_read_timeout(Some(HANG));
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected = None;
    while !expected.is_some_and(|total| received.len() >= total) {
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

/// A stand-in that answers each read with the same row in `Sent` and each mark with `mark`
/// (status and body), and `inbox wake --follow` of bob against it until it has read 4 times:
/// the gaps between the reads, and how many times the row was shown.
fn reads_of_a_row_that_stays_in_sent(
    name: &str,
    mark: (u16, &'static str),
) -> (Vec<Duration>, usize) {
    let dir = Dir::new(name);
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (kept, stopped) = (Arc::clone(&seen), Arc::clone(&stop));
    let id = "01J00000000000000000000001";
    let row = format!(
        r#"{{"message_id":"{id}","from":"{ALICE}","on_behalf_of":"{ALICE}","to":"{BOB}","kind":"note","thread":"{id}","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"{id}","body":"again","state":"Sent"}}"#
    );
    let server = std::thread::spawn(move || {
        let mut idle = Instant::now();
        while !stopped.load(Ordering::SeqCst) && idle.elapsed() < HANG {
            let Some(mut stream) = support::accept_within(&listener, Duration::from_millis(20))
            else {
                continue;
            };
            idle = Instant::now();
            let line = read_line(&mut stream);
            let (status, answer) = if line.starts_with("GET ") {
                (200, format!(r#"{{"rows":[{row}]}}"#))
            } else {
                (mark.0, mark.1.to_owned())
            };
            kept.lock().expect("the requests").push(Seen {
                line,
                at: Instant::now(),
            });
            let text = format!(
                "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                answer.len()
            );
            let _ = stream.write_all(text.as_bytes());
        }
    });
    let mut command = support::command(&dir.path);
    command
        .args(["wake", "--follow"])
        .env("INBOX_URL", &url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", dir.join("bob.outbox"));
    let adapter = Running::spawn(&mut command);
    let reads = |seen: &[Seen]| -> Vec<Instant> {
        seen.iter()
            .filter(|request| request.line.starts_with("GET "))
            .map(|request| request.at)
            .collect()
    };
    wait_for("4 reads", || {
        reads(&seen.lock().expect("the requests")).len() >= 4
    });
    signal(adapter.pid(), "-TERM");
    let (_, stdout, _) = adapter.end();
    stop.store(true, Ordering::SeqCst);
    support::wait_until_finished(&server, "the stand-in");
    let at = reads(&seen.lock().expect("the requests"));
    let shown = String::from_utf8_lossy(&stdout)
        .lines()
        .filter(|line| line.starts_with(id))
        .count();
    (at.windows(2).map(|pair| pair[1] - pair[0]).collect(), shown)
}

/// A service that answers each mark `200 delivered` and the next read with the row still in
/// `Sent`. The adapter shows the row and marks it, again and again, as fast as the loopback
/// answers: no mark failed, so no round is followed by a pause. The case asserts the design's
/// floor, that a row which comes back is read again only after the first pause (100 ms); a
/// lower bound only, a slow host makes the gaps longer. Red against the round. Reached by a
/// service that does not keep its mark (nothing found in this repository's service).
#[test]
fn a_row_still_in_sent_after_a_mark_that_succeeded_is_not_read_again_at_once() {
    let (gaps, shown) = reads_of_a_row_that_stays_in_sent(
        "adv-terminal-spin",
        (200, r#"{"outcome":"delivered","published":[]}"#),
    );
    assert!(
        gaps[1..3]
            .iter()
            .all(|gap| *gap >= Duration::from_millis(100)),
        "reads of the same row at once, the row shown {shown} times: {gaps:?}"
    );
}

/// A service that refuses every mark (`UnknownMessage`) and keeps answering the row: the design
/// says the next round starts after the pause of a failure, "so a service that keeps answering a
/// row it refuses to mark does not make it spin". Green; kills the mutant "no pause after a
/// failed mark", which the cases of `terminal_adapter.rs` let live. A lower bound only.
#[test]
fn a_row_whose_mark_is_refused_is_read_again_only_after_a_pause() {
    let (gaps, shown) = reads_of_a_row_that_stays_in_sent(
        "adv-terminal-refused-mark",
        (
            404,
            r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#,
        ),
    );
    assert!(
        gaps.iter().all(|gap| *gap >= Duration::from_millis(100)),
        "the row shown {shown} times: {gaps:?}"
    );
}

/// `--follow` over a record of another adapter with standard output `/dev/null`: the design says
/// the check of standard output comes first ("exit 4 first of all"), then the adapter of the
/// record. Green; kills the mutant "the adapter check before the check of standard output".
#[test]
fn a_standard_output_no_one_reads_exits_4_before_the_check_of_the_adapter() {
    let world = World::new("adv-terminal-null-other");
    world
        .client("bob", &["open", "--adapter", "claude_code"])
        .ok();
    let output = world
        .client_command("bob", &["wake", "--follow"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .expect("the binary starts");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(support::LOCAL), "{stderr}");
    assert!(stderr.contains("standard output"), "{stderr}");
}

/// SIGTERM during the held read of a second round ends the adapter with the status of the signal
/// and leaves no lock behind: a plain `inbox wake` after it takes the lock and returns on a post.
/// Green; a check of the design's "the kernel releases the lock".
#[test]
fn sigterm_during_a_held_read_leaves_no_lock_behind() {
    let world = World::new("adv-terminal-held");
    world.client("bob", &["open"]).ok();
    let adapter = Running::spawn(&mut world.client_command("bob", &["wake", "--follow"]));
    let first = post(&world, "first round");
    wait_for("the first round is delivered", || {
        state(&world, &first) == "Delivered"
    });
    signal(adapter.pid(), "-TERM");
    let (status, stdout, stderr) = adapter.end();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{stderr}");
    let text = String::from_utf8_lossy(&stdout).into_owned();
    let heads = text.lines().filter(|line| line.starts_with(&first)).count();
    assert_eq!(heads, 1, "{text}");
    let next = world.spawn("bob", &["wake"]);
    let message = post(&world, "after the adapter");
    let run = next.finish();
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(run.stdout.contains(&message), "{}", run.stdout);
}
