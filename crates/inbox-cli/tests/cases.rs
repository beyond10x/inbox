//! The cases story:cli-client owes beside its acceptance: each client command against a running
//! `inbox serve`, refusals and their exit statuses, the outbox and `repost` across a restart, the
//! key commands, and how `inbox serve` starts, refuses to start and stops.

use crate::support;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use inbox::messaging::{DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_verify::{Instant, KeyRing, PublicKey};

use support::{
    ALICE, BOB, Dir, LOCAL, REFUSED, TRANSPORT, USAGE, World, ids, key_new, keys_only, rows, run,
    serve, serve_refused, stand_in, text,
};

fn member<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
    path.iter().fold(value, |at, name| {
        at.member(name)
            .unwrap_or_else(|| panic!("no `{name}` in {at:?}"))
    })
}

fn outcome(value: &Value) -> String {
    text(member(value, &["answer"]), "outcome")
}

fn items(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.clone(),
        other => panic!("not an array: {other:?}"),
    }
}

#[test]
fn each_client_command_works_against_a_running_service() {
    let world = World::new("commands");
    let opened = world.client("bob", &["open"]).ok();
    assert_eq!(opened.stdout.trim(), format!("opened {BOB}"));

    // A request, read, answered, and the answer collected.
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
    let request = text(&posted, "message_id");
    assert_eq!(
        text(&posted, "thread"),
        request,
        "a message sent alone is its own thread"
    );
    assert_eq!(outcome(&posted), "posted");
    assert_eq!(world.kept("alice"), std::slice::from_ref(&request));

    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&request));
    assert_eq!(text(&rows(&pending)[0], "kind"), "request");
    let read = world
        .client("bob", &["read", &request, "--json"])
        .ok()
        .json();
    assert_eq!(text(&read, "outcome"), "read");
    let answered = world
        .client("bob", &["answer", &request, "--body", "Blue", "--json"])
        .ok()
        .json();
    assert_eq!(outcome(&answered), "answered");

    let outbox = world.client("alice", &["outbox", "--json"]).ok().json();
    assert_eq!(ids(&outbox), std::slice::from_ref(&request));
    assert_eq!(text(&rows(&outbox)[0], "state"), "Answered");
    assert_eq!(text(member(&rows(&outbox)[0], &["reply"]), "body"), "Blue");
    let thread = world
        .client("alice", &["thread", &request, "--json"])
        .ok()
        .json();
    assert_eq!(ids(&thread), std::slice::from_ref(&request));
    let replies = world
        .client("alice", &["replies", &request, "--json"])
        .ok()
        .json();
    assert_eq!(ids(&replies), std::slice::from_ref(&request));

    let collected = world
        .client("alice", &["collect", &request, "--json"])
        .ok()
        .json();
    assert_eq!(outcome(&collected), "collected");
    assert_eq!(
        text(member(&collected, &["message", "reply"]), "body"),
        "Blue"
    );
    assert!(
        world.kept("alice").is_empty(),
        "a collected message leaves the outbox file"
    );

    // A decision, refused, and the refusal collected.
    let refused_decision = text(
        &world
            .client(
                "alice",
                &[
                    "post",
                    "--to",
                    BOB,
                    "--kind",
                    "decision",
                    "--option",
                    "A=Red",
                    "--option",
                    "B=Blue",
                    "--default-on-silence",
                    "A",
                    "--body",
                    "Pick one",
                    "--json",
                ],
            )
            .ok()
            .json(),
        "message_id",
    );
    let refused = world
        .client(
            "bob",
            &[
                "refuse",
                &refused_decision,
                "--reason",
                "not mine to pick",
                "--json",
            ],
        )
        .ok()
        .json();
    assert_eq!(text(&refused, "outcome"), "refused");
    let outbox = world.client("alice", &["outbox", "--json"]).ok().json();
    assert_eq!(text(&rows(&outbox)[0], "state"), "Refused");
    assert_eq!(text(&rows(&outbox)[0], "refusal"), "not mine to pick");
    let collected = world.client("alice", &["collect", &refused_decision]).ok();
    assert!(
        collected.stdout.contains("not mine to pick"),
        "{collected:#?}"
    );
    assert!(world.kept("alice").is_empty());

    // A decision answered with an option, in a thread of the sender's choice.
    let thread_id = text(
        &world
            .client("alice", &["post", "--to", BOB, "--body", "x", "--json"])
            .ok()
            .json(),
        "message_id",
    );
    let decision = text(
        &world
            .client(
                "alice",
                &[
                    "post",
                    "--to",
                    BOB,
                    "--kind",
                    "decision",
                    "--thread",
                    &thread_id,
                    "--option",
                    "A=Red",
                    "--option",
                    "B=Blue",
                    "--recommended-option",
                    "B",
                    "--body",
                    "Pick one",
                    "--json",
                ],
            )
            .ok()
            .json(),
        "message_id",
    );
    world
        .client(
            "bob",
            &["answer", &decision, "--option", "B", "--body", "B it is"],
        )
        .ok();
    let replies = world
        .client("alice", &["replies", &thread_id, "--json"])
        .ok()
        .json();
    assert_eq!(ids(&replies), std::slice::from_ref(&decision));
    assert_eq!(text(&rows(&replies)[0], "decided_option"), "B");
    let thread = world
        .client("alice", &["thread", &thread_id, "--json"])
        .ok()
        .json();
    assert_eq!(ids(&thread).len(), 2);

    // Closed: a post to the inbox is refused.
    let closed = world.client("bob", &["close", "--json"]).ok().json();
    assert_eq!(text(&closed, "outcome"), "closed");
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "late"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert!(
        refused.stderr.contains("inbox.messaging.UnknownInbox"),
        "{refused:#?}"
    );
}

