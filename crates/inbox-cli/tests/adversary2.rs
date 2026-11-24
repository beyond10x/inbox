//! Adversary cases of story:cli-client (stage 2, wave 4, pass 2: the correction e411f79). Every
//! key is made at run time with the binary itself; no assertion message prints a secret key
//! file's content.

use crate::support;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant as Clock};

use inbox::server::json::{self, Value};

use support::{BOB, LOCAL, TRANSPORT, ids, key_new, keys_only, run};

/// A child process killed and waited for when dropped, so no `inbox` outlives a case.
struct Reaped(Option<Child>);

impl Reaped {
    fn child(&mut self) -> &mut Child {
        self.0.as_mut().expect("the child is held")
    }

    /// Waits up to `limit` for the exit; `None` when it is still running then.
    fn exit_within(&mut self, limit: Duration) -> Option<std::process::ExitStatus> {
        let started = Clock::now();
        while started.elapsed() < limit {
            if let Ok(Some(status)) = self.child().try_wait() {
                return Some(status);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }
}

impl Drop for Reaped {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn outbox_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

fn message_id_of(line: &str) -> String {
    match json::parse(line)
        .ok()
        .and_then(|value| value.member("message_id").cloned())
    {
        Some(Value::Text(id)) => id,
        other => panic!("no message_id in the outbox line: {other:?}"),
    }
}

// --- 400 and 413 are not final for an envelope the decoder admitted -----------------------------

/// The claim (fix-1, F5): a `413` is final for the envelope's text. The service answers `413`
/// for a body over `--max-request-bytes` of *this run* of the service (`service.md`, step 2): it
/// says nothing about the envelope. An envelope kept after a transport failure and reposted to a
/// service run with a lower limit is dropped, and the repost to the same service restarted with
/// its default limit has nothing left to deliver.
#[test]
fn a_413_of_a_lowered_request_limit_does_not_drop_an_envelope_a_later_repost_delivers() {
    let (dir, operator) = keys_only("adv2-413-limit");
    let outbox_path = dir.join("alice.outbox");
    let outbox = outbox_path.display().to_string();
    let body = "b".repeat(3000);

    // The service is down: the post fails in transport and the envelope is kept for repost.
    let closed_port = support::ClosedPort::new();
    let closed = closed_port.url();
    let posted = run(
        &dir.path,
        &[
            "post",
            "--secret-key-file",
            "alice.key",
            "--to",
            BOB,
            "--body",
            &body,
        ],
        &[
            ("INBOX_URL", closed.as_str()),
            ("INBOX_OUTBOX", outbox.as_str()),
        ],
    );
    assert_eq!(posted.status, TRANSPORT, "{posted:#?}");
    let kept = outbox_lines(&outbox_path);
    assert_eq!(kept.len(), 1, "the envelope is kept");
    let message_id = message_id_of(&kept[0]);

    // The operator starts it with a lower request limit; alice reposts.
    let low = support::Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--max-request-bytes",
            "1024",
        ],
        &[],
    );
    let low_env = [
        ("INBOX_URL", low.url.as_str()),
        ("INBOX_OUTBOX", outbox.as_str()),
    ];
    run(
        &dir.path,
        &["open", "--secret-key-file", "bob.key"],
        &low_env,
    )
    .ok();
    let first = run(
        &dir.path,
        &["repost", "--secret-key-file", "alice.key", "--json"],
        &low_env,
    );
    let after_first = outbox_lines(&outbox_path).len();
    let status = low.terminate();
    assert!(status.success(), "{status:?}");

    // Restarted with the default limit: the same envelope is deliverable.
    let full = support::serve(&dir.path, &operator);
    let full_env = [
        ("INBOX_URL", full.url.as_str()),
        ("INBOX_OUTBOX", outbox.as_str()),
    ];
    run(
        &dir.path,
        &["open", "--secret-key-file", "bob.key"],
        &full_env,
    )
    .ok();
    let second = run(
        &dir.path,
        &["repost", "--secret-key-file", "alice.key", "--json"],
        &full_env,
    );
    let pending = run(
        &dir.path,
        &["pending", "--secret-key-file", "bob.key", "--json"],
        &full_env,
    )
    .ok()
    .json();
    assert!(
        ids(&pending).contains(&message_id),
        "the envelope {message_id} was dropped on a 413 of a lowered limit and never delivered: \
         repost to --max-request-bytes 1024 exited {} ({}), outbox lines after it: {after_first}; \
         repost to the default service exited {} ({}); bob's pending: {:?}",
        first.status,
        first.stderr.trim(),
        second.status,
        second.stdout.trim(),
        ids(&pending)
    );
}

/// A relay on 127.0.0.1:0 in front of `target`: it reads one request from the client in full,
/// passes its head on, waits `stall`, passes the body on, then passes the service's answer back.
/// It answers the message id of the body it relayed.
fn stalling_relay(
    target: SocketAddr,
    stall: Duration,
) -> (String, std::thread::JoinHandle<String>) {
    stalling_relay_within(target, stall, support::STAND_IN_WAIT)
}

/// [`stalling_relay`] with the wait for the request given: the case that holds the bound passes
/// 1 s.
fn stalling_relay_within(
    target: SocketAddr,
    stall: Duration,
    wait: Duration,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the relay binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let handle = std::thread::spawn(move || {
        let mut client = support::accept_or_panic(
            &listener,
            wait,
            "the relay of adversary2.rs",
            "the request of `post`",
        );
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a read timeout");
        let mut received = Vec::new();
        let mut buffer = [0_u8; 4096];
        let mut expected = None;
        let mut head_end = 0;
        loop {
            if expected.is_some_and(|total| received.len() >= total) {
                break;
            }
            let read = client.read(&mut buffer).expect("the request is read");
            assert!(read > 0, "the request ended early");
            received.extend_from_slice(&buffer[..read]);
            let at = received.windows(4).position(|window| window == b"\r\n\r\n");
            if let (None, Some(at)) = (expected, at) {
                head_end = at + 4;
                let head = String::from_utf8_lossy(&received[..at]).to_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                expected = Some(head_end + length);
            }
        }
        let body = String::from_utf8_lossy(&received[head_end..]).into_owned();
        let message_id = json::parse(&body)
            .ok()
            .and_then(|value| match value.member("message_id") {
                Some(Value::Text(id)) => Some(id.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let mut service = TcpStream::connect(target).expect("the service accepts");
        service
            .set_read_timeout(Some(Duration::from_secs(20)))
            .expect("a read timeout");
        service
            .write_all(&received[..head_end])
            .expect("the head is passed on");
        std::thread::sleep(stall);
        let _ = service.write_all(&received[head_end..]);
        let mut answer = Vec::new();
        let _ = service.read_to_end(&mut answer);
        let _ = client.write_all(&answer);
        message_id
    });
    (url, handle)
}

/// The claim (fix-1, F5): a `400` is final for the envelope's text. The service also answers
/// `400` when the bytes of a request stop arriving for more than a second between its head and
/// its body (the generated reader, step 1: "the body was shorter than `Content-Length`
/// announced"), which says nothing about the envelope. The client writes the head and the body in
/// two writes; a client stalled between them (a loaded machine, a stopped process) has its
/// envelope dropped, and a repost to the same service has nothing left to deliver.
#[test]
fn a_400_for_a_request_whose_bytes_stalled_does_not_drop_the_envelope() {
    let world = support::World::new("adv2-400-stall");
    world.client("bob", &["open"]).ok();
    let target: SocketAddr = world
        .service
        .url
        .trim_start_matches("http://")
        .parse()
        .expect("the service address");
    let (relay, relayed) = stalling_relay(target, Duration::from_millis(1600));
    let posted = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "a fine envelope", "--json"],
        &[("INBOX_URL", relay.as_str())],
    );
    let message_id = relayed.join().expect("the relay ends");
    assert!(!message_id.is_empty(), "the relay saw a post");
    let kept_after_post = world.kept("alice");
    let reposted = world.client("alice", &["repost", "--json"]);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert!(
        ids(&pending).contains(&message_id),
        "the envelope {message_id} was dropped on the service's 400 for a stalled request and \
         never delivered: post exited {} with {}; outbox after it: {kept_after_post:?}; repost \
         exited {} ({}); bob's pending: {:?}",
        posted.status,
        posted.stdout.trim(),
        reposted.status,
        reposted.stdout.trim(),
        ids(&pending)
    );
}

// --- The key file commands -----------------------------------------------------------------------

/// The correction rewrote 2 messages of `keyfile add-key` and lost their line continuation: the
/// text an operator reads holds a run of spaces in the middle of a sentence.
#[test]
fn the_key_file_refusals_read_as_one_sentence_without_a_run_of_spaces() {
    let (dir, _operator) = keys_only("adv2-messages");
    let carol = key_new(
        &dir.path,
        "carol.key",
        Some(("carol-1", "agent:test/carol")),
    );
    let add = |operator: &str, key_id: &str| {
        run(
            &dir.path,
            &[
                "keyfile",
                "add-key",
                "--file",
                "keys.json",
                "--operator-secret-key-file",
                operator,
                "--key-id",
                key_id,
                "--principal",
                "agent:test/carol",
                "--public-key",
                &carol,
            ],
            &[],
        )
    };
    let reused = add("operator.key", "alice-1");
    assert_eq!(reused.status, LOCAL, "{reused:#?}");
    key_new(&dir.path, "other-operator.key", None);
    let mixed = add("other-operator.key", "carol-1");
    assert_eq!(mixed.status, LOCAL, "{mixed:#?}");
    for (what, run) in [("a reused id", &reused), ("another operator key", &mixed)] {
        assert!(
            !run.stderr.contains("  "),
            "{what}: the message holds a run of spaces: {:?}",
            run.stderr.trim()
        );
    }
}

// --- serve and the signal thread -----------------------------------------------------------------

/// The claim of the correction (`serve.rs`, the signal thread): "what is written after [the stop]
/// cannot keep the service from stopping". `serve` joins the signal thread before it returns, so a
/// write of the signal thread that blocks (a standard error that is a full pipe whose reader is
/// alive) keeps the process from ending after SIGTERM.
#[test]
fn sigterm_ends_serve_when_its_standard_error_is_a_full_pipe() {
    use std::os::fd::AsRawFd;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (dir, operator) = keys_only("adv2-serve-stderr-full");
    let (mut reader, writer) = std::io::pipe().expect("a pipe");
    let for_child = writer.try_clone().expect("a second write end");
    let filled = writer.as_raw_fd();
    assert!(
        !support::blocked_in_write(filled),
        "blocked_in_write answers true before any thread writes to the pipe"
    );
    let stop = Arc::new(AtomicBool::new(false));
    let filler = {
        let stop = Arc::clone(&stop);
        let mut writer = writer;
        std::thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                if writer.write(&[b'.'; 512]).is_err() {
                    break;
                }
            }
        })
    };
    // Full: the filler is blocked in write(2) on the pipe. Before, a pipe that took no byte for
    // 300 ms counted as full, and a filler that waited for a processor left it with room, so the
    // case passed without a full pipe (story:no-test-holds-a-wall-clock-time).
    support::wait_until_blocked_in_write(filled);
    let mut serve = Reaped(Some(
        support::command(&dir.path)
            .args([
                "serve",
                "--listen",
                "127.0.0.1:0",
                "--key-file",
                "keys.json",
                "--operator-public-key",
                &operator,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::from(for_child))
            .spawn()
            .expect("the binary starts"),
    ));
    let stdout = serve.child().stdout.take().expect("piped");
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("the first line");
    assert!(line.starts_with("http://127.0.0.1:"), "{line:?}");
    let pid = serve.child().id().to_string();
    let sent = Command::new("kill")
        .args(["-TERM", &pid])
        .status()
        .expect("kill runs");
    assert!(sent.success());
    let status = serve.exit_within(Duration::from_secs(10));
    drop(serve);
    stop.store(true, Ordering::SeqCst);
    let drain = std::thread::spawn(move || {
        let mut all = Vec::new();
        let _ = reader.read_to_end(&mut all);
    });
    let _ = filler.join();
    let _ = drain.join();
    assert!(
        status.is_some_and(|status| status.code() == Some(0)),
        "inbox serve after SIGTERM with a full standard error pipe: {status:?} (None: still \
         running 10 s later)"
    );
}

/// A relay whose client never connects ends with a message that names the missing request, and
/// does not wait for it forever: the case that joins it fails with that message. The wait is 1 s
/// here and 10 s for every other case.
#[test]
fn a_relay_whose_client_never_connects_ends_and_names_the_request() {
    let target: SocketAddr = "127.0.0.1:9".parse().expect("an address");
    let (_relay, relayed) =
        stalling_relay_within(target, Duration::from_millis(1), Duration::from_secs(1));
    let message = match relayed.join() {
        Ok(id) => panic!("the relay ended with {id:?} and no message"),
        Err(panic) => panic.downcast_ref::<String>().cloned().unwrap_or_default(),
    };
    assert!(
        message.contains(
            "the relay of adversary2.rs received no connection for the request of `post` within 1 s"
        ),
        "{message:?}"
    );
}
