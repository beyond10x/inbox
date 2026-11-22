//! Adversary cases of story:repost-does-not-redeliver-a-handled-note (stage 2, wave 8, pass 1):
//! `inbox repost` against the epochs file beside the outbox file and the `Inbox-Epoch` header.
//! Every key is made at run time with the binary itself.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use inbox::server::json::Value;

use support::{BOB, LOCAL, REFUSED, TRANSPORT, World, ids, text};

/// An address no inbox is opened for in these cases.
const CAROL: &str = "agent:test/carol";

fn epochs_file(outbox: &Path) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(".epochs");
    PathBuf::from(path)
}

fn reposted(document: &Value) -> Vec<Value> {
    match document.member("reposted") {
        Some(Value::Array(entries)) => entries.clone(),
        other => panic!("`reposted` is {other:?} in {document:?}"),
    }
}

fn post(world: &World, who: &str, args: &[&str]) -> support::Run {
    let mut all = vec!["post"];
    all.extend_from_slice(args);
    world.client(who, &all)
}

/// Reads one request from `stream` until the end of its head and as many body bytes as its
/// `Content-Length` says; answers its request line.
fn request_line(stream: &mut TcpStream) -> String {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
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
        let end = received.windows(4).position(|window| window == b"\r\n\r\n");
        if let (None, Some(end)) = (expected, end) {
            let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
            let length: usize = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0);
            expected = Some(end + 4 + length);
        }
    }
    String::from_utf8_lossy(&received)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// A stand-in on 127.0.0.1:0 that answers one request per connection with `answers` in turn:
/// `None` closes the connection after reading the request, without an answer; `Some((status,
/// epoch, body))` answers it. Each request line is sent on the channel as it is read. The thread
/// is never joined, so a client that sends fewer requests than scripted does not hang the case.
fn stand_in(
    answers: Vec<Option<(u16, Option<&'static str>, &'static str)>>,
) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for answer in answers {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let line = request_line(&mut stream);
            let _ = sender.send(line);
            if let Some((status, epoch, body)) = answer {
                let epoch =
                    epoch.map_or_else(String::new, |epoch| format!("Inbox-Epoch: {epoch}\r\n"));
                let text = format!(
                    "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{epoch}Connection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(text.as_bytes());
            }
        }
    });
    (url, receiver)
}

/// Every request line the stand-in read, waiting up to 2 s after the client ended for each.
fn requests(receiver: &mpsc::Receiver<String>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Ok(line) = receiver.recv_timeout(Duration::from_secs(2)) {
        lines.push(line);
    }
    lines
}

/// F1. After a restart, `repost` must post every kept envelope the restart dropped even when the
/// epochs file cannot be written (a full disk, an exhausted quota, a state directory that is not
/// writable): the client of `d52ba0f` wrote nothing in that run and posted every envelope.
#[test]
fn repost_posts_every_dropped_envelope_when_the_epochs_file_cannot_be_written() {
    let mut world = World::new("adv-w8-unwritable");
    world.client("bob", &["open"]).ok();
    let first = post(&world, "alice", &["--to", BOB, "--body", "first"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let second = post(&world, "alice", &["--to", BOB, "--body", "second"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    world.restart();
    world.client("bob", &["open"]).ok();

    let dir = world.dir.path.clone();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).expect("chmod 0500");
    let reposted = world.client("alice", &["repost", "--json"]);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).expect("chmod 0700");

    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    let mut held = ids(&pending);
    held.sort();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(
        held, expected,
        "the restart dropped both notes; repost delivered only {held:?}; repost: {reposted:#?}"
    );
}

/// F2. A post refused `duplicate-key` did not arrive: the service holds another message of the
/// sender under that key and stores nothing. Once that other message is gone, a `repost` in the
/// same start must post the envelope; the client of `d52ba0f` did. The key is held by a second
/// note of the same sender, so the case holds whether a dedup key is shared by every sender or
/// belongs to its sender (story:dedup-key-scope).
#[test]
fn a_post_refused_duplicate_key_is_posted_again_once_the_message_holding_the_key_is_gone() {
    let world = World::new("adv-w8-dedup");
    // Bob's inbox is not open yet: the post is refused `unknown-inbox` and kept.
    let refused = post(
        &world,
        "alice",
        &[
            "--to",
            BOB,
            "--body",
            "for bob",
            "--dedup-key",
            "daily-report",
        ],
    );
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 1);
    let for_bob = kept[0].clone();

    // Bob opens his inbox, and Alice posts a second note under the same key: it arrives, holds
    // the key and is recorded at the epoch of this start.
    world.client("bob", &["open"]).ok();
    let own = post(
        &world,
        "alice",
        &[
            "--to",
            BOB,
            "--body",
            "the second",
            "--dedup-key",
            "daily-report",
        ],
    )
    .ok()
    .stdout
    .trim()
    .to_owned();

    // Alice reposts: the second note is left alone; for the first the key is taken, the service
    // answers `duplicate-key` and stores nothing.
    let first = world.client("alice", &["repost", "--json"]).ok().json();
    let entries = reposted(&first);
    assert_eq!(entries.len(), 1, "{first:?}");
    assert_eq!(
        text(entries[0].member("answer").expect("an answer"), "outcome"),
        "duplicate-key"
    );
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&own),
        "alice's first note was not stored"
    );

    // Bob reads the second note: the key is free again.
    world.client("bob", &["read", &own]).ok();
    let again = world.client("alice", &["repost", "--json"]);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        vec![for_bob.clone()],
        "the note {for_bob} never reached bob and repost left it alone: {again:#?}"
    );
}

