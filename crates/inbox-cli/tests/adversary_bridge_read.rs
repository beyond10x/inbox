//! Adversary cases of story:the-chat-bridge-marks-a-message-read-on-a-reaction, pass 1: the read
//! of the chat channel against the provider's `oldest` (exclusive unless `inclusive`), the wait
//! a rate-limited read names and the pause after a failed one, and the bound of 10 pages with the
//! cursor of each page. Each case runs against a real `inbox serve` on loopback and a stand-in
//! `connectors` (POSIX sh) first on `PATH`; no case reaches a chat provider, and a bounded wait
//! only ends a hang.

use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use support::{BOB, World};

const CHANNEL: &str = "C0000000001";
const USER: &str = "U0000000001";
const CONNECTOR: &str = "slack";
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

/// Whether `fact` holds within [`HANG`], without failing.
fn holds_within(mut fact: impl FnMut() -> bool) -> bool {
    let started = Instant::now();
    while !fact() {
        if started.elapsed() >= HANG {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// A stand-in `connectors` in `<case>/connectors/`. Per invocation `n`: `args.<n>`, `input.<n>`,
/// `start.<n>` and `end.<n>` (nanoseconds) and `pid.<n>`. A post `p` is answered with the reference
/// `1700000000.<p, 6 digits>`. A read `r` (`read.<r>` holds `n`) answers, in this order:
/// with `paged` present, `page.<k>` where `k` is 1 without a cursor and `j + 1` for the cursor
/// `c<j>`; with `exclusive` present and an input without `"inclusive":true`, `history-exclusive`
/// (the provider leaves out the post at `oldest`); else `history`; else a page without a post.
/// `err.<r>` goes to standard error and `exit.<r>` is the exit status. While `hold-read` exists a
/// read writes `holding-read.<r>` and waits for `release-read.<r>`. An invocation that starts
/// while another runs writes `overlap`.
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
own=0
if mkdir "$d/busy" 2> /dev/null; then own=1; else : > "$d/overlap"; fi
finish() {{
  if [ "$own" = 1 ]; then rmdir "$d/busy"; fi
  exit "$1"
}}
n=1
while [ -e "$d/args.$n" ]; do n=$((n + 1)); done
echo $$ > "$d/pid.$n"
date +%s%N > "$d/start.$n"
cat > "$d/input.$n"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args.$n.new"
mv "$d/args.$n.new" "$d/args.$n"
case "$5" in
chat.postMessage)
  p=1
  while [ -e "$d/post.$p" ]; do p=$((p + 1)); done
  echo "$n" > "$d/post.$p"
  date +%s%N > "$d/end.$n"
  printf '{{"result":{{"mutation":{{"classification":"applied"}},"result":{{"status":200,"body":{{"ok":true,"ts":"1700000000.%06d"}}}}}}}}\n' "$p"
  finish 0
  ;;
conversations.history)
  r=1
  while [ -e "$d/read.$r" ]; do r=$((r + 1)); done
  echo "$n" > "$d/read.$r"
  if [ -e "$d/hold-read" ]; then
    : > "$d/holding-read.$r"
    i=0
    while [ ! -e "$d/release-read.$r" ] && [ "$i" -lt 1000 ]; do sleep 0.01; i=$((i + 1)); done
  fi
  date +%s%N > "$d/end.$n"
  if [ -e "$d/paged" ]; then
    j=$(sed -n 's/.*"cursor":"c\([0-9]*\)".*/\1/p' "$d/input.$n")
    if [ -z "$j" ]; then k=1; else k=$((j + 1)); fi
    cat "$d/page.$k"
  elif [ -e "$d/exclusive" ] && ! grep -q '"inclusive":true' "$d/input.$n"; then
    cat "$d/history-exclusive"
  elif [ -e "$d/history" ]; then cat "$d/history"
  else printf '%s\n' '{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[]}}}}}}}}'
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

    fn path(&self) -> String {
        format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        )
    }

    fn file(&self, name: &str, text: &str) {
        let staged = self.dir.join(format!("{name}.staged"));
        fs::write(&staged, text).expect("a file of the stand-in is written");
        fs::rename(&staged, self.dir.join(name)).expect("a file of the stand-in is replaced");
    }

    fn exists(&self, name: &str) -> bool {
        self.dir.join(name).exists()
    }

    fn count(&self, prefix: &str) -> usize {
        (1..)
            .take_while(|k| self.dir.join(format!("{prefix}.{k}")).exists())
            .count()
    }

    fn reads(&self) -> usize {
        self.count("read")
    }

    fn invocation(&self, kind: &str, k: usize) -> usize {
        let path = self.dir.join(format!("{kind}.{k}"));
        wait_for(&format!("{kind} {k} is numbered"), || {
            fs::read_to_string(&path).is_ok_and(|text| text.ends_with('\n'))
        });
        fs::read_to_string(&path)
            .expect("the number is recorded")
            .trim()
            .parse()
            .expect("a number")
    }

    fn nanos(&self, name: &str, n: usize) -> u128 {
        let path = self.dir.join(format!("{name}.{n}"));
        wait_for(&format!("{name}.{n} is written"), || {
            fs::read_to_string(&path).is_ok_and(|text| text.ends_with('\n'))
        });
        fs::read_to_string(&path)
            .expect("the time is recorded")
            .trim()
            .parse()
            .expect("a number")
    }

    fn read_input(&self, r: usize) -> String {
        let n = self.invocation("read", r);
        fs::read_to_string(self.dir.join(format!("input.{n}"))).expect("the input is recorded")
    }
}

