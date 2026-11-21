//! The cases story:cli-client owes beside its acceptance: each client command against a running
//! `inbox serve`, refusals and their exit statuses, the outbox and `repost` across a restart, the
//! key commands, and how `inbox serve` starts, refuses to start and stops.

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use inbox::messaging::{DelegationId, Principal};
use inbox::server::json::{self, Value};
use inbox_verify::{Instant, KeyRing, PublicKey};

use support::{
    ALICE, BOB, Dir, LOCAL, REFUSED, TRANSPORT, USAGE, World, ids, key_new, keys_only, rows, run,
    serve, serve_refused, text,
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
    let entries = items(member(&same, &["reposted"]));
    assert_eq!(
        outcome(&entries[0]),
        "duplicate",
        "a repost the service still holds is `Duplicate`, and counts as arrived"
    );
    assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(true)));

    world.restart();
    let lost = world.client("bob", &["pending", "--json"]).ok().json();
    assert!(ids(&lost).is_empty(), "a restart lost the message");
    world.client("bob", &["open"]).ok();

    for _ in 0..2 {
        let reposted = world.client("alice", &["repost", "--json"]).ok().json();
        let entries = items(member(&reposted, &["reposted"]));
        assert_eq!(entries.len(), 1);
        assert_eq!(text(&entries[0], "message_id"), message_id);
        assert_eq!(entries[0].member("arrived"), Some(&Value::Bool(true)));
    }
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
