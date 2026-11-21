//! story:post-does-not-tell-a-message-id-exists at the client: `id-taken` (a message id another
//! sender's live message carries) is a final refusal. `post` and `repost` exit 1 naming
//! `inbox.messaging.MessageIdTaken` and drop the envelope from the outbox file. The answer is the
//! one the generated server sends for the outcome; the client changes nothing for it. Every key is
//! made at run time.

mod support;

use std::fs;

use inbox::server::json::Value;

use support::{ALICE, BOB, Dir, REFUSED, key_new, run, stand_in, text};

/// The status and body the generated server answers `id-taken` of `PostMessage` with.
const ID_TAKEN: (u16, &str) = (
    409,
    r#"{"outcome":"id-taken","published":[],"error":"inbox.messaging.MessageIdTaken"}"#,
);

fn outbox_lines(path: &std::path::Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn id_taken_is_final_on_post_and_on_repost() {
    let dir = Dir::new("id-taken");
    key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));

    let outbox = dir.join("alice-post.outbox");
    let outbox_text = outbox.display().to_string();
    let (url, served) = stand_in(ID_TAKEN.0, ID_TAKEN.1);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let posted = run(
        &dir.path,
        &["post", "--to", BOB, "--body", "x", "--json"],
        &env,
    );
    served.join().expect("the stand-in ends");
    assert_eq!(posted.status, REFUSED, "{posted:#?}");
    assert_eq!(
        text(&posted.json(), "error"),
        "inbox.messaging.MessageIdTaken"
    );
    assert!(
        outbox_lines(&outbox).is_empty(),
        "post drops the envelope on id-taken"
    );

    // An envelope kept on a 503 is reposted and answered `id-taken`: not arrived, and dropped.
    let outbox = dir.join("alice-repost.outbox");
    let outbox_text = outbox.display().to_string();
    let (url, served) = stand_in(503, r#"{"refused":"stopping"}"#);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let kept = run(&dir.path, &["post", "--to", BOB, "--body", "y"], &env);
    served.join().expect("the stand-in ends");
    assert_eq!(kept.status, REFUSED, "{kept:#?}");
    assert_eq!(outbox_lines(&outbox).len(), 1, "a 503 keeps it");

    let (url, served) = stand_in(ID_TAKEN.0, ID_TAKEN.1);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let reposted = run(&dir.path, &["repost", "--json"], &env);
    served.join().expect("the stand-in ends");
    assert_eq!(reposted.status, REFUSED, "{reposted:#?}");
    let document = reposted.json();
    assert_eq!(text(&document, "error"), "inbox.messaging.MessageIdTaken");
    let entries = match document
        .member("answer")
        .and_then(|answer| answer.member("reposted"))
    {
        Some(Value::Array(entries)) => entries.clone(),
        other => panic!("no reposted list in {document:?}: {other:?}"),
    };
    assert_eq!(entries.len(), 1, "{reposted:#?}");
    assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(false)));
    assert!(
        reposted
            .stderr
            .contains("inbox.messaging.MessageIdTaken (409)"),
        "{reposted:#?}"
    );
    assert!(
        outbox_lines(&outbox).is_empty(),
        "repost drops the envelope on id-taken"
    );
}
