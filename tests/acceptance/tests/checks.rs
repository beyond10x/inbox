//! The 3 checks of story:service-acceptance, each against its own `inbox serve` on
//! `127.0.0.1:0`, driven by the `inbox` client commands with keys made on the spot.
//!
//! Each check is ignored by a plain `cargo test`: it needs the built binary, which `task
//! acceptance` builds and names in `INBOX_BIN` before it runs them with `--ignored`, and it fails,
//! naming the cause, where `INBOX_BIN` names no file. `cargo test --workspace` therefore runs none
//! of them, and `task acceptance` fails when fewer than 3 ran.

use std::time::{Duration, Instant as Monotonic, SystemTime};

use inbox::server::json::Value;
use inbox_acceptance::{ALICE, BOB, World, ids, member, rows, text};
use inbox_server::format_instant;
use inbox_verify::Instant;

/// The longest wait for the expiry task to sweep at a clock reading the check set: it sweeps
/// every second (`--sweep-interval 1`), and this bound only ends a hang.
const SWEEP_WAIT: Duration = Duration::from_secs(10);

/// The directory each check makes its own directory in.
fn root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
}

fn outcome(posted: &Value) -> String {
    text(posted, &["answer", "outcome"])
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN: run by task acceptance"]
fn two_clients_with_different_keys_exchange_a_signed_request_and_a_reply() {
    let world = World::new(root(), "exchange", &[]);
    world.client("bob", &["open"]).ok();
    let posted = world
        .client(
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
                "--json",
            ],
        )
        .ok()
        .json();
    assert_eq!(outcome(&posted), "posted");
    let request = text(&posted, &["message_id"]);

    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&request));
    let row = &rows(&pending)[0];
    assert_eq!(text(row, &["from"]), ALICE);
    assert_eq!(text(row, &["kind"]), "request");
    assert_eq!(text(row, &["body"]), "Which colour?");
    assert_eq!(
        text(row, &["signature", "key_id"]),
        "alice-1",
        "the request is signed with alice's key"
    );

    world.client("bob", &["read", &request]).ok();
    let answered = world
        .client("bob", &["answer", &request, "--body", "Blue", "--json"])
        .ok()
        .json();
    assert_eq!(outcome(&answered), "answered");

    let outbox = world.client("alice", &["outbox", "--json"]).ok().json();
    assert_eq!(ids(&outbox), std::slice::from_ref(&request));
    let row = &rows(&outbox)[0];
    assert_eq!(text(row, &["state"]), "Answered");
    assert_eq!(text(row, &["reply", "from"]), BOB);
    assert_eq!(text(row, &["reply", "body"]), "Blue");
    assert_eq!(
        text(row, &["reply", "signature", "key_id"]),
        "bob-1",
        "the reply is signed with bob's key"
    );

    let collected = world
        .client("alice", &["collect", &request, "--json"])
        .ok()
        .json();
    assert_eq!(outcome(&collected), "collected");
    assert_eq!(text(&collected, &["message", "reply", "body"]), "Blue");
}

