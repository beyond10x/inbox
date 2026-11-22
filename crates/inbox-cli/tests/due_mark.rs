//! story:a-due-message-takes-no-mark at the client. The client sends `MarkRead` only for
//! `inbox read <id>` and `MarkDelivered` never; `pending` and `wait` read the `Pending` view and
//! mark nothing. So a message whose `expires` has passed, still held until the expiry task runs,
//! is listed by `pending` and `wait` (exit 0) beside a live one, and `inbox read` of it alone exits
//! as a refusal naming `inbox.messaging.Expired` and leaves it listed. A due note alone does not
//! end `wait`: it holds to its bound and prints the note then. A real `inbox serve`, its expiry
//! task an hour apart, its clock a file the case moves (`--clock-file`, a debug build's flag): the
//! note is posted at the clock's start and the clock is moved past its `expires`, so no wait on
//! real time decides whether the note is due (story:no-test-holds-a-wall-clock-time: the case gave
//! the note a life of 2 s on the system clock, and a client start of 3 s had the post refused
//! `AlreadyExpired`). The one wait on real time left is `wait --seconds 1`.

use crate::support;

use std::time::{Duration, SystemTime};

use inbox_server::{format_instant, format_reading};
use inbox_verify::Instant;
use support::{BOB, REFUSED, Service, World, ids, keys_only, text};

/// The system clock's second.
fn this_second() -> Instant {
    Instant::from_unix(
        Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

/// Sets the service's clock file whole: written beside it and renamed over it.
fn set_clock(dir: &std::path::Path, at: Instant) {
    std::fs::write(dir.join("clock.new"), format_reading(at)).expect("the clock file is written");
    std::fs::rename(dir.join("clock.new"), dir.join("clock")).expect("the clock file is replaced");
}

#[test]
fn a_due_message_stays_listed_and_only_its_own_read_is_refused() {
    let (dir, operator_public_key) = keys_only("due-mark");
    set_clock(&dir.path, this_second());
    let service = Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator_public_key,
            "--max-wait",
            "20",
            "--sweep-interval",
            "3600",
            "--clock-file",
            "clock",
        ],
        &[],
    );
    let world = World {
        service,
        operator_public_key,
        dir,
    };
    world.client("bob", &["open"]).ok();

    // 30 s after the system clock just before the post: the client writes `created` from the
    // system clock and the service refuses an `expires` at or before it, so one process start is
    // inside the margin. The service's clock stays at its start until the case moves it.
    let expires_at = this_second().plus_seconds(30);
    let expires = format_instant(expires_at);
    let due = world
        .client(
            "alice",
            &[
                "post",
                "--to",
                BOB,
                "--body",
                "soon due",
                "--expires",
                &expires,
            ],
        )
        .ok()
        .stdout
        .trim()
        .to_owned();
    let live = world
        .client("alice", &["post", "--to", BOB, "--body", "a day to live"])
        .ok()
        .stdout
        .trim()
        .to_owned();

    // The note is due: the clock is 1 s past its `expires`. The clients sign with the system
    // clock, and the service takes a request signed within 60 s of its reading, 31 s after the
    // post; the runner ends a case at 60 s.
    set_clock(&world.dir.path, expires_at.plus_seconds(1));

    let listed = |args: &[&str]| {
        let mut held = ids(&world.client("bob", args).ok().json());
        held.sort();
        held
    };
    let mut both = vec![due.clone(), live.clone()];
    both.sort();
    assert_eq!(listed(&["pending", "--json"]), both);
    assert_eq!(listed(&["wait", "--seconds", "1", "--json"]), both);

    let refused = world.client("bob", &["read", &due, "--json"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert_eq!(text(&refused.json(), "error"), "inbox.messaging.Expired");
    let answer = refused
        .json()
        .member("answer")
        .cloned()
        .expect("the answer");
    assert_eq!(text(&answer, "outcome"), "deadline-passed");

    assert_eq!(listed(&["pending", "--json"]), both, "the due note stays");
    let handled = world.client("bob", &["read", &live, "--json"]).ok();
    assert_eq!(text(&handled.json(), "outcome"), "handled");
    assert_eq!(listed(&["pending", "--json"]), std::slice::from_ref(&due));

    // With only the due note left, `wait` is not ended by it: it holds for its whole second, then
    // prints the rows `pending` prints (the due note) and exits 0, as a wait that ends empty does.
    let started = std::time::Instant::now();
    let waited = world.client("bob", &["wait", "--seconds", "1"]).ok();
    assert!(
        started.elapsed() >= Duration::from_secs(1),
        "the wait ended after {:?}",
        started.elapsed()
    );
    assert!(waited.stdout.contains(&due), "{waited:#?}");
    assert_eq!(
        waited.stdout,
        world.client("bob", &["pending"]).ok().stdout,
        "a wait that ends at its bound prints what pending prints"
    );
}
