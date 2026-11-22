//! story:a-due-message-takes-no-mark at the client. The client sends `MarkRead` only for
//! `inbox read <id>` and `MarkDelivered` never; `pending` and `wait` read the `Pending` view and
//! mark nothing. So a message whose `expires` has passed, still held until the expiry task runs,
//! is listed by `pending` and `wait` (exit 0) beside a live one, and `inbox read` of it alone exits
//! as a refusal naming `inbox.messaging.Expired` and leaves it listed. A due note alone does not
//! end `wait`: it holds to its bound and prints the note then. A real `inbox serve`, its expiry
//! task an hour apart; the waits on real time are until the note's `expires` (at most 2 s) and the
//! one wait of 1 s.

use crate::support;

use std::time::{Duration, SystemTime};

use inbox_server::format_instant;
use inbox_verify::Instant;
use support::{BOB, REFUSED, Service, World, ids, keys_only, text};

#[test]
fn a_due_message_stays_listed_and_only_its_own_read_is_refused() {
    let (dir, operator_public_key) = keys_only("due-mark");
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
        ],
        &[],
    );
    let world = World {
        service,
        operator_public_key,
        dir,
    };
    world.client("bob", &["open"]).ok();

    let expires_at = Instant::from_system_time(SystemTime::now()).unix_seconds() + 2;
    let expires = format_instant(Instant::from_unix(expires_at, 0));
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

    let until = SystemTime::UNIX_EPOCH
        + Duration::from_secs(u64::try_from(expires_at).expect("after 1970"))
        + Duration::from_millis(100);
    if let Ok(left) = until.duration_since(SystemTime::now()) {
        std::thread::sleep(left);
    }

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
