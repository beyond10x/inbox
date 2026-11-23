//! The 6 checks of `task acceptance`, each against its own `inbox serve` on a loopback address,
//! driven by the `inbox` client commands with keys made on the spot: the 3 of
//! story:service-acceptance over plain HTTP, and the 3 of story:the-stage-3-acceptance-checks
//! over TLS between 2 clients that share no file.
//!
//! Each check is ignored by a plain `cargo test`: it needs the built binary, which `task
//! acceptance` builds and names in `INBOX_BIN` before it runs them with `--ignored`, and it fails,
//! naming the cause, where `INBOX_BIN` names no file. `cargo test --workspace` therefore runs none
//! of them, and `task acceptance` fails when fewer than 6 ran.

use std::net::TcpListener;
use std::sync::mpsc;
use std::time::{Duration, Instant as Monotonic, SystemTime};

use inbox::server::json::Value;
use inbox_acceptance::{ALICE, BOB, HANG, TwoHosts, World, ids, member, rows, text};
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

// The 3 checks of stage 3 (`docs/design/inbox.md`, "Stages", item 3). Each starts its own
// `inbox serve` with a certificate (`inbox tls new`, `inbox serve --tls-file`) on a loopback
// address, and 2 clients that share nothing but the binary ([`TwoHosts`]): each its own
// directory, key, outbox file, state directory and home, each command started with an empty
// environment and that client's variables only, each given the pin. They prove that nothing of
// one client host is needed by the other; they do not prove the path between 2 hosts, which the
// operator's run on 2 real hosts with a real session shows, outside the gate.

/// The rows of `value` at `path` that is a list.
fn list(value: &Value, path: &[&str]) -> Vec<Value> {
    match member(value, path) {
        Value::Array(entries) => entries.clone(),
        other => panic!("`{}` is no list: {other:?}", path.join(".")),
    }
}

