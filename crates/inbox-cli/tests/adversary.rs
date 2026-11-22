//! Adversary cases of story:cli-client (stage 2, wave 4, pass 1). Every key is made at run time
//! with the binary itself; no assertion message prints a secret key file's content.

use crate::support;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant as Clock};

use inbox::server::json::{self, Value};
use inbox_verify::{Instant, KeyRing, PublicKey};

use support::{ALICE, BOB, Dir, LOCAL, REFUSED, World, ids, key_new, keys_only, run, text};

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

/// Gives a directory back its owner's write bit when dropped, so the test directory can go.
struct Writable(PathBuf);

impl Drop for Writable {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
    }
}

fn member<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .member(name)
        .unwrap_or_else(|| panic!("no `{name}` in {value:?}"))
}

/// A service stand-in on 127.0.0.1:0 that answers one request with `answer` and reports whether
/// the outbox file at `outbox` held the posted message id at the moment the request arrived.
fn stand_in(answer: &'static str, outbox: PathBuf) -> (String, std::thread::JoinHandle<bool>) {
    stand_in_within(answer, outbox, support::STAND_IN_WAIT)
}

/// [`stand_in`] with the wait for the request given: the case that holds the bound passes 1 s.
fn stand_in_within(
    answer: &'static str,
    outbox: PathBuf,
    wait: Duration,
) -> (String, std::thread::JoinHandle<bool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let handle = std::thread::spawn(move || {
        let mut stream = support::accept_or_panic(
            &listener,
            wait,
            "the stand-in of adversary.rs",
            "the request of `post`",
        );
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a read timeout");
        let mut received = Vec::new();
        let mut buffer = [0_u8; 4096];
        let head_end = loop {
            let read = stream.read(&mut buffer).expect("the request is read");
            assert!(read > 0, "the request ended before its head did");
            received.extend_from_slice(&buffer[..read]);
            if let Some(at) = received.windows(4).position(|window| window == b"\r\n\r\n") {
                break at + 4;
            }
        };
        let head = String::from_utf8_lossy(&received[..head_end]).to_lowercase();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0);
        while received.len() < head_end + length {
            let read = stream.read(&mut buffer).expect("the body is read");
            assert!(read > 0, "the request ended before its body did");
            received.extend_from_slice(&buffer[..read]);
        }
        let body = String::from_utf8_lossy(&received[head_end..head_end + length]).into_owned();
        let message_id = json::parse(&body)
            .ok()
            .and_then(|value| match value.member("message_id") {
                Some(Value::Text(id)) => Some(id.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let kept = !message_id.is_empty()
            && fs::read_to_string(&outbox)
                .unwrap_or_default()
                .contains(&message_id);
        stream
            .write_all(answer.as_bytes())
            .expect("the answer is written");
        kept
    });
    (url, handle)
}

/// A client key for alice in `dir`, and the environment of a client command against `url`.
fn alice_against(dir: &Path, url: &str) -> Vec<(String, String)> {
    key_new(dir, "alice.key", Some(("alice-1", ALICE)));
    vec![
        ("INBOX_URL".to_owned(), url.to_owned()),
        ("INBOX_SECRET_KEY_FILE".to_owned(), "alice.key".to_owned()),
        (
            "INBOX_OUTBOX".to_owned(),
            dir.join("alice.outbox").display().to_string(),
        ),
    ]
}

fn run_env(dir: &Path, args: &[&str], env: &[(String, String)]) -> support::Run {
    let env: Vec<(&str, &str)> = env
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    run(dir, args, &env)
}

// --- The outbox and repost -------------------------------------------------------------------

/// A note the recipient read is handled and gone from the service; `inbox repost` must not put it
/// in the recipient's `pending` a second time.
#[test]
fn repost_does_not_deliver_again_a_note_its_recipient_already_read() {
    let world = World::new("adv-note-twice");
    world.client("bob", &["open"]).ok();
    let message_id = world
        .client("alice", &["post", "--to", BOB, "--body", "once only"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let read = world
        .client("bob", &["read", &message_id, "--json"])
        .ok()
        .json();
    assert_eq!(text(&read, "outcome"), "handled");
    let before = world.client("bob", &["pending", "--json"]).ok().json();
    assert!(ids(&before).is_empty(), "the note is handled");

    let reposted = world.client("alice", &["repost", "--json"]);
    let after = world.client("bob", &["pending", "--json"]).ok().json();
    assert!(
        ids(&after).is_empty(),
        "repost delivered the read note {message_id} again: repost said {}; bob's pending now \
         holds {:?}",
        reposted.stdout.trim(),
        ids(&after)
    );
}

/// `repost` exits 1 when an envelope did not arrive (an inbox not open answers `UnknownInbox`),
/// names the error, reports `arrived: false`, and keeps the envelope.
#[test]
fn repost_exits_1_when_an_envelope_did_not_arrive() {
    let world = World::new("adv-repost-refused");
    let posted = world.client("alice", &["post", "--to", BOB, "--body", "too early"]);
    assert_eq!(posted.status, REFUSED, "{posted:#?}");
    assert_eq!(world.kept("alice").len(), 1);

    let reposted = world.client("alice", &["repost", "--json"]);
    assert_eq!(reposted.status, REFUSED, "{reposted:#?}");
    let document = reposted.json();
    assert_eq!(text(&document, "error"), "inbox.messaging.UnknownInbox");
    let entries = match member(member(&document, "answer"), "reposted") {
        Value::Array(entries) => entries.clone(),
        other => panic!("not an array: {other:?}"),
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(false)));
    assert_eq!(
        world.kept("alice").len(),
        1,
        "UnknownInbox keeps the envelope"
    );
}

/// `post` keeps the envelope in the outbox file before it sends it: the file holds the message id
/// at the moment the request reaches the service.
#[test]
fn post_keeps_the_envelope_before_the_request_reaches_the_service() {
    let dir = Dir::new("adv-keep-first");
    let (url, seen) = stand_in(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 36\r\n\
         Connection: close\r\n\r\n{\"outcome\":\"posted\",\"published\":[]}",
        dir.join("alice.outbox"),
    );
    let env = alice_against(&dir.path, &url);
    let posted = run_env(&dir.path, &["post", "--to", BOB, "--body", "kept?"], &env).ok();
    assert!(
        seen.join().expect("the stand-in ends"),
        "the outbox file did not hold {} when the post arrived",
        posted.stdout.trim()
    );
}

/// A 3xx answer is not the service's `posted`: `post` must not exit 0 and print a message id as
/// if the message had arrived.
#[test]
fn a_redirect_answer_is_not_reported_as_posted() {
    let dir = Dir::new("adv-redirect");
    let (url, _seen) = stand_in(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: http://127.0.0.1:1/\r\n\
         Content-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        dir.join("alice.outbox"),
    );
    let env = alice_against(&dir.path, &url);
    let posted = run_env(
        &dir.path,
        &["post", "--to", BOB, "--body", "moved", "--json"],
        &env,
    );
    assert_ne!(
        posted.status, 0,
        "a 301 was reported as a successful post: {posted:#?}"
    );
}

/// The claim: an entry leaves the outbox file when its `expires` is at or before now. An envelope
/// whose body is over the model's 65536 bytes is refused `400` by the service (kept, as a service
/// refusal) and does not read back through the generated decoder, so it must still leave once its
/// `expires` has passed.
#[test]
fn an_envelope_over_the_body_limit_leaves_the_outbox_when_it_expires() {
    let world = World::new("adv-big-body");
    world.client("bob", &["open"]).ok();
    fs::write(world.dir.join("big.txt"), "a".repeat(65_537)).expect("the body file");
    let expires = inbox_server::format_instant(
        Instant::from_system_time(std::time::SystemTime::now()).plus_seconds(2),
    );
    let posted = world.client(
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--body-file",
            "big.txt",
            "--expires",
            &expires,
            "--json",
        ],
    );
    assert_eq!(posted.status, support::USAGE, "{posted:#?}");
    assert!(
        world.kept("alice").is_empty(),
        "what the decoder refuses is never kept"
    );
    std::thread::sleep(Duration::from_secs(4));
    let reposted = world.client("alice", &["repost", "--json"]);
    assert!(
        world.kept("alice").is_empty(),
        "expires {expires} has passed and the entry is still in the outbox file; post: exit {} \
         {}; repost: exit {} {}",
        posted.status,
        text(&posted.json(), "error"),
        reposted.status,
        reposted.stdout.trim()
    );
}

/// `repost` drops an envelope the service refuses with a declared error other than
/// `UnknownInbox` (here `SignatureInvalid`: a kept line whose body no longer matches its
/// signature) and exits 1 naming it.
#[test]
fn repost_drops_an_envelope_refused_with_a_declared_error() {
    let world = World::new("adv-repost-drop");
    let posted = world.client("alice", &["post", "--to", BOB, "--body", "tamper me"]);
    assert_eq!(posted.status, REFUSED, "{posted:#?}");
    let outbox = world.outbox("alice");
    let line = fs::read_to_string(&outbox).expect("the outbox file");
    assert!(line.contains("\"body\":\"tamper me\""), "{line}");
    fs::write(
        &outbox,
        line.replace("\"body\":\"tamper me\"", "\"body\":\"tampered!\""),
    )
    .expect("rewritten");
    world.client("bob", &["open"]).ok();
    let reposted = world.client("alice", &["repost", "--json"]);
    assert_eq!(reposted.status, REFUSED, "{reposted:#?}");
    assert_eq!(
        text(&reposted.json(), "error"),
        "inbox.messaging.SignatureInvalid"
    );
    assert!(
        world.kept("alice").is_empty(),
        "a declared refusal drops it"
    );
}

/// `post` without `--expires` writes `expires` = `created` plus 24 hours.
#[test]
fn post_writes_expires_24_hours_after_created() {
    let world = World::new("adv-lifetime");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "how long"])
        .ok();
    let line = fs::read_to_string(world.outbox("alice")).expect("the outbox file");
    let envelope = json::parse(line.trim()).expect("one envelope");
    let created = Instant::parse(&text(&envelope, "created")).expect("created");
    assert_eq!(
        text(&envelope, "expires"),
        inbox_server::format_instant(created.plus_seconds(24 * 3600))
    );
}

/// The outbox directory is created 0700 and the file 0600.
#[test]
fn the_outbox_directory_is_0700_and_its_file_0600() {
    let world = World::new("adv-outbox-modes");
    world.client("bob", &["open"]).ok();
    let outbox = world.dir.join("fresh/inbox/alice.outbox");
    let outbox_text = outbox.display().to_string();
    world
        .client_env(
            "alice",
            &["post", "--to", BOB, "--body", "modes"],
            &[("INBOX_OUTBOX", outbox_text.as_str())],
        )
        .ok();
    let mode = |path: &Path| fs::metadata(path).expect("exists").permissions().mode() & 0o777;
    assert_eq!(mode(&world.dir.join("fresh")), 0o700);
    assert_eq!(mode(&world.dir.join("fresh/inbox")), 0o700);
    assert_eq!(mode(&outbox), 0o600);
}

/// `inbox serve --max-messages 1`: the second post is the service's `503 message limit`, exit 1,
/// and the envelope is kept for `repost`.
#[test]
fn serve_takes_max_messages_and_a_503_keeps_the_envelope() {
    let (dir, operator) = keys_only("adv-max-messages");
    let service = support::Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--max-messages",
            "1",
        ],
        &[],
    );
    let outbox = dir.join("alice.outbox").display().to_string();
    let env = [
        ("INBOX_URL", service.url.as_str()),
        ("INBOX_OUTBOX", outbox.as_str()),
    ];
    run(&dir.path, &["open", "--secret-key-file", "bob.key"], &env).ok();
    let post = |body: &str| {
        run(
            &dir.path,
            &[
                "post",
                "--secret-key-file",
                "alice.key",
                "--to",
                BOB,
                "--body",
                body,
            ],
            &env,
        )
    };
    post("one").ok();
    let second = post("two");
    assert_eq!(second.status, REFUSED, "{second:#?}");
    assert!(second.stderr.contains("message limit"), "{second:#?}");
    assert_eq!(
        fs::read_to_string(dir.join("alice.outbox"))
            .expect("the outbox")
            .lines()
            .count(),
        2
    );
}

