//! Adversary cases of story:repost-does-not-redeliver-a-handled-note (stage 2, wave 8, pass 2):
//! the correction a9bd783, which records the epochs of `inbox repost` once after every post.
//! Every key is made at run time with the binary itself. The proxy listens on 127.0.0.1 only;
//! its thread is never joined and each case waits on it with a bound, so a changed client that
//! sends fewer requests fails the case instead of hanging it.

use crate::support;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use inbox::server::json::Value;

use support::{BOB, LOCAL, REFUSED, TRANSPORT, World, ids};

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

fn post_note(world: &World, body: &str) -> String {
    world
        .client("alice", &["post", "--to", BOB, "--body", body])
        .ok()
        .stdout
        .trim()
        .to_owned()
}

/// What the proxy does with one connection.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// Hands the request to the service and its answer back.
    Forward,
    /// Reads the request and closes the connection without an answer.
    Close,
    /// Reads the request and keeps the connection open without an answer for up to 20 s.
    Hold,
}

/// Reads one request: the head, then as many bytes as its `Content-Length` says.
fn read_request(stream: &mut TcpStream) -> Vec<u8> {
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
    received
}

/// A proxy on 127.0.0.1:0 in front of the service at `service`: one connection after another,
/// each handled by the next of `steps`. The request line of each is sent on the channel once
/// the step is done (for `Forward`, once the answer was handed back). The thread is never joined;
/// it ends when no connection comes within 10 s.
fn proxy(service: &str, steps: Vec<Step>) -> (String, mpsc::Receiver<String>) {
    let (url, receiver, _thread) = proxy_within(service, steps, support::STAND_IN_WAIT);
    (url, receiver)
}

/// [`proxy`] with the wait for each connection given, and the handle of its thread: the case that
/// holds the bound passes 1 s and checks that the thread ends. The thread ends with the message
/// of the connection that did not come, naming the wait, or with none.
fn proxy_within(
    service: &str,
    steps: Vec<Step>,
    wait: Duration,
) -> (
    String,
    mpsc::Receiver<String>,
    std::thread::JoinHandle<Option<String>>,
) {
    let upstream = service
        .trim_end_matches('/')
        .strip_prefix("http://")
        .expect("the service URL is http://<address>")
        .to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").expect("the proxy binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let (sender, receiver) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let total = steps.len();
        for (n, step) in steps.into_iter().enumerate() {
            let what = format!("request {} of {total}", n + 1);
            let mut stream =
                match support::accept_or_say(&listener, wait, "the proxy's thread", &what) {
                    Ok(stream) => stream,
                    Err(message) => return Some(message),
                };
            let request = read_request(&mut stream);
            let line = String::from_utf8_lossy(&request)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned();
            match step {
                Step::Forward => {
                    if let Ok(mut service) = TcpStream::connect(&upstream) {
                        let _ = service.set_read_timeout(Some(Duration::from_secs(10)));
                        let _ = service.write_all(&request);
                        let mut answer = Vec::new();
                        let _ = service.read_to_end(&mut answer);
                        let _ = stream.write_all(&answer);
                    }
                    drop(stream);
                    let _ = sender.send(line);
                }
                Step::Close => {
                    drop(stream);
                    let _ = sender.send(line);
                }
                Step::Hold => {
                    let _ = sender.send(line);
                    std::thread::sleep(Duration::from_secs(20));
                    drop(stream);
                }
            }
        }
        None
    });
    (url, receiver, thread)
}

/// `count` request lines from the proxy, waiting up to 10 s for each; the case fails, naming the
/// missing request, when one does not come.
fn requests(receiver: &mpsc::Receiver<String>, count: usize) -> Vec<String> {
    requests_within(receiver, count, support::STAND_IN_WAIT)
}

/// [`requests`] with the wait for each line given.
fn requests_within(receiver: &mpsc::Receiver<String>, count: usize, wait: Duration) -> Vec<String> {
    let mut lines = Vec::new();
    while lines.len() < count {
        match receiver.recv_timeout(wait) {
            Ok(line) => lines.push(line),
            Err(_) => panic!(
                "the proxy received no connection for request {} of {count} within {} s; read {lines:?}",
                lines.len() + 1,
                wait.as_secs()
            ),
        }
    }
    lines
}

/// After a restart alice holds 2 kept notes for bob, both recorded at the start before it. A
/// `repost` posts the first (the running service stores it) and is ended by SIGINT while the post
/// of the second waits for its answer. Bob reads the first note. The next `repost` must leave it
/// alone and deliver only the second: the client of e8266d5 recorded the epoch of the first
/// answer before it posted the second; a9bd783 records it only after the loop, so a run ended in
/// between records nothing and bob gets the note he read a second time.
#[test]
fn a_repost_ended_after_a_post_arrived_leaves_that_note_alone_once_its_recipient_read_it() {
    let mut world = World::new("adv2-w8-ended");
    world.client("bob", &["open"]).ok();
    let first = post_note(&world, "first");
    let second = post_note(&world, "second");
    world.restart();
    world.client("bob", &["open"]).ok();

    let (url, served) = proxy(
        &world.service.url,
        vec![Step::Forward, Step::Forward, Step::Hold],
    );
    let mut command = world.client_command("alice", &["repost", "--json"]);
    command
        .env("INBOX_URL", &url)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = command.spawn().expect("the binary starts");
    let lines = requests(&served, 3);
    let _ = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status();
    let status = child.wait().expect("repost is waited for");
    assert_eq!(
        lines.len(),
        3,
        "the question and the 2 posts reached the proxy: {lines:?}"
    );
    assert!(
        lines[1].starts_with("POST ") && lines[2].starts_with("POST "),
        "{lines:?}"
    );
    assert_eq!(
        status.code(),
        None,
        "repost was ended by SIGINT: {status:?}"
    );

    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&first),
        "the first post arrived"
    );
    world.client("bob", &["read", &first]).ok();

    let again = world.client("alice", &["repost", "--json"]);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&second),
        "bob read {first} after the ended repost delivered it, and the next repost delivered it \
         again: {again:#?}; epochs file: {:?}",
        fs::read_to_string(epochs_file(&world.outbox("alice")))
    );
}

