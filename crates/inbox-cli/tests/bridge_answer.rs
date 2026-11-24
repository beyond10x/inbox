//! The cases of story:the-chat-bridge-turns-a-thread-reply-into-an-answer: a thread reply of the
//! allowed chat user to the chat post of a request or a decision is sent as `Answer`, signed with
//! the bridge's key; a reply that names no option, a reply to a note, a reply over the bound of a
//! body, a reply of another chat user and a reply the service refuses send nothing the person did
//! not mean, and the thread says what became of each reply of the allowed chat user.
//!
//! Every case runs against a real `inbox serve` on loopback (the debug build with its clock file),
//! behind the forwarder of `bridge_once`, which keeps what each connection sent and drops one
//! request when a case asks; the stand-in `connectors` of `bridge_once` answers a read of the
//! replies of a post with the replies the case scripts for it. The bridge runs with its own key
//! under the delegation of bob (`support::Signer::Own`) unless a case says otherwise. No case
//! reaches a chat provider or a network beyond loopback, and no case holds a wall-clock time: a
//! case waits on the stand-in's record, the forwarder's record or a fact, with a bound that only
//! ends a hang, and moves a clock by writing its file. alice collects each answer with
//! `inbox collect`.

use crate::adversary_bridge_answer::scripted;
use crate::bridge_once::{
    self, Bridge, CHANNEL, Connectors, Forwarder, Instead, REACTED, client, history, post, posted,
    recorded, reply, restart, state, wait_for,
};
use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use inbox::server::json::{self, Value};
use inbox_server::format_instant;
use support::{BOB, CHAT_BRIDGE, CHAT_BRIDGE_KEY_ID, Service, Signer, World, keys_only, text};

/// The delegation of the bridge for bob.
const D: &str = "chat-bridge-for-bob";
/// The bridge with its own key, for bob.
const OWN: Signer = Signer::Own {
    person: BOB,
    delegation: D,
};
/// The allowed chat user of the binding of `bridge_once`.
const USER: &str = "U0000000001";
/// Another chat user of the channel.
const OTHER_USER: &str = "U0000000002";
/// Another chat channel, of a bridge started with another binding.
const OTHER_CHANNEL: &str = "C0000000002";
/// The start of the head of an `Answer` request.
const ANSWER: &str = "POST /messaging/commands/answer ";

/// The world of a case: the key set with the key file prepared for `signer`, the clock files at
/// the system clock's second, and a service over them that sweeps every second.
fn world(name: &str, signer: Signer) -> World {
    bridge_once::clocked_signed(name, signer)
}

/// [`world`] with a service whose expiry task sweeps once an hour: a message past its `expires`
/// stays open for the length of a case.
fn unswept(name: &str, signer: Signer) -> World {
    let (dir, operator_public_key) = keys_only(name);
    signer.prepare(&dir.path);
    bridge_once::set_clock(&dir.path, "clock", bridge_once::this_second());
    bridge_once::set_clock(&dir.path, "bridge-clock", bridge_once::this_second());
    let service = Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator_public_key,
            "--sweep-interval",
            "3600",
            "--clock-file",
            "clock",
        ],
        &[],
    );
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// Scripts the thread of the post `reference`: the post itself, then `said`, each a reply's
/// reference, its chat user and its text, as the provider lists a thread.
fn thread(connectors: &Connectors, reference: &str, said: &[(&str, &str, &str)]) {
    let mut lines = vec![format!(
        r#"{{"type":"message","user":"U0000000009","text":"a post","ts":"{reference}"}}"#
    )];
    for (ts, user, said) in said {
        let mut quoted = String::new();
        json::push_text(&mut quoted, said);
        lines.push(format!(
            r#"{{"type":"message","user":"{user}","text":{quoted},"thread_ts":"{reference}","ts":"{ts}"}}"#
        ));
    }
    connectors.file(
        &format!("replies-of.{reference}"),
        &format!(
            "{}\n{}\n{}\n",
            r#"{"result":{"result":{"status":200,"body":{"ok":true,"messages":["#,
            lines.join("\n"),
            r#"],"has_more":false}}}}"#
        ),
    );
}

/// The state of `message_id` in alice's `Outbox`, or an empty text when it is not listed.
fn sent_state(world: &World, url: &str, message_id: &str) -> String {
    support::rows(&client(world, url, "alice", &["outbox", "--json"]).json())
        .iter()
        .find(|row| text(row, "message_id") == message_id)
        .map(|row| text(row, "state"))
        .unwrap_or_default()
}

