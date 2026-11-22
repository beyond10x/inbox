//! Adversary pass 1 on story:a-due-message-takes-no-answer at the client (attack 7): the 3 new
//! outcomes come as status 502 with a declared error. `answer` and `refuse` exit as a refusal
//! naming `inbox.messaging.Expired`; `post` and `repost` exit as a refusal naming
//! `inbox.messaging.AlreadyExpired` and drop the envelope, which the service's clock will never
//! take again. The answers are the generated server's bodies, from a stand-in; every key is made
//! at run time.

use crate::support;

use std::fs;

use support::{ALICE, BOB, Dir, REFUSED, key_new, run, stand_in, text};

const DEADLINE_PASSED: (u16, &str) = (
    502,
    r#"{"outcome":"deadline-passed","published":[],"error":"inbox.messaging.Expired"}"#,
);
const EXPIRED_ON_ARRIVAL: (u16, &str) = (
    502,
    r#"{"outcome":"expired-on-arrival","published":[],"error":"inbox.messaging.AlreadyExpired"}"#,
);
const MESSAGE: &str = "01JGT0DE000000000000000001";

fn lines(path: &std::path::Path) -> usize {
    fs::read_to_string(path).unwrap_or_default().lines().count()
}

#[test]
fn the_three_clock_outcomes_are_refusals_at_the_client() {
    let dir = Dir::new("adversary-deadline");
    key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let outbox = dir.join("alice.outbox");
    let outbox_text = outbox.display().to_string();

    for args in [
        vec!["answer", MESSAGE, "--option", "A", "--body", "A", "--json"],
        vec!["refuse", MESSAGE, "--reason", "late", "--json"],
    ] {
        let (url, served) = stand_in(DEADLINE_PASSED.0, DEADLINE_PASSED.1);
        let env = [
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", "alice.key"),
            ("INBOX_OUTBOX", outbox_text.as_str()),
        ];
        let done = run(&dir.path, &args, &env);
        served.join().expect("the stand-in ends");
        assert_eq!(done.status, REFUSED, "{args:?}: {done:#?}");
        assert_eq!(
            text(&done.json(), "error"),
            "inbox.messaging.Expired",
            "{args:?}"
        );
        let answer = done.json().member("answer").cloned().expect("the answer");
        assert_eq!(text(&answer, "outcome"), "deadline-passed", "{args:?}");
    }

    let (url, served) = stand_in(EXPIRED_ON_ARRIVAL.0, EXPIRED_ON_ARRIVAL.1);
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
        "inbox.messaging.AlreadyExpired"
    );
    assert_eq!(
        lines(&outbox),
        0,
        "post drops an envelope expired on arrival"
    );

    let (url, served) = stand_in(503, r#"{"refused":"stopping"}"#);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let kept = run(&dir.path, &["post", "--to", BOB, "--body", "y"], &env);
    served.join().expect("the stand-in ends");
    assert_eq!(kept.status, REFUSED, "{kept:#?}");
    assert_eq!(lines(&outbox), 1, "a 503 keeps it");
    let (url, served) = stand_in(EXPIRED_ON_ARRIVAL.0, EXPIRED_ON_ARRIVAL.1);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", outbox_text.as_str()),
    ];
    let reposted = run(&dir.path, &["repost", "--json"], &env);
    served.join().expect("the stand-in ends");
    assert_eq!(reposted.status, REFUSED, "{reposted:#?}");
    assert_eq!(
        lines(&outbox),
        0,
        "repost drops an envelope expired on arrival"
    );
}
