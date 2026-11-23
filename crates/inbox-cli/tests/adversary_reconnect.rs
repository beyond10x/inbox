//! Adversary cases of story:the-client-reconnects-after-a-restart (stage 3, pass 1): a kept
//! request that never arrives by itself, a closed inbox that is opened again, a file beside the
//! outbox that hangs every command, and a typed command whose answer waits for the reconnect.
//! Every key is made at run time with the binary itself. Every stand-in listens on 127.0.0.1
//! only, and every wait is bounded: a bound only ends a hang.

use crate::support;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use support::{BOB, LOCAL, REFUSED, TRANSPORT, World, ids};

/// The bound of every wait in these cases: it only ends a hang.
const HANG: Duration = Duration::from_secs(15);

fn beside(outbox: &Path, suffix: &str) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

/// A loopback URL on which nothing listens: the service is down.
fn down_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    drop(listener);
    url
}

/// The message ids in bob's `Pending`, read with bob's own command.
fn pending_of_bob(world: &World) -> Vec<String> {
    ids(&world.client("bob", &["pending", "--json"]).ok().json())
}

/// A request alice posts while the service is down is kept (transport failure, exit 3). After the
/// service is up again as a new start and bob's next command opened his inbox again, alice's next
/// commands do not post it: it records no epoch, and `Outbox::due` reposts only an envelope
/// recorded at another epoch. The pages promise that the sender's next command posts its kept
/// envelopes again by itself (`website/docs/limits.md`, "One backend, in memory").
#[test]
fn a_request_posted_while_the_service_was_down_arrives_after_the_restart_by_itself() {
    let mut world = World::new("adv-reconnect-down");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    let down = down_url();
    let failed = world.client_env(
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
            "posted while the service was down",
        ],
        &[("INBOX_URL", down.as_str())],
    );
    assert_eq!(failed.status, TRANSPORT, "{failed:#?}");
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 1, "the request is kept: {failed:#?}");

    world.restart();
    world.client("bob", &["pending"]).ok();
    world.client("alice", &["outbox"]).ok();
    world.client("alice", &["outbox"]).ok();
    assert_eq!(
        pending_of_bob(&world),
        kept,
        "after the restart, bob's command and two commands of alice, the kept request is in \
         bob's Pending once"
    );
}

/// After a restart alice posts a request before bob's command opened his inbox again: the new
/// start refuses it `UnknownInbox`, which keeps it and records no epoch. Bob's next command opens
/// his inbox again; alice's next commands never post the request, so it does not arrive without
/// a typed `inbox repost` (the story's Context: refused `unknown-inbox`, it "arrives on a later
/// command").
#[test]
fn a_post_refused_unknown_inbox_by_the_new_start_arrives_once_the_owner_opened_again() {
    let mut world = World::new("adv-reconnect-early-post");
    world.client("bob", &["open", "--adapter", "slack"]).ok();
    world.restart();
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "too early"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert!(refused.stderr.contains("UnknownInbox"), "{refused:#?}");
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 1, "the note is kept: {refused:#?}");

    let owner = world.client("bob", &["pending", "--json"]).ok();
    assert!(
        owner.json().member("reconnected").is_some(),
        "bob's command opened his inbox again: {owner:#?}"
    );
    world.client("alice", &["outbox"]).ok();
    world.client("alice", &["outbox"]).ok();
    assert_eq!(
        pending_of_bob(&world),
        kept,
        "two commands of alice after bob opened again: the kept note is in Pending once"
    );
}

/// `inbox close` whose removal of the record fails (the file beside it could not be written:
/// here a directory where the staged file goes, removed again right after, as a full disk that
/// was freed): the command fails before its request, so the service keeps the inbox open and the
/// record stays. The close typed again closes it; after the next restart bob's next command does
/// not open the closed inbox again, and a post to it is refused. The help of `inbox open` says
/// the reconnect opens it "until `inbox close`".
// Round 2 rewrote the steps: `close` now removes the record before it sends `CloseInbox` (F2),
// so a record that cannot be removed fails the command with nothing sent, instead of a close the
// service did whose record stayed; what the case shows is kept.
#[test]
fn a_close_whose_record_could_not_be_removed_does_not_open_the_inbox_after_a_restart() {
    let mut world = World::new("adv-reconnect-close-write");
    world.client("bob", &["open"]).ok();
    let outbox = world.outbox("bob");
    let record = fs::read_to_string(beside(&outbox, ".opened")).expect("the record");
    let staged = beside(&outbox, ".opened.new");
    fs::create_dir(&staged).expect("a directory where the staged file goes");
    let closed = world.client("bob", &["close"]);
    fs::remove_dir(&staged).expect("the directory is removed");
    assert_eq!(closed.status, LOCAL, "{closed:#?}");
    assert_eq!(closed.stdout, "", "nothing was sent: {closed:#?}");
    assert_eq!(
        fs::read_to_string(beside(&outbox, ".opened")).expect("the record"),
        record,
        "the record stays: {closed:#?}"
    );
    world
        .client("alice", &["post", "--to", BOB, "--body", "still open"])
        .ok();
    let closed = world.client("bob", &["close"]).ok();
    assert!(closed.stdout.contains("closed"), "{closed:#?}");

    world.restart();
    let later = world.client("bob", &["pending", "--json"]).ok();
    assert!(
        later.json().member("reconnected").is_none(),
        "bob's command after the restart opened the inbox he closed: {later:#?}"
    );
    let post = world.client("alice", &["post", "--to", BOB, "--body", "is bob there?"]);
    assert_eq!(
        post.status, REFUSED,
        "the closed inbox takes a post: {post:#?}"
    );
}

