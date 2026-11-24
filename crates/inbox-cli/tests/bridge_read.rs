//! The cases of story:the-chat-bridge-marks-a-message-read-on-a-reaction: `inbox bridge` reads the
//! recent posts of its chat channel through `connectors` at an interval and sends `MarkRead` for
//! each of its posts the allowed chat user reacted to. Every case runs against a real `inbox
//! serve` on a loopback address and a stand-in `connectors`, a POSIX shell script the case writes
//! and puts first on `PATH`: it answers a post with a reference of its own and a read of the
//! channel with the list of posts and reactions the case scripts for it. The bridge's requests go
//! through a forwarder that keeps what each connection carried, so a case counts the `read`
//! requests the service got. No case reaches a chat provider or a network beyond loopback, and no
//! case holds an upper bound of time: the interval of the poll is 1 s, and a bounded wait only
//! ends a hang. The keys are the key set of the run (`support::keys_only`); bob is the person.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use inbox::server::json::{self, Value};
use inbox_server::{format_instant, format_reading};
use support::{ALICE, BOB, Service, Signer, World, keys_only};

/// The chat channel of the binding.
const CHANNEL: &str = "C0000000001";
/// The allowed chat user of the binding.
const USER: &str = "U0000000001";
/// Another chat user of the channel.
const OTHER_USER: &str = "U0000000002";
/// Another chat channel, of a bridge started again with another binding.
const OTHER_CHANNEL: &str = "C0000000002";
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
/// (from 1), `args.<n>`, `input.<n>`, `start.<n>` and `end.<n>` (nanoseconds since the epoch).
/// Each post (`chat.postMessage`) is numbered `p` from 1 in `post.<p>` (which holds `n`) and
/// answered as a post made with the reference `1700000000.<p, 6 digits>`. Each read
/// (`conversations.history`) is numbered `r` from 1 in `read.<r>` and answered with
/// `history.<r>`, else `history`, else a page without a post, on its standard output,
/// `err.<r>` on its standard error, and the exit status `exit.<r>`, else 0. While `hold-post`
/// (`hold-read`) exists a post (a read) writes `holding-post.<p>` (`holding-read.<r>`) and waits
/// for `release-post.<p>` (`release-read.<r>`). An invocation that starts while another one runs
/// writes `overlap`.
struct Connectors {
    dir: PathBuf,
}

impl Connectors {
    fn new(world: &World) -> Self {
        Self::named(world, "connectors")
    }

    /// The stand-in in `<case>/<name>/`: a second one numbers its posts from 1 again.
    fn named(world: &World, name: &str) -> Self {
        let dir = world.dir.join(name);
        fs::create_dir_all(dir.join("bin")).expect("the stand-in's directory is made");
        let shown = dir.to_str().expect("a UTF-8 path");
        assert!(!shown.contains('\''), "{shown}");
        let script = format!(
            r#"#!/bin/sh
d='{shown}'
own=0
if mkdir "$d/busy" 2> /dev/null; then own=1; else : > "$d/overlap"; fi
finish() {{
  if [ "$own" = 1 ]; then rmdir "$d/busy"; fi
  exit "$1"
}}
n=1
while [ -e "$d/args.$n" ]; do n=$((n + 1)); done
date +%s%N > "$d/start.$n"
cat > "$d/input.$n"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args.$n.new"
mv "$d/args.$n.new" "$d/args.$n"
page() {{
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
    END {{ print "" }}' "$1"
}}
hold() {{
  if [ -e "$d/hold-$1" ]; then
    : > "$d/holding-$1.$2"
    i=0
    while [ ! -e "$d/release-$1.$2" ] && [ "$i" -lt 1000 ]; do sleep 0.01; i=$((i + 1)); done
  fi
}}
case "$5" in
chat.postMessage)
  p=1
  while [ -e "$d/post.$p" ]; do p=$((p + 1)); done
  echo "$n" > "$d/post.$p"
  hold post "$p"
  date +%s%N > "$d/end.$n"
  printf '{{"result":{{"mutation":{{"classification":"applied"}},"result":{{"status":200,"body":{{"ok":true,"channel":"{CHANNEL}","ts":"1700000000.%06d"}}}}}}}}\n' "$p"
  finish 0
  ;;
conversations.history)
  r=1
  while [ -e "$d/read.$r" ]; do r=$((r + 1)); done
  echo "$n" > "$d/read.$r"
  hold read "$r"
  date +%s%N > "$d/end.$n"
  if [ -e "$d/history.$r" ]; then page "$d/history.$r"
  elif [ -e "$d/history" ]; then page "$d/history"
  else printf '%s\n' '{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[],"has_more":false}}}}}}}}'
  fi
  if [ -e "$d/err.$r" ]; then cat "$d/err.$r" >&2; fi
  if [ -e "$d/exit.$r" ]; then finish "$(cat "$d/exit.$r")"; fi
  finish 0
  ;;
esac
finish 3
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

    /// Writes the file `name` whole: beside it, then renamed over it, so the stand-in never
    /// reads half of it.
    fn file(&self, name: &str, text: &str) {
        let staged = self.dir.join(format!("{name}.staged"));
        fs::write(&staged, text).expect("a file of the stand-in is written");
        fs::rename(&staged, self.dir.join(name)).expect("a file of the stand-in is replaced");
    }

    fn remove(&self, name: &str) {
        fs::remove_file(self.dir.join(name)).expect("a file of the stand-in is removed");
    }

    fn exists(&self, name: &str) -> bool {
        self.dir.join(name).exists()
    }

    /// How many of `<prefix>.<k>` there are, from 1 on.
    fn count(&self, prefix: &str) -> usize {
        (1..)
            .take_while(|k| self.dir.join(format!("{prefix}.{k}")).exists())
            .count()
    }

    /// The invocations so far.
    fn calls(&self) -> usize {
        self.count("args")
    }

