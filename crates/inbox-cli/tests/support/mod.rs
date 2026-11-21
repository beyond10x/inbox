//! Fixtures of the binary's tests: a fresh directory per test, the built `inbox` run in it, a
//! running `inbox serve` that is ended when it goes out of scope (a failed assertion included),
//! and a world of an operator key, two client keys and a key file made with the binary itself.
//! No key material is written into this file: every key is made at run time.

#![allow(dead_code)]

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};

use inbox::server::json::{self, Value};

/// The binary under test.
pub const BIN: &str = env!("CARGO_BIN_EXE_inbox");

pub const ALICE: &str = "agent:test/alice";
pub const BOB: &str = "agent:test/bob";

/// The exit statuses the binary documents.
pub const REFUSED: i32 = 1;
pub const USAGE: i32 = 2;
pub const TRANSPORT: i32 = 3;
pub const LOCAL: i32 = 4;

/// A fresh directory under the target's temporary directory, removed when dropped.
pub struct Dir {
    pub path: PathBuf,
}

impl Dir {
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
            "inbox-cli-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("the test directory is created");
        Self { path }
    }

    pub fn join(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// The binary, run in `dir`, with no `INBOX_*` variable of the test's own environment, and a
/// home and state directory inside `dir`, so nothing outside it is read or written.
pub fn command(dir: &Path) -> Command {
    let mut command = Command::new(BIN);
    command.current_dir(dir).stdin(Stdio::null());
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("INBOX_") {
            command.env_remove(name);
        }
    }
    command
        .env("HOME", dir)
        .env("XDG_STATE_HOME", dir.join("state"));
    command
}

/// What one run of the binary answered.
#[derive(Debug)]
pub struct Run {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// Asserts the run exited 0 and answers it.
    pub fn ok(self) -> Self {
        assert_eq!(self.status, 0, "{self:#?}");
        self
    }

    /// Standard output as one JSON document.
    pub fn json(&self) -> Value {
        json::parse(self.stdout.trim())
            .unwrap_or_else(|error| panic!("{error}: stdout is not one JSON document: {self:#?}"))
    }
}

