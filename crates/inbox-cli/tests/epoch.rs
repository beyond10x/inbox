//! The cases of story:repost-does-not-redeliver-a-handled-note: the epochs file beside the outbox
//! file records, per kept envelope, the `Inbox-Epoch` of the answer that said its post arrived,
//! and `inbox repost` posts again only the envelopes posted to an earlier start of the service, or
//! recorded at none. Every key is made at run time with the binary itself.

use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use inbox::server::json::{self, Value};

use support::{ALICE, BOB, Dir, REFUSED, World, ids, key_new, run, stand_in_answers, text};

/// The `reposted` entries of a `repost --json` document.
fn reposted(document: &Value) -> Vec<Value> {
    match document.member("reposted") {
        Some(Value::Array(entries)) => entries.clone(),
        other => panic!("`reposted` is {other:?} in {document:?}"),
    }
}

/// The outcome the service answered a reposted entry with.
fn outcome(entry: &Value) -> String {
    text(entry.member("answer").expect("an answer"), "outcome")
}

/// The lines of a file, as they are.
fn lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The epochs file beside the outbox file at `outbox`.
fn epochs_file(outbox: &Path) -> PathBuf {
    let mut path = outbox.as_os_str().to_owned();
    path.push(".epochs");
    PathBuf::from(path)
}

/// The entries of the epochs file beside `outbox`: each line split at its tab.
fn entries(outbox: &Path) -> Vec<(String, String)> {
    lines(&epochs_file(outbox))
        .iter()
        .map(|line| {
            let (id, epoch) = line.split_once('\t').expect("a tab in an epochs line");
            (id.to_owned(), epoch.to_owned())
        })
        .collect()
}

fn is_epoch(text: &str) -> bool {
    text.len() == 32
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Posts a note from alice to bob, whose inbox is open, and answers its message id.
fn post_a_note(world: &World, body: &str) -> String {
    world
        .client("alice", &["post", "--to", BOB, "--body", body])
        .ok()
        .stdout
        .trim()
        .to_owned()
}

/// Reposts as alice, asserts it posted exactly the message `message_id` and the service counted
/// it as arrived, and answers the outcome; the run writes nothing to standard error.
fn repost_once(world: &World, message_id: &str) -> String {
    let run = world.client("alice", &["repost", "--json"]).ok();
    assert_eq!(run.stderr, "", "{run:#?}");
    let document = run.json();
    let entries = reposted(&document);
    assert_eq!(entries.len(), 1, "{document:?}");
    assert_eq!(text(&entries[0], "message_id"), message_id);
    assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(true)));
    outcome(&entries[0])
}

/// Reposts as alice and asserts it posted nothing.
fn repost_nothing(world: &World) {
    let said = world.client("alice", &["repost"]).ok();
    assert_eq!(said.stdout.trim(), "nothing to repost", "{said:#?}");
}

#[test]
fn a_post_that_arrived_records_the_epoch_beside_the_outbox_and_one_refused_records_none() {
    let world = World::new("epoch-recorded");
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "too early"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    world.client("bob", &["open"]).ok();
    let arrived = post_a_note(&world, "arrives");
    let outbox = world.outbox("alice");
    for line in lines(&outbox) {
        assert!(json::parse(&line).is_ok(), "{line:?}");
        assert!(!line.ends_with([' ', '\t']), "{line:?}");
    }
    let recorded = entries(&outbox);
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0].0, arrived);
    assert!(is_epoch(&recorded[0].1), "{recorded:?}");
    let mode = fs::metadata(epochs_file(&outbox))
        .expect("the epochs file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(world.kept("alice").len(), 2);
}

#[test]
fn repost_in_the_start_a_post_arrived_at_posts_nothing() {
    let world = World::new("epoch-same-start");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "once");
    for _ in 0..2 {
        let document = world.client("alice", &["repost", "--json"]).ok().json();
        assert!(reposted(&document).is_empty(), "{document:?}");
        repost_nothing(&world);
    }
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message_id));
    assert_eq!(world.kept("alice"), [message_id], "the envelope stays kept");
}