/// The `Answer` envelopes the forwarder carried for `message_id`, as JSON, in their order, also
/// one it dropped.
fn answers(forwarder: &Forwarder, message_id: &str) -> Vec<(String, Value)> {
    let sent = forwarder.sent();
    forwarder
        .commands(0, "answer", message_id)
        .into_iter()
        .map(|at| {
            let request = String::from_utf8_lossy(&sent[at]).into_owned();
            let (head, body) = request.split_once("\r\n\r\n").expect("a head and a body");
            let envelope = json::parse(body).unwrap_or_else(|error| panic!("{error}: {body}"));
            (head.to_lowercase(), envelope)
        })
        .collect()
}

/// The lines the bridge wrote into threads so far: (idempotency key, text), sorted by key.
fn lines(world: &World, connectors: &Connectors) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = (1..=connectors.replies())
        .map(|q| {
            let (said, key, _) = reply(world, connectors, q);
            (key, said)
        })
        .collect();
    found.sort();
    found
}

/// The text of the post of `message_id`, from the stand-in's record of the posts.
fn post_text(world: &World, connectors: &Connectors, message_id: &str) -> String {
    (1..=connectors.posts())
        .map(|p| {
            let input = connectors.input(connectors.invocation("post", p));
            text(input.member("body").expect("a body"), "text")
        })
        .find(|said| said.contains(message_id))
        .unwrap_or_else(|| panic!("no post of {message_id}: {}", world.dir.path.display()))
}

/// alice's `inbox collect` of `message_id`: the settled record it prints.
fn collected(world: &World, url: &str, message_id: &str) -> Value {
    client(world, url, "alice", &["collect", message_id, "--json"])
        .json()
        .member("message")
        .cloned()
        .expect("the settled record")
}

/// The acceptance and checks 2 to 4, with the bridge's own key under the delegation. The post of a
/// decision with the options A and B, B recommended and A taken on silence, shows each option
/// with its letter and text, the recommended one, the option silence takes with `expires`, and how
/// to answer. The person reacts to the request (it is `Read`), and replies `b` to the decision,
/// still `Delivered`, and a text to the request: the bridge sends the `read` of the decision
/// before its `Answer`, then one `Answer` each, `from` the bridge, `on_behalf_of` bob and
/// `authority` the delegation, signed with the bridge's key; the decision moves to `Answered` with
/// option B and the text of B as its body, the request with the text as its body. alice's
/// `inbox collect` gets `answered_by` principal, `decided_option` B and the reply. No line is
/// written into a thread; the record holds both in the state answered.
#[test]
fn a_thread_reply_moves_a_decision_from_read_to_answered_with_the_persons_option() {
    let world = world("answer-acceptance", OWN);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    let expires = format_instant(bridge_once::this_second().plus_seconds(3600));
    let decision = client(
        &world,
        &url,
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--kind",
            "decision",
            "--body",
            "Ship build 7?",
            "--option",
            "A=Ship",
            "--option",
            "B=Hold",
            "--recommended-option",
            "B",
            "--default-on-silence",
            "A",
            "--expires",
            &expires,
        ],
    )
    .stdout
    .trim()
    .to_owned();
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let decision_ts = posted(&world, &url, &decision);
    let request_ts = posted(&world, &url, &request);

    let shown = post_text(&world, &connectors, &decision);
    assert!(
        shown.ends_with(&format!(
            "\n```\nA: Ship\nB: Hold (recommended)\nsilence takes option A at {expires}\n\
             reply in this thread with the letter of an option"
        )),
        "{shown}"
    );

    connectors.file("history", &history(&[(request_ts.as_str(), REACTED)]));
    wait_for("the request is read", || {
        recorded(&world, &request, "state") == "read"
    });
    thread(
        &connectors,
        &decision_ts,
        &[("1700000002.000001", USER, "b")],
    );
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000002", USER, "Build 7 is green.")],
    );
    wait_for("both are answered", || {
        sent_state(&world, &url, &decision) == "Answered"
            && sent_state(&world, &url, &request) == "Answered"
    });
    wait_for("the record holds both answered", || {
        recorded(&world, &decision, "state") == "answered"
            && recorded(&world, &request, "state") == "answered"
    });

    let read = forwarder.commands(0, "read", &decision);
    let answered = forwarder.commands(0, "answer", &decision);
    assert_eq!(answered.len(), 1, "one Answer for the decision");
    assert!(
        read.first().is_some_and(|at| *at < answered[0]),
        "the read of the decision before its Answer: {read:?} {answered:?}"
    );
    for (message_id, option, body) in [
        (&decision, Some("B"), "Hold"),
        (&request, None, "Build 7 is green."),
    ] {
        let sent = answers(&forwarder, message_id);
        assert_eq!(sent.len(), 1, "one Answer for {message_id}");
        let (head, envelope) = &sent[0];
        assert!(
            head.lines()
                .any(|line| line == format!("inbox-key-id: {CHAT_BRIDGE_KEY_ID}")),
            "{head}"
        );
        assert_eq!(text(envelope, "from"), CHAT_BRIDGE);
        assert_eq!(text(envelope, "on_behalf_of"), BOB);
        assert_eq!(text(envelope, "authority"), D);
        assert_eq!(text(envelope, "in_reply_to"), *message_id);
        assert_eq!(
            envelope.member("option").map(|_| text(envelope, "option")),
            option.map(str::to_owned)
        );
        assert_eq!(text(envelope, "body"), body);
        let signature = envelope.member("signature").expect("a signature");
        assert_eq!(text(signature, "key_id"), CHAT_BRIDGE_KEY_ID);

        let message = collected(&world, &url, message_id);
        assert_eq!(text(&message, "answered_by"), "principal", "{message:?}");
        let held = message.member("reply").expect("the reply");
        assert_eq!(text(held, "from"), CHAT_BRIDGE);
        assert_eq!(text(held, "on_behalf_of"), BOB);
        assert_eq!(text(held, "authority"), D);
        assert_eq!(text(held, "body"), body);
        if let Some(option) = option {
            assert_eq!(text(&message, "decided_option"), option);
            assert_eq!(text(held, "option"), option);
        }
    }
    assert_eq!(recorded(&world, &decision, "replied"), "1700000002.000001");
    assert_eq!(connectors.replies(), 0, "no line for a reply taken");
    assert_eq!(bridge.end(), "");
}

