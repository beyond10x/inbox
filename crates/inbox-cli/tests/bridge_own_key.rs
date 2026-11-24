//! The cases of story:the-chat-bridge-runs-with-its-own-key-under-a-delegation: `inbox bridge`
//! with a key of its own (principal `agent:demo/chat-bridge`, `chat-bridge.key`) and
//! `--for <person> --authority <delegation id>` serves the person's inbox: every request names
//! the person's inbox (`Inbox-On-Behalf-Of`), and the lock of the wake, the line of
//! `<outbox>.opened` and the record of the chat posts beside the outbox file are keyed by the
//! person's address. A refused key or delegation ends it with one line.
//!
//! Every case runs against a real `inbox serve` on loopback (the debug build, with the clock file
//! of `bridge_once`), with a key file made in the case that holds the bridge's key entry and the
//! delegation, and the stand-in `connectors` and the forwarder of `bridge_once`, whose helpers it
//! shares; the 3 acceptance cases of the bridge stories run once more here with the bridge's own
//! key. The bridge shares the person's outbox file unless a case says otherwise. No case reaches
//! a chat provider or a network beyond loopback, and no case holds a wall-clock time: a case waits
//! on the stand-in's record, the forwarder's record or a fact, with a bound that only ends a hang,
//! and moves a clock by writing its file.

use crate::bridge_once::{
    self, Bridge, CHANNEL, Connectors, Forwarder, REACTED, client, history, post, posted, recorded,
    restart, state, wait_for,
};
use crate::support;

use std::fs;
use std::path::{Path, PathBuf};

use inbox::server::json::{self, Value};
use inbox_server::format_instant;
use support::{
    ALICE, BOB, CHAT_BRIDGE, CHAT_BRIDGE_KEY_ID, Run, Signer, Spawned, World, add_delegation,
    chat_bridge_key, inboxes, keys_only, text,
};

/// The delegation of the bridge for bob.
const D: &str = "chat-bridge-for-bob";
/// The delegation of the bridge for alice.
const D_ALICE: &str = "chat-bridge-for-alice";
/// The bridge with its own key, for bob.
const OWN: Signer = Signer::Own {
    person: BOB,
    delegation: D,
};
/// The bridge with its own key, for alice.
const OWN_ALICE: Signer = Signer::Own {
    person: ALICE,
    delegation: D_ALICE,
};
/// The exit status of a second wake or bridge of the same inbox on the same host.
const ANOTHER_WAKE: i32 = 5;

/// The world of a case: the key set of the run with the key file prepared for each of `signers`,
/// the clock files of the service and of the bridge at the system clock's second, and a service
/// over them.
fn world(name: &str, signers: &[Signer]) -> World {
    let (dir, operator_public_key) = keys_only(name);
    for signer in signers {
        signer.prepare(&dir.path);
    }
    let now = bridge_once::this_second();
    bridge_once::set_clock(&dir.path, "clock", now);
    bridge_once::set_clock(&dir.path, "bridge-clock", now);
    let service = bridge_once::serve(&dir.path, &operator_public_key);
    World {
        service,
        operator_public_key,
        dir,
    }
}

/// The lines of `<who>.outbox.opened`: the inbox and the adapter of each.
fn opened(world: &World, who: &str) -> Vec<(String, String)> {
    let mut path = world.outbox(who).into_os_string();
    path.push(".opened");
    fs::read_to_string(PathBuf::from(path))
        .unwrap_or_default()
        .lines()
        .map(|line| {
            let line = json::parse(line).unwrap_or_else(|error| panic!("{error}: {line}"));
            (text(&line, "inbox"), text(&line, "adapter"))
        })
        .collect()
}

/// The line of `<who>.outbox.opened` that names `inbox`, as (inbox, adapter).
fn line_of(inbox: &str, adapter: &str) -> (String, String) {
    (inbox.to_owned(), adapter.to_owned())
}

/// The head of a request as the forwarder kept it, lower-cased.
fn head(sent: &[u8]) -> String {
    let text = String::from_utf8_lossy(sent).to_lowercase();
    text.split("\r\n\r\n").next().unwrap_or_default().to_owned()
}

/// The heads of the requests the forwarder carried that the key `key_id` signed, in their order.
fn signed_by(forwarder: &Forwarder, key_id: &str) -> Vec<String> {
    let id = format!("inbox-key-id: {}", key_id.to_lowercase());
    forwarder
        .sent()
        .iter()
        .map(|sent| head(sent))
        .filter(|head| head.lines().any(|line| line.trim() == id))
        .collect()
}