#[test]
fn a_post_refused_unknown_inbox_is_posted_again_in_the_same_start_until_it_arrives() {
    let world = World::new("epoch-opens-later");
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "open up"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let still = world.client("alice", &["repost", "--json"]);
    assert_eq!(still.status, REFUSED, "{still:#?}");
    assert_eq!(text(&still.json(), "error"), "inbox.messaging.UnknownInbox");
    assert!(entries(&world.outbox("alice")).is_empty());

    // The recipient opens later: the next repost in the same start delivers it, once.
    world.client("bob", &["open"]).ok();
    let message_id = world.kept("alice")[0].clone();
    assert_eq!(repost_once(&world, &message_id), "posted");
    repost_nothing(&world);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), [message_id]);
}

/// Deleting the epochs file leaves the outbox file as a write of the outbox file and none of the
/// epochs file leaves it: an interruption between the two writes, or an outbox file a client
/// before the epoch wrote. The envelope is posted again, never skipped.
#[test]
fn without_the_epochs_file_a_message_the_service_holds_is_duplicate_once_then_left_alone() {
    let world = World::new("epoch-missing-held");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "held");
    let outbox = world.outbox("alice");
    let kept = lines(&outbox);
    fs::remove_file(epochs_file(&outbox)).expect("the epochs file is removed");

    assert_eq!(repost_once(&world, &message_id), "duplicate");
    assert_eq!(lines(&outbox), kept, "the outbox file is unchanged");
    assert_eq!(
        entries(&outbox).len(),
        1,
        "the `duplicate` answer is recorded"
    );
    repost_nothing(&world);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message_id));
}

#[test]
fn without_the_epochs_file_after_a_restart_the_message_is_posted_once_then_left_alone() {
    let mut world = World::new("epoch-missing-restart");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "dropped");
    fs::remove_file(epochs_file(&world.outbox("alice"))).expect("the epochs file is removed");
    world.restart();
    world.client("bob", &["open"]).ok();

    assert_eq!(repost_once(&world, &message_id), "posted");
    repost_nothing(&world);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message_id));
}

#[test]
fn after_a_restart_repost_posts_once_and_then_nothing() {
    let mut world = World::new("epoch-restart");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "survive");
    let before = entries(&world.outbox("alice"));
    world.restart();
    world.client("bob", &["open"]).ok();
    assert_eq!(repost_once(&world, &message_id), "posted");
    let after = entries(&world.outbox("alice"));
    assert_eq!(after.len(), 1);
    assert_ne!(after, before, "the line took the epoch of the new start");
    repost_nothing(&world);
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message_id));
}

#[test]
fn an_unreadable_epochs_file_gives_no_epoch_and_fails_no_command() {
    let world = World::new("epoch-unreadable");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "unreadable");
    let path = epochs_file(&world.outbox("alice"));

    // Bytes that are not UTF-8.
    fs::write(&path, [0xff, 0xfe, b'\t', 0x80, b'\n']).expect("written");
    assert_eq!(repost_once(&world, &message_id), "duplicate");
    repost_nothing(&world);

    // A file its owner may not read.
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod");
    world.client("alice", &["outbox", "--json"]).ok();
    assert_eq!(repost_once(&world, &message_id), "duplicate");
    let mode = fs::metadata(&path).expect("replaced").permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "the file was replaced");
    repost_nothing(&world);
}