/// Decided 2 of the coordinator: a bridge with a key of the person (no `--for`) sends the reply
/// as `inbox answer` without flags does: `from` and `on_behalf_of` the person, no `authority`,
/// signed with the person's key, and no `Inbox-On-Behalf-Of` header.
#[test]
fn a_bridge_with_the_persons_key_answers_as_the_person() {
    let world = world("answer-persons-key", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001", USER, "Build 7.")],
    );
    wait_for("the request is answered", || {
        sent_state(&world, &url, &request) == "Answered"
    });
    let sent = answers(&forwarder, &request);
    assert_eq!(sent.len(), 1);
    let (head, envelope) = &sent[0];
    assert!(
        head.lines().any(|line| line == "inbox-key-id: bob-1"),
        "{head}"
    );
    assert!(!head.contains("inbox-on-behalf-of"), "{head}");
    assert_eq!(text(envelope, "from"), BOB);
    assert_eq!(text(envelope, "on_behalf_of"), BOB);
    assert!(envelope.member("authority").is_none(), "{envelope:?}");
    assert_eq!(text(envelope, "body"), "Build 7.");
    let message = collected(&world, &url, &request);
    assert_eq!(text(&message, "answered_by"), "principal");
    assert_eq!(
        text(message.member("reply").expect("the reply"), "from"),
        BOB
    );
    assert_eq!(bridge.end(), "");
}

