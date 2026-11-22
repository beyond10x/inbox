//! The cases story:http-service owes beside its acceptance: authentication, the duplicate-member
//! refusal, the read rule, the wait, the expiry task, each limit, concurrency and binding.

use crate::support;

use std::net::SocketAddr;
use std::sync::mpsc;
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant as Monotonic};

use inbox::messaging::{DecisionAsk, DecisionOption, DedupKey, Kind, OptionLetter, ReplyShape};
use inbox::server::json::Value;
use inbox_core::MemoryBackend;
use inbox_server::{KEY_ID_HEADER, Server, format_instant};
use support::{AGENT, LATER, OTHER, Reply, THIRD, World, encoded, instant, send_raw, ulid};

const POST: &str = "/messaging/commands/post";

fn post(world: &World, address: SocketAddr, from: &str, to: &str, n: u32) -> Reply {
    let envelope = world.post(from, to, n, "a note");
    world.send(address, from, "POST", POST, &encoded(&envelope))
}

fn get(world: &World, address: SocketAddr, principal: &str, target: &str) -> Reply {
    world.send(address, principal, "GET", target, b"")
}

fn pending(principal: &str) -> String {
    format!("/messaging/views/pending?inbox={principal}")
}

// ---- authentication -------------------------------------------------------------------------

#[test]
fn a_request_without_a_signature_is_401_unauthenticated() {
    let world = World::new();
    let server = world.start(|_| {});
    let reply = send_raw(
        server.local_addr(),
        "GET",
        "/messaging/views/inboxes",
        &[],
        b"",
    );
    assert_eq!(reply.status, 401, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"unauthenticated"}"#);
}

#[test]
fn a_request_with_a_wrong_signature_is_401_unauthenticated() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    // Signed with THIRD's key under AGENT's key id.
    let third = world.member(THIRD);
    let mut headers: Vec<(&str, String)> = inbox_server::request_headers(
        "GET",
        "/messaging/views/inboxes",
        b"",
        &world.member(AGENT).key_id,
        &third.key,
        &world.timestamp(),
    )
    .into_iter()
    .collect();
    let reply = send_raw(address, "GET", "/messaging/views/inboxes", &headers, b"");
    assert_eq!(reply.status, 401, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"unauthenticated"}"#);

    // Signed for another target than the one sent.
    headers = inbox_server::request_headers(
        "GET",
        "/messaging/views/inboxes",
        b"",
        &third.key_id,
        &third.key,
        &world.timestamp(),
    )
    .into_iter()
    .collect();
    let reply = send_raw(address, "GET", &pending(THIRD), &headers, b"");
    assert_eq!(reply.status, 401, "{}", reply.body);

    // A key id sent twice.
    headers.push((KEY_ID_HEADER, third.key_id.clone()));
    let reply = send_raw(address, "GET", "/messaging/views/inboxes", &headers, b"");
    assert_eq!(reply.status, 401, "{}", reply.body);
}

#[test]
fn a_stale_request_is_401_and_one_inside_the_window_is_served() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let now = inbox_server::Clock::now(&*world.clock);
    let target = "/messaging/views/inboxes";

    let stale = format_instant(now.plus_seconds(-61));
    let reply = world.send_at(address, AGENT, "GET", target, b"", &stale);
    assert_eq!(reply.status, 401, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"unauthenticated"}"#);

    let edge = format_instant(now.plus_seconds(-60));
    let reply = world.send_at(address, AGENT, "GET", target, b"", &edge);
    assert_eq!(reply.status, 200, "{}", reply.body);
}

// ---- the duplicate-member refusal -----------------------------------------------------------