#[test]
fn a_malformed_epochs_line_gives_no_epoch_to_its_message() {
    let world = World::new("epoch-malformed");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "malformed");
    let path = epochs_file(&world.outbox("alice"));
    let epoch = entries(&world.outbox("alice"))[0].1.clone();
    for malformed in [
        format!("{message_id}\tnot an epoch"),
        format!("{message_id} {epoch}"),
        format!("{message_id}\t{}", epoch.to_uppercase()),
        format!("{message_id}\t{epoch}\t{epoch}"),
    ] {
        fs::write(&path, format!("{malformed}\n")).expect("written");
        assert_eq!(
            repost_once(&world, &message_id),
            "duplicate",
            "{malformed:?}"
        );
        assert_eq!(
            lines(&path),
            [format!("{message_id}\t{epoch}")],
            "{malformed:?}: the file is written again in its form"
        );
        repost_nothing(&world);
    }
}

#[test]
fn two_epochs_lines_for_one_message_give_it_none() {
    let world = World::new("epoch-twice");
    world.client("bob", &["open"]).ok();
    let message_id = post_a_note(&world, "twice");
    let path = epochs_file(&world.outbox("alice"));
    let epoch = entries(&world.outbox("alice"))[0].1.clone();
    fs::write(
        &path,
        format!("{message_id}\t{epoch}\n{message_id}\t{epoch}\n"),
    )
    .expect("written");
    assert_eq!(repost_once(&world, &message_id), "duplicate");
    assert_eq!(lines(&path), [format!("{message_id}\t{epoch}")]);
    repost_nothing(&world);
}

#[test]
fn an_entry_without_an_outbox_line_is_dropped_the_next_time_the_epochs_file_is_written() {
    let world = World::new("epoch-stray");
    world.client("bob", &["open"]).ok();
    let first = post_a_note(&world, "first");
    let second = post_a_note(&world, "second");
    let outbox = world.outbox("alice");
    let epoch = entries(&outbox)[0].1.clone();
    let stray = "01JD0000000000000000009999";
    let mut text = fs::read_to_string(epochs_file(&outbox)).expect("the epochs file");
    text.push_str(&format!("{stray}\t{epoch}\n"));
    fs::write(epochs_file(&outbox), text).expect("written");

    // The outbox file loses the line of the first message; a command that records no epoch does
    // not write the epochs file.
    let kept: Vec<String> = lines(&outbox)
        .into_iter()
        .filter(|line| !line.contains(&first))
        .map(|line| format!("{line}\n"))
        .collect();
    fs::write(&outbox, kept.concat()).expect("written");
    world.client("alice", &["outbox", "--json"]).ok();
    assert_eq!(entries(&outbox).len(), 3);

    let third = post_a_note(&world, "third");
    let recorded: Vec<String> = entries(&outbox).into_iter().map(|(id, _)| id).collect();
    assert_eq!(recorded, [second, third]);
}

/// An outbox file of alice with one envelope whose post arrived at a running service. Answers
/// the world, the path of the outbox file and the epoch recorded for the envelope.
fn an_outbox_with_an_epoch(name: &str) -> (World, PathBuf, String) {
    let world = World::new(name);
    world.client("bob", &["open"]).ok();
    post_a_note(&world, "recorded");
    let path = world.outbox("alice");
    assert_eq!(lines(&path).len(), 1);
    let recorded = entries(&path);
    assert_eq!(recorded.len(), 1);
    let epoch = recorded[0].1.clone();
    (world, path, epoch)
}