/// A FIFO where the file of the opened inbox goes: reading it never fails a command (the module
/// documentation of `src/outbox.rs`), yet every client command whose answer names an epoch opens
/// it for reading in `Outbox::due` and blocks there for ever. The case waits 15 s, which only
/// ends the hang, and kills the command.
#[test]
fn a_fifo_in_place_of_the_opened_file_does_not_hang_a_command() {
    let world = World::new("adv-reconnect-fifo");
    world.client("bob", &["open"]).ok();
    let opened = beside(&world.outbox("bob"), ".opened");
    fs::remove_file(&opened).expect("the record is removed");
    let made = std::process::Command::new("mkfifo")
        .arg(&opened)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "{made:?}");

    let mut command = world.spawn("bob", &["pending"]);
    let started = std::time::Instant::now();
    while command.running() && started.elapsed() < HANG {
        std::thread::sleep(Duration::from_millis(50));
    }
    let hung = command.running();
    drop(command);
    assert!(
        !hung,
        "`inbox pending` still ran {} s later, blocked opening the FIFO {}",
        HANG.as_secs(),
        opened.display()
    );
}

/// A stand-in that answers the first request (`pending`) with an empty view from a start the
/// client has not recorded, then reads the reconnect's open and holds it without an answer until
/// `printed` is set or 15 s pass, then sets `released` and drops it. Answers its URL and whether
/// the second request was an open.
fn stalling_stand_in(
    printed: Arc<AtomicBool>,
    released: Arc<AtomicBool>,
) -> (String, std::thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let thread = std::thread::spawn(move || {
        let read_head = |stream: &mut std::net::TcpStream| {
            let _ = stream.set_read_timeout(Some(support::STAND_IN_WAIT));
            let mut received = Vec::new();
            let mut buffer = [0_u8; 4096];
            while !received.windows(4).any(|window| window == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => received.extend_from_slice(&buffer[..read]),
                }
            }
            String::from_utf8_lossy(&received).into_owned()
        };
        let Some(mut first) = support::accept_within(&listener, support::STAND_IN_WAIT) else {
            return false;
        };
        let _ = read_head(&mut first);
        let body = r#"{"rows":[]}"#;
        let answer = format!(
            "HTTP/1.1 200 Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Inbox-Epoch: {:032x}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
            0x0dd_5eed_u64
        );
        let _ = first.write_all(answer.as_bytes());
        drop(first);
        let Some(mut second) = support::accept_within(&listener, support::STAND_IN_WAIT) else {
            return false;
        };
        let head = read_head(&mut second);
        let is_open = head.starts_with("POST /messaging/commands/open-inbox ");
        let started = std::time::Instant::now();
        while !printed.load(Ordering::SeqCst) && started.elapsed() < HANG {
            std::thread::sleep(Duration::from_millis(20));
        }
        released.store(true, Ordering::SeqCst);
        drop(second);
        is_open
    });
    (url, thread)
}

/// The story's default 2 A: the command's own request first, "so a typed command answers as
/// before and reports the reconnect after it". Bob's `inbox pending` meets a new start whose
/// answer to the reconnect's open stalls: the command's own answer (`no messages`) is printed
/// only after the open ends (here when the stand-in gives up after 15 s; against a service that
/// does not answer, after the client's 30 s answer timeout, and once more per kept envelope).
#[test]
fn the_typed_command_prints_its_own_answer_before_a_stalled_reconnect_ends() {
    let world = World::new("adv-reconnect-stall");
    world.client("bob", &["open"]).ok();
    let printed = Arc::new(AtomicBool::new(false));
    let released = Arc::new(AtomicBool::new(false));
    let (url, stand_in) = stalling_stand_in(Arc::clone(&printed), Arc::clone(&released));

    let mut child = world
        .client_command("bob", &["pending"])
        .env("INBOX_URL", &url)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the binary starts");
    let mut line = String::new();
    let stdout = child.stdout.take().expect("stdout is piped");
    let _ = BufReader::new(stdout).read_line(&mut line);
    let before_release = !released.load(Ordering::SeqCst);
    printed.store(true, Ordering::SeqCst);
    let held_an_open = stand_in.join().expect("the stand-in ends");
    let _ = child.kill();
    let _ = child.wait();
    assert!(held_an_open, "the reconnect sent an open after the command");
    assert_eq!(line.trim(), "no messages");
    assert!(
        before_release,
        "the command's own answer was printed only after the reconnect's open ended"
    );
}
