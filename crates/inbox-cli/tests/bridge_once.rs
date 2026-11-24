//! The cases of story:the-chat-bridge-asks-a-decision-once: `inbox bridge` posts a message once
//! per message id, across restarts of the service and of the bridge; at `expires` the service
//! answers a decision by silence and the bridge answers nothing in the person's place; one line
//! in the thread of the chat post says how a message nobody answered ended; and the record of the
//! chat posts drops an entry once its message is gone and the bridge's clock passed its `expires`
//! (for an entry of a build before, the reading at which the bridge found it gone) plus the
//! retention. A record with a line that is no entry is one the bridge cannot read, and a reply
//! that fails is sent again with the same key.
//!
//! Every case runs against a real `inbox serve` of the debug build with `--clock-file` and
//! `--sweep-interval 1`, the cases that count the bridge's requests behind one forwarder that
//! keeps what each connection carried (a restart of the service is a new `inbox serve` the
//! forwarder is put in front of), and a stand-in
//! `connectors`, a POSIX shell script first on `PATH` that records each invocation: a post, a
//! reply (a post with `thread_ts`) and a read of the channel, each numbered apart. No case reaches
//! a chat provider or a network beyond loopback, and no case holds a wall-clock time: the poll
//! runs every second, a case waits on the stand-in's record of an invocation or on a fact, with a
//! bound that only ends a hang, and moves a clock by writing its file. The keys are the key set
//! of the run (`support::keys_only`); bob is the person.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use inbox::server::json::{self, Value};
use inbox_server::{format_instant, format_reading};
use support::{BOB, Service, World, keys_only};

/// The chat channel of the binding.
const CHANNEL: &str = "C0000000001";
/// Another chat channel, of a bridge started with another binding.
const OTHER_CHANNEL: &str = "C0000000002";
/// The allowed chat user of the binding.
const USER: &str = "U0000000001";
/// The target `connectors invoke` is called with.
const CONNECTOR: &str = "slack";
/// How long a case waits for a fact or an exit: it only ends a hang.
const HANG: Duration = support::STAND_IN_WAIT;
/// A reaction of the allowed chat user, as the provider lists the reactions of a post.
const REACTED: &str = r#"[{"name":"eyes","users":["U0000000001"],"count":1}]"#;

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

/// A stand-in `connectors` in `<case>/connectors/`: `bin/connectors` and, per invocation `n`
/// (from 1), `args.<n>` and `input.<n>`. A post (`chat.postMessage` whose input has no
/// `thread_ts`) is numbered `p` from 1 in `post.<p>` (which holds `n`) and answered as a post made
/// with the reference `1700000000.<p, 6 digits>`; a reply (`chat.postMessage` whose input has
/// `thread_ts`) is numbered `q` from 1 in `reply.<q>` and answered with the reference
/// `1700000001.<q, 6 digits>`, unless the file `fail-reply.<thread_ts>` is there: then it is
/// numbered `f` from 1 in `failed.<f>` and fails (exit 1, a line on standard error). A read
/// (`conversations.history`) is numbered `r` from 1 in `read.<r>` and answered with `history`,
/// else a page without a post: the posts after `oldest`, and the one at `oldest` only with
/// `"inclusive":true`, as the provider lists them.
struct Connectors {
    dir: PathBuf,
}

