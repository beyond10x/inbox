//! Adversary cases of story:the-chat-bridge-asks-a-decision-once: a repost whose `read` fails, a
//! record the rounds cannot read at a repost, and an entry of the build before this story at a
//! restart of the service.
//!
//! Each runs against a real `inbox serve` of the debug build with `--clock-file`, behind a
//! forwarder that can drop one `read` request, and a stand-in `connectors` that numbers its posts.
//! No case reaches a chat provider or a network beyond loopback, and no case holds a wall-clock
//! time: a case waits on the stand-in's record or on a fact, with a bound that only ends a hang.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use inbox::server::json::{self, Value};
use inbox_server::format_reading;
use support::{BOB, Service, World, keys_only};

const CHANNEL: &str = "C0000000001";
const USER: &str = "U0000000001";
const HANG: Duration = support::STAND_IN_WAIT;
const REACTED: &str = r#"[{"name":"eyes","users":["U0000000001"],"count":1}]"#;

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

/// A stand-in `connectors` that numbers each post in `post.<p>` and answers it with the reference
/// `1700000000.<p>`, and answers a read of the channel with the file `history` or an empty page,
/// and a read of the replies of a post with none.
struct Connectors {
    dir: PathBuf,
}

impl Connectors {
    fn new(world: &World) -> Self {
        let dir = world.dir.join("connectors");
        fs::create_dir_all(dir.join("bin")).expect("the stand-in's directory is made");
        let shown = dir.to_str().expect("a UTF-8 path");
        let script = format!(
            r#"#!/bin/sh
d='{shown}'
cat > /dev/null
case "$5" in
chat.postMessage)
  k=1
  while [ -e "$d/post.$k" ]; do k=$((k + 1)); done
  : > "$d/post.$k"
  printf '{{"result":{{"mutation":{{"classification":"applied"}},"result":{{"status":200,"body":{{"ok":true,"channel":"{CHANNEL}","ts":"1700000000.%06d"}}}}}}}}\n' "$k"
  exit 0
  ;;
conversations.history)
  if [ -e "$d/history" ]; then cat "$d/history"; else
  printf '%s\n' '{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[],"has_more":false}}}}}}}}'
  fi
  exit 0
  ;;
conversations.replies)
  printf '%s\n' '{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[]}}}}}}}}'
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

    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    fn posts(&self) -> usize {
        (1..)
            .take_while(|k| self.dir.join(format!("post.{k}")).exists())
            .count()
    }

    fn history(&self, references: &[&str]) {
        let listed: Vec<String> = references
            .iter()
            .map(|ts| {
                format!(
                    r#"{{"type":"message","user":"U0000000009","text":"a post","ts":"{ts}","reactions":{REACTED}}}"#
                )
            })
            .collect();
        let text = format!(
            r#"{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[{}],"has_more":false,"response_metadata":{{"next_cursor":""}}}}}}}}}}"#,
            listed.join(",")
        );
        let staged = self.dir.join("history.staged");
        fs::write(&staged, text).expect("the history is written");
        fs::rename(&staged, self.dir.join("history")).expect("the history is replaced");
    }
}

type Carried = Arc<Mutex<Vec<Vec<u8>>>>;

/// A TCP forwarder in front of the service that keeps the first bytes each connection sent and,
/// while `drop_read` is set, drops the next `read` command without carrying it (once).
struct Forwarder {
    address: String,
    kept: Carried,
    target: Arc<Mutex<String>>,
    drop_read: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
}

impl Forwarder {
    fn start(url: &str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the forwarder binds");
        let address = listener.local_addr().expect("an address").to_string();
        let kept: Carried = Arc::new(Mutex::new(Vec::new()));
        let target = Arc::new(Mutex::new(url.trim_start_matches("http://").to_owned()));
        let drop_read = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (record, to, dropping, stopped) = (
            Arc::clone(&kept),
            Arc::clone(&target),
            Arc::clone(&drop_read),
            Arc::clone(&stop),
        );
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
                let dropping = Arc::clone(&dropping);
                std::thread::spawn(move || carry(client, &target, &record, at, &dropping));
            }
        });
        Self {
            address,
            kept,
            target,
            drop_read,
            stop,
        }
    }

    fn to(&self, url: &str) {
        *self.target.lock().expect("the target") = url.trim_start_matches("http://").to_owned();
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }

    fn sent(&self) -> Vec<Vec<u8>> {
        self.kept.lock().expect("the record").clone()
    }

    /// The connections from `from` on whose request starts with `head`.
    fn heads(&self, from: usize, head: &str) -> usize {
        self.sent()
            .iter()
            .skip(from)
            .filter(|sent| sent.starts_with(head.as_bytes()))
            .count()
    }
}