#[test]
fn wait_is_answered_by_a_post_that_arrives_during_it() {
    let world = World::new("wait");
    world.client("bob", &["open"]).ok();
    let mut waiting = world.spawn("bob", &["wait", "--seconds", "15", "--json"]);
    assert!(waiting.running(), "the wait holds until a message arrives");
    let posted = world
        .client("alice", &["post", "--to", BOB, "--body", "wake up"])
        .ok();
    let woken = waiting.finish().ok().json();
    assert_eq!(ids(&woken), [posted.stdout.trim().to_owned()]);
    assert_eq!(text(&rows(&woken)[0], "body"), "wake up");
}

#[test]
fn a_refusal_exits_1_and_names_the_error() {
    let world = World::new("refusals");

    // A declared refusal: the model's error name.
    let refused = world.client("alice", &["post", "--to", BOB, "--body", "hi", "--json"]);
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    let document = refused.json();
    assert_eq!(text(&document, "error"), "inbox.messaging.UnknownInbox");
    assert_eq!(
        text(member(&document, &["answer"]), "outcome"),
        "unknown-inbox"
    );
    assert!(
        refused
            .stderr
            .contains("refused: inbox.messaging.UnknownInbox")
    );
    assert_eq!(
        world.kept("alice").len(),
        1,
        "an inbox not open yet keeps the envelope"
    );

    let shapeless = world.client(
        "alice",
        &["post", "--to", ALICE, "--kind", "request", "--body", "?"],
    );
    assert_eq!(shapeless.status, REFUSED, "{shapeless:#?}");
    assert!(
        shapeless.stderr.contains("inbox.messaging.ShapeMismatch"),
        "{shapeless:#?}"
    );
    assert_eq!(
        world.kept("alice").len(),
        1,
        "any other declared refusal drops it"
    );

    // The service's own refusal: its `refused` text.
    let carol = key_new(
        &world.dir.path,
        "carol.key",
        Some(("carol-1", "agent:test/carol")),
    );
    assert_eq!(carol.len(), 44);
    let unknown = world.client("carol", &["pending", "--json"]);
    assert_eq!(unknown.status, REFUSED, "{unknown:#?}");
    assert_eq!(text(&unknown.json(), "error"), "unauthenticated");
    assert_eq!(
        unknown.json().member("status"),
        Some(&Value::Number("401".to_owned()))
    );

    // A transport failure and wrong usage.
    let unreachable = world.client_env(
        "alice",
        &["pending", "--json"],
        &[("INBOX_URL", "http://127.0.0.1:1")],
    );
    assert_eq!(unreachable.status, TRANSPORT, "{unreachable:#?}");
    assert_eq!(text(&unreachable.json(), "error"), "transport");
    let usage = world.client("alice", &["post", "--body", "no recipient", "--json"]);
    assert_eq!(usage.status, USAGE, "{usage:#?}");
    assert_eq!(text(&usage.json(), "error"), "usage");
    let no_url = world.client_env("alice", &["pending"], &[("INBOX_URL", "")]);
    assert_eq!(no_url.status, USAGE, "{no_url:#?}");
}

