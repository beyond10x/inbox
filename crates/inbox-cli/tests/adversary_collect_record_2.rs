//! Adversary pass 2 on story:collect-returns-the-settled-record against the binary: answers of a
//! service on `INBOX_URL` that carry a record with the collected `message_id` and a `state` and
//! still break the declared answer of `Collect`.

use crate::support;

use support::{ALICE, BOB, World, text};

/// Bob's inbox open and a request of alice in it, kept in alice's outbox file; answers its id.
fn requested(world: &World, body: &str) -> String {
    world.client("bob", &["open"]);
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
                body,
                "--json",
            ],
        )
        .ok()
        .json();
    text(&posted, "message_id")
}

/// A stand-in for the service on 127.0.0.1:0: each `GET` is answered 200 with an empty `rows`,
/// the first request of another method with `status` and `post`; one request per connection.
/// The thread ends after that answer; it fails when no connection comes within 10 s.
fn stand_in(status: u16, post: String) -> (String, std::thread::JoinHandle<()>) {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("the stand-in binds");
    let url = format!("http://{}", listener.local_addr().expect("an address"));
    listener
        .set_nonblocking(true)
        .expect("the listener does not block");
    let handle = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let mut stream = match listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "the stand-in received no request within 10 s"
                    );
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                Err(error) => panic!("the stand-in's listener failed: {error}"),
            };
            stream
                .set_nonblocking(false)
                .expect("the connection blocks");
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
            let mut received = Vec::new();
            let mut buffer = [0_u8; 4096];
            let mut expected = None;
            while expected.is_none_or(|total| received.len() < total) {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => received.extend_from_slice(&buffer[..read]),
                }
                if expected.is_none()
                    && let Some(end) = received.windows(4).position(|w| w == b"\r\n\r\n")
                {
                    let head = String::from_utf8_lossy(&received[..end]).to_lowercase();
                    let length: usize = head
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .and_then(|value| value.trim().parse().ok())
                        .unwrap_or(0);
                    expected = Some(end + 4 + length);
                }
            }
            let is_get = received.starts_with(b"GET ");
            let (status, body) = if is_get {
                (200, "{\"rows\":[]}".to_owned())
            } else {
                (status, post.clone())
            };
            let answer = format!(
                "HTTP/1.1 {status} Stand-in\r\nContent-Type: application/json\r\nContent-Length: \
                 {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(answer.as_bytes());
            if !is_get {
                return;
            }
        }
    });
    (url, handle)
}

const REPLY: &str = "{\"reply_id\":\"01JGT0AE000000000000000050\",\"from\":\"agent:test/bob\",\
     \"on_behalf_of\":\"agent:test/bob\",\"created\":\"2026-01-05T09:05:00Z\",\"body\":\"Blue\",\
     \"signature\":{\"key_id\":\"bob-1\",\"value\":\"c2lnbmF0dXJl\"}}";

/// Every member of the declared record of `id` but `state` and `reply`, in the order of the
/// declaration, followed by `tail` (which holds `state` and whatever the case adds).
fn record(id: &str, tail: &str) -> String {
    format!(
        "{{\"message_id\":\"{id}\",\"from\":\"{ALICE}\",\"on_behalf_of\":\"{ALICE}\",\
         \"to\":\"{BOB}\",\"kind\":\"request\",\"thread\":\"{id}\",\
         \"reply_shape\":{{\"shape\":\"free\",\"value\":{{}}}},\
         \"created\":\"2026-01-05T09:00:00Z\",\"expires\":\"2026-01-06T09:00:00Z\",\
         \"dedup_key\":\"{id}\",\"body\":\"Which colour?\",\
         \"signature\":{{\"key_id\":\"alice-1\",\"value\":\"c2lnbmF0dXJl\"}},{tail}}}"
    )
}

fn answer(outcome: Option<&str>, response: &str) -> String {
    let outcome = outcome.map_or_else(String::new, |name| format!("\"outcome\":\"{name}\","));
    format!("{{{outcome}\"published\":[],\"response\":{response}}}")
}