impl Drop for Forwarder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
    }
}

fn carry(mut client: TcpStream, target: &str, kept: &Carried, at: usize, dropping: &AtomicBool) {
    let mut first = [0_u8; 4096];
    let Ok(read) = client.read(&mut first) else {
        return;
    };
    kept.lock().expect("the record")[at].extend_from_slice(&first[..read]);
    if first[..read].starts_with(b"POST /messaging/commands/read ")
        && dropping.swap(false, Ordering::SeqCst)
    {
        let _ = client.shutdown(Shutdown::Both);
        return;
    }
    let Ok(mut upstream) = TcpStream::connect(target) else {
        let _ = client.shutdown(Shutdown::Both);
        return;
    };
    if upstream.write_all(&first[..read]).is_err() {
        return;
    }
    let (Ok(mut from_client), Ok(mut to_upstream)) = (client.try_clone(), upstream.try_clone())
    else {
        return;
    };
    let up = std::thread::spawn(move || {
        let _ = std::io::copy(&mut from_client, &mut to_upstream);
        let _ = to_upstream.shutdown(Shutdown::Write);
    });
    let _ = std::io::copy(&mut upstream, &mut client);
    let _ = client.shutdown(Shutdown::Both);
    let _ = upstream.shutdown(Shutdown::Both);
    let _ = up.join();
}