/// A page of a read that lists `messages` (`ts`, reactions or empty) and the cursor `next`.
fn page(messages: &[(&str, &str)], next: &str) -> String {
    let listed: Vec<String> = messages
        .iter()
        .map(|(ts, reactions)| {
            if reactions.is_empty() {
                format!(r#"{{"ts":"{ts}"}}"#)
            } else {
                format!(r#"{{"ts":"{ts}","reactions":{reactions}}}"#)
            }
        })
        .collect();
    format!(
        r#"{{"result":{{"result":{{"status":200,"body":{{"ok":true,"messages":[{}],"response_metadata":{{"next_cursor":"{next}"}}}}}}}}}}"#,
        listed.join(",")
    )
}

struct Bridge {
    child: Option<Child>,
    stderr: PathBuf,
}

impl Bridge {
    fn start(world: &World, connectors: &Connectors) -> Self {
        let stderr = world.dir.join("bridge.err");
        let child = world
            .client_command("bob", &["bridge"])
            .args([
                "--chat-channel",
                CHANNEL,
                "--allowed-chat-user",
                USER,
                "--chat-connector",
                CONNECTOR,
                "--chat-poll-interval",
                "1",
            ])
            .env("PATH", connectors.path())
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
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = Command::new("kill")
                .args(["-TERM", &child.id().to_string()])
                .status();
            let _ = child.wait();
        }
    }
}

fn post(world: &World, body: &str) -> String {
    world
        .client(
            "alice",
            &["post", "--to", BOB, "--kind", "note", "--body", body],
        )
        .ok()
        .stdout
        .trim()
        .to_owned()
}

fn state(world: &World, message_id: &str) -> String {
    support::rows(&world.client("bob", &["pending", "--json"]).ok().json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

fn named(dir: &Path, prefix: &str) -> Vec<PathBuf> {
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

/// The state the bridge's record holds `message_id` in, or an empty text.
fn recorded(world: &World, message_id: &str) -> String {
    let Some(record) = named(&world.dir.path, "bob.outbox.chat.")
        .into_iter()
        .next()
    else {
        return String::new();
    };
    fs::read_to_string(record)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| inbox::server::json::parse(line).ok())
        .find(|line| support::text(line, "message_id") == message_id)
        .map(|line| support::text(&line, "state"))
        .unwrap_or_default()
}

fn posted_and_delivered(world: &World, message_id: &str) -> bool {
    recorded(world, message_id) == "posted" && state(world, message_id) == "Delivered"
}

/// The provider's `conversations.history` lists the posts after `oldest`, and the post at `oldest`
/// only with `inclusive` true. The oldest open post is the one `oldest` names: a reaction of the
/// allowed chat user on it must be seen, so the read asks for it inclusively.
#[test]
fn adversary_a_reaction_on_the_oldest_open_post_is_seen() {
    let world = World::new("adversary-bridge-read-inclusive");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let ts = "1700000000.000001";
    connectors.file("exclusive", "");
    connectors.file("history", &page(&[(ts, REACTED)], ""));
    connectors.file("history-exclusive", &page(&[], ""));
    let bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("the note is posted", || {
        recorded(&world, &note) == "posted" || recorded(&world, &note) == "read"
    });
    let read = holds_within(|| recorded(&world, &note) == "read");
    assert!(
        read,
        "the reaction on the only open post is never seen: the read input is {:?} (no \
         `\"inclusive\":true`), and the provider leaves out the post at `oldest`; stderr {}",
        connectors.read_input(1),
        bridge.stderr()
    );
}

/// A read refused `rate_limited` with `retry_after_seconds` 1 is followed by the next read not
/// before 1 s; a read refused `forbidden` after it by the next one not before the pause of a second
/// failure in a row (200 ms). One line names the run.
#[test]
fn adversary_a_rate_limited_read_waits_its_named_time_and_a_failed_one_its_pause() {
    let world = World::new("adversary-bridge-read-retry-after");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let refusal = |class: &str, extra: &str| {
        format!(
            r#"{{"ok":false,"error":{{"code":"failure","data":{{"code":"service_failure","stage":"dispatch","service_code":"{class}"{extra}}}}}}}"#
        )
    };
    connectors.file(
        "err.1",
        &refusal("rate_limited", r#","retry_after_seconds":1"#),
    );
    connectors.file("exit.1", "1");
    connectors.file("err.2", &refusal("forbidden", ""));
    connectors.file("exit.2", "1");
    let bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("3 reads", || connectors.reads() >= 3);
    let gap = |r: usize| {
        let ended = connectors.nanos("end", connectors.invocation("read", r));
        let next = connectors.nanos("start", connectors.invocation("read", r + 1));
        next.saturating_sub(ended)
    };
    let (after_limit, after_forbidden) = (gap(1), gap(2));
    assert!(
        after_limit >= 1_000_000_000,
        "the read after `rate_limited` with 1 s came {after_limit} ns after it"
    );
    assert!(
        after_forbidden >= 200_000_000,
        "the read after a second failure in a row came {after_forbidden} ns after it"
    );
    assert_eq!(recorded(&world, &note), "posted");
    assert_eq!(bridge.stderr().lines().count(), 1, "{}", bridge.stderr());
}

/// A poll reads at most 10 pages, each after the first with the cursor of the page before: a
/// reaction on the 10th page reads the note; a channel with an 11th page is a failed poll, named
/// once, and marks nothing.
#[test]
fn adversary_a_poll_reads_ten_pages_and_fails_at_an_eleventh() {
    let world = World::new("adversary-bridge-read-pages");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let ts = "1700000000.000001";
    for k in 1..=10 {
        connectors.file(&format!("page.{k}"), &page(&[], &format!("c{k}")));
    }
    connectors.file("page.11", &page(&[(ts, REACTED)], ""));
    connectors.file("paged", "");
    let bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("a poll of 10 pages ends", || {
        bridge.stderr().contains("more than 10 pages") || recorded(&world, &note) == "read"
    });
    assert_eq!(recorded(&world, &note), "posted", "no mark at an 11th page");
    for r in 2..=10 {
        assert_eq!(
            connectors.read_input(r),
            format!(
                r#"{{"channel":"{CHANNEL}","oldest":"{ts}","inclusive":true,"limit":100,"cursor":"c{}"}}"#,
                r - 1
            ),
            "read {r}"
        );
    }
    let stderr = bridge.stderr();
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains("more than 10 pages"), "{stderr}");
    connectors.file("page.10", &page(&[(ts, REACTED)], ""));
    wait_for("the note on the 10th page is read", || {
        recorded(&world, &note) == "read"
    });
}

/// A SIGTERM while a read of the channel holds ends the bridge by that signal, ends the read's
/// `connectors` with it, and writes no line: the thread whose child the signal path took names no
/// failed read.
#[test]
fn adversary_a_signal_during_a_read_ends_it_and_names_nothing() {
    let world = World::new("adversary-bridge-read-signal");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let mut bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("the note is posted", || posted_and_delivered(&world, &note));
    connectors.file("hold-read", "");
    let r = connectors.reads() + 1;
    wait_for("a read holds", || {
        connectors.exists(&format!("holding-read.{r}"))
    });
    let pid = fs::read_to_string(
        connectors
            .dir
            .join(format!("pid.{}", connectors.invocation("read", r))),
    )
    .expect("the pid of the read is recorded")
    .trim()
    .to_owned();
    let mut child = bridge.child.take().expect("running");
    let sent = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success());
    let status = child.wait().expect("the bridge is waited for");
    wait_for("the read's connectors has ended", || {
        !Path::new(&format!("/proc/{pid}")).exists()
    });
    use std::os::unix::process::ExitStatusExt as _;
    assert_eq!(status.signal(), Some(15), "{status:?}");
    assert_eq!(bridge.stderr(), "", "no line");
}

/// Pass 2: a line of the record written before the channel was recorded (no `channel` member)
/// counts as of the bound channel: a bridge bound to that channel asks about it and marks its
/// message read on a reaction of the allowed chat user.
#[test]
fn adversary_an_entry_without_a_channel_is_asked_about_as_of_the_bound_channel() {
    let world = World::new("adversary-bridge-read-no-channel");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let first = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("the note is posted", || posted_and_delivered(&world, &note));
    drop(first);
    let record = named(&world.dir.path, "bob.outbox.chat.")
        .into_iter()
        .next()
        .expect("the record");
    let text = fs::read_to_string(&record).expect("the record is read");
    let member = format!(r#""channel":"{CHANNEL}","#);
    assert!(text.contains(&member), "{text}");
    fs::write(&record, text.replace(&member, "")).expect("the record is written");
    connectors.file("history", &page(&[("1700000000.000001", REACTED)], ""));
    let second = Bridge::start(&world, &connectors);
    let read = holds_within(|| recorded(&world, &note) == "read");
    assert!(
        read,
        "an entry without a channel is not asked about; record {:?}; stderr {}",
        fs::read_to_string(&record).unwrap_or_default(),
        second.stderr()
    );
}

/// The number of the system call `futex`, the wait of a lock inside a process.
#[cfg(target_arch = "x86_64")]
const FUTEX: &str = "202";
#[cfg(target_arch = "aarch64")]
const FUTEX: &str = "98";

/// Whether the main thread of `pid` (the rounds of the bridge) waits in `futex`.
fn main_thread_waits_on_a_lock(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/task/{pid}/syscall"))
        .is_ok_and(|call| call.split_whitespace().next() == Some(FUTEX))
}

/// Pass 2: a SIGTERM while a read of the channel holds and a post waits for the turn ends the
/// bridge by that signal and ends the read's `connectors`; the waiting post starts no child, its
/// message stays in `Sent`, and no line is written.
#[test]
fn adversary_a_signal_while_a_post_waits_for_a_read_starts_no_post() {
    let world = World::new("adversary-bridge-read-signal-waiting");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let mut bridge = Bridge::start(&world, &connectors);
    let first = post(&world, "Build 7 is green.");
    wait_for("the note is posted", || {
        posted_and_delivered(&world, &first)
    });
    connectors.file("hold-read", "");
    let r = connectors.reads() + 1;
    wait_for("a read holds", || {
        connectors.exists(&format!("holding-read.{r}"))
    });
    let second = post(&world, "Build 8 is green.");
    let mut child = bridge.child.take().expect("running");
    let pid = child.id();
    wait_for("the post waits for the turn", || {
        main_thread_waits_on_a_lock(pid)
    });
    let read_pid = fs::read_to_string(
        connectors
            .dir
            .join(format!("pid.{}", connectors.invocation("read", r))),
    )
    .expect("the pid of the read is recorded")
    .trim()
    .to_owned();
    let sent = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success());
    let status = child.wait().expect("the bridge is waited for");
    wait_for("the read's connectors has ended", || {
        !Path::new(&format!("/proc/{read_pid}")).exists()
    });
    use std::os::unix::process::ExitStatusExt as _;
    assert_eq!(status.signal(), Some(15), "{status:?}");
    assert_eq!(connectors.count("post"), 1, "no post after the signal");
    assert_eq!(state(&world, &second), "Sent");
    assert_eq!(bridge.stderr(), "", "no line");
}

/// Pass 2: the change of state of an entry keeps the channel of its post: after a reaction marks
/// the note read, its line of the record still names the channel it was posted in (a later story
/// reads the thread of a post in `Read` from the record).
#[test]
fn adversary_a_marked_entry_keeps_its_channel() {
    let world = World::new("adversary-bridge-read-mark-channel");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("history", &page(&[("1700000000.000001", REACTED)], ""));
    let bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("the note is read", || recorded(&world, &note) == "read");
    let record = named(&world.dir.path, "bob.outbox.chat.")
        .into_iter()
        .next()
        .expect("the record");
    let text = fs::read_to_string(&record).expect("the record is read");
    let line = text
        .lines()
        .find(|line| line.contains(&note))
        .expect("the line of the note");
    let parsed = inbox::server::json::parse(line).expect("a JSON line");
    assert_eq!(
        support::text(&parsed, "channel"),
        CHANNEL,
        "{line}; stderr {}",
        bridge.stderr()
    );
}

/// A bridge of bob bound to `channel`, as [`Bridge::start`] starts one for [`CHANNEL`].
fn bridge_on(world: &World, connectors: &Connectors, channel: &str) -> Bridge {
    let stderr = world.dir.join("bridge.err");
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
        .env("PATH", connectors.path())
        .stdout(std::process::Stdio::null())
        .stderr(fs::File::create(&stderr).expect("the stderr file is made"))
        .spawn()
        .expect("the binary starts");
    Bridge {
        child: Some(child),
        stderr,
    }
}

/// Pass 3: a restart with a second channel and back to the first. The bridge of the second
/// channel neither asks about nor settles the entries of the first (one of them left `Pending`
/// meanwhile); back on the first channel, that entry is settled without a `read`, the other is
/// asked about from its own post on and read on its reaction, and the entry of the second
/// channel is not asked about.
#[test]
fn adversary_a_restart_on_another_channel_and_back_asks_about_the_first_channel_again() {
    let world = World::new("adversary-bridge-read-channel-back");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let first = Bridge::start(&world, &connectors);
    let left = post(&world, "Build 7 is green.");
    let open = post(&world, "Build 8 is green.");
    wait_for("2 notes are posted", || {
        posted_and_delivered(&world, &left) && posted_and_delivered(&world, &open)
    });
    drop(first);
    world.client("bob", &["read", &left]).ok();

    let other = "C0000000002";
    let second = bridge_on(&world, &connectors, other);
    let elsewhere = post(&world, "Build 9 is green.");
    wait_for("the note of the second channel is posted", || {
        posted_and_delivered(&world, &elsewhere)
    });
    let from = connectors.reads();
    wait_for("a read of the second channel", || connectors.reads() > from);
    let asked = connectors.read_input(from + 1);
    assert!(asked.contains(other), "{asked}");
    assert_eq!(
        recorded(&world, &left),
        "posted",
        "not settled by a bridge of another channel"
    );
    drop(second);

    connectors.file("history", &page(&[("1700000000.000002", REACTED)], ""));
    let reads = connectors.reads();
    let third = Bridge::start(&world, &connectors);
    wait_for(
        "back on the first channel, both entries are settled",
        || recorded(&world, &left) == "read" && recorded(&world, &open) == "read",
    );
    let asked = connectors.read_input(reads + 1);
    assert_eq!(
        asked,
        format!(
            r#"{{"channel":"{CHANNEL}","oldest":"1700000000.000002","inclusive":true,"limit":100}}"#
        ),
        "stderr {}",
        third.stderr()
    );
    assert_eq!(recorded(&world, &elsewhere), "posted");
    assert_eq!(state(&world, &open), "", "the note is handled");
    assert_eq!(third.stderr(), "");
}