/// The values of `Inbox-On-Behalf-Of` in a head, one per line that carries it.
fn on_behalf_of(head: &str) -> Vec<String> {
    head.lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.trim() == "inbox-on-behalf-of")
        .map(|(_, value)| value.trim().to_owned())
        .collect()
}

/// The request line of `bob`'s `Pending` with `query` after the inbox, as the client sends it.
fn pending_line(query: &str) -> String {
    format!("get /messaging/views/pending?inbox=agent%3atest%2fbob{query} http/1.1")
}

/// Waits until `spawned` ends by itself; [`bridge_once::wait_for`]'s bound only ends a hang.
fn exits(mut spawned: Spawned, what: &str) -> Run {
    wait_for(what, || !spawned.running());
    spawned.finish()
}

/// The row of `address` in the `Inboxes` view of the service at `url`, read with alice's key.
fn inbox_row(world: &World, url: &str, address: &str) -> Option<Value> {
    inboxes(&world.dir.path, "alice", url)
        .into_iter()
        .find(|row| text(row, "address") == address)
}

/// The acceptance of story:the-chat-bridge-posts-a-pending-message with the bridge's own key.
#[test]
fn the_acceptance_of_the_post_runs_with_the_bridges_own_key() {
    crate::bridge::a_note_a_request_and_a_decision_are_posted_in_the_order_of_pending_and_delivered_signed(
        Signer::Own {
            person: crate::bridge::PERSON,
            delegation: "chat-bridge-for-person",
        },
    );
}

/// The acceptance of story:the-chat-bridge-marks-a-message-read-on-a-reaction with the bridge's
/// own key.
#[test]
fn the_acceptance_of_the_read_mark_runs_with_the_bridges_own_key() {
    crate::bridge_read::a_reaction_of_the_allowed_chat_user_reads_a_note_once_signed(OWN);
}

/// The acceptance of story:the-chat-bridge-asks-a-decision-once with the bridge's own key.
#[test]
fn the_acceptance_of_the_message_asked_once_runs_with_the_bridges_own_key() {
    crate::bridge_once::a_message_posted_again_after_a_restart_is_marked_and_never_posted_twice_signed(
        OWN,
    );
}

