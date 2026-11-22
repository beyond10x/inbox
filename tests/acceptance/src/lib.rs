//! Fixtures of the acceptance checks of story:service-acceptance (`tests/checks.rs`): the built
//! `inbox` binary, a directory per check, keys made on the spot with `inbox key new` and `inbox
//! keyfile add-key`, a running `inbox serve` that is ended when it goes out of scope (a failed
//! assertion included), and the client commands run against it.
//!
//! The binary is the file `INBOX_BIN` names; `task acceptance` builds it (`cargo build --locked
//! -p inbox-cli`) and sets it. Unset, or naming no file, the check fails and says so. No key
//! material is written into this file: every key is made at run time, in the check's directory.

#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Stdio};

use inbox::server::json::{self, Value};
use inbox_server::format_reading;
use inbox_verify::Instant;

/// The service's clock file in a check's directory ([`World::with_clock`]).
const CLOCK: &str = "clock";

/// The principal of the first client.
pub const ALICE: &str = "agent:acceptance/alice";
/// The principal of the second client.
pub const BOB: &str = "agent:acceptance/bob";

/// The `inbox` binary the checks run: the file `INBOX_BIN` names.
///
/// # Panics
///
/// `INBOX_BIN` is unset or names no file.
#[must_use]
pub fn binary() -> PathBuf {
    let Some(path) = std::env::var_os("INBOX_BIN") else {
        panic!(
            "INBOX_BIN is not set: it names the built inbox binary; `task acceptance` builds it with `cargo build --locked -p inbox-cli` and sets it"
        );
    };
    let path = PathBuf::from(path);
    assert!(
        path.is_file(),
        "INBOX_BIN names {}, which is no file: build it with `cargo build --locked -p inbox-cli`",
        path.display()
    );
    path
}

/// What one run of a client command answered.
#[derive(Debug)]
pub struct Run {
    /// The exit status; -1 for an end by signal.
    pub status: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

impl Run {
    /// Asserts the run exited 0 and answers it.
    ///
    /// # Panics
    ///
    /// It exited otherwise.
    pub fn ok(self) -> Self {
        assert_eq!(self.status, 0, "{self:#?}");
        self
    }