#[test]
fn a_body_with_a_member_twice_is_refused_before_any_command_runs() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "signed text");
    let text = String::from_utf8(encoded(&envelope)).unwrap();
    // The signed `body` first, a second `body` after it: the generated decoder takes the first, so
    // without the check this text verifies and is stored.
    let twice = text.replacen(
        r#""body":"signed text""#,
        r#""body":"signed text","body":"another text""#,
        1,
    );
    assert_eq!(twice.matches(r#""body":"#).count(), 2, "{twice}");

    let reply = world.send(address, AGENT, "POST", POST, twice.as_bytes());
    assert_eq!(reply.status, 400, "{}", reply.body);
    assert_eq!(
        reply.body,
        r#"{"refused":"duplicate member","member":"body"}"#
    );
    let outbox = get(
        &world,
        address,
        AGENT,
        &format!("/messaging/views/outbox?sender={AGENT}"),
    );
    assert!(outbox.rows().is_empty(), "{}", outbox.body);

    // Unauthenticated, the same body is answered 401: authentication wins.
    let reply = send_raw(address, "POST", POST, &[], twice.as_bytes());
    assert_eq!(reply.status, 401, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"unauthenticated"}"#);
}

// ---- the read rule --------------------------------------------------------------------------

#[test]
fn a_third_principal_reads_no_row_of_a_message_between_two_others() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    world.open(address, THIRD);
    assert_eq!(post(&world, address, AGENT, OTHER, 1).status, 202);
    let id = ulid(1);
    let views = [
        pending(OTHER),
        format!("/messaging/views/outbox?sender={AGENT}"),
        format!("/messaging/views/thread?thread={id}"),
    ];
    for view in &views {
        let third = get(&world, address, THIRD, view);
        assert_eq!(third.status, 200, "{view}: {}", third.body);
        assert!(third.rows().is_empty(), "{view}: {}", third.body);
    }
    // The owner of the inbox in `to` and the sender in `from` read it.
    for principal in [OTHER, AGENT] {
        for view in &views {
            let reply = get(&world, address, principal, view);
            assert_eq!(
                reply.ids(),
                vec![id.clone()],
                "{principal} {view}: {}",
                reply.body
            );
        }
    }
    // `Inboxes` carries neither `to` nor `from` and is answered whole.
    let inboxes = get(&world, address, THIRD, "/messaging/views/inboxes");
    assert_eq!(inboxes.rows().len(), 2, "{}", inboxes.body);
}

#[test]
fn due_and_messages_are_refused_to_a_client_and_so_is_expire() {
    let world = World::new();
    let server = world.start(|_| {});
    let address = server.local_addr();
    let not_granted = r#"{"refused":"not granted","actor":"inbox.messaging.Client"}"#;
    for target in [
        "/messaging/views/due?now=2026-11-21T12:00:00Z",
        "/messaging/views/messages",
    ] {
        let reply = get(&world, address, AGENT, target);
        assert_eq!(reply.status, 403, "{target}: {}", reply.body);
        assert_eq!(reply.body, not_granted);
    }
    let expire = format!(
        r#"{{"message_id":"{}","now":"2026-11-21T12:00:00Z","keep_until":"2026-11-21T12:00:00Z"}}"#,
        ulid(1)
    );
    let reply = world.send(
        address,
        AGENT,
        "POST",
        "/messaging/commands/expire",
        expire.as_bytes(),
    );
    assert_eq!(reply.status, 403, "{}", reply.body);
    assert_eq!(reply.body, not_granted);
}

// ---- the wait -------------------------------------------------------------------------------

/// Starts a wait on `principal`'s `Pending` on a thread; its answer arrives on the channel.
fn wait_on(
    world: &Arc<World>,
    address: SocketAddr,
    principal: &'static str,
    wait: &str,
) -> mpsc::Receiver<(Reply, Monotonic)> {
    let (sent, answered) = mpsc::channel();
    let world = Arc::clone(world);
    let target = format!("{}&wait={wait}", pending(principal));
    thread::spawn(move || {
        let reply = get(&world, address, principal, &target);
        let _ = sent.send((reply, Monotonic::now()));
    });
    answered
}

#[test]
fn a_wait_is_answered_by_a_post_that_arrives_during_it() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = Duration::from_secs(10));
    let address = server.local_addr();
    world.open(address, OTHER);
    let answered = wait_on(&world, address, OTHER, "10");
    // Held open: no answer while the inbox is empty.
    assert!(answered.recv_timeout(Duration::from_millis(300)).is_err());

    // Another request is served while the wait holds, and its post ends the wait long before
    // the wait's own end.
    let started = Monotonic::now();
    assert_eq!(post(&world, address, AGENT, OTHER, 1).status, 202);
    let (reply, at) = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait is answered");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(reply.ids(), vec![ulid(1)]);
    assert!(at - started < Duration::from_secs(5));
}