/// `inbox post` of a request from alice to bob, with a free reply; answers its message id.
fn post_request(hosts: &TwoHosts, body: &str) -> String {
    let posted = hosts
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
                body,
                "--json",
            ],
        )
        .ok()
        .json();
    assert_eq!(outcome(&posted), "posted");
    text(&posted, &["message_id"])
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN: run by task acceptance"]
fn two_clients_that_share_no_file_exchange_a_request_and_a_reply_over_tls() {
    let hosts = TwoHosts::new(root(), "apart", &[]);
    hosts.assert_apart();
    assert!(
        hosts.url().starts_with("https://127.0.0.1:"),
        "the service serves no https:// URL on 127.0.0.1"
    );

    hosts.client("bob", &["open"]).ok();
    let request = post_request(&hosts, "Which colour?");

    let pending = hosts.client("bob", &["pending", "--json"]).ok().json();
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

    hosts.client("bob", &["read", &request]).ok();
    let answered = hosts
        .client("bob", &["answer", &request, "--body", "Blue", "--json"])
        .ok()
        .json();
    assert_eq!(outcome(&answered), "answered");

    let outbox = hosts.client("alice", &["outbox", "--json"]).ok().json();
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

    let collected = hosts
        .client("alice", &["collect", &request, "--json"])
        .ok()
        .json();
    assert_eq!(outcome(&collected), "collected");
    assert_eq!(text(&collected, &["message", "reply", "body"]), "Blue");

    // A post of alice with the pin of another certificate: the certificate of the service is not
    // the one the pin names, so the client ends the handshake before it sends a byte (exit 3) and
    // nothing reaches bob. It comes after the exchange: the envelope stays kept and marked unsent
    // in alice's outbox, and her next command would post it again.
    let other_pin = hosts.pin_of_another_certificate();
    let refused = hosts.client_with(
        "alice",
        &[("INBOX_TLS_PIN", &other_pin)],
        &[
            "post",
            "--to",
            BOB,
            "--body",
            "To another certificate",
            "--json",
        ],
    );
    assert_eq!(
        refused.status, 3,
        "a post to a service whose certificate the pin does not name ends with 3; standard error: {}",
        refused.stderr
    );
    let pending = hosts.client("bob", &["pending", "--json"]).ok().json();
    assert!(
        ids(&pending).is_empty(),
        "a post with the pin of another certificate reached bob"
    );
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN: run by task acceptance"]
fn a_wake_of_the_second_client_exits_with_the_post_of_the_first_and_marks_it_delivered() {
    // The service holds a read for 60 s at most, longer than the runner lets the check run: only
    // the post ends the wake's held read.
    let mut hosts = TwoHosts::new(root(), "wake", &["--max-wait", "60"]);
    hosts.assert_apart();
    let route = hosts.route("bob");
    hosts.client("bob", &["open"]).ok();

    let alice_before = hosts.snapshot("alice");
    let from = route.connections();
    let mut woken = hosts.spawn("bob", &["wake", "--json"]);
    // The wake reads `Pending` without a wait first, then holds a read open, each on a
    // connection of its own (one request per connection). The fact waited for: the first
    // connection of the wake was answered and ended by the service (a read with no row, or the
    // wake would have exited), and on the second the wake sent, after the service's part of the
    // handshake, at least as many bytes as on the first (its TLS Finished and a request of the
    // same size, `wait=60` where the first had `wait=0`), and the service has not ended it.
    route.wait_until("the wake holds its read of Pending", |carried| {
        let since = &carried[from.min(carried.len())..];
        since.len() == 2
            && since[0].service_ended
            && since[1].from_client_after_service > 0
            && since[1].from_client_after_service >= since[0].from_client_after_service
            && !since[1].service_ended
    });
    assert!(woken.running(), "the wake ended before the post");
    hosts.assert_environment(&woken, "bob");
    assert_eq!(
        woken.working_directory(),
        std::fs::canonicalize(hosts.dir("bob")).expect("bob's directory"),
        "the wake runs in bob's directory"
    );
    hosts.assert_unchanged_since("alice", &alice_before, "the start of bob's wake");

    // While the wake holds its read, bob's directory does not change, so the post of alice is
    // held to it as every client command is.
    let posted = hosts
        .client(
            "alice",
            &["post", "--to", BOB, "--body", "wake up", "--json"],
        )
        .ok()
        .json();
    assert_eq!(outcome(&posted), "posted");
    let message = text(&posted, &["message_id"]);
    let alice_after = hosts.snapshot("alice");
    let ran = woken.finish();
    hosts.assert_unchanged_since("alice", &alice_after, "the end of bob's wake");
    assert_eq!(ran.status, 0, "{ran:#?}");
    assert_eq!(
        ids(&ran.json()),
        std::slice::from_ref(&message),
        "the wake printed the post: {ran:#?}"
    );
    // The post was answered on the read the wake held: the service ended that connection after
    // writing on it more than on the read without a wait, whose answer had no row, by at least
    // what the post's row must hold: its id twice (`message_id`, `thread`) and both principals
    // (`from`, `to`). Otherwise the 2 answers differ in the digits of their `Content-Length` and
    // in how many TLS records carry them (22 bytes for each record more, measured once). A wake
    // that reads without a wait and pauses has its second read answered without a row too, and
    // its post comes on a later connection. A count of the
    // wake's connections (3: the read without a wait, the held read, the `MarkDelivered`) is not
    // asserted: the wake tries again after a failure in transport or a `408` or `5xx`, which
    // adds one and is no defect.
    let carried = route.carried();
    let (first, answered) = (carried[from], carried[from + 1]);
    let row = 2 * message.len() + ALICE.len() + BOB.len();
    assert!(
        answered.service_ended && answered.from_service >= first.from_service + row,
        "the post was not answered on the read the wake held: the service wrote {} bytes on it and \
         {} on the read without a wait",
        answered.from_service,
        first.from_service
    );

    let listed = hosts.client("bob", &["pending", "--json"]).ok().json();
    let states: Vec<(String, String)> = rows(&listed)
        .iter()
        .map(|row| (text(row, &["message_id"]), text(row, &["state"])))
        .collect();
    assert_eq!(
        states,
        [(message, "Delivered".to_owned())],
        "the wake marked the post delivered"
    );
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN: run by task acceptance"]
fn after_a_restart_a_later_command_of_each_client_puts_the_kept_request_in_pending_once() {
    // After the restart this check types no `inbox open` and no `inbox repost`: its commands are
    // bob's `pending` and alice's `outbox`. The proof that their reconnect opened the inbox and
    // posted the request again is the `reconnected` member of each one's answer and bob's
    // `Pending`.
    let mut hosts = TwoHosts::new(root(), "reconnect", &[]);
    hosts.assert_apart();
    hosts.client("bob", &["open"]).ok();
    let request = post_request(&hosts, "Kept across the restart");
    let pending = hosts.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&request));

    // The clients reach the service through the front, a forwarder of the fixture, which carries
    // the bytes unchanged (the `Inbox-Epoch` of each answer included: TLS runs end to end). The
    // restart starts the new service on a port of its own, so a socket of the check that takes
    // the old service's port as soon as it is free, and holds it until the check ends, changes
    // nothing.
    let old = hosts.service_address();
    let (bound, held) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let squatter = std::thread::spawn(move || {
        let started = Monotonic::now();
        while started.elapsed() < HANG {
            if let Ok(listener) = TcpListener::bind(old) {
                let _ = bound.send(());
                let _ = released.recv();
                drop(listener);
                return;
            }
            std::thread::yield_now();
        }
    });
    hosts.restart();
    held.recv_timeout(HANG)
        .expect("a socket of the check holds the old service's port");

    // bob's `inbox pending`, his first command after the restart: its answer names the new
    // start, the restart lost the request, and its reconnect opens bob's inbox again with the
    // adapter of his `inbox open`.
    let lost = hosts.client("bob", &["pending", "--json"]).ok().json();
    assert!(ids(&lost).is_empty(), "the restart lost the request");
    assert_eq!(
        text(&lost, &["reconnected", "open", "outcome"]),
        "opened",
        "the reconnect of bob's pending opened his inbox again: {lost:?}"
    );

    // alice's `inbox outbox`, her first command after the restart: its answer names the new
    // start, at which her kept request did not arrive, and its reconnect posts it again.
    let outbox = alice_outbox(&hosts);
    let reposted: Vec<(String, String)> = list(&outbox, &["reconnected", "repost", "reposted"])
        .iter()
        .map(|entry| (text(entry, &["message_id"]), outcome(entry)))
        .collect();
    assert_eq!(
        reposted,
        [(request.clone(), "posted".to_owned())],
        "the reconnect of alice's outbox posted the kept request again"
    );

    let pending = hosts.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&request),
        "Pending holds the request once"
    );

    // A later command of each starts no reconnect: the epochs file of alice records the start
    // that took the repost, the record of bob's open names it.
    let again = alice_outbox(&hosts);
    assert!(
        again.member("reconnected").is_none(),
        "a second reconnect ran: {again:?}"
    );
    assert_eq!(ids(&again), std::slice::from_ref(&request));
    let pending = hosts.client("bob", &["pending", "--json"]).ok().json();
    assert!(pending.member("reconnected").is_none(), "{pending:?}");
    assert_eq!(
        ids(&pending),
        std::slice::from_ref(&request),
        "Pending still holds the request once"
    );
    let _ = release.send(());
    squatter.join().expect("the squatter ends");
}

/// `inbox outbox --json` of alice.
fn alice_outbox(hosts: &TwoHosts) -> Value {
    hosts.client("alice", &["outbox", "--json"]).ok().json()
}