#[test]
fn repost_after_a_restart_puts_the_message_in_pending_once() {
    let mut world = World::new("repost");
    world.client("bob", &["open"]).ok();
    let message_id = world
        .client("alice", &["post", "--to", BOB, "--body", "survive this"])
        .ok()
        .stdout
        .trim()
        .to_owned();
    let same = world.client("alice", &["repost", "--json"]).ok().json();
    assert!(
        items(member(&same, &["reposted"])).is_empty(),
        "the start the post arrived at still holds it: nothing to repost; {same:?}"
    );

    world.restart();
    let lost = world.client("bob", &["pending", "--json"]).ok().json();
    assert!(ids(&lost).is_empty(), "a restart lost the message");
    world.client("bob", &["open"]).ok();

    // The first repost after the restart delivers the message again and records the new start;
    // the second, in the same start, has nothing to post.
    let reposted = world.client("alice", &["repost", "--json"]).ok().json();
    let entries = items(member(&reposted, &["reposted"]));
    assert_eq!(entries.len(), 1, "{reposted:?}");
    assert_eq!(text(&entries[0], "message_id"), message_id);
    assert_eq!(outcome(&entries[0]), "posted");
    assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(true)));
    let again = world.client("alice", &["repost"]).ok();
    assert_eq!(again.stdout.trim(), "nothing to repost", "{again:#?}");
    let pending = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(ids(&pending), std::slice::from_ref(&message_id));
    assert_eq!(world.kept("alice"), [message_id]);
}

#[test]
fn two_processes_posting_at_once_lose_no_outbox_entry() {
    let world = World::new("concurrent");
    world.client("bob", &["open"]).ok();
    let posts: Vec<_> = (0..8)
        .map(|n| world.spawn("alice", &["post", "--to", BOB, "--body", &format!("n{n}")]))
        .collect();
    let mut posted: Vec<String> = posts
        .into_iter()
        .map(|post| post.finish().ok().stdout.trim().to_owned())
        .collect();
    let mut kept = world.kept("alice");
    posted.sort();
    kept.sort();
    assert_eq!(kept, posted);
}