/// The acceptance and check 1: the bridge with its own key, under the delegation, opens bob's
/// inbox (bob never opened it), posts a note and a request to it as one chat post each and moves
/// both to `Delivered`; a reaction reads the note, whose thread the poll then reads; after a new
/// start of the service its reconnect opens bob's inbox again and the request posted again is
/// marked delivered. Every request the bridge's key signed carries `Inbox-On-Behalf-Of` with bob's
/// address, once: the read with `wait=0` and the held read (both `undelivered=1`), the whole read
/// of the poll, `mark-delivered`, `read`, the `thread` read and both `open-inbox`.
#[test]
fn every_request_of_the_bridge_names_the_persons_inbox() {
    let mut world = world("own-key-requests", &[OWN]);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    wait_for("the bridge reads bob's Pending", || {
        signed_by(&forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .any(|head| head.starts_with(&pending_line("&wait=0&undelivered=1")))
    });
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    let request = post(&world, &url, "request", "Which build is green?", &[]);
    let note_ts = posted(&world, &url, &note);
    posted(&world, &url, &request);
    connectors.file("history", &history(&[(note_ts.as_str(), REACTED)]));
    wait_for("the note is read", || {
        recorded(&world, &note, "state") == "read"
    });
    wait_for("the poll reads the thread of the note", || {
        signed_by(&forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .any(|head| head.starts_with("get /messaging/views/thread?"))
    });
    let opens = |forwarder: &Forwarder| {
        signed_by(forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .filter(|head| head.starts_with("post /messaging/commands/open-inbox "))
            .count()
    };
    assert_eq!(opens(&forwarder), 1, "the open of the start");
    restart(&mut world, &forwarder);
    wait_for("the reconnect opens bob's inbox again", || {
        opens(&forwarder) >= 2
    });
    wait_for("the request posted again is delivered again", || {
        client(&world, &url, "alice", &["outbox"]);
        state(&world, &url, &request) == "Delivered"
    });
    bridge.end();

    let heads = signed_by(&forwarder, CHAT_BRIDGE_KEY_ID);
    for (at, head) in heads.iter().enumerate() {
        assert_eq!(
            on_behalf_of(head),
            [BOB.to_lowercase()],
            "request {at} of the bridge: {head}"
        );
    }
    for kind in [
        pending_line("&wait=0&undelivered=1"),
        pending_line("&wait=60&undelivered=1"),
        pending_line(""),
        "post /messaging/commands/mark-delivered ".to_owned(),
        "post /messaging/commands/read ".to_owned(),
        "get /messaging/views/thread?".to_owned(),
        "post /messaging/commands/open-inbox ".to_owned(),
    ] {
        assert!(
            heads.iter().any(|head| head.starts_with(&kind)),
            "no request {kind}: {heads:#?}"
        );
    }
    assert_eq!(connectors.posts(), 2, "one chat post per message");
}

/// Check 2: the lock of the wake is bob's. A second bridge with the same key and outbox file for
/// bob exits 5 and posts nothing; bob's own `inbox wake` with that outbox file exits 5 too. A
/// bridge with the same key and outbox file for alice runs and posts alice's message.
#[test]
fn the_lock_is_the_persons_and_a_bridge_for_another_person_runs_beside_it() {
    let world = world("own-key-lock", &[OWN, OWN_ALICE]);
    let url = world.service.url.clone();
    let connectors = Connectors::new(&world);
    let first = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    wait_for("the first bridge opened bob's inbox", || {
        opened(&world, "bob").contains(&line_of(BOB, "slack"))
    });

    let (status, said) = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN).exit();
    assert_eq!(status, Some(ANOTHER_WAKE), "{said}");
    assert_eq!(
        said,
        format!("inbox bridge: another wake of {BOB} runs on this host\n")
    );
    let own_wake = exits(world.spawn("bob", &["wake"]), "bob's own wake ends");
    assert_eq!(own_wake.status, ANOTHER_WAKE, "{own_wake:#?}");

    let second = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN_ALICE);
    wait_for("the bridge for alice opened alice's inbox", || {
        opened(&world, "bob").contains(&line_of(ALICE, "slack"))
    });
    let to_alice = client(
        &world,
        &url,
        "bob",
        &[
            "post",
            "--to",
            ALICE,
            "--kind",
            "note",
            "--body",
            "For alice.",
        ],
    )
    .stdout
    .trim()
    .to_owned();
    wait_for("alice's message is delivered", || {
        support::rows(&client(&world, &url, "alice", &["pending", "--json"]).json())
            .iter()
            .any(|row| text(row, "message_id") == to_alice && text(row, "state") == "Delivered")
    });
    assert_eq!(connectors.posts(), 1, "one post: alice's message");
    first.end();
    second.end();
}

/// Check 3: the open of the bridge writes bob's line of `<outbox>.opened` with `slack`, and none
/// of the bridge's own address; after a new start of the service its reconnect opens bob's inbox
/// again (`Inboxes` lists it `Open` with `slack`) and opens no inbox of the bridge's address.
#[test]
fn the_open_of_the_bridge_is_the_persons_line_and_a_restart_opens_the_persons_inbox_again() {
    let mut world = world("own-key-opened", &[OWN]);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    wait_for("the bridge opened bob's inbox", || {
        !opened(&world, "bob").is_empty()
    });
    assert_eq!(opened(&world, "bob"), [line_of(BOB, "slack")]);

    restart(&mut world, &forwarder);
    let service = world.service.url.clone();
    wait_for("the reconnect opens bob's inbox again", || {
        inbox_row(&world, &service, BOB).is_some()
    });
    let row = inbox_row(&world, &service, BOB).expect("bob's inbox");
    assert_eq!(
        (text(&row, "state"), text(&row, "adapter")),
        ("Open".to_owned(), "slack".to_owned())
    );
    assert!(
        inbox_row(&world, &service, CHAT_BRIDGE).is_none(),
        "no inbox of the bridge's own address"
    );
    assert_eq!(opened(&world, "bob"), [line_of(BOB, "slack")]);
    bridge.end();
}

/// The files of `dir` whose name starts with `prefix`, but a staged one.
fn files(dir: &Path, prefix: &str) -> Vec<String> {
    let mut found: Vec<String> = fs::read_dir(dir)
        .expect("the directory is read")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(prefix) && !name.ends_with(".new"))
        .collect();
    found.sort();
    found
}