    /// The posts so far.
    fn posts(&self) -> usize {
        self.count("post")
    }

    /// The reads so far.
    fn reads(&self) -> usize {
        self.count("read")
    }

    /// The invocation number of post or read `k` (`kind` is `post` or `read`).
    fn invocation(&self, kind: &str, k: usize) -> usize {
        let path = self.dir.join(format!("{kind}.{k}"));
        wait_for(&format!("{kind} {k} is numbered"), || {
            fs::read_to_string(&path).is_ok_and(|text| text.ends_with('\n'))
        });
        let text = fs::read_to_string(&path).expect("the number is recorded");
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

    /// The operation of invocation `n`.
    fn operation(&self, n: usize) -> String {
        self.args(n).get(4).cloned().unwrap_or_default()
    }

    /// The input document of read `r`.
    fn read_input(&self, r: usize) -> Value {
        let n = self.invocation("read", r);
        let text =
            fs::read_to_string(self.dir.join(format!("input.{n}"))).expect("the input is recorded");
        json::parse(&text).unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }

    /// The nanoseconds since the epoch that `<name>.<n>` holds.
    fn nanos(&self, name: &str, n: usize) -> u128 {
        let text =
            fs::read_to_string(self.dir.join(format!("{name}.{n}"))).expect("the time is recorded");
        text.trim()
            .parse()
            .unwrap_or_else(|error| panic!("{error}: {text:?}"))
    }

    /// Whether two invocations ever ran at once.
    fn overlapped(&self) -> bool {
        self.exists("overlap")
    }

    /// The first read that holds, once one does.
    fn holding_read(&self) -> usize {
        let mut found = 0;
        wait_for("a read holds", || {
            found = (1..=self.reads())
                .find(|r| self.exists(&format!("holding-read.{r}")))
                .unwrap_or(0);
            found != 0
        });
        found
    }
}

/// The answer of `connectors --output json invoke <target> conversations.history`: one page that
/// lists `messages` (each its reference `ts` and, when not empty, its `reactions` as JSON), newest
/// first, and no further page. Each post is a line of its own, so the stand-in lists only those
/// the read asks for, as the provider does: the posts after `oldest`, and the post at `oldest`
/// only with `"inclusive":true`.
fn history(messages: &[(&str, &str)]) -> String {
    let listed: Vec<String> = messages
        .iter()
        .rev()
        .map(|(ts, reactions)| {
            if reactions.is_empty() {
                format!(r#"{{"type":"message","user":"U0000000009","text":"a post","ts":"{ts}"}}"#)
            } else {
                format!(
                    r#"{{"type":"message","user":"U0000000009","text":"a post","ts":"{ts}","reactions":{reactions}}}"#
                )
            }
        })
        .collect();
    format!(
        "{}\n{}\n{}\n",
        r#"{"result":{"result":{"status":200,"body":{"ok":true,"messages":["#,
        listed.join("\n"),
        r#"],"has_more":false,"response_metadata":{"next_cursor":""}}}}}"#
    )
}

/// The refusal `connectors` writes on its standard error when the provider refused a read with
/// the class `class`.
fn refusal(class: &str) -> String {
    format!(
        r#"{{"ok":false,"error":{{"code":"failure","message":"the service refused","data":{{"code":"service_failure","stage":"dispatch","service_code":"{class}"}}}}}}"#
    )
}

/// What each connection of a [`Forwarder`] carried: what the client sent, what the service
/// answered.
type Carried = Arc<Mutex<Vec<(Vec<u8>, Vec<u8>)>>>;

/// What a [`Forwarder`] does with a read of the whole `Pending` view (a `GET` of
/// `/messaging/views/pending` without `undelivered=1`, the read of the poll): with `fail` it
/// answers `503` itself, counted in `failed`; with `hold` it holds the answer of the service, once
/// its first bytes came, until `released` counts it (the held ones are counted from 1 in `held`).
/// And what it does with a `read` request: while `drop_reads` is over 0 it closes the connection
/// without sending the request on or answering it, counts it down and counts it in `dropped`.
#[derive(Default)]
struct Views {
    fail: bool,
    hold: bool,
    failed: usize,
    held: usize,
    released: usize,
    drop_reads: usize,
    dropped: usize,
}

type Shared = Arc<Mutex<Views>>;

/// A TCP forwarder in front of the service: each connection is carried to a new connection to the
/// service, and what the client sent and what the service answered are kept per connection. The
/// client sends one request per connection. A read of the whole `Pending` view is failed or held
/// as [`Views`] says.
struct Forwarder {
    address: String,
    kept: Carried,
    views: Shared,
    stop: Arc<AtomicBool>,
}

impl Forwarder {
    fn start(url: &str) -> Self {
        let target = url.trim_start_matches("http://").to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").expect("the forwarder binds");
        let address = listener.local_addr().expect("an address").to_string();
        let kept = Arc::new(Mutex::new(Vec::new()));
        let views: Shared = Arc::new(Mutex::new(Views::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let (record, stopped, shared) = (Arc::clone(&kept), Arc::clone(&stop), Arc::clone(&views));
        std::thread::spawn(move || {
            let started = Instant::now();
            while !stopped.load(Ordering::SeqCst) && started.elapsed() < 6 * HANG {
                let Some(client) = support::accept_within(&listener, Duration::from_millis(20))
                else {
                    continue;
                };
                let at = {
                    let mut kept = record.lock().expect("the record");
                    kept.push((Vec::new(), Vec::new()));
                    kept.len() - 1
                };
                let record = Arc::clone(&record);
                let shared = Arc::clone(&shared);
                let target = target.clone();
                std::thread::spawn(move || carry(client, &target, &record, &shared, at));
            }
        });
        Self {
            address,
            kept,
            views,
            stop,
        }
    }

    fn views(&self) -> std::sync::MutexGuard<'_, Views> {
        self.views.lock().expect("the views")
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn kept(&self) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.kept.lock().expect("the record").clone()
    }

    /// The `read` requests (`MarkRead`) the service got for `message_id`.
    fn reads_of(&self, message_id: &str) -> usize {
        self.kept()
            .iter()
            .filter(|(sent, _)| {
                sent.starts_with(b"POST /messaging/commands/read ")
                    && String::from_utf8_lossy(sent).contains(message_id)
            })
            .count()
    }

    /// Every `read` request the service got.
    fn all_reads(&self) -> usize {
        self.kept()
            .iter()
            .filter(|(sent, _)| sent.starts_with(b"POST /messaging/commands/read "))
            .count()
    }

    /// Whether a request starting with `head` was sent.
    fn asked(&self, head: &str) -> bool {
        self.kept()
            .iter()
            .any(|(sent, _)| sent.starts_with(head.as_bytes()))
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

/// Whether the request `sent` reads the whole `Pending` view, as the poll does.
fn whole_view(sent: &[u8]) -> bool {
    let undelivered = b"undelivered=1";
    sent.starts_with(b"GET /messaging/views/pending")
        && !sent
            .windows(undelivered.len())
            .any(|part| part == undelivered)
}

/// Before the first bytes of the answer of connection `at` go to the client: a read of the whole
/// `Pending` view is failed (`503` to `client`, answers `false`: nothing more is carried) or held
/// until released, as `views` says; every other answer goes on at once.
fn carried_on(client: &mut TcpStream, kept: &Carried, views: &Shared, at: usize) -> bool {
    if !whole_view(&kept.lock().expect("the record")[at].0) {
        return true;
    }
    let mut shown = views.lock().expect("the views");
    if shown.fail {
        shown.failed += 1;
        drop(shown);
        let _ = client.write_all(
            b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
        );
        return false;
    }
    if shown.hold {
        shown.held += 1;
        let this = shown.held;
        drop(shown);
        wait_for(&format!("the view {this} is released"), || {
            views.lock().expect("the views").released >= this
        });
    }
    true
}

/// Whether the first bytes `sent` of a connection are a `read` request that `views` says to
/// drop; counts it.
fn dropped(sent: &[u8], views: &Shared) -> bool {
    if !sent.starts_with(b"POST /messaging/commands/read ") {
        return false;
    }
    let mut shown = views.lock().expect("the views");
    if shown.drop_reads == 0 {
        return false;
    }
    shown.drop_reads -= 1;
    shown.dropped += 1;
    true
}

/// Carries one connection of a [`Forwarder`] to `target`, keeping both directions as
/// connection `at`; a read of the whole `Pending` view is failed or held, and a `read` request
/// dropped, as `views` says.
fn carry(client: TcpStream, target: &str, kept: &Carried, views: &Shared, at: usize) {
    let Ok(upstream) = TcpStream::connect(target) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    let (Ok(mut from_client), Ok(mut to_upstream)) = (client.try_clone(), upstream.try_clone())
    else {
        return;
    };
    let sent = Arc::clone(kept);
    let shared = Arc::clone(views);
    let up = std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        let mut first = true;
        loop {
            match from_client.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(read) => {
                    sent.lock().expect("the record")[at]
                        .0
                        .extend_from_slice(&buffer[..read]);
                    if std::mem::take(&mut first) && dropped(&buffer[..read], &shared) {
                        let _ = from_client.shutdown(Shutdown::Both);
                        let _ = to_upstream.shutdown(Shutdown::Both);
                        return;
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
    let mut buffer = [0_u8; 4096];
    let mut first = true;
    loop {
        match from_upstream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => {
                if std::mem::take(&mut first) && !carried_on(&mut to_client, kept, views, at) {
                    break;
                }
                kept.lock().expect("the record")[at]
                    .1
                    .extend_from_slice(&buffer[..read]);
                if to_client.write_all(&buffer[..read]).is_err() {
                    break;
                }
            }
        }
    }
    let _ = to_client.shutdown(Shutdown::Both);
    let _ = from_upstream.shutdown(Shutdown::Both);
    let _ = up.join();
}

/// A running `inbox bridge` of bob, polling every second, its standard output and standard error
/// in files; killed and waited for when dropped unfinished.
struct Bridge {
    child: Option<Child>,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Bridge {
    /// The bridge of bob with the binding of the cases, the stand-in first on `PATH`, and the
    /// service at `url`.
    fn start(world: &World, connectors: &Connectors, url: &str) -> Self {
        Self::bound(world, connectors, url, CHANNEL)
    }

    /// The bridge of bob bound to the chat channel `channel`, else as [`Bridge::start`].
    fn bound(world: &World, connectors: &Connectors, url: &str, channel: &str) -> Self {
        Self::bound_signed(world, connectors, url, channel, Signer::Person)
    }

    /// [`Bridge::bound`] signed by `signer`, with bob's outbox file.
    fn bound_signed(
        world: &World,
        connectors: &Connectors,
        url: &str,
        channel: &str,
        signer: Signer,
    ) -> Self {
        let stdout = world.dir.join(&format!("bridge-{channel}.out"));
        let stderr = world.dir.join(&format!("bridge-{channel}.err"));
        let mut command = world.client_command("bob", &["bridge"]);
        command.args([
            "--chat-channel",
            channel,
            "--allowed-chat-user",
            USER,
            "--chat-connector",
            CONNECTOR,
            "--chat-poll-interval",
            "1",
        ]);
        signer.apply(&mut command);
        let child = command
            .env("PATH", connectors.path())
            .env("INBOX_URL", url)
            .stdout(fs::File::create(&stdout).expect("the stdout file is made"))
            .stderr(fs::File::create(&stderr).expect("the stderr file is made"))
            .spawn()
            .expect("the binary starts");
        Self {
            child: Some(child),
            stdout,
            stderr,
        }
    }

    fn running(&mut self) -> bool {
        matches!(self.child.as_mut().map(Child::try_wait), Some(Ok(None)))
    }

    fn pid(&self) -> u32 {
        self.child.as_ref().expect("running").id()
    }

    fn stderr(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    /// Ends it with SIGTERM and answers its status, standard output and standard error.
    fn end(mut self) -> (ExitStatus, String, String) {
        let mut child = self.child.take().expect("running");
        let sent = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(sent.success(), "kill -TERM failed");
        let status = child.wait().expect("the bridge is waited for");
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

/// alice posts a message of `kind` with `body` and the flags `extra` to bob; answers its id.
fn post(world: &World, kind: &str, body: &str, extra: &[&str]) -> String {
    let mut args = vec!["post", "--to", BOB, "--kind", kind, "--body", body];
    if kind == "request" {
        args.extend(["--reply-shape", "free"]);
    }
    args.extend(extra);
    world.client("alice", &args).ok().stdout.trim().to_owned()
}

/// The state of `message_id` in bob's `Pending`, or an empty text when it is not listed.
fn state(world: &World, message_id: &str) -> String {
    support::rows(&world.client("bob", &["pending", "--json"]).ok().json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// The files of `dir` whose name starts with `prefix`, but a staged `.new` one.
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

/// The lines of the bridge's record of the chat posts beside bob's outbox file: message id,
/// reference and state.
fn record(world: &World) -> Vec<(String, String, String)> {
    let found = files(&world.dir.path, "bob.outbox.chat.");
    if found.is_empty() {
        return Vec::new();
    }
    assert_eq!(found.len(), 1, "one record file: {found:?}");
    fs::read_to_string(&found[0])
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let line = json::parse(line).unwrap_or_else(|error| panic!("{error}: {line}"));
            (
                support::text(&line, "message_id"),
                support::text(&line, "reference"),
                support::text(&line, "state"),
            )
        })
        .collect()
}

/// The state the record holds `message_id` in, or an empty text.
fn recorded(world: &World, message_id: &str) -> String {
    record(world)
        .into_iter()
        .find(|(id, _, _)| id == message_id)
        .map(|(_, _, state)| state)
        .unwrap_or_default()
}

/// Waits until the record holds `message_id` posted and the message is `Delivered`; answers the
/// reference of its chat post.
fn posted(world: &World, message_id: &str) -> String {
    wait_for(&format!("{message_id} is posted and delivered"), || {
        recorded(world, message_id) == "posted" && state(world, message_id) == "Delivered"
    });
    record(world)
        .into_iter()
        .find(|(id, _, _)| id == message_id)
        .map(|(_, reference, _)| reference)
        .expect("the post is recorded")
}

/// The number of the system call `futex`, the wait of a lock inside a process.
#[cfg(target_arch = "x86_64")]
const FUTEX: &str = "202";
#[cfg(target_arch = "aarch64")]
const FUTEX: &str = "98";

/// The `comm` of the thread of the poll (`inbox-bridge-poll`, cut to 15 bytes by Linux).
const POLL_THREAD: &str = "inbox-bridge-po";

/// Whether the thread of the bridge `pid` that `chosen` picks (by its directory under
/// `/proc/<pid>/task` and its `comm`) waits in the system call `futex`, as Linux
/// `/proc/<pid>/task/<tid>/syscall` shows it to the parent of the process. Once the rounds hold a
/// row, or the poll has an open post, the thread's one wait on a lock is the turn of the
/// `connectors` children, or the wait for a child it started.
fn waits_on_a_lock(pid: u32, chosen: impl Fn(&Path, &str) -> bool) -> bool {
    let Ok(tasks) = fs::read_dir(format!("/proc/{pid}/task")) else {
        return false;
    };
    tasks.flatten().any(|task| {
        let path = task.path();
        let comm = fs::read_to_string(path.join("comm")).unwrap_or_default();
        chosen(&path, comm.trim_end())
            && fs::read_to_string(path.join("syscall"))
                .is_ok_and(|call| call.split_whitespace().next() == Some(FUTEX))
    })
}

/// Whether the main thread of the bridge `pid`, the one of the rounds, waits on a lock.
fn rounds_wait(pid: u32) -> bool {
    waits_on_a_lock(pid, |path, _| path.ends_with(pid.to_string()))
}

/// Whether the thread of the poll of the bridge `pid` waits on a lock.
fn poll_waits(pid: u32) -> bool {
    waits_on_a_lock(pid, |_, comm| comm == POLL_THREAD)
}

/// The arguments of a read of the chat channel.
fn read_args() -> Vec<String> {
    [
        "--output",
        "json",
        "invoke",
        CONNECTOR,
        "conversations.history",
        "--input-stdin",
    ]
    .map(str::to_owned)
    .to_vec()
}

/// The input document of a read from the reference `oldest` on.
fn read_input(oldest: &str) -> Value {
    read_input_of(CHANNEL, oldest)
}

/// The input document of a read of `channel` from the reference `oldest` on, the post at `oldest`
/// included (`inclusive`).
fn read_input_of(channel: &str, oldest: &str) -> Value {
    json::parse(&format!(
        r#"{{"channel":"{channel}","oldest":"{oldest}","inclusive":true,"limit":100}}"#
    ))
    .expect("the input is JSON")
}

/// Check 2 and the poll: a reaction of the allowed chat user on the chat post of a note sends one
/// `read` request, the note is handled (gone from `Pending`), and the record holds the post as
/// read; the stand-in lists the same reaction at the next 2 polls and the forwarder still sees one
/// `read`. A read asks for the posts not older than the oldest open post of the record (`oldest`,
/// a string), without an idempotency key; no poll runs before the bridge holds an open post, so
/// the first invocation of the stand-in is a post.
#[test]
fn a_reaction_of_the_allowed_chat_user_reads_a_note_once() {
    a_reaction_of_the_allowed_chat_user_reads_a_note_once_signed(Signer::Person);
}

/// The case above, with the bridge signed by `signer`: also run with the bridge's own key in
/// `bridge_own_key`.
pub(crate) fn a_reaction_of_the_allowed_chat_user_reads_a_note_once_signed(signer: Signer) {
    let world = World::signed("bridge-read-note", signer);
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::bound_signed(&world, &connectors, &forwarder.url(), CHANNEL, signer);
    wait_for("the bridge reads Pending", || {
        forwarder.asked("GET /messaging/views/pending")
    });
    let note = post(&world, "note", "Build 7 is green.", &[]);
    let open = post(&world, "note", "Build 8 is green.", &[]);
    let note_ts = posted(&world, &note);
    let open_ts = posted(&world, &open);
    assert_eq!(
        connectors.operation(1),
        "chat.postMessage",
        "no poll before an open post: {}",
        bridge.stderr()
    );
    connectors.file(
        "history",
        &history(&[(note_ts.as_str(), REACTED), (open_ts.as_str(), "")]),
    );
    wait_for("the note is read", || recorded(&world, &note) == "read");
    let marked = connectors.reads();
    wait_for("2 more polls", || connectors.reads() >= marked + 2);

    assert_eq!(forwarder.reads_of(&note), 1, "one read of the note");
    assert_eq!(forwarder.reads_of(&open), 0);
    assert_eq!(state(&world, &note), "", "the note is handled");
    assert_eq!(state(&world, &open), "Delivered");
    assert_eq!(recorded(&world, &open), "posted");
    assert_eq!(connectors.posts(), 2);
    for r in 1..=connectors.reads() {
        let n = connectors.invocation("read", r);
        assert_eq!(connectors.args(n), read_args(), "read {r}");
    }
    assert_eq!(
        connectors.read_input(1),
        read_input(&note_ts),
        "the oldest open post"
    );
    assert_eq!(
        connectors.read_input(marked + 1),
        read_input(&open_ts),
        "a post read is no longer asked about"
    );
    assert!(!connectors.overlapped(), "one connectors at a time");
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, stdout, stderr) = bridge.end();
    assert_eq!(stdout, "");
    assert_eq!(stderr, "");
}

/// Check 3: reactions of a chat user who is not the allowed one, and reactions that name no list
/// of chat user ids (a text, an id that is no text, no `users`, a reaction that is no object),
/// send no request to the service and nothing to the chat channel, over 2 polls that list them.
#[test]
fn a_reaction_of_another_chat_user_changes_nothing() {
    let world = World::new("bridge-read-other");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    let note = post(&world, "note", "Build 7 is green.", &[]);
    let ts = posted(&world, &note);
    let reactions = format!(
        r#"[{{"name":"eyes","users":["{OTHER_USER}"],"count":1}},{{"name":"tada","users":"{USER}","count":1}},{{"name":"wave","users":[1],"count":1}},{{"name":"ok","count":1}},"{USER}"]"#
    );
    connectors.file("history", &history(&[(ts.as_str(), reactions.as_str())]));
    let from = connectors.reads();
    wait_for("2 polls that list the reactions", || {
        connectors.reads() >= from + 2
    });
    // The second of them has been answered and judged once the next poll starts.
    wait_for("the poll after them", || connectors.reads() >= from + 3);

    assert_eq!(forwarder.all_reads(), 0, "no read request");
    assert_eq!(connectors.posts(), 1, "nothing to the chat channel");
    assert_eq!(state(&world, &note), "Delivered");
    assert_eq!(recorded(&world, &note), "posted");
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    assert_eq!(stderr, "", "no line");
}

/// The system clock's second.
fn this_second() -> inbox_verify::Instant {
    inbox_verify::Instant::from_unix(
        inbox_verify::Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

/// Sets the service's clock file whole.
fn set_clock(dir: &Path, at: inbox_verify::Instant) {
    fs::write(dir.join("clock.new"), format_reading(at)).expect("the clock file is written");
    fs::rename(dir.join("clock.new"), dir.join("clock")).expect("the clock file is replaced");
}

/// Check 4: a reaction on the post of a request that is due (the service's clock, a file the case
/// moves, past its `expires`) is answered `deadline-passed`: one line names the message and
/// `Expired`, and the record holds the post as due. A reaction on a request read with `inbox read`
/// (`already-read`) and on a note read, so gone (`unknown-message`), while the read of the channel
/// of a poll that found both in `Pending` holds, writes no line, and the record holds both as
/// read. The stand-in goes on listing all 3 reactions; a
/// note posted after them is posted, delivered and keeps the poll going, and the forwarder sees one
/// `read` of each of the 3 at 2 polls later.
#[test]
fn a_reaction_on_a_due_message_is_named_once_and_on_a_read_or_gone_one_is_not() {
    let world = clocked("bridge-read-due");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    // 30 s after the system clock just before the post, as `due_mark` does: the clients sign with
    // the system clock and the service takes a request within 60 s of its reading.
    let expires_at = this_second().plus_seconds(30);
    let due = post(
        &world,
        "request",
        "Which build is green?",
        &["--expires", &format_instant(expires_at)],
    );
    let read = post(&world, "request", "Is build 8 green?", &[]);
    let gone = post(&world, "note", "Build 9 is green.", &[]);
    let due_ts = posted(&world, &due);
    let read_ts = posted(&world, &read);
    let gone_ts = posted(&world, &gone);
    // The poll whose read of the channel holds read the record and `Pending` before it, with all
    // 3 posted and listed: the 2 leave `Pending` meanwhile, so that poll sends their `read`
    // requests. The first read that holds may be of a poll that read the record before the last
    // post; the one after it is not.
    connectors.file("hold-read", "");
    let first = connectors.holding_read();
    connectors.file(&format!("release-read.{first}"), "");
    let held = first + 1;
    wait_for(&format!("read {held} holds"), || {
        connectors.exists(&format!("holding-read.{held}"))
    });
    world.client("bob", &["read", &read]).ok();
    world.client("bob", &["read", &gone]).ok();
    set_clock(&world.dir.path, expires_at.plus_seconds(1));
    connectors.file(
        "history",
        &history(&[
            (due_ts.as_str(), REACTED),
            (read_ts.as_str(), REACTED),
            (gone_ts.as_str(), REACTED),
        ]),
    );
    connectors.remove("hold-read");
    connectors.file(&format!("release-read.{held}"), "");
    wait_for("the 3 posts are settled", || {
        recorded(&world, &due) == "due"
            && recorded(&world, &read) == "read"
            && recorded(&world, &gone) == "read"
    });
    let after = post(&world, "note", "Build 10 is green.", &[]);
    posted(&world, &after);
    let from = connectors.reads();
    wait_for("2 more polls", || connectors.reads() >= from + 2);

    for message_id in [&due, &read, &gone] {
        assert_eq!(forwarder.reads_of(message_id), 1, "{message_id}");
    }
    assert_eq!(forwarder.reads_of(&after), 0);
    assert_eq!(recorded(&world, &after), "posted");
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "one line, of the due request: {stderr}");
    assert!(lines[0].starts_with("inbox bridge: "), "{stderr}");
    assert!(lines[0].contains(&due), "{stderr}");
    assert!(lines[0].contains("Expired"), "{stderr}");
}

/// A world whose service reads its clock from the file `clock` of the case, set to the system
/// clock's second, and sweeps once an hour, so a due message stays in `Pending` until the case
/// ends.
fn clocked(name: &str) -> World {
    let (dir, operator_public_key) = keys_only(name);
    set_clock(&dir.path, this_second());
    let service = Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator_public_key,
            "--max-wait",
            "20",
            "--sweep-interval",
            "3600",
            "--clock-file",
            "clock",
        ],
        &[],
    );
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// Check 5: a poll that fails marks nothing, also when its answer lists the reaction before the
/// member that fails it, and is tried again after the pause of a failure; one line on standard
/// error names a run of failed polls (a refusal with its class, an answer with a mistyped
/// member, a standard error that is no JSON, a usage error), the next line comes after a poll
/// succeeded and another failed (`rate_limited`). A note posted meanwhile is posted and delivered.
/// The first poll whose answer is whole marks the note read.
#[test]
fn a_failed_poll_marks_nothing_is_named_once_per_run_and_stops_no_post() {
    let world = World::new("bridge-read-failed");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    connectors.file("hold-read", "");
    let note = post(&world, "note", "Build 7 is green.", &[]);
    let ts = posted(&world, &note);
    assert_eq!(connectors.holding_read(), 1);
    let reaction = history(&[(ts.as_str(), REACTED)]);
    connectors.file("history", &reaction);
    connectors.file("err.1", &refusal("forbidden"));
    connectors.file("exit.1", "1");
    connectors.file(
        "history.2",
        &format!(
            r#"{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[{{"ts":"{ts}","reactions":{REACTED}}},{{"ts":5}}]}}}}}}}}"#
        ),
    );
    connectors.file("err.3", "connectors: the connection is not configured\n");
    connectors.file("exit.3", "1");
    connectors.file("err.4", "error: unexpected argument\n");
    connectors.file("exit.4", "2");
    connectors.file("history.5", &history(&[]));
    connectors.file("err.6", &refusal("rate_limited"));
    connectors.file("exit.6", "1");
    let meanwhile = post(&world, "note", "Build 8 is green.", &[]);
    for r in 1..=6 {
        connectors.file(&format!("release-read.{r}"), "");
        wait_for(&format!("read {} holds", r + 1), || {
            connectors.exists(&format!("holding-read.{}", r + 1))
        });
        assert_eq!(forwarder.reads_of(&note), 0, "no mark after read {r}");
        assert_eq!(recorded(&world, &note), "posted", "after read {r}");
        if r == 2 {
            wait_for(
                "the rounds go on: the note posted meanwhile is delivered",
                || state(&world, &meanwhile) == "Delivered",
            );
        }
    }
    connectors.remove("hold-read");
    connectors.file("release-read.7", "");
    wait_for("the note is read", || recorded(&world, &note) == "read");
    assert_eq!(forwarder.reads_of(&note), 1);
    assert_eq!(state(&world, &note), "");
    assert_eq!(recorded(&world, &meanwhile), "posted");
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 2, "one line per run of failures: {stderr}");
    for line in &lines {
        assert!(line.starts_with("inbox bridge: "), "{stderr}");
        assert!(line.contains(CHANNEL), "{stderr}");
    }
    assert!(lines[0].contains("`service_failure`"), "{stderr}");
    assert!(lines[0].contains("`forbidden`"), "{stderr}");
    assert!(lines[1].contains("`rate_limited`"), "{stderr}");
}

/// Check 6: a request whose chat post has a reaction of the allowed chat user moves from
/// `Delivered` to `Read`: it leaves bob's `Pending`, and alice's `Outbox` shows it `Read`. The
/// stand-in keeps listing the reaction at the next 2 polls (an open note keeps them going), and
/// the forwarder sees one `read` of the request.
#[test]
fn a_request_read_by_a_reaction_stays_read_while_the_reaction_is_listed() {
    let world = World::new("bridge-read-request");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    let request = post(&world, "request", "Which build is green?", &[]);
    let open = post(&world, "note", "Build 8 is green.", &[]);
    let request_ts = posted(&world, &request);
    let open_ts = posted(&world, &open);
    connectors.file(
        "history",
        &history(&[(request_ts.as_str(), REACTED), (open_ts.as_str(), "")]),
    );
    wait_for("the request is read", || {
        recorded(&world, &request) == "read"
    });
    let marked = connectors.reads();
    wait_for("2 more polls", || connectors.reads() >= marked + 2);

    assert_eq!(forwarder.reads_of(&request), 1, "one read of the request");
    assert_eq!(state(&world, &request), "", "not in Pending");
    let sent = support::rows(&world.client("alice", &["outbox", "--json"]).ok().json());
    let row = sent
        .iter()
        .find(|row| support::text(row, "message_id") == request)
        .expect("the request is in alice's Outbox");
    assert_eq!(support::text(row, "state"), "Read");
    assert_eq!(support::text(row, "from"), ALICE);
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    assert_eq!(stderr, "");
}

/// One `connectors` child at a time: a post waits for a poll that is running (the case sees the
/// thread of the rounds wait on a lock while the read holds, and no post before the read ended),
/// and a poll waits for a post that is running (the thread of the poll waits on a lock, and no
/// read starts before the post ended); both go on, with nothing on standard error.
#[test]
fn a_post_waits_for_a_running_poll_and_a_poll_for_a_running_post() {
    let world = World::new("bridge-read-turn");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let mut bridge = Bridge::start(&world, &connectors, &world.service.url);
    let first = post(&world, "note", "Build 7 is green.", &[]);
    posted(&world, &first);

    connectors.file("hold-read", "");
    let held = connectors.holding_read();
    let second = post(&world, "note", "Build 8 is green.", &[]);
    wait_for("the post waits for the turn", || rounds_wait(bridge.pid()));
    assert_eq!(connectors.posts(), 1, "no post while the read runs");
    assert_eq!(state(&world, &second), "Sent");
    connectors.remove("hold-read");
    connectors.file(&format!("release-read.{held}"), "");
    posted(&world, &second);
    assert!(
        connectors.nanos("start", connectors.invocation("post", 2))
            >= connectors.nanos("end", connectors.invocation("read", held)),
        "the post started after the read ended"
    );

    connectors.file("hold-post", "");
    let third = post(&world, "note", "Build 9 is green.", &[]);
    wait_for("post 3 holds", || connectors.exists("holding-post.3"));
    let reads = connectors.reads();
    wait_for("the poll waits for the turn", || poll_waits(bridge.pid()));
    assert_eq!(connectors.reads(), reads, "no read while the post runs");
    connectors.remove("hold-post");
    connectors.file("release-post.3", "");
    posted(&world, &third);
    wait_for("the next read", || connectors.reads() > reads);
    assert!(
        connectors.nanos("start", connectors.invocation("read", reads + 1))
            >= connectors.nanos("end", connectors.invocation("post", 3)),
        "the read started after the post ended"
    );
    assert!(!connectors.overlapped(), "one connectors at a time");
    assert!(connectors.calls() >= 4);
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    assert_eq!(stderr, "");
}

/// The settle: a request the bridge posted that leaves `Pending` with `inbox read`, without a
/// reaction, is settled by the next poll whose read of `Pending` succeeds, without a request and
/// without a line; a poll whose read of `Pending` fails settles nothing. A reaction on the settled
/// post sends no `read`, and the channel is read from the newer open post on. Once no post is
/// open, no poll starts a `connectors` child: the next read starts after the next post and asks
/// from it on.
#[test]
fn a_post_whose_message_left_pending_without_a_reaction_is_settled_and_asked_about_no_more() {
    let world = World::new("bridge-read-settle");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    let older = post(&world, "request", "Which build is green?", &[]);
    let newer = post(&world, "request", "Is build 8 green?", &[]);
    let older_ts = posted(&world, &older);
    let newer_ts = posted(&world, &newer);

    forwarder.views().fail = true;
    world.client("bob", &["read", &older]).ok();
    let failed = forwarder.views().failed;
    wait_for("2 failed reads of Pending", || {
        forwarder.views().failed >= failed + 2
    });
    assert_eq!(
        recorded(&world, &older),
        "posted",
        "no settle after a failed read of Pending"
    );
    assert_eq!(recorded(&world, &newer), "posted");
    connectors.file(
        "history",
        &history(&[(older_ts.as_str(), REACTED), (newer_ts.as_str(), "")]),
    );
    forwarder.views().fail = false;
    wait_for("the older post is settled", || {
        recorded(&world, &older) == "read"
    });
    let from = connectors.reads();
    assert_eq!(connectors.read_input(from + 1), read_input(&newer_ts));
    assert_eq!(recorded(&world, &newer), "posted", "in Pending, open");

    world.client("bob", &["read", &newer]).ok();
    wait_for("the newer post is settled", || {
        recorded(&world, &newer) == "read"
    });
    assert_eq!(forwarder.reads_of(&older), 0, "no read of a settled post");
    let before = connectors.reads();
    let next = post(&world, "note", "Build 9 is green.", &[]);
    let next_ts = posted(&world, &next);
    assert!(
        connectors.nanos("start", connectors.invocation("read", before + 1))
            >= connectors.nanos("end", connectors.invocation("post", 3)),
        "no read between the settle of the last open post and the next post"
    );
    assert_eq!(connectors.read_input(before + 1), read_input(&next_ts));
    assert_eq!(forwarder.all_reads(), 0, "no `read` request at all");
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 1, "one line, of the failed reads: {stderr}");
    assert!(lines[0].contains("`Pending`"), "{stderr}");
}

/// A message still in `Pending` (`Delivered`) keeps its post open over 2 polls, and a post
/// recorded while a poll reads `Pending` (the answer held by the forwarder until the post is
/// recorded and delivered) is not settled by that answer: the poll compares only the entries it
/// read from the record before it read `Pending`.
#[test]
fn a_post_whose_message_is_in_pending_stays_open_also_when_posted_during_a_read_of_pending() {
    let world = World::new("bridge-read-listed");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    let first = post(&world, "request", "Which build is green?", &[]);
    let first_ts = posted(&world, &first);

    forwarder.views().hold = true;
    wait_for("a read of Pending holds", || forwarder.views().held >= 1);
    let second = post(&world, "note", "Build 8 is green.", &[]);
    posted(&world, &second);
    let reads = connectors.reads();
    {
        let mut views = forwarder.views();
        views.hold = false;
        views.released = views.held;
    }
    wait_for("2 more polls", || connectors.reads() >= reads + 2);

    assert_eq!(recorded(&world, &first), "posted", "in Pending, open");
    assert_eq!(
        recorded(&world, &second),
        "posted",
        "not settled by a read of Pending from before its post"
    );
    assert_eq!(connectors.read_input(reads + 1), read_input(&first_ts));
    assert_eq!(forwarder.all_reads(), 0);
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    assert_eq!(stderr, "");
}

/// Each entry of the record names the channel of its post: a bridge started again with another
/// `--chat-channel` asks nothing about the posts of the first channel (its first `connectors` is a
/// post of its own), and a reaction listed in its channel under the reference a post of the first
/// channel has too marks only its own post.
#[test]
fn a_bridge_bound_to_another_channel_asks_about_and_marks_only_its_own_posts() {
    let world = World::new("bridge-read-channel");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let forwarder = Forwarder::start(&world.service.url);
    let first = Connectors::named(&world, "connectors-1");
    let bridge = Bridge::bound(&world, &first, &forwarder.url(), CHANNEL);
    let old = post(&world, "note", "Build 7 is green.", &[]);
    let old_ts = posted(&world, &old);
    let (_, _, stderr) = bridge.end();
    assert_eq!(stderr, "");

    let second = Connectors::named(&world, "connectors-2");
    let mut bridge = Bridge::bound(&world, &second, &forwarder.url(), OTHER_CHANNEL);
    let new = post(&world, "note", "Build 8 is green.", &[]);
    let new_ts = posted(&world, &new);
    assert_eq!(new_ts, old_ts, "the same reference in the other channel");
    assert_eq!(
        second.operation(1),
        "chat.postMessage",
        "no read of the posts of the first channel"
    );
    second.file("history", &history(&[(new_ts.as_str(), REACTED)]));
    wait_for("the new note is read", || recorded(&world, &new) == "read");
    assert_eq!(second.read_input(1), read_input_of(OTHER_CHANNEL, &new_ts));
    assert_eq!(
        forwarder.reads_of(&old),
        0,
        "no read of the post of the first channel"
    );
    assert_eq!(recorded(&world, &old), "posted");
    assert_eq!(state(&world, &old), "Delivered");
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    assert_eq!(stderr, "");
}

/// The `read` is sent at least once. A `read` request that got no answer (the forwarder closes
/// its connection) leaves the entry `posted`, and a later poll marks it. A `deadline-passed`
/// answer whose record change fails (a directory at the name of the record's staged file) writes
/// no line of the due message: the poll fails, names the record once, and the next tries send the
/// `read` again; once the record can change the entry is recorded `due` and its one line written.
#[test]
fn a_read_without_an_answer_or_without_its_record_change_marks_nothing_and_is_sent_again() {
    let world = clocked("bridge-read-again");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let forwarder = Forwarder::start(&world.service.url);
    let mut bridge = Bridge::start(&world, &connectors, &forwarder.url());
    let expires_at = this_second().plus_seconds(30);
    let note = post(&world, "note", "Build 7 is green.", &[]);
    let due = post(
        &world,
        "request",
        "Which build is green?",
        &["--expires", &format_instant(expires_at)],
    );
    let note_ts = posted(&world, &note);
    let due_ts = posted(&world, &due);

    connectors.file("hold-read", "");
    let held = connectors.holding_read();
    forwarder.views().drop_reads = 1;
    connectors.file("history", &history(&[(note_ts.as_str(), REACTED)]));
    connectors.file(&format!("release-read.{held}"), "");
    let next = held + 1;
    wait_for(&format!("read {next} holds"), || {
        connectors.exists(&format!("holding-read.{next}"))
    });
    assert_eq!(forwarder.views().dropped, 1);
    assert_eq!(
        recorded(&world, &note),
        "posted",
        "no mark without an answer to its `read`"
    );
    connectors.remove("hold-read");
    connectors.file(&format!("release-read.{next}"), "");
    wait_for("a later poll marks the note", || {
        recorded(&world, &note) == "read"
    });
    assert_eq!(forwarder.reads_of(&note), 2);

    let record = files(&world.dir.path, "bob.outbox.chat.")
        .into_iter()
        .next()
        .expect("the record");
    let staged = PathBuf::from(format!("{}.new", record.display()));
    fs::create_dir(&staged).expect("a directory at the name of the staged record");
    set_clock(&world.dir.path, expires_at.plus_seconds(1));
    connectors.file("history", &history(&[(due_ts.as_str(), REACTED)]));
    // The first `read` was answered and its record change failed: the second is the same `read`
    // sent again while the entry is still `posted`.
    wait_for("the `read` of the due request is sent again", || {
        forwarder.reads_of(&due) >= 2
    });
    assert_eq!(recorded(&world, &due), "posted");
    assert!(
        !bridge.stderr().contains("Expired"),
        "no line of the due request before it is recorded due: {}",
        bridge.stderr()
    );
    fs::remove_dir(&staged).expect("the directory is removed");
    wait_for("the due request is recorded due", || {
        recorded(&world, &due) == "due" && bridge.stderr().contains("Expired")
    });
    assert!(bridge.running(), "{}", bridge.stderr());
    let (_, _, stderr) = bridge.end();
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), 3, "{stderr}");
    assert!(
        lines[0].contains(&note),
        "the read without an answer: {stderr}"
    );
    assert!(
        lines[1].contains(&due),
        "the record that could not change: {stderr}"
    );
    assert!(!lines[1].contains("Expired"), "{stderr}");
    assert!(lines[2].contains(&due), "{stderr}");
    assert!(lines[2].contains("Expired"), "{stderr}");
}