#[test]
fn key_new_writes_a_0600_file_never_overwrites_it_and_prints_nothing_secret() {
    let dir = Dir::new("key-new");
    let made = run(
        &dir.path,
        &[
            "key",
            "new",
            "--out",
            "k.key",
            "--key-id",
            "k-1",
            "--principal",
            "agent:test/k",
        ],
        &[],
    )
    .ok();
    let file = dir.join("k.key");
    assert_eq!(
        fs::metadata(&file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let stored = fs::read_to_string(&file).unwrap();
    let value = json::parse(&stored).unwrap();
    let seed = text(&value, "seed");
    assert!(!made.stdout.contains(&seed) && !made.stderr.contains(&seed));
    assert_eq!(made.stdout.trim(), text(&value, "public_key"));
    assert!(PublicKey::from_base64(made.stdout.trim()).is_ok());
    assert_eq!(text(&value, "key_id"), "k-1");
    assert_eq!(text(&value, "principal"), "agent:test/k");

    let again = run(&dir.path, &["key", "new", "--out", "k.key", "--json"], &[]);
    assert_eq!(again.status, LOCAL, "{again:#?}");
    assert!(again.stderr.contains("exists"), "{again:#?}");
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        stored,
        "the file is unchanged"
    );

    let shown = run(&dir.path, &["key", "new", "--out", "j.key", "--json"], &[]).ok();
    let document = shown.json();
    assert!(document.member("public_key").is_some());
    let other_seed = text(
        &json::parse(&fs::read_to_string(dir.join("j.key")).unwrap()).unwrap(),
        "seed",
    );
    assert!(!shown.stdout.contains(&other_seed));

    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    let widened = run(
        &dir.path,
        &[
            "pending",
            "--secret-key-file",
            "k.key",
            "--url",
            "http://127.0.0.1:1",
        ],
        &[],
    );
    assert_eq!(widened.status, LOCAL, "{widened:#?}");
    assert!(widened.stderr.contains("0600"), "{widened:#?}");
}

#[test]
fn the_keyfile_commands_write_a_file_the_ring_accepts_and_the_service_refuses_a_wrong_operator_key()
{
    let (dir, operator) = keys_only("keyfile");
    run(
        &dir.path,
        &[
            "keyfile",
            "add-delegation",
            "--file",
            "keys.json",
            "--operator-secret-key-file",
            "operator.key",
            "--delegation-id",
            "d-1",
            "--agent",
            ALICE,
            "--on-behalf-of",
            "human:test/owner",
            "--valid-until",
            "2099-01-01T00:00:00Z",
        ],
        &[],
    )
    .ok();
    let text_of_file = fs::read_to_string(dir.join("keys.json")).unwrap();
    let ring = KeyRing::parse(&text_of_file, &PublicKey::from_base64(&operator).unwrap()).unwrap();
    assert!(ring.ignored().is_empty(), "{:?}", ring.ignored());
    let now = Instant::from_system_time(std::time::SystemTime::now());
    assert_eq!(
        ring.principal("alice-1", now).unwrap(),
        &Principal(ALICE.to_owned())
    );
    assert_eq!(
        ring.principal("bob-1", now).unwrap(),
        &Principal(BOB.to_owned())
    );
    ring.check_delegation(
        &DelegationId("d-1".to_owned()),
        &Principal(ALICE.to_owned()),
        &Principal("human:test/owner".to_owned()),
        now,
    )
    .unwrap();

    let reused = run(
        &dir.path,
        &[
            "keyfile",
            "add-key",
            "--file",
            "keys.json",
            "--operator-secret-key-file",
            "operator.key",
            "--key-id",
            "alice-1",
            "--principal",
            ALICE,
            "--public-key",
            &operator,
        ],
        &[],
    );
    assert_eq!(reused.status, LOCAL, "{reused:#?}");
    assert!(reused.stderr.contains("never reused"), "{reused:#?}");
    assert_eq!(
        fs::read_to_string(dir.join("keys.json")).unwrap(),
        text_of_file
    );

    // The service over the right operator key honours the delegation.
    let service = serve(&dir.path, &operator);
    let env = [("INBOX_URL", service.url.as_str())];
    run(&dir.path, &["open", "--secret-key-file", "bob.key"], &env).ok();
    let spoken = run(
        &dir.path,
        &[
            "post",
            "--secret-key-file",
            "alice.key",
            "--to",
            BOB,
            "--on-behalf-of",
            "human:test/owner",
            "--authority",
            "d-1",
            "--body",
            "for the owner",
            "--json",
        ],
        &env,
    )
    .ok()
    .json();
    assert_eq!(outcome(&spoken), "posted");
    drop(service);

    // Over another operator key no entry counts: every request is unauthenticated.
    let stranger = key_new(&dir.path, "stranger.key", None);
    let service = serve(&dir.path, &stranger);
    assert!(
        service.stderr().contains("alice-1"),
        "the ignored entries are named"
    );
    let refused = run(
        &dir.path,
        &["open", "--secret-key-file", "bob.key", "--json"],
        &[("INBOX_URL", service.url.as_str())],
    );
    assert_eq!(refused.status, REFUSED, "{refused:#?}");
    assert_eq!(text(&refused.json(), "error"), "unauthenticated");
}

#[test]
fn serve_refuses_a_non_loopback_address_an_unreadable_key_file_and_another_backend() {
    let (dir, operator) = keys_only("serve-refusals");
    let base = [
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator.as_str(),
    ];

    let mut args = vec!["--listen", "0.0.0.0:0"];
    args.extend(base);
    let outside = serve_refused(&dir.path, &args, &[]);
    assert_eq!(outside.status, USAGE, "{outside:#?}");
    assert!(outside.stderr.contains("loopback"), "{outside:#?}");

    let missing = serve_refused(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "missing.json",
            "--operator-public-key",
            &operator,
        ],
        &[],
    );
    assert_eq!(missing.status, LOCAL, "{missing:#?}");
    assert!(missing.stderr.contains("missing.json"), "{missing:#?}");

    let mut args = vec!["--listen", "127.0.0.1:0"];
    args.extend(base);
    let nats = serve_refused(&dir.path, &args, &[("INBOX_BACKEND", "nats")]);
    assert_eq!(nats.status, USAGE, "{nats:#?}");
    assert!(
        nats.stderr.contains("INBOX_BACKEND") && nats.stderr.contains("nats"),
        "{nats:#?}"
    );

    let mut args = vec!["--listen", "127.0.0.1:0", "--clock-start", "tomorrow"];
    args.extend(base);
    let clock = serve_refused(&dir.path, &args, &[]);
    assert_eq!(clock.status, USAGE, "{clock:#?}");
    assert!(clock.stderr.contains("--clock-start"), "{clock:#?}");

    // The flag is the conformance run's: `--help` does not name it.
    let help = run(&dir.path, &["serve", "--help"], &[]);
    assert_eq!(help.status, 0, "{help:#?}");
    assert!(!help.stdout.contains("clock-start"), "{help:#?}");
}

