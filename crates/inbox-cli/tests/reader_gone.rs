//! The cases of story:a-wake-whose-reader-is-gone-marks-nothing: an `inbox wake` whose standard
//! output is a pipe or a FIFO ends with 4 when the reading end is closed while it waits, with or
//! without `--follow` and without a post to arrive first; it marks nothing and frees its lock,
//! so a post made after its end stays in `Sent` and the next wake returns on it. Rows written to
//! a pipe whose reader closes are not marked. A terminal and a regular file are not watched.
//! The hook of the Claude Code plugin, ended by SIGKILL, ends its wake the same way. Each case
//! runs the built binary against a real `inbox serve`; a bounded wait only ends a hang, and no
//! case holds an upper bound of time.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use crate::support;

use support::{BIN, BOB, LOCAL, World};

/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

/// The name of the thread of `inbox wake` that watches the reader of its standard output.
const WATCH_THREAD: &str = "inbox-reader";

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

/// A process this case started; killed and waited for when dropped unfinished.
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

    /// Waits for its end ([`HANG`] only ends a hang; `what` names the fact that should have ended
    /// it) and answers its exit status and what it wrote to its standard error pipe.
    fn finish(mut self, what: &str) -> (ExitStatus, String) {
        let started = Instant::now();
        while self
            .child()
            .try_wait()
            .expect("the process is asked")
            .is_none()
        {
            assert!(
                started.elapsed() < HANG,
                "inbox wake did not end within {} s after {what}",
                HANG.as_secs()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut child = self.0.take().expect("not finished yet");
        let status = child.wait().expect("the process is waited for");
        let mut stderr = String::new();
        if let Some(pipe) = child.stderr.as_mut() {
            pipe.read_to_string(&mut stderr).expect("stderr is UTF-8");
        }
        (status, stderr)
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

/// `inbox wake <args>` of bob in `world`, its standard error a pipe, not started.
fn wake(world: &World, args: &[&str]) -> Command {
    let mut all = vec!["wake"];
    all.extend_from_slice(args);
    let mut command = world.client_command("bob", &all);
    command.stderr(Stdio::piped());
    command
}

/// Whether the process `pid` holds a `flock` (`/proc/locks`): a wake holds the lock of its inbox.
fn holds_a_lock(pid: u32) -> bool {
    let pid = pid.to_string();
    fs::read_to_string("/proc/locks")
        .unwrap_or_default()
        .lines()
        .any(|line| {
            let fields: Vec<&str> = line.split_whitespace().collect();
            fields.get(1) == Some(&"FLOCK") && fields.get(4) == Some(&pid.as_str())
        })
}

/// The names of the threads of the process `pid` (`/proc/<pid>/task/<tid>/comm`).
fn threads(pid: u32) -> Vec<String> {
    fs::read_dir(format!("/proc/{pid}/task"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|task| fs::read_to_string(task.path().join("comm")).ok())
        .map(|name| name.trim_end().to_owned())
        .collect()
}

/// Whether the process `pid` runs the watch of its reader: a thread of that name.
fn watching(pid: u32) -> bool {
    threads(pid).iter().any(|name| name == WATCH_THREAD)
}

/// Waits until the wake `pid` holds its lock and runs the watch of its reader.
fn wait_for_the_watch(pid: u32) {
    wait_for("the wake holds its lock", || holds_a_lock(pid));
    wait_for("the wake watches its reader", || watching(pid));
}

/// alice posts a note with `body` to bob; answers its message id.
fn post(world: &World, body: &str) -> String {
    world
        .client("alice", &["post", "--to", BOB, "--body", body])
        .ok()
        .stdout
        .trim()
        .to_owned()
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

/// A wake that ended because its reader is gone: exit 4 and one line on standard error that says
/// so.
fn ended_for_its_reader(status: ExitStatus, stderr: &str) {
    assert_eq!(status.code(), Some(LOCAL), "{status:?} {stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(
        lines[0].starts_with("inbox wake: standard output: ") && lines[0].contains("reader"),
        "{stderr}"
    );
}

/// The next wake of bob takes the lock and returns on `expected` alone, which it marks: the
/// wake before it freed the lock and marked none of them.
fn the_next_wake_returns_on(world: &World, expected: &[String]) {
    let next = world.client("bob", &["wake", "--json"]);
    assert_eq!(next.status, 0, "the next wake: {next:#?}");
    assert_eq!(support::ids(&next.json()), expected, "{next:#?}");
    for message in expected {
        assert_eq!(state(world, message), "Delivered");
    }
}

/// A thread that reads `pipe` until a line holds `needle`, then hands the pipe back with what it
/// read, so the case closes it when it chooses.
fn read_until(
    pipe: ChildStdout,
    needle: String,
) -> std::thread::JoinHandle<(BufReader<ChildStdout>, String)> {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(pipe);
        let mut text = String::new();
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return (reader, text);
            }
            text.push_str(&line);
            if line.contains(&needle) {
                return (reader, text);
            }
        }
    })
}

/// The acceptance of the story for a pipe: a wake that waits on an empty inbox, its standard
/// output a pipe, ends with 4 when the case closes the reading end, without a post; the post
/// made after its end is in `Sent`, and the next wake takes the lock and returns on it.
#[test]
fn a_wake_whose_pipe_loses_its_reader_while_it_waits_ends_with_4_and_marks_nothing() {
    let world = World::new("reader-gone-pipe");
    world.client("bob", &["open"]).ok();
    let mut started = Started::spawn(wake(&world, &[]).stdout(Stdio::piped()));
    let pid = started.pid();
    wait_for("the wake holds its lock", || holds_a_lock(pid));
    drop(started.child().stdout.take());
    let (status, stderr) = started.finish("its reader closed");
    ended_for_its_reader(status, &stderr);

    let message = post(&world, "after the reader went away");
    assert_eq!(state(&world, &message), "Sent");
    the_next_wake_returns_on(&world, &[message]);
}

/// The same for `inbox wake --follow`: a round shown to a reader is marked; then the reader
/// closes, and the mode ends with 4 without a post, the next post stays in `Sent`.
#[test]
fn a_following_wake_whose_pipe_loses_its_reader_ends_with_4_and_marks_nothing_more() {
    let world = World::new("reader-gone-follow");
    world.client("bob", &["open"]).ok();
    let mut started = Started::spawn(wake(&world, &["--follow"]).stdout(Stdio::piped()));
    let pid = started.pid();
    wait_for("the wake holds its lock", || holds_a_lock(pid));
    let first = post(&world, "shown to a reader");
    let pipe = started.child().stdout.take().expect("stdout is piped");
    let reading = read_until(pipe, first.clone());
    support::wait_until_finished(&reading, "the reader of the first round");
    let (pipe, shown) = reading.join().expect("the reader ends");
    assert!(shown.contains(&first), "{shown}");
    wait_for("the first post is delivered", || {
        state(&world, &first) == "Delivered"
    });
    drop(pipe);
    let (status, stderr) = started.finish("its reader closed");
    ended_for_its_reader(status, &stderr);

    let second = post(&world, "after the reader went away");
    assert_eq!(state(&world, &second), "Sent");
    the_next_wake_returns_on(&world, &[second]);
    assert_eq!(state(&world, &first), "Delivered");
}

/// The same for a FIFO: the case holds its only reading end and closes it while the wake waits.
#[test]
fn a_wake_whose_fifo_loses_its_reader_while_it_waits_ends_with_4_and_marks_nothing() {
    let world = World::new("reader-gone-fifo");
    world.client("bob", &["open"]).ok();
    let fifo = world.dir.join("wake.fifo");
    let made = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "mkfifo failed");
    // The reading end first, without waiting for a writer; then the writing end, which finds it.
    let reader = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&fifo)
        .expect("the reading end opens");
    let writer = OpenOptions::new()
        .write(true)
        .open(&fifo)
        .expect("the writing end opens");
    let started = Started::spawn(wake(&world, &[]).stdout(writer));
    let pid = started.pid();
    wait_for("the wake holds its lock", || holds_a_lock(pid));
    drop(reader);
    let (status, stderr) = started.finish("its reader closed");
    ended_for_its_reader(status, &stderr);

    let message = post(&world, "after the reader went away");
    assert_eq!(state(&world, &message), "Sent");
    the_next_wake_returns_on(&world, &[message]);
}

