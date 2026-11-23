//! Fixtures of the acceptance checks of story:service-acceptance (`tests/checks.rs`): the built
//! `inbox` binary, a directory per check, keys made on the spot with `inbox key new` and `inbox
//! keyfile add-key`, a running `inbox serve` that is ended when it goes out of scope (a failed
//! assertion included), and the client commands run against it. [`World`] is the fixture of the
//! 3 checks of stage 2 (one directory, plain HTTP); [`TwoHosts`] that of the 3 checks of
//! story:the-stage-3-acceptance-checks (a service with a certificate and 2 clients that share no
//! file), with [`Route`] for the fact that a wake holds its read.
//!
//! The binary is the file `INBOX_BIN` names; `task acceptance` builds it (`cargo build --locked
//! -p inbox-cli`) and sets it. Unset, or naming no file, the check fails and says so. No key
//! material, certificate or pin is written into this file: each is made at run time, in the
//! check's directory.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::time::Duration;

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
    /// Starts `command` (an `inbox serve`) with its standard error in the file `errors`, and waits
    /// for the first line of its standard output.
    ///
    /// # Panics
    ///
    /// The binary does not start, the file cannot be created, or it prints no first line.
    fn spawn(mut command: Command, errors: &Path) -> Self {
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(File::create(errors).expect("the stderr file is created"))
            .spawn()
            .expect("the binary starts");
        let stdout = child.stdout.take().expect("stdout is piped");
        let mut service = Self {
            child,
            _stdout: BufReader::new(stdout),
            url: String::new(),
        };
        let mut line = String::new();
        let read = service._stdout.read_line(&mut line).unwrap_or(0);
        assert!(
            read > 0,
            "inbox serve printed no first line; its standard error: {}",
            fs::read_to_string(errors).unwrap_or_default()
        );
        service.url = line.trim_end().to_owned();
        service
    }

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
        let mut command = self.command();
        command
            .arg("serve")
            .args([
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                &self.operator_public_key,
            ])
            .args(&self.serve_args);
        self.service = Some(Service::spawn(command, &errors));
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

/// The longest wait of a check of stage 3 on a fact or on the end of a process: it only ends a
/// hang, and no check holds a time of its own.
pub const HANG: Duration = Duration::from_secs(20);

/// The variables a client command of [`TwoHosts`] is started with, and no other.
pub const CLIENT_VARIABLES: [&str; 6] = [
    "HOME",
    "INBOX_OUTBOX",
    "INBOX_SECRET_KEY_FILE",
    "INBOX_TLS_PIN",
    "INBOX_URL",
    "XDG_STATE_HOME",
];

/// The variables of [`CLIENT_VARIABLES`] that name a path; the other 2, the URL and the pin, are
/// the service's, given to each client as a value.
const PATH_VARIABLES: [&str; 4] = [
    "HOME",
    "INBOX_OUTBOX",
    "INBOX_SECRET_KEY_FILE",
    "XDG_STATE_HOME",
];

/// One client host of [`TwoHosts`]: a directory of its own, the working directory of each of its
/// commands, and the variables each of them is started with.
struct Host {
    dir: PathBuf,
    vars: BTreeMap<String, OsString>,
}

/// One check of stage 3: one `inbox serve` with a certificate and 2 clients that share nothing
/// but the binary, as on 2 hosts. The check's directory `<root>/acceptance-<name>-<pid>` holds 3
/// directories:
///
/// - `service/`: the operator key, the key file, the TLS file `inbox tls new` made and the
///   standard error of each start; `inbox serve` runs there with an empty environment.
/// - `alice/` and `bob/`: each client's key (made there by `inbox key new` with the client's own
///   environment; only its public key goes to the service's key file), its home `home/` and its
///   state directory `state/`, which holds its outbox file `state/inbox/outbox.jsonl` and the
///   files beside it.
///
/// Every client command starts with an empty environment (`env_clear`) and only the variables
/// [`CLIENT_VARIABLES`] names, each path of them inside the client's own directory; the URL and
/// the pin are the service's, given to each client as a value. Every client command of
/// [`TwoHosts::client`] fails the check when the other client's directory changed while it ran.
///
/// The clients' URL is the address of a [`Route`] the fixture owns (the front), not the
/// service's own: each start of the service binds an ephemeral port of its own, and the front is
/// pointed at it, so a restart changes no client's environment and binds no port twice.
///
/// What this proves: nothing of one client host is needed by the other, and a client given the
/// pin of another certificate sends nothing (check 1). What it does not prove: the path between
/// 2 hosts (both reach the service on a loopback address of one machine).
pub struct TwoHosts {
    binary: PathBuf,
    dir: PathBuf,
    service_dir: PathBuf,
    operator_public_key: String,
    serve_args: Vec<String>,
    started: usize,
    service: Option<Service>,
    front: Option<Route>,
    alice: Host,
    bob: Host,
}