#[test]
fn sigterm_stops_serve_with_status_0_and_the_first_line_names_the_address() {
    let (dir, operator) = keys_only("sigterm");
    let service = serve(&dir.path, &operator);
    let port: u16 = service
        .first_line
        .strip_prefix("http://127.0.0.1:")
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("the first line is the URL: {:?}", service.first_line));
    assert_ne!(port, 0);
    let status = service.terminate();
    assert_eq!(status.code(), Some(0), "{status:?}");

    let json = support::Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--json",
        ],
        &[],
    );
    let line = json::parse(&json.first_line).expect("one JSON document");
    assert_eq!(
        format!("http://{}", text(&line, "address")),
        text(&line, "url")
    );
    assert_eq!(json.terminate().code(), Some(0));

    // The conformance run's clock is named on standard error, never on standard output.
    let started = support::Service::start(
        &dir.path,
        &[
            "--listen",
            "127.0.0.1:0",
            "--key-file",
            "keys.json",
            "--operator-public-key",
            &operator,
            "--clock-start",
            "2019-12-30T00:00:00Z",
        ],
        &[],
    );
    assert!(
        started.first_line.starts_with("http://127.0.0.1:"),
        "{:?}",
        started.first_line
    );
    assert!(
        started.stderr().contains(
            "the service's clock starts at 2019-12-30T00:00:00Z, not at the system clock"
        ),
        "{}",
        started.stderr()
    );
    assert_eq!(started.terminate().code(), Some(0));
}

#[test]
fn add_key_refuses_a_public_key_that_is_not_one() {
    let dir = Dir::new("bad-public-key");
    key_new(&dir.path, "operator.key", None);
    let refused = run(
        &dir.path,
        &[
            "keyfile",
            "add-key",
            "--file",
            "keys.json",
            "--operator-secret-key-file",
            "operator.key",
            "--key-id",
            "x-1",
            "--principal",
            ALICE,
            "--public-key",
            "not-a-key",
        ],
        &[],
    );
    assert_eq!(refused.status, USAGE, "{refused:#?}");
    assert!(!dir.join("keys.json").exists());
}