#[test]
fn an_answer_without_the_epoch_header_makes_repost_post_every_line() {
    let (world, path, _) = an_outbox_with_an_epoch("epoch-older-service");
    let kept = lines(&path);
    let (url, served) = stand_in_answers(vec![
        (200, None, r#"{"rows":[]}"#),
        (202, None, r#"{"outcome":"posted","published":[]}"#),
    ]);
    let outbox = path.display().to_string();
    let document = world
        .client_env(
            "alice",
            &["repost", "--json"],
            &[
                ("INBOX_URL", url.as_str()),
                ("INBOX_OUTBOX", outbox.as_str()),
            ],
        )
        .ok()
        .json();
    let requests = served.join().expect("the stand-in ends");
    assert_eq!(reposted(&document).len(), 1, "{document:?}");
    assert!(
        requests[0].starts_with("GET /messaging/views/outbox?"),
        "{requests:?}"
    );
    assert!(
        requests[1].starts_with("POST /messaging/commands/post "),
        "{requests:?}"
    );
    assert_eq!(lines(&path), kept, "the envelope is kept as it was");
}

#[test]
fn the_epoch_is_read_from_the_answer_to_the_question_whatever_its_status() {
    let (world, path, epoch) = an_outbox_with_an_epoch("epoch-any-status");
    let kept = lines(&path);
    let recorded = lines(&epochs_file(&path));
    for (status, body) in [
        (200, r#"{"rows":[]}"#),
        (401, r#"{"refused":"unauthenticated"}"#),
        (503, r#"{"refused":"stopping"}"#),
    ] {
        let (url, served) = stand_in_answers(vec![(status, Some(epoch.clone()), body)]);
        let outbox = path.display().to_string();
        let said = world
            .client_env(
                "alice",
                &["repost"],
                &[
                    ("INBOX_URL", url.as_str()),
                    ("INBOX_OUTBOX", outbox.as_str()),
                ],
            )
            .ok();
        let requests = served.join().expect("the stand-in ends");
        assert_eq!(
            said.stdout.trim(),
            "nothing to repost",
            "{status}: {said:#?}"
        );
        assert_eq!(
            requests.len(),
            1,
            "{status}: only the question: {requests:?}"
        );
        assert_eq!(lines(&path), kept, "{status}: the outbox file is unchanged");
        assert_eq!(
            lines(&epochs_file(&path)),
            recorded,
            "{status}: the epochs file is unchanged"
        );
    }
}

#[test]
fn a_line_posted_to_an_earlier_start_is_posted_and_takes_the_new_epoch() {
    let (world, path, epoch) = an_outbox_with_an_epoch("epoch-earlier-start");
    let kept = lines(&path);
    let other = "0123456789abcdef0123456789abcdef";
    assert_ne!(epoch, other);
    let (url, served) = stand_in_answers(vec![
        (200, Some(other.to_owned()), r#"{"rows":[]}"#),
        (
            202,
            Some(other.to_owned()),
            r#"{"outcome":"posted","published":[]}"#,
        ),
        (200, Some(other.to_owned()), r#"{"rows":[]}"#),
    ]);
    let outbox = path.display().to_string();
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_OUTBOX", outbox.as_str()),
    ];
    let first = world
        .client_env("alice", &["repost", "--json"], &env)
        .ok()
        .json();
    assert_eq!(reposted(&first).len(), 1, "{first:?}");
    let second = world.client_env("alice", &["repost"], &env).ok();
    assert_eq!(second.stdout.trim(), "nothing to repost", "{second:#?}");
    let requests = served.join().expect("the stand-in ends");
    assert_eq!(requests.len(), 3, "{requests:?}");
    assert_eq!(entries(&path)[0].1, other);
    assert_eq!(lines(&path), kept);
}

#[test]
fn no_question_is_asked_when_no_line_carries_an_epoch() {
    let dir = Dir::new("epoch-no-question");
    key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let outbox = dir.join("alice.outbox");
    let outbox_text = outbox.display().to_string();
    let (url, served) = stand_in_answers(vec![(503, None, r#"{"refused":"stopping"}"#)]);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let posted = run(&dir.path, &["post", "--to", BOB, "--body", "x"], &env);
    assert_eq!(posted.status, REFUSED, "{posted:#?}");
    served.join().expect("the stand-in ends");
    let (url, served) = stand_in_answers(vec![(
        202,
        Some("0123456789abcdef0123456789abcdef".to_owned()),
        r#"{"outcome":"posted","published":[]}"#,
    )]);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let reposted_run = run(&dir.path, &["repost", "--json"], &env).ok();
    let requests = served.join().expect("the stand-in ends");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert!(requests[0].starts_with("POST "), "{requests:?}");
    assert_eq!(reposted(&reposted_run.json()).len(), 1);
}