/// `inbox serve --max-wait 1`: a `wait --seconds 30` is cut to about a second.
#[test]
fn serve_takes_max_wait() {
    let (dir, operator) = keys_only("adv-max-wait");
    let service = support::Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--max-wait",
            "1",
        ],
        &[],
    );
    let env = [("INBOX_URL", service.url.as_str())];
    run(&dir.path, &["open", "--secret-key-file", "bob.key"], &env).ok();
    let started = Clock::now();
    run(
        &dir.path,
        &["wait", "--secret-key-file", "bob.key", "--seconds", "30"],
        &env,
    )
    .ok();
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "the wait took {:?}",
        started.elapsed()
    );
}

/// `collect` takes the answer off the service. When the outbox file cannot be written after that,
/// the answer must still reach the caller (or stay collectable); it must not be lost.
#[test]
fn collect_does_not_lose_the_answer_when_the_outbox_file_cannot_be_written() {
    let world = World::new("adv-collect-lost");
    let sub = world.dir.join("state-ro");
    fs::create_dir_all(&sub).expect("the outbox directory");
    let outbox = sub.join("alice.outbox").display().to_string();
    let alice_env = [("INBOX_OUTBOX", outbox.as_str())];
    world.client("bob", &["open"]).ok();
    let request = world
        .client_env(
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
                "Which colour?",
            ],
            &alice_env,
        )
        .ok()
        .stdout
        .trim()
        .to_owned();
    world
        .client("bob", &["answer", &request, "--body", "Blue"])
        .ok();

    let _writable = Writable(sub.clone());
    fs::set_permissions(&sub, fs::Permissions::from_mode(0o500)).expect("read-only");
    let collected = world.client_env("alice", &["collect", &request], &alice_env);
    let again = world.client_env("alice", &["outbox", "--json"], &alice_env);
    assert!(
        collected.stdout.contains("Blue") || again.stdout.contains("Blue"),
        "the answer is gone: collect exited {} with stdout {:?} and stderr {:?}; the service's \
         outbox view now: {}",
        collected.status,
        collected.stdout,
        collected.stderr.trim(),
        again.stdout.trim()
    );
}

