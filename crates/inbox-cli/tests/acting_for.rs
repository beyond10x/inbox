//! `inbox <command> --for <address>`: the client half of
//! story:an-agent-with-a-delegation-acts-for-the-inbox-of-its-principal. With `--for` every
//! request carries the header `Inbox-On-Behalf-Of` with the address and signs it as the sixth
//! member; the service answers it as the owner of that inbox while the key file holds a live
//! delegation of the pair (`docs/design/service.md`, "The caller"). ALICE is the agent, BOB the
//! person, CAROL a third principal. Every key and the key file are made when the case runs.

use crate::support;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use inbox::server::json::Value;
use inbox_server::format_instant;
use inbox_verify::Instant;

use support::{
    ALICE, BOB, REFUSED, Run, Service, USAGE, World, add_key, ids, key_new, keys_only, rows, run,
    text,
};

const CAROL: &str = "agent:test/carol";
/// The delegation of ALICE for BOB.
const D: &str = "alice-for-bob";
const HEADER: &str = "inbox-on-behalf-of";
const UNAUTHENTICATED: &str = r#"{"refused":"unauthenticated"}"#;

/// `inbox keyfile add-delegation` into `keys.json` of `dir`.
fn add_delegation(dir: &Path, id: &str, agent: &str, person: &str, until: Option<&str>) {
    let mut args = vec![
        "keyfile",
        "add-delegation",
        "--file",
        "keys.json",
        "--operator-secret-key-file",
        "operator.key",
        "--delegation-id",
        id,
        "--agent",
        agent,
        "--on-behalf-of",
        person,
    ];
    if let Some(until) = until {
        args.extend(["--valid-until", until]);
    }
    run(dir, &args, &[]).ok();
}

/// Replaces the clock file of `dir` whole.
fn set_clock(dir: &Path, at: Instant) {
    std::fs::write(dir.join("clock.new"), format_instant(at)).expect("the clock file is written");
    std::fs::rename(dir.join("clock.new"), dir.join("clock")).expect("the clock file is replaced");
}

/// The key set with CAROL's key added, the delegations `entries` (id, agent, person,
/// `valid_until`), and a service over it; with `clock`, the service reads its clock from the file
/// `clock`, which starts at that instant.
fn world(
    name: &str,
    entries: &[(&str, &str, &str, Option<&str>)],
    clock: Option<Instant>,
) -> World {
    let (dir, operator_public_key) = keys_only(name);
    let carol = key_new(&dir.path, "carol.key", Some(("carol-1", CAROL)));
    add_key(&dir.path, "operator.key", "carol-1", CAROL, &carol);
    for (id, agent, person, until) in entries {
        add_delegation(&dir.path, id, agent, person, *until);
    }
    let mut args = vec![
        "--listen",
        "127.0.0.1:0",
        "--key-file",
        "keys.json",
        "--operator-public-key",
        operator_public_key.as_str(),
        "--max-wait",
        "20",
    ];
    if let Some(at) = clock {
        set_clock(&dir.path, at);
        args.extend(["--clock-file", "clock"]);
    }
    let service = Service::start(&dir.path, &args, &[]);
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// The system clock's second.
fn this_second() -> Instant {
    Instant::from_unix(
        Instant::from_system_time(SystemTime::now()).unix_seconds(),
        0,
    )
}

/// A post of `from` to `to`, `kind` note or decision; answers its message id.
fn post(world: &World, from: &str, to: &str, kind: &str) -> String {
    let mut args = vec!["post", "--to", to, "--kind", kind, "--body", "Pick one"];
    if kind == "decision" {
        args.extend(["--option", "A=Red", "--option", "B=Blue"]);
    }
    args.push("--json");
    text(&world.client(from, &args).ok().json(), "message_id")
}

/// The row of `id` in a view answer.
fn row(view: &Value, id: &str) -> Value {
    rows(view)
        .into_iter()
        .find(|row| text(row, "message_id") == id)
        .unwrap_or_else(|| panic!("no row {id}"))
}

fn member<'a>(value: &'a Value, name: &str) -> &'a Value {
    value
        .member(name)
        .unwrap_or_else(|| panic!("no member {name}"))
}

