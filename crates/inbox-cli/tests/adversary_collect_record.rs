//! Adversary pass 1 on story:collect-returns-the-settled-record against the binary: what
//! `inbox collect` does with an answer that does not carry the record of the message it
//! collected, and what its `--json` document holds under `message` against 0.2.0.

use crate::support;

use std::time::SystemTime;

use inbox::server::json::Value;
use inbox::server::wire;
use inbox_verify::{Instant, KeyRing, PublicKey};

use support::{BOB, World, rows, stand_in_answers, text};

/// The signature of a reply covers its whole `Answer` envelope without `signature`, `expires`
/// included (`inbox_verify::envelope_bytes`). Before the collect, the sender's `Outbox` row
/// carries that `expires` (the answer set the message's `expires` to the reply's), and 0.2.0's
/// `collect --json` printed it under `message`. The record carries every other member of the
/// envelope (`message_id` as `in_reply_to`, and `reply`), and no `expires`: the sender cannot
/// verify the reply it collected from what the collect handed it.
#[test]
fn the_record_lets_the_sender_verify_the_reply_it_collected() {
    let world = World::new("adv-collect-verify");
    let id = requested(&world, "Which colour?");
    world.client("bob", &["answer", &id, "--body", "Blue"]).ok();
    let outbox = world.client("alice", &["outbox", "--json"]).ok().json();
    let row = rows(&outbox)
        .into_iter()
        .find(|row| text(row, "message_id") == id)
        .expect("the outbox row of the answered request");
    let shown_expires = row
        .member("expires")
        .cloned()
        .expect("`expires` in the row");

    let printed = world
        .client("alice", &["collect", &id, "--json"])
        .ok()
        .json();
    let record = printed.member("message").expect("`message`").clone();
    let reply = record.member("reply").expect("the record's reply").clone();
    let ring = KeyRing::parse(
        &std::fs::read_to_string(world.dir.join("keys.json")).expect("the key file"),
        &PublicKey::from_base64(&world.operator_public_key).expect("the operator key"),
    )
    .expect("the key file parses");
    let verifies = |expires: &Value| -> Result<(), String> {
        let Value::Object(members) = &reply else {
            return Err(format!("the reply is no object: {reply:?}"));
        };
        let mut members = members.clone();
        members.push((
            "in_reply_to".to_owned(),
            record.member("message_id").cloned().unwrap_or(Value::Null),
        ));
        members.push(("expires".to_owned(), expires.clone()));
        let answer = wire::decode_command_inbox_messaging_answer(&Value::Object(members), "")
            .map_err(|error| error.to_string())?;
        ring.verify_envelope(&answer, Instant::from_system_time(SystemTime::now()))
            .map_err(|refusal| format!("{refusal:?}"))
    };
    assert_eq!(
        verifies(&shown_expires),
        Ok(()),
        "the record's reply with the `expires` the outbox row showed is the signed envelope"
    );
    let carried = record.member("expires").or_else(|| reply.member("expires"));
    assert_eq!(
        carried,
        Some(&shown_expires),
        "the record carries no `expires` of the reply, so its signature cannot be verified \
         from the record: {record:?}"
    );
}

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

fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

/// A case: its name, the HTTP status of the answer, and the answer's body for a message id.
type Case = (&'static str, u16, Box<dyn Fn(&str) -> String>);

/// The acceptance: "`inbox collect` prints the record of that answer and removes its outbox line
/// only after it has that answer". Each answer below is `2xx` and names `collected`, and none
/// carries the record of the message collected: the answer of a 0.2.0 service still running
/// after the client was upgraded (`202`, no `response`), a `response` of another message, a
/// `response` that is no object, one without `state`, and a truncated body. For each, the client
/// must not exit 0 and must keep the outbox line, as it does for an answer that is lost.
#[test]
fn collect_without_the_record_of_its_message_neither_succeeds_nor_drops_the_line() {
    let world = World::new("adv-collect-no-record");
    let mut wrong = Vec::new();
    let cases: Vec<Case> = vec![
        (
            "0.2.0 answer: 202, no response",
            202,
            Box::new(|id: &str| {
                format!(
                    "{{\"outcome\":\"collected\",\"published\":[{{\"event\":\"inbox.messaging.MessageHandled\",\"payload\":{{\"message_id\":\"{id}\"}}}}]}}"
                )
            }),
        ),
        (
            "response of another message",
            200,
            Box::new(|id: &str| {
                format!(
                    "{{\"outcome\":\"collected\",\"published\":[],\"response\":{{\"message_id\":\"01JGT0AC000000000000000099\",\"to\":\"{BOB}\",\"thread\":\"01JGT0AC000000000000000099\",\"state\":\"Refused\",\"refusal\":\"not {id}\"}}}}"
                )
            }),
        ),
        (
            "response that is a text",
            200,
            Box::new(|_: &str| {
                "{\"outcome\":\"collected\",\"published\":[],\"response\":\"gone\"}".to_owned()
            }),
        ),
        (
            "response without state",
            200,
            Box::new(|id: &str| {
                format!(
                    "{{\"outcome\":\"collected\",\"published\":[],\"response\":{{\"message_id\":\"{id}\",\"to\":\"{BOB}\",\"thread\":\"{id}\"}}}}"
                )
            }),
        ),
        (
            "truncated body",
            200,
            Box::new(|id: &str| {
                format!("{{\"outcome\":\"collected\",\"response\":{{\"message_id\":\"{id}\"")
            }),
        ),
    ];
    for (name, status, body) in &cases {
        let id = requested(&world, "Which colour?");
        assert!(world.kept("alice").contains(&id), "{name}: posted and kept");
        let (url, stand_in) = stand_in_answers(vec![(*status, None, leak(body(&id)))]);
        let run = world.client_env("alice", &["collect", &id], &[("INBOX_URL", url.as_str())]);
        let _ = stand_in.join();
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

/// 0.2.0 printed under `message` the sender's `Outbox` row of the collected message (`from`,
/// `kind`, `body`, `created`, `expires`, ...). The CHANGELOG of this unit says the record is
/// printed "(`--json`: under `message`, as before)". A script that read the message it had sent
/// from `.message.body` or `.message.kind` reads nothing now.
#[test]
fn collect_json_keeps_the_members_0_2_0_printed_under_message() {
    let world = World::new("adv-collect-json-shape");
    let id = requested(&world, "Which colour?");
    world.client("bob", &["answer", &id, "--body", "Blue"]).ok();
    let printed = world
        .client("alice", &["collect", &id, "--json"])
        .ok()
        .json();
    let message = printed.member("message").expect("`message`");
    let mut missing = Vec::new();
    for member in [
        "message_id",
        "from",
        "on_behalf_of",
        "to",
        "kind",
        "thread",
        "reply_shape",
        "created",
        "expires",
        "dedup_key",
        "body",
        "signature",
        "answered_by",
        "reply",
        "state",
    ] {
        if message.member(member).is_none() {
            missing.push(member);
        }
    }
    assert!(
        missing.is_empty(),
        "`message` of `collect --json` lacks {missing:?} that 0.2.0 printed: {message:?}"
    );
    assert_eq!(
        message.member("body"),
        Some(&Value::Text("Which colour?".to_owned()))
    );
}