// --- The key commands ------------------------------------------------------------------------

/// Two `keyfile add-key` at once on one key file lose no entry: every one that exits 0 is in the
/// file the ring reads.
#[test]
fn add_key_processes_at_once_lose_no_entry() {
    let dir = Dir::new("adv-add-key-race");
    let operator = key_new(&dir.path, "operator.key", None);
    let operator = PublicKey::from_base64(&operator).expect("the operator key");
    let mut public_keys = Vec::new();
    for n in 0..8 {
        let principal = format!("agent:test/c{n}");
        public_keys.push(key_new(
            &dir.path,
            &format!("c{n}.key"),
            Some((&format!("c{n}-1"), &principal)),
        ));
    }
    for round in 0..3 {
        let file = format!("keys-{round}.json");
        let children: Vec<(String, Reaped)> = public_keys
            .iter()
            .enumerate()
            .map(|(n, public_key)| {
                let key_id = format!("c{n}-{round}");
                let child = support::command(&dir.path)
                    .args([
                        "keyfile",
                        "add-key",
                        "--file",
                        &file,
                        "--operator-secret-key-file",
                        "operator.key",
                        "--key-id",
                        &key_id,
                        "--principal",
                        &format!("agent:test/c{n}"),
                        "--public-key",
                        public_key,
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("the binary starts");
                (key_id, Reaped(Some(child)))
            })
            .collect();
        let mut done = Vec::new();
        for (key_id, mut child) in children {
            let output = child
                .0
                .take()
                .expect("held")
                .wait_with_output()
                .expect("waited");
            done.push((
                key_id,
                output.status.code(),
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        let content = fs::read_to_string(dir.join(&file)).unwrap_or_default();
        let ring = KeyRing::parse(&content, &operator);
        let now = Instant::from_system_time(std::time::SystemTime::now());
        let lost: Vec<&String> = done
            .iter()
            .filter(|(_, status, _)| *status == Some(0))
            .map(|(key_id, _, _)| key_id)
            .filter(|key_id| {
                !ring
                    .as_ref()
                    .is_ok_and(|ring| ring.principal(key_id, now).is_ok())
            })
            .collect();
        assert!(
            lost.is_empty(),
            "round {round}: add-key exited 0 for {lost:?} and the key file does not hold them \
             (ring: {:?}); runs: {done:?}",
            ring.as_ref().map(|ring| ring.ignored().to_vec())
        );
    }
}

/// `keyfile add-key` rewrites the key file; the rewritten file keeps the mode the operator gave
/// the old one.
#[test]
fn add_key_keeps_the_mode_of_the_key_file() {
    let (dir, _operator) = keys_only("adv-keyfile-mode");
    let file = dir.join("keys.json");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o600)).expect("narrowed");
    let carol = key_new(
        &dir.path,
        "carol.key",
        Some(("carol-1", "agent:test/carol")),
    );
    support::add_key(
        &dir.path,
        "operator.key",
        "carol-1",
        "agent:test/carol",
        &carol,
    );
    let mode = fs::metadata(&file)
        .expect("the key file")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(
        mode, 0o600,
        "the key file was {mode:04o} after add-key, 0600 before"
    );
}

/// A secret key file whose `public_key` is not the public half of its seed is refused (exit 4).
#[test]
fn a_secret_key_file_whose_public_key_is_another_keys_is_refused() {
    let dir = Dir::new("adv-public-mismatch");
    key_new(&dir.path, "one.key", Some(("one-1", "agent:test/one")));
    let other = key_new(&dir.path, "two.key", Some(("two-1", "agent:test/two")));
    let one = json::parse(&fs::read_to_string(dir.join("one.key")).expect("one.key"))
        .expect("one.key is JSON");
    let swapped = match one {
        Value::Object(members) => Value::Object(
            members
                .into_iter()
                .map(|(name, value)| {
                    if name == "public_key" {
                        (name, Value::Text(other.clone()))
                    } else {
                        (name, value)
                    }
                })
                .collect(),
        ),
        _ => panic!("one.key is not an object"),
    };
    let mut out = String::new();
    inbox_cli_write(&swapped, &mut out);
    let path = dir.join("swapped.key");
    fs::write(&path, out).expect("written");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("0600");
    let refused = run(
        &dir.path,
        &[
            "pending",
            "--secret-key-file",
            "swapped.key",
            "--url",
            "http://127.0.0.1:1",
        ],
        &[],
    );
    assert_eq!(refused.status, LOCAL, "exit {}", refused.status);
    assert!(
        refused.stderr.contains("public half"),
        "exit {}, and stderr does not name the mismatch",
        refused.status
    );
}

/// JSON of a parsed value, without the binary's private writer.
fn inbox_cli_write(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(flag) => json::push_bool(out, *flag),
        Value::Number(spelling) => out.push_str(spelling),
        Value::Text(text) => json::push_text(out, text),
        Value::Array(items) => {
            out.push('[');
            for (at, item) in items.iter().enumerate() {
                if at > 0 {
                    out.push(',');
                }
                inbox_cli_write(item, out);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (name, member) in members {
                json::member(out, name);
                inbox_cli_write(member, out);
            }
            out.push('}');
        }
    }
}

// --- serve, the URL and --json ---------------------------------------------------------------

/// `inbox serve ... 2>&1 | head -n1`: the reader of standard error is gone after the first line.
/// SIGTERM must still stop the service with status 0.
#[test]
fn serve_stops_on_sigterm_after_the_reader_of_its_standard_error_is_gone() {
    let (dir, operator) = keys_only("adv-serve-stderr");
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
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary starts"),
    ));
    drop(serve.child().stderr.take());
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
    assert!(
        status.is_some_and(|status| status.code() == Some(0)),
        "inbox serve after SIGTERM, its stderr reader gone: {status:?} (None: still running \
         10 s later)"
    );
}

/// The claim: the signal handlers are installed before the first line is printed. `inbox serve`
/// is held inside the write of its first line (its standard output is a full pipe) and sent
/// SIGTERM there; once the pipe drains it must stop with status 0, not die of the signal.
#[test]
fn sigterm_while_the_first_line_is_written_stops_serve_with_status_0() {
    use std::os::fd::AsRawFd;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let (dir, operator) = keys_only("adv-serve-first-line");
    let (mut reader, writer) = std::io::pipe().expect("a pipe");
    let for_child = writer.try_clone().expect("a second write end");
    let filled = writer.as_raw_fd();
    // No thread writes to the pipe yet: a check that answers "blocked" for any pipe fails here.
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
    // Full: the filler is blocked in write(2) on the pipe. Before, the case took a pipe that
    // took no byte for 300 ms as full, and a filler that waited 300 ms for a processor left it
    // with room (story:no-test-holds-a-wall-clock-time). The bound only ends a hang.
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
            .stdout(Stdio::from(for_child))
            .stderr(Stdio::null())
            .spawn()
            .expect("the binary starts"),
    ));
    let pid = serve.child().id();
    // The main thread in write(2) (x86-64 number 1) on file descriptor 1: the test is its parent,
    // so it may read /proc/<pid>/syscall.
    let syscall = PathBuf::from(format!("/proc/{pid}/syscall"));
    let started = Clock::now();
    let blocked = loop {
        let state = fs::read_to_string(&syscall).unwrap_or_default();
        if state.starts_with("1 0x1 ") {
            break true;
        }
        if started.elapsed() > Duration::from_secs(10) {
            break false;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(blocked, "inbox serve never blocked writing its first line");
    let sent = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("kill runs");
    assert!(sent.success());
    std::thread::sleep(Duration::from_millis(200));
    stop.store(true, Ordering::SeqCst);
    let drain = std::thread::spawn(move || {
        let mut all = Vec::new();
        let _ = reader.read_to_end(&mut all);
        all
    });
    let _ = filler.join();
    let status = serve.exit_within(Duration::from_secs(10));
    drop(serve);
    let output = drain.join().expect("drained");
    assert!(
        String::from_utf8_lossy(&output).contains("http://127.0.0.1:"),
        "the first line was not written"
    );
    assert!(
        status.is_some_and(|status| status.code() == Some(0)),
        "SIGTERM during the first line: {status:?}"
    );
}

/// `INBOX_URL=http://localhost:<port>` reaches a service on 127.0.0.1:<port>.
#[test]
fn a_localhost_url_reaches_a_service_on_127_0_0_1() {
    let world = World::new("adv-localhost");
    let port = world
        .service
        .url
        .rsplit(':')
        .next()
        .expect("a port")
        .to_owned();
    let url = format!("http://localhost:{port}");
    let pending = world.client_env(
        "bob",
        &["pending", "--json"],
        &[("INBOX_URL", url.as_str())],
    );
    assert_eq!(pending.status, 0, "{pending:#?}");
}

/// With `--json`, a usage failure prints one JSON document, also when the subcommand is missing.
#[test]
fn json_without_a_subcommand_prints_one_document() {
    let dir = Dir::new("adv-json-usage");
    let usage = run(&dir.path, &["--json"], &[]);
    assert_ne!(usage.status, 0);
    assert!(
        json::parse(usage.stdout.trim()).is_ok(),
        "exit {}, stdout {:?}",
        usage.status,
        usage.stdout
    );
}

/// A stand-in whose client never connects ends with a message that names the missing request,
/// and does not wait for it forever: the case that joins it fails with that message. The wait is
/// 1 s here and 10 s for every other case.
#[test]
fn a_stand_in_whose_client_never_connects_ends_and_names_the_request() {
    let dir = Dir::new("adv-no-request");
    let (_url, seen) = stand_in_within("", dir.join("alice.outbox"), Duration::from_secs(1));
    let message = match seen.join() {
        Ok(kept) => panic!("the stand-in ended with {kept:?} and no message"),
        Err(panic) => panic.downcast_ref::<String>().cloned().unwrap_or_default(),
    };
    assert!(
        message
            .contains("the stand-in of adversary.rs received no connection for the request of `post` within 1 s"),
        "{message:?}"
    );
}