/// The acceptance through the commands: two posts to the person; the agent lists both with
/// `pending --for`, reads the decision and answers it for the person, and the sender's outbox
/// shows it answered by the principal, the reply on behalf of the person.
#[test]
fn an_agent_reads_and_answers_the_persons_decision_with_for() {
    let world = world("for-acceptance", &[(D, ALICE, BOB, None)], None);
    world.client("bob", &["open"]).ok();
    let note = post(&world, "carol", BOB, "note");
    let decision = post(&world, "carol", BOB, "decision");

    let pending = world
        .client("alice", &["pending", "--for", BOB, "--json"])
        .ok()
        .json();
    let mut listed = ids(&pending);
    listed.sort();
    let mut expected = vec![note.clone(), decision.clone()];
    expected.sort();
    assert_eq!(listed, expected);
    let without = world
        .client("alice", &["pending", "--inbox", BOB, "--json"])
        .ok()
        .json();
    assert!(ids(&without).is_empty(), "without --for: {without:?}");

    let read = world
        .client("alice", &["read", &decision, "--for", BOB, "--json"])
        .ok()
        .json();
    assert_eq!(text(&read, "outcome"), "read");

    // `--authority` stays the caller's to give, and `--on-behalf-of` given stays as given.
    let no_authority = world.client("alice", &["answer", &decision, "--for", BOB, "--body", "x"]);
    assert_eq!(no_authority.status, REFUSED, "{no_authority:#?}");
    assert!(
        no_authority.stderr.contains("inbox.messaging.NoAuthority"),
        "{no_authority:#?}"
    );
    let as_itself = world.client(
        "alice",
        &[
            "answer",
            &decision,
            "--for",
            BOB,
            "--on-behalf-of",
            ALICE,
            "--body",
            "x",
            "--json",
        ],
    );
    assert_eq!(as_itself.status, REFUSED, "{as_itself:#?}");
    assert_eq!(
        text(member(&as_itself.json(), "answer"), "outcome"),
        "authority-invalid"
    );

    let answered = world
        .client(
            "alice",
            &[
                "answer",
                &decision,
                "--for",
                BOB,
                "--authority",
                D,
                "--option",
                "A",
                "--body",
                "Red it is",
                "--json",
            ],
        )
        .ok()
        .json();
    assert_eq!(text(member(&answered, "answer"), "outcome"), "answered");

    let outbox = world.client("carol", &["outbox", "--json"]).ok().json();
    let answered_row = row(&outbox, &decision);
    assert_eq!(text(&answered_row, "state"), "Answered");
    assert_eq!(text(&answered_row, "answered_by"), "principal");
    let reply = member(&answered_row, "reply");
    assert_eq!(
        (text(reply, "from"), text(reply, "on_behalf_of")),
        (ALICE.to_owned(), BOB.to_owned())
    );
}

/// Without a delegation, with one of another pair, and with a lapsed one, every command that
/// takes `--for` ends as a request signed with a key the service does not hold: the same exit
/// status, standard output and standard error. The person's message stays as it was.
#[test]
fn without_a_live_delegation_of_the_pair_every_command_ends_as_an_unknown_key() {
    let start = this_second();
    let until = format_instant(start.plus_seconds(30));
    let world = world(
        "for-refused",
        &[
            (D, ALICE, BOB, Some(until.as_str())),
            ("carol-for-alice", CAROL, ALICE, None),
        ],
        Some(start),
    );
    key_new(
        &world.dir.path,
        "dave.key",
        Some(("dave-1", "agent:test/dave")),
    );
    world.client("bob", &["open"]).ok();
    let decision = post(&world, "carol", BOB, "decision");
    world.client("alice", &["pending", "--for", BOB]).ok();
    set_clock(&world.dir.path, start.plus_seconds(31));

    let commands: Vec<Vec<&str>> = vec![
        vec!["pending"],
        vec!["wait", "--seconds", "0"],
        vec!["read", &decision],
        vec!["answer", &decision, "--option", "A", "--body", "x"],
        vec!["refuse", &decision, "--reason", "no"],
        vec!["outbox"],
        vec!["collect", &decision],
        vec!["thread", &decision],
        vec!["replies", &decision],
        vec!["post", "--to", BOB, "--body", "x"],
    ];
    let shown = |run: Run| (run.status, run.stdout, run.stderr);
    for command in &commands {
        let unknown = shown(world.client("dave", command));
        assert_eq!(unknown.0, REFUSED, "{command:?}: {unknown:?}");
        for (who, person, why) in [
            ("bob", CAROL, "no delegation"),
            ("carol", BOB, "a delegation of another pair"),
            ("alice", BOB, "a lapsed delegation"),
        ] {
            let mut args = command.clone();
            args.extend(["--for", person]);
            assert_eq!(
                shown(world.client(who, &args)),
                unknown,
                "{why}: {command:?}"
            );
        }
    }
    let held = world.client("bob", &["pending", "--json"]).ok().json();
    assert_eq!(text(&row(&held, &decision), "state"), "Sent");
}