impl Connectors {
    fn new(world: &World) -> Self {
        let dir = world.dir.join("connectors");
        fs::create_dir_all(dir.join("bin")).expect("the stand-in's directory is made");
        let shown = dir.to_str().expect("a UTF-8 path");
        assert!(!shown.contains('\''), "{shown}");
        let script = format!(
            r#"#!/bin/sh
d='{shown}'
n=1
while [ -e "$d/args.$n" ]; do n=$((n + 1)); done
cat > "$d/input.$n"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args.$n.new"
mv "$d/args.$n.new" "$d/args.$n"
number() {{
  k=1
  while [ -e "$d/$1.$k" ]; do k=$((k + 1)); done
  echo "$n" > "$d/$1.$k.new"
  mv "$d/$1.$k.new" "$d/$1.$k"
}}
case "$5" in
chat.postMessage)
  if grep -q '"thread_ts"' "$d/input.$n"; then
    t=$(sed -n 's/.*"thread_ts":"\([0-9.]*\)".*/\1/p' "$d/input.$n")
    if [ -n "$t" ] && [ -e "$d/fail-reply.$t" ]; then
      number failed
      echo 'the provider refused the reply' >&2
      exit 1
    fi
    number reply
    printf '{{"result":{{"mutation":{{"classification":"applied"}},"result":{{"status":200,"body":{{"ok":true,"channel":"{CHANNEL}","ts":"1700000001.%06d"}}}}}}}}\n' "$k"
  else
    number post
    printf '{{"result":{{"mutation":{{"classification":"applied"}},"result":{{"status":200,"body":{{"ok":true,"channel":"{CHANNEL}","ts":"1700000000.%06d"}}}}}}}}\n' "$k"
  fi
  exit 0
  ;;
conversations.history)
  if [ -e "$d/history" ]; then
    o=$(sed -n 's/.*"oldest":"\([0-9.]*\)".*/\1/p' "$d/input.$n")
    inc=0
    if grep -q '"inclusive":true' "$d/input.$n"; then inc=1; fi
    awk -v o="$o" -v inc="$inc" '
      /^\{{"type":"message"/ {{
        ts = $0; sub(/.*"ts":"/, "", ts); sub(/".*/, "", ts)
        if (o == "" || (ts "") > (o "") || (inc == 1 && (ts "") == (o ""))) {{ printf "%s%s", sep, $0; sep = "," }}
        next
      }}
      {{ printf "%s", $0 }}
      END {{ print "" }}' "$d/history"
  else
    printf '%s\n' '{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[],"has_more":false}}}}}}}}'
  fi
  number read
  exit 0
  ;;
esac
exit 3
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

    /// Writes the file `name` whole: beside it, then renamed over it.
    fn file(&self, name: &str, text: &str) {
        let staged = self.dir.join(format!("{name}.staged"));
        fs::write(&staged, text).expect("a file of the stand-in is written");
        fs::rename(&staged, self.dir.join(name)).expect("a file of the stand-in is replaced");
    }

    /// How many of `<prefix>.<k>` there are, from 1 on.
    fn count(&self, prefix: &str) -> usize {
        (1..)
            .take_while(|k| self.dir.join(format!("{prefix}.{k}")).exists())
            .count()
    }

    /// The posts so far, replies not counted.
    fn posts(&self) -> usize {
        self.count("post")
    }

    /// The replies in the thread of a post so far.
    fn replies(&self) -> usize {
        self.count("reply")
    }

    /// The reads of the channel so far: one per poll that has an open post.
    fn reads(&self) -> usize {
        self.count("read")
    }

    /// The invocation number of `kind` `k` (`post`, `reply` or `read`).
    fn invocation(&self, kind: &str, k: usize) -> usize {
        let text = fs::read_to_string(self.dir.join(format!("{kind}.{k}")))
            .expect("the number is recorded");
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

    fn input(&self, n: usize) -> Value {
        let text =
            fs::read_to_string(self.dir.join(format!("input.{n}"))).expect("the input is recorded");
        json::parse(&text).unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }
}

/// The answer of a read of the channel that lists `messages` (each its reference and its
/// reactions as JSON), newest first, one post per line, and no further page.
fn history(messages: &[(&str, &str)]) -> String {
    let listed: Vec<String> = messages
        .iter()
        .rev()
        .map(|(ts, reactions)| {
            format!(
                r#"{{"type":"message","user":"U0000000009","text":"a post","ts":"{ts}","reactions":{reactions}}}"#
            )
        })
        .collect();
    format!(
        "{}\n{}\n{}\n",
        r#"{"result":{"result":{"status":200,"body":{"ok":true,"messages":["#,
        listed.join("\n"),
        r#"],"has_more":false,"response_metadata":{"next_cursor":""}}}}}"#
    )
}

/// What each connection of a [`Forwarder`] carried: what the client sent.
type Carried = Arc<Mutex<Vec<Vec<u8>>>>;

/// A TCP forwarder in front of the service: each connection is carried to a new connection to the
/// service it is put in front of ([`Forwarder::to`]), and what the client sent is kept per
/// connection, in the order the connections came. The client sends one request per connection.
struct Forwarder {
    address: String,
    kept: Carried,
    target: Arc<Mutex<String>>,
    stop: Arc<AtomicBool>,
}

impl Forwarder {
    fn start(url: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the forwarder binds");
        let address = listener.local_addr().expect("an address").to_string();
        let kept: Carried = Arc::new(Mutex::new(Vec::new()));
        let target = Arc::new(Mutex::new(url.trim_start_matches("http://").to_owned()));
        let stop = Arc::new(AtomicBool::new(false));
        let (record, to, stopped) = (Arc::clone(&kept), Arc::clone(&target), Arc::clone(&stop));
        std::thread::spawn(move || {
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < 6 * HANG {
                let Some(client) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                let at = {
                    let mut kept = record.lock().expect("the record");
                    kept.push(Vec::new());
                    kept.len() - 1
                };
                let record = Arc::clone(&record);
                let target = to.lock().expect("the target").clone();
                std::thread::spawn(move || carry(client, &target, &record, at));
            }
        });
        Self {
            address,
            kept,
            target,
            stop,
        }
    }

    /// Puts the forwarder in front of the service at `url`: a new start of it.
    fn to(&self, url: &str) {
        *self.target.lock().expect("the target") = url.trim_start_matches("http://").to_owned();
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    /// What each connection so far sent, in the order the connections came.
    fn sent(&self) -> Vec<Vec<u8>> {
        self.kept.lock().expect("the record").clone()
    }

    /// The numbers of the connections from `from` on that sent the command `command` for
    /// `message_id`.
    fn commands(&self, from: usize, command: &str, message_id: &str) -> Vec<usize> {
        let head = format!("POST /messaging/commands/{command} ");
        self.sent()
            .iter()
            .enumerate()
            .skip(from)
            .filter(|(_, sent)| {
                sent.starts_with(head.as_bytes())
                    && String::from_utf8_lossy(sent).contains(message_id)
            })
            .map(|(at, _)| at)
            .collect()
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Carries one connection of a [`Forwarder`] to `target`, keeping what the client sent as
/// connection `at`.
fn carry(client: TcpStream, target: &str, kept: &Carried, at: usize) {
    let Ok(upstream) = TcpStream::connect(target) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    let (Ok(mut from_client), Ok(mut to_upstream)) = (client.try_clone(), upstream.try_clone())
    else {
        return;
    };
    let sent = Arc::clone(kept);
    let up = std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        loop {
            match from_client.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    sent.lock().expect("the record")[at].extend_from_slice(&buffer[..read]);
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
    let _ = from_upstream.shutdown(Shutdown::Both);
    let _ = up.join();
}

/// The system clock's second.
fn this_second() -> inbox_verify::Instant {
    inbox_verify::Instant::from_unix(
        inbox_verify::Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

/// Sets the clock file `name` of the case's directory whole.
fn set_clock(dir: &Path, name: &str, at: inbox_verify::Instant) {
    let staged = dir.join(format!("{name}.new"));
    fs::write(&staged, format_reading(at)).expect("the clock file is written");
    fs::rename(&staged, dir.join(name)).expect("the clock file is replaced");
}

/// A new start of `inbox serve` in `dir`: its clock the file `clock` of the case, a sweep of the
/// expiry task every second.
fn serve(dir: &Path, operator_public_key: &str) -> Service {
    Service::start(
        dir,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            operator_public_key,
            "--sweep-interval",
            "1",
            "--clock-file",
            "clock",
        ],
        &[],
    )
}

/// The world of a case: the key set of the run, the service's clock file and the bridge's, both
/// at the system clock's second, and a service over them.
fn clocked(name: &str) -> World {
    let (dir, operator_public_key) = keys_only(name);
    set_clock(&dir.path, "clock", this_second());
    set_clock(&dir.path, "bridge-clock", this_second());
    let service = serve(&dir.path, &operator_public_key);
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// A running `inbox bridge` of bob that polls every second, its standard error in a file; ended
/// with SIGTERM and waited for when dropped.
struct Bridge {
    child: Option<Child>,
    stderr: PathBuf,
}

impl Bridge {
    /// The bridge of bob bound to `channel` with the allowed chat user and the connector of the
    /// cases and `extra` flags, the stand-in first on `PATH`, and the service at `url`.
    fn start(
        world: &World,
        connectors: &Connectors,
        url: &str,
        channel: &str,
        extra: &[&str],
    ) -> Self {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, Ordering::SeqCst);
        let stderr = world.dir.join(&format!("bridge-{n}.err"));
        let child = world
            .client_command("bob", &["bridge"])
            .args([
                "--chat-channel",
                channel,
                "--allowed-chat-user",
                USER,
                "--chat-connector",
                CONNECTOR,
                "--chat-poll-interval",
                "1",
            ])
            .args(extra)
            .env("PATH", connectors.path())
            .env("INBOX_URL", url)
            .stdout(std::process::Stdio::null())
            .stderr(fs::File::create(&stderr).expect("the stderr file is made"))
            .spawn()
            .expect("the binary starts");
        Self {
            child: Some(child),
            stderr,
        }
    }

    fn stderr(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    /// Ends it with SIGTERM, waits for it, and answers its standard error.
    fn end(mut self) -> String {
        let mut child = self.child.take().expect("running");
        let sent = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(sent.success(), "kill -TERM failed");
        let status = child.wait().expect("the bridge is waited for");
        assert!(
            status.code().is_none(),
            "it ran until the signal: {status:?} {}",
            self.stderr()
        );
        self.stderr()
    }

    /// Waits until it ends by itself ([`HANG`] only ends a hang); answers its exit code and its
    /// standard error.
    fn exit(mut self) -> (Option<i32>, String) {
        let mut status = None;
        wait_for("the bridge ends", || {
            status = self
                .child
                .as_mut()
                .expect("running")
                .try_wait()
                .expect("the bridge is waited for");
            status.is_some()
        });
        self.child = None;
        (status.and_then(|status| status.code()), self.stderr())
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

/// `who` runs `args` against the service at `url`.
fn client(world: &World, url: &str, who: &str, args: &[&str]) -> support::Run {
    world.client_env(who, args, &[("INBOX_URL", url)]).ok()
}

/// alice posts a message of `kind` with `body` and the flags `extra` to bob through `url`;
/// answers its id.
fn post(world: &World, url: &str, kind: &str, body: &str, extra: &[&str]) -> String {
    let mut args = vec!["post", "--to", BOB, "--kind", kind, "--body", body];
    match kind {
        "request" => args.extend(["--reply-shape", "free"]),
        "decision" => args.extend(["--option", "A=Ship", "--option", "B=Hold"]),
        _ => {}
    }
    args.extend(extra);
    client(world, url, "alice", &args).stdout.trim().to_owned()
}

/// The state of `message_id` in bob's `Pending`, or an empty text when it is not listed.
fn state(world: &World, url: &str, message_id: &str) -> String {
    support::rows(&client(world, url, "bob", &["pending", "--json"]).json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// The state of `message_id` in alice's `Outbox`, or an empty text when it is not listed.
fn sent_state(world: &World, url: &str, message_id: &str) -> String {
    support::rows(&client(world, url, "alice", &["outbox", "--json"]).json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// The lines of the bridge's record of the chat posts beside bob's outbox file, as JSON.
fn record(world: &World) -> Vec<Value> {
    let found: Vec<PathBuf> = fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bob.outbox.chat.") && !name.ends_with(".new")
        })
        .collect();
    let Some(path) = found.first() else {
        return Vec::new();
    };
    assert_eq!(found.len(), 1, "one record file: {found:?}");
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| json::parse(line).unwrap_or_else(|error| panic!("{error}: {line}")))
        .collect()
}

/// The line of the record of `message_id`, if it holds one.
fn entry(world: &World, message_id: &str) -> Option<Value> {
    record(world)
        .into_iter()
        .find(|line| support::text(line, "message_id") == message_id)
}

/// The member `name` of the line of the record of `message_id`, or an empty text.
fn recorded(world: &World, message_id: &str, name: &str) -> String {
    entry(world, message_id)
        .and_then(|line| match line.member(name) {
            Some(Value::Text(text)) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// Waits until the record holds `message_id` posted and the message is `Delivered`; answers the
/// reference of its chat post.
fn posted(world: &World, url: &str, message_id: &str) -> String {
    wait_for(&format!("{message_id} is posted and delivered"), || {
        recorded(world, message_id, "state") == "posted"
            && state(world, url, message_id) == "Delivered"
    });
    recorded(world, message_id, "reference")
}

/// A new start of `inbox serve` behind `forwarder`; the start before is ended.
fn restart(world: &mut World, forwarder: &Forwarder) {
    let next = serve(&world.dir.path, &world.operator_public_key);
    forwarder.to(&next.url);
    let before = std::mem::replace(&mut world.service, next);
    assert!(before.terminate().success());
}

/// The lines of a bridge's standard error but the line of a reconnect. After a new start of the
/// service the bridge opens bob's inbox at it again: on the answer `inbox not open` to its read of
/// `Pending` it does so without a line, after another answer that names the new start it writes
/// `inbox: reconnect at the start …`. Which of its requests reaches the new start first is a
/// matter of timing, so that line may come or not.
fn not_reconnect(said: &str) -> Vec<&str> {
    said.lines()
        .filter(|line| !line.starts_with("inbox: reconnect at the start "))
        .collect()
}

/// Writes the line of `message_id` in bob's record of the chat posts as the build of the story
/// before wrote it: no `thread`, no `expires`. The record is replaced whole with mode 0600, as the
/// bridge replaces it.
fn as_of_the_build_before(world: &World, message_id: &str) {
    let path = fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bob.outbox.chat.") && !name.ends_with(".new")
        })
        .expect("the record is written");
    let text = fs::read_to_string(&path).expect("the record is read");
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            let value = json::parse(line).unwrap_or_else(|error| panic!("{error}: {line}"));
            if support::text(&value, "message_id") != message_id {
                return line.to_owned();
            }
            format!(
                r#"{{"message_id":"{message_id}","channel":"{}","reference":"{}","state":"{}"}}"#,
                support::text(&value, "channel"),
                support::text(&value, "reference"),
                support::text(&value, "state")
            )
        })
        .collect();
    let staged = world.dir.join("record.staged");
    fs::write(&staged, format!("{}\n", lines.join("\n"))).expect("the record is written");
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).expect("mode 0600");
    fs::rename(&staged, &path).expect("the record is replaced");
}

/// Checks 2 to 4 and the acceptance: a decision with `default_on_silence`, a second one that
/// expires in 30 s, a request and a note are posted once each. The allowed chat user reacts to
/// the request and to the note, so the request is in `Read` and the note handled, and the record
/// holds both as read. After a restart of the service the bridge finds the 4 gone (their end
/// `gone`), and alice's next command posts the 4 again under their ids: the bridge sends
/// `MarkDelivered` for each and, for the request and the note, `MarkRead` after it; the 2
/// decisions are `Delivered`, the request `Read` in alice's `Outbox`, the note gone from it; no
/// decision gets a `read`; the reposted messages are followed again (their end taken off), so the
/// second decision, decided on silence at its `expires`, gets its line. Then the bridge is ended,
/// the service started again, the 3 left posted again with no bridge running, and a new bridge
/// bound to another channel finds them in `Sent`: it posts none of them, since its record holds
/// them as posted into the first channel, and marks them the same way. The stand-in records one
/// post per message id in all.
#[test]
fn a_message_posted_again_after_a_restart_is_marked_and_never_posted_twice() {
    let mut world = clocked("bridge-once-repost");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::start(&world, &connectors, &url, CHANNEL, &[]);
    // 30 s after the system clock just before the posts: the clients sign with the system clock
    // and the service takes a request within 60 s of its reading.
    let expires_at = this_second().plus_seconds(30);
    let expires = format_instant(expires_at);
    let decision = post(
        &world,
        &url,
        "decision",
        "Ship build 7?",
        &["--default-on-silence", "B"],
    );
    let silence = post(
        &world,
        &url,
        "decision",
        "Ship build 6?",
        &["--expires", &expires, "--default-on-silence", "B"],
    );
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    posted(&world, &url, &decision);
    posted(&world, &url, &silence);
    let request_ts = posted(&world, &url, &request);
    let note_ts = posted(&world, &url, &note);
    connectors.file(
        "history",
        &history(&[(request_ts.as_str(), REACTED), (note_ts.as_str(), REACTED)]),
    );
    wait_for("the request and the note are read", || {
        recorded(&world, &request, "state") == "read" && recorded(&world, &note, "state") == "read"
    });
    assert_eq!(sent_state(&world, &url, &request), "Read");

    // A restart of the service, while the bridge runs: the new start holds no message, so a poll
    // finds no row of the 4, before their `expires`.
    let from = forwarder.sent().len();
    restart(&mut world, &forwarder);
    wait_for("the bridge finds the 4 gone", || {
        [&decision, &silence, &request, &note]
            .iter()
            .all(|id| recorded(&world, id, "end") == "gone")
    });
    wait_for("alice's next command posts the 4 again", || {
        client(&world, &url, "alice", &["outbox"]);
        !state(&world, &url, &decision).is_empty()
    });
    wait_for("the 4 are marked as before the restart", || {
        state(&world, &url, &decision) == "Delivered"
            && state(&world, &url, &silence) == "Delivered"
            && sent_state(&world, &url, &request) == "Read"
            && sent_state(&world, &url, &note).is_empty()
    });
    // The note is not looked at here: handled again, the next poll finds it gone again.
    wait_for("the reposted messages are followed again", || {
        [&decision, &silence, &request]
            .iter()
            .all(|id| recorded(&world, id, "end").is_empty())
    });
    for message_id in [&request, &note] {
        let delivered = forwarder.commands(from, "mark-delivered", message_id);
        let read = forwarder.commands(from, "read", message_id);
        // At least once: the poll sends the `read` too when its read of `Pending` fell between
        // the `MarkDelivered` and the `read` of the round.
        assert!(!read.is_empty(), "a MarkRead of {message_id}");
        assert!(
            delivered.first().is_some_and(|first| *first < read[0]),
            "MarkDelivered before MarkRead of {message_id}: {delivered:?} {read:?}"
        );
    }
    for message_id in [&decision, &silence] {
        assert_eq!(
            forwarder.commands(from, "mark-delivered", message_id).len(),
            1,
            "{message_id}"
        );
        assert!(
            forwarder.commands(0, "read", message_id).is_empty(),
            "no MarkRead of a decision nobody read: {message_id}"
        );
    }
    assert_eq!(connectors.posts(), 4, "no second post after the restart");

    // The second decision is decided on silence at its `expires`: followed again, it gets its
    // line.
    set_clock(&world.dir.path, "clock", expires_at.plus_seconds(1));
    wait_for("the line of the decision taken on silence", || {
        connectors.replies() >= 1
    });
    let (text, key, message_id) = reply(&world, &connectors, 1);
    assert_eq!(message_id, silence);
    assert_eq!(text, "taken on silence: option B, Hold");
    assert_eq!(key, format!("inbox-{silence}-end"));
    wait_for("the decision taken on silence is at its end", || {
        recorded(&world, &silence, "end") == "decided"
    });
    client(&world, &url, "alice", &["collect", &silence]);
    let said = bridge.end();
    assert!(not_reconnect(&said).is_empty(), "{said}");

    // A new bridge bound to another channel, after another restart while no bridge runs: it
    // finds the 3 in `Sent`.
    let from = forwarder.sent().len();
    restart(&mut world, &forwarder);
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    wait_for("alice's next command posts the 3 again", || {
        client(&world, &url, "alice", &["outbox"]);
        state(&world, &url, &decision) == "Sent"
    });
    let bridge = Bridge::start(&world, &connectors, &url, OTHER_CHANNEL, &[]);
    wait_for("the new bridge marks the 3", || {
        state(&world, &url, &decision) == "Delivered"
            && sent_state(&world, &url, &request) == "Read"
            && sent_state(&world, &url, &note).is_empty()
    });
    assert_eq!(
        connectors.posts(),
        4,
        "the new bridge posts no message its record holds, of any channel"
    );
    assert!(forwarder.commands(from, "read", &decision).is_empty());
    assert_eq!(forwarder.commands(from, "read", &request).len(), 1);
    for message_id in [&decision, &request, &note] {
        assert_eq!(
            recorded(&world, message_id, "channel"),
            CHANNEL,
            "{message_id}"
        );
    }
    assert_eq!(connectors.replies(), 1);
    assert_eq!(bridge.end(), "");
}

/// The text a reply of the stand-in's reply `q` carried, its idempotency key, and the message id
/// whose chat post's reference is its `thread_ts`.
fn reply(world: &World, connectors: &Connectors, q: usize) -> (String, String, String) {
    let n = connectors.invocation("reply", q);
    let args = connectors.args(n);
    let input = connectors.input(n);
    let body = input.member("body").expect("the input has a body");
    let Value::Object(members) = body else {
        panic!("the body is no object: {input:?}");
    };
    let names: Vec<&str> = members.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["channel", "text", "thread_ts"], "{input:?}");
    assert_eq!(support::text(body, "channel"), CHANNEL);
    let thread_ts = support::text(body, "thread_ts");
    let message_id = record(world)
        .iter()
        .find(|line| support::text(line, "reference") == thread_ts)
        .map(|line| support::text(line, "message_id"))
        .unwrap_or_else(|| panic!("no post has the reference {thread_ts}"));
    assert_eq!(
        args[..7],
        [
            "--output",
            "json",
            "invoke",
            CONNECTOR,
            "chat.postMessage",
            "--input-stdin",
            "--idempotency-key",
        ],
        "{args:?}"
    );
    (support::text(body, "text"), args[7].clone(), message_id)
}

/// The option silence takes in the case of checks 5 and 6: its text holds a mention, a link, a
/// line end, backticks, an ampersand and a control character.
const HOLD: &str = "B=Hold <!here> <https://example.invalid|the log>\nnow & `then`\u{b}";

/// The line of silence of [`HOLD`]: its text escaped as in the head line of a post.
const HOLD_LINE: &str = "taken on silence: option B, Hold &lt;!here&gt; \
                         &lt;https://example.invalid|the log&gt;\\u{a}now &amp; \
                         \\u{60}then\\u{60}\\u{b}";

/// Checks 5 and 6. A decision with `default_on_silence` B, a decision without one, a request and
/// a note expire unanswered; a request bob answers in a terminal and a note bob reads in a
/// terminal before their `expires`; a note with the default `expires` keeps the poll reading the
/// channel. The bridge's clock is a file of its own, moved past `expires` before the service's.
/// The service decides the first decision on silence and removes the rest; the bridge sends no
/// `answer`, `read` or `refuse`, and alice's `inbox collect` gets `answered_by` silence and
/// option B. The thread of each chat post gets one line: the option, its text escaped as a post's
/// text and "taken on silence" for the decision on silence, "expired, not answered" for the 3
/// that expired, none for the answered request, the read note and a decision bob refuses in a
/// terminal (its end is `answered`, as an answered message's); each reply in the thread of its
/// post, under the key `inbox-<message id>-end`. A new bridge writes none of them again.
#[test]
fn a_message_nobody_answered_gets_one_line_at_its_end_and_none_in_the_persons_place() {
    let world = clocked("bridge-once-silence");
    let forwarder = Forwarder::start(&world.service.url);
    let direct = world.service.url.clone();
    client(&world, &direct, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let flags = ["--clock-file", "bridge-clock"];
    let bridge = Bridge::start(&world, &connectors, &forwarder.url(), CHANNEL, &flags);
    // 30 s after the system clock just before the posts: the clients sign with the system clock
    // and the service takes a request within 60 s of its reading.
    let expires_at = this_second().plus_seconds(30);
    let expires = format_instant(expires_at);
    let at = ["--expires", expires.as_str()];
    let silence = client(
        &world,
        &direct,
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--kind",
            "decision",
            "--body",
            "Ship build 7?",
            "--option",
            "A=Ship",
            "--option",
            HOLD,
            "--expires",
            &expires,
            "--default-on-silence",
            "B",
        ],
    )
    .stdout
    .trim()
    .to_owned();
    let decision = post(&world, &direct, "decision", "Ship build 8?", &at);
    let request = post(&world, &direct, "request", "Which build is green?", &at);
    let note = post(&world, &direct, "note", "Build 7 is green.", &at);
    let answered = post(&world, &direct, "request", "Is build 9 green?", &at);
    let read = post(&world, &direct, "note", "Build 9 is green.", &at);
    let refused = post(&world, &direct, "decision", "Ship build 11?", &at);
    let open = post(&world, &direct, "note", "Build 10 is green.", &[]);
    for message_id in [
        &silence, &decision, &request, &note, &answered, &read, &refused, &open,
    ] {
        posted(&world, &direct, message_id);
    }
    client(
        &world,
        &direct,
        "bob",
        &["answer", &answered, "--body", "Yes."],
    );
    client(&world, &direct, "bob", &["read", &read]);
    client(
        &world,
        &direct,
        "bob",
        &["refuse", &refused, "--reason", "Not mine."],
    );
    wait_for("the answer, the read and the refusal are seen", || {
        recorded(&world, &answered, "end") == "answered"
            && recorded(&world, &read, "end") == "gone"
            && recorded(&world, &refused, "end") == "answered"
    });

    set_clock(&world.dir.path, "bridge-clock", expires_at.plus_seconds(1));
    set_clock(&world.dir.path, "clock", expires_at.plus_seconds(1));
    wait_for("4 lines in the threads", || connectors.replies() >= 4);
    wait_for("the 4 are at their end", || {
        recorded(&world, &silence, "end") == "decided"
            && [&decision, &request, &note]
                .iter()
                .all(|id| recorded(&world, id, "end") == "expired")
    });
    let collected = client(&world, &direct, "alice", &["collect", &silence, "--json"]).json();
    let message = collected.member("message").expect("the settled record");
    assert_eq!(support::text(message, "answered_by"), "silence");
    assert_eq!(support::text(message, "decided_option"), "B");
    for command in ["answer", "read", "refuse"] {
        for message_id in [
            &silence, &decision, &request, &note, &answered, &read, &refused,
        ] {
            assert!(
                forwarder.commands(0, command, message_id).is_empty(),
                "the bridge sent `{command}` for {message_id}"
            );
        }
    }
    let mut lines: Vec<(String, String)> = (1..=4)
        .map(|q| {
            let (text, key, message_id) = reply(&world, &connectors, q);
            assert_eq!(key, format!("inbox-{message_id}-end"));
            (message_id, text)
        })
        .collect();
    lines.sort();
    let mut expected = vec![
        (silence.clone(), HOLD_LINE.to_owned()),
        (decision.clone(), "expired, not answered".to_owned()),
        (request.clone(), "expired, not answered".to_owned()),
        (note.clone(), "expired, not answered".to_owned()),
    ];
    expected.sort();
    assert_eq!(lines, expected);
    assert_eq!(bridge.end(), "");

    // A new bridge writes no line again.
    let bridge = Bridge::start(&world, &connectors, &forwarder.url(), CHANNEL, &flags);
    let reads = connectors.reads();
    wait_for("2 polls of the new bridge", || {
        connectors.reads() >= reads + 2
    });
    assert_eq!(connectors.replies(), 4, "one line per message");
    assert_eq!(connectors.posts(), 8);
    assert_eq!(recorded(&world, &refused, "end"), "answered");
    assert_eq!(bridge.end(), "");
}

/// A record of the chat posts whose line of a posted request is cut short, as no write of the
/// bridge leaves it. The poll fails, with one line that names the record and the line, and is
/// tried again. After a restart of the service the sender's repost of the request reaches the
/// rounds: they post nothing and mark nothing and end the bridge with 4 and one line that names
/// the request, the record and the line. The request stays in `Sent`, the record as it was.
#[test]
fn a_held_line_that_is_no_entry_fails_the_poll_and_ends_the_bridge_at_a_repost() {
    let mut world = clocked("bridge-once-cut-line");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::start(&world, &connectors, &url, CHANNEL, &[]);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    posted(&world, &url, &request);
    let path = fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bob.outbox.chat.") && !name.ends_with(".new")
        })
        .expect("the record is written");
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let line = fs::read_to_string(&path).expect("the record is read");
    let cut = format!("{}\n", &line.trim_end()[..line.trim_end().len() - 2]);
    let staged = world.dir.join("record.staged");
    fs::write(&staged, &cut).expect("the record is written");
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).expect("mode 0600");
    fs::rename(&staged, &path).expect("the record is replaced");
    let named = format!("{name}: line 1 is no entry of the record");
    wait_for("the poll fails on the record", || {
        bridge.stderr().contains(&named)
    });

    restart(&mut world, &forwarder);
    wait_for("alice's next command posts the request again", || {
        client(&world, &url, "alice", &["outbox"]);
        !state(&world, &url, &request).is_empty()
    });
    let (code, said) = bridge.exit();
    assert_eq!(code, Some(4), "{said}");
    let lines = not_reconnect(&said);
    assert_eq!(lines.len(), 2, "{said}");
    assert!(
        lines[0].contains("the read of the reactions in the chat channel")
            && lines[0].contains(&named),
        "{said}"
    );
    assert!(
        lines[1].contains(&format!(
            "{request} is not posted into the chat channel and not marked delivered: the record \
             of the chat posts cannot be read: "
        )) && lines[1].contains(&named),
        "{said}"
    );
    assert_eq!(connectors.posts(), 1);
    assert_eq!(state(&world, &url, &request), "Sent");
    assert_eq!(fs::read_to_string(&path).expect("the record is read"), cut);
}

/// A line at the end of a message whose reply `connectors` fails. A note expiring at 20 s and a
/// note with the default `expires` are posted; the stand-in fails every reply into the thread of
/// the first; both clocks move past its `expires`. The bridge stays up: while the reply fails, at
/// a poll after a poll, it posts a third note and marks the second read on a reaction of the
/// allowed chat user. One line on standard error names the failure, once for the run of
/// failures, and the entry is not at its end. Once the stand-in accepts again, a later poll sends
/// the reply with the same key and the same input as the failed tries, and only then is the
/// entry at its end `expired`; no line follows.
#[test]
fn a_reply_that_fails_is_sent_again_with_the_same_key_and_stops_nothing() {
    let world = clocked("bridge-once-reply-fails");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::start(
        &world,
        &connectors,
        &url,
        CHANNEL,
        &["--clock-file", "bridge-clock"],
    );
    let expires_at = this_second().plus_seconds(20);
    let expires = format_instant(expires_at);
    let ending = post(
        &world,
        &url,
        "note",
        "Build 14 is green.",
        &["--expires", &expires],
    );
    let ending_ts = posted(&world, &url, &ending);
    let open = post(&world, &url, "note", "Build 15 is green.", &[]);
    let open_ts = posted(&world, &url, &open);
    connectors.file(&format!("fail-reply.{ending_ts}"), "");

    set_clock(&world.dir.path, "bridge-clock", expires_at.plus_seconds(1));
    set_clock(&world.dir.path, "clock", expires_at.plus_seconds(1));
    wait_for("the reply fails", || connectors.count("failed") >= 1);
    let later = post(&world, &url, "note", "Build 16 is green.", &[]);
    posted(&world, &url, &later);
    connectors.file("history", &history(&[(open_ts.as_str(), REACTED)]));
    wait_for("the second note is read", || {
        recorded(&world, &open, "state") == "read"
    });
    wait_for("the reply fails at a poll after", || {
        connectors.count("failed") >= 2
    });
    assert_eq!(connectors.replies(), 0);
    assert_eq!(
        recorded(&world, &ending, "end"),
        "",
        "not at its end while its line is not written"
    );
    let said = bridge.stderr();
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), 1, "{said}");
    assert!(
        lines[0].contains(&format!("the line at the end of {ending} "))
            && lines[0].contains("failed"),
        "{said}"
    );

    fs::remove_file(connectors.dir.join(format!("fail-reply.{ending_ts}")))
        .expect("the stand-in accepts again");
    wait_for("the reply goes through", || connectors.replies() >= 1);
    wait_for("the note is at its end", || {
        recorded(&world, &ending, "end") == "expired"
    });
    let (text, key, message_id) = reply(&world, &connectors, 1);
    assert_eq!(
        (message_id.as_str(), text.as_str(), key.clone()),
        (
            ending.as_str(),
            "expired, not answered",
            format!("inbox-{ending}-end")
        )
    );
    let accepted = connectors.invocation("reply", 1);
    let read_input = |n: usize| {
        fs::read(connectors.dir.join(format!("input.{n}"))).expect("the input is recorded")
    };
    for f in 1..=connectors.count("failed") {
        let failed = connectors.invocation("failed", f);
        assert_eq!(
            connectors.args(failed),
            connectors.args(accepted),
            "try {f}"
        );
        assert_eq!(read_input(failed), read_input(accepted), "try {f}");
    }
    assert_eq!(connectors.replies(), 1);
    assert_eq!(connectors.posts(), 3);
    assert_eq!(bridge.end(), said);
}

/// Check 7. With the retention at 1 h and the bridge's clock a file of its own: a note expiring
/// at 40 s posted first, a note expiring at 20 s, and a note whose record line is rewritten as
/// the build of the story before wrote it (no `expires`), which bob then reads in a terminal. The
/// bridge finds that note gone and records the reading of its clock as `gone_at`, with no end
/// (it does not know its `expires`), and does not drop it then. Past both `expires` each expiring note gets its line,
/// the old one none. At the first note's `expires` plus 1 h and 1 s the first note and the old
/// one (gone 1 h and 21 s before) leave the record in one poll, and the note expiring at 40 s,
/// whose line precedes theirs, is kept: its own `expires` plus 1 h is 19 s later.
#[test]
fn an_entry_leaves_the_record_once_its_message_is_gone_and_the_retention_has_passed() {
    let world = clocked("bridge-once-retention");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let started = this_second();
    set_clock(&world.dir.path, "bridge-clock", started);
    let bridge = Bridge::start(
        &world,
        &connectors,
        &url,
        CHANNEL,
        &[
            "--clock-file",
            "bridge-clock",
            "--chat-post-retention",
            "3600",
        ],
    );
    // Within 60 s of the system clock: the clients sign with it, and the service, whose clock
    // moves past both, takes a request within 60 s of its reading.
    let first_at = started.plus_seconds(20);
    let later_at = started.plus_seconds(40);
    let (first_expires, later_expires) = (format_instant(first_at), format_instant(later_at));
    // One after the other, so their lines are in this order in the record.
    let later = post(
        &world,
        &url,
        "note",
        "Build 11 is green.",
        &["--expires", &later_expires],
    );
    posted(&world, &url, &later);
    let first = post(
        &world,
        &url,
        "note",
        "Build 12 is green.",
        &["--expires", &first_expires],
    );
    posted(&world, &url, &first);
    let old = post(&world, &url, "note", "Build 13 is green.", &[]);
    posted(&world, &url, &old);
    let order: Vec<String> = record(&world)
        .iter()
        .map(|line| support::text(line, "message_id"))
        .collect();
    assert_eq!(order, [later.clone(), first.clone(), old.clone()]);
    as_of_the_build_before(&world, &old);
    client(&world, &url, "bob", &["read", &old]);
    let gone_at = format_reading(started);
    wait_for("the bridge finds the old entry's message gone", || {
        recorded(&world, &old, "gone_at") == gone_at
    });
    assert_eq!(recorded(&world, &old, "expires"), "");
    assert_eq!(recorded(&world, &old, "end"), "");

    set_clock(&world.dir.path, "bridge-clock", later_at.plus_seconds(1));
    set_clock(&world.dir.path, "clock", later_at.plus_seconds(1));
    wait_for("the 2 expiring notes are at their end", || {
        recorded(&world, &later, "end") == "expired" && recorded(&world, &first, "end") == "expired"
    });
    assert_eq!(
        connectors.replies(),
        2,
        "a line for each, none for the old one"
    );
    assert!(entry(&world, &old).is_some(), "the old entry is kept");

    set_clock(&world.dir.path, "bridge-clock", first_at.plus_seconds(3601));
    wait_for("the first note and the old entry leave the record", || {
        entry(&world, &first).is_none() && entry(&world, &old).is_none()
    });
    let left: Vec<String> = record(&world)
        .iter()
        .map(|line| support::text(line, "message_id"))
        .collect();
    assert_eq!(
        left,
        std::slice::from_ref(&later),
        "the note expiring later is kept until its own retention has passed"
    );
    assert_eq!(connectors.posts(), 3);
    assert_eq!(connectors.replies(), 2);
    assert_eq!(bridge.end(), "");
}