/// The 64-bit FNV-1a hash of `bytes`, as the names of the files beside the outbox file hold it.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Check 4: a bridge with bob's key posts a note and is ended; after a new start of the service
/// the note is posted again and is in `Sent`; the bridge with its own key for bob, with the same
/// outbox file, reads the same record (`<outbox>.chat.<hash of bob's address>`), marks the note
/// delivered and makes no second post.
#[test]
fn a_bridge_with_its_own_key_reads_the_record_of_a_bridge_with_the_persons_key() {
    let mut world = world("own-key-record", &[OWN]);
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    client(&world, &url, "bob", &["open", "--adapter", "slack"]);
    let connectors = Connectors::new(&world);
    let by_person = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], Signer::Person);
    let note = post(&world, &url, "note", "Build 7 is green.", &[]);
    posted(&world, &url, &note);
    by_person.end();

    restart(&mut world, &forwarder);
    client(&world, &url, "bob", &["pending"]);
    wait_for("alice's next command posts the note again", || {
        client(&world, &url, "alice", &["outbox"]);
        state(&world, &url, &note) == "Sent"
    });
    let own = Bridge::signed(&world, &connectors, &url, CHANNEL, &[], OWN);
    wait_for(
        "the bridge with its own key marks the note delivered",
        || state(&world, &url, &note) == "Delivered",
    );
    assert_eq!(connectors.posts(), 1, "no second post");
    assert_eq!(
        files(&world.dir.path, "bob.outbox.chat."),
        [format!("bob.outbox.chat.{:016x}", fnv1a(BOB.as_bytes()))]
    );
    own.end();
}

/// Check 5: the check of the adapter reads bob's line of `<outbox>.opened`: with bob's inbox
/// opened `terminal` there, the bridge with its own key ends with 1 and its one line, before any
/// request and any post.
#[test]
fn the_check_of_the_adapter_reads_the_persons_line() {
    let world = world("own-key-adapter", &[OWN]);
    world.client("bob", &["open", "--adapter", "terminal"]).ok();
    let forwarder = Forwarder::start(&world.service.url);
    let connectors = Connectors::new(&world);
    let (status, said) =
        Bridge::signed(&world, &connectors, &forwarder.url(), CHANNEL, &[], OWN).exit();
    assert_eq!(status, Some(1), "{said}");
    assert_eq!(said.lines().count(), 1, "{said}");
    assert!(
        said.starts_with(&format!(
            "inbox bridge: {BOB} was opened on this host with the adapter terminal"
        )) && said.contains("inbox open --adapter slack"),
        "{said}"
    );
    assert!(forwarder.sent().is_empty(), "a request was sent");
    assert_eq!(connectors.posts(), 0);
}

/// Check 6: with the delegation missing from the key file, with its `valid_until` before the
/// service's clock at its start, and with the bridge's key entry lapsed, every request is answered
/// `401`: the bridge ends with 1 after one line, the same for the 3 causes, that names the key id,
/// the delegation id, `inbox keyfile add-key`, `inbox keyfile add-delegation` and that the
/// service reads the key file at its start; the forwarder sees no request after the first, and
/// nothing is posted.
#[test]
fn a_refused_key_or_delegation_ends_the_bridge_with_one_line_and_no_further_request() {
    let start = bridge_once::this_second();
    let until = format_instant(start.plus_seconds(30));
    let mut lines = Vec::new();
    for (cause, key_until, delegation) in [
        ("no delegation", None, None),
        ("a lapsed delegation", None, Some(Some(until.as_str()))),
        ("a lapsed key entry", Some(until.as_str()), Some(None)),
    ] {
        let (dir, operator_public_key) = keys_only("own-key-refused");
        chat_bridge_key(&dir.path, key_until);
        if let Some(delegation_until) = delegation {
            add_delegation(&dir.path, D, CHAT_BRIDGE, BOB, delegation_until);
        }
        // The service's clock is past both `valid_until`, within the 60 s a request's timestamp
        // may be off the clock.
        bridge_once::set_clock(&dir.path, "clock", start.plus_seconds(31));
        let service = bridge_once::serve(&dir.path, &operator_public_key);
        let world = World {
            service,
            operator_public_key,
            dir,
        };
        let forwarder = Forwarder::start(&world.service.url);
        let connectors = Connectors::new(&world);
        let (status, said) =
            Bridge::signed(&world, &connectors, &forwarder.url(), CHANNEL, &[], OWN).exit();
        assert_eq!(status, Some(1), "{cause}: {said}");
        assert_eq!(said.lines().count(), 1, "{cause}: {said}");
        for named in [
            CHAT_BRIDGE_KEY_ID,
            D,
            "`inbox keyfile add-key`",
            "`inbox keyfile add-delegation`",
            "reads the key file at its start",
        ] {
            assert!(said.contains(named), "{cause}: no {named}: {said}");
        }
        assert_eq!(forwarder.sent().len(), 1, "{cause}: one request, the first");
        assert_eq!(connectors.posts() + connectors.reads(), 0, "{cause}");
        lines.push(said);
    }
    assert!(
        lines.windows(2).all(|pair| pair[0] == pair[1]),
        "one line for every cause: {lines:#?}"
    );
}