/// Rows of more than a pipe holds: the wake is still writing them when the reader, which took
/// the first bytes, closes the pipe. Exit 4, and neither message is marked: the next wake
/// returns on both.
#[test]
fn rows_written_to_a_pipe_whose_reader_closes_are_not_marked() {
    let world = World::new("reader-gone-rows");
    world.client("bob", &["open"]).ok();
    let body = "x".repeat(40_000);
    let messages = [post(&world, &body), post(&world, &body)];
    let mut started = Started::spawn(wake(&world, &[]).stdout(Stdio::piped()));
    let mut pipe = started.child().stdout.take().expect("stdout is piped");
    let first = std::thread::spawn(move || {
        let mut head = [0_u8; 4096];
        pipe.read_exact(&mut head).expect("the first bytes arrive");
        (pipe, head)
    });
    support::wait_until_finished(&first, "the reader of the first bytes");
    let (pipe, head) = first.join().expect("the reader ends");
    assert!(
        String::from_utf8_lossy(&head).contains(&messages[0]),
        "{}",
        String::from_utf8_lossy(&head)
    );
    // 80,000 bytes and more of rows, 4,096 of them read: the rest does not fit a pipe of
    // 65,536 bytes, so the write still runs.
    drop(pipe);
    let (status, stderr) = started.finish("its reader closed");
    assert_eq!(status.code(), Some(LOCAL), "{status:?} {stderr}");
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    for message in &messages {
        assert_eq!(state(&world, message), "Sent", "{stderr}");
    }
    the_next_wake_returns_on(&world, &messages);
}