#[test]
fn a_wait_ends_empty_at_the_embeddings_bound() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = Duration::from_millis(300));
    let address = server.local_addr();
    world.open(address, OTHER);
    let started = Monotonic::now();
    let (reply, at) = wait_on(&world, address, OTHER, "3600")
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait ends at max_wait");
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert!(reply.rows().is_empty(), "{}", reply.body);
    assert!(at - started >= Duration::from_millis(300));

    let reply = get(
        &world,
        address,
        OTHER,
        &format!("{}&wait=x", pending(OTHER)),
    );
    assert_eq!(reply.status, 400, "{}", reply.body);
    assert_eq!(
        reply.body,
        r#"{"refused":"wait is not a whole number of seconds"}"#
    );
}

#[test]
fn stopping_the_service_ends_a_wait() {
    let world = Arc::new(World::new());
    let server = world.start(|config| config.max_wait = Duration::from_secs(30));
    let address = server.local_addr();
    world.open(address, OTHER);
    let answered = wait_on(&world, address, OTHER, "30");
    assert!(answered.recv_timeout(Duration::from_millis(200)).is_err());
    server.stop().expect("the service stops");
    let (reply, _) = answered
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait is answered when the service stops");
    assert!(reply.rows().is_empty(), "{}", reply.body);
}

// ---- the expiry task ------------------------------------------------------------------------

#[test]
fn the_expiry_task_expires_a_due_message_at_a_moved_clock() {
    let world = World::new();
    let server = world.start(|config| config.answer_retention = Duration::from_secs(3600));
    let address = server.local_addr();
    world.open(address, OTHER);
    assert_eq!(post(&world, address, AGENT, OTHER, 1).status, 202);

    let mut decision = world.post(AGENT, OTHER, 2, "A or B?");
    decision.kind = Kind::Decision;
    decision.reply_shape = Some(ReplyShape::Free(inbox::messaging::FreeShape {}));
    decision.decision = Some(DecisionAsk {
        options: ["A", "B"]
            .into_iter()
            .map(|letter| DecisionOption {
                letter: OptionLetter(letter.to_owned()),
                text: format!("option {letter}"),
            })
            .collect(),
        recommended_option: Some(OptionLetter("A".to_owned())),
        request_ref: None,
    });
    decision.default_on_silence = Some(OptionLetter("B".to_owned()));
    world.sign(&mut decision);
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&decision));
    assert_eq!(reply.status, 202, "{}", reply.body);

    assert_eq!(server.sweep_now(), 0, "nothing is due at the start");
    assert_eq!(get(&world, address, OTHER, &pending(OTHER)).ids().len(), 2);

    world.clock.set(instant(LATER));
    assert_eq!(server.sweep_now(), 2);
    let pending = get(&world, address, OTHER, &pending(OTHER));
    assert!(pending.rows().is_empty(), "{}", pending.body);
    // The decision is answered by silence and waits for its sender until the reading plus the
    // answer retention.
    let settled = get(
        &world,
        address,
        AGENT,
        &format!("/messaging/views/settled?sender={AGENT}"),
    );
    let rows = settled.rows();
    assert_eq!(settled.ids(), vec![ulid(2)], "{}", settled.body);
    assert_eq!(
        rows[0].member("expires"),
        Some(&Value::Text("2026-12-01T01:00:00Z".to_owned()))
    );
    assert_eq!(
        rows[0].member("decided_option"),
        Some(&Value::Text("B".to_owned()))
    );
}

#[test]
fn the_expiry_task_runs_every_sweep_interval() {
    let world = World::new();
    let server = world.start(|config| config.sweep_interval = Duration::from_millis(20));
    let address = server.local_addr();
    world.open(address, OTHER);
    assert_eq!(post(&world, address, AGENT, OTHER, 1).status, 202);
    world.clock.set(instant(LATER));
    let deadline = Monotonic::now() + Duration::from_secs(5);
    while !get(&world, address, OTHER, &pending(OTHER))
        .rows()
        .is_empty()
    {
        assert!(Monotonic::now() < deadline, "the task expired nothing");
        thread::sleep(Duration::from_millis(20));
    }
}