/// Checks 5, 6 and 8, and decided 4 and 9 of the coordinator. In the thread of a decision: a
/// reply `B` of another chat user (nothing: no request, no line); a reply of the allowed chat user
/// that names no option, with a direction character, a mention and a line end in it (no request;
/// one line that names the letters A and B and quotes nothing of the reply); a reply `A ` followed
/// by 65535 bytes, 65537 in all (no request; the line "too long"); ` a: ship it ` (taken: option
/// A, the body `ship it`); `B` (no second `Answer`; the line of `already-settled`). In the thread
/// of a note: `A` (no request; the line that a note takes no answer). Each line goes into the
/// thread of its post under the key `inbox-<message id>-reply-<reference of the reply>`. A new
/// bridge sends no second `Answer` and writes no line again.
#[test]
fn a_reply_that_is_not_taken_gets_one_line_and_sends_nothing() {
    let world = world("answer-not-taken", OWN);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    let decision = post(&world, &url, "decision", "Ship build 7?", &[]);
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    let decision_ts = posted(&world, &url, &decision);
    let note_ts = posted(&world, &url, &note);
    let long = format!("A {}", "x".repeat(65535));
    assert_eq!(long.len(), 65537);
    thread(
        &connectors,
        &decision_ts,
        &[
            ("1700000002.000001", OTHER_USER, "B"),
            (
                "1700000002.000002",
                USER,
                "maybe \u{202e}<!here>\nA: forged line",
            ),
            ("1700000002.000003", USER, &long),
            ("1700000002.000004", USER, " a: ship it "),
            ("1700000002.000005", USER, "B"),
        ],
    );
    thread(&connectors, &note_ts, &[("1700000002.000006", USER, "A")]);
    wait_for("4 lines in the threads", || connectors.replies() >= 4);
    wait_for("the decision is answered", || {
        sent_state(&world, &url, &decision) == "Answered"
    });
    let key = |message_id: &str, ts: &str| format!("inbox-{message_id}-reply-{ts}");
    let mut expected = vec![
        (
            key(&decision, "1700000002.000002"),
            "the reply was not taken: it names no option; reply with one of the letters A, B"
                .to_owned(),
        ),
        (
            key(&decision, "1700000002.000003"),
            "the reply was not taken: it is longer than 65536 bytes".to_owned(),
        ),
        (
            key(&decision, "1700000002.000005"),
            "the reply was not taken: the service refused it: already-settled".to_owned(),
        ),
        (
            key(&note, "1700000002.000006"),
            "the reply was not taken: a note takes no answer".to_owned(),
        ),
    ];
    expected.sort();
    assert_eq!(lines(&world, &connectors), expected);
    for q in 1..=4 {
        let (_, key, message_id) = reply(&world, &connectors, q);
        assert!(
            key.starts_with(&format!("inbox-{message_id}-reply-")),
            "{key}"
        );
    }
    assert_eq!(bridge.end(), "");

    let reads = connectors.thread_reads();
    let again = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    wait_for("2 polls of the new bridge read both threads", || {
        connectors.thread_reads() >= reads + 4
    });
    assert_eq!(again.end(), "");
    assert_eq!(lines(&world, &connectors), expected, "no line again");

    let sent = answers(&forwarder, &decision);
    assert_eq!(sent.len(), 1, "one Answer for the decision");
    assert_eq!(text(&sent[0].1, "option"), "A");
    assert_eq!(text(&sent[0].1, "body"), "ship it");
    for command in ["answer", "read"] {
        assert!(
            forwarder.commands(0, command, &note).is_empty(),
            "the bridge sent `{command}` for the note"
        );
    }
    assert_eq!(state(&world, &url, &note), "Delivered");
    let message = collected(&world, &url, &decision);
    assert_eq!(text(&message, "decided_option"), "A");
    assert_eq!(
        text(message.member("reply").expect("the reply"), "body"),
        "ship it"
    );
}