pub fn run(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
    let output = command(dir)
        .args(args)
        .envs(env.iter().copied())
        .output()
        .expect("the binary starts");
    Run {
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// A running `inbox serve`. Dropped, it is killed and waited for, so no process outlives a test.
pub struct Service {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    pub first_line: String,
    pub url: String,
    stderr: PathBuf,
}

impl Service {
    /// Starts `inbox serve <args>` in `dir` and waits for its first line on standard output.
    pub fn start(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Self {
        let stderr = dir.join(format!(
            "serve-{}.err",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|elapsed| elapsed.as_nanos())
                .unwrap_or_default()
        ));
        let mut child = command(dir)
            .arg("serve")
            .args(args)
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(File::create(&stderr).expect("the stderr file is created"))
            .spawn()
            .expect("the binary starts");
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut service = Self {
            child,
            _stdout: BufReader::new(stdout),
            first_line: String::new(),
            url: String::new(),
            stderr,
        };
        let mut line = String::new();
        let read = service._stdout.read_line(&mut line).unwrap_or(0);
        assert!(
            read > 0,
            "inbox serve printed no first line; stderr: {}",
            service.stderr()
        );
        service.first_line = line.trim_end().to_owned();
        service.url = service.first_line.clone();
        service
    }

    pub fn stderr(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Sends SIGTERM and waits for the exit.
    pub fn terminate(mut self) -> ExitStatus {
        let sent = Command::new("kill")
            .args(["-TERM", &self.child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(sent.success(), "kill -TERM failed");
        self.child.wait().expect("inbox serve is waited for")
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// `inbox key new`, answering the public key it printed.
pub fn key_new(dir: &Path, file: &str, identity: Option<(&str, &str)>) -> String {
    let mut args = vec!["key", "new", "--out", file];
    if let Some((key_id, principal)) = identity {
        args.extend(["--key-id", key_id, "--principal", principal]);
    }
    run(dir, &args, &[]).ok().stdout.trim().to_owned()
}

/// `inbox keyfile add-key`.
pub fn add_key(dir: &Path, operator: &str, key_id: &str, principal: &str, public_key: &str) {
    run(
        dir,
        &[
            "keyfile",
            "add-key",
            "--file",
            "keys.json",
            "--operator-secret-key-file",
            operator,
            "--key-id",
            key_id,
            "--principal",
            principal,
            "--public-key",
            public_key,
        ],
        &[],
    )
    .ok();
}

/// An operator key, the keys of alice and bob, a key file holding both, and a running service
/// over it.
pub struct World {
    pub service: Service,
    pub operator_public_key: String,
    pub dir: Dir,
}

impl World {
    pub fn new(name: &str) -> Self {
        let (dir, operator_public_key) = keys_only(name);
        let service = serve(&dir.path, &operator_public_key);
        Self {
            service,
            operator_public_key,
            dir,
        }
    }

    /// Ends the service and starts another one over the same key file: a restart, which loses
    /// every message the in-memory backend held.
    pub fn restart(&mut self) {
        let old = std::mem::replace(
            &mut self.service,
            serve(&self.dir.path, &self.operator_public_key),
        );
        let status = old.terminate();
        assert!(status.success(), "{status:?}");
    }

    /// `inbox <args> --secret-key-file <who>.key` against the service, with `<who>`'s outbox.
    pub fn client(&self, who: &str, args: &[&str]) -> Run {
        self.client_env(who, args, &[])
    }

    pub fn client_env(&self, who: &str, args: &[&str], env: &[(&str, &str)]) -> Run {
        let output = self
            .client_command(who, args)
            .envs(env.iter().copied())
            .output()
            .expect("the binary starts");
        Run {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
            stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
        }
    }

    /// The command of [`World::client`], not started.
    pub fn client_command(&self, who: &str, args: &[&str]) -> Command {
        let mut command = command(&self.dir.path);
        command
            .args(args)
            .env("INBOX_URL", &self.service.url)
            .env("INBOX_SECRET_KEY_FILE", format!("{who}.key"))
            .env("INBOX_OUTBOX", self.outbox(who));
        command
    }

    /// [`World::client`] started and not waited for.
    pub fn spawn(&self, who: &str, args: &[&str]) -> Spawned {
        Spawned(Some(
            self.client_command(who, args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the binary starts"),
        ))
    }

    pub fn outbox(&self, who: &str) -> PathBuf {
        self.dir.join(&format!("{who}.outbox"))
    }

    /// The message ids the outbox file of `who` holds.
    pub fn kept(&self, who: &str) -> Vec<String> {
        fs::read_to_string(self.outbox(who))
            .unwrap_or_default()
            .lines()
            .map(|line| {
                text(
                    &json::parse(line).expect("an outbox line is JSON"),
                    "message_id",
                )
            })
            .collect()
    }
}

/// A client process started and not waited for; killed when dropped unfinished.
pub struct Spawned(Option<Child>);

impl Spawned {
    /// Whether it is still running.
    pub fn running(&mut self) -> bool {
        matches!(self.0.as_mut().map(Child::try_wait), Some(Ok(None)))
    }

    /// Waits for it to end and answers what it printed.
    pub fn finish(mut self) -> Run {
        let output = self
            .0
            .take()
            .expect("not finished yet")
            .wait_with_output()
            .expect("the process is waited for");
        Run {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
            stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
        }
    }
}

impl Drop for Spawned {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// `inbox serve <args>` that is expected to refuse to start: answers its exit status and
/// standard error once it has ended. Fails, and kills it, if it prints a first line instead.
pub fn serve_refused(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> Run {
    let mut spawned = Spawned(Some(
        command(dir)
            .arg("serve")
            .args(args)
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary starts"),
    ));
    let mut line = String::new();
    let stdout = spawned.0.as_mut().and_then(|child| child.stdout.as_mut());
    let read = BufReader::new(stdout.expect("stdout is piped"))
        .read_line(&mut line)
        .unwrap_or(0);
    assert_eq!(read, 0, "inbox serve started and printed {line:?}");
    spawned.finish()
}

/// A directory with an operator key and a key file holding the keys of alice and bob, and no
/// service; answers the operator public key.
pub fn keys_only(name: &str) -> (Dir, String) {
    let dir = Dir::new(name);
    let operator_public_key = key_new(&dir.path, "operator.key", None);
    for (file, key_id, principal) in [("alice.key", "alice-1", ALICE), ("bob.key", "bob-1", BOB)] {
        let public_key = key_new(&dir.path, file, Some((key_id, principal)));
        add_key(&dir.path, "operator.key", key_id, principal, &public_key);
    }
    (dir, operator_public_key)
}

/// `inbox serve` on an ephemeral loopback port over `keys.json` in `dir`.
pub fn serve(dir: &Path, operator_public_key: &str) -> Service {
    Service::start(
        dir,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            operator_public_key,
            "--max-wait",
            "20",
        ],
        &[],
    )
}

/// The text member `name` of `value`.
pub fn text(value: &Value, name: &str) -> String {
    match value.member(name) {
        Some(Value::Text(text)) => text.clone(),
        other => panic!("`{name}` is {other:?}"),
    }
}

/// The rows of a view answer.
pub fn rows(value: &Value) -> Vec<Value> {
    match value.member("rows") {
        Some(Value::Array(rows)) => rows.clone(),
        other => panic!("`rows` is {other:?}"),
    }
}

/// The `message_id` of every row of a view answer.
pub fn ids(value: &Value) -> Vec<String> {
    rows(value)
        .iter()
        .map(|row| text(row, "message_id"))
        .collect()
}

/// A stand-in for the service on 127.0.0.1:0 that reads one request in full and answers it with
/// the status `status` and the body `body`; answers its URL. The thread ends after one request.
pub fn stand_in(status: u16, body: &'static str) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let handle = std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
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
            let head_end = received.windows(4).position(|window| window == b"\r\n\r\n");
            if let (None, Some(end)) = (expected, head_end) {
                let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                expected = Some(end + 4 + length);
            }
        }
        let answer = format!(
            "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(answer.as_bytes());
    });
    (url, handle)
}