impl TwoHosts {
    /// Makes the 3 directories, the keys and the certificate in a fresh `<root>/acceptance-
    /// <name>-<pid>` and starts `inbox serve --listen 127.0.0.1:0 --key-file keys.json
    /// --operator-public-key <key> --tls-file tls.json <serve_args>`.
    ///
    /// # Panics
    ///
    /// The binary is missing, a command that makes a key or the certificate fails, or the
    /// service prints no first line or one that is no `https://` URL.
    #[must_use]
    pub fn new(root: &Path, name: &str, serve_args: &[&str]) -> Self {
        let binary = binary();
        let dir = root.join(format!("acceptance-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let service_dir = dir.join("service");
        fs::create_dir_all(&service_dir).expect("the service's directory is created");
        let host = |who: &str| {
            let at = dir.join(who);
            fs::create_dir_all(at.join("home")).expect("the client's home is created");
            let vars = [
                ("HOME", at.join("home").into_os_string()),
                ("XDG_STATE_HOME", at.join("state").into_os_string()),
                (
                    "INBOX_SECRET_KEY_FILE",
                    at.join(format!("{who}.key")).into(),
                ),
                ("INBOX_OUTBOX", at.join("state/inbox/outbox.jsonl").into()),
            ]
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect();
            Host { dir: at, vars }
        };
        let mut hosts = Self {
            alice: host("alice"),
            bob: host("bob"),
            binary,
            dir,
            service_dir,
            operator_public_key: String::new(),
            serve_args: serve_args.iter().map(|arg| (*arg).to_owned()).collect(),
            started: 0,
            service: None,
            front: None,
        };
        hosts.operator_public_key = hosts.at_the_service(&["key", "new", "--out", "operator.key"]);
        let pin = hosts.make_a_certificate(&hosts.service_dir);
        for (who, principal) in [("alice", ALICE), ("bob", BOB)] {
            let key_id = format!("{who}-1");
            let key_file = format!("{who}.key");
            let public_key = hosts
                .client_command(who)
                .args([
                    "key",
                    "new",
                    "--out",
                    &key_file,
                    "--key-id",
                    &key_id,
                    "--principal",
                    principal,
                ])
                .output()
                .map(Run::from)
                .expect("the binary starts")
                .ok()
                .stdout
                .trim()
                .to_owned();
            hosts.at_the_service(&[
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
            ]);
            hosts
                .host_mut(who)
                .vars
                .insert("INBOX_TLS_PIN".to_owned(), pin.clone().into());
        }
        hosts.start();
        hosts
    }

    /// `inbox <args>` in the service's directory with an empty environment; answers its standard
    /// output, trimmed.
    fn at_the_service(&self, args: &[&str]) -> String {
        let output = Command::new(&self.binary)
            .current_dir(&self.service_dir)
            .env_clear()
            .stdin(Stdio::null())
            .args(args)
            .output()
            .expect("the binary starts");
        Run::from(output).ok().stdout.trim().to_owned()
    }

    /// `inbox tls new --out tls.json` in `dir` with an empty environment; answers the pin it
    /// printed. A failure names its exit status and standard error, never its standard output,
    /// which is the pin.
    fn make_a_certificate(&self, dir: &Path) -> String {
        fs::create_dir_all(dir).expect("the certificate's directory is created");
        let output = Command::new(&self.binary)
            .current_dir(dir)
            .env_clear()
            .stdin(Stdio::null())
            .args(["tls", "new", "--out", "tls.json"])
            .output()
            .expect("the binary starts");
        assert!(
            output.status.success(),
            "inbox tls new exited {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    /// The pin of another certificate, made by `inbox tls new` in the check's directory
    /// `elsewhere/` and served by no service: a client given it reaches the service, whose
    /// certificate it does not name.
    ///
    /// # Panics
    ///
    /// `inbox tls new` fails, or it printed the pin of the service's certificate.
    #[must_use]
    pub fn pin_of_another_certificate(&self) -> String {
        let pin = self.make_a_certificate(&self.dir.join("elsewhere"));
        assert!(
            self.alice.vars["INBOX_TLS_PIN"] != OsStr::new(&pin),
            "the 2 certificates have one pin"
        );
        pin
    }

    /// Starts the service on an ephemeral port of its own, asserts it serves `https://`, and
    /// points the front at it; the first start makes the front and gives each client its URL.
    fn start(&mut self) {
        self.started += 1;
        let errors = self.service_dir.join(format!("serve-{}.err", self.started));
        let mut command = Command::new(&self.binary);
        command
            .current_dir(&self.service_dir)
            .env_clear()
            .stdin(Stdio::null())
            .arg("serve")
            .args([
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                &self.operator_public_key,
                "--tls-file",
                "tls.json",
            ])
            .args(&self.serve_args);
        let service = Service::spawn(command, &errors);
        let Some(address) = service.url.strip_prefix("https://") else {
            panic!("inbox serve does not serve https://");
        };
        if let Some(front) = &self.front {
            front.to(address);
        } else {
            let front = Route::start(address);
            let url = format!("https://{}", front.address);
            for host in [&mut self.alice, &mut self.bob] {
                host.vars.insert("INBOX_URL".to_owned(), url.clone().into());
            }
            self.front = Some(front);
        }
        self.service = Some(service);
    }

    /// Stops the service with SIGTERM, asserts it exited 0, and starts another one over the same
    /// key file and certificate on a port of its own, which the front then carries to: a
    /// restart, which loses every message the in-memory backend held. The clients' environments
    /// are not changed.
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

    /// The URL of the running service, as it printed it (not the clients' URL, the front's).
    ///
    /// # Panics
    ///
    /// No service runs.
    #[must_use]
    pub fn url(&self) -> &str {
        &self.service.as_ref().expect("a service runs").url
    }

    /// The address of the running service, `127.0.0.1:<port>`.
    ///
    /// # Panics
    ///
    /// No service runs.
    #[must_use]
    pub fn service_address(&self) -> SocketAddr {
        self.url()
            .strip_prefix("https://")
            .and_then(|address| address.parse().ok())
            .expect("the service's URL is https://<address>")
    }

    fn host(&self, who: &str) -> &Host {
        match who {
            "alice" => &self.alice,
            "bob" => &self.bob,
            other => panic!("no client {other}: alice or bob"),
        }
    }

    fn host_mut(&mut self, who: &str) -> &mut Host {
        match who {
            "alice" => &mut self.alice,
            "bob" => &mut self.bob,
            other => panic!("no client {other}: alice or bob"),
        }
    }

    fn other(&self, who: &str) -> &Host {
        match who {
            "alice" => &self.bob,
            "bob" => &self.alice,
            other => panic!("no client {other}: alice or bob"),
        }
    }

    /// The binary in the client's directory, with an empty environment and the client's own
    /// variables only.
    fn client_command(&self, who: &str) -> Command {
        let host = self.host(who);
        let mut command = Command::new(&self.binary);
        command
            .current_dir(&host.dir)
            .env_clear()
            .envs(&host.vars)
            .stdin(Stdio::null());
        command
    }

    /// The directory of `who` (`alice` or `bob`).
    #[must_use]
    pub fn dir(&self, who: &str) -> &Path {
        &self.host(who).dir
    }

    /// Asserts the 2 clients share no path: neither directory is inside the other, each client's
    /// variables are [`CLIENT_VARIABLES`], and each of them that names a path ([`PATH_VARIABLES`])
    /// names an absolute one inside that client's own directory and outside the other's. A
    /// failure names the variable and the client, and no value.
    ///
    /// # Panics
    ///
    /// They share one.
    pub fn assert_apart(&self) {
        let (alice, bob) = (&self.alice.dir, &self.bob.dir);
        assert!(
            !alice.starts_with(bob) && !bob.starts_with(alice),
            "the directories of alice and bob are nested"
        );
        for who in ["alice", "bob"] {
            let (own, other) = (self.host(who), self.other(who));
            let names: Vec<&str> = own.vars.keys().map(String::as_str).collect();
            assert_eq!(
                names, CLIENT_VARIABLES,
                "the names of the variables of {who}"
            );
            for name in PATH_VARIABLES {
                let path = Path::new(&own.vars[name]);
                assert!(
                    path.is_absolute() && path.starts_with(&own.dir),
                    "{name} of {who} names no absolute path inside {who}'s directory"
                );
                assert!(
                    !path.starts_with(&other.dir),
                    "{name} of {who} names a path inside the other client's directory"
                );
            }
        }
    }

    /// Asserts `started`, a command of `who`, runs with exactly the variables of `who`, read from
    /// `/proc/<pid>/environ`. A failure names the variables that are missing, extra or of
    /// another value, and no value.
    ///
    /// # Panics
    ///
    /// They differ.
    pub fn assert_environment(&self, started: &Started, who: &str) {
        let running = started.environment();
        let own: BTreeMap<String, String> = self
            .host(who)
            .vars
            .iter()
            .map(|(name, value)| (name.clone(), value.to_string_lossy().into_owned()))
            .collect();
        let missing: Vec<&String> = own
            .keys()
            .filter(|name| !running.contains_key(*name))
            .collect();
        let extra: Vec<&String> = running
            .keys()
            .filter(|name| !own.contains_key(*name))
            .collect();
        let other: Vec<&String> = own
            .iter()
            .filter(|(name, value)| running.get(*name).is_some_and(|seen| seen != *value))
            .map(|(name, _)| name)
            .collect();
        assert!(
            missing.is_empty() && extra.is_empty() && other.is_empty(),
            "the process of {who} does not run with {who}'s variables alone: missing {missing:?}, \
             extra {} variables {:?}, of another value {other:?}",
            extra.len(),
            extra.iter().take(10).collect::<Vec<_>>()
        );
    }

    /// `inbox <args>` as `who` (`alice` or `bob`), run to its end. Fails when the other client's
    /// directory changed while it ran: a file the other did not write is in it.
    ///
    /// # Panics
    ///
    /// The binary does not start, or the other client's directory changed.
    #[must_use]
    pub fn client(&self, who: &str, args: &[&str]) -> Run {
        self.client_with(who, &[], args)
    }

    /// [`TwoHosts::client`] with the variables `changed` set over the client's own, for this one
    /// command.
    ///
    /// # Panics
    ///
    /// As [`TwoHosts::client`], or a name of `changed` is not one of [`CLIENT_VARIABLES`].
    #[must_use]
    pub fn client_with(&self, who: &str, changed: &[(&str, &str)], args: &[&str]) -> Run {
        let other = self.other(who);
        let before = tree(&other.dir);
        let mut command = self.client_command(who);
        for (name, value) in changed {
            assert!(
                CLIENT_VARIABLES.contains(name),
                "{name} is no client variable"
            );
            command.env(name, value);
        }
        let output = command.args(args).output().expect("the binary starts");
        assert_unchanged(
            &other.dir,
            &before,
            &format!("`inbox {}` of {who}", args.join(" ")),
        );
        Run::from(output)
    }

    /// `inbox <args>` as `who`, started and not waited for: its standard output and standard
    /// error are pipes. This call does not hold the other client's directory, since the other
    /// client runs commands meanwhile: the check holds it with [`TwoHosts::snapshot`] and
    /// [`TwoHosts::assert_unchanged_since`] over the spans in which only this process runs.
    ///
    /// # Panics
    ///
    /// The binary does not start.
    #[must_use]
    pub fn spawn(&self, who: &str, args: &[&str]) -> Started {
        let child = self
            .client_command(who)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary starts");
        Started {
            pid: child.id(),
            child: Some(child),
        }
    }

    /// What the directory of `who` holds now, for [`TwoHosts::assert_unchanged_since`].
    #[must_use]
    pub fn snapshot(&self, who: &str) -> Snapshot {
        Snapshot(tree(&self.host(who).dir))
    }

    /// Asserts the directory of `who` holds what `before` held: `what`, run by the other client
    /// meanwhile, wrote nothing in it.
    ///
    /// # Panics
    ///
    /// It changed.
    pub fn assert_unchanged_since(&self, who: &str, before: &Snapshot, what: &str) {
        assert_unchanged(&self.host(who).dir, &before.0, what);
    }

    /// Puts a [`Route`] of its own between `who` and the running service: each later command of
    /// `who` reaches the service through it, and the check sees each connection it carries.
    ///
    /// # Panics
    ///
    /// No service runs or the route cannot bind.
    pub fn route(&mut self, who: &str) -> Route {
        let route = Route::start(&self.service_address().to_string());
        let url = format!("https://{}", route.address);
        self.host_mut(who)
            .vars
            .insert("INBOX_URL".to_owned(), url.into());
        route
    }
}

impl Drop for TwoHosts {
    fn drop(&mut self) {
        self.service = None;
        self.front = None;
        let _ = fs::remove_dir_all(&self.dir);
    }
}

/// What a directory held at one time ([`TwoHosts::snapshot`]).
pub struct Snapshot(BTreeMap<PathBuf, Entry>);

/// One entry of a directory tree, with what it holds.
#[derive(Debug, PartialEq, Eq)]
enum Entry {
    Directory,
    File(Vec<u8>),
    Link(PathBuf),
    Other,
}

/// Every entry under `dir`, by its path below `dir`, with what each file holds.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Entry> {
    let mut found = BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(at) = todo.pop() {
        let Ok(entries) = fs::read_dir(&at) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            let kind = if meta.file_type().is_symlink() {
                Entry::Link(fs::read_link(&path).unwrap_or_default())
            } else if meta.is_dir() {
                todo.push(path.clone());
                Entry::Directory
            } else if meta.is_file() {
                Entry::File(fs::read(&path).unwrap_or_default())
            } else {
                Entry::Other
            };
            let below = path.strip_prefix(dir).unwrap_or(&path).to_path_buf();
            found.insert(below, kind);
        }
    }
    found
}

fn assert_unchanged(dir: &Path, before: &BTreeMap<PathBuf, Entry>, what: &str) {
    let after = tree(dir);
    let changed: Vec<String> = before
        .keys()
        .chain(after.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        changed.is_empty(),
        "{what} wrote in the directory of the other client, below it: {changed:?}"
    );
}

/// A client command started by [`TwoHosts::spawn`]. Dropped unfinished, it is killed and
/// waited for.
pub struct Started {
    pid: u32,
    child: Option<Child>,
}

impl Started {
    /// Whether it still runs.
    ///
    /// # Panics
    ///
    /// It was finished, or it cannot be asked.
    pub fn running(&mut self) -> bool {
        self.child
            .as_mut()
            .expect("not finished yet")
            .try_wait()
            .expect("the process is asked")
            .is_none()
    }

    /// The environment it runs with, read from `/proc/<pid>/environ`; only
    /// [`TwoHosts::assert_environment`] reads it, and prints no value of it.
    ///
    /// # Panics
    ///
    /// The file cannot be read.
    fn environment(&self) -> BTreeMap<String, String> {
        let bytes = fs::read(format!("/proc/{}/environ", self.pid)).expect("its environ is read");
        bytes
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                let text = String::from_utf8_lossy(entry);
                let (name, value) = text.split_once('=').unwrap_or((&text, ""));
                (name.to_owned(), value.to_owned())
            })
            .collect()
    }