/// An empty value and one with a space or a tab at either end are wrong usage, before any
/// request.
#[test]
fn an_empty_or_padded_for_is_wrong_usage_and_sends_nothing() {
    let (dir, _) = keys_only("for-usage");
    let (url, heads) = recording(401, UNAUTHENTICATED);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    for value in ["", " x", "x ", "\tx", "x\t"] {
        for command in [
            vec!["pending", "--for", value],
            vec!["post", "--to", BOB, "--body", "x", "--for", value],
            vec![
                "answer",
                "01J0000000000000000000000A",
                "--body",
                "x",
                "--for",
                value,
            ],
        ] {
            let refused = run(&dir.path, &command, &env);
            assert_eq!(refused.status, USAGE, "{value:?} {command:?}: {refused:#?}");
            assert!(refused.stderr.contains("--for"), "{refused:#?}");
        }
    }
    assert!(
        heads.lock().expect("not poisoned").is_empty(),
        "a request was sent"
    );
    assert!(!dir.join("alice.outbox").exists(), "an envelope was kept");
}

/// `inbox wake`, `inbox bridge`, `inbox open` and `inbox close` keep state on this host keyed by
/// the caller's own address: they do not take `--for`. Exit 2 and one line, nothing sent, no
/// file written.
#[test]
fn wake_bridge_open_and_close_do_not_take_for() {
    let (dir, _) = keys_only("for-local");
    let (url, heads) = recording(401, UNAUTHENTICATED);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let listing = || {
        let mut names: Vec<String> = std::fs::read_dir(&dir.path)
            .expect("listed")
            .map(|entry| {
                entry
                    .expect("an entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    };
    let before = listing();
    for command in ["wake", "bridge", "open", "close"] {
        let refused = run(&dir.path, &[command, "--for", BOB], &env);
        assert_eq!(refused.status, USAGE, "{command}: {refused:#?}");
        assert_eq!(
            refused.stderr,
            format!("inbox {command}: does not take --for\n"),
            "{command}"
        );
        assert!(refused.stdout.is_empty(), "{command}: {refused:#?}");
    }
    assert!(
        heads.lock().expect("not poisoned").is_empty(),
        "a request was sent"
    );
    assert_eq!(listing(), before, "a file was written");
}

/// Reads one request from `stream`: its head, lower-cased, and its body.
fn read_head(stream: &mut TcpStream) -> String {
    let mut received = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let end = received.windows(4).position(|window| window == b"\r\n\r\n");
        if let Some(end) = end {
            let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
            let length: usize = head
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0);
            if received.len() >= end + 4 + length {
                return head;
            }
        }
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return String::from_utf8_lossy(&received).to_lowercase(),
            Ok(read) => received.extend_from_slice(&buffer[..read]),
        }
    }
}

/// A stand-in that answers every request with `status` and `body` and keeps the head of each: a
/// client that should send nothing and sends gets an answer at once and does not hang the case.
fn recording(status: u16, body: &'static str) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    let heads = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&heads);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let head = read_head(&mut stream);
            kept.lock().expect("not poisoned").push(head);
            let _ = stream.write_all(
                format!(
                    "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
        }
    });
    (url, heads)
}

