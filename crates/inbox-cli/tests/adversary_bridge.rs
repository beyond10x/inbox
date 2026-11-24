//! Adversary pass 1 on story:the-chat-bridge-posts-a-pending-message: cases against `inbox
//! bridge` with a stand-in `connectors` (POSIX sh, written by the case, first on `PATH`). No case
//! reaches a chat provider, a network beyond loopback or a model. Every key is made at run time.

use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use support::{ALICE, Dir, World};

const PERSON: &str = "human:person";
const CHANNEL: &str = "C0000000001";
const USER: &str = "U0000000001";
const CONNECTOR: &str = "slack";
const HANG: Duration = support::STAND_IN_WAIT;

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

fn world(name: &str) -> World {
    let dir = Dir::new(name);
    let alice = support::key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let person = support::key_new(&dir.path, "person.key", Some(("person-1", PERSON)));
    let operator_public_key = support::key_new(&dir.path, "operator.key", None);
    support::add_key(&dir.path, "operator.key", "alice-1", ALICE, &alice);
    support::add_key(&dir.path, "operator.key", "person-1", PERSON, &person);
    let service = support::serve(&dir.path, &operator_public_key);
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// A stand-in `connectors`: per invocation `n` it writes its pid to `pid.<n>`, its input to
/// `input.<n>`, its arguments to `args.<n>`; while `hold` exists it waits for `release.<n>` (at
/// most 10 s); with `grandchild` it leaves a process behind that keeps its standard output and
/// standard error open until `release-grandchild` exists (at most 30 s, longer than the bound `HANG` of a case); with `grandchild-first`
/// it starts, before it holds, a process with its pid in `gpid` that holds no pipe of the bridge
/// and runs until `release-grandchild` exists (at most 30 s, longer than the bound `HANG` of a case); it answers `reply.<n>`
/// or `reply` on standard output, `err.<n>` on standard error, and exits `exit.<n>` or 0.
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
echo $$ > "$d/pid.$n"
cat > "$d/input.$n"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args.$n.new"
mv "$d/args.$n.new" "$d/args.$n"
if [ -e "$d/grandchild-first" ]; then
  sh -c 'echo $$ > "$1/gpid"; i=0; while [ ! -e "$1/release-grandchild" ] && [ "$i" -lt 3000 ]; do sleep 0.01; i=$((i + 1)); done' sh "$d" > /dev/null 2>&1 &
fi
if [ -e "$d/hold" ]; then
  i=0
  while [ ! -e "$d/release.$n" ] && [ "$i" -lt 3000 ]; do sleep 0.01; i=$((i + 1)); done
fi
if [ -e "$d/grandchild" ]; then
  ( i=0; while [ ! -e "$d/release-grandchild" ] && [ "$i" -lt 3000 ]; do sleep 0.01; i=$((i + 1)); done ) &
fi
if [ -e "$d/reply.$n" ]; then cat "$d/reply.$n"; elif [ -e "$d/reply" ]; then cat "$d/reply"; fi
if [ -e "$d/err.$n" ]; then cat "$d/err.$n" >&2; fi
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

    fn calls(&self) -> usize {
        (1..)
            .take_while(|n| self.dir.join(format!("args.{n}")).exists())
            .count()
    }

    fn args(&self, n: usize) -> Vec<String> {
        fs::read_to_string(self.dir.join(format!("args.{n}")))
            .expect("the arguments are recorded")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn pid(&self, n: usize) -> u32 {
        let text = fs::read_to_string(self.dir.join(format!("pid.{n}"))).expect("the pid");
        text.trim().parse().expect("a pid")
    }

    /// The text member of the input document of invocation `n`.
    fn text(&self, n: usize) -> String {
        let raw = fs::read_to_string(self.dir.join(format!("input.{n}"))).expect("the input");
        let input = inbox::server::json::parse(&raw).expect("one JSON document");
        support::text(input.member("body").expect("a body"), "text")
    }
}

/// Whether the process `pid` has ended (or is a zombie) within [`HANG`], which only ends a hang:
/// a SIGKILL takes effect after `kill` returns, so the case waits for the fact. Every process of
/// the stand-in lives at least 30 s unless released, longer than [`HANG`].
fn gone(pid: u32) -> bool {
    let started = Instant::now();
    while alive(pid) {
        if started.elapsed() >= HANG {
            return false;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    true
}

/// Whether the process `pid` runs (exists and is no zombie).
fn alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| {
            stat.rsplit_once(')')
                .map(|(_, rest)| rest.trim_start().chars().next())
        })
        .flatten()
        .is_some_and(|state| state != 'Z')
}

fn answer(ts: &str) -> String {
    format!(
        r#"{{"result":{{"mutation":{{"classification":"applied"}},"result":{{"status":200,"body":{{"ok":true,"channel":"{CHANNEL}","ts":"{ts}"}}}}}}}}"#
    )
}

struct Bridge(Option<Child>);

impl Bridge {
    fn start(world: &World, connectors: &Connectors) -> Self {
        let child = world
            .client_command("person", &["bridge"])
            .args([
                "--chat-channel",
                CHANNEL,
                "--allowed-chat-user",
                USER,
                "--chat-connector",
                CONNECTOR,
            ])
            .env("PATH", connectors.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the binary starts");
        Self(Some(child))
    }

    fn terminate(&mut self) {
        let mut child = self.0.take().expect("running");
        let sent = Command::new("kill")
            .args(["-TERM", &child.id().to_string()])
            .status()
            .expect("kill runs");
        assert!(sent.success());
        child.wait().expect("the bridge is waited for");
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn post(world: &World, body: &str) -> String {
    world
        .client(
            "alice",
            &["post", "--to", PERSON, "--kind", "note", "--body", body],
        )
        .ok()
        .stdout
        .trim()
        .to_owned()
}

fn pending_ids(world: &World) -> Vec<String> {
    support::rows(&world.client("person", &["pending", "--json"]).ok().json())
        .iter()
        .map(|row| support::text(row, "message_id"))
        .collect()
}

fn state(world: &World, message_id: &str) -> String {
    support::rows(&world.client("person", &["pending", "--json"]).ok().json())
        .iter()
        .find(|row| support::text(row, "message_id") == message_id)
        .map(|row| support::text(row, "state"))
        .unwrap_or_default()
}

fn key_of(args: &[String]) -> String {
    args.last().cloned().unwrap_or_default()
}

/// Group 4: SIGTERM to the bridge while its `connectors` child is still running. The bridge
/// dies by the default action of the signal; the child must not outlive it (a hung child of a
/// bridge that is gone holds a post nobody judges, and nothing ends it).
#[test]
fn a_signal_to_the_bridge_leaves_no_connectors_child_running() {
    let world = world("adv-bridge-signal");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("hold", "");
    connectors.file("reply", &answer("1700000000.000001"));
    let mut bridge = Bridge::start(&world, &connectors);
    let _note = post(&world, "Build 7 is green.");
    wait_for("the first invocation", || connectors.calls() >= 1);
    let child = connectors.pid(1);
    assert!(alive(child), "the stand-in runs");
    bridge.terminate();
    let outlives = !gone(child);
    connectors.file("release.1", "");
    assert!(
        !outlives,
        "the connectors child {child} still runs after the bridge ended by SIGTERM"
    );
}

/// Group 4: `connectors` exits 0 with a post made and leaves a process behind that holds its
/// standard output (a helper it started). The post is made and the child has ended: the bridge
/// must judge it by the child's exit, not wait for the end of every holder of the pipe.
#[test]
fn a_grandchild_holding_the_pipes_does_not_hold_the_post_of_an_ended_child() {
    let world = world("adv-bridge-grandchild");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("grandchild", "");
    connectors.file("reply", &answer("1700000000.000001"));
    let _bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("the first invocation", || connectors.calls() >= 1);
    let child = connectors.pid(1);
    wait_for("the stand-in ends", || !alive(child));
    let started = Instant::now();
    let mut delivered = false;
    while started.elapsed() < Duration::from_secs(5) {
        if state(&world, &note) == "Delivered" {
            delivered = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    connectors.file("release-grandchild", "");
    assert!(
        delivered,
        "the note is not Delivered 5 s after connectors exited 0 with `applied`: a grandchild \
         holds its standard output"
    );
}

/// Group 6/7: a bridge started while the service does not answer (a closed loopback port) waits
/// for it, as the rounds of `inbox wake --follow` do ("trying again until the service answers"),
/// instead of ending at its first request.
#[test]
fn a_bridge_started_before_the_service_answers_waits_for_it() {
    let world = world("adv-bridge-early");
    let connectors = Connectors::new(&world);
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a loopback port")
        .local_addr()
        .expect("its address")
        .port();
    let mut child = world
        .client_command("person", &["bridge"])
        .args([
            "--chat-channel",
            CHANNEL,
            "--allowed-chat-user",
            USER,
            "--chat-connector",
            CONNECTOR,
        ])
        .env("PATH", connectors.path())
        .env("INBOX_URL", format!("http://127.0.0.1:{port}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary starts");
    std::thread::sleep(Duration::from_secs(1));
    let ended = child.try_wait().expect("the process is asked");
    let _ = child.kill();
    let out = child.wait_with_output().expect("waited for");
    assert!(
        ended.is_none(),
        "the bridge ended with {ended:?} while the service did not answer: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Group 8 (mutant: the cut moved by one byte, `> MOST_CHAT_BODY_BYTES + 1`): a body of 3000
/// ASCII bytes is not cut, one of 3001 is cut to exactly 3000.
#[test]
fn a_body_of_3001_bytes_is_cut_to_3000_and_one_of_3000_is_whole() {
    let world = world("adv-bridge-cut");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("reply", &answer("1700000000.000001"));
    let whole = post(&world, &"a".repeat(3000));
    let over = post(&world, &"b".repeat(3001));
    let _bridge = Bridge::start(&world, &connectors);
    for id in [&whole, &over] {
        wait_for(&format!("{id} is delivered"), || {
            state(&world, id) == "Delivered"
        });
    }
    let by_id = |id: &str| {
        (1..=connectors.calls())
            .find(|n| key_of(&connectors.args(*n)) == format!("inbox-{id}"))
            .unwrap_or_else(|| panic!("no invocation for {id}"))
    };
    let text = connectors.text(by_id(&whole));
    let (_, rest) = text.split_once("\n```\n").expect("a block");
    assert_eq!(rest, format!("{}\n```", "a".repeat(3000)));
    let text = connectors.text(by_id(&over));
    let (_, rest) = text.split_once("\n```\n").expect("a block");
    let (block, after) = rest.split_once("\n```").expect("the end of the block");
    assert_eq!(block, "b".repeat(3000), "cut at exactly 3000 bytes");
    assert!(after.starts_with("\ncut at 3000 bytes"), "{after}");
}

/// Group 8 (mutant: a failed post does not end the round, the next row is posted): two notes in
/// `Pending`, the first post of the first fails; the next invocation is the first note again,
/// so the posts keep the order of `Pending`.
#[test]
fn after_a_failed_post_the_next_invocation_is_the_same_message() {
    let world = world("adv-bridge-order");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("reply", &answer("1700000000.000001"));
    connectors.file(
        "err.1",
        r#"{"ok":false,"error":{"code":"failure","message":"m","data":{"code":"provider_error","mutation":{"classification":"refused"}}}}"#,
    );
    connectors.file("exit.1", "1");
    let a = post(&world, "First");
    let b = post(&world, "Second");
    let order = pending_ids(&world);
    assert_eq!(order.len(), 2);
    let _bridge = Bridge::start(&world, &connectors);
    for id in [&a, &b] {
        wait_for(&format!("{id} is delivered"), || {
            state(&world, id) == "Delivered"
        });
    }
    let keys: Vec<String> = (1..=connectors.calls())
        .map(|n| key_of(&connectors.args(n)))
        .collect();
    assert_eq!(
        keys,
        vec![
            format!("inbox-{}", order[0]),
            format!("inbox-{}", order[0]),
            format!("inbox-{}", order[1]),
        ],
        "the failed message is tried again before the next one"
    );
}

// Adversary pass 2 (round 2 of the implementor).

/// The bridge of the person with stdout and stderr as given.
fn spawn_bridge(
    world: &World,
    connectors: &Connectors,
    env: &[(&str, &str)],
    stdout: std::process::Stdio,
    stderr: std::process::Stdio,
) -> Child {
    world
        .client_command("person", &["bridge"])
        .args([
            "--chat-channel",
            CHANNEL,
            "--allowed-chat-user",
            USER,
            "--chat-connector",
            CONNECTOR,
        ])
        .env("PATH", connectors.path())
        .envs(env.iter().copied())
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .expect("the binary starts")
}

/// Pass 2, group 1: SIGTERM to the bridge while its `connectors` child holds its answer, after
/// the child started a helper process of its own (which holds no pipe of the bridge). The child
/// is ended; no process the post started may outlive the bridge either.
#[test]
fn a_signal_to_the_bridge_leaves_no_process_of_the_post_running() {
    let world = world("adv2-bridge-signal-helper");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("hold", "");
    connectors.file("grandchild-first", "");
    connectors.file("reply", &answer("1700000000.000001"));
    let mut bridge = Bridge::start(&world, &connectors);
    let _note = post(&world, "Build 7 is green.");
    wait_for("the helper of the first invocation", || {
        connectors.dir.join("gpid").exists() && connectors.calls() >= 1
    });
    wait_for("the helper's pid", || {
        fs::read_to_string(connectors.dir.join("gpid")).is_ok_and(|text| text.ends_with('\n'))
    });
    let helper: u32 = fs::read_to_string(connectors.dir.join("gpid"))
        .expect("the helper's pid")
        .trim()
        .parse()
        .expect("a pid");
    let child = connectors.pid(1);
    bridge.terminate();
    let (child_alive, helper_alive) = (!gone(child), !gone(helper));
    connectors.file("release.1", "");
    connectors.file("release-grandchild", "");
    assert!(
        !child_alive,
        "the connectors child {child} outlives the bridge"
    );
    assert!(
        !helper_alive,
        "the helper {helper} that the connectors child started outlives the bridge ended by SIGTERM"
    );
}

/// Pass 2, group 6 (mutant: `read > room` as `read >= room` in `read_into`): an answer of
/// exactly 262144 bytes (`MOST_ANSWER_BYTES`) is a post made, one of 262145 bytes is not.
#[test]
fn an_answer_of_exactly_the_bound_is_a_post_made() {
    const MOST: usize = 256 * 1024;
    let world = world("adv2-bridge-bound");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    let made = answer("1700000000.000001");
    connectors.file(
        "reply.1",
        &format!("{made}{}", " ".repeat(MOST - made.len() + 1)),
    );
    connectors.file(
        "reply.2",
        &format!("{made}{}", " ".repeat(MOST - made.len())),
    );
    let mut bridge = Bridge::start(&world, &connectors);
    let note = post(&world, "Build 7 is green.");
    wait_for("the note is delivered", || {
        state(&world, &note) == "Delivered"
    });
    bridge.terminate();
    assert_eq!(
        connectors.calls(),
        2,
        "the answer of 262145 bytes fails, the one of 262144 bytes is a post made"
    );
}

/// Pass 2, groups 3 and 6 (mutant: the start line printed on every try): a bridge started while
/// the service does not answer prints one line over several tries (100, 200, 400, 800 ms of
/// pause), and a SIGTERM during the pauses ends it by the signal at once.
#[test]
fn a_bridge_that_waits_for_the_service_says_so_once_and_ends_on_a_signal() {
    let world = world("adv2-bridge-early");
    let connectors = Connectors::new(&world);
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a loopback port")
        .local_addr()
        .expect("its address")
        .port();
    let url = format!("http://127.0.0.1:{port}");
    let errors = world.dir.join("early.err");
    let mut child = spawn_bridge(
        &world,
        &connectors,
        &[("INBOX_URL", &url)],
        std::process::Stdio::null(),
        fs::File::create(&errors).expect("the stderr file").into(),
    );
    std::thread::sleep(Duration::from_millis(1600));
    let running = child.try_wait().expect("asked").is_none();
    let sent = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success());
    let asked = Instant::now();
    let mut status = None;
    while asked.elapsed() < HANG {
        if let Some(ended) = child.try_wait().expect("asked") {
            status = Some(ended);
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let _ = child.kill();
    let _ = child.wait();
    let stderr = fs::read_to_string(&errors).unwrap_or_default();
    assert!(running, "the bridge waits for the service: {stderr}");
    assert_eq!(
        stderr.lines().count(),
        1,
        "one line over the tries of the start: {stderr}"
    );
    let status = status.expect("the bridge ends on SIGTERM in a pause of the start");
    assert!(
        status.code().is_none() || status.code() == Some(128 + 15),
        "{status:?}"
    );
}

/// Pass 2, group 4 (mutant: the bridge gets the watch of the reader of standard output): a
/// bridge whose standard output is a pipe with no reader left (a service manager that dropped
/// it) writes nothing there, so it runs on, posts and marks.
#[test]
fn a_bridge_whose_standard_output_has_no_reader_runs_on() {
    let world = world("adv2-bridge-no-reader");
    world.client("person", &["open", "--adapter", "slack"]).ok();
    let connectors = Connectors::new(&world);
    connectors.file("reply", &answer("1700000000.000001"));
    let mut child = spawn_bridge(
        &world,
        &connectors,
        &[],
        std::process::Stdio::piped(),
        std::process::Stdio::null(),
    );
    drop(child.stdout.take());
    std::thread::sleep(Duration::from_millis(1500));
    let running = child.try_wait().expect("asked").is_none();
    let note = post(&world, "Build 7 is green.");
    if running {
        wait_for("the note is delivered", || {
            state(&world, &note) == "Delivered"
        });
    }
    let _ = child.kill();
    let ended = child.wait().expect("waited for");
    assert!(
        running,
        "the bridge ended ({ended:?}) because its standard output has no reader"
    );
}
