//! The cases of story:the-terminal-adapter-shows-a-post: `inbox wake --follow`, the terminal
//! adapter, shows each message posted to the caller's inbox while it runs, one head line per
//! message, marks it delivered after it was written, and waits again; it ends only by a signal.
//! It rings the terminal's bell once per round when its standard output is a terminal, and never
//! when it is not. An inbox this host opened with another adapter ends it with 1 before the lock
//! and before any request. No case holds an upper bound of time: a bounded wait only ends a hang.
//! Every key is made at run time with the binary itself.

use crate::support;

use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use inbox::server::json;

use support::{ALICE, BOB, Dir, REFUSED, World};

/// The exit status of a second `inbox wake` of the same inbox on the same host.
const ANOTHER_WAKE: i32 = 5;

/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

/// The terminal's bell.
const BEL: u8 = 0x07;

/// What the case on a terminal needs: `script -q -e -c` makes its pseudo terminal, and it reads
/// Linux `/proc` to find the binary under it. A host without it fails the case with this name.
const SCRIPT: &str = "`script` of util-linux (`script -q -e -c`)";

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

/// A process this case started, whose standard output (unless the case gave it a file) and
/// standard error are pipes that threads read as the bytes arrive; killed and waited for when
/// dropped unfinished.
struct Running {
    child: Option<Child>,
    stdout: Arc<Mutex<Vec<u8>>>,
    stderr: Arc<Mutex<Vec<u8>>>,
    readers: Vec<std::thread::JoinHandle<()>>,
}

impl Running {
    /// Starts `command` with its standard output a pipe.
    fn spawn(command: &mut Command) -> Self {
        Self::start(command.stdout(Stdio::piped()))
    }

    /// Starts `command` with its standard output the file `stdout`.
    fn spawn_to(command: &mut Command, stdout: File) -> Self {
        Self::start(command.stdout(stdout))
    }

    /// [`Running::spawn`], answering the error when the program cannot be started.
    fn try_spawn(command: &mut Command) -> std::io::Result<Self> {
        Self::try_start(command.stdout(Stdio::piped()))
    }

    fn start(command: &mut Command) -> Self {
        Self::try_start(command).expect("the process starts")
    }

    fn try_start(command: &mut Command) -> std::io::Result<Self> {
        let mut child = command.stderr(Stdio::piped()).spawn()?;
        let stdout = Arc::new(Mutex::new(Vec::new()));
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let mut readers = Vec::new();
        if let Some(pipe) = child.stdout.take() {
            readers.push(read_into(pipe, Arc::clone(&stdout)));
        }
        if let Some(pipe) = child.stderr.take() {
            readers.push(read_into(pipe, Arc::clone(&stderr)));
        }
        Ok(Self {
            child: Some(child),
            stdout,
            stderr,
            readers,
        })
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("not finished yet").id()
    }

    /// The bytes it wrote to its standard output pipe so far.
    fn stdout(&self) -> Vec<u8> {
        self.stdout.lock().expect("the bytes").clone()
    }

    fn stdout_text(&self) -> String {
        String::from_utf8_lossy(&self.stdout()).into_owned()
    }

    fn stderr_text(&self) -> String {
        String::from_utf8_lossy(&self.stderr.lock().expect("the bytes")).into_owned()
    }

    /// Whether it still runs.
    fn running(&mut self) -> bool {
        matches!(self.child.as_mut().map(Child::try_wait), Some(Ok(None)))
    }

    fn signal(&self, name: &str) {
        signal(self.pid(), name);
    }