/// An answer to the epoch question without the header makes `repost` post every kept line: the
/// one recorded at an epoch and the one recorded at none.
#[test]
fn an_answer_without_the_epoch_header_posts_the_kept_lines_without_an_epoch_too() {
    let world = World::new("adv-w8-no-header-mixed");
    world.client("bob", &["open"]).ok();
    post(&world, "alice", &["--to", BOB, "--body", "recorded"]).ok();
    let refused = post(&world, "alice", &["--to", CAROL, "--body", "not recorded"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let outbox = world.outbox("alice");
    let recorded = fs::read_to_string(epochs_file(&outbox)).expect("the epochs file");
    assert_eq!(recorded.lines().count(), 1, "{recorded:?}");
    assert_eq!(world.kept("alice").len(), 2);

    let (url, served) = stand_in(vec![
        Some((200, None, r#"{"rows":[]}"#)),
        Some((202, None, r#"{"outcome":"posted","published":[]}"#)),
        Some((202, None, r#"{"outcome":"posted","published":[]}"#)),
    ]);
    let outbox_text = outbox.display().to_string();
    let run = world.client_env(
        "alice",
        &["repost", "--json"],
        &[
            ("INBOX_URL", url.as_str()),
            ("INBOX_OUTBOX", outbox_text.as_str()),
        ],
    );
    let lines = requests(&served);
    let run = run.ok();
    assert_eq!(reposted(&run.json()).len(), 2, "{run:#?}");
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[0].starts_with("GET "), "{lines:?}");
    assert!(
        lines[1].starts_with("POST ") && lines[2].starts_with("POST "),
        "{lines:?}"
    );
}

/// A transport failure of the epoch question ends `repost` with exit 3, before any post.
#[test]
fn a_transport_failure_of_the_epoch_question_ends_repost_with_exit_3_and_posts_nothing() {
    let world = World::new("adv-w8-question-fails");
    world.client("bob", &["open"]).ok();
    post(&world, "alice", &["--to", BOB, "--body", "recorded"]).ok();
    let outbox = world.outbox("alice");
    let (url, served) = stand_in(vec![
        None,
        Some((
            202,
            Some("0123456789abcdef0123456789abcdef"),
            r#"{"outcome":"posted","published":[]}"#,
        )),
    ]);
    let outbox_text = outbox.display().to_string();
    let run = world.client_env(
        "alice",
        &["repost", "--json"],
        &[
            ("INBOX_URL", url.as_str()),
            ("INBOX_OUTBOX", outbox_text.as_str()),
        ],
    );
    let lines = requests(&served);
    assert_eq!(run.status, TRANSPORT, "{run:#?}");
    assert_eq!(lines.len(), 1, "only the question: {lines:?}");
    assert!(lines[0].starts_with("GET "), "{lines:?}");
}

/// A post that arrived and whose epoch cannot be recorded prints its message id, reports the
/// local failure and exits 4; the note is held by the service.
#[test]
fn a_post_that_arrived_and_cannot_record_its_epoch_prints_its_id_and_exits_4() {
    let world = World::new("adv-w8-post-unrecorded");
    world.client("bob", &["open"]).ok();
    let outbox = world.outbox("alice");
    let mut staged = epochs_file(&outbox).into_os_string();
    staged.push(".new");
    fs::create_dir(&staged).expect("a directory where the staged epochs file goes");
    let run = post(&world, "alice", &["--to", BOB, "--body", "arrives"]);
    assert_eq!(run.status, LOCAL, "{run:#?}");
    let message_id = run.stdout.trim().to_owned();
    assert_eq!(world.kept("alice"), vec![message_id.clone()]);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), [message_id]);
}

/// F3. Every command that touches the outbox file reads the epochs file; reading one of 20 000
/// lines must take about as long as reading the outbox file, not seconds.
#[test]
fn an_epochs_file_of_20000_lines_is_read_within_2_seconds() {
    let world = World::new("adv-w8-long-epochs");
    world.client("bob", &["open"]).ok();
    post(&world, "alice", &["--to", BOB, "--body", "recorded"]).ok();
    let outbox = world.outbox("alice");
    let path = epochs_file(&outbox);
    let mut text = fs::read_to_string(&path).expect("the epochs file");
    let epoch = text
        .lines()
        .next()
        .and_then(|line| line.split_once('\t'))
        .map(|(_, epoch)| epoch.to_owned())
        .expect("an entry");
    for n in 0..20_000_u32 {
        text.push_str(&format!("01JD{n:022}\t{epoch}\n"));
    }
    fs::write(&path, text).expect("written");

    // Wall time on a shared host: one run took 2.2 s at a host load of about 80 (2026-11-22).
    // A read that is slow because of the file is slow in every run; a run that waited for a
    // processor is not. So the fastest of at most 5 runs is held to the bound.
    let mut fastest = Duration::MAX;
    for _ in 0..5 {
        let started = Instant::now();
        world.client("alice", &["outbox", "--json"]).ok();
        fastest = fastest.min(started.elapsed());
        if fastest < Duration::from_secs(2) {
            break;
        }
    }
    assert!(
        fastest < Duration::from_secs(2),
        "`inbox outbox` over an epochs file of 20 001 lines took {fastest:?} in the fastest of 5 runs"
    );
}