fn this_second() -> inbox_verify::Instant {
    inbox_verify::Instant::from_unix(
        inbox_verify::Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

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

fn clocked(name: &str) -> World {
    let (dir, operator_public_key) = keys_only(name);
    fs::write(dir.path.join("clock"), format_reading(this_second())).expect("the clock is set");
    let service = serve(&dir.path, &operator_public_key);
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// A new start of the service behind `forwarder`.
fn restart(world: &mut World, forwarder: &Forwarder) {
    let next = serve(&world.dir.path, &world.operator_public_key);
    forwarder.to(&next.url);
    let before = std::mem::replace(&mut world.service, next);
    assert!(before.terminate().success());
}

struct Bridge {
    child: Option<Child>,
    stderr: PathBuf,
}

impl Bridge {
    fn start(world: &World, connectors: &Connectors, url: &str) -> Self {
        let stderr = world.dir.join("bridge.err");
        let child = world
            .client_command("bob", &["bridge"])
            .args([
                "--chat-channel",
                CHANNEL,
                "--allowed-chat-user",
                USER,
                "--chat-connector",
                "slack",
                "--chat-poll-interval",
                "1",
            ])
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

    fn exited(&mut self) -> bool {
        self.child
            .as_mut()
            .is_some_and(|child| child.try_wait().ok().flatten().is_some())
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

fn client(world: &World, url: &str, who: &str, args: &[&str]) -> support::Run {
    world.client_env(who, args, &[("INBOX_URL", url)]).ok()
}

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

fn state(world: &World, url: &str, message_id: &str) -> String {
    support::rows(&client(world, url, "bob", &["pending", "--json"]).json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

fn sent_state(world: &World, url: &str, message_id: &str) -> String {
    support::rows(&client(world, url, "alice", &["outbox", "--json"]).json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

/// The path of bob's record of the chat posts, once there is one.
fn record_path(world: &World) -> Option<PathBuf> {
    fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bob.outbox.chat.") && !name.ends_with(".new")
        })
}

fn recorded(world: &World, message_id: &str, name: &str) -> String {
    let Some(path) = record_path(world) else {
        return String::new();
    };
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| json::parse(line).ok())
        .find(|line| support::text(line, "message_id") == message_id)
        .and_then(|line| match line.member(name) {
            Some(Value::Text(text)) => Some(text.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn posted(world: &World, url: &str, message_id: &str) -> String {
    wait_for(&format!("{message_id} is posted and delivered"), || {
        recorded(world, message_id, "state") == "posted"
            && state(world, url, message_id) == "Delivered"
    });
    recorded(world, message_id, "reference")
}

/// alice's next commands, until the service holds `message_id` again (her repost).
fn reposted(world: &World, url: &str, message_id: &str) {
    wait_for("alice's next command posts it again", || {
        client(world, url, "alice", &["outbox"]);
        !state(world, url, message_id).is_empty()
    });
}

/// Check 3: a request the person read in the channel, posted again after a restart of the service.
/// The bridge's `MarkDelivered` passes; its `read` is lost on the way once (the connection dropped,
/// as a network failure or a restart does). The story says the request is in `Read` again; the
/// round never gets the row again (it is `Delivered`, not `Sent`) and the poll asks nothing of an
/// entry that is `read`, so it stays `Delivered`.
#[test]
fn adversary_a_read_lost_on_a_repost_leaves_the_request_delivered_for_good() {
    let mut world = clocked("adv-bridge-once-read-lost");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::start(&world, &connectors, &url);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let reference = posted(&world, &url, &request);
    connectors.history(&[reference.as_str()]);
    wait_for("the request is read", || {
        recorded(&world, &request, "state") == "read"
    });
    assert_eq!(sent_state(&world, &url, &request), "Read");

    forwarder.drop_read.store(true, Ordering::SeqCst);
    restart(&mut world, &forwarder);
    reposted(&world, &url, &request);
    wait_for("the bridge's read of the repost is dropped", || {
        !forwarder.drop_read.load(Ordering::SeqCst)
    });
    let started = Instant::now();
    while sent_state(&world, &url, &request) != "Read" && started.elapsed() < HANG {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        sent_state(&world, &url, &request),
        "Read",
        "the request is in `Read` again; the bridge said: {}",
        bridge.stderr()
    );
    assert_eq!(connectors.posts(), 1, "{}", bridge.stderr());
}

/// Checks 2 and 4: a record of the chat posts the rounds cannot read (over its bound, made sparse
/// so no disk is spent) when a message it holds comes back by a repost. The stand-in must record no
/// second post operation for that message id; the rounds read the record as empty and post it.
#[test]
fn adversary_a_record_the_rounds_cannot_read_at_a_repost_gives_no_second_post() {
    let mut world = clocked("adv-bridge-once-unreadable");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let mut bridge = Bridge::start(&world, &connectors, &url);
    let decision = post(
        &world,
        &url,
        "decision",
        "Ship build 7?",
        &["--default-on-silence", "B"],
    );
    posted(&world, &url, &decision);
    let path = record_path(&world).expect("the record is written");
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the record opens")
        .set_len(64 * 1024 * 1024 + 1)
        .expect("the record grows past its bound");

    restart(&mut world, &forwarder);
    reposted(&world, &url, &decision);
    let from = forwarder.sent().len();
    wait_for("the rounds reach the reposted decision", || {
        connectors.posts() > 1
            || bridge.exited()
            || forwarder.heads(from, "GET /messaging/views/pending") >= 4
    });
    assert_eq!(
        connectors.posts(),
        1,
        "a second post operation of {decision}: {}",
        bridge.stderr()
    );
}

/// Checks 2 and 4 for an entry of the build before this story (no `thread`, no `expires`), the
/// record a bridge of the story before left. After a restart of the service a poll finds no row of
/// it before the sender reposts and drops the entry; the repost is then posted a second time.
#[test]
fn adversary_an_entry_of_the_build_before_is_posted_again_after_a_restart_of_the_service() {
    let mut world = clocked("adv-bridge-once-old-entry");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::start(&world, &connectors, &url);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let reference = posted(&world, &url, &request);
    let path = record_path(&world).expect("the record is written");
    let old = format!(
        r#"{{"message_id":"{request}","channel":"{CHANNEL}","reference":"{reference}","state":"posted"}}"#
    );
    let staged = path.with_extension("adv");
    fs::write(&staged, format!("{old}\n")).expect("the old line is written");
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).expect("mode 0600");
    fs::rename(&staged, &path).expect("the record is replaced");

    let from = forwarder.sent().len();
    restart(&mut world, &forwarder);
    wait_for("a poll settles the entry after the restart", || {
        forwarder.heads(from, "GET /messaging/views/thread") >= 2
            || recorded(&world, &request, "state").is_empty()
    });
    reposted(&world, &url, &request);
    wait_for("the repost is delivered", || {
        state(&world, &url, &request) == "Delivered"
    });
    assert_eq!(
        connectors.posts(),
        1,
        "a second post operation of {request}: {}",
        bridge.stderr()
    );
}

// Pass 2.

/// Gives a directory its mode back when dropped, so the case's directory can be removed.
struct Mode {
    dir: PathBuf,
    mode: u32,
}

impl Drop for Mode {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.dir, fs::Permissions::from_mode(self.mode));
    }
}

/// bob's bridge with `extra` flags and `env` set after the world's.
fn bridge_with(
    world: &World,
    connectors: &Connectors,
    url: &str,
    extra: &[&str],
    env: &[(&str, &str)],
) -> Bridge {
    let stderr = world.dir.join("bridge-2.err");
    let child = world
        .client_command("bob", &["bridge"])
        .args([
            "--chat-channel",
            CHANNEL,
            "--allowed-chat-user",
            USER,
            "--chat-connector",
            "slack",
            "--chat-poll-interval",
            "1",
        ])
        .args(extra)
        .envs(env.iter().copied())
        .env("PATH", connectors.path())
        .env("INBOX_URL", url)
        .stdout(std::process::Stdio::null())
        .stderr(fs::File::create(&stderr).expect("the stderr file is made"))
        .spawn()
        .expect("the binary starts");
    Bridge {
        child: Some(child),
        stderr,
    }
}

/// Waits for the bridge's exit; answers its status code (`None`: a signal).
fn exit_code(bridge: &mut Bridge, what: &str) -> Option<i32> {
    let child = bridge.child.as_mut().expect("running");
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("the bridge is waited for") {
            return status.code();
        }
        assert!(
            started.elapsed() < HANG,
            "{what}: not within {} s; it said: {}",
            HANG.as_secs(),
            fs::read_to_string(&bridge.stderr).unwrap_or_default()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// The claim of the first bridge story that round 2 left without a case
/// (`docs/design/bridge.md`: a post whose record cannot be written): the record can be read and
/// not changed (bob's outbox directory loses its write permission after the bridge started). The
/// post is made once, the bridge ends with 4 naming the record, and the message stays in `Sent`.
#[test]
fn adversary_a_post_whose_record_cannot_be_written_ends_the_bridge_with_4_and_stays_sent() {
    let world = clocked("adv-bridge-once-unwritable");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    let home = world.dir.join("bob-home");
    fs::create_dir_all(&home).expect("bob's directory is made");
    let outbox = home.join("bob.outbox");
    let outbox = outbox.to_str().expect("a UTF-8 path").to_owned();
    let env = [("INBOX_OUTBOX", outbox.as_str())];
    let opened = world.client_env(
        "bob",
        &["open", "--adapter", "slack"],
        &[env[0], ("INBOX_URL", url.as_str())],
    );
    assert_eq!(opened.status, 0, "{}", opened.stderr);
    let connectors = Connectors::new(&world);
    let from = forwarder.sent().len();
    let mut bridge = bridge_with(&world, &connectors, &url, &[], &env);
    wait_for("the bridge's first round", || {
        forwarder.heads(from, "GET /messaging/views/pending") >= 1
    });
    let _mode = Mode {
        dir: home.clone(),
        mode: 0o700,
    };
    fs::set_permissions(&home, fs::Permissions::from_mode(0o500)).expect("read only");
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    let code = exit_code(&mut bridge, "the bridge ends");
    let said = bridge.stderr();
    assert_eq!(code, Some(4), "{said}");
    assert_eq!(connectors.posts(), 1, "{said}");
    assert!(said.contains(".chat."), "the line names the record: {said}");
    assert_eq!(sent_state(&world, &url, &note), "Sent", "{said}");
}

/// F2 for a record whose line of a held message does not read (a line cut short): the rounds take
/// it as holding no entry for the message, and its repost gets a second post operation.
#[test]
fn adversary_a_held_line_that_does_not_read_gives_no_second_post_at_a_repost() {
    let mut world = clocked("adv-bridge-once-cut-line");
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let mut bridge = Bridge::start(&world, &connectors, &url);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    posted(&world, &url, &request);
    let path = record_path(&world).expect("the record is written");
    let line = fs::read_to_string(&path).expect("the record reads");
    let cut = &line.trim_end()[..line.trim_end().len() - 2];
    let staged = path.with_extension("adv");
    fs::write(&staged, format!("{cut}\n")).expect("the cut line is written");
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).expect("mode 0600");
    fs::rename(&staged, &path).expect("the record is replaced");

    restart(&mut world, &forwarder);
    let from = forwarder.sent().len();
    reposted(&world, &url, &request);
    // Pass 3: the decided way out is the end of the bridge (exit 4, no post, no mark); a second
    // post or a `MarkDelivered` ends the wait too, so the claims below name what went wrong.
    wait_for("the rounds reach the repost", || {
        connectors.posts() > 1
            || forwarder.heads(from, "POST /messaging/commands/mark-delivered") >= 1
            || bridge.exited()
    });
    assert_eq!(
        connectors.posts(),
        1,
        "a second post operation of {request}: {}",
        bridge.stderr()
    );
    assert_eq!(
        forwarder.heads(from, "POST /messaging/commands/mark-delivered"),
        0,
        "no MarkDelivered without a readable record: {}",
        bridge.stderr()
    );
    let code = exit_code(&mut bridge, "the bridge ends");
    assert_eq!(code, Some(4), "{}", bridge.stderr());
    assert_eq!(sent_state(&world, &url, &request), "Sent");
}

/// `inbox bridge --clock-file`: the file removed while the bridge runs. Like the service's clock
/// it fails closed: the bridge ends with 4 and one line naming `inbox bridge` and the file.
#[test]
fn adversary_the_bridge_clock_file_removed_while_it_runs_ends_it_with_4() {
    let world = clocked("adv-bridge-once-clock-gone");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    fs::write(
        world.dir.join("bridge-clock"),
        format_reading(this_second()),
    )
    .expect("the bridge's clock is set");
    let connectors = Connectors::new(&world);
    let mut bridge = bridge_with(
        &world,
        &connectors,
        &url,
        &["--clock-file", "bridge-clock"],
        &[],
    );
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    wait_for("the note is posted", || connectors.posts() == 1);
    fs::remove_file(world.dir.join("bridge-clock")).expect("the clock file is removed");
    let code = exit_code(&mut bridge, "the bridge ends");
    let said = bridge.stderr();
    assert_eq!(code, Some(4), "{said}");
    assert!(
        said.contains("inbox bridge: the clock file") && said.contains("bridge-clock"),
        "{said}"
    );
    assert!(!sent_state(&world, &url, &note).is_empty());
}

// Pass 3.

/// `inbox bridge --clock-file` at the start: an empty file holds no instant. The bridge fails
/// closed before any post (exit 4, one line naming the flag and the file), as `inbox serve` does;
/// it does not fall back to the system clock.
#[test]
fn adversary_an_empty_bridge_clock_file_at_the_start_ends_it_with_4_before_any_post() {
    let world = clocked("adv-bridge-once-clock-empty");
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    fs::write(world.dir.join("bridge-clock"), "").expect("an empty clock file");
    let connectors = Connectors::new(&world);
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    let mut bridge = bridge_with(
        &world,
        &connectors,
        &url,
        &["--clock-file", "bridge-clock"],
        &[],
    );
    let code = exit_code(&mut bridge, "the bridge ends");
    let said = bridge.stderr();
    assert_eq!(code, Some(4), "{said}");
    assert!(
        said.contains("--clock-file") && said.contains("bridge-clock"),
        "{said}"
    );
    assert_eq!(connectors.posts(), 0, "{said}");
    assert_eq!(sent_state(&world, &url, &note), "Sent", "{said}");
}