/// `inbox wake --for bob` of the bridge's key, with an outbox file of its own: its lock is bob's
/// (a second one for bob, and bob's own `inbox wake` with that outbox file, exit 5), every request
/// it sends names bob's inbox, and it prints bob's note and marks it delivered.
#[test]
fn a_wake_for_the_person_takes_the_persons_lock_and_names_the_inbox_in_every_request() {
    let world = world("own-key-wake", &[OWN]);
    world.client("bob", &["open"]).ok();
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    let env = [("INBOX_URL", url.as_str())];
    let mut command = world.client_command("chat-bridge", &["wake", "--for", BOB]);
    command.envs(env);
    let woken = Spawned::from(command);
    wait_for("the wake holds its read", || {
        signed_by(&forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .any(|head| head.starts_with(&pending_line("&wait=60&undelivered=1")))
    });

    let mut second = world.client_command("chat-bridge", &["wake", "--for", BOB]);
    second.envs(env);
    let second = exits(Spawned::from(second), "the second wake ends");
    assert_eq!(second.status, ANOTHER_WAKE, "{second:#?}");
    assert_eq!(
        second.stderr,
        format!("inbox wake: another wake of {BOB} runs on this host\n")
    );
    let outbox = world.outbox("chat-bridge");
    let outbox = outbox.to_str().expect("a UTF-8 path");
    let mut own = world.client_command("bob", &["wake"]);
    own.env("INBOX_OUTBOX", outbox);
    let own = exits(Spawned::from(own), "bob's own wake ends");
    assert_eq!(own.status, ANOTHER_WAKE, "{own:#?}");

    let note = post(&world, &world.service.url, "note", "Build 7 is green.", &[]);
    let woken = exits(woken, "the wake ends after its row");
    assert_eq!(woken.status, 0, "{woken:#?}");
    assert!(woken.stdout.contains(&note), "{woken:#?}");
    assert_eq!(state(&world, &world.service.url, &note), "Delivered");
    let heads = signed_by(&forwarder, CHAT_BRIDGE_KEY_ID);
    for (at, head) in heads.iter().enumerate() {
        assert_eq!(
            on_behalf_of(head),
            [BOB.to_lowercase()],
            "request {at}: {head}"
        );
    }
    assert!(
        heads
            .iter()
            .any(|head| head.starts_with("post /messaging/commands/mark-delivered ")),
        "{heads:#?}"
    );
}

/// `inbox open --for bob` and `inbox close --for bob` of the bridge's key, with an outbox file of
/// its own: the open writes bob's line of `<outbox>.opened` and none of the caller's own; after a
/// new start of the service the reconnect of a command under `--for` opens bob's inbox again and
/// not the caller's; the close removes bob's line and closes bob's inbox.
#[test]
fn open_and_close_for_the_person_keep_the_persons_line_and_its_reconnect_opens_it() {
    let mut world = World::signed("own-key-open-close", OWN);
    let opened_now = world
        .client("chat-bridge", &["open", "--for", BOB, "--json"])
        .ok()
        .json();
    assert_eq!(text(&opened_now, "outcome"), "opened");
    assert_eq!(opened(&world, "chat-bridge"), [line_of(BOB, "terminal")]);

    world.restart();
    let acting = world.client("chat-bridge", &["pending", "--for", BOB]).ok();
    assert!(
        acting
            .stderr
            .contains(&format!("opened {BOB} again with the adapter terminal")),
        "{acting:#?}"
    );
    assert!(!acting.stderr.contains(CHAT_BRIDGE), "{acting:#?}");
    let service = world.service.url.clone();
    let row = inbox_row(&world, &service, BOB).expect("bob's inbox is open again");
    assert_eq!(
        (text(&row, "state"), text(&row, "adapter")),
        ("Open".to_owned(), "terminal".to_owned())
    );
    assert!(inbox_row(&world, &service, CHAT_BRIDGE).is_none());

    let closed = world
        .client("chat-bridge", &["close", "--for", BOB, "--json"])
        .ok()
        .json();
    assert_eq!(text(&closed, "outcome"), "closed");
    assert!(opened(&world, "chat-bridge").is_empty());
    let row = inbox_row(&world, &service, BOB).expect("bob's inbox");
    assert_eq!(text(&row, "state"), "Closed");
}

/// The connections the forwarder carried that the key `key_id` signed: their number and head.
fn signed_at(forwarder: &Forwarder, key_id: &str) -> Vec<(usize, String)> {
    let id = format!("inbox-key-id: {}", key_id.to_lowercase());
    forwarder
        .sent()
        .iter()
        .map(|sent| head(sent))
        .enumerate()
        .filter(|(_, head)| head.lines().any(|line| line.trim() == id))
        .collect()
}

/// The number of the last connection of the bridge's key whose head starts with `line`.
fn last_of(forwarder: &Forwarder, line: &str) -> usize {
    signed_at(forwarder, CHAT_BRIDGE_KEY_ID)
        .into_iter()
        .filter(|(_, head)| head.starts_with(line))
        .map(|(at, _)| at)
        .next_back()
        .unwrap_or_else(|| panic!("no request {line}"))
}

/// Starts the bridge with its own key for bob, has alice post a note, and waits until the note
/// is delivered and the rounds hold their read after its mark: from then on the rounds send
/// nothing until the service answers that read. Answers the bridge.
fn holding_after_a_note(world: &World, forwarder: &Forwarder, connectors: &Connectors) -> Bridge {
    let bridge = Bridge::signed(world, connectors, &forwarder.url(), CHANNEL, &[], OWN);
    wait_for("the bridge reads bob's Pending", || {
        signed_by(forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .any(|head| head.starts_with(&pending_line("&wait=0&undelivered=1")))
    });
    let service = world.service.url.clone();
    let note = post(world, &service, "note", "Build 7 is green.", &[]);
    wait_for("the note is delivered", || {
        state(world, &service, &note) == "Delivered"
    });
    let marked = last_of(forwarder, "post /messaging/commands/mark-delivered ");
    wait_for("the rounds hold their read after the mark", || {
        signed_at(forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .any(|(at, head)| {
                *at > marked && head.starts_with(&pending_line("&wait=60&undelivered=1"))
            })
    });
    bridge
}

/// The bridge ended with 1 and the one line of a refused key: answers the line.
fn ended_on_the_refused_key(bridge: Bridge) -> String {
    let (status, said) = bridge.exit();
    assert_eq!(status, Some(1), "{said}");
    assert_eq!(said.lines().count(), 1, "{said:?}");
    for named in [CHAT_BRIDGE_KEY_ID, D, "`inbox keyfile add-delegation`"] {
        assert!(said.contains(named), "no {named}: {said}");
    }
    said
}

/// Finding 2 of the adversary pass 1: the first request answered `401` is the poll's whole read
/// of bob's `Pending` (the forwarder answers it so once the rounds hold their read). The client
/// of the poll ends the process as the rounds' would: exit 1, the one line, and no request of the
/// bridge after the refused one.
#[test]
fn a_401_to_the_read_of_the_poll_ends_the_bridge_with_one_line_and_no_further_request() {
    let world = world("own-key-poll-401", &[OWN]);
    let forwarder = Forwarder::start(&world.service.url);
    let connectors = Connectors::new(&world);
    let bridge = holding_after_a_note(&world, &forwarder, &connectors);
    forwarder.refuse(&pending_line(""));
    ended_on_the_refused_key(bridge);
    let refused = forwarder.refused();
    assert_eq!(refused.len(), 1, "{refused:?}");
    let after: Vec<_> = signed_at(&forwarder, CHAT_BRIDGE_KEY_ID)
        .into_iter()
        .filter(|(at, _)| *at > refused[0])
        .collect();
    assert!(after.is_empty(), "requests after the 401: {after:#?}");
    assert_eq!(connectors.posts(), 1);
}

/// Whether the process `pid` is gone: no entry in `/proc`, or one that only waits to be reaped.
fn gone(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).map_or(true, |stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| rest.starts_with('Z'))
    })
}