    /// Standard output as one JSON document.
    ///
    /// # Panics
    ///
    /// It is not one.
    #[must_use]
    pub fn json(&self) -> Value {
        json::parse(self.stdout.trim())
            .unwrap_or_else(|error| panic!("{error}: stdout is not one JSON document: {self:#?}"))
    }
}

/// A running `inbox serve`. Dropped, it is killed and waited for, so no process outlives a check.
pub struct Service {
    child: Child,
    _stdout: BufReader<ChildStdout>,
    /// The URL from the first line of its standard output.
    pub url: String,
}

impl Service {
    /// Asks it to stop with SIGTERM and waits for its exit.
    ///
    /// # Panics
    ///
    /// `kill` does not run or fails, or the wait fails.
    pub fn stop(mut self) -> ExitStatus {
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

/// One check's world: its directory, an operator key, the keys of [`ALICE`] and [`BOB`] in a key
/// file the operator key signed, and a running service over it. Dropped, the service is ended
/// and the directory removed.
pub struct World {
    binary: PathBuf,
    dir: PathBuf,
    operator_public_key: String,
    serve_args: Vec<String>,
    started: usize,
    service: Option<Service>,
}

impl World {
    /// Makes the keys in a fresh directory `<root>/<name>` and starts `inbox serve --listen
    /// 127.0.0.1:0 --key-file keys.json --operator-public-key <key> <serve_args>`.
    ///
    /// # Panics
    ///
    /// The binary is missing, a key command fails, or the service prints no first line.
    #[must_use]
    pub fn new(root: &Path, name: &str, serve_args: &[&str]) -> Self {
        Self::make(root, name, serve_args, None)
    }

    /// [`World::new`] with the service's clock in the file `clock` of the check's directory
    /// (`inbox serve --clock-file`, a flag of a debug build only), set to `at` before the
    /// service starts. The clock reads what the file holds until [`World::set_clock`] moves it:
    /// it does not move on by itself, so a slow host passes no instant. The client commands sign
    /// with the system clock, and the service takes a request signed within 60 s of its reading:
    /// a check keeps the clock within that of the system clock.
    ///
    /// # Panics
    ///
    /// As [`World::new`], or the clock file cannot be written.
    #[must_use]
    pub fn with_clock(root: &Path, name: &str, serve_args: &[&str], at: Instant) -> Self {
        Self::make(root, name, serve_args, Some(at))
    }

    fn make(root: &Path, name: &str, serve_args: &[&str], clock: Option<Instant>) -> Self {
        let binary = binary();
        let dir = root.join(format!("acceptance-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("the check's directory is created");
        let mut world = Self {
            binary,
            dir,
            operator_public_key: String::new(),
            serve_args: serve_args.iter().map(|arg| (*arg).to_owned()).collect(),
            started: 0,
            service: None,
        };
        world.operator_public_key = world.key_new("operator.key", None);
        for (who, principal) in [("alice", ALICE), ("bob", BOB)] {
            let key_id = format!("{who}-1");
            let public_key = world.key_new(&format!("{who}.key"), Some((&key_id, principal)));
            world
                .run(
                    [
                        "keyfile",
                        "add-key",
                        "--file",
                        "keys.json",
                        "--operator-secret-key-file",
                        "operator.key",
                        "--key-id",
                        &key_id,
                        "--principal",
                        principal,
                        "--public-key",
                        &public_key,
                    ],
                    None,
                )
                .ok();
        }
        if let Some(at) = clock {
            world.set_clock(at);
            world
                .serve_args
                .extend(["--clock-file".to_owned(), CLOCK.to_owned()]);
        }
        world.start();
        world
    }

    /// Writes `at` into the clock file whole: into a file beside it, renamed over it, so the
    /// service never reads half an instant.
    ///
    /// # Panics
    ///
    /// The file cannot be written or renamed.
    pub fn set_clock(&self, at: Instant) {
        let staged = self.dir.join(format!("{CLOCK}.new"));
        fs::write(&staged, format_reading(at)).expect("the clock file is written");
        fs::rename(&staged, self.dir.join(CLOCK)).expect("the clock file is replaced");
    }

    /// The binary, run in the check's directory with no `INBOX_*` variable of the check's own
    /// environment and a home and state directory inside it, so nothing outside it is read or
    /// written.
    fn command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command.current_dir(&self.dir).stdin(Stdio::null());
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("INBOX_") {
                command.env_remove(name);
            }
        }
        command
            .env("HOME", &self.dir)
            .env("XDG_STATE_HOME", self.dir.join("state"));
        command
    }

    fn run<I, S>(&self, args: I, who: Option<&str>) -> Run
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.command();
        command.args(args);
        if let Some(who) = who {
            let url = self.service.as_ref().map_or("", |service| &service.url);
            command
                .env("INBOX_URL", url)
                .env("INBOX_SECRET_KEY_FILE", format!("{who}.key"))
                .env("INBOX_OUTBOX", self.dir.join(format!("{who}.outbox")));
        }
        let output = command.output().expect("the binary starts");
        Run {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// `inbox key new`, answering the public key it printed.
    fn key_new(&self, file: &str, identity: Option<(&str, &str)>) -> String {
        let mut args = vec!["key", "new", "--out", file];
        if let Some((key_id, principal)) = identity {
            args.extend(["--key-id", key_id, "--principal", principal]);
        }
        self.run(args, None).ok().stdout.trim().to_owned()
    }

    /// Starts the service and waits for the first line of its standard output.
    fn start(&mut self) {
        self.started += 1;
        let errors = self.dir.join(format!("serve-{}.err", self.started));
        let mut child = self
            .command()
            .arg("serve")
            .args([
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                &self.operator_public_key,
            ])
            .args(&self.serve_args)
            .stdout(Stdio::piped())
            .stderr(File::create(&errors).expect("the stderr file is created"))
            .spawn()
            .expect("the binary starts");
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut service = Service {
            child,
            _stdout: BufReader::new(stdout),
            url: String::new(),
        };
        let mut line = String::new();
        let read = service._stdout.read_line(&mut line).unwrap_or(0);
        assert!(
            read > 0,
            "inbox serve printed no first line; its standard error: {}",
            fs::read_to_string(&errors).unwrap_or_default()
        );
        service.url = line.trim_end().to_owned();
        self.service = Some(service);
    }

    /// Stops the service with SIGTERM, asserts it exited 0, and starts another one over the same
    /// key file: a restart, which loses every message the in-memory backend held.
    ///
    /// # Panics
    ///
    /// No service runs, it exits otherwise, or the new one prints no first line.
    pub fn restart(&mut self) {
        let old = self.service.take().expect("a service runs");
        let status = old.stop();
        assert!(status.success(), "inbox serve exited {status:?}");
        self.start();
    }

    /// The URL of the running service.
    ///
    /// # Panics
    ///
    /// No service runs.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.service.as_ref().expect("a service runs").url
    }

    /// The outbox file of `who` (`alice` or `bob`).
    #[must_use]
    pub fn outbox(&self, who: &str) -> PathBuf {
        self.dir.join(format!("{who}.outbox"))
    }

    /// `inbox <args>` as `who` (`alice` or `bob`): its key file, its outbox file and the URL of
    /// the running service.
    #[must_use]
    pub fn client(&self, who: &str, args: &[&str]) -> Run {
        self.run(args, Some(who))
    }
}

impl Drop for World {
    fn drop(&mut self) {
        self.service = None;
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// The member at `path` of `value`.
///
/// # Panics
///
/// A member of the path is absent.
#[must_use]
pub fn member<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |at, name| {
        at.member(name)
            .unwrap_or_else(|| panic!("no `{name}` in {at:?}"))
    })
}

/// The text member at `path` of `value`.
///
/// # Panics
///
/// It is absent or not text.
#[must_use]
pub fn text(value: &Value, path: &[&str]) -> String {
    match member(value, path) {
        Value::Text(text) => text.clone(),
        other => panic!("`{}` is {other:?}", path.join(".")),
    }
}

/// The rows of a view answer.
///
/// # Panics
///
/// `rows` is absent or not an array.
#[must_use]
pub fn rows(value: &Value) -> Vec<Value> {
    match member(value, &["rows"]) {
        Value::Array(rows) => rows.clone(),
        other => panic!("`rows` is {other:?}"),
    }
}

/// The `message_id` of every row of a view answer.
#[must_use]
pub fn ids(value: &Value) -> Vec<String> {
    rows(value)
        .iter()
        .map(|row| text(row, &["message_id"]))
        .collect()
}