/// Check 7. An `Answer` lost in transport (the forwarder drops it) fails the poll, gets no line
/// in the thread, and is sent again after the pause of a failure: the request is answered. A
/// reply to a decision after its `expires` (the service's clock moved past it, its expiry task
/// not yet run) is refused `deadline-passed`, and a reply of 2 lines to a request of the shape
/// `lines=1` is refused `shape-mismatch`: each gets one line that names the refusal and says the
/// reply was not taken, neither is sent again, and the decision is left to silence.
#[test]
fn a_refused_answer_gets_one_line_and_a_lost_one_is_sent_again() {
    let world = unswept("answer-refused", OWN);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    let expires_at = bridge_once::this_second().plus_seconds(30);
    let expires = format_instant(expires_at);
    let decision = post(
        &world,
        &url,
        "decision",
        "Ship build 7?",
        &["--expires", &expires],
    );
    let shaped = client(
        &world,
        &url,
        "alice",
        &[
            "post",
            "--to",
            BOB,
            "--kind",
            "request",
            "--body",
            "Which build, in one line?",
            "--reply-shape",
            "lines=1",
        ],
    )
    .stdout
    .trim()
    .to_owned();
    let lost = post(&world, &url, "request", "Which build is green?", &[]);
    let decision_ts = posted(&world, &url, &decision);
    let shaped_ts = posted(&world, &url, &shaped);
    let lost_ts = posted(&world, &url, &lost);

    forwarder.drop_next(ANSWER);
    thread(
        &connectors,
        &lost_ts,
        &[("1700000002.000001", USER, "Build 7 is green.")],
    );
    wait_for("the first Answer is dropped", || !forwarder.dropping());
    wait_for("the request is answered", || {
        sent_state(&world, &url, &lost) == "Answered"
    });
    assert_eq!(
        answers(&forwarder, &lost).len(),
        2,
        "the Answer lost and the one sent again"
    );
    assert_eq!(connectors.replies(), 0, "no line for an Answer lost");

    bridge_once::set_clock(&world.dir.path, "clock", expires_at.plus_seconds(1));
    thread(
        &connectors,
        &decision_ts,
        &[("1700000002.000002", USER, "A")],
    );
    thread(
        &connectors,
        &shaped_ts,
        &[("1700000002.000003", USER, "Build 7\nBuild 8")],
    );
    wait_for("2 lines in the threads", || connectors.replies() >= 2);
    let reads = connectors.thread_reads();
    wait_for("2 more polls of the 3 threads", || {
        connectors.thread_reads() >= reads + 6
    });
    let mut expected = vec![
        (
            format!("inbox-{decision}-reply-1700000002.000002"),
            "the reply was not taken: the service refused it: deadline-passed".to_owned(),
        ),
        (
            format!("inbox-{shaped}-reply-1700000002.000003"),
            "the reply was not taken: the service refused it: shape-mismatch".to_owned(),
        ),
    ];
    expected.sort();
    assert_eq!(lines(&world, &connectors), expected);
    assert_eq!(answers(&forwarder, &decision).len(), 1, "not sent again");
    assert_eq!(answers(&forwarder, &shaped).len(), 1, "not sent again");
    assert_ne!(sent_state(&world, &url, &decision), "Answered");
    assert_ne!(sent_state(&world, &url, &shaped), "Answered");
    let said = bridge.end();
    assert_eq!(said.lines().count(), 1, "{said}");
    assert!(said.contains(&lost), "{said}");
}