/// `script` of util-linux runs `command` on a pseudo terminal (the crate forbids the unsafe code
/// that would open one in the case), with the environment and directory of `inner`.
fn on_a_terminal(inner: &Command, command: &str) -> Command {
    let mut script = Command::new("script");
    script
        .args(["-q", "-e", "-c", command, "/dev/null"])
        .env("SHELL", "/bin/sh")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
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

/// The process `parent` started, once it runs the binary: its pid.
fn child_running_inbox(parent: u32) -> u32 {
    let mut found = None;
    wait_for("the binary runs under `script` of util-linux", || {
        found = fs::read_to_string(format!("/proc/{parent}/task/{parent}/children"))
            .unwrap_or_default()
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

/// `kill <name> <pid>`.
fn signal(pid: u32, name: &str) {
    let sent = Command::new("kill")
        .args([name, &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success(), "kill {name} {pid} failed");
}

/// A regular file and a terminal are not watched: `inbox wake --follow` on either shows a post
/// and marks it as before, and runs no watch of its reader once it has; a pipe is watched (the
/// cases above, and the first part here, so that the name looked for is the watch's).
#[test]
fn a_wake_on_a_regular_file_or_a_terminal_runs_no_watch_of_its_reader() {
    let world = World::new("reader-not-watched");
    world.client("bob", &["open"]).ok();

    let piped = Started::spawn(wake(&world, &[]).stdout(Stdio::piped()));
    wait_for_the_watch(piped.pid());
    drop(piped);

    let path = world.dir.join("wake.out");
    let to_file =
        Started::spawn(wake(&world, &["--follow"]).stdout(File::create(&path).expect("a file")));
    let message = post(&world, "to a file");
    wait_for("the post is delivered", || {
        state(&world, &message) == "Delivered"
    });
    let written = fs::read_to_string(&path).expect("the file is read");
    assert!(written.contains(&message), "{written}");
    let pid = to_file.pid();
    assert!(!watching(pid), "a file is watched: {:?}", threads(pid));
    drop(to_file);

    let inner = wake(&world, &["--follow"]);
    let terminal = Started::spawn(&mut on_a_terminal(
        &inner,
        &format!("exec '{BIN}' wake --follow"),
    ));
    let pid = child_running_inbox(terminal.pid());
    let shown = post(&world, "to a terminal");
    wait_for("the post is delivered", || {
        state(&world, &shown) == "Delivered"
    });
    assert!(!watching(pid), "a terminal is watched: {:?}", threads(pid));
    signal(pid, "-TERM");
    drop(terminal);
}

/// The hook of the Claude Code plugin under `sh`, with the built `inbox` first on its `PATH`, for
/// bob, whose inbox is opened with `--adapter claude_code`.
fn hook(world: &World, tmp: &Path) -> Command {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../adapters/claude-code/scripts/wake.sh");
    let bin = Path::new(BIN).parent().expect("the binary's directory");
    let mut command = Command::new("sh");
    command
        .arg(script)
        .current_dir(&world.dir.path)
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
        .env("TMPDIR", tmp)
        .env("HOME", &world.dir.path)
        .env("INBOX_URL", &world.service.url)
        .env("INBOX_SECRET_KEY_FILE", world.dir.join("bob.key"))
        .env("INBOX_OUTBOX", world.outbox("bob"))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

/// Whether the process `pid` has ended: no entry in `/proc`, or a zombie.
fn gone(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).map_or(true, |stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| rest.starts_with('Z'))
    })
}

/// The second acceptance statement: the hook reads the wake's output itself, so a hook ended by
/// SIGKILL leaves its wake without a reader, and the wake ends by itself, without a post, and
/// frees its lock; the post made after it stays in `Sent` and the next wake returns on it.
#[test]
fn a_hook_ended_by_sigkill_leaves_a_wake_that_ends_without_marking() {
    let world = World::new("reader-gone-hook");
    world
        .client("bob", &["open", "--adapter", "claude_code"])
        .ok();
    let tmp: PathBuf = world.dir.join("tmp");
    fs::create_dir_all(&tmp).expect("the hook's TMPDIR");
    let mut started = Started::spawn(&mut hook(&world, &tmp));
    let wake = child_running_inbox(started.pid());
    wait_for("the hook's wake holds its lock", || holds_a_lock(wake));
    started.child().kill().expect("SIGKILL to the hook");
    let _ = started.child().wait();
    wait_for("the hook's wake ended by itself", || gone(wake));
    // A thread group leader is a zombie while another thread of it still exits and holds the
    // lock: the lock is freed a moment later.
    wait_for("the hook's wake freed its lock", || !holds_a_lock(wake));

    let message = post(&world, "after the hook was killed");
    assert_eq!(state(&world, &message), "Sent");
    the_next_wake_returns_on(&world, &[message]);
}