// ---- the limits -----------------------------------------------------------------------------

#[test]
fn a_body_over_the_embeddings_limit_is_413() {
    let world = World::new();
    let server = world.start(|config| config.max_body = 64);
    let address = server.local_addr();
    let envelope = world.post(
        AGENT,
        OTHER,
        1,
        "a body that takes the post past sixty-four bytes",
    );
    let reply = world.send(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(reply.status, 413, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"body too large","limit":64}"#);

    // Past the generated reader's own bound, the reader refuses before the body is read.
    let headers = [("Content-Length", "2000000".to_owned())];
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    let head = format!(
        "POST {POST} HTTP/1.1\r\n{}: {}\r\n\r\n",
        headers[0].0, headers[0].1
    );
    std::io::Write::write_all(&mut stream, head.as_bytes()).unwrap();
    let reply = support::read_reply(stream);
    assert_eq!(reply.status, 413, "{}", reply.body);
}

#[test]
fn a_post_beyond_the_message_count_is_503_before_the_command_runs() {
    let world = World::new();
    let server = world.start(|config| config.max_messages = 1);
    let address = server.local_addr();
    world.open(address, OTHER);
    assert_eq!(post(&world, address, AGENT, OTHER, 1).status, 202);
    let reply = post(&world, address, AGENT, OTHER, 2);
    assert_eq!(reply.status, 503, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"message limit","limit":1}"#);
    let outbox = get(
        &world,
        address,
        AGENT,
        &format!("/messaging/views/outbox?sender={AGENT}"),
    );
    assert_eq!(outbox.ids(), vec![ulid(1)]);
}

#[test]
fn a_connection_beyond_the_limit_waits_until_one_ends() {
    let world = Arc::new(World::new());
    let server = world.start(|config| {
        config.max_connections = 1;
        config.max_wait = Duration::from_millis(600);
    });
    let address = server.local_addr();
    world.open(address, OTHER);
    let waiting = wait_on(&world, address, OTHER, "60");
    thread::sleep(Duration::from_millis(100));
    let (sent, second) = mpsc::channel();
    let other = Arc::clone(&world);
    thread::spawn(move || {
        let asked = Monotonic::now();
        let reply = get(&other, address, AGENT, "/messaging/views/inboxes");
        let _ = sent.send((reply, Monotonic::now() - asked));
    });
    let (wait, _) = waiting
        .recv_timeout(Duration::from_secs(5))
        .expect("the wait ends");
    assert!(wait.rows().is_empty(), "{}", wait.body);
    let (reply, took) = second
        .recv_timeout(Duration::from_secs(5))
        .expect("the second request is served");
    assert_eq!(reply.status, 200, "{}", reply.body);
    // Asked about 100 ms into a 600 ms wait: served beside it, it would take a few milliseconds.
    assert!(
        took >= Duration::from_millis(300),
        "the second connection was served beside the first after {took:?}"
    );
}

// ---- concurrency ----------------------------------------------------------------------------

#[test]
fn two_posts_at_once_for_one_dedup_key_are_one_posted_and_one_refused() {
    let world = Arc::new(World::new());
    let server = world.start(|_| {});
    let address = server.local_addr();
    world.open(address, OTHER);
    let barrier = Arc::new(Barrier::new(2));
    let posts: Vec<_> = [1, 2]
        .into_iter()
        .map(|n| {
            let mut envelope = world.post(AGENT, OTHER, n, "one logical message");
            envelope.dedup_key = DedupKey("one-key".to_owned());
            world.sign(&mut envelope);
            let (world, barrier) = (Arc::clone(&world), Arc::clone(&barrier));
            thread::spawn(move || {
                barrier.wait();
                world.send(address, AGENT, "POST", POST, &encoded(&envelope))
            })
        })
        .collect();
    let mut outcomes: Vec<String> = posts
        .into_iter()
        .map(|post| post.join().expect("the post is answered").text("outcome"))
        .collect();
    outcomes.sort();
    assert_eq!(outcomes, vec!["duplicate-key", "posted"]);
}

// ---- binding --------------------------------------------------------------------------------

#[test]
fn only_a_loopback_address_is_bound_and_port_zero_names_the_port_taken() {
    let world = World::new();
    let mut config = world.config();
    config.address = "0.0.0.0:0".parse().unwrap();
    let refused = Server::bind(config, MemoryBackend::new()).expect_err("not loopback");
    assert_eq!(refused.kind(), std::io::ErrorKind::InvalidInput);

    let server = Server::bind(world.config(), MemoryBackend::new()).expect("loopback binds");
    assert_ne!(server.local_addr().port(), 0);
    assert!(server.local_addr().ip().is_loopback());
}

// ---- a request that does not arrive -------------------------------------------------------------

/// Connects and sends the start of a request head, then nothing more.
fn half_a_request(address: SocketAddr) -> std::net::TcpStream {
    let mut stream = std::net::TcpStream::connect(address).expect("the service accepts");
    std::io::Write::write_all(
        &mut stream,
        b"GET /messaging/views/inboxes HTTP/1.1\r\nHost: ",
    )
    .expect("the start of the head is sent");
    stream
}

#[test]
fn a_request_not_arrived_within_the_request_timeout_is_408() {
    let world = World::new();
    // Shorter than the reader's own 1 s gap bound, so the answer is this service's and not the
    // reader's `400`.
    let server = world.start(|config| config.request_timeout = Duration::from_millis(300));
    let started = Monotonic::now();
    let reply = support::read_reply(half_a_request(server.local_addr()));
    assert_eq!(reply.status, 408, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"request timeout"}"#);
    assert!(Monotonic::now() - started < Duration::from_millis(900));
}

#[test]
fn stop_answers_a_connection_still_sending_its_request_with_503() {
    let world = World::new();
    let server = world.start(|config| config.request_timeout = Duration::from_secs(3600));
    let stream = half_a_request(server.local_addr());
    thread::sleep(Duration::from_millis(100));
    let started = Monotonic::now();
    server.stop().expect("the service stops");
    assert!(Monotonic::now() - started < Duration::from_secs(2));
    let reply = support::read_reply(stream);
    assert_eq!(reply.status, 503, "{}", reply.body);
    assert_eq!(reply.body, r#"{"refused":"stopping"}"#);
}

// ---- the epoch ------------------------------------------------------------------------------

/// The one `Inbox-Epoch` header of an answer's head: 32 lower-case hex digits.
fn epoch_of(head: &str) -> String {
    let values = support::header(head, "Inbox-Epoch");
    assert_eq!(values.len(), 1, "one Inbox-Epoch header in {head}");
    let value = values[0].clone();
    assert!(
        value.len() == 32
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "Inbox-Epoch {value:?} is not 32 lower-case hex digits"
    );
    value
}

#[test]
fn every_answer_of_one_start_carries_its_epoch_whatever_its_status() {
    let world = World::new();
    let server = world.start(|config| {
        config.max_body = 4096;
        config.max_messages = 1;
    });
    let address = server.local_addr();
    let mut seen = Vec::new();

    // 200: a view, signed.
    let (head, reply) = world.send_answer(address, AGENT, "GET", "/messaging/views/inboxes", b"");
    assert_eq!(reply.status, 200, "{}", reply.body);
    seen.push((200, epoch_of(&head)));

    // 202: a command, signed.
    world.open(address, OTHER);
    let envelope = world.post(AGENT, OTHER, 1, "a note");
    let (head, reply) = world.send_answer(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(reply.status, 202, "{}", reply.body);
    seen.push((202, epoch_of(&head)));

    // 503: the message limit, before the command runs.
    let envelope = world.post(AGENT, OTHER, 2, "a second note");
    let (head, reply) = world.send_answer(address, AGENT, "POST", POST, &encoded(&envelope));
    assert_eq!(reply.status, 503, "{}", reply.body);
    seen.push((503, epoch_of(&head)));

    // 401: no request signature.
    let (head, reply) =
        support::send_raw_answer(address, "GET", "/messaging/views/inboxes", &[], b"");
    assert_eq!(reply.status, 401, "{}", reply.body);
    seen.push((401, epoch_of(&head)));

    // 400: a member twice, signed.
    let twice = br#"{"address":"x","address":"y","adapter":"terminal"}"#;
    let (head, reply) = world.send_answer(
        address,
        AGENT,
        "POST",
        "/messaging/commands/open-inbox",
        twice,
    );
    assert_eq!(reply.status, 400, "{}", reply.body);
    seen.push((400, epoch_of(&head)));

    // 413: the body limit of the embedding, before authentication.
    let (head, reply) = support::send_raw_answer(address, "POST", POST, &[], &[b' '; 5000]);
    assert_eq!(reply.status, 413, "{}", reply.body);
    seen.push((413, epoch_of(&head)));

    // 413: the generated reader's own refusal, before the body is read.
    let mut stream = std::net::TcpStream::connect(address).unwrap();
    std::io::Write::write_all(
        &mut stream,
        format!("POST {POST} HTTP/1.1\r\nContent-Length: 2000000\r\n\r\n").as_bytes(),
    )
    .unwrap();
    let (head, reply) = support::read_answer(stream);
    assert_eq!(reply.status, 413, "{}", reply.body);
    seen.push((413, epoch_of(&head)));

    let first = seen[0].1.clone();
    for (status, epoch) in &seen {
        assert_eq!(epoch, &first, "the {status} answer names another start");
    }
}

#[test]
fn the_answers_cut_off_before_a_request_arrived_carry_the_epoch() {
    let world = World::new();
    let server = world.start(|config| config.request_timeout = Duration::from_millis(300));
    let (head, reply) = support::read_answer(half_a_request(server.local_addr()));
    assert_eq!(reply.status, 408, "{}", reply.body);
    let timed_out = epoch_of(&head);
    let (head, _) = world.send_answer(
        server.local_addr(),
        AGENT,
        "GET",
        "/messaging/views/inboxes",
        b"",
    );
    assert_eq!(epoch_of(&head), timed_out, "one start, one epoch");

    let server = world.start(|config| config.request_timeout = Duration::from_secs(3600));
    let stream = half_a_request(server.local_addr());
    thread::sleep(Duration::from_millis(100));
    server.stop().expect("the service stops");
    let (head, reply) = support::read_answer(stream);
    assert_eq!(reply.status, 503, "{}", reply.body);
    epoch_of(&head);
}

#[test]
fn two_starts_answer_two_epochs() {
    let world = World::new();
    let first = world.start(|_| {});
    let second = world.start(|_| {});
    let epochs: Vec<String> = [&first, &second]
        .into_iter()
        .map(|server| {
            let (head, reply) = world.send_answer(
                server.local_addr(),
                AGENT,
                "GET",
                "/messaging/views/inboxes",
                b"",
            );
            assert_eq!(reply.status, 200, "{}", reply.body);
            epoch_of(&head)
        })
        .collect();
    assert_ne!(
        epochs[0], epochs[1],
        "two starts over one clock and one store answer one epoch"
    );
    // A start over the same store, after the first stopped: a restart.
    first.stop().expect("the first service stops");
    let third = world.start(|_| {});
    let (head, _) = world.send_answer(
        third.local_addr(),
        AGENT,
        "GET",
        "/messaging/views/inboxes",
        b"",
    );
    let restarted = epoch_of(&head);
    assert!(
        !epochs.contains(&restarted),
        "a restart answers an epoch of an earlier start"
    );
}

#[test]
fn every_duration_of_the_configuration_may_be_the_largest_there_is() {
    let world = World::new();
    let server = world.start(|config| {
        config.max_wait = Duration::MAX;
        config.sweep_interval = Duration::MAX;
        config.request_timeout = Duration::MAX;
        config.answer_retention = Duration::MAX;
    });
    let address = server.local_addr();
    world.open(address, OTHER);
    assert_eq!(post(&world, address, AGENT, OTHER, 1).status, 202);
    world.clock.set(instant(LATER));
    // `keep_until` is held to the last second RFC 3339 writes, and the note is expired.
    assert_eq!(server.sweep_now(), 1);
    assert!(
        get(&world, address, OTHER, &pending(OTHER))
            .rows()
            .is_empty()
    );
    // Neither task died on its arithmetic: `stop` reports a task that panicked.
    server.stop().expect("the service and both its tasks stop");
}
