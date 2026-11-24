//! Fixtures of the binary's tests: a fresh directory per test, the built `inbox` run in it, a
//! running `inbox serve` that is ended when it goes out of scope (a failed assertion included),
//! and a world of an operator key, two client keys and a key file made with the binary itself.
//! No key material is written into this file: every key is made at run time, the set of
//! [`keys_only`] once per test run ([`key_set`]).

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

    /// Waits up to `limit` for the exit it makes itself; `None` when it is still running then.
    /// The limit only ends a hang.
    pub fn exit_within(&mut self, limit: std::time::Duration) -> Option<ExitStatus> {
        let started = std::time::Instant::now();
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                return Some(status);
            }
            if started.elapsed() >= limit {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
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

impl From<Command> for Spawned {
    /// `command` started with its standard output and standard error piped, not waited for.
    fn from(mut command: Command) -> Self {
        Self(Some(
            command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the binary starts"),
        ))
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

/// The files of the key set of [`keys_only`]; beside them in the entry, `operator.pub` holds the
/// operator public key.
const KEY_SET: [&str; 4] = ["operator.key", "alice.key", "bob.key", "keys.json"];

/// A directory with an operator key and a key file holding the keys of alice and bob, and no
/// service; answers the operator public key. The files are copies of the key set of this test
/// run ([`key_set`]), each of the directory's own, with the mode the binary gave them.
pub fn keys_only(name: &str) -> (Dir, String) {
    let dir = Dir::new(name);
    let set = key_set();
    for file in KEY_SET {
        fs::copy(set.join(file), dir.join(file))
            .unwrap_or_else(|error| panic!("{file} is copied from {}: {error}", set.display()));
    }
    let operator_public_key =
        fs::read_to_string(set.join("operator.pub")).expect("the operator public key is read");
    (dir, operator_public_key)
}

/// The key set of this test run: a cache entry under the target's temporary directory,
/// `key-set/<key>/`, made by the first case that asks for it and read by every other.
///
/// Making it starts the binary 5 times (3 `inbox key new`, 2 `inbox keyfile add-key`), each
/// writing and syncing a file; cargo-nextest runs each case in a process of its own, so a set
/// made in memory would be made once per case. The key ([`key_set_key`]) names the binary and
/// this test binary (path, length, modification time, inode) and the day: another build makes
/// another set, and a set is never older than a day, so its entries (valid 30 days from when
/// `add-key` ran) are valid for 29 days past any case's start.
pub fn key_set() -> PathBuf {
    key_set_in(&Path::new(env!("CARGO_TARGET_TMPDIR")).join("key-set"))
}

/// The key set under `cache`, made when it is not there. Several processes ask at once: only the
/// holder of an exclusive lock on `<key>.lock` makes it, in `<key>.partial` (a leftover of a run
/// that was killed is removed first), and renames that to `<key>`; the others wait on the lock and
/// then find it. A directory named by the key is complete, because it appears by one rename.
pub fn key_set_in(cache: &Path) -> PathBuf {
    let key = key_set_key();
    let done = cache.join(&key);
    if done.is_dir() {
        return done;
    }
    fs::create_dir_all(cache).expect("the cache of the key set is created");
    let lock = File::create(cache.join(format!("{key}.lock"))).expect("the lock file is created");
    lock.lock().expect("the lock of the key set is taken");
    if !done.is_dir() {
        let partial = cache.join(format!("{key}.partial"));
        let _ = fs::remove_dir_all(&partial);
        fs::create_dir_all(&partial).expect("the partial key set is created");
        let operator_public_key = make_keys(&partial);
        fs::write(partial.join("operator.pub"), operator_public_key)
            .expect("the operator public key is written");
        fs::rename(&partial, &done).expect("the key set is complete");
    }
    done
}

/// The name of the key set of this build on this day: the day since the epoch and a hash of the
/// path, length, modification time and inode of the binary and of this test binary.
fn key_set_key() -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    use std::os::unix::fs::MetadataExt;
    let mut hasher = DefaultHasher::new();
    let this = std::env::current_exe().expect("the test binary is named");
    for program in [Path::new(BIN), this.as_path()] {
        let meta = fs::metadata(program).expect("the program is there");
        program.hash(&mut hasher);
        (meta.len(), meta.mtime(), meta.mtime_nsec(), meta.ino()).hash(&mut hasher);
    }
    let day = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("the clock is after the epoch")
        .as_secs()
        / 86_400;
    format!("{day}-{:016x}", hasher.finish())
}

/// Makes the key set in `dir` with the binary: the operator key, the keys of alice and bob, and
/// the key file holding both; answers the operator public key.
fn make_keys(dir: &Path) -> String {
    let members = [("alice.key", "alice-1", ALICE), ("bob.key", "bob-1", BOB)];
    // The 3 keys are made at once, each by a process of its own; the key file is then written by
    // one `add-key` after the other.
    let (operator_public_key, public_keys) = std::thread::scope(|scope| {
        let made: Vec<_> = members
            .iter()
            .map(|(file, key_id, principal)| {
                scope.spawn(|| key_new(dir, file, Some((key_id, principal))))
            })
            .collect();
        let operator = key_new(dir, "operator.key", None);
        let keys: Vec<String> = made
            .into_iter()
            .map(|key| key.join().expect("the key is made"))
            .collect();
        (operator, keys)
    });
    for ((_, key_id, principal), public_key) in members.iter().zip(&public_keys) {
        add_key(dir, "operator.key", key_id, principal, public_key);
    }
    operator_public_key
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

/// The principal of the chat bridge's own key in the cases of
/// story:the-chat-bridge-runs-with-its-own-key-under-a-delegation. Its secret key file is
/// `chat-bridge.key`, so `World::client("chat-bridge", …)` signs with it.
pub const CHAT_BRIDGE: &str = "agent:demo/chat-bridge";
/// The key id of the chat bridge's own key.
pub const CHAT_BRIDGE_KEY_ID: &str = "chat-bridge-1";

/// Who signs the requests of `inbox bridge` in a case of the bridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signer {
    /// A key of the person: the bridge as the stories before ran it.
    Person,
    /// The bridge's own key (`chat-bridge.key`, principal [`CHAT_BRIDGE`]) with
    /// `--for <person> --authority <delegation>`, under that delegation of the key file.
    Own {
        person: &'static str,
        delegation: &'static str,
    },
}

impl Signer {
    /// Before the service starts: for [`Signer::Own`] the bridge's key and its entry in
    /// `keys.json` of `dir` (once), and the delegation of the bridge for the person.
    pub fn prepare(self, dir: &Path) {
        if let Self::Own { person, delegation } = self {
            chat_bridge_key(dir, None);
            add_delegation(dir, delegation, CHAT_BRIDGE, person, None);
        }
    }

    /// Makes `command`, an `inbox bridge` (or `inbox wake`) of the person, one of this signer:
    /// for [`Signer::Own`] the bridge's key file and `--for <person> --authority <delegation>`.
    /// The outbox file stays the one the command names.
    pub fn apply(self, command: &mut Command) {
        if let Self::Own { person, delegation } = self {
            command
                .env("INBOX_SECRET_KEY_FILE", "chat-bridge.key")
                .args(["--for", person, "--authority", delegation]);
        }
    }
}

/// The bridge's own key `chat-bridge.key` in `dir` and its entry in `keys.json`, valid until
/// `until` (30 days from now without one); nothing when the key file is there already.
pub fn chat_bridge_key(dir: &Path, until: Option<&str>) {
    if dir.join("chat-bridge.key").exists() {
        return;
    }
    let public_key = key_new(
        dir,
        "chat-bridge.key",
        Some((CHAT_BRIDGE_KEY_ID, CHAT_BRIDGE)),
    );
    let mut args = vec![
        "keyfile",
        "add-key",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "operator.key",
        "--key-id",
        CHAT_BRIDGE_KEY_ID,
        "--principal",
        CHAT_BRIDGE,
        "--public-key",
        public_key.as_str(),
    ];
    if let Some(until) = until {
        args.extend(["--valid-until", until]);
    }
    run(dir, &args, &[]).ok();
}

/// `inbox keyfile add-delegation` into `keys.json` of `dir`, valid until `until` (30 days from
/// now without one).
pub fn add_delegation(dir: &Path, id: &str, agent: &str, person: &str, until: Option<&str>) {
    let mut args = vec![
        "keyfile",
        "add-delegation",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "operator.key",
        "--delegation-id",
        id,
        "--agent",
        agent,
        "--on-behalf-of",
        person,
    ];
    if let Some(until) = until {
        args.extend(["--valid-until", until]);
    }
    run(dir, &args, &[]).ok();
}

impl World {
    /// [`World::new`] with the key file prepared for `signer` before the service starts.
    pub fn signed(name: &str, signer: Signer) -> Self {
        let (dir, operator_public_key) = keys_only(name);
        signer.prepare(&dir.path);
        let service = serve(&dir.path, &operator_public_key);
        Self {
            service,
            operator_public_key,
            dir,
        }
    }
}

/// The 32 bytes of standard base64 `text` (44 characters, one `=`).
fn base64_32(text: &str) -> [u8; 32] {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = [0_u8; 32];
    let (mut bits, mut count, mut at) = (0_u32, 0, 0);
    for symbol in text.trim_end_matches('=').bytes() {
        let value = ALPHABET
            .iter()
            .position(|known| *known == symbol)
            .expect("a base64 symbol");
        bits = (bits << 6) | u32::try_from(value).expect("6 bits");
        count += 6;
        if count >= 8 {
            count -= 8;
            out[at] = u8::try_from((bits >> count) & 0xFF).expect("a byte");
            at += 1;
        }
        bits &= (1 << count) - 1;
    }
    assert_eq!(at, 32, "32 bytes");
    out
}

/// The rows of the `Inboxes` view of the service at `url` (`http://<host>:<port>`), read with the
/// key `<who>.key` of `dir`: a signed `GET` no command of the binary sends. Each row has
/// `address`, `owner`, `adapter` and `state`.
pub fn inboxes(dir: &Path, who: &str, url: &str) -> Vec<Value> {
    use std::io::{Read, Write};
    let secret = json::parse(
        &fs::read_to_string(dir.join(format!("{who}.key"))).expect("the secret key file is read"),
    )
    .expect("the secret key file is JSON");
    let key = inbox_verify::SecretKey::from_seed(&base64_32(&text(&secret, "seed")));
    let timestamp = inbox_server::format_instant(inbox_verify::Instant::from_system_time(
        std::time::SystemTime::now(),
    ));
    let target = "/messaging/views/inboxes";
    let headers = inbox_server::request_headers(
        "GET",
        target,
        b"",
        &text(&secret, "key_id"),
        &key,
        &timestamp,
    );
    let host = url.trim_start_matches("http://");
    let mut stream = std::net::TcpStream::connect(host).expect("the service is reached");
    let mut head = format!("GET {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n");
    for (name, value) in headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    stream
        .write_all(head.as_bytes())
        .expect("the request is sent");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .expect("the answer is read");
    let (status, body) = answer.split_once("\r\n\r\n").expect("a head and a body");
    assert!(status.starts_with("HTTP/1.1 200"), "{answer}");
    rows(&json::parse(body).expect("the view is JSON"))
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
/// the status `status` and the body `body`; answers its URL. The thread ends after one request,
/// or fails as [`stand_in_answers`] fails when no connection comes.
pub fn stand_in(status: u16, body: &'static str) -> (String, std::thread::JoinHandle<()>) {
    let (url, handle) = stand_in_answers(vec![(status, None, body)]);
    let handle = std::thread::spawn(move || {
        if let Err(panic) = handle.join() {
            std::panic::resume_unwind(panic);
        }
    });
    (url, handle)
}

/// How long a stand-in waits for each connection: a case whose client never connects fails
/// within seconds, naming the answer that was not asked for, and does not wait forever.
pub const STAND_IN_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// The bound of the acceptance: a stand-in whose client never connects ends within 15 s. A longer
/// `STAND_IN_WAIT` does not compile.
const _: () = assert!(STAND_IN_WAIT.as_secs() <= 15);

/// Whether a thread of this process is in write(2) (x86-64 number 1) on the file descriptor
/// `fd`, read from `/proc/self/task/<tid>/syscall`: a writer of a pipe that is full.
pub fn blocked_in_write(fd: std::os::fd::RawFd) -> bool {
    let state = format!("1 {fd:#x} ");
    fs::read_dir("/proc/self/task")
        .into_iter()
        .flatten()
        .flatten()
        .any(|task| {
            fs::read_to_string(task.path().join("syscall"))
                .is_ok_and(|line| line.starts_with(&state))
        })
}

/// Waits until a thread of this process is blocked writing to `fd` ([`blocked_in_write`]): the
/// pipe is full. [`STAND_IN_WAIT`] only ends a hang.
///
/// # Panics
///
/// No thread blocks within it.
pub fn wait_until_blocked_in_write(fd: std::os::fd::RawFd) {
    let started = std::time::Instant::now();
    while !blocked_in_write(fd) {
        assert!(
            started.elapsed() < STAND_IN_WAIT,
            "no thread blocked writing to the pipe within {} s",
            STAND_IN_WAIT.as_secs()
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Waits until `thread` has ended. [`STAND_IN_WAIT`] only ends a hang: a thread whose own wait
/// is shorter ends long before it.
///
/// # Panics
///
/// It has not ended within it.
pub fn wait_until_finished<T>(thread: &std::thread::JoinHandle<T>, what: &str) {
    let started = std::time::Instant::now();
    while !thread.is_finished() {
        assert!(
            started.elapsed() < STAND_IN_WAIT,
            "{what} still waited {} s later",
            STAND_IN_WAIT.as_secs()
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// The next connection on `listener`, or `None` when none comes within `wait`.
pub fn accept_within(
    listener: &std::net::TcpListener,
    wait: std::time::Duration,
) -> Option<std::net::TcpStream> {
    listener
        .set_nonblocking(true)
        .expect("the stand-in's listener does not block");
    let deadline = std::time::Instant::now() + wait;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_nonblocking(false)
                    .expect("the stand-in's connection blocks");
                return Some(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(error) => panic!("the stand-in's listener failed: {error}"),
        }
    }
}

/// The next connection on `listener`; a stand-in that is joined by its case panics with a message
/// that names itself (`who`) and what did not arrive (`what`) when none comes within `wait`.
pub fn accept_or_panic(
    listener: &std::net::TcpListener,
    wait: std::time::Duration,
    who: &str,
    what: &str,
) -> std::net::TcpStream {
    accept_or_say(listener, wait, who, what).unwrap_or_else(|message| panic!("{message}"))
}

/// The next connection on `listener`, or, when none comes within `wait`, a message that names
/// `who`, what did not arrive (`what`) and the wait it was given: a case that passes a short wait
/// asserts the message, so a stand-in that waits longer than it was told fails it.
pub fn accept_or_say(
    listener: &std::net::TcpListener,
    wait: std::time::Duration,
    who: &str,
    what: &str,
) -> Result<std::net::TcpStream, String> {
    accept_within(listener, wait).ok_or_else(|| {
        format!(
            "{who} received no connection for {what} within {} s",
            wait.as_secs()
        )
    })
}

/// Reads one request in full from `stream`: the head, then as many bytes as `Content-Length`
/// says, or what arrives until the stream ends or 10 s pass without a byte.
fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    use std::io::Read;
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
    received
}

/// A stand-in for the service on 127.0.0.1:0 that answers one request after another, one per
/// connection, with the answers in `answers` in turn: the status, the value of an `Inbox-Epoch`
/// header (none when `None`) and the body. Answers its URL; the thread ends after the last answer
/// and hands back the request line of each request it read. When no connection comes for an
/// answer within 10 s the thread fails with a message naming that answer, so joining it fails.
pub fn stand_in_answers(
    answers: Vec<(u16, Option<String>, &'static str)>,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    stand_in_answers_within(answers, STAND_IN_WAIT)
}

/// A stand-in for the service on 127.0.0.1:0 that answers each `GET` with the status 200 and
/// `get`, and the first request of another method with the status 200 and `post`, one request
/// per connection; the thread ends after that answer and hands back the request line of each
/// request it read. When no connection comes within 10 s the thread fails, so joining it fails.
pub fn stand_in_by_method(
    get: String,
    post: String,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        loop {
            let Some(mut stream) = accept_within(&listener, STAND_IN_WAIT) else {
                panic!(
                    "the stand-in received no connection within {} s after {requests:?}",
                    STAND_IN_WAIT.as_secs()
                );
            };
            let received = read_request(&mut stream);
            let line = String::from_utf8_lossy(&received)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            let is_get = line.starts_with("GET ");
            requests.push(line);
            let body = if is_get { &get } else { &post };
            let answer = format!(
                "HTTP/1.1 200 Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(answer.as_bytes());
            if !is_get {
                return requests;
            }
        }
    });
    (url, handle)
}

/// [`stand_in_answers`] with the wait for each connection given: the case that holds the bound
/// passes 1 s, so it does not cost the gate the 10 s every other case allows.
pub fn stand_in_answers_within(
    answers: Vec<(u16, Option<String>, &'static str)>,
    wait: std::time::Duration,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    use std::io::Write;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let handle = std::thread::spawn(move || {
        let mut requests = Vec::new();
        let total = answers.len();
        for (at, (status, epoch, body)) in answers.into_iter().enumerate() {
            let Some(mut stream) = accept_within(&listener, wait) else {
                panic!(
                    "the stand-in received no connection for answer {} of {total} within {} s",
                    at + 1,
                    wait.as_secs()
                );
            };
            let received = read_request(&mut stream);
            requests.push(
                String::from_utf8_lossy(&received)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            );
            let epoch = epoch.map_or_else(String::new, |epoch| format!("Inbox-Epoch: {epoch}\r\n"));
            let answer = format!(
                "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{epoch}Connection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(answer.as_bytes());
        }
        requests
    });
    (url, handle)
}