/// Whether `message` is in a view either client reads, and which: `Outbox` of the sender,
/// `Pending` of the recipient, `Thread` and `Replies` of its thread (the message's own id). Every
/// other view of a message is a part of `Outbox` of its sender: `Unanswered` holds the messages of
/// one recipient in state `Read`, `Settled` the messages of one sender in `Answered` or
/// `Refused`, and `Outbox` every live message of its sender, in whatever state
/// (`ess/domains/messaging.yaml`). `Inboxes` holds inboxes, no message.
fn views_holding(world: &World, message: &str) -> Vec<&'static str> {
    let reads: [(&str, &str, Vec<&str>); 4] = [
        ("Outbox", "alice", vec!["outbox", "--json"]),
        ("Pending", "bob", vec!["pending", "--json"]),
        ("Thread", "alice", vec!["thread", message, "--json"]),
        ("Replies", "alice", vec!["replies", message, "--json"]),
    ];
    reads
        .into_iter()
        .filter(|(_, who, args)| {
            ids(&world.client(who, args).ok().json()).contains(&message.to_owned())
        })
        .map(|(view, _, _)| view)
        .collect()
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN: run by task acceptance"]
fn a_read_note_and_an_expired_unanswered_request_are_in_no_view() {
    // The service's clock is a file the check moves (`inbox serve --clock-file`): it reads the
    // system clock's second at the start and stays there until the check moves it, so however
    // long the client processes take on a loaded host, no view is read after `expires`
    // (2026-11-22: with the system clock, `Thread` was read after a life of 2 s at a host load
    // of about 100).
    let start = Instant::from_unix(
        Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    );
    let world = World::with_clock(root(), "gone", &["--sweep-interval", "1"], start);
    world.client("bob", &["open"]).ok();

    // A request read by bob and not answered. Its `expires` is 30 s after the system clock just
    // before the post: the client writes `created` from the system clock and the service refuses
    // an `expires` at or before it, so only the start of the one `post` process is inside the
    // margin.
    let expires_at = Instant::from_unix(
        Instant::from_system_time(SystemTime::now()).unix_seconds() + 30,
        0,
    );
    let expires = format_instant(expires_at);
    let request = world
        .client(
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
                "Answer before it expires",
                "--expires",
                &expires,
            ],
        )
        .ok()
        .stdout
        .trim()
        .to_owned();
    world.client("bob", &["read", &request]).ok();
    assert_eq!(
        views_holding(&world, &request),
        ["Outbox", "Thread"],
        "a request read and not answered is in its sender's Outbox and in its thread"
    );

    // A note, read by bob: handled, and removed.
    let note = world
        .client(
            "alice",
            &["post", "--to", BOB, "--body", "Just so you know"],
        )
        .ok()
        .stdout
        .trim()
        .to_owned();
    assert_eq!(
        views_holding(&world, &note),
        ["Outbox", "Pending", "Thread"]
    );
    world.client("bob", &["read", &note]).ok();
    assert!(
        views_holding(&world, &note).is_empty(),
        "a read note is in {:?}",
        views_holding(&world, &note)
    );

    // The clock passes `expires` by 1 s. The clients sign with the system clock, and the service
    // takes a request signed within 60 s of its reading: 31 s after the post, it takes every
    // request the check signs within 91 s of the post, and the runner ends a check at 60 s.
    world.set_clock(expires_at.plus_seconds(1));
    // The expiry task sweeps every second at the clock's reading. The event waited for is the
    // request leaving its sender's `Outbox`, a sweep's work; the bound only ends a hang.
    let waited = Monotonic::now();
    while ids(&world.client("alice", &["outbox", "--json"]).ok().json()).contains(&request)
        && waited.elapsed() < SWEEP_WAIT
    {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        views_holding(&world, &request).is_empty(),
        "the request that expired at {expires} is in {:?}",
        views_holding(&world, &request)
    );
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN: run by task acceptance"]
fn after_a_restart_two_reposts_put_the_unread_message_in_pending_once() {
    let mut world = World::new(root(), "restart", &[]);
    world.client("bob", &["open"]).ok();
    let message = world
        .client(
            "alice",
            &["post", "--to", BOB, "--body", "Survive the restart"],
        )
        .ok()
        .stdout
        .trim()
        .to_owned();
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message));

    world.restart();
    let lost = world.client("bob", &["pending", "--json"]).ok().json();
    assert!(ids(&lost).is_empty(), "the restart lost the message");

    world.client("bob", &["open"]).ok();
    // The first repost delivers the message again, and the epochs file records the start that
    // said it arrived; the second, in that start, posts nothing.
    let outcomes: Vec<Vec<String>> = (0..2)
        .map(|_| {
            let reposted = world.client("alice", &["repost", "--json"]).ok().json();
            let Value::Array(entries) = member(&reposted, &["reposted"]) else {
                panic!("`reposted` is no list: {reposted:?}");
            };
            entries
                .iter()
                .map(|entry| {
                    assert_eq!(text(entry, &["message_id"]), message);
                    text(entry, &["answer", "outcome"])
                })
                .collect()
        })
        .collect();
    assert_eq!(
        outcomes,
        [vec!["posted".to_owned()], Vec::new()],
        "what the 2 reposts after the restart posted, and what each was answered"
    );
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&message),
        "Pending holds the message once"
    );
}