/// Finding 3 of the adversary pass 1: a `connectors` child runs (the poll's read of the channel,
/// held by the stand-in until the case releases it) when the `401` arrives (the answer to the
/// rounds' held read, which alice's second note ends). When the bridge has ended, the child is
/// gone with it.
#[test]
fn a_401_while_a_child_runs_ends_the_child_with_the_bridge() {
    let world = world("own-key-child-401", &[OWN]);
    let forwarder = Forwarder::start(&world.service.url);
    let connectors = Connectors::new(&world);
    connectors.file("hold-read", "");
    let bridge = holding_after_a_note(&world, &forwarder, &connectors);
    let mut child = None;
    wait_for("the poll's read of the channel holds", || {
        child = connectors.holding_read();
        child.is_some()
    });
    let child = child.expect("the child's process id");
    assert!(!gone(child), "the child runs");
    forwarder.refuse(&pending_line("&wait=60&undelivered=1"));
    post(&world, &world.service.url, "note", "Build 8 is green.", &[]);
    ended_on_the_refused_key(bridge);
    let is_gone = gone(child);
    connectors.file("release-read", "");
    assert!(is_gone, "the child {child} outlived the bridge");
    assert_eq!(forwarder.refused().len(), 1);
}

/// The ids of the envelopes the connections from `from` on posted, with the head of each.
fn posts_from(forwarder: &Forwarder, from: usize) -> Vec<(String, String)> {
    forwarder
        .sent()
        .iter()
        .skip(from)
        .map(|sent| (head(sent), String::from_utf8_lossy(sent).into_owned()))
        .filter(|(head, _)| head.starts_with("post /messaging/commands/post "))
        .map(|(head, sent)| {
            let body = sent.split("\r\n\r\n").nth(1).unwrap_or_default().to_owned();
            let envelope = json::parse(&body).unwrap_or_else(|error| panic!("{error}: {body}"));
            (text(&envelope, "message_id"), head)
        })
        .collect()
}