/// The lines of the outbox file at `path`, as they are.
fn outbox_lines(path: &std::path::Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_services_own_400_401_408_413_503_keep_the_envelope_on_post_and_on_repost() {
    let dir = Dir::new("own-refusals");
    key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let refusals = [
        (
            400,
            r#"{"refused":"the body was shorter than `Content-Length` announced"}"#,
        ),
        (413, r#"{"refused":"body too large","limit":1}"#),
        (401, r#"{"refused":"unauthenticated"}"#),
        (408, r#"{"refused":"request timeout"}"#),
        (503, r#"{"refused":"stopping"}"#),
    ];
    for (status, body) in refusals {
        let outbox = dir.join(&format!("alice-{status}.outbox"));
        let outbox_text = outbox.display().to_string();
        let (url, served) = stand_in(status, body);
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
        assert_eq!(posted.status, REFUSED, "{status}: {posted:#?}");
        assert_eq!(
            posted.json().member("status"),
            Some(&Value::Number(status.to_string()))
        );
        assert_eq!(outbox_lines(&outbox).len(), 1, "{status} keeps it on post");
    }

    // The envelope kept on the 503 is reposted to each refusal in turn and stays every time.
    let outbox = dir.join("alice-503.outbox");
    let outbox_text = outbox.display().to_string();
    let before = outbox_lines(&outbox);
    for (status, body) in refusals {
        let (url, served) = stand_in(status, body);
        let env = [
            ("INBOX_URL", url.as_str()),
            ("INBOX_SECRET_KEY_FILE", "alice.key"),
            ("INBOX_OUTBOX", outbox_text.as_str()),
        ];
        let reposted = run(&dir.path, &["repost", "--json"], &env);
        served.join().expect("the stand-in ends");
        assert_eq!(reposted.status, REFUSED, "{status}: {reposted:#?}");
        assert_eq!(outbox_lines(&outbox), before, "{status} keeps it on repost");
    }
}

#[test]
fn an_answer_neither_success_nor_refusal_is_a_transport_failure() {
    let dir = Dir::new("odd-status");
    key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let (url, served) = stand_in(404, r#"{"no":"refused member"}"#);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
    ];
    let pending = run(&dir.path, &["pending", "--json"], &env);
    served.join().expect("the stand-in ends");
    assert_eq!(pending.status, TRANSPORT, "{pending:#?}");
    assert!(pending.stderr.contains("404"), "{pending:#?}");
}

/// A stand-in whose client never connects ends with a message that says so, and does not wait
/// for a connection forever: a case that joins it fails with that message. The wait is 1 s here
/// and 10 s for every other case; the case holds no time of its own, the runner ends a test at
/// 60 s.
#[test]
fn a_stand_in_that_receives_no_request_ends_and_says_so() {
    let (_url, served) = support::stand_in_answers_within(
        vec![(200, None, "{}")],
        std::time::Duration::from_secs(1),
    );
    let message = match served.join() {
        Ok(requests) => panic!("the stand-in ended with {requests:?} and no message"),
        Err(panic) => panic.downcast_ref::<String>().cloned().unwrap_or_default(),
    };
    assert!(
        message.contains("the stand-in received no connection for answer 1 of 1 within 1 s"),
        "{message:?}"
    );
}

#[test]
fn an_unreadable_outbox_line_is_listed_by_outbox_and_named_by_repost_and_never_dropped() {
    let world = World::new("unreadable-line");
    world.client("bob", &["open"]).ok();
    world
        .client("alice", &["post", "--to", BOB, "--body", "fine"])
        .ok();
    let path = world.outbox("alice");
    let mut text_of_file = fs::read_to_string(&path).unwrap();
    text_of_file.push_str("{\"not\":\"an envelope\"}\n");
    fs::write(&path, &text_of_file).unwrap();

    let listed = world.client("alice", &["outbox", "--json"]).ok().json();
    let unreadable = match member(&listed, &["unreadable"]) {
        Value::Array(entries) => entries.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(unreadable.len(), 1);
    assert_eq!(
        unreadable[0].member("line"),
        Some(&Value::Number("2".to_owned()))
    );
    let listed_text = world.client("alice", &["outbox"]).ok();
    assert!(
        listed_text.stdout.contains("unreadable outbox line 2"),
        "{listed_text:#?}"
    );

    let reposted = world.client("alice", &["repost", "--json"]);
    assert_eq!(reposted.status, REFUSED, "{reposted:#?}");
    assert_eq!(text(&reposted.json(), "error"), "unreadable outbox entry");
    assert!(reposted.stderr.contains("line 2"), "{reposted:#?}");
    assert_eq!(outbox_lines(&path).len(), 2, "nothing is dropped");
}

#[test]
fn keyfile_refuses_a_lapsed_valid_until_and_a_file_another_operator_signed() {
    let (dir, _operator) = keys_only("keyfile-refusals");
    let before = fs::read_to_string(dir.join("keys.json")).unwrap();
    let carol = key_new(
        &dir.path,
        "carol.key",
        Some(("carol-1", "agent:test/carol")),
    );
    let add = |operator: &str, valid_until: Option<&str>| {
        let mut args = vec![
            "keyfile",
            "add-key",
            "--file",
            "keys.json",
            "--operator-secret-key-file",
            operator,
            "--key-id",
            "carol-1",
            "--principal",
            "agent:test/carol",
            "--public-key",
            &carol,
        ];
        if let Some(valid_until) = valid_until {
            args.extend(["--valid-until", valid_until]);
        }
        run(&dir.path, &args, &[])
    };
    let lapsed = add("operator.key", Some("2000-01-01T00:00:00Z"));
    assert_eq!(lapsed.status, USAGE, "{lapsed:#?}");
    assert!(lapsed.stderr.contains("not after now"), "{lapsed:#?}");

    key_new(&dir.path, "other-operator.key", None);
    let mixed = add("other-operator.key", None);
    assert_eq!(mixed.status, LOCAL, "{mixed:#?}");
    assert!(
        mixed.stderr.contains("alice-1"),
        "the foreign entry is named: {mixed:#?}"
    );
    assert_eq!(
        fs::read_to_string(dir.join("keys.json")).unwrap(),
        before,
        "no mixed file"
    );
}

#[test]
fn a_url_outside_loopback_is_refused_before_anything_is_sent() {
    let dir = Dir::new("outside-url");
    key_new(&dir.path, "alice.key", Some(("alice-1", ALICE)));
    let refused = run(
        &dir.path,
        &["pending", "--secret-key-file", "alice.key", "--json"],
        &[("INBOX_URL", "http://192.0.2.1:9")],
    );
    assert_eq!(refused.status, USAGE, "{refused:#?}");
    assert!(
        refused.stderr.contains("not a loopback address"),
        "{refused:#?}"
    );
}
