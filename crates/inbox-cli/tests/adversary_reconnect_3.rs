//! Adversary cases of story:the-client-reconnects-after-a-restart (stage 3, pass 3, round 3): the
//! `CloseInbox` a reconnect sends after its open found the record gone, reaching the service after
//! the owner's typed `inbox open`; the record of a typed `inbox open` overwritten by a reconnect's
//! open that was on its way; a post the stopping service answered `503` that no command posts
//! again after the restart; and the epochs file after two commands of one sender posted one
//! envelope at once. Every key is made at run time with the binary itself. Every proxy and
//! stand-in listens on 127.0.0.1 only, and every wait is bounded: a bound only ends a hang.

use crate::support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use inbox::server::json::{self, Value};

use support::{BOB, REFUSED, TRANSPORT, World, ids, text};

/// The bound of every wait in these cases: it only ends a hang.
const HANG: Duration = Duration::from_secs(15);

/// The file beside the outbox file at `outbox` whose name ends in `suffix`.
fn beside(outbox: &Path, suffix: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// The lines of a file; none when it does not exist.
fn lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The message ids in bob's `Pending`, read with bob's own command.
fn pending_of_bob(world: &World) -> Vec<String> {
    ids(&world.client("bob", &["pending", "--json"]).ok().json())
}

/// Waits until `flag` is set; [`HANG`] only ends a hang.
fn wait_for(flag: &AtomicBool, what: &str) {
    let started = std::time::Instant::now();
    while !flag.load(Ordering::SeqCst) {
        assert!(
            started.elapsed() < HANG,
            "{what}: not within {} s",
            HANG.as_secs()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// Reads one request whole from `stream`: its head, then as many bytes as `Content-Length` says.
/// Answers the bytes and the request line.
fn read_request(stream: &mut TcpStream) -> (Vec<u8>, String) {
    let _ = stream.set_read_timeout(Some(support::STAND_IN_WAIT));
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected = None;
    let mut head_end = None;
    loop {
        if expected.is_some_and(|total| received.len() >= total) {
            break;
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => received.extend_from_slice(&buffer[..read]),
        }
        if head_end.is_none() {
            head_end = received.windows(4).position(|window| window == b"\r\n\r\n");
            if let Some(end) = head_end {
                let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                expected = Some(end + 4 + length);
            }
        }
    }
    let line = String::from_utf8_lossy(&received)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    (received, line)
}

/// One gate of a [`Gates`] proxy: set when a request it holds arrived, and the release.
#[derive(Default)]
struct Gate {
    held: AtomicBool,
    release: AtomicBool,
}

/// A proxy on 127.0.0.1:0 in front of the service at `service`: each connection runs in its own
/// thread, the request is read whole, and one whose request line `gate` names is held at that
/// gate until it is released (or [`HANG`] passes); then it is carried on and the answer carried
/// back.
struct Gates {
    url: String,
    gates: Arc<Vec<Gate>>,
}

impl Gates {
    fn start(service: &str, count: usize, gate: fn(&str) -> Option<usize>) -> Self {
        let target = service
            .strip_prefix("http://")
            .expect("a plain service")
            .to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").expect("the proxy binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let gates: Arc<Vec<Gate>> = Arc::new((0..count).map(|_| Gate::default()).collect());
        let shared = Arc::clone(&gates);
        std::thread::spawn(move || {
            while let Some(mut client) = support::accept_within(&listener, support::STAND_IN_WAIT) {
                let target = target.clone();
                let gates = Arc::clone(&shared);
                std::thread::spawn(move || {
                    let (bytes, line) = read_request(&mut client);
                    if let Some(index) = gate(&line) {
                        let at = &gates[index];
                        at.held.store(true, Ordering::SeqCst);
                        let started = std::time::Instant::now();
                        while !at.release.load(Ordering::SeqCst) && started.elapsed() < HANG {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                    }
                    let Ok(mut service) = TcpStream::connect(&target) else {
                        return;
                    };
                    let _ = service.set_read_timeout(Some(HANG));
                    if service.write_all(&bytes).is_err() {
                        return;
                    }
                    let mut answer = Vec::new();
                    let _ = service.read_to_end(&mut answer);
                    let _ = client.write_all(&answer);
                    let _ = client.shutdown(std::net::Shutdown::Both);
                });
            }
        });
        Self { url, gates }
    }

    fn wait_held(&self, index: usize, what: &str) {
        wait_for(&self.gates[index].held, what);
    }

    fn release(&self, index: usize) {
        self.gates[index].release.store(true, Ordering::SeqCst);
    }
}

/// The gates of the two cases on the open of a reconnect: 0 holds `OpenInbox`, 1 `CloseInbox`.
fn open_and_close(line: &str) -> Option<usize> {
    if line.contains("/open-inbox ") {
        Some(0)
    } else if line.contains("/close-inbox ") {
        Some(1)
    } else {
        None
    }
}

/// After a restart bob's `inbox pending` reconnects; its open is on its way when bob types
/// `inbox close` (refused `UnknownInbox`, the record is taken). The open then reaches the service
/// and finds no record, so the reconnect sends `CloseInbox` (`closed_meanwhile`). While that
/// close is on its way bob types `inbox open`, which opens the inbox and writes a new record. The
/// reconnect's close then reaches the service and closes the inbox bob opened last: his record
/// says open, the service holds it closed, and a post to him is refused until the next restart.
#[test]
fn a_reconnects_close_after_closed_meanwhile_does_not_undo_a_typed_open_that_came_between() {
    let mut world = World::new("adv3-closed-meanwhile-reopened");
    world.client("bob", &["open"]).ok();
    world.restart();

    let proxy = Gates::start(&world.service.url, 2, open_and_close);
    let reconnecting = world
        .client_command("bob", &["pending"])
        .env("INBOX_URL", &proxy.url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary starts");
    proxy.wait_held(0, "the reconnect's open reaches the proxy");
    let closed = world.client("bob", &["close"]);
    assert_eq!(closed.status, REFUSED, "{closed:#?}");
    proxy.release(0);
    proxy.wait_held(1, "the reconnect's close reaches the proxy");
    let opened = world.client("bob", &["open"]).ok();
    proxy.release(1);
    let output = reconnecting.wait_with_output().expect("the pending ends");
    let said = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(said.contains("closed meanwhile"), "{said}");
    assert_eq!(
        lines(&beside(&world.outbox("bob"), ".opened")).len(),
        1,
        "bob's typed open wrote a record"
    );

    let post = world.client("alice", &["post", "--to", BOB, "--body", "is bob there?"]);
    assert_eq!(
        post.status, 0,
        "bob's last command opened his inbox ({opened:?}); the reconnect's CloseInbox that \
         reached the service after it closed it: {post:#?}; pending said {said:?}"
    );
}

/// Bob opened his inbox with the adapter `terminal`. After a restart his `inbox pending`
/// reconnects; its open (`terminal`) is on its way when bob types `inbox close` and then
/// `inbox open --adapter slack`, which writes the record `slack`. The reconnect's open then
/// reaches the service (`already-open`, the service keeps `slack`) and, the file holding a record
/// again, `record_opened` with `held_only` replaces bob's record with `terminal`: the adapter bob
/// chose last is lost, and the reconnect after the next restart opens with the old one.
#[test]
fn a_reconnects_open_on_its_way_does_not_overwrite_the_record_of_a_typed_open_after_a_close() {
    let mut world = World::new("adv3-record-overwritten");
    world.client("bob", &["open", "--adapter", "terminal"]).ok();
    world.restart();

    let proxy = Gates::start(&world.service.url, 2, open_and_close);
    let reconnecting = world
        .client_command("bob", &["pending"])
        .env("INBOX_URL", &proxy.url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary starts");
    proxy.wait_held(0, "the reconnect's open reaches the proxy");
    let closed = world.client("bob", &["close"]);
    assert_eq!(closed.status, REFUSED, "{closed:#?}");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    proxy.release(0);
    proxy.release(1);
    let output = reconnecting.wait_with_output().expect("the pending ends");
    let said = String::from_utf8_lossy(&output.stderr).into_owned();

    let records: Vec<Value> = lines(&beside(&world.outbox("bob"), ".opened"))
        .iter()
        .map(|line| json::parse(line).expect("a record is JSON"))
        .collect();
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(
        text(&records[0], "adapter"),
        "slack",
        "bob's last typed open chose `slack`; the reconnect's open that was on its way wrote its \
         own adapter over that record; pending said {said:?}"
    );
}

/// Alice's `inbox post` reaches a service that is stopping: the service answers `503`
/// (`{"refused":"stopping"}`, what the listener writes for a request still arriving when the
/// service stops) and stores nothing. `post` takes that as a refusal of the service's own and
/// removes the mark, so after the restart no command of alice posts the envelope: it is never
/// delivered unless she types `inbox repost`, though the pages say the sender's next command
/// posts a post that did not reach the service.
#[test]
fn a_post_the_stopping_service_answered_503_is_posted_again_by_a_later_command() {
    let world = World::new("adv3-stopping-503");
    world.client("bob", &["open"]).ok();
    let (stopping, answered) = support::stand_in_answers(vec![(
        503,
        Some("0123456789abcdef0123456789abcdef".to_owned()),
        r#"{"refused":"stopping"}"#,
    )]);
    let refused = world.client_env(
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--body",
            "sent while the service stopped",
        ],
        &[("INBOX_URL", stopping.as_str())],
    );
    support::wait_until_finished(&answered, "the stopping stand-in");
    let _ = answered.join();
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 1, "the envelope is kept");
    let marks = lines(&beside(&world.outbox("alice"), ".epochs"));

    world.client("alice", &["outbox"]).ok();
    assert_eq!(
        pending_of_bob(&world),
        kept,
        "the post the stopping service answered 503 never reached it, and alice's next command \
         after the restart did not post it again (epochs file after the 503: {marks:?})"
    );
}

/// Alice keeps a note marked unsent; her `inbox post` of a second note is on its way when her
/// `inbox outbox` reconnects and posts both. Afterwards each kept envelope has one epoch and no
/// mark: the epochs file names each once, at the service's epoch.
#[test]
fn two_commands_that_post_one_envelope_at_once_leave_one_epoch_and_no_mark() {
    let world = World::new("adv3-at-once-epochs");
    world.client("bob", &["open"]).ok();
    let closed_port = support::ClosedPort::new();
    let down = closed_port.url();
    let failed = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "while the service was down"],
        &[("INBOX_URL", down.as_str())],
    );
    assert_eq!(failed.status, TRANSPORT, "{failed:#?}");

    let proxy = Gates::start(&world.service.url, 1, |line| {
        line.contains("/post ").then_some(0)
    });
    let on_its_way = world
        .client_command("alice", &["post", "--to", BOB, "--body", "on its way"])
        .env("INBOX_URL", &proxy.url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary starts");
    proxy.wait_held(0, "the post reaches the proxy");
    world.client("alice", &["outbox"]).ok();
    proxy.release(0);
    let output = on_its_way.wait_with_output().expect("the post ends");
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    let kept = world.kept("alice");
    let mut named: Vec<(String, String)> = lines(&beside(&world.outbox("alice"), ".epochs"))
        .iter()
        .map(|line| {
            let (id, epoch) = line.split_once('\t').expect("a tab");
            (id.to_owned(), epoch.to_owned())
        })
        .collect();
    named.sort();
    let mut expected = kept.clone();
    expected.sort();
    assert_eq!(
        named.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>(),
        expected,
        "{named:?}"
    );
    assert!(
        named
            .iter()
            .all(|(_, epoch)| epoch.len() == 32 && epoch == &named[0].1),
        "one epoch each, no mark: {named:?}"
    );
    assert_eq!(pending_of_bob(&world).len(), 2);
}

/// Alice posts a note with the dedup key `the-one-question`, then a second with the same key: the
/// service answers the second `Duplicate` with the outcome `duplicate-key` (another message holds
/// the key, nothing of this one is stored). `inbox post` exits 1, drops the envelope and prints
/// no message id; it is not taken as arrived the way `duplicate` for its own id is
/// (`own_duplicate`).
#[test]
fn a_typed_post_refused_duplicate_key_exits_1_and_keeps_nothing() {
    let world = World::new("adv3-typed-duplicate-key");
    world.client("bob", &["open"]).ok();
    let post = |body: &str| {
        world.client(
            "alice",
            &[
                "post",
                "--to",
                BOB,
                "--dedup-key",
                "the-one-question",
                "--body",
                body,
            ],
        )
    };
    post("first").ok();
    let first = world.kept("alice");
    let second = post("second");
    assert_eq!(second.status, REFUSED, "{second:#?}");
    assert!(second.stderr.contains("Duplicate"), "{second:#?}");
    assert_eq!(
        second.stdout, "",
        "no message id of a post that was not stored: {second:#?}"
    );
    assert_eq!(
        world.kept("alice"),
        first,
        "the refused envelope is dropped"
    );
    assert_eq!(pending_of_bob(&world), first);
}