/// Finding 4 of the adversary pass 1: the agent (the bridge's key) posts one note to alice under
/// `--for bob` and one without, with one outbox file. Both envelopes are the agent's (`from` is
/// the agent; the kept envelopes are keyed by their sender, decided in round 1): after a new start
/// of the service the agent's next command, under `--for`, posts both again, each once, with the
/// header of bob; its next command without `--for` posts none again. alice's inbox is open at
/// both starts (her own command opens it again), so each post is answered.
#[test]
fn the_agents_kept_envelopes_are_posted_again_once_by_its_next_command_under_for() {
    let mut world = world("own-key-kept", &[OWN]);
    world.client("alice", &["open"]).ok();
    let forwarder = Forwarder::start(&world.service.url);
    let url = forwarder.url();
    let note = |extra: &[&str], body: &str| {
        let mut args = vec!["post", "--to", ALICE, "--kind", "note", "--body", body];
        args.extend(extra);
        client(&world, &url, "chat-bridge", &args)
            .stdout
            .trim()
            .to_owned()
    };
    let under_for = note(&["--for", BOB], "For alice, sent for bob.");
    let own = note(&[], "For alice.");

    restart(&mut world, &forwarder);
    let reopened = world.client("alice", &["outbox"]).ok();
    assert!(
        reopened
            .stderr
            .contains(&format!("opened {ALICE} again with the adapter")),
        "{reopened:#?}"
    );
    let from = forwarder.sent().len();
    client(&world, &url, "chat-bridge", &["pending", "--for", BOB]);
    let again = posts_from(&forwarder, from);
    let mut ids: Vec<&str> = again.iter().map(|(id, _)| id.as_str()).collect();
    ids.sort_unstable();
    let mut both = vec![under_for.as_str(), own.as_str()];
    both.sort_unstable();
    assert_eq!(ids, both, "each envelope once: {again:#?}");
    for (id, head) in &again {
        assert_eq!(on_behalf_of(head), [BOB.to_lowercase()], "{id}: {head}");
    }

    let from = forwarder.sent().len();
    client(&world, &url, "chat-bridge", &["outbox"]);
    assert!(
        posts_from(&forwarder, from).is_empty(),
        "posted again twice"
    );
}