/// A full page of replies of another chat user (the stand-in lists at most `limit` messages after
/// `oldest`, as the provider lists a page): the next read starts after the last of them, so the
/// person's reply after the page is read at the next poll and sent; the replies of the other
/// chat user get no line and send nothing.
#[test]
fn a_reply_after_a_full_page_of_other_users_is_read_at_the_next_poll() {
    let world = world("answer-full-page", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    let others: Vec<String> = (1..=100).map(|k| format!("1700000002.{k:06}")).collect();
    let mut said: Vec<(&str, &str, &str)> = others
        .iter()
        .map(|ts| (ts.as_str(), OTHER_USER, "B"))
        .collect();
    said.push(("1700000003.000001", USER, "Build 7 is green."));
    thread(&connectors, &request_ts, &said);
    wait_for("the request is answered", || {
        sent_state(&world, &url, &request) == "Answered"
    });
    // The service shows the request answered before the bridge has its answer and records it.
    wait_for("the record holds the request answered", || {
        recorded(&world, &request, "state") == "answered"
    });
    let sent = answers(&forwarder, &request);
    assert_eq!(sent.len(), 1, "one Answer");
    assert_eq!(text(&sent[0].1, "body"), "Build 7 is green.");
    assert_eq!(connectors.replies(), 0, "no line for another chat user");
    assert_eq!(recorded(&world, &request, "read_to"), "1700000003.000001");
}

/// An `Answer` the service took whose answer was lost on the way back (the forwarder carries it
/// and throws the service's answer away). The poll fails; the next one finds the message out of
/// `Pending`, its row in `Answered`, and gives the entry the end `answered`: one `Answer`, no line
/// in the thread, the reply as the person wrote it. Standard error holds one line, the `Answer`'s
/// own: whether the service took it is not known (no line of a failed read of the reactions).
#[test]
fn an_answer_taken_whose_answer_was_lost_ends_the_entry_with_no_line() {
    let world = world("answer-lost-answer", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    forwarder.next(ANSWER, Instead::AnswerLost);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001", USER, "Build 7 is green.")],
    );
    wait_for("the answer of the Answer is lost", || !forwarder.dropping());
    wait_for("the entry ends answered", || {
        recorded(&world, &request, "end") == "answered"
    });
    assert_eq!(answers(&forwarder, &request).len(), 1, "one Answer");
    assert_eq!(connectors.replies(), 0, "no line in the thread");
    let message = collected(&world, &url, &request);
    assert_eq!(
        text(message.member("reply").expect("the reply"), "body"),
        "Build 7 is green."
    );
    let said = bridge.end();
    let lines: Vec<&str> = said.lines().collect();
    assert_eq!(lines.len(), 1, "{said}");
    assert!(
        lines[0].starts_with(&format!(
            "inbox bridge: the answer to {request} from the thread of its post got no answer of \
             the service"
        )),
        "{said}"
    );
    assert!(
        lines[0].contains(
            "whether the service took it is not known; the next poll sends it again unless the \
             message has left `Pending` by then"
        ),
        "{said}"
    );
}

/// An `Answer` the service answers `408` (it did not read the request in time; nothing reached
/// the model) gets no line and is sent again after the pause of a failure: the request is
/// answered with the reply.
#[test]
fn an_answer_answered_408_gets_no_line_and_is_sent_again() {
    let world = world("answer-408", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    forwarder.next(ANSWER, Instead::TimedOut);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001", USER, "Build 7 is green.")],
    );
    wait_for("the first Answer is answered 408", || !forwarder.dropping());
    wait_for("the request is answered", || {
        sent_state(&world, &url, &request) == "Answered"
    });
    assert_eq!(
        answers(&forwarder, &request).len(),
        2,
        "the Answer answered 408 and the one sent again"
    );
    assert_eq!(connectors.replies(), 0, "no line for a 408");
}

/// An entry of the record of another channel (posted by a bridge bound to it before) is not
/// read: a bridge bound to the channel of the binding reads only the thread of its own post, and
/// a reply of the allowed chat user in the thread of the other post sends nothing.
#[test]
fn an_entry_of_another_channel_is_not_read() {
    let world = world("answer-other-channel", Signer::Person);
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let first = Bridge::signed(
        &world,
        &connectors,
        &url,
        OTHER_CHANNEL,
        &[],
        Signer::Person,
    );
    let elsewhere = post(&world, &url, "request", "Which build is red?", &[]);
    let elsewhere_ts = posted(&world, &url, &elsewhere);
    assert_eq!(first.end(), "");
    assert_eq!(recorded(&world, &elsewhere, "channel"), OTHER_CHANNEL);
    thread(
        &connectors,
        &elsewhere_ts,
        &[("1700000002.000001", USER, "Build 6 is red.")],
    );
    let before = connectors.thread_reads();
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000002", USER, "Build 7 is green.")],
    );
    wait_for("the request is answered", || {
        state(&world, &url, &request).is_empty()
    });
    let reads = connectors.thread_reads();
    wait_for("2 more reads of a thread", || {
        connectors.thread_reads() >= reads + 2
    });
    let read: Vec<String> = (before + 1..=connectors.thread_reads())
        .map(|k| {
            text(
                &connectors.input(connectors.invocation("thread-read", k)),
                "ts",
            )
        })
        .collect();
    assert!(
        read.iter().all(|ts| *ts == request_ts),
        "only the thread of the post in the bound channel: {read:?}"
    );
    assert_eq!(state(&world, &url, &elsewhere), "Delivered");
    assert_eq!(bridge.end(), "");
}

/// A repost of a message whose answer the bridge took from the thread (a new start of the
/// service lost the answer; the sender's next command posts the message again) makes the entry
/// `read` again: the bridge marks it read after `MarkDelivered`, and the person's next reply is
/// sent as the answer.
#[test]
fn a_repost_of_an_answered_message_takes_a_later_reply_again() {
    let mut world = world("answer-repost", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001", USER, "Build 7 is green.")],
    );
    wait_for("the entry is answered", || {
        recorded(&world, &request, "state") == "answered"
    });
    restart(&mut world, &forwarder);
    wait_for("the bridge finds the answered message gone", || {
        recorded(&world, &request, "end") == "answered"
    });
    // alice's `outbox` posts it again (the reconnect after its answer); the next one lists it, in
    // whatever state the bridge has moved it to meanwhile (bob's `Pending` no longer lists it once
    // it is `Read`).
    wait_for("alice's next command posts it again", || {
        !sent_state(&world, &url, &request).is_empty()
    });
    wait_for("the repost is read again", || {
        recorded(&world, &request, "state") == "read"
            && recorded(&world, &request, "end").is_empty()
            && sent_state(&world, &url, &request) == "Read"
    });
    thread(
        &connectors,
        &request_ts,
        &[
            ("1700000002.000001", USER, "Build 7 is green."),
            ("1700000002.000002", USER, "Build 8 is green."),
        ],
    );
    wait_for("the request is answered again", || {
        sent_state(&world, &url, &request) == "Answered"
    });
    let message = collected(&world, &url, &request);
    assert_eq!(
        text(message.member("reply").expect("the reply"), "body"),
        "Build 8 is green."
    );
    assert_eq!(connectors.replies(), 0, "no line");
}