    /// Its working directory, read from `/proc/<pid>/cwd`.
    ///
    /// # Panics
    ///
    /// The link cannot be read.
    #[must_use]
    pub fn working_directory(&self) -> PathBuf {
        fs::read_link(format!("/proc/{}/cwd", self.pid)).expect("its cwd is read")
    }

    /// Waits for its end and answers what it wrote; [`HANG`] only ends a hang.
    ///
    /// # Panics
    ///
    /// It did not end within [`HANG`] (it is then killed), or it cannot be waited for.
    #[must_use]
    pub fn finish(mut self) -> Run {
        let child = self.child.take().expect("not finished yet");
        let (sender, receiver) = mpsc::channel();
        let waiter = std::thread::spawn(move || {
            let _ = sender.send(child.wait_with_output());
        });
        let Ok(output) = receiver.recv_timeout(HANG) else {
            let _ = Command::new("kill")
                .args(["-KILL", &self.pid.to_string()])
                .status();
            let _ = waiter.join();
            panic!(
                "process {} did not end within {} s",
                self.pid,
                HANG.as_secs()
            );
        };
        let _ = waiter.join();
        Run::from(output.expect("the process is waited for"))
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl From<Output> for Run {
    fn from(output: Output) -> Self {
        Self {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

/// What a [`Route`] saw of one connection it carried. It reads no byte of it: over TLS it sees
/// only how many bytes went each way and in which order.
#[derive(Clone, Copy, Debug, Default)]
pub struct Carried {
    /// Bytes the service sent to the client.
    pub from_service: usize,
    /// Bytes the client sent after the service's first bytes had been carried to it: its answer
    /// to the service's part of the handshake, and its request after it.
    pub from_client_after_service: usize,
    /// The service ended its side of the connection.
    pub service_ended: bool,
}

type Record = Arc<(Mutex<Vec<Carried>>, Condvar)>;

/// A TCP forwarder on `127.0.0.1:0` to the service: each connection a client makes to it is
/// carried to a connection of its own to the service, unchanged; the target is read when the
/// connection is accepted. Dropped, it accepts no more.
pub struct Route {
    address: SocketAddr,
    target: Arc<Mutex<String>>,
    record: Record,
    stop: Arc<AtomicBool>,
}

impl Route {
    fn start(target: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the route binds");
        let address = listener.local_addr().expect("the route has an address");
        let record: Record = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let target = Arc::new(Mutex::new(target.to_owned()));
        let (kept, stopped, to) = (Arc::clone(&record), Arc::clone(&stop), Arc::clone(&target));
        std::thread::spawn(move || {
            for client in listener.incoming() {
                if stopped.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(client) = client else {
                    continue;
                };
                let target = to.lock().unwrap_or_else(PoisonError::into_inner).clone();
                let at = note(&kept, None, |_| {});
                let kept = Arc::clone(&kept);
                std::thread::spawn(move || carry(client, &target, &kept, at));
            }
        });
        Self {
            address,
            target,
            record,
            stop,
        }
    }

    /// Carries each later connection to `target`.
    fn to(&self, target: &str) {
        *self.target.lock().unwrap_or_else(PoisonError::into_inner) = target.to_owned();
    }

    /// A copy of what it saw of each connection so far.
    ///
    /// # Panics
    ///
    /// The record is poisoned.
    #[must_use]
    pub fn carried(&self) -> Vec<Carried> {
        self.record.0.lock().expect("the record").clone()
    }

    /// How many connections it carried so far.
    ///
    /// # Panics
    ///
    /// The record is poisoned.
    #[must_use]
    pub fn connections(&self) -> usize {
        self.record.0.lock().expect("the record").len()
    }

    /// Waits until `fact` holds of the connections carried so far, woken by each change of
    /// them; [`HANG`] only ends a hang.
    ///
    /// # Panics
    ///
    /// It does not hold within [`HANG`].
    pub fn wait_until(&self, what: &str, mut fact: impl FnMut(&[Carried]) -> bool) {
        let (record, changed) = &*self.record;
        let held = record.lock().expect("the record");
        let (held, waited) = changed
            .wait_timeout_while(held, HANG, |carried| !fact(carried))
            .expect("the record");
        let carried = format!("{:?}", *held);
        drop(held);
        assert!(
            !waited.timed_out(),
            "{what}: not within {} s; carried: {carried}",
            HANG.as_secs()
        );
    }
}

impl Drop for Route {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
    }
}

/// Changes the record of connection `at` (a new one when `None`), wakes every waiter, and
/// answers the connection's index.
fn note(record: &Record, at: Option<usize>, change: impl FnOnce(&mut Carried)) -> usize {
    let (carried, changed) = &**record;
    let mut carried = carried.lock().unwrap_or_else(PoisonError::into_inner);
    let at = at.unwrap_or_else(|| {
        carried.push(Carried::default());
        carried.len() - 1
    });
    change(&mut carried[at]);
    changed.notify_all();
    at
}

/// Carries one connection of a [`Route`] to `target`. The service's bytes are counted before
/// they are written to the client, so every byte the client sends in answer to them is counted
/// after them.
fn carry(client: TcpStream, target: &str, record: &Record, at: usize) {
    let Ok(service) = TcpStream::connect(target) else {
        note(record, Some(at), |carried| carried.service_ended = true);
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    let (Ok(mut from_client), Ok(mut to_service)) = (client.try_clone(), service.try_clone())
    else {
        return;
    };
    let kept = Arc::clone(record);
    let up = std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match from_client.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    note(&kept, Some(at), |carried| {
                        if carried.from_service > 0 {
                            carried.from_client_after_service += read;
                        }
                    });
                    if to_service.write_all(&buffer[..read]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = to_service.shutdown(Shutdown::Write);
    });
    let (mut from_service, mut to_client) = (service, client);
    let mut buffer = [0_u8; 4096];
    loop {
        match from_service.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                note(record, Some(at), |carried| carried.from_service += read);
                if to_client.write_all(&buffer[..read]).is_err() {
                    break;
                }
            }
        }
    }
    note(record, Some(at), |carried| carried.service_ended = true);
    let _ = to_client.shutdown(Shutdown::Both);
    let _ = from_service.shutdown(Shutdown::Both);
    let _ = up.join();
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
