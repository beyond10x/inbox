//! Adversary cases of story:the-client-reconnects-after-a-restart (stage 3, pass 2, round 2): a
//! post on its way that another command of the sender posts first, a kept envelope the service
//! answered `duplicate-key` that every command posts again, a post killed before its mark that no
//! command posts again, and a close that a reconnect's open on its way undoes. Every key is made
//! at run time with the binary itself. Every proxy and stand-in listens on 127.0.0.1 only, and
//! every wait is bounded: a bound only ends a hang.

use crate::support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use inbox::server::json::Value;

use support::{BOB, REFUSED, TRANSPORT, World, ids, text};

/// The bound of every wait in these cases: it only ends a hang.
const HANG: Duration = Duration::from_secs(15);

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
/// Answers the bytes, the request line and the body.
fn read_request(stream: &mut TcpStream) -> (Vec<u8>, String, String) {
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
    let text = String::from_utf8_lossy(&received).into_owned();
    let line = text.lines().next().unwrap_or_default().to_owned();
    let body = head_end.map_or_else(String::new, |end| text[end + 4..].to_owned());
    (received, line, body)
}

/// Which requests a [`Holding`] proxy holds, by request line and body.
type Hold = fn(&str, &str) -> bool;

/// A proxy on 127.0.0.1:0 in front of the service at `url`. Each connection runs in its own
/// thread: the request is read whole; one that `hold` names sets `held` and waits until `release`
/// is set (or [`HANG`] passes) before it is carried on; then the service's answer is carried back
/// until the service closes the connection (one request per connection). With `forward` false a
/// held request is never carried on: the connection is dropped once `release` is set.
struct Holding {
    url: String,
    held: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl Holding {
    fn start(service: &str, hold: Hold, forward: bool) -> Self {
        let target = service
            .strip_prefix("http://")
            .expect("a plain service")
            .to_owned();
        let listener = TcpListener::bind("127.0.0.1:0").expect("the proxy binds");
        let url = format!("http://{}", listener.local_addr().expect("an address"));
        let held = Arc::new(AtomicBool::new(false));
        let release = Arc::new(AtomicBool::new(false));
        let (held_by, released) = (Arc::clone(&held), Arc::clone(&release));
        std::thread::spawn(move || {
            while let Some(mut client) = support::accept_within(&listener, support::STAND_IN_WAIT) {
                let target = target.clone();
                let (held_by, released) = (Arc::clone(&held_by), Arc::clone(&released));
                std::thread::spawn(move || {
                    let (bytes, line, body) = read_request(&mut client);
                    if hold(&line, &body) {
                        held_by.store(true, Ordering::SeqCst);
                        let started = std::time::Instant::now();
                        while !released.load(Ordering::SeqCst) && started.elapsed() < HANG {
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        if !forward {
                            return;
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
        Self { url, held, release }
    }

    fn release(&self) {
        self.release.store(true, Ordering::SeqCst);
    }
}

/// The message ids the repost step of a reconnect posted, from a `--json` document.
fn reconnect_reposted(document: &Value) -> Vec<String> {
    match document
        .member("reconnected")
        .and_then(|reconnect| reconnect.member("repost"))
        .and_then(|repost| repost.member("reposted"))
    {
        Some(Value::Array(entries)) => entries
            .iter()
            .map(|entry| text(entry, "message_id"))
            .collect(),
        _ => Vec::new(),
    }
}

/// Alice keeps one note marked unsent (posted while the service was down). Her `inbox post` of a
/// second note is on its way (held by a proxy) when her `inbox outbox` runs: its reconnect, due
/// for the marked note, posts every kept envelope at no epoch, the one on its way too, although
/// `Outbox::due` says such an envelope "may still be on its way in another command". When the
/// held post reaches the service it is answered `Duplicate`, and `inbox post` drops the envelope
/// and exits 1 for a message that arrived.
#[test]
fn a_post_on_its_way_is_not_answered_duplicate_because_another_command_reposted_it() {
    let world = World::new("adv2-on-its-way");
    world.client("bob", &["open"]).ok();
    let closed_port = support::ClosedPort::new();
    let down = closed_port.url();
    let failed = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "while the service was down"],
        &[("INBOX_URL", down.as_str())],
    );
    assert_eq!(failed.status, TRANSPORT, "{failed:#?}");

    let proxy = Holding::start(
        &world.service.url,
        |line, body| line.contains("/post ") && body.contains("on its way"),
        true,
    );
    let on_its_way = world
        .client_command("alice", &["post", "--to", BOB, "--body", "on its way"])
        .env("INBOX_URL", &proxy.url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary starts");
    wait_for(&proxy.held, "the post reaches the proxy");
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 2, "both notes are kept");

    let other = world.client("alice", &["outbox", "--json"]).ok();
    let reposted = reconnect_reposted(&other.json());
    proxy.release();
    let output = on_its_way.wait_with_output().expect("the post ends");
    let typed = (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    );
    assert_eq!(
        reposted, kept,
        "the reconnect of `inbox outbox` posted the note on its way: {other:#?}"
    );
    assert_eq!(
        pending_of_bob(&world),
        kept,
        "both notes arrived, once each"
    );
    assert_eq!(
        typed.0,
        Some(0),
        "`inbox post` of a note that arrived exits non-zero because another command of the \
         sender posted it while its own request was on its way: {typed:#?}"
    );
}

/// Alice's post with a dedup key fails in transport (kept, marked unsent); her second post with
/// the same key arrives. The reconnect of that post posts the first again, and the service
/// answers `Duplicate` with the outcome `duplicate-key`: another message holds the key, it will
/// not store this one. The mark stays, so every later command of alice posts it again and says so
/// on standard error, until the envelope expires.
#[test]
fn an_envelope_the_service_answered_duplicate_key_is_not_posted_again_by_every_command() {
    let world = World::new("adv2-duplicate-key");
    world.client("bob", &["open"]).ok();
    let closed_port = support::ClosedPort::new();
    let down = closed_port.url();
    let post = |body: &str, url: Option<&str>| {
        let args = [
            "post",
            "--to",
            BOB,
            "--dedup-key",
            "the-one-question",
            "--body",
            body,
        ];
        match url {
            Some(url) => world.client_env("alice", &args, &[("INBOX_URL", url)]),
            None => world.client("alice", &args),
        }
    };
    let failed = post("first try", Some(down.as_str()));
    assert_eq!(failed.status, TRANSPORT, "{failed:#?}");
    let retried = post("second try", None).ok();
    assert!(
        retried.stderr.contains("reconnect"),
        "the reconnect of the second post posted the first: {retried:#?}"
    );

    let later = world.client("alice", &["outbox", "--json"]).ok();
    let again = world.client("alice", &["outbox", "--json"]).ok();
    assert_eq!(
        (
            reconnect_reposted(&later.json()),
            reconnect_reposted(&again.json())
        ),
        (Vec::new(), Vec::new()),
        "after the service answered duplicate-key, each later command posted the envelope \
         again: {later:#?} {again:#?}"
    );
}

/// Alice's `inbox post` is killed while its request is on its way (a stand-in that never
/// answers, as a service that hangs, and the user ends the command): the envelope is kept and has
/// neither an epoch nor the mark `none`. After a restart of the service and bob's command that
/// opens his inbox again, two commands of alice do not post it: the pages say that the next
/// command of the sender posts its kept envelopes again by itself (`website/docs/limits.md`, "One
/// backend, in memory").
#[test]
fn a_post_killed_on_its_way_is_posted_again_by_a_later_command_after_a_restart() {
    let mut world = World::new("adv2-killed");
    world.client("bob", &["open"]).ok();
    let stand_in = Holding::start(&world.service.url, |line, _| line.contains("/post "), false);
    let mut killed = world
        .client_command(
            "alice",
            &[
                "post",
                "--to",
                BOB,
                "--kind",
                "request",
                "--reply-shape",
                "free",
                "--body",
                "killed on its way",
            ],
        )
        .env("INBOX_URL", &stand_in.url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("the binary starts");
    wait_for(&stand_in.held, "the post reaches the stand-in");
    let _ = killed.kill();
    let _ = killed.wait();
    stand_in.release();
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 1, "the request is kept");

    world.restart();
    world.client("bob", &["pending"]).ok();
    world.client("alice", &["outbox"]).ok();
    world.client("alice", &["outbox"]).ok();
    assert_eq!(
        pending_of_bob(&world),
        kept,
        "after the restart, bob's command and two commands of alice, the request whose post was \
         killed is in bob's Pending once"
    );
}

/// After a restart bob's `inbox pending` reconnects: its open is on its way (held by a proxy)
/// when bob types `inbox close`. The close removes the record and is refused `UnknownInbox` (the
/// new start holds no inbox of bob), so nothing is put back; then the held open reaches the
/// service and opens the inbox again. The client's record says closed, the service holds it
/// open, and a post to the inbox bob closed is accepted.
#[test]
fn a_close_while_a_reconnects_open_is_on_its_way_leaves_the_inbox_closed() {
    let mut world = World::new("adv2-close-race");
    world.client("bob", &["open"]).ok();
    world.restart();

    let proxy = Holding::start(
        &world.service.url,
        |line, _| line.contains("/open-inbox "),
        true,
    );
    let reconnecting = world
        .client_command("bob", &["pending"])
        .env("INBOX_URL", &proxy.url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the binary starts");
    wait_for(&proxy.held, "the reconnect's open reaches the proxy");
    let closed = world.client("bob", &["close"]);
    assert_eq!(closed.status, REFUSED, "{closed:#?}");
    assert!(closed.stderr.contains("UnknownInbox"), "{closed:#?}");
    proxy.release();
    let output = reconnecting.wait_with_output().expect("the pending ends");
    assert!(output.status.success(), "{output:?}");

    let post = world.client("alice", &["post", "--to", BOB, "--body", "is bob there?"]);
    assert_eq!(
        post.status,
        REFUSED,
        "the inbox bob closed takes a post: the reconnect's open reached the service after the \
         close; pending said {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}
