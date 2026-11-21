//! Adversary case of story:service-acceptance on the third acceptance check.
//!
//! `after_a_restart_two_reposts_put_the_unread_message_in_pending_once` asserts that `Pending`
//! holds the message once. `MemoryBackend` keys a message by its id, so a second post of the same
//! envelope replaces the row and can never make two: the assertion holds for a service whose
//! `PostMessage` has no duplicate check at all (measured: `inbox-core` with both duplicate
//! guards of `PostMessage` removed passes all 3 checks). What the repost scenario claims, that the
//! second post arrives as `duplicate` and changes nothing, is asserted here from the answer the
//! client prints. Ignored like the 3 checks; run with `INBOX_BIN` and `--ignored`.
//!
//! The first repost records, in the epochs file beside the outbox file, the epoch of the start
//! that said the post arrived, and a second repost in that start posts nothing
//! (story:repost-does-not-redeliver-a-handled-note). Each repost here runs without that file, as
//! after a client before the epoch, so the second post reaches the duplicate check.

use inbox_acceptance::{BOB, World, ids, member, text};

/// Removes the epochs file beside the outbox file of `who`, if there is one.
fn forget_the_epochs(world: &World, who: &str) {
    let mut path = world.outbox(who).into_os_string();
    path.push(".epochs");
    let _ = std::fs::remove_file(path);
}

fn root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
}

#[test]
#[ignore = "needs the built inbox binary in INBOX_BIN"]
fn after_a_restart_the_second_repost_is_answered_duplicate() {
    let mut world = World::new(root(), "adversary-restart", &[]);
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
    world.restart();
    world.client("bob", &["open"]).ok();
    let outcomes: Vec<String> = (0..2)
        .map(|_| {
            forget_the_epochs(&world, "alice");
            let reposted = world.client("alice", &["repost", "--json"]).ok().json();
            let inbox::server::json::Value::Array(entries) = member(&reposted, &["reposted"])
            else {
                panic!("`reposted` is no list: {reposted:?}");
            };
            assert_eq!(entries.len(), 1, "{reposted:?}");
            text(&entries[0], &["answer", "outcome"])
        })
        .collect();
    assert_eq!(
        outcomes,
        ["posted", "duplicate"],
        "the second repost of {message} after the restart"
    );
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message));
}