type Case = (&'static str, Box<dyn Fn(&str) -> String>);

/// Each answer below is `200`, carries no `error`, and its `response` names the collected
/// `message_id` and has a `state`, so it passes the client's check; and each is outside the
/// declared answer of `Collect` (`outcome` `collected`, a record of `Collect`'s response type:
/// `state` one of the labels of `Message.State` and settled, `reply` a `CollectedReply`, each
/// member once). On each the client must not exit 0, and the outbox line must stay, as for an
/// answer without the record.
#[test]
fn an_answer_outside_the_declared_record_neither_succeeds_nor_drops_the_line() {
    let answered = format!(",\"answered_by\":\"principal\",\"reply\":{REPLY}");
    let cases: Vec<Case> = vec![
        (
            "outcome `not-settled` without an error, state Sent",
            Box::new(|id: &str| answer(Some("not-settled"), &record(id, "\"state\":\"Sent\""))),
        ),
        (
            "no outcome",
            Box::new({
                let answered = answered.clone();
                move |id: &str| {
                    answer(
                        None,
                        &record(id, &format!("\"state\":\"Answered\"{answered}")),
                    )
                }
            }),
        ),
        (
            "`collected` with the state Sent",
            Box::new(|id: &str| answer(Some("collected"), &record(id, "\"state\":\"Sent\""))),
        ),
        (
            "`collected` with a state no label of Message.State",
            Box::new(|id: &str| answer(Some("collected"), &record(id, "\"state\":\"Lost\""))),
        ),
        (
            "`collected` with a reply that is a text",
            Box::new(|id: &str| {
                answer(
                    Some("collected"),
                    &record(
                        id,
                        "\"state\":\"Answered\",\"answered_by\":\"principal\",\"reply\":\"Blue\"",
                    ),
                )
            }),
        ),
        (
            "`collected` with a reply whose body is a list",
            Box::new(|id: &str| {
                let reply = REPLY.replace("\"body\":\"Blue\"", "\"body\":[\"Blue\"]");
                answer(
                    Some("collected"),
                    &record(
                        id,
                        &format!(
                            "\"state\":\"Answered\",\"answered_by\":\"principal\",\"reply\":{reply}"
                        ),
                    ),
                )
            }),
        ),
        (
            "`collected` with `message_id` twice, the second of another message",
            Box::new({
                let answered = answered.clone();
                move |id: &str| {
                    answer(
                        Some("collected"),
                        &record(
                            id,
                            &format!(
                                "\"state\":\"Answered\"{answered},\
                                 \"message_id\":\"01JGT0AE000000000000000099\""
                            ),
                        ),
                    )
                }
            }),
        ),
        (
            "`collected` with `reply` twice, two bodies",
            Box::new({
                let answered = answered.clone();
                move |id: &str| {
                    let other = REPLY.replace("\"body\":\"Blue\"", "\"body\":\"Red\"");
                    answer(
                        Some("collected"),
                        &record(
                            id,
                            &format!("\"state\":\"Answered\"{answered},\"reply\":{other}"),
                        ),
                    )
                }
            }),
        ),
    ];
    let world = World::new("adv2-collect-outside");
    let mut wrong = Vec::new();
    for (name, body) in &cases {
        let id = requested(&world, "Which colour?");
        assert!(world.kept("alice").contains(&id), "{name}: posted and kept");
        let (url, stand_in) = stand_in(200, body(&id));
        let run = world.client_env("alice", &["collect", &id], &[("INBOX_URL", url.as_str())]);
        stand_in.join().expect("the stand-in answered");
        let kept = world.kept("alice").contains(&id);
        if run.status == 0 || !kept {
            wrong.push(format!(
                "{name}: exit {} and the outbox line {}; stdout {:?}",
                run.status,
                if kept { "kept" } else { "dropped" },
                run.stdout
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
