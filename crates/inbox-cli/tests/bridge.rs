//! The cases of story:the-chat-bridge-posts-a-pending-message: `inbox bridge` posts each message
//! of the person's inbox into the chat channel of its binding through `connectors`, then marks it
//! delivered. Every case runs against a real `inbox serve` on a loopback address and a stand-in
//! `connectors`, a POSIX shell script the case writes and puts first on `PATH`: it records the
//! arguments, the environment and the input document of each invocation and answers from the
//! files the case gives it. No case reaches a chat provider or a network beyond loopback, and no
//! case holds an upper bound of time: a bounded wait only ends a hang. Every key is made at run
//! time with the binary itself.

use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use inbox::server::json::{self, Value};

use support::{ALICE, Dir, LOCAL, REFUSED, Signer, USAGE, World};

/// The address of the person's inbox the bridge serves.
pub(crate) const PERSON: &str = "human:person";
/// The chat channel of the binding.
const CHANNEL: &str = "C0000000001";
/// The allowed chat user of the binding.
const USER: &str = "U0000000001";
/// The target `connectors invoke` is called with.
const CONNECTOR: &str = "slack";
/// The exit status of a second wake or bridge of the same inbox on the same host.
const ANOTHER_WAKE: i32 = 5;
/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;

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

/// An operator key, the keys of alice and of the person, a key file holding both, and a running
/// service over it.
fn world(name: &str) -> World {
    world_signed(name, Signer::Person)
}