/// The bridge's own line in a thread, listed by the provider as a reply of the allowed chat user
/// (a connection of `connectors` that posts as that user), with the reference the post of the
/// line answered: it sends nothing and gets no line. The person's reply after it is the answer.
#[test]
fn the_bridges_own_line_read_back_as_the_persons_reply_is_not_sent() {
    let world = world("answer-own-line", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    let long = "x".repeat(65537);
    let first = ("1700000000.900000", USER, long.as_str());
    thread(&connectors, &request_ts, &[first]);
    wait_for("the line about the long reply", || {
        connectors.replies() >= 1
    });
    let (said, _, _) = reply(&world, &connectors, 1);
    assert_eq!(
        said,
        "the reply was not taken: it is longer than 65536 bytes"
    );
    // The reference the stand-in answered the post of the line with.
    let own = "1700000001.000001";
    thread(
        &connectors,
        &request_ts,
        &[
            first,
            (own, USER, said.as_str()),
            ("1700000002.000001", USER, "Build 7 is green."),
        ],
    );
    wait_for("the request is answered", || {
        sent_state(&world, &url, &request) == "Answered"
    });
    let sent = answers(&forwarder, &request);
    assert_eq!(sent.len(), 1, "one Answer");
    assert_eq!(text(&sent[0].1, "body"), "Build 7 is green.");
    // The read of the next poll: the poll that sent the Answer has written its lines.
    let reads = connectors.thread_reads();
    wait_for("1 more read of the thread", || {
        connectors.thread_reads() > reads
    });
    assert_eq!(
        connectors.replies(),
        1,
        "no line about the bridge's own line"
    );
}

/// The `ts` of the post of each read of the replies so far, in their order.
fn thread_posts(connectors: &Connectors) -> Vec<String> {
    (1..=connectors.thread_reads())
        .map(|k| {
            text(
                &connectors.input(connectors.invocation("thread-read", k)),
                "ts",
            )
        })
        .collect()
}

/// Each call of the stand-in so far, in their order: its operation and, for a read of the
/// replies, the `ts` of the post it read (else an empty text).
fn calls(world: &World, connectors: &Connectors) -> Vec<(String, String)> {
    (1..)
        .take_while(|n| world.dir.join(&format!("connectors/args.{n}")).exists())
        .map(|n| {
            let operation = connectors.args(n).get(4).cloned().unwrap_or_default();
            let ts = if operation == "conversations.replies" {
                text(&connectors.input(n), "ts")
            } else {
                String::new()
            };
            (operation, ts)
        })
        .collect()
}

/// An `Answer` without an answer (the forwarder drops it) fails its poll: the thread of the
/// post after it is not read in that poll, but in a later one. Each poll starts with the read of
/// the channel (the second post is `posted`), so the calls of the stand-in fall into polls; a poll
/// that read the first thread and not the second is the one that failed. A poll that went on
/// after the failure would read both, as every other poll does.
#[test]
fn an_answer_without_an_answer_fails_its_poll_and_the_posts_after_it_wait_for_the_next() {
    let world = world("answer-lost-fails-poll", Signer::Person);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let first = post(&world, &url, "request", "Which build is green?", &[]);
    let second = post(&world, &url, "request", "Which build is red?", &[]);
    let first_ts = posted(&world, &url, &first);
    let second_ts = posted(&world, &url, &second);
    wait_for("a poll reads both threads, the second last", || {
        thread_posts(&connectors).last() == Some(&second_ts)
    });
    let from = calls(&world, &connectors).len();
    forwarder.drop_next(ANSWER);
    thread(
        &connectors,
        &first_ts,
        &[("1700000002.000001", USER, "Build 7 is green.")],
    );
    wait_for("the first Answer is dropped", || !forwarder.dropping());
    wait_for("the request is answered", || {
        sent_state(&world, &url, &first) == "Answered"
    });
    let answered = connectors.thread_reads();
    wait_for("a read of the second thread after it", || {
        thread_posts(&connectors)
            .iter()
            .skip(answered)
            .any(|ts| *ts == second_ts)
    });
    assert_eq!(answers(&forwarder, &first).len(), 2, "dropped, sent again");
    let mut polls: Vec<Vec<String>> = Vec::new();
    for (operation, ts) in calls(&world, &connectors).split_off(from) {
        if operation == "conversations.history" {
            polls.push(Vec::new());
        } else if let Some(poll) = polls.last_mut()
            && !ts.is_empty()
        {
            poll.push(ts);
        }
    }
    // The last poll may not be at its end yet.
    polls.pop();
    assert!(
        polls
            .iter()
            .any(|poll| poll.contains(&first_ts) && !poll.contains(&second_ts)),
        "the poll whose Answer got no answer read the thread of the post after it: {polls:?}"
    );
}

/// Takes the member `channel` out of the line of `message_id` in bob's record of the chat posts,
/// as a build before the member existed wrote it. The record is replaced whole with mode 0600, as
/// the bridge replaces it.
fn without_channel(world: &World, message_id: &str) {
    let path = fs::read_dir(&world.dir.path)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("bob.outbox.chat.") && !name.ends_with(".new")
        })
        .expect("the record is written");
    let record = fs::read_to_string(&path).expect("the record is read");
    let member = format!(r#","channel":"{CHANNEL}""#);
    let lines: Vec<String> = record
        .lines()
        .map(|line| {
            if line.contains(&format!(r#""message_id":"{message_id}""#)) {
                assert!(line.contains(&member), "{line}");
                line.replacen(&member, "", 1)
            } else {
                line.to_owned()
            }
        })
        .collect();
    let staged = world.dir.join("record.staged");
    fs::write(&staged, format!("{}\n", lines.join("\n"))).expect("the record is written");
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600)).expect("mode 0600");
    fs::rename(&staged, &path).expect("the record is replaced");
}