/// The value of the header in a request head, once per line that carries it.
fn header_values(head: &str) -> Vec<String> {
    head.lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.trim() == HEADER)
        .map(|(_, value)| value.trim().to_owned())
        .collect()
}

/// Every request of every command that takes `--for` carries the header once with the address
/// (lower-cased here, as the stand-in reads the head); without `--for` no request carries it.
#[test]
fn with_for_every_request_carries_the_header_and_without_it_none() {
    let (dir, _) = keys_only("for-header");
    let (url, heads) = recording(200, r#"{"rows":[]}"#);
    let env = [
        ("INBOX_URL", url.as_str()),
        ("INBOX_SECRET_KEY_FILE", "alice.key"),
        ("INBOX_OUTBOX", "alice.outbox"),
    ];
    let id = "01J0000000000000000000000A";
    let groups: [(&str, Vec<&str>); 10] = [
        ("views", vec!["pending"]),
        ("views", vec!["wait", "--seconds", "0"]),
        ("views", vec!["outbox"]),
        ("views", vec!["thread", id]),
        ("views", vec!["replies", id]),
        ("marks", vec!["read", id]),
        ("marks", vec!["refuse", id, "--reason", "no"]),
        ("marks", vec!["collect", id]),
        ("answer", vec!["answer", id, "--body", "x"]),
        ("post", vec!["post", "--to", BOB, "--body", "x"]),
    ];
    for (group, command) in &groups {
        for acting in [None, Some(BOB)] {
            let taken = heads.lock().expect("not poisoned").len();
            let mut args = command.clone();
            if let Some(person) = acting {
                args.extend(["--for", person]);
            }
            run(&dir.path, &args, &env);
            let sent: Vec<String> = heads.lock().expect("not poisoned")[taken..].to_vec();
            assert!(!sent.is_empty(), "{group} {args:?}: no request");
            for head in &sent {
                let expected: Vec<String> = acting.map(str::to_lowercase).into_iter().collect();
                assert_eq!(header_values(head), expected, "{group} {args:?}: {head}");
            }
        }
    }
}

/// `inbox serve` writes no line that tells a request for an inbox the signer holds no delegation
/// of from a request whose signature does not verify: its standard error stays as it was after
/// each, and the client prints the same for both.
#[test]
fn serve_writes_no_line_for_an_uncovered_inbox_or_a_bad_signature() {
    let world = world("for-serve-lines", &[(D, ALICE, BOB, None)], None);
    // A key that names ALICE's key id and is not ALICE's key: its signatures do not verify.
    key_new(&world.dir.path, "forged.key", Some(("alice-1", ALICE)));
    let before = world.service.stderr();
    let uncovered = world.client("carol", &["pending", "--for", BOB]);
    let after_uncovered = world.service.stderr();
    let forged = world.client("forged", &["pending"]);
    let after_forged = world.service.stderr();
    assert_eq!(uncovered.status, REFUSED, "{uncovered:#?}");
    assert_eq!(
        (uncovered.status, &uncovered.stdout, &uncovered.stderr),
        (forged.status, &forged.stdout, &forged.stderr)
    );
    assert_eq!(after_uncovered, before, "a line for the uncovered inbox");
    assert_eq!(after_forged, before, "a line for the bad signature");
}

/// After a new start of the service the reconnect of a command under `--for` does not open the
/// caller's own inbox (its record is keyed by the caller's own address, and under the header the
/// service answers that open `not-owner`); the next command without `--for` opens it again.
#[test]
fn under_for_the_reconnect_leaves_the_callers_own_inbox_to_a_command_without_it() {
    let mut world = world("for-reconnect", &[(D, ALICE, BOB, None)], None);
    world.client("alice", &["open"]).ok();
    world.client("bob", &["open"]).ok();
    world.restart();
    let acting = world.client("alice", &["pending", "--for", BOB]).ok();
    assert!(
        !acting.stderr.contains("reconnect"),
        "under --for: {acting:#?}"
    );
    let own = world.client("alice", &["pending"]).ok();
    assert!(
        own.stderr
            .contains(&format!("opened {ALICE} again with the adapter terminal")),
        "without --for: {own:#?}"
    );
}