/// [`world`] with the key file prepared for `signer` before the service starts.
fn world_signed(name: &str, signer: Signer) -> World {
    let dir = Dir::new(name);
    let members = [
        ("alice.key", "alice-1", ALICE),
        ("person.key", "person-1", PERSON),
    ];
    let (operator_public_key, public_keys) = std::thread::scope(|scope| {
        let made: Vec<_> = members
            .iter()
            .map(|(file, key_id, principal)| {
                scope.spawn(|| support::key_new(&dir.path, file, Some((key_id, principal))))
            })
            .collect();
        let operator = support::key_new(&dir.path, "operator.key", None);
        let keys: Vec<String> = made
            .into_iter()
            .map(|key| key.join().expect("the key is made"))
            .collect();
        (operator, keys)
    });
    for ((_, key_id, principal), public_key) in members.iter().zip(&public_keys) {
        support::add_key(&dir.path, "operator.key", key_id, principal, public_key);
    }
    signer.prepare(&dir.path);
    let service = support::serve(&dir.path, &operator_public_key);
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// A stand-in `connectors` in a directory of its own under the case's directory: `bin/connectors`
/// and, per invocation `n` (from 1), `args.<n>` (one argument per line), `env.<n>`, `pid.<n>`
/// (its process id) and `input.<n>`. It answers with `reply.<n>`, else `reply`, on its standard
/// output and `err.<n>` on its standard error, and exits with `exit.<n>`, else 0; with
/// `signal.<n>` it ends itself by SIGKILL instead. While `hold` exists, invocation `n` waits for
/// `release.<n>` before it answers. While `helper` exists, invocation `n` first starts a helper in
/// the background, a process of its group that records its id in `helper.<n>` and runs until
/// `release-helper` exists (at most some 30 s, longer than [`HANG`]).
struct Connectors {
    dir: PathBuf,
}

impl Connectors {
    fn new(world: &World, name: &str) -> Self {
        let dir = world.dir.join(name);
        fs::create_dir_all(dir.join("bin")).expect("the stand-in's directory is made");
        let shown = dir.to_str().expect("a UTF-8 path");
        assert!(!shown.contains('\''), "{shown}");
        let script = format!(
            r#"#!/bin/sh
d='{shown}'
n=1
while [ -e "$d/args.$n" ]; do n=$((n + 1)); done
echo $$ > "$d/pid.$n"
date +%s%N > "$d/start.$n"
cat > "$d/input.$n"
env > "$d/env.$n"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args.$n.new"
mv "$d/args.$n.new" "$d/args.$n"
if [ -e "$d/helper" ]; then
  sh -c 'echo $$ > "$1/helper.$2.new"; mv "$1/helper.$2.new" "$1/helper.$2"; i=0; while [ ! -e "$1/release-helper" ] && [ "$i" -lt 3000 ]; do sleep 0.01; i=$((i + 1)); done' sh "$d" "$n" > /dev/null 2>&1 &
fi
if [ -e "$d/hold" ]; then
  i=0
  while [ ! -e "$d/release.$n" ] && [ "$i" -lt 1000 ]; do sleep 0.01; i=$((i + 1)); done
fi
date +%s%N > "$d/end.$n"
if [ -e "$d/reply.$n" ]; then cat "$d/reply.$n"; elif [ -e "$d/reply" ]; then cat "$d/reply"; fi
if [ -e "$d/err.$n" ]; then cat "$d/err.$n" >&2; fi
if [ -e "$d/signal.$n" ]; then kill -KILL $$; fi
if [ -e "$d/exit.$n" ]; then exit "$(cat "$d/exit.$n")"; fi
exit 0
"#
        );
        let program = dir.join("bin/connectors");
        fs::write(&program, script).expect("the stand-in is written");
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
            .expect("the stand-in is executable");
        Self { dir }
    }

    /// `PATH` with the stand-in first.
    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    fn file(&self, name: &str, text: &str) {
        fs::write(self.dir.join(name), text).expect("a file of the stand-in is written");
    }

    /// Answers every invocation without a reply of its own with `text`.
    fn reply(&self, text: &str) {
        self.file("reply", text);
    }

    fn hold(&self) {
        self.file("hold", "");
    }

    fn release(&self, n: usize) {
        self.file(&format!("release.{n}"), "");
    }

    /// The invocations so far.
    fn calls(&self) -> usize {
        (1..)
            .take_while(|n| self.dir.join(format!("args.{n}")).exists())
            .count()
    }

    /// The nanoseconds since the epoch that `<name>.<n>` holds: when invocation `n` started
    /// (`start`) or gave its answer (`end`).
    fn nanos(&self, name: &str, n: usize) -> u128 {
        let text =
            fs::read_to_string(self.dir.join(format!("{name}.{n}"))).expect("the time is recorded");
        text.trim()
            .parse()
            .unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }

    /// The process id of the helper invocation `n` started, once it is recorded.
    fn helper(&self, n: usize) -> u32 {
        let path = self.dir.join(format!("helper.{n}"));
        wait_for(&format!("the helper of invocation {n}"), || path.exists());
        let text = fs::read_to_string(&path).expect("the helper's id is recorded");
        text.trim()
            .parse()
            .unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }

    /// The process id of invocation `n`.
    fn pid(&self, n: usize) -> u32 {
        let text =
            fs::read_to_string(self.dir.join(format!("pid.{n}"))).expect("the pid is recorded");
        text.trim()
            .parse()
            .unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }

    fn args(&self, n: usize) -> Vec<String> {
        fs::read_to_string(self.dir.join(format!("args.{n}")))
            .expect("the arguments are recorded")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn env(&self, n: usize) -> String {
        fs::read_to_string(self.dir.join(format!("env.{n}"))).expect("the environment is recorded")
    }

    /// The input document of invocation `n`, as it arrived.
    fn input_text(&self, n: usize) -> String {
        fs::read_to_string(self.dir.join(format!("input.{n}"))).expect("the input is recorded")
    }

    fn input(&self, n: usize) -> Value {
        let text = self.input_text(n);
        json::parse(&text).unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }

    /// The channel and the text of the input document of invocation `n`.
    fn post(&self, n: usize) -> (String, String) {
        let input = self.input(n);
        let body = input.member("body").expect("the input has a body");
        (support::text(body, "channel"), support::text(body, "text"))
    }
}

/// Whether the process `pid` ends: it is waited for until it no longer runs; [`HANG`] only ends a
/// hang. A signal is delivered after `kill` returns, so a process a signal ends may still run for
/// a moment.
fn ends(pid: u32) -> bool {
    let started = Instant::now();
    while alive(pid) {
        if started.elapsed() >= HANG {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// The process group of the process `pid`.
fn group(pid: u32) -> u32 {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).expect("the process is there");
    let (_, rest) = stat.rsplit_once(')').expect("a stat line");
    rest.split_whitespace()
        .nth(2)
        .and_then(|field| field.parse().ok())
        .unwrap_or_else(|| panic!("no group in {stat:?}"))
}

/// Whether the process `pid` runs: it exists and is no zombie.
fn alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .and_then(|(_, rest)| rest.trim_start().chars().next())
        })
        .is_some_and(|state| state != 'Z')
}

/// The answer of `connectors --output json invoke` to a post with the classification
/// `classification`, the status 200, `ok` and the reference `ts`.
fn answer(classification: &str, ts: &str) -> String {
    format!(
        r#"{{"result":{{"mutation":{{"classification":"{classification}","standing":{{"grant_id":"grant-1"}}}},"result":{{"status":200,"body":{{"ok":true,"channel":"{CHANNEL}","ts":"{ts}"}}}}}}}}"#
    )
}

/// A running `inbox bridge`, its standard output and standard error in files; killed and waited
/// for when dropped unfinished.
struct Bridge {
    child: Option<Child>,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Bridge {
    /// `inbox bridge` of `who` with `args`, `PATH` set to `path`, and the extra variables `env`.
    fn start(world: &World, who: &str, args: &[&str], path: &str, env: &[(&str, &str)]) -> Self {
        Self::start_signed(world, who, args, path, env, Signer::Person)
    }

    /// [`Bridge::start`] signed by `signer`.
    fn start_signed(
        world: &World,
        who: &str,
        args: &[&str],
        path: &str,
        env: &[(&str, &str)],
        signer: Signer,
    ) -> Self {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let stdout = world.dir.join(&format!("bridge-{n}.out"));
        let stderr = world.dir.join(&format!("bridge-{n}.err"));
        let mut command = world.client_command(who, &["bridge"]);
        command.args(args);
        signer.apply(&mut command);
        let child = command
            .env("PATH", path)
            .envs(env.iter().copied())
            .stdout(fs::File::create(&stdout).expect("the stdout file is made"))
            .stderr(fs::File::create(&stderr).expect("the stderr file is made"))
            // The leader of a process group of its own, as a job a shell starts.
            .process_group(0)
            .spawn()
            .expect("the binary starts");
        Self {
            child: Some(child),
            stdout,
            stderr,
        }
    }

    /// The bridge of the person with the binding of the cases.
    fn bound(world: &World, connectors: &Connectors) -> Self {
        Self::bound_signed(world, connectors, Signer::Person)
    }

    /// [`Bridge::bound`] signed by `signer`, with the person's outbox file.
    fn bound_signed(world: &World, connectors: &Connectors, signer: Signer) -> Self {
        Self::start_signed(world, "person", &binding(), &connectors.path(), &[], signer)
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("not finished yet").id()
    }

    fn running(&mut self) -> bool {
        matches!(self.child.as_mut().map(Child::try_wait), Some(Ok(None)))
    }

    fn stderr(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    /// Sends the signal `name` to the process group the bridge leads, as the interrupt key of a
    /// terminal sends SIGINT to the foreground job.
    fn signal_group(&self, name: &str) {
        let sent = Command::new("kill")
            .args([name, "--", &format!("-{}", self.pid())])
            .status()
            .expect("kill runs");
        assert!(sent.success(), "kill {name} to the group failed");
    }

    fn signal(&self, name: &str) {
        let sent = Command::new("kill")
            .args([name, &self.pid().to_string()])
            .status()
            .expect("kill runs");
        assert!(sent.success(), "kill {name} failed");
    }

    /// Waits for its end ([`HANG`] only ends a hang) and answers its status, its standard output
    /// and its standard error.
    fn end(mut self) -> (ExitStatus, String, String) {
        let started = Instant::now();
        let status = loop {
            let child = self.child.as_mut().expect("not finished yet");
            if let Some(status) = child.try_wait().expect("the process is asked") {
                break status;
            }
            assert!(
                started.elapsed() < HANG,
                "the bridge did not end within {} s: {}",
                HANG.as_secs(),
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        self.child = None;
        (
            status,
            fs::read_to_string(&self.stdout).unwrap_or_default(),
            self.stderr(),
        )
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// The flags of the binding of the cases, and a poll of the channel every hour: the stand-in of
/// these cases knows the post only, and the read mark is `bridge_read`'s.
fn binding() -> Vec<&'static str> {
    vec![
        "--chat-channel",
        CHANNEL,
        "--allowed-chat-user",
        USER,
        "--chat-connector",
        CONNECTOR,
        "--chat-poll-interval",
        "3600",
    ]
}

/// alice posts a message of `kind` with `body` to the person; answers its message id.
fn post(world: &World, kind: &str, body: &str) -> String {
    let mut args = vec!["post", "--to", PERSON, "--kind", kind, "--body", body];
    match kind {
        "request" => args.extend(["--reply-shape", "free"]),
        "decision" => args.extend(["--option", "A=Ship", "--option", "B=Hold"]),
        _ => {}
    }
    world.client("alice", &args).ok().stdout.trim().to_owned()
}

/// The person's `Pending`, as rows.
fn pending(world: &World) -> Vec<Value> {
    support::rows(&world.client("person", &["pending", "--json"]).ok().json())
}

/// The state of `message_id` in the person's `Pending`, or an empty text when it is not listed.
fn state(world: &World, message_id: &str) -> String {
    pending(world)
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// The arguments `connectors` is invoked with for the post of `message_id`.
fn invocation(message_id: &str) -> Vec<String> {
    [
        "--output",
        "json",
        "invoke",
        CONNECTOR,
        "chat.postMessage",
        "--input-stdin",
        "--idempotency-key",
        &format!("inbox-{message_id}"),
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The files of `dir` whose name starts with `prefix`, but the staged `.new` of one.
fn files(dir: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with(prefix) && !name.ends_with(".new")
        })
        .collect();
    found.sort();
    found
}

/// The lines of the bridge's record of the chat posts beside the person's outbox file, as JSON.
fn record(world: &World) -> Vec<Value> {
    let found = files(&world.dir.path, "person.outbox.chat.");
    assert_eq!(found.len(), 1, "one record file: {found:?}");
    let mode = fs::metadata(&found[0])
        .expect("the record is there")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "{:o}", mode);
    fs::read_to_string(&found[0])
        .expect("the record is read")
        .lines()
        .map(|line| json::parse(line).unwrap_or_else(|error| panic!("{error}: {line}")))
        .collect()
}

/// The 64-bit FNV-1a hash of `bytes`, as the name of the files of the wake beside the outbox file
/// holds it (`docs/design/wake.md`, "One wake per inbox on a host").
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The inodes of the sockets of the process `pid` that listen: a TCP socket in the state LISTEN,
/// or any UDP socket, read from Linux `/proc`.
fn listening(pid: u32) -> Vec<String> {
    let own: Vec<String> = fs::read_dir(format!("/proc/{pid}/fd"))
        .expect("the descriptors of the bridge are read")
        .flatten()
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .filter_map(|link| {
            link.to_str()
                .and_then(|link| link.strip_prefix("socket:["))
                .and_then(|link| link.strip_suffix(']'))
                .map(str::to_owned)
        })
        .collect();
    let mut found = Vec::new();
    for (table, listen_only) in [
        ("tcp", true),
        ("tcp6", true),
        ("udp", false),
        ("udp6", false),
    ] {
        let text = fs::read_to_string(format!("/proc/{pid}/net/{table}")).unwrap_or_default();
        for line in text.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            let (Some(state), Some(inode)) = (fields.get(3), fields.get(9)) else {
                continue;
            };
            if own.iter().any(|held| held == inode) && (!listen_only || *state == "0A") {
                found.push(format!("{table} {line}"));
            }
        }
    }
    found
}

/// The acceptance and check 2: a note, a request and a decision posted to the person's inbox
/// each give one invocation of the post operation, in the order of `Pending`, each naming the
/// channel of the binding and carrying the kind, the sender and the body; `MarkDelivered` is
/// sent for a message only after the stand-in answered its invocation (the stand-in holds each
/// answer until the case saw the message still in `Sent`), and the message is then `Delivered`.
/// A note posted while the bridge runs makes exactly one more invocation and is `Delivered` with
/// no typed command. Each post is in the record with the reference the stand-in answered; the
/// bridge listens on no socket.
#[test]
fn a_note_a_request_and_a_decision_are_posted_in_the_order_of_pending_and_delivered() {
    a_note_a_request_and_a_decision_are_posted_in_the_order_of_pending_and_delivered_signed(
        Signer::Person,
    );
}

/// The case above, with the bridge signed by `signer`: also run with the bridge's own key in
/// `bridge_own_key`.
pub(crate) fn a_note_a_request_and_a_decision_are_posted_in_the_order_of_pending_and_delivered_signed(
    signer: Signer,
) {
    let world = world_signed("bridge-acceptance", signer);
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.hold();
    let note = post(&world, "note", "Build 7 is green.");
    let request = post(&world, "request", "Which build is green?");
    let decision = post(&world, "decision", "Ship build 7?");
    let order: Vec<String> = pending(&world)
        .iter()
        .map(|row| support::text(row, "message_id"))
        .collect();
    let mut sorted = order.clone();
    sorted.sort();
    let mut posted = vec![note.clone(), request.clone(), decision.clone()];
    posted.sort();
    assert_eq!(sorted, posted, "{order:?}");
    for at in 0..order.len() {
        connectors.file(
            &format!("reply.{}", at + 1),
            &answer("applied", &format!("1700000000.00000{}", at + 1)),
        );
    }

    let mut bridge = Bridge::bound_signed(&world, &connectors, signer);
    for (at, message_id) in order.iter().enumerate() {
        let n = at + 1;
        wait_for(&format!("invocation {n}"), || connectors.calls() >= n);
        assert_eq!(connectors.calls(), n, "one invocation at a time");
        assert_eq!(
            state(&world, message_id),
            "Sent",
            "no mark before the stand-in answered invocation {n}: {}",
            bridge.stderr()
        );
        connectors.release(n);
        wait_for(&format!("{message_id} is delivered"), || {
            state(&world, message_id) == "Delivered"
        });
    }
    for (at, message_id) in order.iter().enumerate() {
        let n = at + 1;
        assert_eq!(connectors.args(n), invocation(message_id), "invocation {n}");
        let input = connectors.input(n);
        let names = |value: &Value| match value {
            Value::Object(members) => members
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>(),
            other => panic!("not an object: {other:?}"),
        };
        assert_eq!(names(&input), ["body"], "nothing beside `body`");
        assert_eq!(
            names(input.member("body").expect("a body")),
            ["channel", "text"],
            "exactly the 2 members the standing approval admits"
        );
        let (channel, text) = connectors.post(n);
        assert_eq!(channel, CHANNEL, "{text}");
        let kind = if *message_id == note {
            ("note", "Build 7 is green.")
        } else if *message_id == request {
            ("request", "Which build is green?")
        } else {
            ("decision", "Ship build 7?")
        };
        let head = text.lines().next().unwrap_or_default();
        assert!(
            head.starts_with(&format!("{} from {ALICE} ", kind.0)),
            "{text}"
        );
        assert!(head.contains(message_id), "{text}");
        assert!(text.contains(&format!("\n```\n{}\n```", kind.1)), "{text}");
        assert!(
            !connectors.env(n).contains(kind.1),
            "no message text in the environment"
        );
    }

    fs::remove_file(connectors.dir.join("hold")).expect("the hold ends");
    connectors.reply(&answer("applied", "1700000000.000004"));
    let fourth = post(&world, "note", "Build 8 is green.");
    wait_for("the fourth note is delivered", || {
        state(&world, &fourth) == "Delivered"
    });
    assert_eq!(
        connectors.calls(),
        4,
        "exactly one invocation for the fourth note"
    );
    assert_eq!(connectors.args(4), invocation(&fourth));
    assert!(connectors.post(4).1.contains("Build 8 is green."));
    assert!(bridge.running(), "{}", bridge.stderr());
    assert_eq!(listening(bridge.pid()), Vec::<String>::new(), "no listener");

    let records = record(&world);
    let expected: Vec<(String, String)> = order
        .iter()
        .cloned()
        .enumerate()
        .map(|(at, id)| (id, format!("1700000000.00000{}", at + 1)))
        .chain([(fourth.clone(), "1700000000.000004".to_owned())])
        .collect();
    let held: Vec<(String, String)> = records
        .iter()
        .map(|line| {
            assert_eq!(support::text(line, "state"), "posted", "{line:?}");
            (
                support::text(line, "message_id"),
                support::text(line, "reference"),
            )
        })
        .collect();
    assert_eq!(held, expected);

    bridge.signal("-TERM");
    let (status, stdout, stderr) = bridge.end();
    assert_eq!(status.signal(), Some(libc::SIGTERM), "{status:?} {stderr}");
    assert_eq!(stdout, "");
    assert_eq!(stderr, "");
}

/// The failure document `connectors` writes on its standard error when it exits 1: `ok` false, the
/// fixed `error.code` `failure`, a message, and under `error.data` the real code, the
/// classification of the post with `attempt` and `standing.grant_id` (the refusal at the recheck
/// before dispatch), and `extra`, members of
/// `error.data` as JSON (none when empty).
fn failure(classification: &str, code: &str, message: &str, extra: &str) -> String {
    let extra = if extra.is_empty() {
        String::new()
    } else {
        format!(",{extra}")
    };
    format!(
        r#"{{"ok":false,"error":{{"code":"failure","message":"{message}","data":{{"code":"{code}","mutation":{{"classification":"{classification}","attempt":{{"number":1}},"standing":{{"grant_id":"grant-1"}}}}{extra}}}}}}}"#
    )
}

impl Connectors {
    /// Invocation `n` exits 1 with `document` on its standard error and nothing on its standard
    /// output.
    fn fail(&self, n: usize, document: &str) {
        self.file(&format!("err.{n}"), document);
        self.file(&format!("exit.{n}"), "1");
    }
}

/// Posts one note per item of `first`, and for each lets invocation `first` fail and the next
/// one succeed: the note stays in `Sent` through both tries and is `Delivered` after them.
fn fail_then_post(world: &World, connectors: &Connectors, bridge: &Bridge, first: &[usize]) {
    for &first in first {
        let note = post(world, "note", &format!("Note {first}"));
        for n in [first, first + 1] {
            wait_for(&format!("try {n}"), || connectors.calls() >= n);
            assert_eq!(
                state(world, &note),
                "Sent",
                "no mark at try {n}: {}",
                bridge.stderr()
            );
            assert_eq!(connectors.args(n), invocation(&note), "try {n}");
            connectors.release(n);
        }
        wait_for(&format!("note {first} is delivered"), || {
            state(world, &note) == "Delivered"
        });
    }
}

/// Check 3: a post that fails sends no `MarkDelivered`, and the bridge tries it again after the
/// pause of a failure, with the same idempotency key. The stand-in exits 1 with a failure
/// document classified `refused` whose message holds an ESC, then exits 1 with a standard error
/// that is not JSON, then exits 0 with an answer classified `not_attempted` (every other member
/// of a success present), then exits 1 classified `unknown`, then answers `applied`. The message
/// stays in `Sent` through the 4 failures; one line on standard error names the run of them, with
/// the message of the first and its ESC as `\u{1b}`; after the fifth try the message is
/// `Delivered` and in the record with the reference of that answer.
#[test]
fn a_failed_post_sends_no_mark_and_is_tried_again_with_the_same_key() {
    let world = world("bridge-failed");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.hold();
    connectors.fail(
        1,
        &failure(
            "refused",
            "provider_error",
            "no standing approval \\u001b[2J for this channel",
            "",
        ),
    );
    connectors.fail(2, "connectors: the connection is not configured\n");
    connectors.file("reply.3", &answer("not_attempted", "1700000000.000003"));
    connectors.fail(4, &failure("unknown", "provider_timeout", "no answer", ""));
    connectors.file("reply.5", &answer("applied", "1700000000.000005"));
    let mut bridge = Bridge::bound(&world, &connectors);
    let note = post(&world, "note", "Is build 7 green?");
    for n in 1..=5 {
        wait_for(&format!("try {n}"), || connectors.calls() >= n);
        assert_eq!(
            state(&world, &note),
            "Sent",
            "no mark at try {n}: {}",
            bridge.stderr()
        );
        connectors.release(n);
    }
    wait_for("the note is delivered", || {
        state(&world, &note) == "Delivered"
    });
    assert_eq!(connectors.calls(), 5);
    for n in 1..=5 {
        assert_eq!(connectors.args(n), invocation(&note), "try {n}");
    }
    let records = record(&world);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(support::text(&records[0], "message_id"), note);
    assert_eq!(support::text(&records[0], "reference"), "1700000000.000005");
    assert!(bridge.running(), "{}", bridge.stderr());
    bridge.signal("-TERM");
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "one line per run of failures: {stderr}");
    assert!(lines[0].starts_with("inbox bridge: "), "{stderr}");
    assert!(lines[0].contains(&note), "{stderr}");
    assert!(
        lines[0].contains("`refused` with the code `provider_error`"),
        "{stderr}"
    );
    assert!(
        lines[0].contains("no standing approval \\u{1b}[2J for this channel"),
        "{stderr}"
    );
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
}

/// Each run of `connectors` that is not a post made and has no failure document is a failed post
/// with a line of its own that says which: an exit 0 whose answer lacks the reference `ts`, an
/// exit 1 whose standard error is not JSON, an exit 2 (a usage error), a child ended by SIGKILL,
/// and an exit 0 whose standard output is longer than the bound the bridge reads. Each note fails
/// once and then is posted; the text of the child is shown with its ESC as `\u{1b}`.
#[test]
fn each_kind_of_failed_run_of_connectors_is_named_on_its_own_line() {
    let world = world("bridge-runs");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.hold();
    connectors.file("reply.1", &answer("applied", ""));
    connectors.fail(3, "panic \u{1b}[2J at the connection\n");
    connectors.file("err.5", "error: unexpected argument \u{1b}[31m\n");
    connectors.file("exit.5", "2");
    connectors.file("signal.7", "");
    connectors.file(
        "reply.9",
        &format!(r#"{{"result":"{}"}}"#, "a".repeat(300_000)),
    );
    for n in [2, 4, 6, 8, 10] {
        connectors.file(
            &format!("reply.{n}"),
            &answer("applied", &format!("1700000000.0000{n:02}")),
        );
    }
    let mut bridge = Bridge::bound(&world, &connectors);
    fail_then_post(&world, &connectors, &bridge, &[1, 3, 5, 7, 9]);
    assert_eq!(connectors.calls(), 10);
    assert!(bridge.running(), "{}", bridge.stderr());
    bridge.signal("-TERM");
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 5, "one line per run of failures: {stderr}");
    for (line, which) in lines.iter().zip([
        "`connectors` exited with 0 and its answer is not a post made: the provider's answer names \
         no reference of the post (`ts`)",
        "`connectors` exited with 1 and its standard error is not one JSON document: panic \
         \\u{1b}[2J at the connection",
        "`connectors` exited with 2, a usage error: error: unexpected argument \\u{1b}[31m",
        "`connectors` was ended by the signal 9",
        "`connectors` exited with 0 and its standard output is longer than 262144 bytes",
    ]) {
        assert!(line.starts_with("inbox bridge: "), "{stderr}");
        assert!(line.contains(which), "{which}: {stderr}");
    }
    assert!(!stderr.contains('\u{1b}'), "{stderr:?}");
}

/// A refusal of `connectors` at the first check of its standing grant: exit 1, nothing on
/// standard output, and on standard error the failure document whose `error.data.mutation` holds
/// the classification `not_attempted` and no `standing` member.
fn first_check(code: &str, message: &str) -> String {
    format!(
        r#"{{"ok":false,"error":{{"code":"failure","message":"{message}","data":{{"code":"{code}","mutation":{{"classification":"not_attempted"}}}}}}}}"#
    )
}

/// The 4 codes of `connectors` for a post it did not make under a standing grant, each in a
/// failure document on its standard error with exit 1 (`error.data.code`, the classification
/// `not_attempted`): `approval_required` and `unavailable` as the refusal of the first check (no
/// `standing` member), `idempotency_conflict` and `rate_limited` with `retry_after_seconds` 1 as
/// the refusal of the recheck before dispatch (`mutation.attempt` and
/// `mutation.standing.grant_id`). Each sends no `MarkDelivered`, the message stays in `Sent`, its
/// run of failures gets one line naming the classification and the code (for
/// `approval_required` also the operator's grant), and the next try succeeds. The try after
/// `rate_limited` starts at least 1 s after the stand-in gave that answer: a lower bound of time,
/// which a slow host cannot break, as the wake cases hold their pause.
#[test]
fn a_post_connectors_did_not_admit_sends_no_mark_and_names_its_code() {
    let world = world("bridge-codes");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.hold();
    connectors.fail(1, &first_check("approval_required", "not admitted"));
    connectors.fail(
        3,
        &failure("not_attempted", "idempotency_conflict", "key reused", ""),
    );
    connectors.fail(
        5,
        &failure(
            "not_attempted",
            "rate_limited",
            "cap reached",
            r#""retry_after_seconds":1"#,
        ),
    );
    connectors.fail(7, &first_check("unavailable", "the clock cannot be read"));
    for n in [2, 4, 6, 8] {
        connectors.file(
            &format!("reply.{n}"),
            &answer("applied", &format!("1700000000.00000{n}")),
        );
    }
    let mut bridge = Bridge::bound(&world, &connectors);
    fail_then_post(&world, &connectors, &bridge, &[1, 3, 5, 7]);
    assert_eq!(connectors.calls(), 8);
    let waited = connectors.nanos("start", 6) - connectors.nanos("end", 5);
    assert!(
        waited >= 1_000_000_000,
        "the try after `rate_limited` came {waited} ns after it"
    );
    assert!(bridge.running(), "{}", bridge.stderr());
    bridge.signal("-TERM");
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 4, "one line per run of failures: {stderr}");
    for (line, code) in lines.iter().zip([
        "approval_required",
        "idempotency_conflict",
        "rate_limited",
        "unavailable",
    ]) {
        assert!(line.starts_with("inbox bridge: "), "{stderr}");
        assert!(line.contains("`connectors` exited with 1"), "{stderr}");
        assert!(
            line.contains(&format!("`not_attempted` with the code `{code}`")),
            "{code}: {stderr}"
        );
        assert!(
            line.contains("tried again after a pause"),
            "{code}: {stderr}"
        );
    }
    assert!(lines[0].contains("grant of `connectors`"), "{stderr}");
    assert!(lines[2].contains("not before 1 s"), "{stderr}");
    assert!(
        lines[3].contains("approval clock cannot be read"),
        "{stderr}"
    );
}

/// A SIGINT or a SIGTERM to the bridge while its `connectors` child runs (the stand-in has started
/// a helper and holds its answer) ends the child's process group with the bridge. The child leads
/// a group of its own, the helper is in it, and the bridge's group is another one. SIGINT goes to
/// the bridge's group, as the interrupt key of a terminal sends it to the foreground job: it
/// reaches the bridge and not the child, and the bridge's signal path ends the child's group.
/// SIGTERM goes to the bridge alone. Once the bridge has ended, neither the child nor the helper
/// runs, and the message stays in `Sent` with nothing recorded.
#[test]
fn a_signal_during_a_post_ends_the_group_of_the_connectors_child_with_the_bridge() {
    for (signal, to_group) in [("-INT", true), ("-TERM", false)] {
        let world = world("bridge-signal");
        world.client("person", &["open", "--adapter", "slack"]).ok();
        let connectors = Connectors::new(&world, "connectors");
        connectors.hold();
        connectors.file("helper", "");
        connectors.reply(&answer("applied", "1700000000.000001"));
        let bridge = Bridge::bound(&world, &connectors);
        let note = post(&world, "note", "Build 7 is green.");
        wait_for(&format!("{signal}: the post runs"), || {
            connectors.calls() >= 1
        });
        let child = connectors.pid(1);
        let helper = connectors.helper(1);
        assert!(alive(child), "{signal}: the stand-in runs");
        assert!(alive(helper), "{signal}: the helper runs");
        assert_eq!(group(child), child, "{signal}: the child leads a group");
        assert_eq!(group(helper), child, "{signal}: the helper is in it");
        assert_ne!(group(bridge.pid()), child, "{signal}: the bridge is not");
        if to_group {
            bridge.signal_group(signal);
        } else {
            bridge.signal(signal);
        }
        let (status, _, stderr) = bridge.end();
        let (child_after, helper_after) = (!ends(child), !ends(helper));
        connectors.release(1);
        connectors.file("release-helper", "");
        assert!(
            !child_after,
            "{signal}: the connectors child {child} outlives the bridge ({status:?}): {stderr}"
        );
        assert!(
            !helper_after,
            "{signal}: the helper {helper} of the child outlives the bridge ({status:?}): {stderr}"
        );
        assert_eq!(state(&world, &note), "Sent", "{signal}");
        assert!(
            files(&world.dir.path, "person.outbox.chat.").is_empty(),
            "{signal}: nothing recorded"
        );
    }
}

/// A helper that a `connectors` child leaves behind when it ends by itself is not the bridge's:
/// the post is made and marked, and a SIGTERM to the bridge after it ends the bridge and leaves
/// the helper running (the signal path ends only the group of a post that runs).
#[test]
fn a_helper_of_a_post_that_ended_by_itself_is_not_ended_by_the_bridge() {
    let world = world("bridge-helper-left");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.file("helper", "");
    connectors.reply(&answer("applied", "1700000000.000001"));
    let bridge = Bridge::bound(&world, &connectors);
    let note = post(&world, "note", "Build 7 is green.");
    wait_for("the note is delivered", || {
        state(&world, &note) == "Delivered"
    });
    let helper = connectors.helper(1);
    assert!(!alive(connectors.pid(1)), "the child has ended");
    bridge.signal("-TERM");
    let (status, _, stderr) = bridge.end();
    let helper_after = alive(helper);
    connectors.file("release-helper", "");
    assert!(
        helper_after,
        "the helper {helper} of a finished post was ended ({status:?}): {stderr}"
    );
    wait_for("the released helper ends", || !alive(helper));
}

/// A bridge started while the service does not answer (a closed loopback port) says so in the
/// line of the wake, under its own name, and keeps trying instead of ending at the open of its
/// start.
#[test]
fn a_bridge_started_before_the_service_answers_says_so_and_keeps_trying() {
    let world = world("bridge-early");
    let connectors = Connectors::new(&world, "connectors");
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a loopback port")
        .local_addr()
        .expect("its address")
        .port();
    let url = format!("http://127.0.0.1:{port}");
    let mut bridge = Bridge::start(
        &world,
        "person",
        &binding(),
        &connectors.path(),
        &[("INBOX_URL", &url)],
    );
    wait_for("the line of a service that does not answer", || {
        bridge
            .stderr()
            .contains("trying again until the service answers")
    });
    assert!(bridge.running(), "{}", bridge.stderr());
    let stderr = bridge.stderr();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "one line for the run of failures: {stderr}");
    assert!(lines[0].starts_with("inbox bridge: "), "{stderr}");
    assert_eq!(connectors.calls(), 0);
}

/// The record of the chat posts is replaced through the staged file `<record>.new` of the outbox
/// files: a symbolic link planted there, pointing at another file of the directory, is removed and
/// not followed. The post is recorded in a regular file of mode 0600, the other file keeps its
/// bytes and its mode, and no staged entry is left.
#[test]
fn a_link_at_the_staged_place_of_the_record_is_removed_not_followed() {
    const OTHER: &[u8] = b"another file of the directory\n";
    let world = world("bridge-staged");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.reply(&answer("applied", "1700000000.000001"));
    let other = world.dir.join("other");
    fs::write(&other, OTHER).expect("the other file is written");
    fs::set_permissions(&other, fs::Permissions::from_mode(0o640)).expect("its mode");
    let staged = world.dir.join(&format!(
        "person.outbox.chat.{:016x}.new",
        fnv1a(PERSON.as_bytes())
    ));
    std::os::unix::fs::symlink("other", &staged).expect("the link is planted");
    let mut bridge = Bridge::bound(&world, &connectors);
    let note = post(&world, "note", "Build 7 is green.");
    wait_for("the note is delivered", || {
        state(&world, &note) == "Delivered"
    });
    let records = record(&world);
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(support::text(&records[0], "message_id"), note);
    assert_eq!(fs::read(&other).expect("the other file"), OTHER);
    let kept = fs::symlink_metadata(&other).expect("the other file");
    assert!(kept.file_type().is_file());
    assert_eq!(kept.permissions().mode() & 0o7777, 0o640);
    assert!(
        fs::symlink_metadata(&staged).is_err(),
        "no staged entry is left"
    );
    assert!(bridge.running(), "{}", bridge.stderr());
}

/// Check 3: the record of a post is written before `MarkDelivered` is sent. With a directory in
/// the place of the record file, the round cannot read the record and makes no post (from
/// story:the-chat-bridge-asks-a-decision-once on, no post without a readable record, since a
/// message it holds is not posted again): the bridge ends with 4 and one line naming the message
/// and the file, and the message stays in `Sent`. The poll that runs at the start cannot read the
/// record either: a failed poll, named in one line of its own that names the file too, written
/// before the round's.
#[test]
fn a_post_that_cannot_be_recorded_is_not_marked_delivered() {
    let world = world("bridge-record");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.reply(&answer("applied", "1700000000.000001"));
    let name = format!("person.outbox.chat.{:016x}", fnv1a(PERSON.as_bytes()));
    fs::create_dir(world.dir.join(&name)).expect("a directory in the place of the record");
    let bridge = Bridge::bound(&world, &connectors);
    wait_for("the poll names the record", || {
        bridge.stderr().contains("the read of the reactions")
    });
    let note = post(&world, "note", "Build 7 is green.");
    let (status, _, stderr) = bridge.end();
    assert_eq!(status.code(), Some(LOCAL), "{stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 2, "the poll's line and the post's: {stderr}");
    assert!(lines[0].contains("the read of the reactions"), "{stderr}");
    assert!(lines[0].contains(&name), "{stderr}");
    assert!(lines[1].contains(&note), "{stderr}");
    assert!(lines[1].contains(&name), "{stderr}");
    assert_eq!(connectors.calls(), 0, "no post without a readable record");
    assert_eq!(state(&world, &note), "Sent", "no mark without the record");
}

/// Check 4: the body of a message reaches the stand-in inside one block of literal text. The
/// channel-wide mention and a user mention of the provider arrive with `<` and `>` escaped
/// (`&lt;`, `&gt;`), only inside the block, a run of 3 backticks (the delimiter of the block) as
/// `\u{60}` each while a run of 2 stays, `&` as `&amp;`, and an ESC and a U+202E as `\u{1b}` and
/// `\u{202e}`, as `rows_text` shows them. A body whose text in the block would be over 3000 bytes
/// is cut at a whole character, its last escape whole, and a line after the block names the
/// message and `inbox pending`.
#[test]
fn a_mention_and_the_delimiter_stay_inside_one_block_and_a_long_body_is_cut() {
    let world = world("bridge-escapes");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world, "connectors");
    connectors.reply(&answer("applied", "1700000000.000001"));
    let hostile = post(
        &world,
        "note",
        "Hi <!channel> and <!here>\n```\nrm -rf /\n``` & <@U0000000002> \u{1b}[2J \u{202e}evil ``two``",
    );
    let long = post(&world, "note", &format!("{}END", "<>".repeat(1500)));
    let mut bridge = Bridge::bound(&world, &connectors);
    for message_id in [&hostile, &long] {
        wait_for(&format!("{message_id} is delivered"), || {
            state(&world, message_id) == "Delivered"
        });
    }
    assert!(bridge.running(), "{}", bridge.stderr());
    let by_id = |message_id: &str| {
        (1..=connectors.calls())
            .find(|n| connectors.args(*n) == invocation(message_id))
            .unwrap_or_else(|| panic!("no invocation for {message_id}"))
    };

    let n = by_id(&hostile);
    let raw = connectors.input_text(n);
    assert!(!raw.contains("<!"), "no mention outside an escape: {raw}");
    assert!(
        !raw.contains('\u{1b}') && !raw.contains('\u{202e}'),
        "{raw:?}"
    );
    let (_, text) = connectors.post(n);
    assert_eq!(text.matches("```").count(), 2, "one block: {text}");
    let (head, rest) = text
        .split_once("\n```\n")
        .expect("a block after the head line");
    let (block, after) = rest.rsplit_once("\n```").expect("the end of the block");
    assert!(!head.contains("&lt;") && !after.contains("&lt;"), "{text}");
    assert_eq!(after, "", "{text}");
    for shown in [
        "Hi &lt;!channel&gt; and &lt;!here&gt;\n",
        "\\u{60}\\u{60}\\u{60}\nrm -rf /\n\\u{60}\\u{60}\\u{60} &amp; &lt;@U0000000002&gt;",
        " \\u{1b}[2J \\u{202e}evil ``two``",
    ] {
        assert!(block.contains(shown), "{shown:?} in {block:?}");
    }
    assert!(!connectors.args(n).iter().any(|arg| arg.contains("rm -rf")));
    assert!(!connectors.env(n).contains("rm -rf"));

    let (_, text) = connectors.post(by_id(&long));
    let (_, rest) = text
        .split_once("\n```\n")
        .expect("a block after the head line");
    let (block, after) = rest.rsplit_once("\n```").expect("the end of the block");
    assert!(block.len() <= 3000, "{}", block.len());
    assert!(block.len() > 2990, "{}", block.len());
    assert_eq!(
        block.replace("&lt;", "").replace("&gt;", ""),
        "",
        "whole escapes only"
    );
    assert!(!text.contains("END"), "{text}");
    assert!(after.contains(&long), "{after}");
    assert!(after.contains("inbox pending"), "{after}");
    assert!(!after.contains('`'), "{after}");
}

/// Check 5: the bridge opens the person's inbox with the adapter `slack` and takes the lock of
/// the wake: while it runs, a second bridge of the same inbox with the same outbox file exits 5,
/// sends nothing and invokes nothing, `inbox wake` exits 5, and `inbox wake --follow` ends with 1
/// naming the adapter `slack`. A bridge of an inbox this host opened with another adapter ends
/// with 1 and one line naming `inbox open --adapter slack`, before the lock and any request.
#[test]
fn a_second_bridge_exits_5_and_the_terminal_adapter_refuses_the_bridges_inbox() {
    let world = world("bridge-lock");
    let connectors = Connectors::new(&world, "connectors");
    connectors.reply(&answer("applied", "1700000000.000001"));
    let mut bridge = Bridge::bound(&world, &connectors);
    let opened = world.dir.join("person.outbox.opened");
    wait_for("the bridge opened the inbox", || {
        fs::read_to_string(&opened).is_ok_and(|text| text.contains(r#""adapter":"slack""#))
    });
    let note = post(&world, "note", "Build 7 is green.");
    wait_for("the note is delivered", || {
        state(&world, &note) == "Delivered"
    });

    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let silent_url = format!("http://{}", silent.local_addr().expect("an address"));
    let second = Connectors::new(&world, "second");
    let (status, stdout, stderr) = Bridge::start(
        &world,
        "person",
        &binding(),
        &second.path(),
        &[("INBOX_URL", &silent_url)],
    )
    .end();
    assert_eq!(status.code(), Some(ANOTHER_WAKE), "{stderr}");
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        format!("inbox bridge: another wake of {PERSON} runs on this host\n")
    );
    assert_eq!(second.calls(), 0, "the second bridge invoked nothing");
    for args in [&["wake"][..], &["wake", "--follow"][..]] {
        let mut command = world.client_command("person", args);
        command.env("INBOX_URL", &silent_url);
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("the binary starts");
        let stderr = String::from_utf8_lossy(&output.stderr);
        if args.len() == 1 {
            assert_eq!(output.status.code(), Some(ANOTHER_WAKE), "{stderr}");
        } else {
            assert_eq!(output.status.code(), Some(REFUSED), "{stderr}");
            assert!(stderr.contains("the adapter slack"), "{stderr}");
        }
    }
    silent.set_nonblocking(true).expect("no blocking");
    assert!(
        matches!(silent.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "nothing was sent while the bridge held the lock"
    );
    assert!(bridge.running(), "{}", bridge.stderr());
    bridge.signal("-TERM");
    drop(bridge.end());

    world.client("person", &["open"]).ok();
    let (status, stdout, stderr) = Bridge::start(
        &world,
        "person",
        &binding(),
        &second.path(),
        &[("INBOX_URL", &silent_url)],
    )
    .end();
    assert_eq!(status.code(), Some(REFUSED), "{stderr}");
    assert_eq!(stdout, "");
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "{stderr}");
    assert!(lines[0].contains("terminal"), "{stderr}");
    assert!(
        lines[0].contains("`inbox open --adapter slack`"),
        "{stderr}"
    );
    assert!(
        matches!(silent.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "the refused bridge sent nothing"
    );
    assert_eq!(second.calls(), 0);
    assert_eq!(
        files(&world.dir.path, "person.outbox.wake").len(),
        1,
        "the file of the lock of the first bridge only"
    );
}

/// One start the bridge refuses: what it is, the arguments, `PATH`, `INBOX_URL`, the exit status
/// and what its line names.
type Refusal<'a> = (&'a str, Vec<String>, String, String, i32, &'a str);

/// Check 6: the bridge refuses to start, with one line on standard error and no request, when a
/// flag of the binding is missing or its chat user is not one id, when `connectors` is not on
/// `PATH`, and when the URL of the service is not a loopback URL (`0.0.0.0`, which the kernel would
/// connect to this host, here to a listener of the case that would see the request).
#[test]
fn the_bridge_refuses_to_start_without_a_binding_connectors_or_a_loopback_url() {
    let world = world("bridge-refusals");
    let connectors = Connectors::new(&world, "connectors");
    let silent = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let port = silent.local_addr().expect("an address").port();
    let silent_url = format!("http://127.0.0.1:{port}");
    let outside = format!("http://0.0.0.0:{port}");
    let empty = world.dir.join("empty");
    fs::create_dir(&empty).expect("an empty directory");
    let empty = empty.to_str().expect("a UTF-8 path").to_owned();
    let without = |flag: &str| {
        let all = binding();
        let at = all.iter().position(|held| *held == flag).expect("a flag");
        all.iter()
            .enumerate()
            .filter(|(index, _)| *index != at && *index != at + 1)
            .map(|(_, held)| (*held).to_owned())
            .collect::<Vec<String>>()
    };
    let with_user = |user: &str| {
        binding()
            .iter()
            .map(|held| if *held == USER { user } else { *held })
            .map(str::to_owned)
            .collect::<Vec<String>>()
    };
    let bound: Vec<String> = binding().iter().map(|held| (*held).to_owned()).collect();
    let cases: Vec<Refusal> = vec![
        (
            "no channel",
            without("--chat-channel"),
            connectors.path(),
            silent_url.clone(),
            USAGE,
            "no binding is configured",
        ),
        (
            "no chat user",
            without("--allowed-chat-user"),
            connectors.path(),
            silent_url.clone(),
            USAGE,
            "no binding is configured",
        ),
        (
            "no connector",
            without("--chat-connector"),
            connectors.path(),
            silent_url.clone(),
            USAGE,
            "no binding is configured",
        ),
        (
            "an empty chat user",
            with_user(""),
            connectors.path(),
            silent_url.clone(),
            USAGE,
            "--allowed-chat-user",
        ),
        (
            "2 chat users",
            with_user("U0000000001 U0000000002"),
            connectors.path(),
            silent_url.clone(),
            USAGE,
            "--allowed-chat-user",
        ),
        (
            "a control character",
            with_user("U0000000001\u{1b}"),
            connectors.path(),
            silent_url.clone(),
            USAGE,
            "--allowed-chat-user",
        ),
        (
            "no connectors",
            bound.clone(),
            empty.clone(),
            silent_url.clone(),
            LOCAL,
            "`connectors` is not on PATH",
        ),
        (
            "not loopback",
            bound,
            connectors.path(),
            outside,
            USAGE,
            "not a loopback address",
        ),
    ];
    for (what, args, path, url, expected, named) in cases {
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let (status, stdout, stderr) =
            Bridge::start(&world, "person", &args, &path, &[("INBOX_URL", &url)]).end();
        assert_eq!(status.code(), Some(expected), "{what}: {stderr}");
        assert_eq!(stdout, "", "{what}");
        let lines: Vec<&str> = stderr.lines().collect();
        assert_eq!(lines.len(), 1, "{what}: {stderr}");
        assert!(lines[0].starts_with("inbox bridge: "), "{what}: {stderr}");
        assert!(lines[0].contains(named), "{what}: {stderr}");
        assert!(!lines[0].contains('\u{1b}'), "{what}: {stderr}");
    }
    silent.set_nonblocking(true).expect("no blocking");
    assert!(
        matches!(silent.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "no refused bridge sent a request"
    );
    assert_eq!(
        connectors.calls(),
        0,
        "no refused bridge invoked connectors"
    );
    assert_eq!(files(&world.dir.path, "person.outbox").len(), 0);
}