/// An entry of the record that names no channel (written by a build before the member existed)
/// counts as a post of the bound channel: a new bridge reads its thread and sends the reply of the
/// allowed chat user as the answer.
#[test]
fn an_entry_that_names_no_channel_is_read_as_a_post_of_the_bound_channel() {
    let world = world("answer-no-channel", Signer::Person);
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let first = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    assert_eq!(first.end(), "");
    without_channel(&world, &request);
    assert_eq!(recorded(&world, &request, "channel"), "");
    thread(
        &connectors,
        &request_ts,
        &[("1700000002.000001", USER, "Build 7 is green.")],
    );
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    wait_for("the record holds the request answered", || {
        recorded(&world, &request, "state") == "answered"
    });
    assert!(state(&world, &url, &request).is_empty(), "left `Pending`");
    assert_eq!(bridge.end(), "");
}

/// A read of the replies refused `rate_limited` with `retry_after_seconds` 2 holds the poll for
/// that wait: the next call of `connectors` starts not before 2 s after the refusal. A lower
/// bound of time, which a slow host cannot break; the times are the system clock the stand-in
/// read when it wrote the refusal and when the next call started.
#[test]
fn a_read_of_the_replies_refused_with_a_wait_holds_the_next_call_for_that_wait() {
    let world = world("answer-named-wait", Signer::Person);
    let url = world.service.url.clone();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let dir = scripted(&connectors);
    let _bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let request_ts = posted(&world, &url, &request);
    fs::write(dir.join(format!("limit.{request_ts}")), "2").expect("the limit is armed");
    wait_for("the next call after the refusal", || {
        dir.join("after-limit").exists()
    });
    let at = |name: &str| -> u128 {
        let read = fs::read_to_string(dir.join(name)).expect("a time is recorded");
        read.trim()
            .parse()
            .unwrap_or_else(|error| panic!("{error}: {read:?}"))
    };
    let waited = at("after-limit").saturating_sub(at("limited-at"));
    assert!(
        waited >= 2_000_000_000,
        "the call after the read refused with a wait of 2 s came {waited} ns after it"
    );
}