/// The 401 of a delegation that lapses while the bridge runs (taken from the adversary pass 1, a
/// green case it did not keep): the service's clock moves past the delegation's `valid_until`
/// while the rounds hold their read, and alice posts a note. The bridge's next request is
/// answered `401` (the mark of the note, or the next read when the held read ended first): it
/// ends with 1 and the one line, and the forwarder sees at most 1 request of the bridge's key
/// after the clock moved, the one answered `401` (which may have been carried just before the
/// clock moved and read by the service just after). A post made is recorded.
#[test]
fn a_delegation_that_lapses_while_the_bridge_runs_ends_it_with_one_line() {
    let start = bridge_once::this_second();
    let until = format_instant(start.plus_seconds(30));
    let (dir, operator_public_key) = keys_only("own-key-lapses");
    chat_bridge_key(&dir.path, None);
    add_delegation(&dir.path, D, CHAT_BRIDGE, BOB, Some(until.as_str()));
    bridge_once::set_clock(&dir.path, "clock", start);
    bridge_once::set_clock(&dir.path, "bridge-clock", start);
    let service = bridge_once::serve(&dir.path, &operator_public_key);
    let world = World {
        service,
        operator_public_key,
        dir,
    };
    let forwarder = Forwarder::start(&world.service.url);
    let connectors = Connectors::new(&world);
    let bridge = Bridge::signed(&world, &connectors, &forwarder.url(), CHANNEL, &[], OWN);
    wait_for("the bridge holds its read", || {
        signed_by(&forwarder, CHAT_BRIDGE_KEY_ID)
            .iter()
            .any(|head| head.starts_with(&pending_line("&wait=60&undelivered=1")))
    });
    let before = signed_by(&forwarder, CHAT_BRIDGE_KEY_ID).len();
    // Past `valid_until`, within the 60 s a request's timestamp may be off the service's clock.
    bridge_once::set_clock(&world.dir.path, "clock", start.plus_seconds(31));
    let note = post(&world, &world.service.url, "note", "Build 7 is green.", &[]);
    ended_on_the_refused_key(bridge);
    assert!(connectors.posts() <= 1, "{}", connectors.posts());
    if connectors.posts() == 1 {
        assert_eq!(recorded(&world, &note, "state"), "posted");
    }
    let after = signed_by(&forwarder, CHAT_BRIDGE_KEY_ID);
    assert!(
        after.len() - before <= 1,
        "at most the request answered 401: {:#?}",
        &after[before..]
    );
}

/// One outbox file for 2 persons (taken from the adversary pass 1, a green case it did not keep):
/// the bridge's key opens bob's inbox with `slack` and alice's with `terminal` under `--for`, then
/// closes alice's. bob's line of `<outbox>.opened` stays and alice's is gone; after a new start of
/// the service a command for alice opens nothing and one for bob opens bob's inbox again.
#[test]
fn a_close_for_one_person_keeps_the_line_of_the_other_and_only_that_is_opened_again() {
    let (dir, operator_public_key) = keys_only("own-key-two-persons");
    OWN.prepare(&dir.path);
    add_delegation(&dir.path, D_ALICE, CHAT_BRIDGE, ALICE, None);
    let service = support::serve(&dir.path, &operator_public_key);
    let mut world = World {
        service,
        operator_public_key,
        dir,
    };
    world
        .client("chat-bridge", &["open", "--for", BOB, "--adapter", "slack"])
        .ok();
    world.client("chat-bridge", &["open", "--for", ALICE]).ok();
    world.client("chat-bridge", &["close", "--for", ALICE]).ok();
    assert_eq!(opened(&world, "chat-bridge"), [line_of(BOB, "slack")]);

    world.restart();
    let for_alice = world.client("chat-bridge", &["pending", "--for", ALICE]);
    assert!(
        !for_alice.stderr.contains("again with the adapter"),
        "{for_alice:#?}"
    );
    let for_bob = world.client("chat-bridge", &["pending", "--for", BOB]).ok();
    assert!(
        for_bob
            .stderr
            .contains(&format!("opened {BOB} again with the adapter slack")),
        "{for_bob:#?}"
    );
    assert_eq!(opened(&world, "chat-bridge"), [line_of(BOB, "slack")]);
}
