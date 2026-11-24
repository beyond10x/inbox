//! Adversary cases of story:no-case-of-the-bounded-run-waits-on-a-real-tool (pass 1), for the
//! case `adversary::an_envelope_no_service_took_leaves_the_outbox_when_it_expires`: it posts
//! with an `--expires` one second after the instant the case reads, so on a loaded host the
//! client may start after that `expires`. These cases hold what the case then relies on. Every
//! key is made at run time.

use crate::support;

use std::fs;
use std::time::{Duration, SystemTime};

use inbox::server::json::{self, Value};
use inbox_verify::Instant;

use support::{BOB, World};

/// No service listens there; a loopback literal, so no name is resolved.
const NOWHERE: [(&str, &str); 1] = [("INBOX_URL", "http://127.0.0.1:1")];

/// A client held back past the `expires` it was given, as a loaded host holds the client of the
/// case: its post reaches no service, and the envelope is still kept (one line), so the case's
/// `lines().count() == 1` holds; the next command that reads the outbox drops it by its clock.
#[test]
fn a_post_that_starts_after_its_expires_is_kept_and_dropped_by_the_next_command() {
    let world = World::new("adv-expired-on-post");
    let past = inbox_server::format_instant(Instant::from_system_time(
        SystemTime::now() - Duration::from_secs(2),
    ));
    let kept = world.client_env(
        "alice",
        &["post", "--to", BOB, "--body", "late", "--expires", &past],
        &NOWHERE,
    );
    assert_eq!(kept.status, support::TRANSPORT, "{kept:#?}");
    let lines = fs::read_to_string(world.outbox("alice")).unwrap_or_default();
    assert_eq!(
        lines.lines().count(),
        1,
        "a post whose expires had passed when it started is not kept: {lines}"
    );
    let reposted = world.client_env("alice", &["repost", "--json"], &NOWHERE);
    assert!(
        world.kept("alice").is_empty(),
        "the expired entry is still kept after a repost: {reposted:#?}"
    );
    // Dropped before it is sent: a repost that tried to send it would end in transport (exit
    // 3), and the line would still leave, by the mark of the failure the repost writes after it.
    assert_eq!(
        reposted.status, 0,
        "repost sent an expired envelope: {reposted:#?}"
    );
    let report = json::parse(reposted.stdout.trim()).expect("the report is JSON");
    match report.member("reposted") {
        Some(Value::Array(entries)) => assert!(entries.is_empty(), "{report:?}"),
        other => panic!("`reposted` is {other:?} in {report:?}"),
    }
}