    /// Waits for its end ([`HANG`] only ends a hang), then for the end of its pipes, and answers
    /// its exit status, its standard output and its standard error.
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

/// A thread that appends what `pipe` gives to `into` until it ends.
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

/// `kill <name> <pid>`.
fn signal(pid: u32, name: &str) {
    let sent = Command::new("kill")
        .args([name, &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success(), "kill {name} {pid} failed");
}

/// `inbox wake --follow <args>` of `who` in `world`, not started.
fn follow(world: &World, who: &str, args: &[&str]) -> Command {
    let mut command = world.client_command(who, &["wake", "--follow"]);
    command.args(args);
    command
}

/// alice posts a message of `kind` with `body` to bob (a request takes a free reply); answers
/// its message id.
fn post(world: &World, kind: &str, body: &str) -> String {
    let mut args = vec!["post", "--to", BOB, "--kind", kind, "--body", body];
    if kind == "request" {
        args.extend(["--reply-shape", "free"]);
    }
    world.client("alice", &args).ok().stdout.trim().to_owned()
}

/// The state of the message `message_id` in bob's `Pending`, or an empty text when it is not
/// listed.
fn state(world: &World, message_id: &str) -> String {
    let listed = world.client("bob", &["pending", "--json"]).ok();
    support::rows(&listed.json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// The lines of `text` that are not indented by at least 2 spaces: one head line per message.
fn unindented(text: &str) -> Vec<&str> {
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.starts_with("  "))
        .collect()
}

/// The head line `inbox pending` prints for a message of alice to bob in `Sent`, up to its
/// thread.
fn head(message_id: &str, kind: &str) -> String {
    format!("{message_id} {kind} Sent from {ALICE} to {BOB} thread ")
}

/// Whether `text` holds a head line of `message_id` of `kind`.
fn shows(text: &str, message_id: &str, kind: &str) -> bool {
    unindented(text)
        .iter()
        .any(|line| line.starts_with(&head(message_id, kind)))
}

/// The acceptance of the story: a message posted while `inbox wake --follow` runs is shown on
/// its standard output in a head line naming the sender and the kind, and is `Delivered`, with no
/// typed command, and the adapter still runs; a second message is shown and delivered too, the
/// first is not shown again. SIGTERM ends it as it ends `inbox wake`. Its standard output is a
/// pipe, which gets no bell.
#[test]
fn a_post_is_shown_by_the_terminal_adapter_and_delivered_and_it_keeps_running() {
    let world = World::new("terminal-acceptance");
    world.client("bob", &["open"]).ok();
    let mut adapter = Running::spawn(&mut follow(&world, "bob", &[]));

    let first = post(&world, "request", "Which build is green?");
    wait_for("the request is shown", || {
        shows(&adapter.stdout_text(), &first, "request")
    });
    wait_for("the request is delivered", || {
        state(&world, &first) == "Delivered"
    });
    assert!(adapter.running(), "{}", adapter.stderr_text());

    let second = post(&world, "note", "Build 7 is green.");
    wait_for("the note is shown", || {
        shows(&adapter.stdout_text(), &second, "note")
    });
    wait_for("the note is delivered", || {
        state(&world, &second) == "Delivered"
    });
    assert!(adapter.running(), "{}", adapter.stderr_text());
    assert_eq!(state(&world, &first), "Delivered");

    adapter.signal("-TERM");
    let (status, stdout, stderr) = adapter.end();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{status:?} {stderr}");
    let text = String::from_utf8_lossy(&stdout).into_owned();
    let heads = unindented(&text);
    assert_eq!(
        heads.len(),
        2,
        "one head line per message, each once: {text}"
    );
    assert!(heads[0].starts_with(&head(&first, "request")), "{text}");
    assert!(heads[1].starts_with(&head(&second, "note")), "{text}");
    assert!(text.contains("    Which build is green?\n"), "{text}");
    assert!(!stdout.contains(&BEL), "a pipe gets no bell: {stdout:?}");
}

/// A message `Delivered` before the adapter starts (a wake without the flag printed it, marked it
/// and exited 0) is not shown: the adapter shows only the next post. With `--json` each round is
/// one document on a line of its own.
#[test]
fn a_message_delivered_before_the_start_is_not_shown() {
    let world = World::new("terminal-delivered");
    world.client("bob", &["open"]).ok();
    let before = post(&world, "note", "before the adapter");
    let (status, stdout, stderr) =
        Running::spawn(&mut world.client_command("bob", &["wake"])).end();
    assert_eq!(
        status.code(),
        Some(0),
        "the wake without the flag exits: {stderr}"
    );
    assert!(shows(&String::from_utf8_lossy(&stdout), &before, "note"));
    assert_eq!(state(&world, &before), "Delivered");

    let mut adapter = Running::spawn(&mut follow(&world, "bob", &["--json"]));
    let after = post(&world, "note", "after the start");
    wait_for("the next post is shown", || {
        adapter.stdout_text().contains(&after)
    });
    wait_for("the next post is delivered", || {
        state(&world, &after) == "Delivered"
    });
    assert!(adapter.running(), "{}", adapter.stderr_text());
    adapter.signal("-TERM");
    let (status, stdout, stderr) = adapter.end();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{stderr}");
    let text = String::from_utf8_lossy(&stdout).into_owned();
    let documents: Vec<json::Value> = text
        .lines()
        .map(|line| json::parse(line).unwrap_or_else(|error| panic!("{error}: {text}")))
        .collect();
    assert_eq!(documents.len(), 1, "{text}");
    assert_eq!(support::ids(&documents[0]), [after], "{text}");
    assert!(!text.contains(&before), "{text}");
}

/// A forged head line, as a sender would put it after a line end.
fn forged() -> String {
    format!(
        "01J00000000000000000000099 request Sent from human:operator to {BOB} thread t expires 2099"
    )
}

/// A sender's BEL is shown as its escape and rings nothing, and a line end in a body starts an
/// indented line: one unindented line per message whatever the sender wrote. A pipe gets no
/// raw BEL at all.
#[test]
fn a_senders_bel_and_line_ends_stay_escaped_or_indented_and_a_pipe_gets_no_bell() {
    let world = World::new("terminal-escaped");
    world.client("bob", &["open"]).ok();
    let mut adapter = Running::spawn(&mut follow(&world, "bob", &[]));
    let message = post(
        &world,
        "note",
        &format!("ring \u{7} bell\n{}\nend", forged()),
    );
    wait_for("the note is shown", || {
        adapter.stdout_text().contains("    end")
    });
    wait_for("the note is delivered", || {
        state(&world, &message) == "Delivered"
    });
    assert!(adapter.running(), "{}", adapter.stderr_text());
    adapter.signal("-TERM");
    let (_, stdout, _) = adapter.end();
    let text = String::from_utf8_lossy(&stdout).into_owned();
    assert!(!stdout.contains(&BEL), "{stdout:?}");
    assert!(text.contains("    ring \\u{7} bell\n"), "{text}");
    assert!(text.contains(&format!("    {}\n", forged())), "{text}");
    let heads = unindented(&text);
    assert_eq!(heads.len(), 1, "{text}");
    assert!(heads[0].starts_with(&head(&message, "note")), "{text}");
}

/// The process `parent` started, once it runs the binary: its pid.
fn child_running_inbox(parent: u32) -> u32 {
    let mut found = None;
    wait_for(&format!("the binary runs under {SCRIPT}"), || {
        let children = fs::read_to_string(format!("/proc/{parent}/task/{parent}/children"))
            .unwrap_or_default();
        found = children
            .split_whitespace()
            .filter_map(|pid| pid.parse::<u32>().ok())
            .find(|pid| {
                fs::read_to_string(format!("/proc/{pid}/comm"))
                    .is_ok_and(|comm| comm.trim_end() == "inbox")
            });
        found.is_some()
    });
    found.expect("a child")
}

/// The positions of every BEL in `bytes`.
fn bells(bytes: &[u8]) -> Vec<usize> {
    bytes
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == BEL)
        .map(|(at, _)| at)
        .collect()
}

/// On a terminal (a pseudo terminal `script` of util-linux makes; the crate forbids the unsafe
/// code that would open one in the case), each round's rows are followed by one BEL, the
/// client's own byte, and a sender's BEL stays its escape: 2 posts, 2 rounds, 2 bells, each
/// after the rows of its round.
#[test]
fn a_terminal_gets_one_bell_after_the_rows_of_each_round() {
    let world = World::new("terminal-bell");
    world.client("bob", &["open"]).ok();
    let inner = follow(&world, "bob", &[]);
    let mut script = Command::new("script");
    script
        .args(["-q", "-e", "-c"])
        .arg(format!("exec '{}' wake --follow", support::BIN))
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
    let terminal = Running::try_spawn(&mut script).unwrap_or_else(|error| {
        panic!("this case needs {SCRIPT} on PATH, which makes its pseudo terminal: {error}")
    });
    let wake = child_running_inbox(terminal.pid());

    let first = post(&world, "note", "ring \u{7} once");
    wait_for("the first round rings", || {
        bells(&terminal.stdout()).len() == 1
    });
    let shown = terminal.stdout();
    let rung = bells(&shown);
    let before = String::from_utf8_lossy(&shown[..rung[0]]).into_owned();
    assert!(
        shows(&before, &first, "note"),
        "the rows come before the bell: {before:?}"
    );
    assert!(before.contains("ring \\u{7} once"), "{before:?}");
    assert!(before.ends_with('\n'), "{before:?}");
    wait_for("the first post is delivered", || {
        state(&world, &first) == "Delivered"
    });

    let second = post(&world, "note", "and again");
    wait_for("the second round rings", || {
        bells(&terminal.stdout()).len() == 2
    });
    let shown = terminal.stdout();
    let rung = bells(&shown);
    let between = String::from_utf8_lossy(&shown[rung[0] + 1..rung[1]]).into_owned();
    assert!(shows(&between, &second, "note"), "{between:?}");
    assert!(!shows(&between, &first, "note"), "{between:?}");

    signal(wake, "-TERM");
    let (_, stdout, stderr) = terminal.end();
    assert_eq!(
        bells(&stdout).len(),
        2,
        "one bell per round: {stdout:?} {stderr}"
    );
}

/// A message id made from `n`: 26 characters of the model's alphabet.
fn id(n: u32) -> String {
    format!("01J{n:023}")
}

/// A row of `Pending` for bob, from alice, in `Sent`, with `body`.
fn row(n: u32, body: &str) -> String {
    let id = id(n);
    format!(
        r#"{{"message_id":"{id}","from":"{ALICE}","on_behalf_of":"{ALICE}","to":"{BOB}","kind":"note","thread":"{id}","created":"2026-11-23T00:00:00Z","expires":"2099-01-01T00:00:00Z","dedup_key":"{id}","body":"{body}","state":"Sent"}}"#
    )
}

/// One request a [`StandIn`] read: its request line, its body, and what the file of the
/// adapter's standard output held when it arrived.
#[derive(Debug, Clone)]
struct Request {
    line: String,
    body: String,
    stdout: String,
}

impl Request {
    fn is_pending(&self) -> bool {
        self.line.starts_with("GET ") && self.line.contains("/pending?")
    }

    fn is_mark(&self) -> bool {
        self.line.starts_with("POST ") && self.line.contains("/mark-delivered ")
    }
}

/// Reads one request in full: the head, then as many bytes as `Content-Length` says.
fn read_request(stream: &mut TcpStream) -> (String, String) {
    let _ = stream.set_read_timeout(Some(HANG));
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected = None;
    let mut head_end = None;
    while !expected.is_some_and(|total| received.len() >= total) {
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

/// How a [`StandIn`] answers a request, given the requests before it: the status and the body.
type Answering = dyn FnMut(&[Request], &Request) -> (u16, String) + Send;

/// A stand-in for the service on 127.0.0.1:0 that answers each request, one per connection, as
/// `answering` says, and keeps each with what the file `stdout` held when it arrived. It ends
/// when it is dropped, or when no connection comes for [`HANG`], which only ends a hang.
struct StandIn {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
    stop: Arc<AtomicBool>,
}

impl StandIn {
    fn start(stdout: PathBuf, mut answering: Box<Answering>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (kept, stopped) = (Arc::clone(&requests), Arc::clone(&stop));
        std::thread::spawn(move || {
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
                    stdout: fs::read_to_string(&stdout).unwrap_or_default(),
                };
                let before = kept.lock().expect("the requests").clone();
                let (status, answer) = answering(&before, &request);
                kept.lock().expect("the requests").push(request);
                let text = format!(
                    "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                    answer.len()
                );
                let _ = stream.write_all(text.as_bytes());
            }
        });
        Self {
            url,
            requests,
            stop,
        }
    }

    fn requests(&self) -> Vec<Request> {
        self.requests.lock().expect("the requests").clone()
    }
}

impl Drop for StandIn {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// `inbox wake <args>` of bob in `dir` against `url` with the outbox file `outbox`, not started.
fn wake(dir: &Path, url: &str, outbox: &Path, args: &[&str]) -> Command {
    let mut command = support::command(dir);
    command
        .arg("wake")
        .args(args)
        .env("INBOX_URL", url)
        .env("INBOX_SECRET_KEY_FILE", "bob.key")
        .env("INBOX_OUTBOX", outbox);
    command
}

/// Each round writes its rows to standard output, and flushes them, before its first mark (the
/// stand-in reads the file of standard output as each mark arrives). A mark that fails (here
/// refused `UnknownMessage`) is named in one line on standard error and the adapter goes on: it
/// reads again, shows the next row, marks it, and reads again after it. A file gets no bell.
#[test]
fn each_round_writes_its_rows_before_its_marks_and_a_failed_mark_does_not_end_the_adapter() {
    let dir = Dir::new("terminal-order");
    support::key_new(&dir.path, "bob.key", Some(("bob-1", BOB)));
    let stdout = dir.join("adapter.out");
    let (one, two) = (id(1), id(2));
    let (first, second) = (one.clone(), two.clone());
    let stand_in = StandIn::start(
        stdout.clone(),
        Box::new(move |before, request| {
            let marked = |message_id: &str| {
                before
                    .iter()
                    .any(|seen| seen.is_mark() && seen.body.contains(message_id))
            };
            if request.is_pending() {
                let rows = if !marked(&first) {
                    row(1, "first")
                } else if !marked(&second) {
                    row(2, "second")
                } else {
                    String::new()
                };
                (200, format!(r#"{{"rows":[{rows}]}}"#))
            } else if request.body.contains(&first) {
                (
                    404,
                    r#"{"outcome":"unknown-message","published":[],"error":"inbox.messaging.UnknownMessage"}"#
                        .to_owned(),
                )
            } else {
                (200, r#"{"outcome":"delivered","published":[]}"#.to_owned())
            }
        }),
    );
    let mut adapter = Running::spawn_to(
        &mut wake(
            &dir.path,
            &stand_in.url,
            &dir.join("bob.outbox"),
            &["--follow"],
        ),
        File::create(&stdout).expect("the stdout file is created"),
    );
    wait_for("a read after the second mark", || {
        let requests = stand_in.requests();
        requests
            .iter()
            .position(|request| request.is_mark() && request.body.contains(&two))
            .is_some_and(|at| requests[at + 1..].iter().any(Request::is_pending))
    });
    assert!(adapter.running(), "{}", adapter.stderr_text());
    let requests = stand_in.requests();
    let marks: Vec<&Request> = requests
        .iter()
        .filter(|request| request.is_mark())
        .collect();
    assert_eq!(marks.len(), 2, "{requests:#?}");
    assert!(marks[0].body.contains(&one), "{requests:#?}");
    assert!(shows(&marks[0].stdout, &one, "note"), "{requests:#?}");
    assert!(!marks[0].stdout.contains(&two), "{requests:#?}");
    assert!(marks[1].body.contains(&two), "{requests:#?}");
    assert!(shows(&marks[1].stdout, &two, "note"), "{requests:#?}");

    adapter.signal("-TERM");
    let (status, _, stderr) = adapter.end();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(lines[0].contains(&one), "{stderr}");
    assert!(lines[0].contains("UnknownMessage"), "{stderr}");
    let printed = fs::read(&stdout).expect("the stdout file is read");
    assert!(!printed.contains(&BEL), "a file gets no bell: {printed:?}");
    assert_eq!(
        unindented(&String::from_utf8_lossy(&printed)).len(),
        2,
        "{printed:?}"
    );
}

/// The files of `dir` whose name starts with `prefix`.
fn files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .expect("the directory is read")
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(prefix))
        .map(|entry| entry.path())
        .collect()
}

/// An inbox this host opened with another adapter (`claude_code`) ends `inbox wake --follow` with
/// 1 and one line naming `inbox open --adapter terminal`, before the lock and before any request
/// (`--json`: the document of the failure); `inbox wake` without the flag over the same record
/// waits and returns on a post as before. With no record the adapter does what `inbox wake`
/// does without one: an inbox not open at the service ends it with 1 and the line of the wake.
#[test]
fn an_inbox_opened_with_another_adapter_ends_the_terminal_adapter_with_1_and_no_request() {
    let world = World::new("terminal-other-adapter");
    world
        .client("bob", &["open", "--adapter", "claude_code"])
        .ok();
    let silent = TcpListener::bind("127.0.0.1:0").expect("a port");
    let silent_url = format!("http://{}", silent.local_addr().expect("an address"));
    let dir = &world.dir.path;
    let outbox = world.outbox("bob");
    for args in [&[][..], &["--json"][..]] {
        let (status, stdout, stderr) = Running::spawn(&mut wake(
            dir,
            &silent_url,
            &outbox,
            &[&["--follow"][..], args].concat(),
        ))
        .end();
        assert_eq!(status.code(), Some(REFUSED), "{args:?} {stderr}");
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(lines.len(), 1, "{args:?} {stderr}");
        assert!(lines[0].starts_with("inbox wake: "), "{stderr}");
        assert!(lines[0].contains("claude_code"), "{stderr}");
        assert!(
            lines[0].contains("`inbox open --adapter terminal`"),
            "{stderr}"
        );
        let stdout = String::from_utf8_lossy(&stdout).into_owned();
        if args.is_empty() {
            assert_eq!(stdout, "", "{stdout}");
        } else {
            let document = json::parse(stdout.trim()).expect("one JSON document");
            assert_eq!(
                support::text(&document, "error"),
                "another adapter",
                "{stdout}"
            );
        }
    }
    silent.set_nonblocking(true).expect("no blocking");
    assert!(
        matches!(silent.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "the adapter sent nothing"
    );
    assert_eq!(
        files(dir, "bob.outbox.wake"),
        Vec::<PathBuf>::new(),
        "no lock was taken"
    );

    let plain = Running::spawn(&mut world.client_command("bob", &["wake"]));
    let message = post(&world, "note", "for the plugin");
    let (status, stdout, stderr) = plain.end();
    assert_eq!(status.code(), Some(0), "{stderr}");
    assert!(shows(&String::from_utf8_lossy(&stdout), &message, "note"));

    let (status, stdout, stderr) = Running::spawn(&mut follow(&world, "alice", &[])).end();
    assert_eq!(status.code(), Some(REFUSED), "{stderr}");
    assert!(stdout.is_empty(), "{stdout:?}");
    assert!(
        stderr.contains("this host has no record of an `inbox open` of it"),
        "{stderr}"
    );
}

/// The record is read in every round, before the rows are written: once bob's inbox is opened
/// with `claude_code` while the adapter waits, the next post ends its round with 1 and the line
/// of the start, no row written and nothing marked. The post stays in `Sent`, and a wake without
/// the flag, as the plugin runs it, takes the free lock and returns on it.
#[test]
fn a_record_of_another_adapter_ends_the_adapter_at_its_next_round_without_a_row() {
    let world = World::new("terminal-switched");
    world.client("bob", &["open"]).ok();
    let adapter = Running::spawn(&mut follow(&world, "bob", &[]));
    let first = post(&world, "note", "before the switch");
    wait_for("the first note is shown", || {
        shows(&adapter.stdout_text(), &first, "note")
    });
    wait_for("the first note is delivered", || {
        state(&world, &first) == "Delivered"
    });
    world
        .client("bob", &["open", "--adapter", "claude_code"])
        .ok();
    let second = post(&world, "note", "for the Claude Code session");
    let (status, stdout, stderr) = adapter.end();
    assert_eq!(status.code(), Some(REFUSED), "{stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(lines[0].contains("claude_code"), "{stderr}");
    assert!(
        lines[0].contains("`inbox open --adapter terminal`"),
        "{stderr}"
    );
    let text = String::from_utf8_lossy(&stdout).into_owned();
    assert!(!text.contains(&second), "no row of the round: {text}");
    assert_eq!(unindented(&text).len(), 1, "{text}");
    assert_eq!(state(&world, &second), "Sent", "nothing is marked");

    let (status, stdout, stderr) =
        Running::spawn(&mut world.client_command("bob", &["wake"])).end();
    assert_eq!(status.code(), Some(0), "the lock is free: {stderr}");
    assert!(shows(&String::from_utf8_lossy(&stdout), &second, "note"));
}

/// The adapter takes the lock of `inbox wake`: while it runs, a second wake of bob with the same
/// outbox file, with the flag or without it, exits 5 and sends nothing. SIGINT ends the adapter
/// as it ends `inbox wake`.
#[test]
fn a_second_wake_while_the_terminal_adapter_runs_exits_5_and_sigint_ends_it() {
    let world = World::new("terminal-lock");
    world.client("bob", &["open"]).ok();
    let mut adapter = Running::spawn(&mut follow(&world, "bob", &[]));
    let message = post(&world, "note", "the adapter runs");
    wait_for("the note is shown", || {
        shows(&adapter.stdout_text(), &message, "note")
    });

    let silent = TcpListener::bind("127.0.0.1:0").expect("a port");
    let silent_url = format!("http://{}", silent.local_addr().expect("an address"));
    let outbox = world.outbox("bob");
    for args in [&["--follow"][..], &[][..]] {
        let (status, stdout, stderr) =
            Running::spawn(&mut wake(&world.dir.path, &silent_url, &outbox, args)).end();
        assert_eq!(status.code(), Some(ANOTHER_WAKE), "{args:?} {stderr}");
        assert!(stdout.is_empty(), "{stdout:?}");
        assert_eq!(
            stderr,
            format!("inbox wake: another wake of {BOB} runs on this host\n")
        );
    }
    silent.set_nonblocking(true).expect("no blocking");
    assert!(
        matches!(silent.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "the second wakes sent nothing"
    );
    assert!(adapter.running(), "{}", adapter.stderr_text());

    adapter.signal("-INT");
    let (status, _, stderr) = adapter.end();
    assert_eq!(status.signal(), Some(libc::SIGINT), "{stderr}");
}