/// The same run ended by a transport failure of the second post instead: the epoch of the first
/// answer is recorded before `repost` exits 3, so after bob read the first note the next `repost`
/// delivers only the second.
#[test]
fn a_repost_ended_by_a_transport_failure_records_the_epochs_of_the_posts_that_arrived() {
    let mut world = World::new("adv2-w8-transport");
    world.client("bob", &["open"]).ok();
    let first = post_note(&world, "first");
    let second = post_note(&world, "second");
    world.restart();
    world.client("bob", &["open"]).ok();

    let (url, served) = proxy(
        &world.service.url,
        vec![Step::Forward, Step::Forward, Step::Close],
    );
    let run = world.client_env(
        "alice",
        &["repost", "--json"],
        &[("INBOX_URL", url.as_str())],
    );
    let lines = requests(&served, 3);
    assert_eq!(run.status, TRANSPORT, "{run:#?}");
    assert_eq!(lines.len(), 3, "{lines:?}");

    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&first));
    world.client("bob", &["read", &first]).ok();

    let again = world.client("alice", &["repost", "--json"]);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&second),
        "the epoch of {first} was not recorded before the transport failure ended repost: \
         {again:#?}"
    );
}

/// A `repost` that cannot write the epochs file posts every envelope, prints its report with
/// each of them, names the refusal on standard error, and exits 4: the local failure decides the
/// exit over the refusal of the third envelope.
#[test]
fn a_repost_that_cannot_write_the_epochs_file_reports_every_post_and_exits_4_over_a_refusal() {
    let mut world = World::new("adv2-w8-local");
    world.client("bob", &["open"]).ok();
    let first = post_note(&world, "first");
    let second = post_note(&world, "second");
    let refused = world.client("alice", &["post", "--to", CAROL, "--body", "for carol"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let kept = world.kept("alice");
    assert_eq!(kept.len(), 3, "{kept:?}");
    let for_carol = kept[2].clone();
    world.restart();
    world.client("bob", &["open"]).ok();

    let mut staged = epochs_file(&world.outbox("alice")).into_os_string();
    staged.push(".new");
    fs::create_dir(&staged).expect("a directory where the staged epochs file goes");
    let run = world.client("alice", &["repost", "--json"]);
    assert_eq!(run.status, LOCAL, "{run:#?}");
    let document = run.json();
    assert_eq!(
        document.member("error"),
        Some(&Value::Text("local".to_owned())),
        "{run:#?}"
    );
    let entries = reposted(document.member("done").expect("the report is printed"));
    let arrived: Vec<(String, Option<Value>)> = entries
        .iter()
        .map(|entry| {
            (
                support::text(entry, "message_id"),
                entry.member("arrived").cloned(),
            )
        })
        .collect();
    assert_eq!(
        arrived,
        vec![
            (first.clone(), Some(Value::Bool(true))),
            (second.clone(), Some(Value::Bool(true))),
            (for_carol.clone(), Some(Value::Bool(false))),
        ],
        "{run:#?}"
    );
    assert!(run.stderr.contains(&for_carol), "{run:#?}");
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    let mut held = ids(&pending);
    held.sort();
    let mut expected = vec![first, second];
    expected.sort();
    assert_eq!(held, expected);
}

/// A proxy whose client never connects ends its thread, and fails the case that reads its
/// requests with a message that names the missing request. The wait is 1 s here and 10 s for
/// every other case; the thread's message names the wait it kept, so a proxy that waits longer
/// than it was told fails the case. The case waits for the thread with `STAND_IN_WAIT` only as
/// the end of a hang (it held the thread's end to 3 s before, a time a slow host can pass:
/// story:no-test-holds-a-wall-clock-time).
#[test]
#[should_panic(expected = "the proxy received no connection for request 1 of 1")]
fn a_proxy_whose_client_never_connects_ends_its_thread_and_fails_the_case() {
    let (_url, served, thread) = proxy_within(
        "http://127.0.0.1:9",
        vec![Step::Close],
        Duration::from_secs(1),
    );
    support::wait_until_finished(&thread, "the proxy's thread");
    let said = thread
        .join()
        .expect("the proxy's thread ends without a panic");
    assert_eq!(
        said.as_deref(),
        Some("the proxy's thread received no connection for request 1 of 1 within 1 s"),
        "what the proxy's thread said"
    );
    let _ = requests_within(&served, 1, Duration::from_secs(1));
}
